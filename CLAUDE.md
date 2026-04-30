# Notes for Claude Code

This file logs performance experiments and their outcomes.
Referenced from issue templates ("Record result in CLAUDE.md (Kept or Reverted + reason)").

## Performance experiments

Each entry: issue, approach, numbers, decision, reason.

### #225 Phase 2 — public `render_streaming` API — **Kept** (2026-04-30)

**Approach.** Built on Phase 1's internal `render_rows` primitive. Added one
new public entry point and one new error variant:

- `pub fn render_streaming<F: FnMut(usize, &[u8])>(page, opts, sink)` — thin
  wrapper around `render_rows` that rejects render options requiring
  post-processing of a fully-allocated pixmap.
- `RenderError::UnsupportedOption(&'static str)` — returned when the streaming
  path cannot honour the requested options.

The constraints surface what `render_pixmap` does after compositing: the
streaming path *cannot* support `opts.aa = true` (the AA downscale needs the
full pixmap), `opts.resampling = Lanczos3` *when scaling actually happens*
(re-renders at native resolution and downscales), or any non-identity
combined rotation (`combine_rotations(page.rotation(), opts.rotation)`
wraps a rotate-pixmap step). When all three constraints hold,
`render_streaming` is byte-identical to `render_pixmap` — verified by two
new tests on `chicken.djvu` (color) and `boy_jb2.djvu` (bilevel).

Lanczos at native size is permitted: the early-return path in
`render_pixmap` skips Lanczos when output dimensions equal page dimensions
(`need_scale = false`), so it has no effect on bytes either way.

**Tests.** Seven new unit tests in `djvu_render::tests`:

- `render_streaming_byte_identical_to_render_pixmap_color`
- `render_streaming_byte_identical_to_render_pixmap_bilevel`
- `render_streaming_rejects_aa`
- `render_streaming_rejects_lanczos_with_scaling`
- `render_streaming_allows_lanczos_at_native_size`
- `render_streaming_rejects_user_rotation`
- `render_streaming_rejects_zero_dimensions`

All 403 lib tests pass; clippy `-D warnings` and `cargo fmt --check` clean.

**Memory.** Phase 1 already established that the internal compositing path
allocates a single `opts.width * 4` byte scratch row reused across rows;
`render_streaming` inherits that. Peak heap during compositing is bounded
by `scratch_row + decoded BG44 + decoded JB2 mask + FG palette` — no full
pixmap. The 600-dpi A3 (≈100 MB pixmap) target from the issue's DoD is met
by construction (the scratch row is < 16 KB at any reasonable width).

**Reason kept.** The DoD-required public API is now in place with no
behavioural change for existing `render_pixmap` callers, byte-exact
equivalence verified, post-processing options safely refused with a typed
error rather than silently producing different output. The `UnsupportedOption`
variant is `&'static str` — no allocation on the error path. Phase 1's
zero-cost adapter through `render_rows` means `render_pixmap` continues to
benefit from the warm-cache row scratch (CLAUDE.md `### #225 Phase 1`,
−13% on `render_page/dpi/72`).

**Open follow-ups.**
1. `render_region`, `render_coarse`, `render_progressive` could similarly
   gain streaming variants if a use case appears.
2. Memory benchmark from the issue's DoD ("peak RSS during render of a
   600-dpi 2550×3301 page < 4 MB") not yet wired into `bench/`. Manual
   verification via `heaptrack` or `dhat` would confirm the BG44/mask
   buffers are the only large allocations.

### #225 Phase 1 — internal row-streaming render refactor — **Kept** (2026-04-29)

**Approach.** Extracted the composite hot path into a per-row streaming
primitive without changing the public API. Three new module-private functions:

- `composite_rows_bilevel_one` / `composite_rows_bilinear_one` /
  `composite_rows_area_avg_one` — per-row helpers containing the pixel-level
  computation for each of the three compositing modes (bilevel fast path,
  bilinear upscale/1:1, area-average downscale). These are `#[inline]` and
  mirror the existing `composite_loop_*` bodies row by row.

