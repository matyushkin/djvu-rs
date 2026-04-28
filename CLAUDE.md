# Notes for Claude Code

This file logs performance experiments and their outcomes.
Referenced from issue templates ("Record result in CLAUDE.md (Kept or Reverted + reason)").

## Performance experiments

Each entry: issue, approach, numbers, decision, reason.

### #185 — perf(jb2): bit-pack Jbm to 1 bit/pixel — **Kept** (2026-04-18)

**Approach.** Changed the internal `Jbm` working bitmap from 1 byte/pixel
(`Vec<u8>` of `w * h`) to 1 bit/pixel packed (`Vec<u8>` of
`((w + 7) / 8) * h`, MSB-first within byte) — matching `Bitmap`'s public
convention. 8× memory reduction on the symbol dict.

Decoder hot path uses **Variant A**: `decode_bitmap_direct` and
`decode_bitmap_ref` keep rolling unpacked scratch rows (3 for direct,
3 mbm + 2 cbm for ref) and pack into `Jbm.data` once per row. The ZP
inner loop is unchanged. New helpers: `pack_row_into`, `unpack_row_into`.

`blit_indexed`: reads packed source with a byte-at-a-time skip of
all-zero bytes (common for sparse symbols). `blit_to_bitmap`: source and
dest are both packed MSB-first; byte-aligned branch becomes a direct `|=`
row copy, unaligned branch is a shift-and-OR.

**Bench** (`cargo bench`, 100 samples, Linux x86_64, Criterion p-values):

| Benchmark                    | Baseline  | Packed    | Δ      | p    |
|------------------------------|-----------|-----------|--------|------|
| `jb2_decode`                 | 187.93 µs | 188.79 µs | +0.5%  | 0.31 |
| `jb2_decode_corpus_bilevel`  | 813.80 µs | 782.21 µs | −3.9%  | 0.00 |
| `jb2_decode_large_600dpi`    | 4.37 µs   | 4.27 µs   | −2.3%  | 0.06 |
| `render_corpus_bilevel`      | 189.76 ms | 191.36 ms | +0.8%  | 0.19 |

No regression anywhere; `jb2_decode_corpus_bilevel` is significantly
faster (p = 0.00), consistent with reduced L2 pressure on the decoded
symbol dict.

**Reason kept.** 8× memory reduction on working bitmaps with neutral-to-
positive decode/render perf. The scratch allocation in the hot path
(three `Vec<u8>` × `width` bytes per symbol decode, reused across rows)
adds no measurable overhead vs the previous direct-indexed `bm.data`
split. All 324 library + 71 integration tests pass.

**Notes.** The issue suggested `Vec<u32>` + 32-bit row alignment for SIMD
potential. That was relaxed to byte-aligned `Vec<u8>` to match `Bitmap`
exactly (avoiding the byte→bit packing step in `blit_to_bitmap`). A
follow-up could explore word-granular compositing once there is a
workload that stresses the unaligned `blit_to_bitmap` branch.

### #184 — perf(iw44): column_pass SIMD at s=2 — **Reverted** (2026-04-18)

**Approach.** Generalised the existing `s == 1` SIMD fast path in the column
pass of `inverse_wavelet_transform_from` to `s ∈ {1, 2}`. Introduced
stride-aware helpers `load8_col_s` / `store8_col_s` that gather/scatter 8
`i16` samples at stride `s`, threaded an `allow_simd` parameter for
comparability, and added a golden test
(`simd_inverse_wavelet_transform_matches_scalar`) that confirmed bit-exact
parity with the scalar path on 32×32 and 33×32 planes.

**Bench** (`cargo bench --bench codecs -- 'iw44_decode_first_chunk|iw44_decode_corpus_color'`,
release, 100 samples, Linux x86_64):

| Benchmark                  | Scalar   | SIMD s=2 | Δ     |
|----------------------------|----------|----------|-------|
| `iw44_decode_first_chunk`  | 1.226 ms | 1.206 ms | −1.6% |
| `iw44_decode_corpus_color` | 3.747 ms | 3.669 ms | −2.1% |

Run-to-run noise on the same build was ±2–5% (e.g. `iw44_decode_corpus_color`
ranged 3.31 ms → 3.81 ms across consecutive runs). Criterion's change test
came back non-significant (`p ∈ {0.09, 0.20, 0.24, 0.36, 0.68}`) once the
cold-start outlier was excluded.

**Reason.** On x86_64, the implementation must fall back to 8 scalar loads
assembled into an `i32x8` — `wide::i32x8` exposes no strided / gather load for
`i16`, and no native `_mm*_i16gather_*` intrinsic exists for 16-bit lanes.
The arithmetic savings at `s == 2` (which already processes half as many
columns as `s == 1`) do not exceed the gather overhead.