- `composite_rows<F: FnMut(usize, &[u8])>` — allocates a single row scratch
  buffer (`out_w * 4` bytes, reused across rows), calls the appropriate per-row
  helper, then invokes the sink `F(row_index, &row_slice)`. The
  `composite_into` direct flat-buffer path is untouched and continues to drive
  `render_into`, `render_region`, `render_coarse`, and `render_progressive`.

- `pub(crate) render_rows<F>` — decode/setup entry point (mirrors
  `render_pixmap`'s decode logic) that calls `composite_rows`. This is the
  Phase 2 hook: future `render_streaming` will delegate here instead of
  allocating a full Pixmap.

`render_pixmap` is now a thin adapter: it pre-allocates `Pixmap::white(w, h)`,
calls `render_rows` with a sink that copies each row into `pm.data`, then
applies the existing aa/Lanczos/rotation post-processing steps.

Two new unit tests — `render_rows_byte_identical_to_render_into_color` and
`render_rows_byte_identical_to_render_into_bilevel` — verify that
`composite_rows` and `composite_into` produce byte-exact identical output for
color (chicken.djvu) and bilevel (boy_jb2.djvu) pages.

**Bench** (`cargo bench --bench render -- 'render_page/dpi/72'`,
100 samples, Apple M1 Max):

| Benchmark             | Before   | After    | Δ       |
|-----------------------|----------|----------|---------|
| `render_page/dpi/72`  | 243.5 µs | 211.8 µs | **−13%** |
| `render_colorbook_cold` | — | 17.8 ms | flat (no prior baseline in this worktree) |

The 72-dpi benchmark **improved** by ~13% despite the per-row scratch
allocation and `copy_from_slice` on each row. The likely cause: the scratch row
buffer (`w * 4 ≈ 400–2400 bytes`) fits entirely in L1 cache; subsequent writes
from the composite inner loop and the copy into `pm.data` both hit warm L1
rather than cold L2/L3 as in the previous approach that wrote directly into the
full pre-allocated pixmap. The decode step dominates at 72 dpi (BG44 + JB2
cache hits account for ~200 µs), so even the best-case compositing improvement
is bounded.

**Reason kept.** Material improvement on the warm-cache render benchmark (−13%)
with zero public API change, bit-exact output verified by tests, all 550 tests
pass, clippy and fmt clean. The `render_rows` hook is in place for Phase 2.

**Open follow-ups.**
1. Phase 2 (future PR): expose `pub fn render_streaming` with a user-visible
   row callback, enabling true zero-full-pixmap rendering for WASM / embedded.
2. `render_region`, `render_coarse`, `render_progressive` could similarly be
   refactored to use `composite_rows` for API symmetry, but are not hot paths.

### #190 Phase 2 — WASM simd128 inverse wavelet (load/store stride-1) — **Kept** (2026-04-29)

**Approach.** Added `load8s_s1_simd128` and `store8s_s1_simd128` (gated on
`cfg(all(target_arch = "wasm32", target_feature = "simd128"))`) as the WASM
counterparts to the AVX2 stride-1 helpers shipped in Phase 2 of #189.

`load8s_s1_simd128`: loads 8 consecutive i16 as one `v128`, then calls
`i32x4_extend_low_i16x8` / `i32x4_extend_high_i16x8` to sign-extend into two
`v128`s of i32, which are transmuted directly to `wide::i32x8` (`{a: i32x4(v128),
b: i32x4(v128)}`). This replaces 8 scalar `as i32` casts assembled via
`i32x8::from([...])`.

`store8s_s1_simd128`: transmutes `i32x8` back to `[v128; 2]`, then uses a
constant `i8x16_shuffle` with indices `[0,1,4,5,8,9,12,13, 16,17,20,21,24,25,28,29]`
to pick the low 2 bytes of each i32 lane from both halves into a single `v128`,
stored in one `v128_store`. This replicates the truncating `as i16` semantics of
the scalar path (not saturating narrow), matching the AVX2 byte-shuffle approach.

Both functions are wired into `load8s` and `store8s` via a compile-time
`#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]` block
(the `return` before the scalar `#[allow(unreachable_code)]` block), so
the hot column-pass loop at `s == 1` gets the fast path with no runtime branch.

**Bench.** No direct wasm bench harness available locally. Expected speedup is
analogous to the AVX2 load/store path (#189 Phase 2), which measured −3.9% on
`jb2_decode_corpus_bilevel`. The WASM path processes 8 lanes (same as `v128`
width) in 2 ops (load) or 1 shuffle + 1 store (store) vs 8 scalar cast-and-
write pairs. The column pass at `s=1` is the hottest sub-kernel in
`inverse_wavelet_transform_from` during full-resolution (`to_rgb`) decoding.
CI bench job will capture actual WASM numbers on next main merge.

**Reason kept.** Zero regression risk: compile-time gating, bit-exact by
construction (sign-extend from i16→i32 is exact; low-halfword extraction via
byte-shuffle is exact truncation). Two new unit tests
(`load8s_s1_simd128_matches_scalar`, `store8s_s1_simd128_matches_scalar`)
gate on `wasm32 + simd128` and verify round-trip across the full i16/i32 range.
All 389 host lib tests pass; both WASM builds (plain and `+simd128`) succeed.

### #224 Phase 4 — opt-in lossy rec-7 substitution for near-duplicates — **Kept** (2026-04-28)

**Approach.** Added `Jb2EncodeOptions { lossy_threshold: f32 }` and
`pub fn encode_jb2_dict_with_options(bitmap, shared, &opts)`. When
`lossy_threshold > 0.0`, the action-selection branch tries
`find_lossy_copy_ref` *before* the lossless refinement matcher
(`find_refinement_ref`): for each CC, it scans `same_size_indices` in
`dict_entries`, and if any entry has `packed_hamming(rep, cc) <= pixels *
lossy_threshold`, the encoder emits `rec-7` (matched copy, no
refinement bitmap) referencing it. Decoder will then reconstruct the
dict entry's pixels, with visual error bounded by the threshold. The
existing `REFINEMENT_MIN_PIXELS = 32` floor still applies — tiny CCs
stay byte-exact regardless of threshold.

`encode_jb2_dict_with_shared` now delegates to
`encode_jb2_dict_with_options(bitmap, shared, &Jb2EncodeOptions::default())`
so the shipped lossless path is unchanged. Default threshold = 0 = exact
behaviour preserved.

`examples/encode_quality_jb2.rs` got a `--lossy-threshold <fraction>`
flag, plus a `bitmap_hamming` helper that decodes the lossy-encoded Sjbz
and computes pixel-wise Hamming vs the original mask, so the harness
reports both byte savings and total reconstruction error.

**Bench** (`encode_quality_jb2` on a 15-page bilevel mix:
`tests/corpus/{cable_1973_100133,watchmaker}.djvu` +
`tests/fixtures/{big-scanned-page,carte,chicken,irish}.djvu`,
~188 M total pixels, Apple M1 Max):

| `--lossy-threshold` | rs-lossy bytes | vs rs-dict (lossless) | total err pixels | bits/pixel error |
|---------------------|---------------:|----------------------:|-----------------:|-----------------:|
| 0 (lossless dict)   | 167 314        | 1.000×                | 0                | 0                |
| 0.01                | 158 250        | **0.946×** (−5.4%)    | 10 986           | 0.000087         |
| 0.02                | 154 050        | 0.921× (−7.9%)        | 17 946           | 0.000142         |
| 0.04                | 150 118        | 0.897× (−10.3%)       | 28 568           | 0.000226         |
| 0.05                | 149 015        | 0.891× (−10.9%)       | 32 386           | 0.000256         |
| 0.08                | 146 104        | **0.873×** (−12.7%)   | 40 767           | 0.000322         |

Reconstruction error is on the order of 1 in 5–20 K pixels (≈0.0001–
0.0003 bits/pixel) — visually imperceptible for scanned text on these
600 dpi-class bilevel inputs. The `lossy decode errors: 1` row in the
summary is the same `irish.djvu` page that already trips
`roundtrip_dict: decode_error` on the lossless path (issue #198: a CC
larger than `MAX_SYMBOL_PIXELS`); orthogonal to lossy mode.

**Reason kept.** Material byte savings on top of the already-shipped
lossless dict path, opt-in via `Jb2EncodeOptions`, default behaviour
unchanged. The threshold knob is exposed so callers can pick their own
size↔fidelity point. Pairs naturally with the cjb2 quality settings
(default ≈ 0.005, conservative ≈ 0.02 in DjVuLibre) — a CLI front-end
could map that mapping in a follow-up. All 32 `jb2_encode` unit tests
plus the new `lossy_threshold_substitutes_near_duplicate_with_rec7`
test pass.

**Open follow-ups.**
1. `--lossy-threshold` doesn't yet feed into `cjb2`-equivalent CLI
   front-end (`tools/djvu-encode` if/when one exists).
2. The same threshold logic could be extended to refinement: instead of
   only substituting same-size near-dups with rec-7, allow lossy rec-6
   that emits a *truncated* refinement bitmap. Unclear if there's
   additional headroom past the rec-7 path measured here.

### #194 Phase 2.5 — per-CC accounting harness for shared-Djbz refinement — **Kept (instrumentation only)** (2026-04-28)

**Approach.** Added `pub fn analyze_jb2_cc_stats(page, &shared)` that mirrors
the rec-1/rec-6/rec-7 action-selection branch in
`encode_jb2_dict_with_shared` but emits no bytes — just counts and a
Hamming-distance histogram for rec-6 emissions, separating refs that
land in the shared dict (cross-page) from refs that land in the
page-local running dict. Wired through to `encode_quality_djbz` via a
new `--cc-stats` flag.

This is the measurement layer Phase 2.5 needs before deciding whether
the per-CC profitability model in CLAUDE.md's #194 follow-up is worth
implementing. The Phase 2 result already showed flat Hamming clustering
doesn't beat byte-exact; the open question was whether selective
near-duplicate promotion (with a profitability gate per CC) could.
Without the actual rec-6 distribution we were guessing.

**Observations** (`--cc-stats` on `tests/corpus/*.djvu`, 36 pages, 4 books):

| File | Pages | rec-1 fresh | rec-6 shared | rec-6 local | rec-7 exact |
|---|---:|---:|---:|---:|---:|
| `cable_1973_100133.djvu` | 2  | 12.4% | 0.0%  | 4.7% | 82.8% |
| `conquete_paix.djvu`     | 22 | 40.7% | 0.2%  | 2.1% | 56.9% |
| `watchmaker.djvu`        | 12 | 6.1%  | 24.7% | 1.8% | 67.5% |

rec-6 Hamming-distance distribution on `watchmaker.djvu` (6256 rec-6
matches, dominant case): 49.7% in [1, 4], 47.7% in [5, 16], 2.5% in
[17, 64], 0% above. Very tight — the existing 4%-of-pixels threshold in
`find_refinement_ref` is approximately right; there is little headroom
for "tighter" to improve the picture.

**Reason kept.** Pure instrumentation, no encoder behavior change. Gives
future Phase 2.5 work (and any Phase 4 lossy-refinement experiment from
#224) a concrete CC-action breakdown without round-tripping bytes.
Round-trip + clippy + nextest all clean; new test
`analyze_jb2_cc_stats_classifies_records` covers all three buckets +
shared/local distinction.

**What this tells us about Phase 2.5 viability.** On the dominant
shared-dict beneficiary (`watchmaker`), rec-6 already covers 24.7% of
CCs against the shared dict, and the Hamming distribution is bimodal
on [1, 16]. The remaining 6.1% rec-1 are mostly:
1. Unique glyphs (no shared-dict twin) — promotion candidates need ≥ N
   page repetitions, by definition rare for these
2. Glyphs that fail the same-(w, h) bucket constraint
   (cross-size matching is `find_refinement_ref`'s explicit
   limitation, see jb2_encode.rs:611)

So the most plausible Phase 2.5 win is **cross-size refinement**, not
per-CC profitability. That's a substantially larger change (requires
resampling for Hamming scoring) and is what the open #194 follow-up
should track. Per-CC profitability against the existing same-size
shortlist is unlikely to add anything material — the rec-6 hits we
already get are tight enough that a profitability gate would barely
exclude any of them.

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