The issue expected the win to come from ARM64 NEON `vld2q_s16` / `vst2q_s16`,
which are not reachable through `wide` and would require raw
`core::arch::aarch64` intrinsics. Without that, there is no benefit on the
x86_64 CI host. The stride-aware helpers would be reusable if the ARM64
follow-up lands, but committing them today costs complexity for zero measured
gain.

**Next step.** Re-attempt on ARM64 (M1) with raw NEON `vld2q_s16`, measure
against the baseline `iw44_decode_first_chunk` (715 µs) on the reference
hardware listed in `BENCHMARKS_RESULTS.md`.

### #194 Phase 2 — multi-page shared Djbz with Hamming clustering — **Reverted default, kept tunable knob** (2026-04-28)

**Approach.** Phase 1 (#194, shipped) builds the shared Djbz dictionary by
byte-exact `(w, h, data)` dedup of CCs across pages: any CC signature
appearing on `≥ threshold` distinct pages becomes a shared symbol, the rest
stay per-page Sjbz. Phase 2 attempted to widen the cluster predicate to
"same `(w, h)` AND `packed_hamming(rep, cc) ≤ pixels * fraction`", folding
near-duplicate scanned-glyph variants into one shared rep so the per-page
Sjbz can emit `rec-7` (matched copy) or `rec-6` (matched refinement)
instead of `rec-1` (new direct).

Implementation: `cluster_shared_symbols_tunable(pages, page_threshold,
diff_fraction)` — bucketed by `(w, h)`, linear scan per bucket choosing the
nearest existing rep within `max_diff = pixels * diff_fraction / 100` (with
a `REFINEMENT_MIN_PIXELS = 32` floor that keeps tiny CCs byte-exact).
`encode_djvm_bundle_jb2_with_shared(pages, &shared)` lets a benchmark
harness drive cluster selection without re-running the IFF/DIRM pipeline.

**Harness.** `examples/encode_quality_djbz.rs` — for each multi-page DjVu
input, computes total bytes for {original Sjbz, independent
`encode_jb2_dict` per page, bundled `encode_djvm_bundle_jb2_with_shared`}
across configurable Hamming thresholds; verifies pixel-exact bundle
round-trip.

**Bench** (`encode_quality_djbz` on `pathogenic_bacteria_1896.djvu`,
517 pages of cjb2 scans, Apple M1 Max):

| `--diff-fraction` | shared syms | Djbz bytes | Σ Sjbz | bundle / independent | round-trip |
|-------------------|-------------|------------|--------|----------------------|------------|
| 0 (byte-exact, shipped) | 1 568 | 41 KB | 7.40 MB | **0.870×** (−13.0%) | ✓ |
| 1% | 1 547 | 40 KB | 7.40 MB | 0.870× (−13.0%) | ✓ |
| 2% | 1 503 | 39 KB | 7.41 MB | 0.871× (−12.9%) | ✓ |
| 3% | 1 449 | 38 KB | — | — | **✗ mismatch** |
| 4% | 1 387 | 36 KB | 7.50 MB | 0.877× (−12.3%) | ✓ |

Small corpus (`tests/corpus/*.djvu`, 36 pages from 4 books):

| `--diff-fraction` | bundle / independent |
|-------------------|----------------------|
| 0 (byte-exact)    | 1.021× (+2.1%) |
| 4%                | 1.150× (+15.0%) |

**Reason reverted as default.** The Phase 1 byte-exact win (−13.0% bundle
vs independent on the 517-page corpus) is the entire shared-Djbz benefit.
Hamming clustering at 1–2% is within 0.05% of byte-exact; at 4% it is
strictly worse. Hypothesis: the per-page `symbol_index_ctx` encoding pays
≈ `log2(K)` bits per reference, so growing `K` (more shared reps) inflates
every `rec-7` reference; meanwhile `rec-6` refinement bitmaps cost more
ZP-coded bits than a fresh `rec-1` direct emission whenever the shared rep
isn't a near-perfect match. Net: cross-page Hamming clustering must match
*better than* the per-page intra-CC refinement matcher already does within
each page (#188 Phase 3) — and on this corpus it doesn't.

**Reason kept tunable.** `cluster_shared_symbols_tunable` and
`encode_djvm_bundle_jb2_with_shared` are exposed `pub` so the benchmark
harness — and any future Phase 2.5 calibration work (per-CC profitability
model instead of a flat fraction) — can sweep thresholds without forking
the encoder. The default `cluster_shared_symbols` continues to delegate to
`diff_fraction = 0`.

**Open follow-ups.**
1. The `diff_fraction = 3%` round-trip mismatch on the big corpus is a real
   bug in the rec-6 refinement path against shared reps — should be filed
   as a sub-issue. (Doesn't block ship: 0% remains lossless and is the
   shipped default.)
2. Per-CC profitability model: instead of a flat Hamming fraction, decide
   per CC whether `cost(rec-6 against shared rep)` < `cost(rec-1 fresh) +
   amortized log2(K) increase`. Unclear if the win exists — would need to
   re-measure with a corpus where intra-page refinement is already
   exhausted.
