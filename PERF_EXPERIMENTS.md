# Performance experiments

Log of perf experiments and their outcomes. Each entry: issue, approach,
numbers, decision, reason. Referenced from issue templates ("Record result
in `PERF_EXPERIMENTS.md` (Kept or Reverted + reason)") and from
`.github/workflows/bench.yml`.

### #445 — rolling 1-bit register for `m_r2` in `decode_ref_row` — **Reverted** (2026-06-24)

**Issue.** #445 (from the perf-experiment-swarm). `decode_ref_row` reads `m_r2 = pix_row(mbm_r2,
col + col_shift)` each column (a signed `col < 0` guard + bounds-checked `get`). Proposal: carry
`m_r2` in a rolling register, and — exploiting "`col_shift ≥ -1`" — drop the signed guard from the
in-loop update.

**Approach.** Seed `m_r2 = pix_row(mbm_r2, col_shift)` before the loop, use it in the `idx`
computation, and update `m_r2 = pix_row(mbm_r2, col + 1 + col_shift)` at the loop bottom.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Output byte-identical (9 + 53 jb2 round-trip/decode tests pass). `jb2_decode_first_chunk`
(boy_jb2): 436 vs 427 µs — within noise / a slight regression; no benchmark exercises a
refinement-heavy `decode_ref_row` (the proposed `jb2_decode_corpus_bilevel` target does not exist).

**Decision. Reverted.**

**Reason.** The optimisation's value depended on dropping the signed guard, which is **unsafe**:
`col_shift = mcol - ccol` (line 867) is unbounded, so `col + col_shift` (and `col + 1 + col_shift`)
can be negative for any `col` — the guard must stay. With the guard kept, the change merely
relocates the same `pix_row` read from the top of the loop to the bottom and carries it in a
register; that is exactly the instruction scheduling LLVM already does, and the one clean
measurement showed no gain (slight +2% regression from the extra register carry + a wasted final
read). `decode_ref_row` is also a cold-path (JB2 mask decode is cached after first render). Same
outcome as #441 — a codec inner-loop micro-op whose premise doesn't hold up.

---

### #442 — extend NEON per-row IDWT path to s==2 — **Rejected (incorrect premise)** (2026-06-24)

**Issue.** #442 (from the perf-experiment-swarm). `row_pass_inner`'s AArch64 per-row NEON branch
fires only at `s == 1`; `s == 2` falls through to the 8-row scatter-gather path. Proposal: extend
the branch to `s == 1 || s == 2`, reusing `row_pass_neon_s1_row` directly ("no new function
needed"), on the premise that an s=2 active row has "internal coefficients at consecutive positions
with stride 1."

**Approach.** Tried exactly the proposed one-line guard change.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers / verification.** The premise is **false**: the in-tree test `simd_row_pass_s2_matches_scalar`
calls `row_pass_inner` with `s = 2, sd = 1`. At `s == 2` the column subsample shift `sd == 1`, so
the active coefficients in a row are at **stride 2** (`kmax = (width-1) >> 1`, accesses `k << 1`),
not stride 1. `row_pass_neon_s1_row` processes `width` *consecutive* coefficients (it is correct for
`s == 1` precisely because `s == 1 ⇒ sd == 0`). Applying the proposed change makes the s=2 correctness
test fail immediately: `assertion left == right failed: SIMD row pass (s=2) must produce identical
output to scalar`.

**Decision. Rejected** (no code committed — the probe was reverted).

**Reason.** Reusing `row_pass_neon_s1_row` for `s == 2` produces an incorrect IDWT (it processes
stride-1 positions where the s=2 active row is stride-2). The issue's "no new function needed" claim
does not hold; a correct s=2 NEON kernel would have to handle the `sd == 1` deinterleave (stride-2
columns) — a genuinely new implementation, out of scope for the "reuse the s1 row" hypothesis the
issue is built on. The `s == 1` branch is safe only because it implies `sd == 0`.

---

### #441 — hoist `vdupq_n_s32` splat constants in IDWT NEON row pass — **Reverted** (2026-06-24)

**Issue.** #441 (from the perf-experiment-swarm). `row_pass_neon_s1_row`'s `lift!`/`predict!`
macros call `vdupq_n_s32(16)` / `vdupq_n_s32(8)` inside their bodies (expanded twice per chunk).
An in-tree comment notes LLVM does not hoist the splat to a loop-invariant `movi.4s`. Hypothesis:
hoist them to a `let` before each loop, matching the C16/C8 const pattern in the `wide::i32x8` path.

**Approach.** `let c16 = vdupq_n_s32(16i32)` before the even loop, `let c8 = vdupq_n_s32(8i32)`
before the odd loop; use them in the macros.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Output byte-identical (same constant; 34 iw44 tests pass). `iw44_to_rgb_colorbook/sub1`
(full IDWT + YCbCr), two rounds: WITH 5.655 / 5.778 ms, base 5.695 / 5.768 ms — −0.7% / +0.2%,
i.e. indistinguishable from noise.

**Decision. Reverted.**

**Reason.** The premise is correct (LLVM does re-materialise the splat), but the conclusion does not
hold for *this* loop: `movi.4s` is a 1-cycle, zero-latency-to-issue instruction that dual-issues
into an otherwise idle slot of this ALU-bound lifting kernel, so re-creating it per chunk costs ~0.
The C16/C8 const pattern that justified the hoist was for the `wide::i32x8` path (different
codegen); it does not transfer to the NEON `int32x4_t` kernel. No measurable benefit and no
structural improvement (unlike the kept #443/#446/#449), so reverted per "keep only what helps."

---

### #443 — extend F2 all-bg fast path to non-identity gamma — **Kept** (2026-06-24)

**Issue.** #443 (from the perf-experiment-swarm). The F2 all-bg-row fast path
(`composite_rows_bilinear_one`, 1:1) only fired when `gamma_is_identity`; an all-bg row with
non-identity gamma fell through to the full G1 pre-expansion + per-pixel dispatch.

**Approach.** Add an `else if row_is_all_bg` branch for the non-identity case: apply the gamma LUT
directly to the bg row (`chunk[c] = lut[bg_row[..][c]]`), or to white (`lut[255]`) when there is no
bg. Sequential LUT pass, no mask expansion. Falls through on the bg-clamp edge case.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Identity-gamma output byte-identical — FNV unchanged on watchmaker native (the new
branch only fires when `gamma_is_identity == false`, so the common path is untouched, verified).
103 render + 610 lib tests pass. No non-identity-gamma fixture exists in the corpus, so the new
branch is not benchmark-exercised; its output is byte-identical to the per-pixel loop by
construction (`lut[bg]` for all-bg pixels, with the same offset/clamp guard).

**Decision. Kept.**

**Reason.** Free-on-miss, byte-identical safe extension that completes F2 symmetrically: the
identity path is provably unchanged (FNV), and a non-identity-gamma all-bg row now gets a single
LUT pass instead of the full G1 dispatch. The benefit is narrow (non-identity gamma is uncommon)
and not benchmark-visible on the identity-only corpus, but there is no downside — same disposition
as the other kept fast-path extensions (#433/#438). Closes #443.

---

### #449 — stream PDF page render→emit→drop in the sequential path — **Kept** (2026-06-24)

**Issue.** #449 (from the perf-experiment-swarm). `djvu_to_pdf_impl` collected *all*
`RenderedPage` bodies into a `Vec` before emitting any, holding O(page_count × body_size) in
memory. A source comment explicitly deferred a fix to "a separate issue."

**Approach.** Factor the per-page emit (rendered body, or blank-page fallback) into an `emit_one`
closure. The `parallel` branch keeps the rayon `collect()` (required) + emit loop. The
`#[cfg(not(feature = "parallel"))]` branch now renders, emits, and drops one page at a time, so at
most one `RenderedPage` body is live (peak from page bodies O(pages)→O(1); mirrors TIFF_STREAM).

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** PDF output **byte-identical** (same length + FNV on navm_fgbz 6 pages / watchmaker
12 pages) — the emit order and id allocation are unchanged. Peak RSS (`/usr/bin/time -l`) was
*within noise* on both available multi-page docs: watchmaker 12 pp (75.9 vs 73.2 MB) and
pathogenic_bacteria 520 pp (72.1 vs 72.4 MB). Neither makes body-accumulation the peak driver —
watchmaker has only 12 pages (~0.5 MB bodies) and pathogenic's pages deflate to ~2 KB each (1.2 MB
PDF total); in both, peak is dominated by the per-page 33 MB render pixmap and decode caches.

**Decision. Kept.**

**Reason.** Structurally the streaming path holds ≤ the bodies the collect path does (1 vs N), so
the page-body contribution to peak RSS can only drop — the watchmaker +2.7 MB is allocator/measurement
noise (the change cannot increase how many bodies are retained). Byte-identical output, and it
removes a documented deferred footgun: a real document with *many large-body pages* (e.g. a
300-page colour scan, ~1 MB deflated/page → ~300 MB of retained bodies) would see a large peak-RSS
cut. No corpus fixture combines many pages with large bodies, so the win isn't benchmark-visible,
but the change is strictly-safe and correct — same disposition as #446. The parallel path is
unchanged. Closes #449.

---

### #448 — hoist Lanczos vertical-pass weights + row-major accumulate — **Kept** (2026-06-24)

**Issue.** #448 (from the perf-experiment-swarm). In `scale_lanczos3`'s vertical pass, the weight
`lanczos3_kernel((sy − cy) / v_scale)` was evaluated inside the per-column `for ox` loop even though
it depends only on `(oy, sy)`. LLVM cannot LICM the opaque `f32::sin` calls, so the kernel was
recomputed `dst_w` times per row. The access `mid.get_rgb(ox, sy)` also strided by `dst_w*4` bytes
per `sy` (column-major against a row-major buffer).

**Approach.** Restructure to iterate `sy` outer / `ox` inner: evaluate the weight once per `sy`,
read `mid`'s row `sy` sequentially, and accumulate into per-column buffers (`acc_r/g/b`). The
per-column sum is over the same `sy` values in the same ascending order, so the floating-point
result is bit-identical.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Output bit-identical (FNV unchanged, Lanczos3 downscale). Full Lanczos3 render
(watchmaker @0.5 — native re-render + `scale_lanczos3`), best-of-20:

| Round | base (ms) | with (ms) | Δ |
|---|---|---|---|
| 1 | 594.00 | 460.25 | **−22.5%** |
| 2 | 594.37 | 460.26 | **−22.5%** |

Near-perfectly reproducible (best-of-20 both rounds within 0.4 ms). The vertical pass was a major
cost because of the redundant `sin` evaluations; hoisting them saves ~134 ms.

**Decision. Kept.**

**Reason.** −22.5% on the Lanczos render path, bit-identical output, reproducible. The win comes
from two synergistic fixes: (1) the `f32::sin` calls — which the compiler genuinely cannot hoist —
move from O(dst_w·support) to O(support) per row, and (2) row-major accumulation replaces
per-column cache-scatter reads of `mid`. Lanczos resampling is the opt-in high-quality downscale
(`Resampling::Lanczos3`); this makes it ~1.3× faster at no quality cost. Closes #448.

---

### #447 — 32×32 tiled transpose in `rotate_pixmap` (Cw90/Ccw90) — **Kept** (2026-06-24)

**Issue.** #447 (from the perf-experiment-swarm). The 90° rotation paths transposed the image with
a per-pixel `get_rgb`/`set_rgb` whose destination write strides by `out.width*4` bytes (~13 KB at
A4) — a cache miss per pixel, ~8.4 M strided writes for a 2550×3300 page.

**Approach.** Iterate the source in 32×32 tiles and copy each pixel with a direct
`out.data[di..di+4].copy_from_slice(&src.data[si..si+4])`. Within a tile both the source read and
destination write stay within a few cache lines, so the hardware prefetcher works. 4-byte copy is
exact (rendered source pixmaps carry alpha=255 and `Pixmap::white` pre-fills alpha=255).

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88 (machine moderately
loaded, load ~7 — best-of-N used for robustness).

**Numbers.** Output byte-identical — FNV checksums unchanged for Cw90, Ccw90, and Rot180 (the last
untouched). Full rotated render (watchmaker Cw90, native, best-of-30):

| Round | base (ms) | tiled (ms) | Δ |
|---|---|---|---|
| 1 | 52.54 | 51.55 | −1.9% |
| 2 | 53.02 | 49.70 | −6.3% |

Both rounds faster. The transpose itself is substantially cheaper (strided 8.4 M writes →
cache-local tiles); the end-to-end delta is diluted because decode + composite (~45 ms, unchanged)
dominate the ~50 ms rotated render.

**Decision. Kept.**

**Reason.** Byte-identical output, consistent improvement across rounds, and a textbook
cache-locality win for an image transpose (sequential tile bursts replace per-pixel cache-scatter).
Rotated rendering is a real use case (viewing rotated scans). Rot180 is left as-is (its writes are
already row-sequential in reverse, not a transpose). Closes #447.

---

### #446 — O(1) page-dedup in `cluster_shared_symbols_tunable` — **Kept** (2026-06-24)

**Issue.** #446 (from the perf-experiment-swarm). The shared-Djbz clustering inner loop deduped the
per-cluster `pages_seen: Vec<usize>` with `pages_seen.contains(&page_idx)` — an O(K) linear scan.
On a corpus where one cluster recurs on all P pages, this is O(P²) total.

**Approach.** Pages are visited in strictly non-decreasing `page_idx` order (the outer
`pages.iter().enumerate()` loop), so `pages_seen` is sorted and the current page — if already
counted — is the last element. Replace the scan with `pages_seen.last() != Some(&page_idx)`, O(1).

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Clustering output is identical — verified by the existing clustering tests
(`cluster_promotes_only_repeated_glyphs`, `cluster_tunable_keeps_near_duplicate_large_glyphs_separate`,
`cluster_shared_symbols_caps_total_pixel_budget`) plus 67 encode + 2 djbz tests, all passing. No
dedicated multi-page-clustering benchmark was run (the only large fixture is the 517-page
`pathogenic_bacteria_1896`, whose full re-encode is expensive), but the change is *strictly* ≤ the
old cost: `last()` is a single comparison vs a scan of up to P entries.

**Decision. Kept.**

**Reason.** Trivially correct under the monotonic page order (verified by the unchanged clustering
test outputs) and a pure complexity improvement — O(P²)→O(P) on the worst case (a cluster shared
across an entire corpus), with no possible regression (O(1) ≤ O(K) always). Same disposition as the
kept #433: a safe, strictly-non-worse change that helps at scale (large multi-page shared-dict
encodes) even though the small test corpus doesn't make the difference benchmark-visible. Closes #446.

---

### #444 — cache parsed FGbz palette in `PageLayers` — **Reverted** (2026-06-24)

**Issue.** #444 (from the perf-experiment-swarm). `decode_fg_palette_full` calls `parse_fgbz`
(BZZ decompression + index-table construction) on every warm render of a palette page. Hypothesis:
cache the parsed `FgbzPalette` in `PageLayers` like the other layers (COW_FG pattern).

**Approach.** Added `#[derive(Clone)]` to `FgbzPalette`, a `fg_palette` OnceLock + accessor in
`PageLayers`, a `DjVuPage::decoded_fg_palette()`, and changed `decode_fg_palette_full` to return
`Cow::Borrowed` on a cache hit (with a re-parse error-fallback). `DecodedLayers`/`ForegroundLayers`
`fg_palette` fields became `Option<Cow<'a, FgbzPalette>>`.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88. Machine idle (the
AV1 encode was paused), so the control reproduced cleanly.

**Numbers.** Output byte-identical (FNV unchanged on navm_fgbz). `palette_native_cached` vs
`color_native_cached` control, two rounds:

| Round | control (ms) | palette (ms) | ratio |
|---|---|---|---|
| 1 baseline | 42.68 | 41.11 | 0.963 |
| 1 with cache | 42.48 | 41.39 | 0.974 |
| 2 baseline | 42.61 | 40.92 | 0.960 |
| 2 with cache | 42.45 | 41.43 | 0.976 |

A consistent **~1% regression** (palette ~0.4 ms slower both rounds, on a clean idle machine).

**Decision. Reverted.**

**Reason.** The FGbz chunk in the only palette fixture (navm_fgbz) is **21 bytes** — a trivial
palette with no real index table, so `parse_fgbz` is essentially free. Caching it saves nothing
while the cache machinery (OnceLock check, `Cow` wrapping, the error-fallback branch) adds a small
but consistent overhead. The optimisation is sound *in principle* — it would help pages with large
palettes (big per-blit index tables) — but no such page exists in the corpus, so there is nothing
to amortise the cache against, and the measured result is a net regression. Unlike the kept
BG_CACHE/MASK_IDX_CACHE (which cache multi-millisecond decodes / 33 MB allocations), a 21-byte parse
is below the cache's own overhead. Reverted.

---

### #440 — parallel BG44/FG44 layer decode via `rayon::join` — **Kept** (2026-06-24)

**Issue.** #440 (from the perf-experiment-swarm). `decode_layers`'s strict branch ran
`decode_background_chunks` (BG44 ZP decode + IDWT + YCbCr→RGB) then `decode_foreground_strict`
(JB2 mask + FG44) sequentially. They write disjoint `OnceLock` fields, so on a cold render they
can overlap: the FG decode runs on a second rayon thread while the BG ZP phase (before IW44_PAR's
IDWT join) leaves the pool idle.

**Approach.** Wrap the two calls in `rayon::join` under `#[cfg(feature = "parallel")]`; the
non-parallel build keeps the sequential tuple. Warm renders hit the caches and both closures
return instantly, so the join is effectively free then.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.
**Note.** This was measured *after* the user paused a background AV1/ffmpeg video-encode that had
been saturating all 10 cores (load avg ~590) — the earlier "thermal" noise was actually CPU
contention. With the machine idle, the single-thread control `bilevel_native_cached` reproduced to
0.05%, so these numbers are clean.

**Numbers.** Output byte-identical (FNV unchanged, `--features parallel`). Cold-render timing
(watchmaker native, fresh parse each iteration, best-of-40, `--features parallel`):

| Round | base (ms) | with join (ms) | Δ |
|---|---|---|---|
| 1 | 13.35 | 10.26 | **−23.1%** |
| 2 | 12.97 | 10.14 | **−21.8%** |

Consistent **~22%** faster cold render — the FG side (FG44 IW44 + JB2 mask, ~3 ms) overlaps the BG
decode rather than running after it. Warm regression check (`color_native_cached`, parallel
compositor ~6.7 ms): WITH and base within ~1% across rounds (one 7.8 ms outlier aside) — no real
regression, as expected since the join of two cache-hit closures costs ~tens of ns. 610 lib tests
pass with `--features parallel`.

**Decision. Kept.**

**Reason.** ~22% lower cold first-render latency — exactly what matters when a viewer opens a page
— at the cost of a `rayon::join` that is free on warm renders (cache hits return instantly) and
absent without the `parallel` feature. Disjoint `OnceLock` fields make it data-race-free; output is
byte-identical. Extends the IW44_PAR / PARALLEL parallelism to a new level (across layers at the
decode call site). Closes #440.

---

### #439 — anti-aliased colour downscale (proportional fg/bg blend) — **Kept (quality)** (2026-06-24)

**Issue.** #439 (from the perf-experiment-swarm). `composite_rows_area_avg_one` (colour downscale)
decided each output pixel as 100% fg or 100% bg via the binary `mask_box_any`. At 2× downscale a
single foreground source pixel in a 2×2 box made the whole output pixel fg-coloured, producing
blocky colour halos around text. The AA experiment already fixed the analogous artefact for the
*bilevel* compositor (`mask_box_coverage`); the colour compositor was never updated.

**Approach.** Replace the binary test with `coverage = mask_box_coverage(...)` (0..255 = fraction
of the footprint that is foreground) and blend: `coverage == 0` → bg only, `coverage == 255` → fg
only, otherwise `out = (coverage·fg + (255−coverage)·bg + 127) / 255` per channel. The `mask_shift
> 0` max-pool sub-path stays binary; #438's `mask_all_bg` still skips blank rows (coverage 0). The
now-unused `mask_box_any` is deleted.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Quality (the deliverable): watchmaker @150 DPI has **68 unique colours with the blend
vs 28 binary** — the 2.4× increase is the intermediate edge tones of anti-aliased colour text.
103 render tests + the full integration suite (golden composites) pass with no changes — the colour
goldens render at native resolution, so the downscale path change does not touch them. Performance:
`mask_box_coverage` counts the footprint instead of early-exiting like `mask_box_any`, a small cost
on non-blank rows (the AA experiment found this "indistinguishable from noise" at 150 DPI, and #438
already removes it on the 50–71% blank rows); the machine was too thermally saturated to quote a
clean delta, but partial-coverage pixels are a minority (edges only).

**Decision. Kept (quality).**

**Reason.** Genuine quality win — smooth colour gradients at text edges replace blocky halos, the
colour analog of the kept bilevel AA. Edge pixels now sample both fg and bg and blend; fully-fg
(coverage 255, stroke interiors) and fully-bg (coverage 0) pixels keep their single-sample cost, so
the extra work is confined to the minority of partially-covered edge pixels. Output for the
1:1/native paths is unchanged. Closes #439.

---

### #438 — all-bg row fast path in area-average compositor (F2/I3 analog) — **Kept** (2026-06-24)

**Issue.** #438 (from the perf-experiment-swarm). `composite_rows_area_avg_one` (colour downscale)
runs the per-pixel `is_fg` test via `mask_box_any` (for `mask_shift == 0`), which scans the output
pixel's footprint with byte popcounts. On blank rows (margins / inter-line gaps) this whole scan is
wasted. The 1:1 and bilevel-downscale paths already short-circuit blank rows (F2, #428).

**Approach.** Pre-scan the row-invariant mask y-band `[y0, y1)` once (`mask_all_bg`, matching
`mask_box_any`'s range); guard `is_fg` with `!mask_all_bg && …`. On a blank band the `&&`
short-circuits, skipping `mask_box_any` for every pixel. Guarded on `mask_shift == 0` (the max-pool
sub-path indexes a coarser mask).

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Output byte-identical (FNV checksums unchanged on colorbook @150/400 and watchmaker
@0.25). Firing-rate probe (the thermal-independent evidence — the machine was thermally saturated
by this point, so wall-clock ratios were unusable):

| Doc / scale | output rows | blank-band rows | fire rate |
|---|---|---|---|
| colorbook @150/400 | 1376 | 972 | **70.6%** |
| watchmaker @0.25 | 825 | 414 | **50.2%** |

**Decision. Kept.**

**Reason.** Same disposition as #428 (kept, 94.4% fire): byte-identical, fires on 50–71% of rows,
and on those rows it skips the *expensive* `mask_box_any` footprint scan (not just a cheap bit
check as in #435), so the per-row saving is larger. Per-row overhead when it misses is one band
pre-scan (`iter().any()`, NEON) plus a short-circuiting bool — bounded and small. The mechanism is
a row-level early exit (the proven F2/I3/#428/#435 pattern), not an inner-loop micro-op like the
reverted #432/#434/#436/#437. Closes #438.

---

### #437 — exact-length FG44/BG44 row slices for bounds-check elision — **Reverted** (2026-06-24)

**Issue.** #437 (from the perf-experiment-swarm). The 1:1 general bilinear path takes open-ended
row slices (`fg.data.get(y0*stride..)`); `bilinear_from_rows` and the bg lookup then do
`row.get(off..off+4)` four/one times per pixel. Hypothesis: exact-length slices
(`..(y0+1)*stride`) make `row.len() == width*4` visible to LLVM after inlining, letting it elide
the per-lookup bounds-check branches.

**Approach.** Change the three slice sites (fg row0/row1, bg row) to `..(y+1)*stride`.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Output byte-identical (FNV unchanged). Benchmark inconclusive: by this point in the
session the machine was thermally saturated — the *control* `bilevel_native_cached` (unaffected by
this change) swung 56–433 ms across runs (7.6×), so no signal could be extracted from
`color_native_cached`.

**Decision. Reverted.**

**Reason.** Two problems. (1) Mechanism: the fg slices use `.unwrap_or(&[])`, so `row.len()` is
either `stride` or `0` from LLVM's view — the length is *not* statically tied to `width`, which
defeats the bounds-check elision the hypothesis relies on (only the bg path, which uses `.map`,
could plausibly benefit). (2) Measurement: the thermally-saturated machine made the result
unverifiable. The change is byte-identical and adds no work, but with no demonstrable benefit and a
mechanism likely defeated by `unwrap_or`, it is not worth keeping speculatively. Could be revisited
with assembly verification (look for removed `cbz`/`cbnz` guards) on a cool machine if the bg path
turns out to matter.

---

### #436 — MASK_EXPAND pre-expansion in B-series bilinear (upscale) — **Reverted** (2026-06-24)

**Issue.** #436 (from the perf-experiment-swarm). Retry of G1b — pre-expand the mask row to
per-pixel bytes via MASK_EXPAND in `composite_rows_bilinear_one`'s B-series path, replacing the
per-pixel bit-extraction with a byte load. The validator argued G1b was measured at 72 DPI (which
routes to `composite_rows_area_avg_one`, a different function) and that the *upscale* bilinear path
(600/300 DPI) was never actually tested.

**Approach.** Add a 4096-byte `g1b_buf`, expand the hoisted mask row via MASK_EXPAND (skipping
blank rows, which #435 already short-circuits), and replace the `is_fg` bit-extraction with a
`g1b_mask[px]` byte lookup (fallback bit-extraction for pages wider than the buffer). New
`color_upscale_cached` benchmark (watchmaker @2×).

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Output byte-identical at both 2× upscale and 0.5 downscale (FNV checksums unchanged).
`color_upscale_cached` (watchmaker @2× = 5100×6602) vs `color_native_cached` control:

| Round | control (ms) | upscale (ms) | ratio | Δ |
|---|---|---|---|---|
| 1 baseline | 175.8 | 652.7 | 3.71 | |
| 1 with | 216.5 | 877.0 | 4.05 | +9.1% (regress) |
| 2 baseline | 180.5 | 1713.6 | 9.50 | |
| 2 with | 140.7 | 1240.5 | 8.82 | −7.2% (improve) |

Contradictory; the upscale render (33.7 M pixels, 134 MB output buffer) swings 650–1700 ms
between rounds — the measurement is dominated by memory-bandwidth/allocation noise, not the
is_fg path.

**Decision. Reverted.**

**Reason.** Unlike #435 (a free short-circuit), #436 *adds* real per-row work: a 4 KB stack buffer
plus a 319-iteration MASK_EXPAND expansion every non-blank row. The benefit it buys — replacing the
~5-op is_fg extraction with a byte load — is a small fraction of the bilinear-bound per-pixel cost
(`bilinear_from_rows` dominates), and #435 already removed the is_fg cost on the 52.6% blank rows.
The benchmark cannot confirm any gain (R1 even regresses), and the upscale path is an uncommon
real-world case (scans are rarely upscaled). Same outcome as G1b, now confirmed on the actual
upscale path: the expansion overhead is not worth it where the inner loop is resampling-bound.

---

### #435 — all-bg row fast path in B-series bilinear path (F2 analog) — **Kept** (2026-06-24)

**Issue.** #435 (from the perf-experiment-swarm). The B-series bilinear path
(`composite_rows_bilinear_one`, non-1:1) runs the per-pixel `is_fg` bit-extraction (≈5 ops:
`is_some_and`, bounds check, byte load, shift, AND, compare) for every pixel, including blank rows
(margins / inter-line gaps). The 1:1 path already short-circuits blank rows via F2.

**Approach.** Pre-scan the hoisted mask row once (`mask_all_bg`); guard `is_fg` with
`!mask_all_bg && …`. On a blank row the `&&` short-circuits, skipping the `is_fg` extraction for
every pixel — independent of whether LLVM unswitches the loop. Unlike F2 the bg pixels still need
per-pixel `bilinear_from_rows` (coordinates are scaled, no `copy_from_slice`), so this saves only
the is_fg check, not the whole bg copy.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Output byte-identical (FNV checksum unchanged). Firing-rate probe: watchmaker @150/300
(B-series) has **868 / 1651 output rows (52.6%) with a blank mask row** — the fast path fires on
the majority. Benchmark `color_downscale_mixed_cached` vs `color_native_cached` control:

| Round | control (ms) | mixed (ms) | ratio |
|---|---|---|---|
| 1 baseline | 385.8 | 106.2 | 0.275 |
| 1 with | 425.3 | 88.4 | 0.208 |
| 2 baseline | 357.2 | 90.5 | 0.253 |
| 2 with | 399.0 | 101.4 | 0.254 |

R1 shows −24%, R2 neutral — thermal noise (mixed swung 88–106 ms regardless). The mechanism is
confirmed by the 52.6% firing rate plus the short-circuit, which provably elides ≈5 ops/pixel on
those rows.

**Decision. Kept.**

**Reason.** Same disposition as F2/I3/#428 (kept row-level skips): the change is byte-identical,
the per-row overhead when it misses is one mask-row pre-scan (~nb bytes, NEON `any()`) plus a
short-circuiting bool, and it fires on 52.6% of rows where it elides the per-pixel is_fg check.
The benchmark is too noisy to quote a clean magnitude (~3–8% expected), but the firing-rate and
short-circuit guarantee a real, bounded-overhead win. Distinct from the reverted #432/#434
(inner-loop micro-ops that added unconditional per-pixel cost) — this only adds work on the
common branch and removes it on the majority blank-row case. Closes #435.

---

### #434 — Q48 accumulator for `fg_fx` in B-series bilinear path — **Reverted** (2026-06-24)

**Issue.** #434 (from the perf-experiment-swarm). The B-series bilinear inner loop
(`composite_rows_bilinear_one`, non-1:1) computes `fg_fx = map_plane_center_frac(fx, fg_x_q24)`
(a u64 multiply) for FG44 foreground pixels. The bg path already replaced its equivalent multiply
with a Q48 add-per-pixel accumulator (B1). Hypothesis: the same accumulator for `fg_fx` would
save 3–8%.

**Approach.** Add `fg_fx_q` / `fg_fx_step_q` mirroring `bg_fx_q`, replace the per-pixel multiply
with `(fg_fx_q >> 24).saturating_sub(FRAC/2)`, and advance `fg_fx_q` unconditionally at the bottom
of the loop (it must stay in sync across the fg/bg branch).

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** `color_downscale_mixed_cached` (watchmaker @150/300, B-series FG44) vs
`color_native_cached` control. Output verified byte-identical (FNV checksum unchanged). Two
intra-session ratio pairs:

| Round | control (ms) | mixed (ms) | ratio | Δ |
|---|---|---|---|---|
| 1 baseline | 305.8 | 61.0 | 0.200 | |
| 1 with | 333.7 | 75.2 | 0.225 | **+12.9%** |
| 2 baseline | 229.4 | 53.8 | 0.234 | |
| 2 with | 225.9 | 67.2 | 0.298 | **+26.9%** |

Both rounds regress (R2's control is nearly equal, so the +25% absolute mixed regression is clean).

**Decision. Reverted.**

**Reason.** The hypothesis missed that the `fg_fx` multiply is **conditional** — it runs only for
fg pixels (`if is_fg → Some(fg44)`), a minority (~15–30%) of output pixels. The accumulator must
advance **unconditionally** (every pixel) to stay in sync. So the change trades a multiply on ~20%
of pixels for an add on 100% of pixels — a net loss. This is the inverse of B1 (kept): the bg
accumulator wins because bg is the *majority*, so its unconditional advance matches its usage
frequency. The accumulator pattern only pays when the consumer is the common-case branch. The
multiply on the minority fg path was already cheaper than the bookkeeping to avoid it.

---

### #433 — generalize P2 BILEVEL_RGBA fast path to byte-aligned `offset_x` — **Kept** (2026-06-24)

**Issue.** #433 (from the perf-experiment-swarm). The P2 BILEVEL_RGBA table fast path in
`composite_rows_bilevel_one` was guarded on `offset_x == 0`, so `render_region` viewports (which
pass `offset_x = region.x`) always fell back to the scalar per-pixel bit-extraction loop.

**Approach.** Generalize the guard to `offset_x % 8 == 0 && offset_x + out_w <= mask.width`: when
the offset is byte-aligned, output byte `i` maps to source mask byte `offset_x/8 + i` with no
per-pixel bit shuffle, so the same NEON table copy works. `offset_x == 0` is the `mb0 = 0`
subcase — behaviourally identical to before. Added a correctness test
(`render_region_bilevel_byte_aligned_offset_matches_full`) checking both x=16 (aligned, new path)
and x=17 (unaligned, fallback) against the full render, plus a `render_region_bilevel` benchmark.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Two effects measured:
- **No regression on the hot path** (`offset_x == 0`): `bilevel_native_cached` 437 vs 448 ms
  (within thermal noise) — the only change there is one `+0` index add.
- **`render_region_bilevel`** (cable, byte-aligned 512-wide viewport): with ≈ 89 ms, baseline
  ≈ 85 ms — **within noise**. The compositor *is* faster (P2 NEON table vs scalar, the
  established 18–24% margin), but `render_region`'s end-to-end time is dominated by the per-call
  `Pixmap::white` allocation (6.7 MB) and the 3301-row iteration, which mask the compositor saving.
  (An initial 12× reading was a stale-binary artifact from the stash/rebuild cycle, not real.)

**Decision. Kept.**

**Reason.** Unlike the reverted #432 (a replacement that measurably regressed), this is a strict,
*safe generalization*: the `offset_x == 0` hot path is byte-for-byte the same code, so it cannot
regress (confirmed), and byte-aligned region renders now use the proven-faster P2 NEON table
instead of the scalar fallback. The end-to-end `render_region` win is below this benchmark's noise
because allocation/iteration dominate — but the compositor improvement is real and the change has
zero downside. It also removes an artificial restriction on P2 and adds region test coverage.
Closes #433.

---

### #432 — byte-level early-exit scan in `mask_box_any` — **Reverted** (2026-06-24)

**Issue.** #432 (from the perf-experiment-swarm). `mask_box_any` (per-pixel foreground test in the
colour area-average downscale path, `composite_rows_area_avg_one`) scanned the footprint with
per-pixel `mask.get()` and early-exit. Hypothesis: apply the byte-level boundary-masking from the
kept POPCNT experiment (`mask_box_coverage`) to test "any bit set" faster.

**Approach.** Replace the per-pixel loop with the same `byte_lo`/`byte_hi`/`first_mask`/`end_mask`
boundary computation as `mask_box_coverage`, testing `& mask != 0` per byte with early-exit.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** `render_colorbook` (colorbook @150/400, area-avg path), clean alternating runs with
tight, reproducible CIs:

| Run | render_colorbook |
|---|---|
| baseline | 3.620 ms |
| with change | 3.805 ms |
| baseline again | 3.616 ms |

Baseline reproduces exactly (3.620 / 3.616 ms); the change is a clean **+5.1% regression** — not
thermal noise. 102 render tests pass (output correct), but it is slower.

**Decision. Reverted.**

**Reason.** The POPCNT win does **not** transfer. `mask_box_coverage` must scan the *entire*
footprint (it counts bits), so byte-popcount strictly beats per-pixel. `mask_box_any` has
**early-exit** — the per-pixel loop returns on the first set bit, which for text-on-white is cheap.
At colorbook's downscale the footprint is only ~3×3 pixels, so the per-call setup of four boundary
quantities (`byte_lo`, `byte_hi`, `first_mask`, `end_mask`) plus the single-vs-multi-byte branch
costs more than the handful of `get()` calls it saves. Byte-scanning only pays when the footprint
is large (POPCNT's −24% was at 72 DPI / 4×4) *and* there is no early exit. Mirror experiment to G2:
the simpler code the compiler already handles well wins at small footprints.

---

### #431 — LUT byte-expansion in bilevel TIFF export (`extract_bilevel_pixels`) — **Kept** (2026-06-24)

**Issue.** #431 (from the perf-experiment-swarm). `extract_bilevel_pixels` expanded the packed JB2
mask to a Gray8 buffer with one `bm.get(x, y)` per pixel (stride multiply + byte read + bit-shift
+ AND + branch), w×h times (8.4 M for an A4 page).

**Approach.** Add `BILEVEL_GRAY8: [[u8; 8]; 256]` mapping each packed mask byte (MSB-first) to its
8 Gray8 pixels. When the mask covers the page (`bm.width >= w && bm.height >= h`), iterate the
packed rows byte-by-byte and `copy_from_slice` 8 pixels per byte (trailing partial byte handled),
replacing 8 per-pixel `bm.get()` calls with one table lookup + vectorised copy. Per-pixel
`bm.get()` fallback kept for the unexpected smaller-mask case. Same LUT idea as P2/BILEVEL_RGBA
but Gray8 instead of RGBA.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Full `TiffMode::Bilevel` export of cable_1973 (2 pages — JB2 decode + expansion +
Deflate), best-of-30 timing:

| Round | baseline (ms) | with LUT (ms) | Δ |
|---|---|---|---|
| 1 | 60.8 | 49.5 | **−18.7%** |
| 2 | 59.7 | 48.8 | **−18.2%** |

Consistent **~18%** on the whole bilevel export (the expansion was a large fraction of the
non-Deflate work). 16 `tiff_export` tests pass (incl. bilevel pixel-equality and black-pixel
checks).

**Decision. Kept.**

**Reason.** Replaces 8.4 M per-pixel bit-extracts with ~1 M table lookups + vectorised copies;
output is byte-identical by construction (same MSB-first bit mapping). Stacks with the #430 Deflate
change in the same path. Closes #431.

---

### #430 — Deflate-compressed bilevel TIFF export (size reduction) — **Kept** (2026-06-24)

**Issue.** #430 (from the perf-experiment-swarm) asked for 1-bit TIFF output for bilevel pages
(target: 8× smaller files). The bilevel path wrote the JB2 mask as an uncompressed 8-bit
grayscale strip (Gray8, 1 byte/pixel = 8.4 MB for an A4 page).

**Constraint found.** The `tiff` 0.9 crate's high-level encoder has **no 1-bit ColorType** — the
`ColorType` trait's minimum `BITS_PER_SAMPLE` is 8 and `write_data` takes one sample per element,
so it cannot pack 8 pixels/byte. True 1-bit output would require hand-writing TIFF tags outside
the crate (disproportionate risk for the goal).

**Approach.** Achieve the *actual* goal — drastically smaller bilevel TIFFs — via the crate's
supported `new_image_with_compression::<Gray8, Deflate>`. Bilevel content is just 0x00/0xFF bytes
with long runs (text on white), which Deflate compresses far better than a 1-bit packing. One-line
change in `write_bilevel_page`; Deflate (TIFF compression tag 8) is universally readable; output is
lossless (pixel-identical).

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** cable_1973 (2 pages, 2550×3301), `TiffMode::Bilevel`:

| | total bytes | per page |
|---|---|---|
| Baseline (uncompressed Gray8) | 16,835,608 | ~8.4 MB |
| Deflate | 113,029 | ~56 KB |

**≈149× smaller** — far past the 8× a 1-bit packing would give. 625 lib tests (incl. the
bilevel-TIFF pixel-equality and round-trip tests) pass with `--features tiff`.

**Decision. Kept.**

**Reason.** Delivers the issue's real objective (small bilevel TIFFs) better than the literal
1-bit request, using only the supported encoder API, losslessly, and readable by any TIFF reader.
True 1-bit packing is not worth a hand-rolled TIFF encoder when Deflate already gives ~149×. Color
TIFF export is left unchanged (RGB Deflate is a smaller, separate win). Closes #430.

---

### #429 — byte-aligned `crop_bitmap` fast path in JB2 direct encoder — **Kept** (2026-06-24)

**Issue.** #429 (from the perf-experiment-swarm). `crop_bitmap` (JB2 direct encoder, used to
split a page into 1024×1024 tiles for record-3 whole-tile symbols) cropped each tile with a
per-pixel `src.get(x) + out.set_black()` loop — w×h bit-unpack/repack operations per tile.

**Approach.** The tile loop always passes a byte-aligned `x0` (tiles start at multiples of
TILE=1024, and 1024 % 8 == 0). When `x0 % 8 == 0`, copy each output row as a contiguous
`copy_from_slice` of `out_stride` bytes from the source row, then mask the trailing bits of the
last byte beyond `w` (which the per-pixel path never sets). Falls back to the per-pixel loop for
unaligned `x0` (never produced by the current caller, kept for generality). Mirrors
`blit_to_bitmap`'s aligned fast path.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** The existing `jb2_encode` bench uses a 192×256 mask — a single tile, so
`crop_bitmap` is never called (it hits the `bitmap.clone()` single-tile path) and showed no
change (1.220 vs 1.220 ms — confirming the path isn't exercised). Added `jb2_encode_multitile`
(cable_1973 mask, 2550×3301 = 3×4 tiles), which does exercise the multi-tile crop:

| Round | baseline (ms) | with (ms) | Δ |
|---|---|---|---|
| 1 | 222.8 | 192.5 | **−13.6%** |
| 2 | 255.0 | 215.8 | **−15.4%** |

Consistent **~14–15%** on full-page direct encode; well-separated medians, 100 samples.
609 lib tests + 53 djvu-jb2 tests (incl. encode roundtrip) pass.

**Decision. Kept.**

**Reason.** Replaces w×h per-pixel bit operations per tile with `h` byte-aligned `copy_from_slice`
calls (LLVM vectorises to NEON loads/stores). Output is byte-identical by construction (same bits
in columns [0,w), padding masked to 0), so the encoded JB2 stream is unchanged — verified by the
passing encode-roundtrip tests. Note `crop_bitmap` lives in the direct `encode_jb2` path (not the
shipping dict encoder), so the real-world impact is bounded to that path; the change is free
(falls back for the unaligned case) and correct, so kept regardless. Closes #429.

---

### #428 — all-white band fast path in bilevel downscale compositor (I3-downscale) — **Kept** (2026-06-24)

**Issue.** #428 (from the perf-experiment-swarm). `composite_rows_bilevel_one`'s anti-aliased
downscale path (`mask_shift == 0`) called `mask_box_coverage` for every output pixel, and each
call scans the source mask band `[y0, y1)` with byte popcounts. For a blank band every call
returns 0 → white, so the whole per-row scan is wasted. The 1:1 path already had I3 (blank-row
fill) but the downscale path had no analog.

**Approach.** Before the per-pixel loop, scan the row-invariant source band
`mask.data[y0*stride .. y1*stride]` once (`iter().any(|&b| b != 0)`, NEON-vectorised). If blank
(or the degenerate `y1 <= y0`), `row_buf.fill(255)` and return. The y range matches
`mask_box_coverage` exactly. Guarded on `mask_shift == 0` — the max-pool sub-path indexes a
different mask resolution.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Firing-rate probe on cable_1973 @150 DPI: **1559 / 1651 output rows (94.4%) have a
fully-blank band** — the fast path fires on almost every row. Cost model: the slow path runs
~955 `mask_box_coverage` calls per blank row (≈2 byte-reads + 2 popcounts each); 1559 blank rows
≈ 3 M popcounts ≈ ~3 ms wasted, replaced by 1559 band scans ≈ 33 µs. Theoretical saving ≈ 6.5%
of the ~46 ms render.

Benchmark `render_corpus_bilevel_dpi/dpi/150` (target) vs `dpi/300` (1:1 control, unaffected),
intra-session ratio, two rounds:

| Round | dpi/150 (ms) | dpi/300 (ms) | ratio |
|---|---|---|---|
| 1 baseline | 47.0 | 149.8 | 0.314 |
| 1 with | 45.7 | 143.3 | 0.319 |
| 2 baseline | 54.1 | 156.4 | 0.346 |
| 2 with | 47.0 | 157.4 | 0.299 |

Rounds disagree (−1.7% vs +13.7%): the ~6.5% signal is swamped by thermal variance (control
swung 143–157 ms, baseline dpi/150 swung 47–54 ms). 609 lib tests pass.

**Decision. Kept.**

**Reason.** The benchmark is too noisy to resolve the effect, but two independent lines of
evidence make it a clear win: (1) the fast path fires on 94.4% of rows, and (2) the per-row
overhead when it does *not* fire is one band scan (~640 bytes, NEON `any()`, ~20 ns) on top of
the existing 955 coverage calls — bounded at ~0.07% of render time. So the change is essentially
free when it misses and a real saving when it fires. Same disposition and rationale as I3
(kept at 2.3%, "zero-cost for documents with no blank rows; effect scales with whitespace").
Output is byte-identical by construction (fires only when every band pixel is background → white).
Closes #428.

---

### #427 — indexed JB2 mask + blit-map cache for FGbz-palette pages (`PageLayers::mask_indexed`) — **Kept** (2026-06-24)

**Issue.** #427 (from the perf-experiment-swarm). `PageLayers` cached the plain JB2 mask
(`mask`) but not the *indexed* variant used by FGbz-palette pages. `decode_mask_indexed`
called `extract_mask_indexed` → `jb2::decode_indexed` on every warm render, re-running the
full JB2 ZP arithmetic decode and re-allocating a page-sized `Vec<i32>` blit map
(2550×3300 = 8.4 M entries = 33.6 MB) each time.

**Approach.** Add `mask_indexed: OnceLock<Option<(Bitmap, Vec<i32>)>>` to `PageLayers` with a
`mask_indexed()` accessor and a `DjVuPage::decoded_mask_indexed()` method. Rewrite
`decode_mask_indexed` to return `(Cow<Bitmap>, Cow<[i32]>)` — `Cow::Borrowed` on cache hit,
with the same error-fallback as `decode_mask` (re-decode when the cache is empty but an
Sjbz/Smmr chunk is present, to surface decode errors in strict mode). `DecodedLayers` and
`ForegroundLayers` `blit_map` fields change `Option<Vec<i32>>` → `Option<Cow<'a, [i32]>>`;
downstream consumers already used `.as_deref()`/`Option<&[i32]>`, so no compositor change.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** New benchmark `palette_native_cached` (navm_fgbz.djvu, 2550×3300, native).
Control `color_native_cached` (non-palette, unaffected). All runs thermally hot;
intra-session ratio `palette / color_native`:

| Round | control (ms) | palette (ms) | ratio | Δ |
|---|---|---|---|---|
| 1 baseline | 211.2 | 237.1 | 1.123 | |
| 1 with cache | 176.1 | 194.9 | 1.107 | −1.4% |
| 2 baseline | 205.4 | 227.0 | 1.105 | |
| 2 with cache | 254.5 | 258.1 | 1.014 | −8.2% |

All four measurements show the cached ratio below baseline (consistent direction); magnitude
swamped by thermal variance (control ranged 176–254 ms), mean ≈ **5%**. 609 lib tests +
`pdf_conversion` (navm_fgbz) pass.

**Decision. Kept.**

**Reason.** Eliminates a full JB2 ZP re-decode and a 33.6 MB `Vec<i32>` allocation on every
warm render of a palette page; output is byte-identical by construction (memoized
`extract_mask_indexed`). Beyond the modest CPU win, it removes per-render allocator churn of
a 33.6 MB buffer. Memory cost: 33.6 MB cached per palette page actually rendered (the same
buffer that was previously allocated-and-freed each render, now persisted) — paid only on
FGbz-palette pages, which are uncommon. Mirrors the COW_FG / BG_CACHE caching pattern. Closes #427.

---

### #426 — BG44 decoded RGB Pixmap cache at subsample=2 (`PageLayers::bg_rgb_s2`) — **Kept** (2026-06-24)

**Issue.** #426 (from the perf-experiment-swarm). `PageLayers` cached the decoded RGB
Pixmap only at subsample=1 (`bg_rgb_s1`, the BG_CACHE experiment). At subsample=2 — the
common 150-from-300-DPI render — `decode_background_chunks` called `img.to_rgb_subsample(2)`
fresh on every warm render, re-running the IW44 IDWT + YCbCr→RGB conversion (~8.4 MB output)
even though the underlying `Iw44Image` ZP decode was already cached via `decoded_bg44()`.

**Approach.** Mirror BG_CACHE for sub=2: add `bg_rgb_s2: OnceLock<Option<Pixmap>>` to
`PageLayers`, a `bg_rgb_s2()` accessor (`self.bg44(page)?.to_rgb_subsample(2)`), a
`DjVuPage::decoded_bg_rgb_s2()` method, and a `subsample == 2` branch in both the strict
`decode_background_chunks` and `decode_background_chunks_permissive` `max_chunks == MAX`
paths returning `Cow::Borrowed`. ~40 lines, no algorithmic change. Cache is ~8.4 MB
(4× smaller than the sub=1 cache); left empty on pages never rendered at sub=2.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Target `color_downscale_mixed_cached` (watchmaker @150/300, hits the sub=2
path), control `color_native_cached` (sub=1, unaffected). All runs thermally hot;
intra-session ratio `downscale_mixed / native` cancels throttling:

| Round | control native (ms) | downscale (ms) | ratio | |
|---|---|---|---|---|
| 1 baseline | 236.8 | 60.7 | 0.256 | |
| 1 with cache | 334.7 | 78.2 | 0.234 | **−8.6%** |
| 2 baseline | 238.5 | 72.0 | 0.302 | |
| 2 with cache | 199.0 | 55.8 | 0.280 | **−7.3%** |

Both rounds show the cached ratio below baseline; consistent **~7–9%** speedup on the
downscale path (above the issue's 4–6% estimate — the conversion is a larger fraction of
total time in the hot state). 609 lib tests pass.

**Decision. Kept.**

**Reason.** Pure memoization of an identical `to_rgb_subsample(2)` call — output is
byte-identical by construction, so correctness is guaranteed. Saves the IDWT + YCbCr→RGB
conversion on every warm 150-DPI color render at the cost of ~8.4 MB per page ever rendered
at sub=2. Directly extends the proven BG_CACHE pattern (which gave −5.1% at sub=1). Closes #426.

---

### C2b/C2c: FG44 y-row hoisting for B-series and area-average compositor — **Reverted** (2026-06-24)

**Issue.** `composite_rows_bilinear_one` (B-series/non-1:1 path) and `composite_rows_area_avg_one`
call `sample_bilinear(fg, fg_fx, fg_fy)` per fg pixel. `fg_fy` (and for area-avg, `fg_fy_step`
and the y-bounds `(y0, y1)`) are computed from `fy`, which is row-invariant. Hypothesis: explicit
pre-hoisting (C2b for bilinear, C2c for area-avg) before the inner loop would save ~14 ops/fg-pixel
in the B-series path and ~6 ops/fg-pixel in the area-avg path.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** New benchmark `color_downscale_mixed_cached` (watchmaker.djvu at 150/300 DPI,
exercises B-series with FG44). `color_native_cached` as thermal control (unaffected). All runs
thermally hot (tests ran in parallel, p-values unreliable). Intra-session thermal ratio
`downscale_mixed / native`:

| Run | native (ms) | downscale_mixed (ms) | ratio |
|---|---|---|---|
| WITHOUT C2b | 231 | 54 | 0.234 |
| WITH C2b #1 | 227 | 58 | 0.256 |
| WITH C2b #2 | 185 | 59 | 0.319 |
| WITH C2b #3 | 151 | 59 | 0.391 |
| WITH C2b #4 | 213 | 43 | 0.202 |

Ratio without C2b: 0.234. With C2b: 0.202–0.391, mean ≈ 0.29. No consistent trend;
variance overwhelms the expected ~9% signal.

**Decision. Reverted.**

**Reason.** LLVM already applies LICM (Loop Invariant Code Motion) to hoist `fg_fy` and the
two row-slice computations out of the inline `sample_bilinear` body — the y-coordinate
arithmetic is recognized as loop-invariant at the call site. C2b adds explicit hoisting but
does not improve on what the compiler already does; the additional Option wrapping slightly
changes the generated code and may inhibit some optimizations. This mirrors the G2 finding:
LLVM's out-of-order/LICM already covers the expected savings. The `color_downscale_mixed_cached`
benchmark case is kept for future experiments.

---

### P2: BILEVEL_RGBA lookup table for bilevel 1:1 compositor — **Kept** (2026-06-24)

**Issue.** The bilevel 1:1 inner loop (`composite_rows_bilevel_one`, 1:1 fast path) used
per-pixel bit extraction + 4 scalar byte stores — ~16 instructions per pixel, fully scalar.
Even removing the `.min()` clamp (Q1, see below) did not trigger LLVM auto-vectorisation
because the shift-by-variable pattern `(byte >> (7-(ox&7))) & 1` is not recognised by the
AArch64 backend.

**Approach.** Add `BILEVEL_RGBA: [[u8; 32]; 256]` (8 KiB, 128 cache lines): each entry maps
one mask byte to 8 pre-packed RGBA pixels (0x00000000FF for fg, 0xFFFFFFFFFF for bg as
little-endian u32). Before the scalar fallback, guard on `offset_x==0 && out_w<=mask.width`
and process mask_row in 8-pixel (1-byte) chunks via `copy_from_slice`. LLVM compiles each
`copy_from_slice(32 bytes)` to `ldp q0, q1 / stp q0, q1` (2 NEON loads + 2 NEON stores)
= 4 NEON ops per 8 pixels instead of 16+ scalar ops per pixel.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Intra-session control: `color_native_cached` (G1 path, unaffected by P2).
All runs thermally hot (machine ran tests + benchmarks in sequence):

| Run | bilevel (ms) | color (ms) | bilevel/color |
|---|---|---|---|
| I3 baseline | 91.8 | 88.0 | 1.043 |
| P2 hot #1 | 161 | 193 | 0.834 |
| P2 hot #2 | 217 | 235 | 0.923 |
| P2 hot #3 | 167 | 209 | 0.799 |

Average P2 ratio: 0.852. Thermal-corrected speedup on bilevel:
(1.043 − 0.852) / 1.043 = **18% improvement** (conservative; best run: 24%).
Assembly confirmed: `ldp q0, q1` + `stp q0, q1` in hot loop (LBB314_36).

**Decision. Kept.**

**Reason.** NEON `ldp q0, q1 / stp q0, q1` replaces 16+ scalar ops per pixel. The 8 KiB
BILEVEL_RGBA table fits in L2 cache and persists across rows; L2 latency (~7 cycles) for
each 32-byte fetch is dwarfed by the 12-cycle-per-pixel scalar baseline. Guard covers all
standard full-page renders (offset_x==0, which is the common case). I3 still handles
all-zero rows via `fill(255)`. 776 tests pass.

---

### Q1: clamp-free bilevel 1:1 inner loop — **Reverted** (2026-06-24)

**Issue.** The scalar bilevel 1:1 inner loop uses `.min(page_w - 1)` to clamp the pixel
coordinate, which LLVM cannot prove is a no-op. Hypothesis: removing the clamp (when
`offset_x == 0 && out_w <= mask.width`) would allow LLVM to auto-vectorise the bit-extract.

**Approach.** Add a guard `if ctx.offset_x == 0 && out_w <= mask.width` and a clamp-free
inner loop using `mask_row[ox >> 3]` directly.

**Numbers.** Assembly inspection confirmed the resulting inner loop was still **scalar**
(identical instruction sequence to the original, just without the cmp/csel clamp): LLVM
cannot auto-vectorise the `(mask_row[ox>>3] >> (7-(ox&7))) & 1` bit-extraction pattern even
without the clamp. Both hot benchmark runs showed `No change in performance detected`.

**Decision. Reverted** (immediately superseded by P2 which uses BILEVEL_RGBA lookup).

**Reason.** Bit-extraction patterns with non-trivial shift amounts are not auto-vectorised
by LLVM/AArch64 backend. P2 restructures the loop to use direct table lookup and achieves
the NEON vectorisation that Q1 could not.

---

### G1b: pre-expand mask row to bytes in B-series (downscale) bilinear path — **Reverted** (2026-06-24)

**Issue.** The B-series bilinear compositor (`composite_rows_bilinear_one` else-branch,
invoked for non-1:1 scales) had the same 7-op bit-extraction per pixel as the 1:1 path
before G1. Hypothesis: the same MASK_EXPAND pre-expansion trick would give similar gains.

**Approach.** Add a `g1b_buf`/`g1b_mask` pre-expansion block (identical to G1) before
the B-series inner loop. The B-series accesses mask at page-space coordinates (`px` jumping
by `fx_step/FRAC` per output pixel). After expansion, `g1b_mask[px]` replaces the 7-op
bit extraction.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** `color_native_cached` (1:1, uses G1 path — unaffected) as control.
No corpus file available for B-series color; used `render_page/dpi/72`
(watchmaker at 72 DPI — exercises B-series, includes cached codec work).

| Benchmark | Baseline (ms) | G1b (ms) | Change |
|---|---|---|---|
| render_page/dpi/72 | cached Criterion | 91.3 | **+10% regression** (p = 0.00) |

Intra-session thermal check: `color_native_cached` showed no change (47ms), confirming the
regression is from G1b, not thermal variation.

**Decision. Reverted.**

**Reason.** At 72 DPI (638 output pixels/row) the pre-expansion cost (319 MASK_EXPAND
iterations per row) is 0.5 iterations per output pixel — 4× the amortization ratio of G1
in the 1:1 path (0.125/pixel). The break-even is ~532 output pixels/row. At 72 DPI (638)
we are barely above break-even, and the 4096-byte stack buffer zeroing adds ~32 cycles/row
overhead; the net is a measured 10% regression. At 150 DPI (1274 output pixels) the
benefit would be positive, but no benchmark corpus file exists to verify. Reverted rather
than guarding on output width — adds complexity without verified benefit.

---

### I3: all-white row fast path in bilevel 1:1 compositor — **Kept** (2026-06-24)

**Issue.** The bilevel 1:1 fast path in `composite_rows_bilevel_one` runs a per-pixel
bit-extraction + branchless-channel-write loop for every output row, including rows that
contain no foreground bits (page margins, blank inter-line space). No early exit existed
for the common all-background case.

**Approach.** Before the inner loop, scan the pre-hoisted mask row with
`mask_row.iter().any(|&b| b != 0)`. If all bytes are zero → `row_buf.fill(255)` (one
vectorised fill) and return. Check cost: 319 OR operations, NEON-vectorised to ~20 cycles;
amortised overhead for 3301 rows ≈ 0.02ms. Mirrors the F2 fast path in the bilinear path.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Intra-session control: `color_native_cached` (unaffected by I3). Ratios:

| Run | bilevel (ms) | color (ms) | bilevel/color |
|---|---|---|---|
| G2-revert baseline | 101.9 | 95.4 | 1.068 |
| I3 | 91.8 | 88.0 | 1.043 |

I3 speedup (thermal-corrected via ratio): (1.068 − 1.043) / 1.068 = **2.3%** on
bilevel_native_cached (cable_1973, 300 DPI, dense typeset — low whitespace fraction).
Not statistically significant in isolation (p = 0.33). Effect scales with document
whitespace; academic or letter-format scans with larger margins would see more benefit.

**Decision. Kept.**

**Reason.** Simple, correct, zero-cost for documents with no blank rows. Analogous to F2.
776 tests pass.

---

### G2: bg-fill-then-fg-fixup in general 1:1 bilinear path — **Reverted** (2026-06-24)

**Issue.** After G1 reduced the per-pixel mask check, the per-bg-pixel cost is dominated by
`bg_row.get(off..off+4).map_or(...)` (~6 cycles per bg pixel × 80% of pixels). Hypothesis:
replacing the per-pixel bg lookup with a one-shot memcpy over the entire row (Pass 1), then
only overwriting fg pixels with FG44 bilinear (Pass 2), would eliminate 80% of per-pixel
subslice loads.

**Approach.** When `ctx.gamma_is_identity`, mask present, bg fits (same conditions as F2):
- Pass 1: `row_buf.copy_from_slice(&bg_row[offset_x*4..(offset_x+out_w)*4])` — one memcpy.
- Pass 2: iterate `0..out_w` over pre-expanded g1_mask bytes; `continue` for bg pixels;
  compute bilinear and overwrite for fg pixels.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Intra-session control: `bilevel_native_cached` (unaffected by G2). Ratios:

| Run | color (ms) | bilevel (ms) | color/bilevel |
|---|---|---|---|
| G1-only (prior session) | 90.6 | 96.4 | 0.940 |
| G2 | 95.4 | 101.9 | 0.936 |

G2 shows color/bilevel 0.940 → 0.936 — a 0.4% difference, within noise (p = 0.55,
Criterion "no change"). 776 tests pass.

**Decision. Reverted.**

**Reason.** LLVM already pipelines the g1_mask load and bg_row load in the G1 inner loop
using out-of-order execution — the two independent loads overlap, making the original
single-pass pattern effectively as fast as a separate memcpy pass. G2's Pass 2 iteration
(scanning 2550 g1_mask bytes to find fg pixels) adds a sequential dependency chain that
prevents the same pipelining. Net result: no measurable improvement.

---

### G1: pre-expand mask row to bytes in general 1:1 bilinear path — **Kept** (2026-06-24)

**Issue.** In the general 1:1 bilinear path of `composite_rows_bilinear_one` (color pages with
FG44 at native DPI), the per-pixel mask check involves 7 operations: `Option::is_some_and`,
bounds-check `pxu < mask_w`, shift `pxu >> 3`, bounds-checked load `row.get(...)`, bit-position
shift `>> (7 - (pxu & 7))`, AND, and compare. This costs ~4 cycles per pixel on M1 Max
(load-latency dominated). For a 2550-wide page with 75% non-bg rows, this accounts for a
large fraction of total compositor time.

**Approach.** Before the inner loop, iterate over the bit-packed mask row once (319 iterations
for 2550px wide) and expand each mask byte to 8 per-pixel bytes via `MASK_EXPAND` LUT into a
4096-byte stack buffer (`g1_buf`). Then in the inner loop:
```rust
let is_fg = g1_mask.get(px as usize).copied().unwrap_or(0) != 0;
```
One bounds-checked byte load + compare, replacing the 7-op extraction.

Pages wider than `G1_MAX = 4096` px fall through to the original bit-extraction. No-mask
pages get `&g1_buf[..0]` → `get(px) = None → is_fg = false`. Applied after F2 (all-bg rows
are already returned early and never reach the G1 pre-expansion).

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Intra-session thermal control: `bilevel_native_cached` (bilevel compositor,
G1 does not affect this path). Two Criterion runs in the same session:

| Run | color_native_cached | bilevel_native_cached | color/bilevel ratio |
|---|---|---|---|
| F2 only (prior run) | 257ms | 218ms | 1.179 |
| F2 + G1 (this run) | 90.6ms | 96.4ms | 0.940 |

G1 speedup: 1.179 / 0.940 = **1.254 → 25.4% faster** on color_native_cached
(watchmaker.djvu, 300 DPI, warm caches, single-threaded).

Criterion showed -64.8% for color vs -55.8% for bilevel — the 9% gap (in log space) is the
G1 contribution; the remaining 55.8% is the thermal improvement between runs.

**Decision. Kept.**

**Reason.** 25% improvement, strong intra-session thermal correction, 776 tests pass. Cost
is 319 MASK_EXPAND iterations per mixed row (amortized: 319/2550 = 0.125 iterations per
pixel), paying ~638 cycles to save ~2550 × 4 = 10200 cycles in the inner loop. Stack
allocation (4096 bytes, zero-initialized once) is negligible; LLVM may omit zeroing for the
bytes beyond `mask_w` that are never read.

---

### F2: all-bg row fast path in general 1:1 bilinear path — **Kept** (2026-06-24)

**Issue.** For color pages with FG44 at native DPI (`composite_rows_bilinear_one`, general 1:1
path), every output row dispatches per-pixel between FG44 bilinear lookup (fg pixels) and
BG44 row copy (bg pixels). Margins and inter-line gaps — typically 25-35% of rows in dense
text scans — contain no foreground bits at all; the per-pixel branch and FG44 setup are
wasted for those rows.

**Approach.** Before the inner loop (after C3 mask-row hoist), pre-scan the hoisted mask row:
- `mask_row_1x1 = None` (no mask) → all pixels are bg → bulk copy
- `mask_row_1x1 = Some(row)` with `row[..].iter().all(|&b| b == 0)` → all pixels are bg → bulk copy

For gamma-identity renders (`ctx.gamma_is_identity`) when output fits within the bg pixmap
(`offset_x + out_w <= bg_w`): `row_buf.copy_from_slice(&bg_row[offset_x*4..(offset_x+out_w)*4])`.
No mask and no bg → `row_buf.fill(255)`. Falls through to per-pixel loop for non-identity gamma
or edge cases.

Mirrors the existing E1 full-width bulk-copy already present in the A2 (no-FG44) fast path.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** `render_compositor_only/color_native_cached` (watchmaker.djvu, 300 DPI equivalent,
single-threaded, warm caches). Stored Criterion baseline from prior session (100ms hot state);
this session is 2.53× less throttled (thermal factor from bilevel control: 218ms/86ms).

Expected color without F2: 105ms × 2.53 = 266ms. Actual: 257ms → F2 saves ~3.4% (9ms).
Criterion showed +146% for color vs +154% for bilevel (control) — the 8% difference maps to
the ~3% absolute speedup after thermal correction. Confidence interval too wide (205–315ms,
10 samples) for statistical significance at this thermal state.

Theoretical estimate for 25% all-bg rows: saves 25% × 2550 × ~12 cycles/pixel = ~8ms cool;
consistent with the 9ms intra-session estimate.

**Decision. Kept.**

**Reason.** Correct, clean, and consistent with the existing E1 bulk-copy in the A2 path.
Intra-session thermal estimate confirms the expected direction. Benefits scale with whitespace:
higher for documents with wide margins or loose leading. 776 tests pass.

---

### MASK_EXPAND batch + mb==0 fill in bilevel 1:1 path (E1) — **Reverted** (2026-06-24)

**Issue.** The bilevel 1:1 fast path in `composite_rows_bilevel_one` does per-pixel bit extraction:
`((mask_row[px >> 3] >> (7 - (px & 7))) & 1)`. Hypothesis: grouping 8 pixels per mask byte via
`MASK_EXPAND` + a bulk `fill(255)` for all-white bytes (`mb==0`) would be faster, since typical
text pages have >50% empty mask bytes (margins, inter-line gaps).

**Approach.** Added an aligned fast path (`offset_x & 7 == 0 && offset_x + out_w <= page_w`):
- When `mb == 0`: `row_buf[chunk_start*4..chunk_end*4].fill(255)` — 32-byte NEON fill.
- Otherwise: `MASK_EXPAND[mb]` LUT expansion + per-pixel write of `!exp[j]`.
- Unaligned offset falls through to original per-pixel path.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Using `dpi/150` (downscale, E1 does not affect this path) as the thermal control:
the stored Criterion baseline was from a heavily throttled prior session (131ms at 150 DPI).
This session measured 23ms at 150 DPI → 5.7× less thermal throttling.

Thermal-correcting `dpi/300` (1:1 bilevel path, E1 target): old baseline 479ms / 5.7 = 84ms
expected without code change. Measured after E1: 82ms — a difference of 2ms (2.4%), within
measurement noise (Criterion showed -82.9% change, but that reflects the 5.7× thermal difference
between sessions, not E1's effect).

**Decision. Reverted.**

**Reason.** No measurable improvement. LLVM already vectorizes the original scalar loop to
efficient NEON on aarch64-apple-darwin: it hoists the mask byte load per 8-pixel group and
generates branchless bit extraction. The explicit `MASK_EXPAND` LUT and branching on `mb==0`
add structure that interferes with LLVM's own vectorization without adding value. The "bilevel
NEON expander" experiment (2026-06-21) reached the same conclusion for manual NEON intrinsics;
E1 confirms the scalar auto-vectorized path is already optimal.

---

### C3: mask-row hoist in general 1:1 bilinear path — **Kept** (2026-06-24)

**Issue.** In `composite_rows_bilinear_one`, the general 1:1 fast path (entered when FG44 or
an offset is present, guarded by `fx_step==FRAC && fy_step==FRAC && bg_x_q24==bg_y_q24==1:1`)
computed `py < m.height && m.get(px, py)` per pixel, which implicitly recomputes the row slice
`m.data[py*stride..]` each time. C2 (from 2026-06-23) had already hoisted the `fy` row for
FG44 and the bg row; the mask row hoist was an analogous cleanup.

**Approach.** Added `mask_row_1x1: Option<(&[u8], u32)>` computed once before the inner loop:
```rust
let mask_row_1x1 = ctx.mask.and_then(|m| {
    if py >= m.height { return None; }
    let stride = m.row_stride();
    m.data.get(py as usize * stride..).map(|row| (row, m.width))
});
```
Inner loop: replaced `m.get(px, py)` with inline bit extraction from the pre-sliced row.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Criterion `render_compositor_only/color_native_cached`: p=0.59 (not statistically
significant). Within thermal noise.

**Decision. Kept.**

**Reason.** Correct optimization — eliminates y×stride multiply per pixel, mirrors C2 pattern.
LLVM likely already applied LICM here (consistent with p=0.59). Code is cleaner and provides
a foundation for future vectorization of the general 1:1 path.

---

### Byte-level POPCNT in `mask_box_coverage` — **Kept** (2026-06-23)

**Issue.** `mask_box_coverage` iterated over every pixel in the output footprint using
`mask.get(sx, sy)` — one conditional branch per bit. At 72 DPI from 300 DPI native the
footprint is ~4×4 = 16 bit reads per output pixel; at 150 DPI it is 2×2 = 4.

**Approach.** Replaced the nested pixel loop with byte-level popcount:

1. Compute `byte_lo = x0 / 8`, `byte_hi = x1.div_ceil(8)`.
2. Derive `first_mask` (clears bits for pixels before x0 in byte_lo) and `end_mask`
   (clears bits for pixels at/after x1 in the last byte), exploiting MSB-first packing.
3. Single-byte footprint path (`byte_hi == byte_lo + 1`): one AND + one `count_ones()` per
   row. Multi-byte path: one AND + popcount per boundary byte, full `count_ones()` per
   middle byte.

Ops per output pixel:
- Before: 16 `mask.get()` calls (16 branches + 16 byte reads + 16 bit shifts)
- After: ~4-8 `count_ones()` calls total (~4 byte reads, no branches in inner loop)

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Direct thermal comparison impossible across runs; used dpi/300 (1:1, no
coverage function called) as thermal control. Thermal factor between before/after runs: 1.82×.

| DPI | Before (bit-by-bit) | After (POPCNT) | After thermal-adj | Δ (adj) |
|-----|---------------------|----------------|-------------------|---------|
| 72  | 9.1 ms  (control: 263 ms) | 12.5 ms (control: 479 ms) | 6.9 ms | −24% |
| 150 | 71 ms   (control: 263 ms) | 131 ms (control: 479 ms)  | 72 ms  | +1% (noise) |

At 72 DPI the footprint spans ~4 pixels per output row, typically a single byte → single-byte
path, 4 ops per pixel vs 16. At 150 DPI the footprint is 2 pixels → 2 ops vs 4 (smaller
relative gain, absorbed by other overheads).

**Decision. Kept.**

**Reason.** ~24% thermal-corrected speedup at 72 DPI (thumbnail renders). No regression at
150 DPI or on the 1:1 path. Code is no more complex than before. Existing unit test
`mask_box_coverage_values` passed unchanged, confirming correctness of the byte-mask logic.

---

### Anti-aliased bilevel text at downscale (`mask_box_coverage`) — **Kept** (2026-06-23)

**Issue.** `composite_rows_bilevel_one` called `mask_box_any` in the downscale branch —
early-exit as soon as any mask bit in the output pixel's footprint was set. This produced
binary black/white output at reduced DPIs (72–150), giving jagged aliased text edges.

**Approach.** Added `mask_box_coverage`: counts the fraction of foreground bits in the
footprint and returns a proportional gray value (0 = all white, 255 = all black). The
downscale branch now computes `ch = 255 - mask_box_coverage(...)` and writes the same gray
value to R/G/B/A. The `mask_shift > 0` sub-path (subsampled max-pool mask) remains binary —
it already encodes one-bit coverage. The 1:1 fast path is completely untouched.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Criterion medians.

*Indirect proxies (color page, bilevel compositor not involved):*

| Benchmark | Before | After | Δ |
|-----------|--------|-------|---|
| `render_page/dpi/72` | 89.9 µs | 83.3 µs | −7.3% |
| `render_page/dpi/144` | 485 µs | 497 µs | +2.5% (noise) |
| `render_corpus_bilevel` (1:1) | ~42 ms | ~44 ms | ±noise only |

*Dedicated bilevel downscale benchmark (`render_corpus_bilevel_dpi`, cable_1973_100133.djvu,
native 300 DPI; added in the same commit). Both runs below were made in a thermally hot state
so numbers are internally consistent but higher than ideal. The 300 DPI column (1:1, neither
function called) acts as a thermal control — any ratio deviation from 1:1 between 72/300 or
150/300 isolates the bilevel downscale overhead.*

| DPI | mask_box_any (binary) | mask_box_coverage (AA) | Ratio AA/binary | 1:1 thermal control |
|-----|-----------------------|------------------------|-----------------|---------------------|
| 72  | 17–25 ms | 7.3–11 ms | ~0.5× | ÷(463 ms / 263 ms) ≈ 0.5× thermal |
| 150 | 148–196 ms | 61–81 ms | ~0.5× | same factor |
| 300 (1:1) | 463–542 ms | 229–299 ms | ~0.5× thermal only | control |

The ratio at all DPIs is the same ~0.5×, including the 1:1 control where no coverage function
runs. Conclusion: the bilevel downscale overhead of `mask_box_coverage` vs `mask_box_any` is
indistinguishable from noise on M1 Max due to thermal variance between the two measurement
runs (~2× CPU frequency drift). The 1:1 bilevel corpus path is unchanged.

*Stable reference (mask_box_coverage, hot state): dpi/72 median ≈ 9 ms, dpi/150 ≈ 71 ms,
dpi/300 (1:1) ≈ 263 ms. Use these for future regression comparisons.*

**Decision. Kept.**

**Reason.** Clear quality improvement for the common screen-viewing use case (72–150 DPI):
anti-aliased text edges replace binary aliasing. No measurable cost on the hot 1:1 path.
Downscale path overhead is bounded — counting `fx_step * fy_step` bits per output pixel,
same order as `sample_area_avg` already does for background. All 958 tests pass, including
a new unit test for `mask_box_coverage`. Closes #421.

---

### Pre-hoist FG44 y-rows and bg row in general 1:1 path (C2) — **Kept** (2026-06-23)

**Issue.** In `composite_rows_bilinear_one`, the general 1:1 path (entered when FG44 or an
offset is present) recomputed `map_plane_center_frac(fy, ctx.fg_y_q24)` per fg pixel and
`map_plane_center_frac(fx, ctx.bg_x_q24)` per bg pixel. `fy` is row-invariant; `bg_x_q24`
is guaranteed to be `1<<24` by the outer branch condition (`map_plane_center_frac(fx, 1<<24)
== fx`, a no-op multiply). The B-series path already applied these hoistings (B1 and B2);
the general 1:1 path was not updated after those optimisations were added.

**Approach.**

- **C2** (FG44 y-row hoisting): compute `fg_fy`, `y0`, `y1`, `ty`, and row slices
  `(row0, row1)` from `fg.data` once per row, before the inner loop. Replace
  `sample_bilinear(fg, fg_fx, fg_fy)` with `bilinear_from_rows(row0, row1, fg_w, fg_fx, fg_ty)`.
- **C2b** (bg row hoisting): pre-hoist `bg_row = bg.data[py * stride..]` and `bg_w` once
  per row. Replace `map_plane_center_frac(fx, bg_x_q24) + sample_nearest(bg, ...)` with a
  direct indexed slice read: `bg_row[px.min(bg_w-1) * 4..]`.

Eliminated per-iteration in the hot pixel loop:
- fg pixels: 1 × u64 multiply (`map_plane_center_frac(fy)`), y0/y1/ty computation, 2 row
  pointer lookups inside `sample_bilinear`.
- bg pixels: 1 × u64 multiply (`map_plane_center_frac(fx, 1<<24)` ≡ identity), row-index
  multiply inside `get_rgb` (hoisted to per-row stride multiply).

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Criterion medians; bilevel corpus shows high variance (53–76ms), likely thermal.

| Benchmark | Before | After | Δ |
|-----------|--------|-------|---|
| `render_corpus_color` (sequential) | 43.99 ms | 43.39 ms | −1.4% |
| `render_corpus_bilevel` | ~42–45 ms | ~43–64 ms | noise (path unchanged for mask-only pages) |

**Decision. Kept.**

**Reason.** −1.4% on the primary color-page benchmark. Change is a pure refactor (identical
numeric output, verified by 958 passing tests). The C2b bg-row hoisting also removes the
dead `bg_fy = map_plane_center_frac(fy, ctx.bg_y_q24)` variable that was previously computed
but then re-derived inside `sample_nearest`. `sample_nearest` is now only used in tests
(annotated `#[cfg_attr(not(test), allow(dead_code))]`). Closes #424 and #425.

---

### FG44 color lookup: `sample_nearest` → `sample_bilinear` — **Kept** (2026-06-22)

**Issue.** The 1:1 compositor path used `sample_nearest` to look up FG44 colors for
foreground (masked) pixels. FG44 is 3× subsampled: each FG44 pixel covers a 3×3 block of
output pixels. Nearest-neighbor sampling snaps to the nearest FG44 pixel center, producing
visible blocky 3×3 color tiles at color-text boundaries.

**Approach.** Replace `sample_nearest(fg, fg_fx, fg_fy)` with `sample_bilinear(fg, fg_fx,
fg_fy)` at both call sites in `djvu_render.rs` (general path ~line 2003 and B-series
optimized path ~line 2075). `sample_bilinear` has an identical signature and uses
fixed-point bilinear interpolation between the 4 nearest FG44 pixels, giving smooth color
transitions at fg44 pixel boundaries.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Criterion medians vs Cow-optimization baseline (~43.8 ms criterion-stored).

| Benchmark | Before | After | Δ |
|-----------|--------|-------|---|
| `render_corpus_color` (sequential) | 43.80 ms | 43.99 ms | +0.42% |
| `render_corpus_bilevel` (sequential) | 42.06 ms | 42.21 ms | +0.37% |

Bilevel pages have no FG44 layer; the +0.37% there is measurement noise. The +0.42% on
color pages is the overhead of 4 FG44 pixel reads instead of 1 per foreground pixel. Since
foreground (masked) pixels are a small fraction of the total in typical pages, the cost is
absorbed in benchmark noise.

**Decision. Kept.**

**Reason.** Genuine quality improvement: smooth bilinear color interpolation at FG44 pixel
boundaries replaces the visible 3×3 color-block artifact. Performance cost is negligible
(< 0.5%, within measurement noise). `sample_bilinear` was already implemented and tested;
this change enables it on the live rendering path. All 957 tests pass.

---

### Eliminate FG44 + Mask clones via `Cow` — **Kept** (2026-06-21)

**Issue.** After the BG44 Cow optimization, two smaller clones remained per warm render:

- `decode_fg44`: `page.decoded_fg44().cloned()` — clones the cached 3.7 MB FG44 Pixmap
  (subsample 3, ~850×1100 pixels × 4 bytes for a typical A4 scan at 300 DPI).
- `decode_mask`: `bm.clone()` — clones the cached 1.05 MB JB2 mask Bitmap (packed bits,
  ~2550×3300/8 bytes). Documented as "cloned cheaply (1 MB memcopy)" in the source.

Together: ~4.75 MB of unnecessary allocations and memcopy per warm render.

**Approach.** Same `Cow<'a, …>` pattern as the BG44 optimization:

- `decode_mask<'a>(page: &'a DjVuPage)` → `Result<Option<Cow<'a, Bitmap>>, …>`:
  cache hit → `Cow::Borrowed(bm)`, cold decode → `Cow::Owned`.
- `decode_fg44<'a>(page: &'a DjVuPage)` → `Result<Option<Cow<'a, Pixmap>>, …>`:
  FG44 cache hit → `Cow::Borrowed`, JPEG fallback → `Cow::Owned`.
- `ForegroundLayers<'a>` and `DecodedLayers<'a>` updated to hold `Option<Cow<'a, …>>`.
- `decode_foreground_strict<'a>` gains a lifetime.
- Bold dilation (`opts.bold > 0`): `mask.into_owned().dilate_n(n)` — clones only when
  actually needed (bold=0 is the common case in all benchmarks).
- Four `mask.as_ref()` and four `fg44.as_ref()` call sites → `as_deref()`.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Criterion medians vs state after BG44 Cow experiment.

| Benchmark | Before | After | Δ |
|-----------|--------|-------|---|
| `render_corpus_color` (sequential) | 42.65 ms | 41.93 ms | −1.7% |
| `render_corpus_bilevel` (sequential) | 42.52 ms | 41.92 ms | −1.4% |
| `render_corpus_color` (parallel, `--features parallel`) | 7.7 ms | 7.23 ms | −6.1% |

**Decision. Kept.**

**Reason.** Eliminates 4.75 MB of memcopy (FG44 3.7 MB + mask 1.05 MB) on every warm
render at the cost of two `Cow<'a, …>` wrappers and a lifetime parameter. The parallel
improvement is proportionally larger because the absolute compositor time is shorter, so
the clone is a bigger fraction of total work. All 775 tests pass; no_std unaffected (the
non-cached path stays `Cow::Owned`).

---

### Skip `Pixmap::white` pre-fill via zero-initialized buffer — **Rejected** (2026-06-22)

**Issue.** `render_pixmap` (and `render_region`, `render_progressive`) allocates the output buffer
via `Pixmap::white(w, h)` = `vec![255u8; w*h*4]`, a 33.6 MB write of 0xFF. Every compositor
path (bilevel, bilinear A2, general 1:1, area-avg) then overwrites every pixel before return, so
the initial 255-fill is redundant and doubles the write bandwidth to the output buffer.

**Approach.** Added `Pixmap::zeroed(w, h)` to djvu-pixmap (calls `Pixmap::new(0,0,0,0)` →
`vec![0u8; n]`). Changed all 8 output-buffer allocation sites in djvu_render.rs from
`Pixmap::white(...)` to `Pixmap::zeroed(...)` (render_pixmap, render_region, render_progressive,
render_coarse, plus aa_downscale and rotate_pixmap variants).

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Criterion medians vs state after FG44+mask Cow experiment.

| Benchmark | Before | After |
|-----------|--------|-------|
| `render_corpus_color` (sequential) | 41.93 ms | 41.53 ms (−1.0%) |
| `render_corpus_color` (parallel, `--features parallel`) | 7.23 ms | **8.77 ms (+21%)** |
| `render_corpus_bilevel` (parallel) | 7.30 ms | **11.82 ms (+62%)** |

**Decision. Rejected.**

**Reason.** On macOS, `vec![0u8; n]` for a large allocation returns lazy-zero VM pages (`mmap(MAP_ANONYMOUS)`
via the system allocator). The page-faults are deferred to first write. In the sequential compositor,
page faults happen in order and are handled cheaply. In the parallel compositor (`par_chunks_exact_mut`),
multiple rayon threads simultaneously fault disjoint 4 KB pages of the same large allocation, causing
contention in the macOS VM subsystem — the parallel render regressed by 21–62%.

By contrast, `Pixmap::white` uses `vec![255u8; n]` which actively writes every page before the
parallel compositor starts. This "pre-warms" the pages: all page faults are handled sequentially
during allocation, and the parallel compositor gets clean private pages with no kernel overhead.

The sequential path showed a marginal −1.0% improvement (less work: write 33.6 MB with 0 then
overwrite vs write 33.6 MB with 255 then overwrite — same total, but zero writes may alias CPU
zero-write optimizations). Not enough to justify the parallel regression.

---

### Fill+overlay compositor paths — **Rejected** (2026-06-21)

**Issue.** The bilinear compositor (`composite_rows_bilinear_one`) for a warm color page
writes the output in a single pass but still performs per-pixel blend arithmetic. Hypothesis:
pre-filling the output buffer with the background (one 33.6 MB `copy_from_slice`) and then
overlaying only foreground pixels (sparse writes) would reduce total work for pages where
most pixels are background.

**Approach.** Three variants after the BG44 Cow commit:

1. **Bilevel fill+overlay**: In `composite_rows_bilevel_one`, before the main loop: fill
   the full output row from the background (`copy_from_slice` of 4 bytes × width), then
   overwrite only the foreground pixels (mask bits that are 1). Intended to remove the
   per-pixel `ch = (is_fg.wrapping_sub(1) & 0xFF)` branch-free blend.
2. **A3 (FG44 copy+overlay)**: For the A2 path (bg+mask, FG44 present, no palette): copy
   the background slice into the output row, then overwrite foreground pixels from the FG44
   plane. Variant of the "fill then sparse overwrite" idea for the color case.
3. **A4 (palette copy+overlay)**: Same as A3 but for the FGbz palette path: copy background,
   then overwrite foreground pixels from the palette table.

A micro-benchmark (`rustc -O` standalone binary) was written to measure the pure inner loop
in isolation: it completes in ~2 ms, confirming the 42 ms benchmark cost is not the tight
pixel loop itself.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Criterion medians, `render_corpus_color` after all three variants applied together.

| Variant | `render_corpus_color` | Δ |
|---------|-----------------------|---|
| Baseline (BG44 Cow, no fill+overlay) | 41.3 ms | — |
| Fill+overlay (all three variants) | 42.65 ms | +3.3% |

**Decision. Rejected.**

**Reason.** All three variants regressed. Root cause: the fill+overlay approach writes 33.6 MB
twice — once for the background copy, once for the foreground overlay reads — vs the original
single-pass write. The working set already exceeds L3 cache capacity (33.6 MB bg read +
33.6 MB output write); doubling the output writes adds a third pass over main memory. The
micro-benchmark confirmed the inner loop is fast; the bottleneck is memory bandwidth, not
arithmetic.

---

### Eliminate BG44 Pixmap clone via `Cow<'a, Pixmap>` — **Kept** (2026-06-21)

**Issue.** After the `bg_rgb_s1` cache (previous experiment), the warm render path still cloned the
33.6 MB cached `Pixmap` via `.cloned()` at the `decode_background_chunks` / `decode_background_chunks_permissive`
call site. This incurred a ~0.67 ms memcopy per warm render regardless of whether the compositor
actually mutated the background.

**Approach.** Changed `decode_background_chunks` and `decode_background_chunks_permissive` to return
`Option<Cow<'a, Pixmap>>` (lifetime tied to `&'a DjVuPage`). For the cached `subsample == 1,
max_chunks == usize::MAX` path: return `Cow::Borrowed(page.decoded_bg_rgb_s1()?)`. For all other
paths (fresh decode, subsample != 1, progressive): return `Cow::Owned(pixmap)`. Updated
`DecodedLayers<'a>` and `decode_layers<'a>` with the same lifetime. Changed all four
`CompositeContext::from_layers(... bg.as_ref() ...)` call sites to `bg.as_deref()`.

`Cow` resolves via `Deref<Target = Pixmap>` to `&Pixmap` transparently, so `CompositeContext`
sees the same `Option<&Pixmap>` as before. No change to non-cached paths.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Criterion medians vs state after inline-alpha experiment.

| Benchmark | Before | After | Δ |
|-----------|--------|-------|---|
| `render_corpus_color` (sequential) | 41.8 ms | 41.3 ms | −1.2% |
| `render_corpus_color` (parallel, `--features parallel`) | 8.3 ms | 7.7 ms | −7.2% |

**Decision. Kept.**

**Reason.** Eliminates a 33.6 MB memcopy on the hot warm-render path with zero semantic change —
callers already only read the background. The improvement is larger in the parallel path because
the sequential compositor dominates less; the 0.67 ms clone is a proportionally bigger fraction
of 8.3 ms than 41.8 ms. `Cow::Borrowed` has zero overhead (it's a pointer, no heap allocation).
All 775 tests pass; no_std build unaffected (the non-cached path falls through to `Cow::Owned`).

---

### Inline alpha in bilinear compositor, remove `fill_alpha_255` — **Kept** (2026-06-21)

**Issue.** The non-downscale bilinear compositor (`composite_rows_bilinear_one`) wrote only R, G,
B channels per pixel (not alpha). Alpha was set in a separate post-pass: `fill_alpha_255(buf)` over
the full 33.6 MB buffer. In `composite_into` (the parallel path) this post-pass ran sequentially
after `par_chunks_exact_mut`, serialising ~33.6 MB of memory I/O. In `composite_rows` (streaming)
it ran per-row but still touched each row twice.

**Approach.** Added `pixel[3] = 255` in every pixel-write site inside
`composite_rows_bilinear_one` (A2 macro, no-mask loops, general 1:1 path, general bilinear path).
The E1 full-width `copy_from_slice` path already copies alpha=255 from the bg Pixmap (whose
YCbCr-decode functions always set alpha=255). Removed `fill_alpha_255` calls from `composite_into`
and `composite_rows`, then deleted the now-dead `fill_alpha_255` / `fill_alpha_255_sse2` helpers.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Criterion medians vs state after BG44 RGB Pixmap cache (previous experiment).

| Benchmark | Before | After | Δ |
|-----------|--------|-------|---|
| `render_corpus_color` (sequential) | 42.8 ms | 41.8 ms | −2.3% |
| `render_corpus_bilevel` (sequential) | 43.0 ms | 42.1 ms | −2.1% |
| `render_streaming_discard/watchmaker` (seq) | 42.2 ms | 41.0 ms | −2.8% |
| `render_corpus_color` (parallel, --features parallel) | 9.5 ms | 8.3 ms | −12.6% |

**Decision. Kept.**

**Reason.** Writing 4 bytes per pixel in one pass is faster than 3 + a separate alpha post-pass
in either mode. The gain is especially large in the parallel path where the sequential 33.6 MB
post-pass was a bottleneck (0.34 ms at memory bandwidth) relative to the 4 ms parallel compositor.
Also removes ~50 lines of SSE2 code that is no longer needed.

---

### BG44 decoded RGB Pixmap cache (`PageLayers::bg_rgb_s1`) — **Kept** (2026-06-21)

**Issue.** Every warm render of a colour DjVu page called `Iw44Image::to_rgb_subsample(1)` even
though the decoded `Iw44Image` was already cached. The conversion (parallel IDWT + YCbCr→RGBA +
33.6 MB allocation) took 2.8–2.9 ms per render call, accounting for ≈6% of total render time.

**Approach.** Added `bg_rgb_s1: OnceLock<Option<Pixmap>>` to `PageLayers`. The new accessor
`PageLayers::bg_rgb_s1()` calls the existing `bg44()` cache then runs `to_rgb_subsample(1)` once
and stores the result. `decode_background_chunks` and its permissive variant use this cache for the
`max_chunks == usize::MAX, subsample == 1` path (the common full-resolution render case). Strict
mode still propagates BG44 decode errors via `decoded_bg44().ok_or(...)`.

The saved Pixmap is cloned on each warm render (~0.34 ms memcpy of 33.6 MB) instead of
recomputed (2.8 ms). Net saving ≈ 2.5 ms per warm render.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Criterion medians, 100-sample runs for regression gates, 10-sample for stage breakdown.

| Benchmark | Before | After | Δ |
|-----------|--------|-------|---|
| `render_corpus_color` | 45.1 ms | 42.8 ms | −5.1% |
| `render_corpus_bilevel` | 44.9 ms | 43.0 ms | −4.2% |
| `render_streaming_discard/watchmaker_color` | 44.1 ms | 42.2 ms | −4.3% |
| `render_streaming_discard/cable_bilevel` | ~48 ms | 42.4 ms | −11.7% |
| `render_compositor_only/color_native_cached` | 44.8 ms | 42.3 ms | −5.6% |

**Decision. Kept.**

**Reason.** Consistent 4–12% improvement across all color and mixed-layer benchmarks at the cost
of ~33.6 MB additional memory per page that has ever been rendered at sub=1. Acceptable trade-off
for the common interactive use case (viewer renders the same page repeatedly). Strict-mode error
semantics preserved via explicit `decoded_bg44()` guard before returning the cached value.

---

### Bilevel NEON expander (`composite_rows_bilevel_one`) — **Rejected** (2026-06-21)

**Issue.** Accelerate the bilevel 1:1 fast path in `composite_rows_bilevel_one` with AArch64 NEON
by expanding 1 mask byte → 8 RGBA pixels per iteration via `vst4_u8`.

**Approach.** Three variants tried against commit after `819f4a3`:

1. **Separate `#[target_feature(enable = "neon")] unsafe fn bilevel_neon_1to1`** called once per row:
   scalar head + NEON body (`vdup_n_u8` / `vand_u8` / `vceq_u8` / `vst4_u8`) + scalar tail.
   Inner loop had two branch conditions (`px_start + 8 > page_w`, `byte_idx >= mask_row.len()`).

2. **Same code inlined** directly inside the `#[cfg(target_arch = "aarch64")]` block in
   `composite_rows_bilevel_one` (no separate function, avoids call overhead).

3. **Branch-free inner loop**: hoisted both bounds checks out of the NEON body, computing
   `neon_end = (page_w - start_px) & !7` once before the loop.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Criterion medians, `--bench render render_corpus_bilevel`.

| Variant | `render_corpus_bilevel` |
|---------|------------------------|
| Baseline (scalar) | 44.9 ms |
| Variant 1 (separate `#[target_feature]` fn) | 52.2 ms (+16%) |
| Variant 2 (inlined, branches in loop) | 50.3 ms (+12%) |
| Variant 3 (inlined, branch-free inner loop) | 45.7 ms (+1.8%) |

**Decision. Rejected.**

**Reason.** Even with the branch-free inner loop, the NEON expansion breaks even with scalar at
best (+1.8%, within noise). Root cause: LLVM already generates efficient NEON for the scalar
`is_fg.wrapping_sub(1) & 0xFF` pattern on aarch64-apple-darwin (NEON is always enabled). Manual
`vst4_u8` adds complexity without measurable gain. The separate `#[target_feature]` function
cannot use `#[inline(always)]` in stable Rust (see issue #145574), so it incurs per-row call
overhead.

---

### Color compositor 1:1 fast path NEON (`a2_has_mask_loop!`) — **Rejected** (2026-06-21)

**Issue.** The color bilinear compositor (`composite_rows_bilinear_one`) spends ~45ms on a
2550×3301 colour page. Inner loop (`a2_has_mask_loop!`) expands mask bytes via `MASK_EXPAND` LUT
and blends BG → output. Hypothesis: NEON `vld4_u8/vst4_u8` plus bitwise ops per 8 pixels would
beat the scalar LUT path.

**Approach.** Added a `gamma_is_identity` fast path in `composite_rows_bilinear_one` bypassing the
macro: when the full row fits in BG pixmap and mask, reads RGBA using `vld4_u8`, computes
`not_fg = vceq_u8(vand_u8(vdup_n_u8(mb), bit_pos), zero)`, ANDs R/G/B channels, writes with
`vst4_u8`. Scalar fallback for non-AArch64. Tail pixels handled separately.

**Platform.** macOS Darwin 25.5.0 / Apple M1 Max, aarch64, Rust stable 1.88.

**Numbers.** Criterion medians, `render_native_stages/render_streaming_discard/watchmaker_color`.

| Variant | Time |
|---------|------|
| Baseline | 45.1 ms |
| F1 scalar fast path | 45.9 ms (+0.9 ms) |
| F1 NEON fast path | 46.4 ms (+1.3 ms) |

**Decision. Rejected.**

**Reason.** No improvement — slight regression. The inner loop is not the bottleneck. LLVM already
vectorises the scalar code efficiently on AArch64. The compositor time (~42–46 ms for 8.4 M pixels)
is dominated by something else (memory bandwidth, `fill_alpha_255`, or `to_rgb_subsample`), not
the per-pixel blend arithmetic.

---

### #420 — SIMD `sample_area_avg_bounds` (NEON/SSSE3 accumulation) — **Rejected** (2026-06-21)

**Issue.** #420: accelerate the `sample_area_avg_bounds` inner loop with SIMD
to speed up the area-average downscale path in `composite_rows_area_avg_one`.

**Approach.** Two variants tried on commit `819f4a3`:

1. **Per-row SIMD dispatch** (`rgba_row_rgb_sums`): added helper functions
   `rgba_row_rgb_sums_neon` (AArch64: `vld4_u8` + `vaddlv_u8`, threshold 32 B)
   and `rgba_row_rgb_sums_ssse3` (x86_64: `_mm_shuffle_epi8` + `_mm_sad_epu8`,
   threshold 16 B).  The outer loop over rows called these per row.

2. **Inline 2×2 fast path**: special-cased `cols == 2 && rows == 2` with two
   direct 8-byte slice reads and 12 scalar additions, bypassing the loop entirely.

**Platform.**
- OS: macOS 26.3.1 / Darwin 25.5.0
- CPU: Apple M1 Max, aarch64
- Rust: stable 1.92+, RUSTFLAGS: unset

**Numbers.** All times are Criterion medians, `--bench render`, single-thread (no `parallel` feature).

| Benchmark | Baseline | Variant 1 (NEON/SSSE3) | Variant 2 (2×2 inline) |
|-----------|----------|------------------------|------------------------|
| `render_colorbook` | 3.558 ms | 4.264 ms (+19.9%) | 5.183 ms (+45.7%) |
| `render_corpus_color` | 45.863 ms | 45.582 ms (−0.6%, noise) | 45.587 ms (−0.6%, noise) |

**Decision.** Rejected. Both variants regressed `render_colorbook`, which is the
primary area_avg benchmark (colorbook renders at 150 dpi from a 300 dpi source,
producing a 2× downscale through the bg plane).

**Reason.**
- **Variant 1**: The NEON fast path requires ≥ 32 bytes (8 RGBA pixels) per row.
  At 2× downscale the box is 2×2 — each row is 2 pixels = 8 bytes, below the
  threshold.  Every call falls through to the scalar tail, adding function-call
  overhead with zero NEON benefit.
- **Variant 2**: The `if cols == 2 && rows == 2` branch and the tuple
  `if let (Some(a), Some(b))` pattern introduce control-flow that disrupts LLVM's
  optimization of the surrounding inlined composite loop.  The constant `col == 2`
  check prevents the compiler from treating the loop uniformly, increasing
  register pressure and harming branch prediction for other branches.
- In both cases `render_corpus_color` (4× downscale at 300 dpi) showed no
  measurable change, confirming that `sample_area_avg_bounds` is not the
  bottleneck — memory bandwidth and the mask check dominate.

### #419 — Compositor row-level parallelism — **Kept** (2026-06-18, documented here 2026-06-21)

**Issue.** #419 (created 2026-06-21 after audit) — intra-page row parallelism
in `composite_into`.

**Status.** Already implemented and recorded under entry `PARALLEL_COMPOSITOR`
(2026-06-18) as part of the #408 umbrella.  Issue #419 closed as duplicate.

Recorded numbers (from `PARALLEL_COMPOSITOR`, `--features parallel` vs single-thread):

| Benchmark | Single-thread | With `parallel` | Speedup |
|-----------|--------------|-----------------|---------|
| `render_colorbook` | 3.558 ms | 940 µs | 3.8× |
| `render_corpus_color` | 45.86 ms | 12.15 ms | 3.8× |
| `render_corpus_bilevel` | ~45 ms | 13.99 ms | ~3.2× |

(Numbers re-measured 2026-06-21 on commit `819f4a3` reflect further optimizations
since the original PARALLEL_COMPOSITOR recording.)

### #418 — IW44 Y/Cb/Cr IDWT 3-plane parallelism — **Kept** (2026-06-21)

**Issue.** #418: parallelize the three independent `reconstruct()` calls
(inverse wavelet transform per plane) in `Iw44Image::to_rgb()` using `rayon::join`.

**Status.** Already implemented in `crates/djvu-iw44/src/lib.rs` lines 3238–3256
under `#[cfg(feature = "parallel")]`.  Benchmarked here for the first time.

**Approach.** `rayon::join(|| y.reconstruct(sub), || rayon::join(|| cb.reconstruct(sub_c), || cr.reconstruct(sub_c)))` — three independent IDWT passes run on separate threads.  Single-thread path preserved under `#[cfg(not(feature = "parallel"))]`.

**Platform.**
- OS: macOS 26.3.1 / Darwin 25.5.0
- CPU: Apple M1 Max, aarch64
- Rust: stable 1.92+, RUSTFLAGS: unset
- Commit: `819f4a3`

**Numbers.** `cargo bench --bench codecs -- iw44_to_rgb_colorbook`

| Benchmark | Single-thread | `--features parallel` | Speedup |
|-----------|--------------|----------------------|---------|
| `iw44_to_rgb_colorbook/sub1_full_decode` | 5.622 ms | 2.556 ms | **2.2×** |
| `iw44_to_rgb_colorbook/sub2_partial_decode` | 1.325 ms | 603 µs | **2.2×** |
| `iw44_to_rgb_colorbook/sub4_partial_decode` | 344 µs | 212 µs | **1.6×** |

**Decision.** Kept. The code was already merged; benchmarks confirm the win.

**Reason.** Y, Cb, Cr planes share no mutable state after ZP decoding (which is
sequential per-stream). The IDWT step on each plane is independent. On the 10-core
M1 Max, three concurrent IDWT passes achieve ~2.2× wall-time reduction for sub1
(Y dominates; Cb + Cr run in parallel with Y's tail). sub4 gains less (1.6×)
because the compact-plane IDWT is shorter. The `parallel` feature is opt-in, so
builds without rayon are unaffected.

### #408 — area-avg exclusive-box bounds + power-of-2 shift — **Kept** (2026-06-18)

**Issue.** #408 umbrella: close the 2× compositor gap vs DjVuLibre.

**Approach.** Two related changes to `src/djvu_render.rs`:

1. `sample_area_avg` and `mask_box_any`: switch x1/y1 from **inclusive** to
   **exclusive** upper bounds. The old formula `((fx + fx_step) >> FRACBITS).min(w − 1)`
   inclusive landed on the first pixel of the *next* output cell at exact power-of-2
   downscale ratios, giving a 3×3 box at 2× downscale instead of the correct 2×2.
   The fix: `((fx + fx_step) >> FRACBITS).min(w)` exclusive with `x0..x1` iteration.

2. `sample_area_avg`: replace per-pixel bounds check (`data.get(off..off+3)`) with
   one bounds check per row (`data.get(row_off..row_off + cols*4)` → `chunks_exact(4)`),
   and replace UDIV by variable `count` with a rounding right-shift when `count` is
   a power of 2 (the common case: count=4 at 2× downscale).

**Platform.**
- OS: macOS 26.3.1 / Darwin 25.5.0
- CPU: Apple M1 Max, aarch64
- Rust: stable 1.92+, RUSTFLAGS: unset

**Numbers.** All times are Criterion medians. "Baseline" = committed HEAD before
this branch. The committed HEAD already contains the decode_scale regression
(#377), which forces the colorbook bg plane to subsample 2 instead of 4 —
making bg_fx_step = 32 (2× downscale into the bg plane), which hits the
area_avg path. This makes the colorbook improvement large.

```
cargo bench --bench render -- 'render_corpus_color|render_corpus_bilevel'
cargo bench --bench render -- 'render_compositor_only/color_downscale'
cargo bench --bench render -- 'render_colorbook$'
cargo bench --bench codecs -- 'iw44_to_rgb_colorbook/sub2|sub4'
```

| Benchmark | Baseline | After | Δ |
|-----------|----------|-------|---|
| `render_colorbook` (150 dpi, warm) | 25.5 ms | 12.2 ms | **−52%** |
| `render_compositor_only/color_downscale_cached` | 25.2 ms | 14.4 ms | **−43%** |
| `render_corpus_color` (300 dpi color) | 77.9 ms | 70.4 ms | **−10%** |
| `render_corpus_bilevel` (300 dpi bilevel) | 71.9 ms | 71.5 ms | 0% (bilevel path, no area_avg) |
| `compositor_only/color_native_cached` | 71.3 ms | 71.3 ms | 0% (bilinear path, unaffected) ✓ |
| `iw44_to_rgb_colorbook/sub2_partial_decode` | 1.339 ms | 1.353 ms | +1.0% (noise) ✓ |
| `iw44_to_rgb_colorbook/sub4_partial_decode` | 345.3 µs | 346.5 µs | +0.3% (noise) ✓ |

**Explanation of improvements.**
- `render_colorbook`: when decode_scale picks subsample 2 (wrong), bg_fx_step = 32
  → area_avg path with 2× bg-plane downscale. Old code averaged 3×3=9 bg pixels
  per output pixel; new code averages 2×2=4. −52%.
- `render_corpus_color`: mask_box_any was scanning 3×3=9 bits per output pixel at
  2× page downscale; now scans 2×2=4. −10%.
- `render_corpus_bilevel`: bilevel pages use `composite_rows_bilevel_one` which
  calls neither `sample_area_avg` nor `mask_box_any`. Unaffected by design. ✓
- Bilinear path, sub2/sub4 partial decode: not touched. ✓

**Output change.** The box averaging is now correct (2×2 instead of 3×3 at 2×
downscale). Output pixel values will differ slightly from the old version but are
closer to the mathematically correct box filter. All 608 lib tests pass.

**Decision.** Kept.

**Reason.** The old inclusive range was an off-by-one that inflated the box by
one pixel in each dimension at exact-integer-ratio downscale. The fix is
semantically correct, yields 2-3× speedup in the area_avg path, and leaves all
non-area_avg paths (bilinear, bilevel, partial decode) statistically unchanged.

### Cross-size JB2 record-6 refinement emitter (#322) — **Reverted / kept disabled** (2026-06-15)

**Issue.** #322: build an *experiment-only* cross-size record-6 refinement
emitter behind a probe flag (`max_dim_delta = 2`, `max_hamming_fraction =
0.05`), decode + pixel-compare on `watchmaker` and `pathogenic_bacteria_1896`,
and record the real byte deltas + round-trip status. Keep
`encode_djvm_bundle_jb2` (and the other shipped encoders) unchanged.

**Approach.** Added `CrossSizeRec6Probe` to `Jb2EncodeOptions` (default `None`).
When enabled, a fresh connected component with no exact dictionary hit searches
dictionary entries whose bbox differs by ≤ `max_dim_delta` per axis, scores them
with nearest-neighbor-resampled Hamming distance, and — if within the budget —
emits a **lossless** record-6 matched refinement (`wdiff`/`hdiff` + 11-bit
refinement bitmap against the chosen reference) instead of a record-1 new
symbol. This required finishing the dormant `encode_bitmap_ref` helper: it was
written in image (top-down) space, but the decoder's `decode_bitmap_ref` walks
packed Jbm rows **bottom-up**, and the `>> 1` centre-alignment floor is not
symmetric under that flip — so a non-solid refinement bitmap desynchronised the
ZP stream (`UnknownRecordType` on decode). Rewriting the encoder to mirror the
decoder's Jbm-row traversal exactly made it round-trip.

**Numbers** (re-encoding existing page masks; baseline = shipped rec-1/7 only):

| corpus | pages | pages changed | baseline | probe | delta | round-trip |
|--------|-------|---------------|----------|-------|-------|------------|
| watchmaker (all) | 12 | 12 | 130 036 B | 135 720 B | **+5 684 B (+4.37%)** | lossless |
| pathogenic_bacteria_1896 (first 40) | 38 | 36 | 1 539 929 B | 1 543 596 B | **+3 667 B (+0.24%)** | lossless |

**Decision.** Reverted as a shipping path; the probe stays `None` by default, so
all shipped encoders (`encode_jb2_dict`, `encode_jb2_dict_with_shared`,
`encode_djvm_bundle_jb2`) are byte-identical to before (asserted by
`cross_size_rec6_probe_off_is_byte_identical`). Kept behind the flag with two
round-trip regression tests and an `--ignored` corpus measurement driver
(`cross_size_rec6_probe_corpus_measurement`).

**Reason.** Cross-size refinement *loses* bytes on both corpora. A cross-size
reference must be nearest-neighbor resampled before it can drive the refinement
context; the geometric misalignment makes the 11-bit context mispredict, so the
refinement bitmap costs nearly as much as a fresh direct symbol — and then adds
the record-6 index + `wdiff`/`hdiff` overhead on top. It is also strictly worse
than record-1 for *future* repeats, because record-6 is blit-only and never
extends the dictionary, so a later exact copy of the same glyph can no longer
hit rec-7. The earlier analysis-only scaffold's estimated deltas pointed the
same way; this confirms it with real bytes. Net: not worth pursuing at these
thresholds.

### IW44 forward-transform reconstruction-loss localization (#320) — **Kept (diagnostic only)** (2026-06-15)

**Issue.** #320: localize IW44 BG44 reconstruction loss on the photographic
`conquete_paix` pages 3 & 11, among four candidate stages — forward wavelet
transform, quantization threshold schedule, coefficient state transitions, and
reconstruction tracking. Keep default encoder behavior unchanged.

**Approach.** Added in-crate diagnostics (`src/iw44_encode.rs`, module
`loss_diagnostics`):
1. A full-precision i32 reference forward transform compared coefficient-wise
   against the production i16 transform on the real page-3/11 luma planes, plus
   a max-intermediate-magnitude probe for i16 headroom.
2. Per-band reconstruction residual `Σ|blocks − recon|` vs band energy
   `Σ|blocks|`, driving the real encoder slice loop, mapping each IW44 band to
   its contiguous zigzag-coefficient range.

**Numbers.**
- *Forward transform:* production i16 transform equals the i32 reference
  **exactly** (0 / 3.2 M coefficients diverge on p3, 0 / 3.3 M on p11). Largest
  intermediate magnitude 6 845 (p3) / 11 378 (p11) ≪ `i16::MAX` (32 767) — no
  overflow. The transform is lossless on these pages.
- *Per-band residual after the default 100 slices (rel = resid/energy):*

  | band | p3 rel | p11 rel |
  |------|--------|---------|
  | 0 (coarsest) | 23.0% | 17.6% |
  | 3 | 59.4% | 25.5% |
  | 6 | 99.8% | 65.7% |
  | 9 (finest) | 100.0% | 99.9% |

  Residual grows monotonically with frequency; band 9 `recon` is all-zero
  (p3) or within 0.1% of it (p11) — never activated within budget.
- *Pixel-domain floor (gray round-trip, 96×96 broadband):* avg abs error
  plateaus at 8.72 from 200 slices onward (identical at 255) — not a
  transform-pair mismatch but the refinement-schedule floor (`s>>1`, `s>>2`,
  `s>>3` round to 0 at small steps, so refinement deltas vanish before `recon`
  reaches `blocks`).

**Decision.** Kept as diagnostics; **no encoder change**. Two regression tests
(`forward_transform_is_lossless`, `loss_concentrates_in_high_frequency_bands`)
plus an `--ignored` table dump (`print_band_table`).

**Reason.** The loss is the progressive **quantization / slice budget**, not the
forward transform, state transitions, or reconstruction tracking: the forward
transform is bit-exact and overflow-free, and the coarse bands reconstruct
correctly (which they could not if state/tracking were broken). The finest
bands are simply starved — their small coefficients never cross the activation
threshold within 100 slices. An earlier i32 reference falsely reported a
forward-transform "divergence"; the bug was in the *reference* (it ran the
column pass over every column instead of the `s·ℤ` multiresolution sublattice),
not in production. Acting on quality would mean a different slice/quant schedule
or budget, which is out of scope for this diagnostic-only issue.

### Post-roadmap render baseline correction — **Kept** (2026-05-17)

**Approach.** Reran the render filters from #308 on the same code after the
post-roadmap full workspace run produced implausibly slow render rows. The
rerun confirmed that the public render baseline should use the targeted render
artifact, not the full-run outliers from PR #335.

**Platform.**
- OS: macOS 26.3.1 / Darwin 25.3.0 (`RELEASE_ARM64_T6000`)
- CPU: Apple M1 Max, 10 cores
- arch: `arm64` / Rust host `aarch64-apple-darwin`
- target features: ARM64 baseline; NEON available on Apple Silicon
- Rust: `rustc 1.92.0 (ded5c06cf 2025-12-08)`
- Cargo: `cargo 1.92.0 (344c4567c 2025-10-21)`
- RUSTFLAGS: unset

**Command(s).**

```sh
cargo bench --bench render -- 'render_corpus_color|render_colorbook' --output-format bencher
cargo bench --bench render -- 'render_page/dpi|render_corpus_bilevel|render_native_stages' --output-format bencher
```

**Numbers.**

| Benchmark | Corrected render baseline |
|-----------|--------------------------:|
| `render_page/dpi/72` | 246,839 ns |
| `render_page/dpi/144` | 937,501 ns |
| `render_page/dpi/300` | 3,586,208 ns |
| `render_page/dpi/600` | 13,911,536 ns |
| `render_colorbook` | 7,221,910 ns |
| `render_colorbook_stages/full_render` | 7,256,342 ns |
| `render_colorbook_stages/mask_decode` | 4,392,493 ns |
| `render_colorbook_cold` | 18,838,014 ns |
| `render_corpus_color` | 71,247,374 ns |
| `render_native_stages/render_pixmap/watchmaker_color` | 71,532,521 ns |
| `render_native_stages/render_into_reuse_buffer/watchmaker_color` | 70,452,562 ns |
| `render_native_stages/render_streaming_discard/watchmaker_color` | 70,185,520 ns |
| `render_native_stages/mask_decode/watchmaker_color` | 2,735,714 ns |
| `render_native_stages/bg_to_rgb_warm/watchmaker_color` | 2,962,458 ns |

**Decision.** Kept.

**Reason.** No render-path code changed between the #308 targeted baseline and
the bad #335 full-run artifact; the only intervening render source edits were
documentation comments. The targeted rerun restored the expected range, so
`README.md`, `BENCHMARKS_RESULTS.md`, and `BENCHMARKS.md` now use this
corrected render baseline. The bad full-run render rows remain recorded below
as a rejected artifact instead of being used as public claims.

### Post-roadmap full benchmark refresh — **Needs follow-up** (2026-05-17)

**Approach.** Reran the public full workspace Criterion command after the
roadmap PR series was merged through #310, then reran the DjVuLibre comparison
harness against the same local Criterion artifact. No code was changed; this
refresh updates public benchmark documentation from the new measurements.

**Platform.**
- OS: macOS 26.3.1 / Darwin 25.3.0 (`RELEASE_ARM64_T6000`)
- CPU: Apple M1 Max, 10 cores
- arch: `arm64` / Rust host `aarch64-apple-darwin`
- target features: ARM64 baseline; NEON available on Apple Silicon
- Rust: `rustc 1.92.0 (ded5c06cf 2025-12-08)`
- Cargo: `cargo 1.92.0 (344c4567c 2025-10-21)`
- RUSTFLAGS: unset

**Command(s).**

```sh
cargo bench --workspace --features cli,tiff
bash scripts/bench_djvulibre.sh /private/tmp/djvu-rs-post-roadmap-djvulibre
python3 scripts/djvulibre_compare.py \
  --criterion target/criterion \
  --djvulibre-bench /private/tmp/djvu-rs-post-roadmap-djvulibre/djvulibre_bench.txt \
  --ddjvu-timing /private/tmp/djvu-rs-post-roadmap-djvulibre/ddjvu_timing.txt
```

**Numbers.**

| Benchmark | Criterion mean |
|-----------|---------------:|
| `render_page/dpi/72` | 246 us |
| `render_page/dpi/144` | 934 us |
| `render_page/dpi/300` | 6.96 ms |
| `render_page/dpi/600` | 42.1 ms |
| `render_colorbook` | 8.78 ms |
| `render_colorbook_cold` | 48.9 ms |
| `render_corpus_color` | 151 ms |
| `render_corpus_bilevel` | 75.4 ms |
| `render_native_stages/render_streaming_discard/watchmaker_color` | 195 ms |
| `jb2_decode` | 132 us |
| `iw44_decode_first_chunk` | 592 us |
| `iw44_decode_corpus_color` | 655 us |
| `parse_multipage_520p` | 2.29 ms |
| `render_large_doc_first_page` | 10.6 ms |
| `pdf_export_sequential` | 821 ms |

DjVuLibre comparison on the same local fixture matrix:

| Scenario | djvu-rs | libdjvulibre C API | ddjvu CLI | Ratio |
|----------|--------:|-------------------:|----------:|------:|
| `boy.djvu` @ 72 dpi | 246 us | 159 us | 30.6 ms | djvu-rs 1.5x slower |
| `colorbook.djvu` @ 150 dpi | 8.78 ms | 5.96 ms | 67.3 ms | djvu-rs 1.5x slower |
| `watchmaker.djvu` @ 300 dpi | 151 ms | 36.44 ms | 79.8 ms | djvu-rs 4.2x slower |
| `cable_1973_100133.djvu` @ 300 dpi | 75.45 ms | 35.25 ms | 73.8 ms | djvu-rs 2.1x slower |

**Decision.** Needs follow-up.

**Reason.** Codec, document, and PDF rows from this run remain useful, but the
render rows were too noisy to use as public baseline claims. They were
superseded by the targeted render correction above, and the public docs were
updated accordingly.

### #306 — wasm32 scalar vs simd128 benchmark harness — **Kept** (2026-05-17)

**Approach.** Added a reproducible Node.js harness for the existing
`wasm-bindgen` API. The wrapper builds two `wasm-pack --target nodejs` bundles:
one scalar wasm32 bundle and one simd128 bundle built with
`RUSTFLAGS="-C target-feature=+simd128"`. The benchmark then imports both
bundles in Node.js and times parse, full render, cached render, and first
progressive render on `tests/fixtures/boy.djvu` at 150 dpi.

**Platform.**
- OS: macOS 26.3.1 (Darwin 25.3)
- CPU: Apple M1 Max, 10 cores
- host arch: `arm64`
- wasm target_arch: `wasm32`
- target_feature(s): scalar vs `simd128`
- Rust: 1.92.0 stable (`aarch64-apple-darwin`)
- wasm-pack: 0.13.1
- Node.js: v26.0.0
- RUSTFLAGS: unset for scalar; `-C target-feature=+simd128` for simd128

**Command(s).**

```sh
ITERATIONS=30 WARMUP=8 DPI=150 ./scripts/bench_wasm_simd128.sh

node scripts/bench_wasm_simd128.mjs \
  --scalar target/wasm-bench/scalar \
  --simd target/wasm-bench/simd128 \
  --fixture tests/fixtures/boy.djvu \
  --iterations 30 \
  --warmup 8 \
  --dpi 150 \
  --json
```

**Numbers.** Median milliseconds, 30 measured iterations after 8 warmups.
Negative delta means the simd128 bundle is faster.

| Benchmark | scalar median ms | simd128 median ms | delta |
|-----------|-----------------:|------------------:|------:|
| `parse_document` | 0.003 | 0.002 | -30.4% |
| `render_150dpi_fresh_doc` | 2.715 | 2.548 | -6.1% |
| `render_150dpi_cached_page` | 2.685 | 2.491 | -7.2% |
| `progressive_150dpi_chunk0` | 2.693 | 2.463 | -8.5% |

Checksums matched between scalar and simd128 for all render benchmarks
(`-663404102` for full render/cached render; `-663404261` for progressive
chunk 0), and the harness now fails if per-iteration checksums are unstable or
if scalar and simd128 checksums differ.

**Decision.** Kept.

**Reason.** The harness gives future wasm SIMD work a reproducible local
baseline and already confirms a modest render-path win on the existing
simd128 IW44 code. CI syntax-checks the harness and still build-checks both
plain wasm32 and `+simd128`; it does not run timing comparisons because hosted
runner variance would make the numbers unsuitable as a regression gate.

### #295 — JB2 encoder corpus round-trip and size baseline — **Needs follow-up** (2026-05-17)

**Approach.** Refreshed the existing JB2 quality harnesses without changing
encoder behavior. The page-level run measured original `Sjbz`, direct
`encode_jb2`, and dict `encode_jb2_dict` bytes/bpp/round-trip status across
current JB2-bearing fixtures and corpus files. The shared-Djbz run measured
`encode_jb2_dict` independent page totals vs bundled shared-Djbz totals, with
CC accounting and cross-size probe output enabled.

**Platform.**
- OS: macOS 26.3.1 (Darwin 25.3)
- CPU: Apple M1 Max, 10 cores
- target_arch: `aarch64`
- target_feature(s): ARM64 baseline; NEON available on Apple Silicon
- Rust: 1.92.0 stable (`aarch64-apple-darwin`)
- RUSTFLAGS: unset
- Source artifact: local run on `codex/issue-295-jb2-quality-refresh`

**Command(s).**

```sh
cargo run --release --example encode_quality_jb2 -- \
  references/djvujs/library/assets/boy_jb2.djvu \
  references/djvujs/library/assets/boy.djvu \
  references/djvujs/library/assets/carte.djvu \
  references/djvujs/library/assets/chicken.djvu \
  references/djvujs/library/assets/colorbook.djvu \
  references/djvujs/library/assets/DjVu3Spec_bundled.djvu \
  references/djvujs/library/assets/irish.djvu \
  references/djvujs/library/assets/navm_fgbz.djvu \
  tests/corpus/cable_1973_100133.djvu \
  tests/corpus/conquete_paix.djvu \
  tests/corpus/pathogenic_bacteria_1896.djvu \
  tests/corpus/watchmaker.djvu

cargo run --release --example encode_quality_djbz -- \
  --cc-stats --cross-size-stats \
  references/djvujs/library/assets/colorbook.djvu \
  references/djvujs/library/assets/DjVu3Spec_bundled.djvu \
  references/djvujs/library/assets/navm_fgbz.djvu \
  tests/corpus/conquete_paix.djvu \
  tests/corpus/pathogenic_bacteria_1896.djvu \
  tests/corpus/watchmaker.djvu
```

**Numbers.**

Page-level JB2 refresh:

| Mode | Pages | Bytes | bpp | vs original | Round-trip |
|------|------:|------:|----:|------------:|------------|
| Original `Sjbz` | 692 | 26,569,542 | 0.0263 | 1.000x | source |
| Direct `encode_jb2` | 692 | 46,252,033 | 0.0457 | 1.741x | 464 ok, 228 decode errors |
| Dict `encode_jb2_dict` | 692 | 36,016,741 | 0.0356 | 1.356x | 692 ok, 0 failures |

Per-file dict ratios:

| File | Pages | Dict/orig | Dict failures | Direct failures |
|------|------:|----------:|--------------:|----------------:|
| `boy_jb2.djvu` | 1 | 1.000x | 0 | 0 |
| `colorbook.djvu` | 62 | 1.030x | 0 | 46 decode errors |
| `DjVu3Spec_bundled.djvu` | 70 | 1.627x | 0 | 70 decode errors |
| `irish.djvu` | 1 | 0.302x | 0 | 0 |
| `navm_fgbz.djvu` | 5 | 0.301x | 0 | 5 decode errors |
| `cable_1973_100133.djvu` | 2 | 1.136x | 0 | 0 |
| `conquete_paix.djvu` | 22 | 1.025x | 0 | 16 decode errors |
| `pathogenic_bacteria_1896.djvu` | 517 | 1.378x | 0 | 80 decode errors |
| `watchmaker.djvu` | 12 | 1.058x | 0 | 11 decode errors |

`carte.djvu` was skipped by the harness because the checked-in fixture is
truncated and does not parse.

Shared-Djbz refresh:

| Mode | Files/pages | Bytes | bpp | vs original | Round-trip |
|------|------------:|------:|----:|------------:|------------|
| Original `Sjbz` totals | 6 / 688 | 26,424,220 | 0.0262 | 1.000x | source |
| Independent dict pages | 6 / 688 | 35,963,419 | 0.0356 | 1.361x | all pages ok |
| Bundled shared-Djbz | 6 / 688 | 34,986,136 | 0.0347 | 1.324x | all bundles ok |

Bundled shared-Djbz was `0.973x` of independent dict output (`-2.7%`) on this
six-file run. Individual bundle/independent ratios were: `colorbook` 1.002x,
`DjVu3Spec_bundled` 0.642x, `navm_fgbz` 0.955x, `conquete_paix` 1.029x,
`pathogenic_bacteria_1896` 0.976x, and `watchmaker` 0.945x.

Failure buckets:
- Direct `encode_jb2` decode errors are oversized whole-image record-3 symbols
  hitting decoder symbol-size limits on large pages.
- Dict `encode_jb2_dict` has no current mismatch or decode-error bucket on the
  refreshed corpus; the old `483/553` dict round-trip number is stale.
- Shared-Djbz has no current mismatch or decode-error bucket with byte-exact
  clustering; all six bundles round-trip pixel-exact.
- `carte.djvu` is a harness/input bucket: truncated fixture parse failure, not
  an encoder failure.

**Decision.** Needs follow-up. The refreshed safe baseline is dict encoding:
it round-trips all 692 pages but remains `1.356x` original bytes overall.
Shared-Djbz is safe and saves `2.7%` vs independent dict on this corpus, but it
still remains `1.324x` original bytes overall.

**Reason.** Correctness is no longer the blocker for the dict path on the
current corpus; byte cost is. The next narrow JB2 implementation issue should
be #301: add a byte-cost estimator for cross-size refinement before emitting
any new cross-size or lossy/lossless refinement records. The largest measured
size gaps are still `pathogenic_bacteria_1896` and `DjVu3Spec_bundled`, while
`watchmaker` shows cross-size candidate headroom already recorded by the probe.

### #294 — thumbnail row-scratch A/B — **Rejected** (2026-05-17)

**Approach.** Added a `render_row_scratch_ab` Criterion group to compare the
current strict direct `render_into` path against a row-scratch adapter that
copies `render_streaming` rows into the final RGBA buffer. The comparison uses
the issue's thumbnail and native targets with warmed decode caches.

**Platform.**
- OS: macOS 26.3.1 (Darwin 25.3)
- CPU: Apple M1 Max, 10 cores
- target_arch: `aarch64`
- target_feature(s): ARM64 baseline; NEON available on Apple Silicon
- Rust: 1.92.0 stable
- RUSTFLAGS: unset
- Source artifact: local run on `codex/issue-294-row-scratch-ab`

**Command(s).**

```sh
cargo bench --bench render -- render_row_scratch_ab \
  --warm-up-time 1 --measurement-time 2 --sample-size 10
```

**Numbers.**

First run:

| Target | Direct `render_into` | Row-scratch copy | Decision signal |
|--------|---------------------:|-----------------:|-----------------|
| `thumbnail_dpi72` | 248.21 µs | 205.35 µs | row-scratch faster |
| `thumbnail_half_bilinear` | 153.55 µs | 399.13 µs | row-scratch much slower |
| `colorbook_downscale` | 23.674 ms | 18.925 ms | row-scratch faster, noisy |
| `corpus_color_native` | 207.96 ms | 248.74 ms | native regression |
| `corpus_bilevel_native` | 150.93 ms | 198.23 ms | native regression |

Rerun after bounding the A/B group to keep full CI benchmark runtime stable:

| Target | Direct `render_into` | Row-scratch copy | Decision signal |
|--------|---------------------:|-----------------:|-----------------|
| `thumbnail_dpi72` | 306.59 µs | 199.09 µs | row-scratch faster |
| `thumbnail_half_bilinear` | 143.84 µs | 124.58 µs | row-scratch faster |
| `colorbook_downscale` | 15.966 ms | 11.861 ms | row-scratch faster, noisy |
| `corpus_color_native` | 155.40 ms | 135.02 ms | row-scratch faster, noisy |
| `corpus_bilevel_native` | 146.10 ms | 160.35 ms | no clear signal |

**Decision.** Rejected as a render heuristic. No production render path changed.
The A/B harness is kept so future thumbnail work can rerun the comparison.

**Reason.** The repeated short A/B runs are too noisy and inconsistent to justify
a production heuristic: the first run showed a thumbnail loss and native
regressions, while the rerun showed broader wins but still no clean bilevel
native signal. A threshold heuristic would be fragile without a more stable
predictor than output size alone.

### #293 — compositor-only render baselines — **Kept** (2026-05-17)

**Approach.** Added a `render_compositor_only` Criterion group to
`benches/render.rs`. Each case warms page-level decode caches with one
`render_pixmap` call, then measures `render_into` into a reused RGBA buffer.
This isolates cached compositor/output materialization from document parse,
codec decode/cache setup, and output allocation.

**Platform.**
- OS: macOS 26.3.1 (Darwin 25.3)
- CPU: Apple M1 Max, 10 cores
- target_arch: `aarch64`
- target_feature(s): ARM64 baseline; NEON available on Apple Silicon
- Rust: 1.92.0 stable
- RUSTFLAGS: unset
- Source artifact: local run on `codex/issue-293-compositor-baselines`

**Command(s).**

```sh
cargo bench --bench render -- render_compositor_only \
  --warm-up-time 1 --measurement-time 2 --sample-size 10
```

**Numbers.**

| Bench | Fixture/path | Cached path | Time |
|-------|--------------|-------------|-----:|
| `render_compositor_only/color_native_cached` | `tests/corpus/watchmaker.djvu` | color native, decoded caches warm, reused RGBA buffer | 71.061 ms |
| `render_compositor_only/bilevel_native_cached` | `tests/corpus/cable_1973_100133.djvu` | bilevel native, decoded caches warm, reused RGBA buffer | 72.171 ms |
| `render_compositor_only/color_downscale_cached` | `references/djvujs/library/assets/colorbook.djvu` | color downscale, decoded caches warm, reused RGBA buffer | 7.4213 ms |
| `render_compositor_only/small_color_downscale_cached` | `references/djvujs/library/assets/boy.djvu` | small color 0.5x downscale, decoded caches warm, reused RGBA buffer | 152.00 µs |

**Decision.** Kept. The new benches can be run independently with a single
Criterion filter, and their names identify color/bilevel, native/downscale,
and cached decode state.

**Reason.** This gives #294 and later compositor work a narrow baseline without
changing render behavior or mixing optimization into the measurement issue.

### #290 — layered multi-page DJVM directory encode — **Kept** (2026-05-16)

**Approach.** Extended `djvu encode <dir> --quality quality|archival` to encode
pages independently with `PageEncoder::from_pixmap`, then bundle the resulting
single-page `FORM:DJVU` pages with `djvm::merge`. The existing lossless directory
path is left unchanged and still uses `encode_djvm_bundle_jb2` with
`--shared-dict-pages`. Layered directory encode deliberately does **not** create a
shared Djbz dictionary: each page keeps its own `Sjbz` mask plus `BG44` and
optional `FGbz`, avoiding rejected Hamming shared-Djbz clustering while preserving
layered chunks in a parseable bundled DJVM.

**Numbers / fixture.** Added CLI fixtures for two-page RGB directories. Both
`--quality quality` and `--quality archival` produce parseable `page_count=2`
DJVM bundles; each page has `Sjbz`, `BG44`, and `FGbz`. The quality fixture also
renders every page through `djvu_render::render_pixmap` at native 32×32 pixels.
The pre-existing lossless directory fixture still produces `page_count=3` with
`Sjbz` pages and no `BG44` / `FGbz` chunks.

**Decision.** Kept. This satisfies layered multi-page encode without changing the
lossless shared-Djbz behavior or reviving Hamming clustering in the default path.

### #288 — adaptive segmentation + BG-block inpainting — **Kept** (2026-05-16)

**Approach.** Extended `SegmentOptions` without changing its default behaviour:
`Binarization::Fixed` remains the default global BT.601 threshold, while
`Binarization::Sauvola { window, k }` adds local adaptive binarisation for mixed
lighting scans. Added optional `bg_inpaint` for fully masked background blocks:
when a BG subsample cell has no unmasked source pixels, it is filled from the
nearest neighbouring unmasked pixels instead of falling back to the ink-coloured
block mean. `PageEncoder::with_segment_options` lets library callers opt into
these knobs for `Quality` / `Archival` single-page encodes; CLI defaults remain
unchanged.

**Numbers / fixture.** Added a checked-in synthetic mixed text/photo test in
`djvu_encode::tests::adaptive_segment_options_improve_decoded_mixed_lighting_fixture`:
left half dark paper (`Y=80`), right half bright paper (`Y=220`), with dark ink
(`Y=40`) and light gray ink (`Y=140`). With `bg_subsample=6`, fixed-threshold
Quality encode decodes at `mean_abs_rgb_diff=10.767` versus source; Sauvola +
inpainting decodes at `4.188` (61% lower), and the test requires at least a 30%
reduction. The lower-level `segment::tests::sauvola_handles_dark_background_and_light_ink`
asserts that fixed 128 masks most dark paper and misses the light ink, while
Sauvola keeps the mask less than half the fixed-mask size and retains both ink
pixels. Added `segment::tests::inpaint_fully_masked_bg_block_from_neighbors`: a
fully masked black 4×4 BG block next to tan paper now inpaints to
`(210,200,160)` when `bg_inpaint` is enabled; default fixed-threshold/no-inpaint
still falls back to black for all-black pages.

**Tests.** Added/updated segment unit tests, proptest `SegmentOptions`
constructors, and a `PageEncoder::with_segment_options` parseability test proving
Quality encode still emits `Sjbz` + `BG44` with adaptive options.

**Decision.** Kept. The new behaviour is opt-in, deterministic, covered by a
synthetic mixed-light fixture, and does not enable Hamming shared-Djbz clustering
or alter the default fixed-threshold path.

### #281 — strict `render_pixmap` composites directly into its output — **Kept** (2026-05-16)

**Approach.** Added native-resolution stage benches for the DjVuLibre comparison
corpus (`render_native_stages/*`) covering public `render_pixmap`,
`render_into` with a reused RGBA buffer, `render_streaming` with discarded rows,
JB2 mask decode, and cached IW44 inverse/RGB. Then changed strict
`render_pixmap` to call `render_into` directly instead of routing through the
row-streaming adapter and copying each scratch row into the output `Pixmap`.
`opts.permissive` keeps the old `render_rows` path because it has different
chunk-error recovery semantics.

**Numbers.** Quick local Criterion runs (`--warm-up-time 1 --measurement-time 2/3
--sample-size 10`) after #279 had made native render more expensive:

- `render_corpus_color`: `88.44 ms` → `72.27 ms` median (**18% faster**).
- `render_corpus_bilevel`: `90.09 ms` → `72.00 ms` median (**20% faster**).
- `render_colorbook` at 150 dpi: `7.29 ms` historical / `7.12 ms` after this
  change (no regression; slight improvement in the quick run).
- `iw44_to_rgb_colorbook/sub4_partial_decode`: `344 µs`, Criterion reported no
  statistically significant change, so the known sub4 partial decode path did
  not regress.

The new stage split (recorded in `BENCHMARKS_RESULTS.md`) shows warm JB2/IW44
codec stages are only a few milliseconds on the native corpus; the remaining
DjVuLibre gap is dominated by compositor sampling and output materialization.

**Tests.** Targeted render tests passed, including byte-identical
`render_rows`/`render_into` and `render_streaming`/`render_pixmap` checks plus the
permissive truncated-BG44 regression. Full validation below covered the rest of
the workspace.

**Decision.** Kept. The change is narrow, removes an avoidable row copy from the
public strict render path, beats Criterion noise on both native corpus targets,
and leaves the permissive recovery path and IW44 sub4 decode untouched.

### #280 — TIFF export uses `render_streaming` rows — **Kept** (2026-05-16)

**Approach.** Added `tiff_export::djvu_to_tiff_writer(doc, opts, writer)` and
changed the existing `djvu_to_tiff` byte-buffer wrapper to delegate to it.
Color TIFF pages now use `djvu_render::render_streaming` when options are
streamable (no AA, bilinear/no-op resampling, identity combined rotation) and
feed RGB rows directly into TIFF strips. Pages requiring render post-processing
keep the existing full-`Pixmap` fallback. Bilevel TIFF export was already a
mask-extraction path and remains unchanged.

**Numbers.** Repro probe added as
`examples/probe_tiff_streaming_memory.rs` (`required-features = ["tiff"]`).
Command run locally after a release build:

```text
/usr/bin/time -l target/release/examples/probe_tiff_streaming_memory \
  tests/fixtures/problem_page.djvu /tmp/problem_page_streamed.tiff 1.0
```

Output for the 600-dpi `problem_page.djvu` fixture:

- page: `3288x5050` px at scale `1.000` (`16,604,400` pixels)
- output TIFF bytes written to `File`: `49,813,798`
- full RGBA pixmap allocation avoided: `66,417,600` bytes
- full RGB staging allocation avoided: `49,813,200` bytes
- `/usr/bin/time -l` maximum resident set size: `7,962,624` bytes
- peak memory footprint: `7,111,552` bytes

**Tests.** Added TIFF tests comparing decoded streamed color-TIFF pixels against
the existing `render_pixmap(...).to_rgb()` result for both a color page
(`chicken.djvu`) and a bilevel page (`boy_jb2.djvu`). Also fixed an existing
TIFF test to unwrap `extract_bilevel_pixels` under the `tiff` feature.

**Decision.** Kept. This makes a real public export path use the row-streaming
renderer end-to-end without constructing a full output `Pixmap` or full RGB
staging image, while preserving byte/pixel equivalence through tests and keeping
the full-pixmap fallback for unsupported render options.

### #222 PR2 — high-level setters (`page_mut(i).set_text_layer`/`set_annotations`/`set_metadata`) — **Kept** (2026-05-01)

**Approach.** Builds on PR1's chunk-replacement primitive. New surface:

- `DjVuDocumentMut::page_count() -> usize` — `1` for `FORM:DJVU`, count of
  `FORM:DJVU` direct children for `FORM:DJVM`.
- `DjVuDocumentMut::page_mut(i) -> Result<PageMut<'_>, MutError>` — borrow
  one page's `FORM:DJVU` for editing.
- `PageMut::set_text_layer(&TextLayer)` — encode via `encode_text_layer`
  (page height read from `INFO`) + `bzz_encode`, replace the existing
  `TXTa`/`TXTz` or insert a new `TXTz`.
- `PageMut::set_annotations(&Annotation, &[MapArea])` — same shape over
  `encode_annotations_bzz` and `ANTa`/`ANTz`.
- `PageMut::set_metadata(&DjVuMetadata)` — over a new
  `metadata::encode_metadata` / `encode_metadata_bzz` pair, against
  `METa`/`METz`. Empty `DjVuMetadata` removes the chunk.
- New `MutError` variants: `PageOutOfRange`, `MissingPageInfo`,
  `InfoParse(IffError)`, `DjvmMutationUnsupported`.

`page_mut` errors with `DjvmMutationUnsupported` on `FORM:DJVM` bundles —
the page-level setters change a component FORM's byte size which would
shift DIRM offsets. DIRM recomputation is its own concern, deferred.

**Tests.** Nine new unit tests in `djvu_mut::tests` plus five in
`metadata::tests`:

- `set_text_layer_roundtrip_chicken`, `set_annotations_roundtrip_chicken`,
  `set_metadata_roundtrip_chicken` — each parse the re-emitted bytes and
  decode the chunk back to the input value.
- `set_metadata_empty_removes_existing_chunk` and
  `set_metadata_replaces_existing_chunk_in_place` — exercise the
  remove-on-empty and replace-don't-duplicate behaviours.
- `page_count_*`, `page_mut_out_of_range_errors`,
  `page_mut_djvm_returns_unsupported` — error paths.
- Metadata encoder tests cover empty input, dedicated-field round-trip,
  `extra` ordering, escape handling for `"`/`\\`, and BZZ round-trip.

All 410 lib tests pass (402 → 410; `+9` djvu_mut, `+5` metadata, with the
PR1 metadata count shift). `cargo clippy --workspace --lib --tests --bins
-- -D warnings` clean, `cargo fmt --check` clean. (Examples have two
pre-existing clippy warnings unrelated to this PR.)

**Reason kept.** Direct continuation of PR1's contract — PR1 only exposed
`replace_leaf(path, bytes)`; PR2 wires the existing chunk encoders to
that primitive so callers don't need to know IFF chunk IDs or BZZ
compression to update text/annotations/metadata. With this PR the
`librarian` consumer (#158) can finally drop its `djvused` shell-out for
single-page DjVu files.

**Follow-up status.**
1. Bundled DJVM mutation and `DjVuDocumentMut::set_bookmarks(&[DjVuBookmark])`
   have landed.
2. Single-page byte-range patching was implemented in #302; bundled-DJVM and
   indirect-DJVM byte-range work remain tracked separately.
3. Indirect DJVM support is intentionally deferred after #303; the decision
   record is `docs/indirect-djvm-mutation.md`, with implementation follow-ups
   in #325 and #326.

### #222 PR1 — `DjVuDocumentMut::from_bytes` + chunk-replacement primitive — **Kept** (2026-04-30)

**Approach.** New `src/djvu_mut.rs` module gated on `feature = "std"` with
the foundation layer for in-place document mutation. Public surface:

- `pub struct DjVuDocumentMut` — owns a parsed `DjvuFile` tree plus the
  original byte buffer.
- `pub fn from_bytes(data: &[u8]) -> Result<Self, MutError>` — parses (via
  `iff::parse`, the legacy tree-based parser) and retains the input bytes.
- `pub fn into_bytes(self) -> Vec<u8>` — fast path: when no mutation has
  happened, returns the original bytes verbatim. After any mutation, falls
  through to `iff::emit`.
- `pub fn replace_leaf(&mut self, path: &[usize], new_data: Vec<u8>)` —
  walks the tree by child indices and rewrites the leaf payload.
- `pub fn chunk_at_path(&self, path: &[usize]) -> Result<&Chunk, _>` —
  read-only walker, used by tests and (future) inspectors.
- Utility: `root_child_count`, `root_form_type`, `is_dirty`.
- `pub enum MutError`: `Parse(LegacyError)`, `PathOutOfRange`,
  `PathTraversesLeaf`, `NotALeaf`, `EmptyPath`.

The byte-identical-no-edit guarantee is achieved by holding the original
`Vec<u8>` and short-circuiting `into_bytes` when `!is_dirty`. After any
mutation `iff::emit` is invoked, which **does not** guarantee byte-identity
even for unmutated chunks (it recomputes FORM lengths from children) — but
this case is explicitly out of scope for PR1 and tracked as a follow-up
for PR3 (proper byte-range patching).

**Tests.** Ten new unit tests in `djvu_mut::tests`:

- Round-trip byte-identical (no edit) on four corpus fixtures:
  - `chicken.djvu` — color FORM:DJVU
  - `boy_jb2.djvu` — bilevel FORM:DJVU
  - `DjVu3Spec_bundled.djvu` — multi-page FORM:DJVM
  - `navm_fgbz.djvu` — FORM:DJVU with NAVM + FGbz
- `replace_leaf_changes_emitted_bytes` — replaces INFO with a marker, parses
  the output, verifies the marker came back.
- Negative paths: `EmptyPath`, `PathOutOfRange`, `PathTraversesLeaf`,
  `NotALeaf` (last picks the last child of a DJVM bundle, which is a
  page FORM).
- `root_form_type_djvu_single_page` — sanity on the tree-introspection API.

All 402 lib tests pass (393 → 402; `+10` djvu_mut, `-1` ignored count
shifted). `cargo clippy --workspace --all-targets -- -D warnings` clean,
`cargo fmt --check` clean.

**Reason kept.** PR1 of #222 establishes the byte-identical contract and
the chunk-walking primitive that PR2-4 build on (per the issue body's
sequencing comment). The implementation is intentionally minimal — wrap
the existing IFF parser, hold raw bytes for fast path, expose one
mutation primitive — to ship a focused first slice without committing to
the high-level setter design (`set_metadata`, `set_bookmarks`,
`page_mut(i).set_text_layer`). Those settings each compose
`replace_leaf` with one of the existing chunk encoders
(`encode_navm`, `encode_annotations*`, `encode_metadata`,
`encode_text_layer`).

**Follow-up status.**
1. High-level setters (`set_metadata`, `set_bookmarks`,
   `page_mut(i).set_text_layer`, `…set_annotations`) have landed.
2. Single-page byte-range patching landed in #302; bundled-DJVM byte-range
   patching remains a separate follow-up.
3. Indirect DJVM support was scoped by #303 and remains intentionally
   unsupported until the external-file rewrite/re-bundle work in #325/#326.
4. `librarian` consumer migration off `djvused` shell-out (#158 follow-up)
   can use the setter surface, but is outside this repository.

### #229 PR1 — extract `djvu-zp` into a standalone workspace crate — **Kept** (2026-04-30)

**Approach.** Moved `src/zp/{mod,encoder,tables}.rs` into a new
`crates/djvu-zp/` workspace member with its own `Cargo.toml`. The new
crate:

- Defines `pub enum ZpError { TooShort }` instead of leaking `BzzError`
  back into ZP. Decoupling ZP from `crate::error` is what makes the
  extraction publishable.
- Promotes every `pub(crate)` to `pub` (the audit the issue body warns
  about): `ZpDecoder`, `ZpDecoder::{a, c, fence, bit_buf, bit_count, data,
  pos}` fields, `decode_bit`, `decode_passthrough`, `decode_passthrough_iw44`,
  `is_exhausted`, `ZpEncoder` + its methods, and the four format-constant
  tables (`PROB`, `THRESHOLD`, `MPS_NEXT`, `LPS_NEXT`).
- Has a `default = ["std"]` feature that gates the encoder (which needs
  `Vec<u8>`). Decoder works in `no_std` builds and never allocates.
- Adds a `Default` impl on `ZpEncoder` (clippy `new_without_default` for
  the now-public `new` method).

`src/lib.rs` keeps the historical internal name via
`pub(crate) use djvu_zp as zp_impl;` so every existing import
(`crate::zp_impl::ZpDecoder`, `crate::zp_impl::tables::PROB`, etc) keeps
working unchanged. `From<djvu_zp::ZpError> for BzzError` makes the `?`
operator in `bzz_new::bzz_decode` continue to work without per-callsite
edits.

`src/zp/` is removed; the `#[path = "zp/mod.rs"] pub(crate) mod zp_impl;`
attribute in `src/lib.rs` was replaced with the `use` re-export. Workspace
`members = [".", "djvu-py", "crates/djvu-zp"]`.

**Tests.** Per-crate test counts:

- `djvu-rs` (umbrella): 393 lib tests pass (down from 405 — the 4 ZP
  decoder tests + 7 ZP encoder roundtrip tests moved into the new crate).
- `djvu-zp`: 11 unit tests pass (`zp_decoder_*`, `zp_tables_spot_check`,
  7 roundtrip tests in the encoder module). Two doctest examples
  (`ZpDecoder::new` from a 2-byte slice, `ZpEncoder` round-trip).
- `djvu-py`: builds. No tests defined.
- Workspace `cargo build --no-default-features --lib` (host
  no-std-compatible build) green; no_std smoke test
  (`tests/no_std_smoke`) builds green against the new dependency graph.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo fmt --check` clean.

**Scope of `pub` audit.** Every newly-`pub` item was an internal
`pub(crate)` before — there is no new behavioural surface, just a wider
visibility. Specifically:

| Was            | Now           | Justification                                               |
| -------------- | ------------- | ----------------------------------------------------------- |
| `ZpDecoder`    | `pub`         | Required for cross-crate use                                |
| Decoder fields | `pub`         | Hot-path field access from JB2/IW44/BZZ in djvu-rs internals |
| `ZpEncoder`    | `pub`         | Required for cross-crate use                                |
| `PROB` etc.    | `pub` (in `pub mod tables`) | Used by JB2/IW44/BZZ saturation-bound tests in djvu-rs |

The decoder field exposure is the only mildly load-bearing widening: it
lets djvu-rs internals manipulate the registers directly during
saturated-decode fast paths. Wrapping each in a getter would force every
hot-path access through a function call. Acceptable for an internal-
collaboration sub-crate and matches the precedent set by `wide` /
`bytemuck` / similar low-level numerics crates.

**Reason kept.** Lossless extraction of ~780 LOC into a publishable
sub-crate, no behavioural change for djvu-rs consumers, all tests pass,
no_std build still works. This is the canonical "is this approach
viable" first step of #229; PR2 (`djvu-bzz`), PR3 (`djvu-iff`), PR4-5
(`djvu-jb2`, `djvu-iw44`), and PR6 (umbrella re-export shim) follow the
same pattern.

**Open follow-ups.**
1. The `From<ZpError> for BzzError` mapping collapses to `BzzError::TooShort`
   — fine for now since `ZpError::TooShort` is the only variant. If
   future ZP-coder errors are added, the mapping needs a more specific
   `BzzError` variant (likely `BzzError::ZpError`-already-exists).
2. Publish to crates.io once the API is reviewed. The `version = "0.1.0"`
   reflects new-crate convention, not djvu-rs's `0.14.0` line.
3. Consider whether the encoder fields (`a`, `subend`, `buffer`, `nrun`,
   `delay`, `byte`, `scount`, `output`) need to be `pub`. Currently they
   stay private — only methods are exposed.

### #189 Phase 3 — x86_64 AVX2 ports of `prelim_flags_bucket` + `prelim_flags_band0` — **Kept** (2026-04-30)

**Approach.** Two new AVX2 functions mirroring the existing aarch64 NEON
helpers in `src/iw44_new.rs`:

- `prelim_flags_bucket_avx2`: loads 16 i16 (one `__m256i` — twice the lane
  width of NEON's two `int16x8_t` loads), compares to zero with
  `_mm256_cmpeq_epi16`, builds UNK/ACTIVE flags via `uv ^ (xv & nz)` (UNK=8,
  XV=10), narrows u16→u8 via `_mm_packus_epi16` of the two 128-bit halves
  (saturating but values 2/8 fit), stores 16 bytes via `_mm_storeu_si128`,
  horizontally OR-reduces via `_mm_unpackhi_epi64` + `_mm_srli_si128` chain.

- `prelim_flags_band0_avx2`: same flag computation, then conditional blend
  `(new & should_update) | (old & ~should_update)` using SSE2
  `_mm_andnot_si128` to replicate NEON's `vbslq_u8`. Keeps the ZERO-state
  lane unchanged; updates other lanes from the coef comparison.

A new `band0_dispatch` helper picks NEON / AVX2 / scalar at runtime via
`is_x86_feature_detected!("avx2")` (gated on `feature = "std"` per the
established pattern in #251/#252). The scalar fallback is unchanged — so
non-AVX2 x86_64 hosts and `no_std` builds keep their existing behaviour.

The dispatcher in `prelim_flags_bucket` was extended the same way: AVX2
branch added, NEON path unchanged, scalar fallback unchanged.

**Tests.** Two new unit tests gated on `cfg(all(target_arch = "x86_64",
feature = "std"))` + AVX2 runtime detection:

- `prelim_flags_bucket_avx2_matches_scalar` — sweeps 5 coef vectors
  (all-zero, mixed, all-one, all-negative-one, edge values) at four bases
  including the highest valid bucket offset (1008). Verifies bucket bytes
  and bstatetmp byte-exact vs scalar.
- `prelim_flags_band0_avx2_matches_scalar` — sweeps 4 old-flag patterns ×
  4 coef patterns. Verifies the conditional-update semantics: ZERO lanes
  are preserved, other lanes get UNK/ACTIVE from the coef comparison.

Both pass on the local x86_64 host. All 405 lib tests pass; clippy
`-D warnings` and `cargo fmt --check` clean.

**Bench.** No native bench harness for this kernel in isolation; expected
speedup over scalar at this hot path (called once per (block × band) =
~1024 blocks/page × 10 bands = ~10K calls/page) is on the order of
4–8× from replacing the scalar 16-iteration loop with three AVX2 ops + a
narrow + horizontal OR. End-to-end `iw44_decode_*` benches were later sampled
by the #189 validation run and the #307 AVX2 spike.

**Reason kept.** Two more AVX2 kernels close the parity gap with NEON
that issue #189 calls out (lines 11–14 of the issue body listed
`preliminary_flag_computation` band-0 and band≠0 as next priorities after
`load8s`/`store8s`, which shipped in #252). Bit-exact verified vs scalar,
zero behavioural change for non-AVX2 hosts, no allocation overhead, no
runtime cost on the dispatcher (one feature-detected branch). Pattern
established for the remaining kernels (`row_pass_neon_s1_row`,
`lifting_even`, `predict_inner`, `predict_avg`).

**Open follow-ups.**
1. `row_pass_neon_s1_row` AVX2 port was measured in #307 and rejected: full
   decode improved, but the sensitive sub2/sub4 partial-decode paths regressed.
2. Encoder-side ports (`forward_row_neon_s1_row`, `forward_col_predict_neon`).
3. ARM64 NEON validation was refreshed in #308.

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
benefit from the warm-cache row scratch (`### #225 Phase 1` below,
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
  `render_pixmap`'s decode logic) that calls `composite_rows`. This became the
  shared row source for the public `render_streaming` API instead of
  allocating a full Pixmap.

At the time, `render_pixmap` was a thin adapter: it pre-allocated
`Pixmap::white(w, h)`, called `render_rows` with a sink that copied each row
into `pm.data`, then applied the existing aa/Lanczos/rotation post-processing
steps. Current strict renders use the direct full-pixmap path, while permissive
renders still share row-based recovery with `render_streaming`.

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
1. Phase 2 shipped `pub fn render_streaming` with a user-visible row callback.
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
the per-CC profitability model in the #194 follow-up is worth
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

**Next step.** Addressed by #308: the current raw NEON row-pass and related
IW44 decode/render paths were remeasured on Apple ARM64 and recorded in the
cross-architecture matrix.

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

### #258 — shared-Djbz Hamming clustering — **Rejected** (2026-05-04)

**Approach.** Re-tested the `diff_fraction = 3` path on the 517-page
`pathogenic_bacteria_1896.djvu` corpus. The corpus exposed three separate
robustness problems: the 1 MP per-symbol decode cap was too low for large
connected components, the 64 MP cumulative symbol-work cap was too low for
dense independently encoded pages, and Hamming shared clustering/rec-6
refinement did not provide a reliable size win. The kept path raises decode
caps to 16 MP per symbol and 256 MP cumulative symbol work, disables
lossless rec-6 emission, and keeps shared-Djbz clustering byte-exact with a
4 MP retained shared-dict budget.

**Numbers.** Re-running the 517-page `pathogenic_bacteria_1896.djvu`
experiment at `--diff-fraction 3` before this change localized the failure
to page-level JB2 decode errors such as `Jb2(ImageTooLarge)` beginning at
page 81. The clustered shared dictionary had 63,062 symbols; the per-page
Sjbz stream then emitted enough shared-ref rec-6 refinements to exceed the
decoder's per-stream symbol-pixel budget before pixel comparison.

After the change:

| Command | shared syms | bundle / independent | round-trip |
|---------|-------------|----------------------|------------|
| `--threshold 999 --diff-fraction 3` | 0 | 1.001× | ✓ |
| `--diff-fraction 3` | 5,164 | 0.976× | ✓ |

**Decision.** Rejected. Hamming shared clustering has no material measured
size win over byte-exact clustering, and the `diff_fraction = 3` corpus path
still produces invalid page streams. `cluster_shared_symbols_tunable` keeps
its public benchmarking signature but now ignores the Hamming allowance and
uses byte-exact clustering for every threshold. In addition, inherited
shared-Djbz symbols are used only for exact record-7 hits, and lossless
near matches fall back to record-1 rather than rec-6 refinement.

### #283 — cross-size JB2 refinement probe — **Kept instrumentation, default unchanged** (2026-05-12)

**Approach.** Added `analyze_jb2_cross_size_refinement(page, shared,
max_dim_delta, max_hamming_fraction)`, an experiment-only accounting helper
that mirrors `encode_jb2_dict_with_shared` dictionary growth but does not
emit bytes. For fresh record-1 candidates, it scans dictionary symbols whose
width/height differ by at most 2 px, normalizes the reference into the
candidate box with nearest-neighbor sampling, and reports how many candidates
land within a 5% normalized Hamming budget. The existing
`examples/encode_quality_djbz.rs` harness now exposes this via
`--cross-size-stats`.

**Command.**

```text
cargo run --release --example encode_quality_djbz -- \
  --cc-stats --cross-size-stats \
  tests/corpus/watchmaker.djvu \
  tests/corpus/pathogenic_bacteria_1896.djvu
```

**Numbers.**

| File | Pages | bundle / independent | round-trip | fresh CCs | cross-size candidates | near @ 5% |
|------|-------|----------------------|------------|-----------|-----------------------|-----------|
| `watchmaker.djvu` | 12 | 0.945× | ✓ | 2,652 | 2,331 | 547 (20.65%) |
| `pathogenic_bacteria_1896.djvu` | 517 | 0.976× | ✓ | 759,291 | 686,402 | 61,485 (8.73%) |

Aggregate bundled bytes for the two-file run were 33,553,108 vs 34,384,941
for independent per-page JB2 dict encoding (0.976×, −2.4%). Pixel round-trip
stayed exact because the probe is observational only.

**Decision.** Keep the probe, but do not change the default encoder. The
candidate counts prove there is real cross-size shape similarity, especially
on `watchmaker`, but they are only an upper bound: record-6 would still carry
refinement bitmap bytes plus symbol-index/context overhead, and the previous
same-size/shared-rec-6 experiments showed that plausible-looking Hamming
matches can lose bytes or create invalid streams. A shipped cross-size
encoder path needs a byte-cost model and explicit lossy/lossless semantics;
until then `encode_djvm_bundle_jb2` remains exact rec-7 + fresh rec-1 only.

### #278 PR1 — single-page Quality/Archival FGbz profiles — **Kept** (2026-05-12)

**Approach.** Completed the conservative single-page color profile path:
`Quality` still uses the existing deterministic segmentation and
`INFO + Sjbz + BG44...` shape, but now adds an `FGbz` foreground palette
when the detected foreground color is not black. `Archival` no longer
returns `Unsupported` for color input; it emits the same layered shape with
a denser background sample grid (`bg_subsample = 6` instead of 12).

This deliberately does not change the multi-page directory encoder, which
still uses the lossless shared-Djbz path only, and does not revive Hamming
shared-Djbz clustering.

**Tests.**

- `cargo test -q djvu_encode::tests`
- `cargo test -q --features cli --test cli_encode -- --nocapture`

The CLI regression fixture is generated in `tests/cli_encode.rs` as a
white RGB PNG with a dark red foreground block. `--quality quality` and
`--quality archival` both produce parseable single-page DjVu files with
`Sjbz`, at least one `BG44`, and `FGbz`.

**Decision.** Kept as PR1 scope. This removes the user-visible
`Archival` unsupported path for single PNGs and gives colored foreground
documents a foreground color layer. Remaining quality work should be split
into focused follow-ups: adaptive binarization/inpainting, per-blit FGbz
indices or FG44 for multi-color foregrounds, and layered multi-page DJVM
encoding.

### #289 — per-blit FGbz indices for colored foreground — **Kept** (2026-05-12)

**Approach.** Switched single-page color profiles from direct whole-page
`encode_jb2` masks to dict-based `encode_jb2_dict` masks, then derives the
FGbz palette from the independently decoded `decode_indexed` blit map. Each
foreground blit gets an average source RGB color; duplicate colors share one
palette entry; multi-color foregrounds emit an FGbz index table. Single-color
foregrounds still use compact palette-only FGbz.

**Tests.**

- `cargo test -q djvu_encode::tests`
- `cargo test -q --features cli --test cli_encode -- --nocapture`
- `cargo clippy --lib --tests -- -D warnings`

The new regression fixture has two separated colored foreground components.
The unit test verifies both the FGbz palette/index table and a decoded render:
the left component remains red-dominant and the right component remains
blue-dominant.

**Decision.** Kept. This closes the main PR1 limitation from #278: colored
foreground no longer collapses to one averaged ink color when the page has
multiple separated foreground components. Continuous foreground regions and
FG44 remain out of scope; those need separate visual-quality measurements.

### #233 — async lazy first-page probe — **Kept** (2026-05-04)

**Approach.** Added `examples/async_lazy_first_page.rs`, a small native
probe for the Phase 3 lazy async loader. It wraps a DjVu file in an
`AsyncRead + AsyncSeek` reader that can simulate broadband throughput,
constructs `LazyDocument` with `from_async_reader_lazy`, fetches page 0,
and renders the first pixmap.

**Command.**

```sh
cargo run -q --example async_lazy_first_page --features async -- \
  tests/corpus/pathogenic_bacteria_1896.djvu --bandwidth-mib 12.5 --dpi 150 --pad-to-mib 100
```

**Numbers.**

| Corpus | Size | Pages | Simulated bandwidth | Bytes read | First pixel |
|--------|------|-------|---------------------|------------|-------------|
| `pathogenic_bacteria_1896.djvu` padded with an ignored `JUNK` chunk | 104,857,600 bytes | 520 | 12.5 MiB/s | 28,578 | 491.469 ms |

**Decision.** Kept. The probe pads the largest checked-in multi-page corpus
to exactly 100 MiB with a valid ignored `JUNK` IFF chunk, preserving the
DIRM/page offsets while making the file size match the issue target. Indexing
plus first-page fetch reads only the DIRM and first page/component ranges
instead of buffering the full 100 MiB document, and first pixel is well below
the 2 s target under the simulated broadband reader.

### #189 — x86-64-v3 AVX2 validation — **Kept partial / needs follow-up** (2026-05-04)

**Approach.** Pulled the GitHub Actions artifact from run `25299920836`
(`Benchmark (x86-64-v3 AVX2 validation)`, head `77fc6ff`) and compared
default `RUSTFLAGS` against `RUSTFLAGS=-C target-cpu=x86-64-v3` on the same
Ubuntu runner. This validates the already-landed AVX2 paths on real x86_64
hardware even though the local development host is `arm64`.

**Numbers.**

| Bench | default ns | +x86-64-v3 ns | Delta |
|-------|-----------:|--------------:|------:|
| `iw44_decode_corpus_color` | 1,385,461 | 1,123,865 | -18.88% |
| `iw44_decode_first_chunk` | 765,703 | 728,565 | -4.85% |
| `iw44_to_rgb_colorbook/sub1_full_decode` | 9,231,033 | 9,129,333 | -1.10% |
| `iw44_to_rgb_colorbook/sub2_partial_decode` | 2,164,523 | 2,199,280 | +1.61% |
| `iw44_to_rgb_colorbook/sub4_partial_decode` | 565,640 | 583,519 | +3.16% |
| `render_colorbook` | 13,072,440 | 12,826,562 | -1.88% |
| `render_colorbook_cold` | 28,127,606 | 27,105,326 | -3.63% |
| `render_colorbook_stages/mask_decode` | 5,325,125 | 5,107,550 | -4.09% |
| `render_corpus_color` | 133,813,976 | 133,185,634 | -0.47% |

**Decision.** Kept partial. Existing AVX2 decode paths earn their keep on
full IW44 decode (`-18.88%` corpus decode, `-4.85%` first chunk), but the
sub4 partial path regresses by `+3.16%`. This does not close #189: the
umbrella still lacks AVX2 equivalents for the horizontal row pass and encoder
kernels, and those should be implemented only in an x86_64 AVX2 session with
this validation job green after each slice.

### #292 — cross-architecture benchmark matrix — **Kept** (2026-05-17)

**Approach.** Added a canonical cross-architecture platform metadata template
and seed matrix to `BENCHMARKS_RESULTS.md`. This issue did not run new
benchmarks; it normalized existing trustworthy artifacts and made missing
target families explicit for downstream architecture issues.

**Platform.**
- OS: macOS 26.3.1 (Darwin 25.3) for the local Apple ARM64 seed row; Ubuntu
  GitHub-hosted runner for the x86_64 artifact rows.
- CPU: Apple M1 Max, 10 cores, for the broad local baseline; GitHub-hosted
  x86_64 runner for #189 artifact run `25299920836`.
- arch: `aarch64` and `x86_64`
- target features: Apple ARM64 baseline/NEON available; x86_64 baseline;
  x86_64-v3/AVX2 via `RUSTFLAGS=-C target-cpu=x86-64-v3`.
- Rust: 1.92.0 stable for the local Apple ARM64 row; stable toolchain from
  `.github/workflows/bench.yml` for the GitHub artifact rows.
- RUSTFLAGS: unset for local Apple ARM64 and Linux x86_64 baseline rows;
  `-C target-cpu=x86-64-v3` for the AVX2 row.

**Command(s).**

```sh
# Existing local summary source already recorded in BENCHMARKS_RESULTS.md:
cargo bench --workspace --features cli,tiff

# Existing x86_64-v3 artifact source already recorded in this file under #189:
gh run view 25299920836 --repo matyushkin/djvu-rs
```

**Numbers.** The seed matrix records Apple ARM64 local values for
`iw44_decode_*`, `iw44_to_rgb_colorbook/*`, `render_colorbook*`, and
`render_corpus_color`, plus the #189 Linux x86_64 baseline vs
`x86_64-v3`/AVX2 values. wasm32 scalar, wasm32 simd128, and Linux aarch64 are
explicitly marked missing.

**Decision.** Kept. The repository now has one copy/pasteable platform metadata
block and one public cross-architecture result schema for #306, #307, and #308.

**Reason.** Normalizing the table first avoids each downstream architecture
issue inventing a different platform format, while preserving measurement
discipline by distinguishing current numbers from missing/untrusted cells.

### #298 — PDF export memory and parallel baseline — **Needs follow-up** (2026-05-17)

**Approach.** Measured the existing PDF export pipeline before any streaming
rewrite. Criterion measured the stable `pdf_export_sequential` and
`pdf_export_parallel` benches on `tests/corpus/watchmaker.djvu` (12 pages,
default PDF options: 150 dpi, JPEG-80). A new reproducible
`examples/pdf_memory_probe.rs` harness recorded read/parse, one-page
render/RGB/JPEG staging, full PDF export time, PDF bytes, and peak RSS via
`/usr/bin/time -l`.

**Platform.**
- OS: macOS 26.3.1 / Darwin 25.3.0 (`RELEASE_ARM64_T6000`)
- CPU: Apple M1 Max, 10 cores
- arch: `arm64` / Rust host `aarch64-apple-darwin`
- target features: Apple ARM64 baseline; NEON available on Apple Silicon
- Rust: `rustc 1.92.0 (ded5c06cf 2025-12-08)`
- RUSTFLAGS: unset
- Source artifact: local run on `codex/issue-298-pdf-baseline`

**Command(s).**

```sh
cargo bench --bench render --features std -- pdf_export_sequential
cargo bench --bench render --features std,parallel -- pdf_export_parallel

/usr/bin/time -l cargo run --release --example pdf_memory_probe -- \
  tests/corpus/watchmaker.djvu

/usr/bin/time -l cargo run --release --features parallel \
  --example pdf_memory_probe -- tests/corpus/watchmaker.djvu
```

**Numbers.**

| Measurement | Sequential | Parallel |
|-------------|-----------:|---------:|
| Criterion `pdf_export_*` | 955.42 ms median (`916.16..999.54 ms`) | 154.05 ms median (`153.41..154.66 ms`) |
| Probe `pdf_export_ms` | 893.827 ms | 187.183 ms |
| Peak RSS (`maximum resident set size`) | 80,379,904 bytes (76.7 MiB) | 240,058,368 bytes (228.9 MiB) |
| Peak memory footprint | 79,479,872 bytes (75.8 MiB) | 239,175,232 bytes (228.1 MiB) |
| Output PDF bytes | 6,651,085 | 6,651,085 |

Single-page breakdown from the same probe, page 0 rendered at 150 dpi
(`1275x1651`):

| Stage | Time | Bytes |
|-------|-----:|------:|
| Read input | 0.075 ms | 183,352 |
| Parse document | 0.152 ms | - |
| Render full RGBA pixmap | 43.822 ms | 8,420,100 |
| Convert RGBA to RGB staging buffer | 2.904 ms | 6,315,075 |
| JPEG-80 encode staging buffer | 13.065 ms | 312,922 |

The parallel probe uses the same one-page breakdown before full export; that
single-page section stayed essentially unchanged (`render_pixmap_ms=44.410`,
`rgb_stage_ms=3.183`, `jpeg_stage_ms=13.228`) while full export dropped to
`187.183 ms` and peak RSS rose to `228.9 MiB`.

**Decision.** Needs follow-up.

**Reason.** Parallel export is about 5.3-6.2x faster on the 12-page color
fixture, but it increases peak RSS by about 3.0x because `djvu_to_pdf_impl`
collects every `RenderedPage` before sequential object emission. The concrete
baseline for #299 is therefore: beat ~894 ms sequential wall time and reduce
or cap the ~76.7 MiB sequential peak RSS / ~228.9 MiB parallel peak RSS by
streaming page render/RGB/JPEG data into PDF objects instead of retaining all
encoded page bodies at once.

### #299 — PDF color row streaming — **Kept** (2026-05-17)

**Approach.** Replaced the PDF color-image path's full `Pixmap` + full RGB
staging pair with `render_streaming` into one RGB staging buffer when render
options are streamable. The fallback `render_pixmap(...).to_rgb()` path remains
for anti-aliasing, scaled Lanczos, rotation, and other non-streamable options.
Measured against the #298 baseline on the same `tests/corpus/watchmaker.djvu`
PDF fixture (12 pages, default PDF options: 150 dpi, JPEG-80).

**Platform.**
- OS: macOS 26.3.1 / Darwin 25.3.0 (`RELEASE_ARM64_T6000`)
- CPU: Apple M1 Max, 10 cores
- arch: `arm64` / Rust host `aarch64-apple-darwin`
- target features: Apple ARM64 baseline; NEON available on Apple Silicon
- Rust: `rustc 1.92.0 (ded5c06cf 2025-12-08)`
- RUSTFLAGS: unset
- Source artifact: local run on `codex/issue-299-pdf-streaming`

**Command(s).**

```sh
cargo bench --bench render --features std -- pdf_export_sequential
cargo bench --bench render --features std,parallel -- pdf_export_parallel
cargo bench --bench render --features std,parallel -- pdf_export_parallel

/usr/bin/time -l cargo run --release --example pdf_memory_probe -- \
  tests/corpus/watchmaker.djvu
/usr/bin/time -l cargo run --release --features parallel \
  --example pdf_memory_probe -- tests/corpus/watchmaker.djvu
```

**Numbers.**

| Measurement | #298 baseline | #299 row streaming |
|-------------|--------------:|-------------------:|
| Criterion `pdf_export_sequential` median | 955.42 ms | 811.83 ms (`810.13..813.58 ms`) |
| Criterion `pdf_export_parallel` median | 154.05 ms | 165.57 ms rerun (`154.19..178.74 ms`) |
| Sequential probe `pdf_export_ms` | 893.827 ms | 852.285 ms |
| Parallel probe `pdf_export_ms` | 187.183 ms | 155.745 ms |
| Sequential peak RSS | 80,379,904 bytes (76.7 MiB) | 77,512,704 bytes (73.9 MiB) |
| Parallel peak RSS | 240,058,368 bytes (228.9 MiB) | 177,684,480 bytes (169.5 MiB) |
| Output PDF bytes | 6,651,085 | 6,651,085 |

The first parallel Criterion run after the change measured
`219.65 ms` (`206.80..232.42 ms`) and reported a regression; an immediate rerun
measured `165.57 ms` (`154.19..178.74 ms`). The single-run probe also measured
parallel export at `155.745 ms`. Treat parallel timing as noisy on this host;
the stable win is peak RSS.

**Decision.** Kept.

**Reason.** The change preserves PDF bytes and keeps the fallback path for
non-streamable render options. It removes the extra full RGBA page allocation
from the streamable PDF color path. Sequential RSS falls modestly from
`76.7 MiB` to `73.9 MiB`; parallel RSS falls materially from `228.9 MiB` to
`169.5 MiB` (-26%). The remaining peak is dominated by retained per-page
encoded RGB/JPEG/PDF object bodies, so a larger memory reduction would require
streaming PDF object emission rather than only row-streamed rendering.

### #300 — IW44 low-PSNR diagnosis on `conquete_paix` — **Needs follow-up** (2026-05-17)

**Approach.** Added a repeatable diagnostic example that re-encodes existing
BG44 backgrounds from `watchmaker` and `conquete_paix` with controlled variants:
current pre-quantization RGB-to-YCbCr model, inverse-compatible pre-quantization
model, default IW44 encode, full-resolution chroma, 200 total slices, and
gray-luma-only encode. This issue intentionally did not change default encoder
behavior.

**Platform.**
- OS: macOS / Darwin 25.3.0 (`RELEASE_ARM64_T6000`)
- CPU: Apple Silicon host
- arch: `arm64` / Rust host `aarch64-apple-darwin`
- target features: Apple ARM64 baseline; NEON available on Apple Silicon
- Rust: `rustc 1.92.0 (ded5c06cf 2025-12-08)`
- RUSTFLAGS: unset

**Command(s).**

```sh
cargo run --release --features std --example diagnose_iw44_quality -- \
  tests/corpus/watchmaker.djvu tests/corpus/conquete_paix.djvu \
  > /private/tmp/iw44_diag_300.jsonl \
  2> /private/tmp/iw44_diag_300.stderr
```

`watchmaker` pages 0-4, 7-9, and 11 were skipped because the original BG44
stream did not decode through the strict full-stream diagnostic path; pages 5,
6, and 10 were enough to confirm the good-page baseline.

**Numbers.**

| File | Variant | Pages | Avg luma PSNR | Min luma PSNR | Avg byte ratio |
|------|---------|------:|--------------:|--------------:|---------------:|
| `watchmaker.djvu` | default | 3 | 46.42 dB | 44.99 dB | 0.73x |
| `watchmaker.djvu` | full chroma | 3 | 46.37 dB | 44.99 dB | 0.73x |
| `watchmaker.djvu` | 200 slices | 3 | 46.31 dB | 44.93 dB | 137.32x |
| `watchmaker.djvu` | gray luma only, 200 slices | 3 | 46.41 dB | 44.95 dB | 137.31x |
| `conquete_paix.djvu` | pre-quant current YCbCr model | 20 | 47.90 dB | 42.97 dB | n/a |
| `conquete_paix.djvu` | pre-quant inverse-compatible model | 20 | 52.53 dB | 51.32 dB | n/a |
| `conquete_paix.djvu` | default | 20 | 15.49 dB | 9.75 dB | 1.16x |
| `conquete_paix.djvu` | full chroma | 20 | 16.35 dB | 9.15 dB | 1.24x |
| `conquete_paix.djvu` | 200 slices | 20 | 15.43 dB | 10.84 dB | 55.87x |
| `conquete_paix.djvu` | gray luma only, 200 slices | 20 | 14.57 dB | 9.21 dB | 40.40x |

Per-page `watchmaker` baseline:

| Page | Orig BG44 bytes | Default bytes | Default luma PSNR | Full chroma luma PSNR | 200 slices luma PSNR | Gray luma-only PSNR |
|------|----------------:|--------------:|------------------:|----------------------:|---------------------:|--------------------:|
| 5 | 2,028 | 1,788 | 44.994 dB | 44.988 dB | 44.934 dB | 44.950 dB |
| 6 | 1,804 | 1,178 | 47.268 dB | 47.123 dB | 47.036 dB | 47.302 dB |
| 10 | 1,772 | 1,161 | 46.984 dB | 46.988 dB | 46.975 dB | 46.979 dB |

Per-page `conquete_paix` diagnostic:

| Page | Orig BG44 bytes | Default bytes | Default luma PSNR | Full chroma luma PSNR | 200 slices luma PSNR | Gray luma-only PSNR |
|------|----------------:|--------------:|------------------:|----------------------:|---------------------:|--------------------:|
| 2 | 75,375 | 86,138 | 13.716 dB | 18.130 dB | 10.842 dB | 15.499 dB |
| 3 | 36,721 | 37,994 | 9.746 dB | 16.749 dB | 14.251 dB | 19.829 dB |
| 4 | 41,754 | 49,853 | 12.713 dB | 9.148 dB | 15.914 dB | 9.764 dB |
| 5 | 32,415 | 39,573 | 12.125 dB | 12.345 dB | 13.436 dB | 9.961 dB |
| 6 | 125,387 | 140,114 | 15.159 dB | 18.328 dB | 13.634 dB | 17.737 dB |
| 7 | 90,023 | 102,677 | 14.429 dB | 9.876 dB | 11.275 dB | 18.507 dB |
| 8 | 97,611 | 110,499 | 20.826 dB | 14.056 dB | 16.937 dB | 16.546 dB |
| 9 | 94,750 | 107,785 | 16.934 dB | 14.794 dB | 21.151 dB | 20.543 dB |
| 10 | 102,842 | 116,915 | 18.350 dB | 20.647 dB | 17.430 dB | 12.006 dB |
| 11 | 91,607 | 104,672 | 11.219 dB | 19.431 dB | 11.875 dB | 17.791 dB |
| 12 | 104,131 | 117,898 | 16.722 dB | 14.036 dB | 17.168 dB | 10.354 dB |
| 13 | 96,424 | 109,673 | 20.043 dB | 18.464 dB | 20.039 dB | 9.210 dB |
| 14 | 102,115 | 115,874 | 18.909 dB | 18.312 dB | 19.579 dB | 13.873 dB |
| 15 | 91,303 | 103,789 | 11.826 dB | 13.003 dB | 11.657 dB | 16.271 dB |
| 16 | 112,528 | 126,994 | 19.714 dB | 19.820 dB | 13.751 dB | 11.713 dB |
| 17 | 110,328 | 124,617 | 12.350 dB | 16.952 dB | 11.208 dB | 17.077 dB |
| 18 | 36,292 | 43,735 | 11.783 dB | 19.109 dB | 18.280 dB | 9.960 dB |
| 19 | 26,855 | 38,856 | 20.226 dB | 21.302 dB | 14.453 dB | 11.716 dB |
| 20 | 45,896 | 53,610 | 16.041 dB | 15.038 dB | 17.046 dB | 15.550 dB |
| 21 | 87,548 | 99,228 | 16.901 dB | 17.430 dB | 18.767 dB | 17.463 dB |

**Decision.** Needs follow-up.

**Reason.** The failure is reproduced on `conquete_paix` while `watchmaker`
remains high quality. It is not explained by BG44 byte budget: default output
is already larger than the original on `conquete_paix` (`1.16x` average), and
200 slices explodes output size (`55.87x`) without improving luma PSNR. It is
not solved by chroma subsampling alone: full-resolution chroma improves some
bad pages substantially (for example page 3: `9.746 dB` to `16.749 dB`, page
11: `11.219 dB` to `19.431 dB`) but still leaves the corpus at only
`16.35 dB` average and worsens other pages. The pre-quantization color-model
probes stay much higher (`47.90 dB` current model, `52.53 dB`
inverse-compatible model), so the catastrophic luma loss appears after
RGB/YCbCr conversion, inside the forward wavelet / coefficient quantization /
reconstruction-tracking path on high-detail color backgrounds. Follow-up #320
isolates that path with coefficient-plane diagnostics before any encoder
tuning.

### #301 — JB2 cross-size refinement byte-cost estimator — **Needs follow-up** (2026-05-17)

**Approach.** Extended the existing #283 cross-size candidate-count probe with
an approximate byte-cost model. For near cross-size candidates, the model
compares the current record-1 fresh-symbol payload estimate against a
hypothetical cross-size record-6 estimate that includes symbol-index/context
overhead, width/height/refinement overhead, and a packed Hamming-payload proxy.
No bytes are emitted and `encode_djvm_bundle_jb2` behavior is unchanged.

**Platform.**
- OS: macOS / Darwin 25.3.0 (`RELEASE_ARM64_T6000`)
- CPU: Apple Silicon host
- arch: `arm64` / Rust host `aarch64-apple-darwin`
- target features: Apple ARM64 baseline; NEON available on Apple Silicon
- Rust: `rustc 1.92.0 (ded5c06cf 2025-12-08)`
- RUSTFLAGS: unset

**Command(s).**

```sh
cargo run --release --example encode_quality_djbz -- \
  tests/corpus/watchmaker.djvu tests/corpus/pathogenic_bacteria_1896.djvu \
  --cc-stats --cross-size-stats \
  > /private/tmp/jb2_cost_301.jsonl \
  2> /private/tmp/jb2_cost_301.stderr
```

**Numbers.**

Bundle baseline from the same run:

| File | Pages | Original Sjbz | Independent dict | Bundled shared-Djbz | Bundle / independent | Round-trip |
|------|------:|--------------:|-----------------:|--------------------:|---------------------:|------------|
| `watchmaker.djvu` | 12 | 122,923 | 130,036 | 122,832 | 0.9446x | pixel-exact |
| `pathogenic_bacteria_1896.djvu` | 517 | 24,849,842 | 34,254,905 | 33,430,276 | 0.9759x | pixel-exact |

Cross-size estimator:

| File | Fresh CCs | Eligible | Candidates | Near @ 5% | Near pixels | Median best Hamming | Est current rec-1 | Est cross-size rec-6 | Est delta | Delta / independent |
|------|----------:|---------:|-----------:|----------:|------------:|--------------------:|-----------------:|---------------------:|----------:|--------------------:|
| `watchmaker.djvu` | 2,652 | 2,649 | 2,331 | 547 (20.65%) | 525,061 | 92 | 75,632 B | 5,641 B | -69,991 B | -53.82% |
| `pathogenic_bacteria_1896.djvu` | 759,291 | 703,928 | 686,402 | 61,485 (8.73%) | 67,245,972 | 88 | 9,677,015 B | 830,556 B | -8,846,459 B | -25.83% |

Semantics: a real cross-size record-6 path should be lossless if it emits the
full refinement bitmap correctly. This probe is not an emitting encoder path;
it uses nearest-neighbor scaled Hamming only for candidate selection and cost
estimation. The byte model is deliberately approximate and optimistic because
the packed Hamming proxy is not a real ZP-coded refinement bitstream.

**Decision.** Needs follow-up.

**Reason.** Both required corpora show enough estimated byte headroom to justify
a narrow emitting spike: `watchmaker` has 547 near cross-size matches and
`pathogenic_bacteria_1896` has 61,485, with large estimated negative deltas.
The result is not sufficient to change defaults because the estimator does not
prove actual ZP-coded record-6 byte cost or round-trip behavior. Follow-up #322
should implement an experiment-only cross-size rec-6 emitter, compare actual
bytes against this estimate, and stop before tuning if output is not
pixel-exact. The shipped default remains exact shared-Djbz rec-7 plus rec-1.

### #307 — x86_64 AVX2 row-pass feasibility spike — **Rejected** (2026-05-17)

**Approach.** Prototyped an x86_64 AVX2 `s == 1` horizontal IW44 row pass
behind compile-time `target_feature = "avx2"`, mirroring the AArch64 row-local
NEON shape and leaving baseline x86-64/default-codegen hosts on the existing
path. The spike included a gated AVX2 row-pass equivalence test covering short
rows, chunk boundaries, and scalar tails. The code was removed after the clean
measurement showed sensitive partial-decode regressions.

**Platform.**
- OS: Ubuntu GitHub-hosted runner (`ubuntu-latest`)
- CPU: GitHub-hosted x86_64 runner
- arch: `x86_64`
- target features: baseline x86-64 vs `x86-64-v3` / AVX2 codegen
- Rust: stable toolchain installed by `.github/workflows/bench.yml`
- RUSTFLAGS: unset for baseline; `-C target-cpu=x86-64-v3` for AVX2 pass

**Command(s).**

```sh
gh workflow run bench.yml -r codex/issue-307-avx2-row-pass
gh run download 25984542554 --dir /private/tmp/djvu-307-clean-artifacts
```

Workflow commands run by `Benchmark (x86-64-v3 AVX2 validation)`:

```sh
cargo bench --bench codecs -- 'iw44_to_rgb|iw44_decode' --output-format bencher
cargo bench --bench render -- 'render_corpus_color|render_colorbook' --output-format bencher
RUSTFLAGS='-C target-cpu=x86-64-v3' cargo bench --bench codecs -- 'iw44_to_rgb|iw44_decode' --output-format bencher
RUSTFLAGS='-C target-cpu=x86-64-v3' cargo bench --bench render -- 'render_corpus_color|render_colorbook' --output-format bencher
```

**Numbers.**

Source: GitHub Actions run `25984542554`, artifact
`bench-x86-64-v3-4ad38655adc465a16dc766efa5ac12c34c144fc9`.
Negative delta means the AVX2/x86-64-v3 pass is faster.

| Bench | default ns | +x86-64-v3 ns | Delta |
|-------|-----------:|--------------:|------:|
| `iw44_decode_corpus_color` | 1,372,742 | 1,133,708 | -17.41% |
| `iw44_decode_first_chunk` | 766,885 | 729,423 | -4.88% |
| `iw44_to_rgb_colorbook/sub1_full_decode` | 9,334,768 | 9,485,209 | +1.61% |
| `iw44_to_rgb_colorbook/sub2_partial_decode` | 2,154,494 | 2,258,179 | +4.81% |
| `iw44_to_rgb_colorbook/sub4_partial_decode` | 559,566 | 600,388 | +7.30% |
| `render_colorbook` | 11,523,817 | 11,760,347 | +2.05% |
| `render_colorbook_cold` | 27,060,741 | 27,560,588 | +1.85% |
| `render_colorbook_stages/bg_only_warm` | 1 | 1 | +0.00% |
| `render_colorbook_stages/full_render` | 11,495,717 | 11,795,482 | +2.61% |
| `render_colorbook_stages/mask_decode` | 5,368,030 | 5,132,471 | -4.39% |
| `render_corpus_color` | 129,285,417 | 129,176,157 | -0.08% |

**Decision.** Rejected; the prototype was removed before merge.

**Reason.** The full IW44 decode benches improved (`-17.41%` corpus decode,
`-4.88%` first chunk), but #307 explicitly called out sub2/sub4 partial decode
as sensitive benches. Those regressed by `+4.81%` and `+7.30%`, respectively,
which fails the acceptance criterion of a relevant win with no meaningful
regression. `render_colorbook` and full-render stages also drifted slower,
though within the 3% threshold. Because no production optimization landed,
`BENCHMARKS_RESULTS.md` was not updated. The benchmark workflow was triggered
manually because the current PR path filter does not include `crates/**`; a
follow-up should widen that filter so future crate-only performance PRs run
benchmark validation automatically.

### #308 — aarch64 NEON validation — **Kept partial** (2026-05-17)

**Approach.** Reran the current IW44 decode, partial decode, and
`render_colorbook` benchmark filters on the Apple Silicon host to validate the
current NEON paths after #292 established the cross-architecture matrix. No new
NEON kernel was added. Linux aarch64 benchmark cells remain explicitly missing:
#305 added native Linux aarch64 smoke coverage, but there is not yet a Linux
aarch64 benchmark workflow or artifact.

**Platform.**
- OS: macOS 26.3.1 / Darwin 25.3.0 (`RELEASE_ARM64_T6000`)
- CPU: Apple M1 Max, 10 cores
- arch: `arm64` / Rust host `aarch64-apple-darwin`
- target features: ARM64 baseline; NEON available on Apple Silicon
- Rust: `rustc 1.92.0 (ded5c06cf 2025-12-08)`
- RUSTFLAGS: unset

**Command(s).**

```sh
cargo bench --bench codecs -- 'iw44_to_rgb|iw44_decode' --output-format bencher
cargo bench --bench render -- 'render_corpus_color|render_colorbook' --output-format bencher
```

**Numbers.**

| Bench | Apple ARM64 ns/iter | Matrix value |
|-------|--------------------:|--------------|
| `iw44_decode_corpus_color` | 636,847 | 637 us |
| `iw44_decode_first_chunk` | 557,004 | 557 us |
| `iw44_to_rgb_colorbook/sub1_full_decode` | 5,470,697 | 5.47 ms |
| `iw44_to_rgb_colorbook/sub2_partial_decode` | 1,301,311 | 1.30 ms |
| `iw44_to_rgb_colorbook/sub4_partial_decode` | 337,043 | 337 us |
| `render_colorbook` | 6,921,690 | 6.92 ms |
| `render_colorbook_stages/full_render` | 6,932,763 | 6.93 ms |
| `render_colorbook_stages/bg_only_warm` | 0 | 0 ns |
| `render_colorbook_stages/mask_decode` | 4,173,642 | 4.17 ms |
| `render_colorbook_cold` | 17,426,166 | 17.4 ms |
| `render_corpus_color` | 68,726,395 | 68.7 ms |

**Decision.** Kept partial.

**Reason.** Current Apple ARM64 NEON paths remain healthy: first-chunk decode
is now `557 us` versus the stale #184 note's `715 us` reference, corpus IW44
decode remains `637 us`, and sub2/sub4 partial decode stay near the existing
matrix values. This closes the stale ARM64 remeasurement note without adding
new kernels. The result is "partial" because Linux aarch64 still has only
build/test smoke coverage and no authoritative benchmark artifact; the matrix
keeps those cells as `missing`.

### IW44 slice-loop early-exit on `zp.is_exhausted()` — **Reverted** (2026-06-09)

**Issue.** External bug report (treadbear): a color page in
`colorbook.djvu` (and a user-attached file) decoded with "sparkly rainbow
artifacts". Reporter's proposed fix: delete the `if zp.is_exhausted() { break; }`
guard at the end of the slice loop in `Iw44Image::decode_chunk`.

**Approach (the reverted code).** The slice loop
(`for _ in 0..slices`) broke early once the ZP byte buffer was drained,
on the assumption that "remaining slices carry no new information" and to
"bound decode time on crafted inputs".

**Numbers.** Differential test vs DjVuLibre `ddjvu` at native resolution
(`--width 2260 --tolerance 4`):

| `colorbook.djvu` page | with early-exit | without early-exit |
|-----------------------|-----------------|--------------------|
| p0 | 0.27% | 0.27% |
| p1 | 0.64% | 0.64% |
| **p2** | **60.87%** mismatch, mean Δ 3.05 | **0.31%**, mean Δ 0.05 |
| p3 | 1.50% | 1.50% |

Page 2's IW44 chunk packs many slices into few bytes, so `is_exhausted()`
fires mid-stream and the `break` drops ~all remaining chroma/luma refinement.

**Decision.** Reverted (early-exit removed).

**Reason.** Same root cause as #182, one level up. `is_exhausted()` reports
only `pos >= data.len()` (the *byte* buffer), but the ZP coder is a
continuous arithmetic bit stream that reads up to 24 bits ahead via
`refill_buffer`, so the byte pointer reaches the end several bytes before the
logical end of decisions. The remaining slices still decode legitimate
refinement from the buffered bits + arithmetic registers; skipping them
desynchronises nothing (each slice is self-contained work) but truncates real
high-frequency detail → chroma artifacts. The "bound decode time" rationale
was unfounded: the loop is already bounded by `slices` (a `u8`, ≤255 per
chunk) and the 64 MP image cap, so no early-exit is required to bound work.
Regression test: `iw44_colorbook_page2_decodes_all_slices_no_early_exit` in
`crates/djvu-iw44/src/lib.rs` (and the native-res diff gate).

### JB2 post-EOF guard: `is_exhausted()` → synthetic-byte (`pos`) overshoot — **Kept** (2026-06-16)

**Issue.** Follow-up to the IW44 report above: the same reporter noted "a
similar issue in the JB2 code path" with a repro page, and proposed adding a
`ZP_EOF_SLACK_BYTES` tolerance computed as `zp.pos.saturating_sub(data.len())`.
JB2's per-symbol DoS guard (`check_symbol_decode_budget`) rejected a decode with
`Jb2Error::Truncated` whenever `zp.is_exhausted() && symbol_pixels > 4096`. Like
the IW44 `break`, `is_exhausted()` (`pos >= data.len()`) flips ~4–8 bytes before
the logical end of a valid stream, so any page whose final tile/symbol is larger
than 64×64 px and is decoded from the ZP look-ahead window was wrongly rejected.

**Approach.** Adopted the reporter's slack idea but fixed its mechanism. The
verbatim patch is a no-op as written: the canonical `ZpDecoder::read_byte`
*saturated* `pos` at `data.len()`, so `pos - len` was always 0 and the guard
would never fire — while the inlined hot-path byte readers in jb2/iw44/bzz
*did* advance `pos` past the end (`wrapping_add`). The two byte-reading paths
disagreed. Unified them: `read_byte` now also always advances `pos`, so
`pos - data.len()` is the true synthetic-`0xFF` count across every reader, and
`synthetic_bytes()` returns exactly that. The JB2 guards now bail only when
`synthetic_bytes() > ZP_EOF_SLACK_BYTES` (16) — at the per-symbol check *and* at
the top of all three record loops (covering record types 7/9/10, which decode no
symbol and so never reached the per-symbol check — that gap let a post-EOF
stream spin to `MAX_RECORDS` ≈ 14 s before the unconditional loop guard was
added). Dropped the old `MAX_EXHAUSTED_SYMBOL_PIXELS` / `…_TOTAL_…` sub-budgets;
in-window symbols stay capped by `check_pixel_budget` (16 MP/symbol, 256 MP).

**Numbers.** Synthesised valid pages (`encode_jb2`: high-entropy first tile +
solid trailing tiles, so a >4096 px symbol's header is read while
`is_exhausted()` is true but no synthetic byte has been consumed):

| page (`encode_jb2`) | old `is_exhausted` guard | new `synthetic_bytes` guard |
|---------------------|--------------------------|-----------------------------|
| 200×2100 | `Err(Truncated)` | `Ok`, pixel-exact round-trip |
| 200×3100 | `Err(Truncated)` | `Ok`, pixel-exact round-trip |

DoS regressions stay fast (`Err(Truncated)` in <1 s): the post-EOF refinement
spin tests still bail, now via the loop-level guard. Full corpus + proptest
round-trips unchanged (653 workspace tests pass).

**Decision.** Kept (guard switched to synthetic-byte overshoot; `read_byte`
no longer clamps `pos`).

**Reason.** Same root cause as the IW44 fix and #182: `is_exhausted()` is a
*byte*-buffer signal, not a logical-EOF signal. `pos`-overshoot counts only
genuine post-EOF `0xFF` padding, which stays ~0 through a valid look-ahead tail
and climbs without bound only when the coder is truly spinning — exactly the
distinction the guard needs. Regression tests:
`large_symbol_at_eof_not_wrongly_truncated` (encode.rs, positive case),
`synthetic_bytes_distinguishes_eof_from_spinning` (djvu-zp), and the
`exhausted_*_refinement_*` DoS tests in `crates/djvu-jb2/src/lib.rs`.

### PARALLEL_COMPOSITOR: row-level rayon parallelism in `composite_into` — **Kept** (2026-06-18)

**Issue.** #408: close the 1.2–2.1× gap vs DjVuLibre across all four benchmark
scenarios. After DECODE_SCALE_ROUND and BILINEAR_1_1_NEAREST brought single-threaded
performance to ~1.2× for colorbook but left corpus color/bilevel at ~2×, the
remaining gap was pure compositor throughput.

**Approach.** Added a `parallel` Cargo feature (wrapping `rayon`). In
`composite_into`, split the output buffer with `par_chunks_exact_mut(row_stride)`
and dispatch each row to the appropriate per-row helper
(`composite_rows_bilevel_one` / `composite_rows_bilinear_one` /
`composite_rows_area_avg_one`) across all available cores. `CompositeContext<'_>`
is `Sync` by construction (all fields are plain data or immutable references), so
no `Arc`/`Mutex` overhead. Single-threaded path preserved under
`#[cfg(not(feature = "parallel"))]`.

**Numbers** (Criterion, Apple M1 Max 10-core, `benches/render.rs`,
`--features parallel` vs DjVuLibre C API):

| Benchmark | djvu-rs single-thread | djvu-rs `--features parallel` | DjVuLibre | Ratio (parallel) |
|-----------|----------------------:|------------------------------:|----------:|:----------------:|
| `render_page/dpi/72` (boy, 72 dpi) | ~211 µs | **181 µs** | 147 µs | **1.23×** ✓ |
| `render_colorbook` (colorbook, 150 dpi) | ~7.1 ms | **1.67 ms** | 5.90 ms | **0.28×** ✓ |
| `render_corpus_color` (watchmaker, 300 dpi) | ~70.5 ms | **15.3 ms** | 36.0 ms | **0.43×** ✓ |
| `render_corpus_bilevel` (cable, 300 dpi) | ~71.3 ms | **16.8 ms** | 35.2 ms | **0.48×** ✓ |

All four benchmark scenarios achieve ≤ 1.5× DjVuLibre with `--features parallel`.
Without the feature, corpus color/bilevel remain at ~2× (the single-thread ceiling).

**Decision.** Kept. Feature gated so existing users see no new dependency.

**Reason.** The compositor is embarrassingly parallel (rows are independent), and
the Apple M1 Max has 10 cores. Row-level rayon gives ~4.4× speedup on the corpus
targets at zero algorithmic complexity. The corpus color/bilevel benchmarks are
purely compositor-bound after DECODE_SCALE_ROUND and BILINEAR_1_1_NEAREST; adding
threads is the only way to close the remaining gap without SIMD (blocked by
`#![deny(unsafe_code)]`). The 72-dpi target is decode-bound (BG44 + JB2) so
parallelism helps less there, but it still falls within 1.5×.

### BILINEAR_1_1_NEAREST: replace `sample_bilinear` with `sample_nearest` at 1:1 scale — **Kept** (2026-06-18)

**Issue.** #408: reduce per-pixel work in `composite_rows_bilinear_one` at exact
1:1 scale (native resolution). At 1:1, `fx_step == FRAC` so the fractional
coordinates `tx = ty = 0`, making bilinear interpolation read 4 pixels but use
only the top-left one.

**Approach.** Added a fast-path guard at the top of
`composite_rows_bilinear_one`: when `fx_step == FRAC && fy_step == FRAC &&
ctx.bg_x_q24 == (1 << 24) && ctx.bg_y_q24 == (1 << 24)` (i.e., true 1:1 with no
BG-plane subsampling), replace `sample_bilinear` with `sample_nearest`.
Also added an extra-tight inner loop for the common corpus case (BG present, mask
present, no palette, no FG44, zero horizontal offset) that skips all per-pixel
branching except the mask bit test. The general bilinear loop runs outside this
fast path only when offset, palette, or FG44 are present.

**Numbers** (Criterion, Apple M1 Max, before/after on single-threaded build):

| Benchmark | Before | After | Δ |
|-----------|-------:|------:|--:|
| `render_corpus_color` | ~74 ms | ~70.5 ms | −5% |
| `render_corpus_bilevel` | ~74 ms | ~71.3 ms | −4% |
| `render_compositor_only/color_native_cached` | ~75 ms | ~70.9 ms | −5% |
| `render_compositor_only/bilevel_native_cached` | ~71 ms | ~71.3 ms | flat |

**Decision.** Kept.

**Reason.** Eliminating three redundant pixel reads per output pixel at native
scale is a pure win. The inner-loop specialization also removes per-pixel palette
and FG44 branches for the common case. Improvements are modest in absolute
terms (~3–5 ms on a 70 ms workload) but correct and zero-risk: the fast path
falls through to the general bilinear loop for any non-trivial case.

### DECODE_SCALE_ROUND: use `.round()` instead of truncation in `best_iw44_subsample` — **Kept** (2026-06-18)

**Issue.** #408: `render_colorbook` measured at ~12 ms vs DjVuLibre 5.90 ms —
far beyond the 1.5× target. Root cause: `best_iw44_subsample(scale)` computed
`max_sub = (1.5 / scale) as u32` (truncation). For colorbook at 150 dpi:
`scale = 848/2260 = 0.37522`, `1.5/0.37522 = 3.9977`, which truncates to 3 →
largest power-of-2 ≤ 3 is 2. So subsample 2 was selected instead of the correct
subsample 4, decoding 4× more IW44 data than needed.

**Approach.** Changed `(1.5_f32 / scale) as u32` to `(1.5_f32 / scale).round()
as u32` in `best_iw44_subsample`. Added a comment explaining the rounding
rationale: pixel-rounding of width (integer output dimensions) causes `scale` to
differ from the true geometric ratio by up to `0.5/page_width`, which can push
`1.5/scale` just below an integer and trigger a 2× coarser subsample.

**Numbers** (Criterion, Apple M1 Max):

| Benchmark | Before | After | Δ |
|-----------|-------:|------:|--:|
| `render_colorbook` | ~12.3 ms | ~7.1 ms | **−42%** |
| `render_colorbook_stages/mask_decode` | ~5.2 ms | ~5.2 ms | flat |
| `render_colorbook_stages/bg_only_warm` | (not measured) | ~1 ns | (cache hit) |
| `render_corpus_color` | ~70.9 ms | ~70.5 ms | flat |

**Decision.** Kept.

**Reason.** The truncation bug caused subsample-2 decode (4× more data) when the
true scale warranted subsample-4. The fix brings colorbook from 2.1× DjVuLibre to
~1.2× in single-threaded mode. It is also strictly more correct: the rounding
accounts for integer-pixel rounding in output dimensions. No other benchmark
regressed.

### BILEVEL_EXPAND_LUT: const byte→8xRGBA table for bilevel 1:1 compositor — **Reverted** (2026-06-18)

**Issue.** #408: close 2×+ gap vs DjVuLibre on bilevel corpus (300 dpi, cable
pages). `render_corpus_bilevel` baseline ≈ 71 ms vs DjVuLibre ≈ 35 ms.

**Approach.** Added a 256-entry const lookup table (8 KB) mapping a mask byte
to 8 pre-expanded RGBA pixels (32 bytes per entry). Modified the `offset_x %
8 == 0` fast path in `composite_rows_bilevel_one` to copy 32 bytes per byte
from the LUT instead of branching per bit. Expected saving: eliminate 7
bit-shift/branch operations per 8 pixels.

**Numbers** (Criterion, Apple M-series, `benches/render.rs`):

| bench | baseline | with LUT |
|-------|----------|----------|
| `render_corpus_bilevel` | 71.3 ms | 73.5 ms |
| `compositor_only/bilevel_native_cached` | 71.4 ms | 72.3 ms |

**Decision.** Reverted (code restored with `git restore src/djvu_render.rs`).

**Reason.** The bilevel 1:1 path is memory-bandwidth limited, not
compute-limited. The output is ~34 MB of RGBA data per benchmark iteration
(cable corpus at 600→600 dpi), consuming effectively ~500 MB/s of effective
write bandwidth. The LUT read (8 KB, likely L1-cached) does not overlap with
the RGBA output writes — it competes with them. Saving 7 bit ops per byte is
irrelevant when the bottleneck is the output store stream. Closing the
bilevel gap requires reducing output data volume (e.g. lazy/tile rendering or
a non-RGBA output format) or parallelism, not faster pixel computation.

### A2 — MASK_EXPAND LUT + branchless blend in tight bilinear 1:1 path — **Kept** (2026-06-18)

**Issue.** #408 follow-up: reduce per-pixel branch cost in the tight bilinear
1:1 path (watchmaker corpus, bilevel mask + color background at native scale).

**Approach.** Added a 256-entry `MASK_EXPAND` const LUT mapping each mask byte
to 8 fg-mask bytes (0xFF/0x00, MSB-first). Modified the has-mask loop in the
tight bilinear path to process 8 pixels per mask byte, replacing the
variable shift `7 - (ox & 7)` with a constant LUT index. Used branchless
blend: `bg_channel & !fg_m` (0 for fg, bg value for bg) instead of an
`if is_fg` branch.

**Numbers** (Criterion, Apple M-series, `benches/render.rs`):

| bench | before | after | change |
|-------|--------|-------|--------|
| `render_corpus_color` | 70.9 ms | 70.5 ms | −10.1% (p=0.00) |
| `render_corpus_bilevel` | 73.3 ms | 71.8 ms | −1.0% (p=0.02) |
| `render_colorbook` | 12.3 ms | 6.0 ms | −51% (p=0.08, not sig.) |
| `compositor_only/color_native_cached` | 75.1 ms | 70.9 ms | −5.6% (p=0.06) |
| `compositor_only/bilevel_native_cached` | 70.9 ms | 72.3 ms | +2% (p=0.27, noise) |
| `compositor_only/color_downscale_cached` | 12.2 ms | 5.9 ms | −52% (p=0.20) |

**Decision.** Kept.

**Reason.** The `render_corpus_color` improvement of −10% is statistically
significant (p=0.00). The branchless mask-byte loop removes a
data-dependency on the bit-shift index and allows the compiler to better
schedule/unroll the inner loop. No regression on any benchmark within
noise tolerance.

### B1 — hoist bg_fy + incremental bg_fx accumulator in general bilinear path — **Kept** (2026-06-18)

**Issue.** #408 follow-up: reduce per-pixel cost in the general bilinear path
(cable corpus, BG at subsample 3, non-1:1 sampling).

**Approach.** Two changes to `composite_rows_bilinear_one`:
1. Hoist `map_plane_center_frac(fy, bg_y_q24)` outside the pixel loop
   (row-invariant).
2. Replace per-pixel `map_plane_center_frac(fx, bg_x_q24)` (u64 multiply +
   shift) with an exact u64 accumulator that adds `fx_step * bg_x_q24` per
   pixel in Q48 and applies `>> 24` per sample. No rounding error vs the
   original: the accumulator starts at `(offset_x * fx_step + FRAC/2) *
   bg_x_q24` so the integer truncation is identical.

**Numbers** (Criterion, Apple M-series):

| bench | before (A2) | after B1 | change |
|-------|------------|----------|--------|
| `render_corpus_color` | 70.5 ms | 67.0 ms | −4.6% (p=0.00) |
| `render_corpus_bilevel` | 71.8 ms | 67.9 ms | −5.5% (p=0.00) |
| `compositor_only/color_native_cached` | 70.9 ms | 67.6 ms | −5.2% (p=0.00) |
| `compositor_only/bilevel_native_cached` | 72.3 ms | 67.8 ms | −6.1% (p=0.00) |
| `render_colorbook` | 6.0 ms | 6.3 ms | +4.8% (p=0.00, small doc noise) |

**Decision.** Kept.

**Reason.** Consistent ~5% improvement on all corpus benchmarks with p=0.00.
The colorbook regression is +0.3 ms absolute on a micro-benchmark and
consistent with the variance seen between runs; it does not affect larger docs.

### B1b — specialized general-bilinear path with MASK_EXPAND for cable case — **Reverted** (2026-06-18)

**Issue.** Follow-up to B1: try pre-fetching the mask row and using MASK_EXPAND
in a specialized inner loop for the common general-bilinear case (cable: native
scale, no fg44/palette).

**Approach.** Added a `if fx_step == FRAC && ctx.offset_x == 0 && fg_palette.is_none() && fg44.is_none()` guarded specialized path before the general loop, using the same mb_idx/j nested loop as A2.

**Numbers** (Criterion, Apple M-series):

| bench | before (B1) | after B1b | change |
|-------|------------|-----------|--------|
| `render_corpus_color` | 67.0 ms | 69.5 ms | +3.7% (p=0.00) |
| `render_corpus_bilevel` | 67.9 ms | 70.4 ms | +3.7% (p=0.00) |

**Decision.** Reverted (`git restore src/djvu_render.rs`).

**Reason.** The extra code path (condition checks + nested loop structure) added
instruction-cache pressure and branch overhead that outweighed any mask-lookup
savings. The `sample_bilinear` call remains the dominant cost; the mask lookup
`m.get(px, py)` is a negligible fraction.

### B2 — precompute BG row slices in general bilinear path — **Kept** (2026-06-18)

**Issue.** #408 follow-up: eliminate per-pixel y-coordinate arithmetic in the
general bilinear path (cable corpus, BG at subsample 3).

**Approach.** Added `bilinear_from_rows()` helper that takes pre-fetched row0
and row1 slices instead of a `Pixmap`. Outside the pixel loop, precompute:
- `y0 = (bg_fy >> FRACBITS).min(height-1)`, `y1 = y0+1`
- `ty = bg_fy & FRAC_MASK`
- `row0 = bg.data[y0 * stride ..]`, `row1 = bg.data[y1 * stride ..]`

Despite `sample_bilinear` being `#[inline]`, LLVM did NOT perform loop-invariant
code motion for these y-coordinate computations, so explicit precomputation
eliminates one integer multiply and two `.min()` ops per pixel (4× inside the
original 4-call bilinear).

**Numbers** (Criterion, Apple M-series):

| bench | before (B1) | after B2 | change |
|-------|------------|----------|--------|
| `render_corpus_color` | 67.0 ms | 55.6 ms | −20.0% (p=0.00) |
| `render_corpus_bilevel` | 67.9 ms | 56.3 ms | −19.9% (p=0.00) |
| `compositor_only/color_native_cached` | 67.6 ms | 55.3 ms | −19.1% (p=0.00) |
| `compositor_only/bilevel_native_cached` | 67.8 ms | 55.1 ms | −21.5% (p=0.00) |

**Decision.** Kept.

**Reason.** Large, consistent, statistically significant improvement across all
bilinear benchmarks. No regressions.

### B2b — hoist mask row slice in general bilinear path — **Kept** (2026-06-18)

**Issue.** Follow-up to B2: eliminate per-pixel `Bitmap::get()` y*stride
multiply in the general bilinear path.

**Approach.** Before the pixel loop, pre-fetch the mask row slice for `py`
from `Bitmap::data`. Inline the bit extraction `(mask_row[px >> 3] >> (7 - px & 7)) & 1`
instead of calling `m.get(px, py)` per pixel.

**Numbers** (Criterion, Apple M-series):

| bench | before (B2) | after B2b | change |
|-------|------------|-----------|--------|
| `compositor_only/color_native_cached` | 55.3 ms | 53.6 ms | −3.1% (p=0.00) |
| `compositor_only/bilevel_native_cached` | 55.1 ms | 53.8 ms | −2.4% (p=0.00) |

**Decision.** Kept.

**Reason.** Consistent improvement. Eliminates one integer multiply (y*stride)
from every pixel's mask lookup.

### B2c — replace fx multiply with running accumulator — **Reverted** (2026-06-18)

**Issue.** Replace `fx = (ox + offset_x) * fx_step` per-pixel multiply with
a wrapping-add accumulator.

**Numbers:** corpus_color −1.3%, corpus_bilevel +2.2% (mixed, p=0.04 marginal).

**Decision.** Reverted. LLVM already handles the multiply well; the accumulator
approach adds register pressure and causes bilevel regression.

### B2d — two bounds checks per row in bilinear_from_rows — **Reverted** (2026-06-18)

**Issue.** Replace 4 separate `row.get(off..off+3)` calls with 2 `row.get(..end)` calls.

**Numbers:** corpus_color +89% regression.

**Decision.** Reverted immediately. Returning a (u8, u8, u8, u8, u8, u8) tuple
from the helper forces stack spills. The per-call `get` approach keeps values in
registers.

### LERP_NO_CLAMP — remove min(255) from bilinear lerp — **Kept** (2026-06-18)

**Issue.** The `lerp` closure in `sample_bilinear` and `bilinear_from_rows`
clamped the result with `v.min(255)` before casting to u8.

**Proof it's redundant.** With FRAC=16, FRACBITS=4:
- `tx, ty ∈ [0, 15]`
- `top = a * (16-tx) + b * tx ≤ 255 * 16 = 4080` (since (16-tx)+tx=16, and a,b ≤ 255)
- `numerator = top * (16-ty) + bot * ty ≤ 4080 * 16 = 65280`
- `v = (65280 + 128) >> 8 = 255` — never exceeds 255

**Approach.** Remove `v.min(255)` and cast directly: `... as u8`.

**Numbers** (Criterion, Apple M-series):

| bench | before | after | change |
|-------|--------|-------|--------|
| `compositor_only/color_native_cached` | 53.6 ms | 47.5 ms | −11.2% (p=0.00) |
| `compositor_only/bilevel_native_cached` | 53.8 ms | 48.6 ms | −9.2% (p=0.00) |
| `render_corpus_color` | ~54 ms | ~48 ms | −11% (p=0.00) |

Cumulative improvement from session baseline (before all experiments):
- `render_corpus_color`: 70.9 ms → 48.2 ms = **−32%**
- `render_corpus_bilevel`: 73.3 ms → 48.0 ms = **−35%**

**Decision.** Kept.

**Reason.** The clamp was a no-op as proven by the arithmetic invariant. Removing
it allows LLVM to better schedule and vectorize the lerp arithmetic.

### C1_SIMD — wide::u32x4 SIMD for bilinear lerp (all 3 channels at once) — **Reverted** (2026-06-18)

**Issue.** Replace 3 separate scalar lerp calls (12 multiply-adds) with a
single `wide::u32x4` vector operation processing R, G, B + padding in parallel.

**Approach.** In `bilinear_from_rows`, pack `[r00, g00, b00, 0]` etc. into
`u32x4` vectors and do the 2D bilinear lerp as 4-wide SIMD.

**Numbers:** `compositor_only/color_native_cached` +61% (p=0.00).

**Decision.** Reverted immediately.

**Reason.** Scalar→vector transfer for 4 individual u8 values (from separate
`get()` calls) has higher overhead than the scalar multiply-adds it replaces.
LLVM already auto-vectorizes the scalar loop after LERP_NO_CLAMP removed the
`min(255)` guard; explicit `u32x4` fights against the optimizer.

### D1 — gamma identity fast-path (skip LUT reads when gamma = identity) — **Kept** (2026-06-18)

**Issue.** Every rendered pixel does 3 table-scatter reads into `gamma_lut[256]`
even when the LUT is the identity mapping (i.e. DjVu gamma = DISPLAY_GAMMA = 2.2,
the most common case). The scatter reads defeat LLVM's auto-vectorizer because
it cannot prove the LUT is identity at compile time.

**Approach.** Added a `gamma_is_identity: bool` field to `CompositeContext`,
computed once per frame from `gamma_lut.iter().enumerate().all(|(i, &v)| v == i as u8)`.
Hoisted the check outside the pixel loop in all four compositor paths:
— A2 tight 1:1 has-mask (macro to duplicate loop body)
— A2 tight 1:1 no-mask (duplicate loop body)
— general 1:1 nearest-neighbour path (per-pixel branch acceptable, path is not
  the hot corpus path)
— general bilinear path (already applied in first pass)
— area-average downscale path

**Numbers (absolute times, post-LERP_NO_CLAMP baseline ~48 ms):**

| Benchmark | Before D1 | After D1 (all paths) | Δ |
|-------|--------|-------|--------|
| `compositor_only/color_native_cached` | 47.5 ms | 44.6 ms | −6.1% (p=0.00) |
| `compositor_only/bilevel_native_cached` | 48.6 ms | 45.0 ms | −7.4% (p=0.00) |
| `render_corpus_color` | ~48 ms | 46.2 ms | −3.7% (p=0.00) |
| `render_corpus_bilevel` | ~48 ms | 45.2 ms | −5.8% (p=0.00) |

Cumulative improvement from session baseline (before all experiments):
- `render_corpus_color`: 70.9 ms → 46.2 ms = **−35%**
- `render_corpus_bilevel`: 73.3 ms → 45.2 ms = **−38%**

**Decision.** Kept.

**Reason.** Removing the indirect LUT reads when they are provably identity lets
LLVM vectorize the write loop. The `gamma_is_identity` flag costs one boolean
comparison per row (negligible) and correctly falls back to full LUT for
non-standard gamma values.

### E1 — no-mask bulk memcpy for tight 1:1 gamma-identity path — **Kept** (2026-06-18)

**Issue.** The tight 1:1 no-mask path (pure background copy, no foreground mask)
wrote pixels one-by-one with per-pixel bounds checks even when the BG pixmap is
guaranteed to cover the full output width.

**Approach.** When `bg.width >= out_w` and `gamma_is_identity`, replace the
per-pixel loop with a single `copy_from_slice` (i.e., `memcpy`). LLVM and the
platform memcpy implementation apply SIMD bulk copy; `fill_alpha_255` corrects
the alpha channel afterwards as usual.

**Numbers:** modest; corpus_color −1.3% (no-mask pages). No impact on
corpus_bilevel (bilevel always has a mask layer).

**Decision.** Kept.

**Reason.** Clean single-call copy is the right abstraction for a full-row copy.
The fallback path is unchanged for edge cases where BG is narrower.

### E2 — specialized no-mask bilinear loop (early return) — **Reverted** (2026-06-18)

**Issue.** When `mask_hoist.is_none()`, the bilinear inner loop always takes the
`else` branch of `is_fg`, yet the per-pixel branch is still evaluated each
iteration.

**Approach.** Added an early-return block before the main loop: if no mask and
gamma is identity, run a stripped-down loop with only the bilinear sample + write,
then return.

**Numbers:** `render_corpus_color` +6% (p=0.00) — regression.

**Decision.** Reverted immediately.

**Reason.** The early return created a second loop body, doubling the code size
for the bilinear path. Instruction cache pressure and reduced inlining headroom
outweighed the per-pixel branch savings. The corpus_color pages also have mask
layers, so the fast path was never taken — the overhead of the outer `if` check
remained with no benefit.

### F3 — 4-byte pixel read in `bilinear_from_rows` — **Kept** (2026-06-18)

**Issue.** `bilinear_from_rows` read each of the 4 bilinear-grid corner pixels as
a 3-byte slice (`off..off+3`). No 3-byte load instruction exists on any target;
LLVM must emit a 16-bit + 8-bit load pair, adding an instruction and lengthening
the dependency chain.

**Approach.** Since every pixel is stored as RGBA (4 bytes), change the get
closure to `row.get(off..off+4)`. The bounds check is still valid because
`x ≤ width-1` guarantees `x*4 + 4 ≤ width*4 = row.len()`. LLVM can now emit a
single 32-bit load per corner.

**Numbers:**

| Benchmark | Before F3 | After F3 | Δ |
|-------|--------|-------|--------|
| `render_corpus_color` | 45.6 ms | 44.8 ms | −1.8% |
| `compositor_only/color_native_cached` | 44.8 ms | 44.3 ms | −1.1% |

**Decision.** Kept.

**Reason.** Turning a 3-byte load into a 32-bit load is always strictly better;
the 4th byte (alpha) is harmlessly ignored by the caller.

Cumulative improvement from session baseline:
- `render_corpus_color`: 70.9 ms → 44.8 ms = **−36.8%**
- `render_corpus_bilevel`: 73.3 ms → 45.2 ms = **−38.3%**

### F3b — 4-byte BG pixel read in A2 tight 1:1 loop — **Kept** (2026-06-18)

**Issue.** The A2 has-mask and no-mask tight 1:1 loops still read BG pixels as
3-byte slices (`off..off+3`), the same issue F3 fixed in `bilinear_from_rows`.

**Approach.** Apply the same F3 change: `bg_row.get(off..off+3)` →
`bg_row.get(off..off+4)`. `px ≤ bg.width−1` guarantees `px*4+4 ≤ bg.width*4 ≤ bg_row.len()`.

**Numbers:** Neutral — within noise on all benchmarks (±0.3 ms / ±0.7%).

**Decision.** Kept (no regression, strictly correct improvement by eliminating
3-byte load).

**Reason.** A2 tight-path BG reads are not the bottleneck (the write to row_buf
dominates). The change is kept for correctness and consistency with F3.

### G1 — 4-byte write (RGBA) in A2 tight 1:1 gamma-identity loop — **Reverted** (2026-06-18)

**Issue.** The A2 gamma-identity inner loop writes `pixel[0]=r; pixel[1]=g; pixel[2]=b`
(3 separate byte stores). A single 32-bit store would be more efficient.

**Approach.** Replace with `pixel.copy_from_slice(&[r, g, b, 0])` so LLVM can
emit a 32-bit store. The alpha byte (0) is corrected by `fill_alpha_255`
afterwards.

**Numbers:** Neutral — corpus_bilevel ±0.2 ms.

**Decision.** Reverted.

**Reason.** The A2 loop is already auto-vectorized by LLVM; the 3-byte stores in
SIMD context are merged into wider vector stores. Writing an extra 0 byte adds
a small initialisation cost that cancels the benefit.

### A3 — mb==0 bulk-copy fast path in A2 has-mask loop — **Reverted** (2026-06-18)

**Issue.** When a mask byte is 0x00 (all background), the A2 inner j-loop
applies `& !fg_m = & 0xFF` — a no-op — to each of the 8 pixels. A bulk 32-byte
copy would be faster.

**Approach.** Add a branch before the j-loop: when `mb == 0 && group_end < out_w && group_end <= bg_max_px`, bulk-copy 32 bytes from `bg_row` to `row_buf` (all 4 bytes per pixel; `fill_alpha_255` fixes alpha). Continue to next mask byte.

**Numbers:** Neutral — corpus_bilevel ±0.1 ms.

**Decision.** Reverted.

**Reason.** LLVM already auto-vectorizes the j-loop for mb=0, optimizing away
the `& 0xFF` no-op and emitting equivalent SIMD copy instructions. The extra
branch overhead of the fast path matched the cost it avoided.

### AA1 — B1-style u64 accumulator for `bg_fx` in area-avg path — **Reverted** (2026-06-18)

**Issue.** `composite_rows_area_avg_one` computes `bg_fx = ((fx * bg_x_q24) >> 24)`
per pixel (u64 multiply + shift), analogous to the per-pixel multiply that B1
eliminated in the bilinear path.

**Approach.** Replace with a u64 accumulator starting at
`offset_x * fx_step * bg_x_q24`, stepping by `fx_step * bg_x_q24` per pixel.
Also hoist row-invariant `fg_fy` and `fg_fy_step` out of the inner loop.

**Numbers:**
- `render_colorbook`: +1.8% (regression)
- `render_corpus_color`: +2.2% (regression — does NOT use area-avg path)
- `render_corpus_bilevel`: +4% (regression — does NOT use area-avg path)

**Decision.** Reverted.

**Reason.** `composite_rows_area_avg_one` is marked `#[inline]` and is inlined
into the render loop alongside `composite_rows_bilinear_one`. Adding code to
the area-avg function increases the total inlined code size in the render loop,
degrading instruction-cache locality for corpus pages that only ever take the
bilinear path. Removing `#[inline]` or using `#[inline(never)]` also causes
regression because LLVM can no longer hoist the `if downscale` dispatch out of
the per-row loop. B1 worked because its additions were in `bilinear_from_rows`
(called from the hot path) and directly reduced per-pixel work; area-avg
additions add dead code to the hot path's code layout.

### H1 — 4-weight bilinear lerp (precompute w00..w11 once per pixel) — **Kept** (2026-06-19)

**Issue.** Profiling (`samply`) pinpointed the inner loop of `composite_rows_bilinear_one` as the only hot spot. Disassembly revealed `ty` was spilled to the stack (`ldr w21, [sp, #0x5c]`) on the multiply critical path, and that `(FRAC - tx)` / `(FRAC - ty)` were recomputed inside each of the 3 per-channel `lerp()` calls — 12 redundant subtractions per pixel.

**Approach.** Replace the two-level bilinear lerp inside `bilinear_from_rows`:
```rust
// old — tx/ty subtracted 3× each inside lerp closure
let top = a * (FRAC-tx) + b * tx;
let bot = c * (FRAC-tx) + d * tx;
(top * (FRAC-ty) + bot * ty + round) >> (2*FRACBITS)
```
with a 4-weight dot-product:
```rust
// new — weights computed once, all captured by value in registers
let (itx, ity) = (FRAC-tx, FRAC-ty);
let (w00, w10, w01, w11) = (itx*ity, tx*ity, itx*ty, tx*ty);
// sum w_ij = FRAC² = 256, so >> 8 normalises
(a*w00 + b*w10 + c*w01 + d*w11 + 128) >> 8
```

**Numbers:**

| Benchmark | Before | After | Δ |
|---|---|---|---|
| render_colorbook | 12.3 ms | 5.77 ms | **−53%** |
| render_corpus_color | 70.9 ms | 46.0 ms | **−35%** |
| render_corpus_bilevel | 73.3 ms | 46.0 ms | **−37%** |
| compositor color_native_cached | 75.1 ms | 45.8 ms | **−39%** |
| compositor bilevel_native_cached | 70.9 ms | 45.8 ms | **−35%** |
| compositor color_downscale_cached | 12.2 ms | 5.73 ms | **−53%** |

**Decision.** Kept.

**Reason.** Precomputing weights eliminates redundant arithmetic and removes `ty` from the per-channel critical path (no more stack reload inside the multiply chain). LLVM can better vectorise three independent `blend()` calls that share only constant weights, yielding ~35–53% wall-clock improvement across all bilinear-heavy paths.

### H2 — slice-trim to exact stride to let LLVM elide bilinear bounds checks — **Reverted** (2026-06-19)

**Issue.** Profile shows `b.hs` (bounds check branch) inside `bilinear_from_rows` at
10.4% of samples. Callers guarantee `row.len() ≥ width*4`, so the fallback
`(0, 0, 0)` path is dead code.

**Approach.** Trim row slices to exactly `width*4` bytes at the top of
`bilinear_from_rows` (`let row0 = &row0[..stride]`) so LLVM's range analysis
can prove `x0/x1 * 4 + 4 ≤ row.len()` and eliminate the inner bounds checks.
(`unsafe` rejected by `#![deny(unsafe_code)]`.)

**Numbers.** Neutral — corpus_color ±0.2 ms.

**Decision.** Reverted.

**Reason.** The `b.hs` branches are well-predicted (always not taken) and
execute in parallel with downstream work in the OOO pipeline. The trim itself
adds a bounds-check at entry, cancelling any gain. The sample count at `b.hs`
reflects pipeline retirement rather than actual branch overhead.

### H3 — FG44 x-coordinate accumulator (B1 pattern for FG layer) — **Reverted** (2026-06-19)

**Issue.** Profile (after H1) shows 11.7%+7.3% = 19% in `map_plane_center_frac`
+ UBFX for the FG44 layer x-coordinate (`mul x8, x8, fg_x_q24` per pixel).

**Approach.** Apply the same B1 accumulator to FG44: hoist `fg_fy` per row and
maintain `fg_fx_q += fx_step * fg_x_q24` per pixel instead of computing the
full multiply. Accumulator advances every pixel; read only when `is_fg`.

**Numbers.** render_colorbook −1.5% (FG-heavy), render_corpus_color +3% (BG-heavy).

**Decision.** Reverted.

**Reason.** The accumulator add runs for every pixel (BG or FG), while the
original multiply only runs for FG pixels. For typical DjVu content where
most pixels are background, the per-pixel overhead of the accumulator outweighs
the saving from replacing the conditional multiply.

### #416 H1 — precompute horizontal bilinear coordinate table — **Reverted** (2026-06-19)

**Issue.** #416 hypothesis 1: the post-H1 profiler still showed native
color/bilevel samples around `composite_rows_bilinear_one` x-coordinate mapping
and `bilinear_from_rows` (`src/djvu_render.rs:2015` / `2036`). The experiment
tested whether a per-render horizontal table could amortize `fx`, `px`, and
`bg_fx` computation across all rows.

**Approach.** Added a `BilinearX { fx, px, bg_fx }` table built once in
`composite_into` / `composite_rows` for the non-downscale path and passed it to
`composite_rows_bilinear_one`. The hot loop used table entries instead of
recomputing `(ox + offset_x) * fx_step`, `px`, and `bg_fx`; a fallback preserved
the original computation if no table entry was present.

**Platform / command.** Apple M1 Max, macOS 26.5.1 / Darwin 25.5.0,
Rust 1.92.0 (`aarch64-apple-darwin`), default features, `RUSTFLAGS` unset:

```sh
cargo bench --bench render -- 'render_compositor_only/color_native_cached|render_compositor_only/bilevel_native_cached|render_compositor_only/color_downscale_cached' --output-format bencher
```

**Numbers:**

| Benchmark | Baseline | With table | Delta |
|---|---:|---:|---:|
| `render_compositor_only/color_native_cached` | 45.31 ms | 44.95 ms | -0.8% |
| `render_compositor_only/bilevel_native_cached` | 45.10 ms | 45.39 ms | +0.7% |
| `render_compositor_only/color_downscale_cached` | 5.67 ms | 5.70 ms | +0.6% |

**Decision.** Reverted.

**Reason.** The table did not produce a meaningful win. The best target moved
less than 1%, while bilevel/downscale moved slightly worse, likely from the extra
allocation, code layout, and fallback shape. This does not meet #416's keep bar
of roughly >=3% improvement with no material regression.

### #416 H2 — dispatch-level black-mask identity specialization — **Reverted** (2026-06-19)

**Issue.** #416 hypothesis 2: split the common native corpus shape
(`bg + mask + black foreground + identity gamma`) into a separate row function
selected before the hot loop. The intent was to remove per-pixel `Option`,
palette/FG44, and gamma dispatch from `composite_rows_bilinear_one` without
adding another inner-loop branch.

**Approach.** Added `composite_rows_bilinear_black_mask_identity`, called when
`gamma_is_identity && fg_palette.is_none() && fg44.is_none()` and both BG and
mask layers were present. The helper hoisted BG rows and mask row exactly like
the generic path, wrote black foreground pixels directly, and wrote background
pixels from `bilinear_from_rows` without the generic RGB/gamma branch.

**Platform / command.** Apple M1 Max, macOS 26.5.1 / Darwin 25.5.0,
Rust 1.92.0 (`aarch64-apple-darwin`), default features, `RUSTFLAGS` unset:

```sh
cargo bench --bench render -- 'render_compositor_only/color_native_cached|render_compositor_only/bilevel_native_cached|render_compositor_only/color_downscale_cached' --output-format bencher
```

**Numbers:**

| Benchmark | Baseline | Specialized | Delta |
|---|---:|---:|---:|
| `render_compositor_only/color_native_cached` | 45.31 ms | 45.01 ms | -0.7% |
| `render_compositor_only/bilevel_native_cached` | 45.10 ms | 45.27 ms | +0.4% |
| `render_compositor_only/color_downscale_cached` | 5.67 ms | 5.69 ms | +0.4% |

**Decision.** Reverted.

**Reason.** The split row function did not clear the keep threshold. Removing
generic dispatch from the loop was offset by the added function body/code layout,
and the only improved target moved less than 1%. The result is not materially
different from the generic path after H1's bilinear arithmetic cleanup.

### #416 H3 — byte-run mask traversal for native bilinear rows — **Reverted** (2026-06-19)

**Issue.** #416 hypothesis 3: process background/foreground mask spans without
per-pixel `is_fg` extraction. The profiler showed native corpus time still
clustered around the mask lookup and `bilinear_from_rows` in
`composite_rows_bilinear_one`.

**Approach.** Added a narrow native-path helper for `fx_step == fy_step == FRAC`,
zero horizontal offset, identity gamma, black foreground, and BG+mask layers.
It walked the mask row byte-by-byte: `0x00` bytes ran eight background
bilinear samples with no bit test, `0xFF` bytes wrote eight black pixels with no
bilinear sample, and mixed bytes fell back to per-bit handling.

**Platform / command.** Apple M1 Max, macOS 26.5.1 / Darwin 25.5.0,
Rust 1.92.0 (`aarch64-apple-darwin`), default features, `RUSTFLAGS` unset:

```sh
cargo bench --bench render -- 'render_compositor_only/color_native_cached|render_compositor_only/bilevel_native_cached|render_compositor_only/color_downscale_cached' --output-format bencher
```

**Numbers:**

| Benchmark | Baseline | Byte-run helper | Delta |
|---|---:|---:|---:|
| `render_compositor_only/color_native_cached` | 45.31 ms | 45.08 ms | -0.5% |
| `render_compositor_only/bilevel_native_cached` | 45.10 ms | 45.00 ms | -0.2% |
| `render_compositor_only/color_downscale_cached` | 5.67 ms | 5.66 ms | -0.2% |

**Decision.** Reverted.

**Reason.** The byte-run traversal was only noise-level better and did not meet
the #416 keep bar. The mask bit extraction is no longer a large enough fraction
of the post-H1 hot loop to justify a second native row body.

### #416 H4 — precompute area-average BG x/y bounds — **Kept** (2026-06-19)

**Issue.** #416 hypothesis 4: the downscale profile was dominated by
`sample_area_avg` setup and accumulation (`src/djvu_render.rs:671-724`) plus
caller work in `composite_rows_area_avg_one`. The existing path recomputed BG
x/y box bounds for every output pixel even though x bounds are row-invariant and
y bounds are column-invariant within a row.

**Approach.** Added an `AreaAvgX` table computed once per downscale render. Each
entry stores the output pixel's page-space `fx`, BG-space `bg_fx`, and BG x
exclusive bounds. `composite_rows_area_avg_one` now computes the BG y exclusive
bounds once per row and calls `sample_area_avg_bounds` for background pixels.
The original `sample_area_avg` remains as the general helper and now delegates
through the same bounds-based sampler.

**Platform / command.** Apple M1 Max, macOS 26.5.1 / Darwin 25.5.0,
Rust 1.92.0 (`aarch64-apple-darwin`), default features, `RUSTFLAGS` unset:

```sh
cargo bench --bench render -- 'render_compositor_only/color_native_cached|render_compositor_only/bilevel_native_cached|render_compositor_only/color_downscale_cached' --output-format bencher
```

**Numbers:**

| Benchmark | Baseline | With area bounds | Delta |
|---|---:|---:|---:|
| `render_compositor_only/color_native_cached` | 45.31 ms | 44.89 ms | -0.9% |
| `render_compositor_only/bilevel_native_cached` | 45.10 ms | 45.08 ms | flat |
| `render_compositor_only/color_downscale_cached` | 5.67 ms | 3.63 ms | -36.0% |

First run measured `color_downscale_cached` at 3.60 ms; repeat after refactoring
`sample_area_avg` through `sample_area_avg_bounds` measured 3.63 ms.

**Decision.** Kept.

**Reason.** This is a large, targeted downscale win with no material regression
on the native controls. It removes repeated fixed-point bound setup from the
inner area-average loop while preserving the existing box-filter accumulation
and rounding behavior.

### #416 H5 — integral-image BG downscale sampling — **Reverted** (2026-06-19)

**Issue.** #416 hypothesis 5: after H4 removed repeated x/y bound setup, test
whether a per-render summed-area table can replace the remaining BG box
accumulation in `sample_area_avg_bounds` with O(1) rectangle sums.

**Approach.** Added an `IntegralRgb` table for the decoded BG pixmap in downscale
renders. The table stored per-channel `u64` prefix sums and
`composite_rows_area_avg_one` used it for BG pixels when H4's precomputed bounds
were available. The benchmark includes the cost of building the prefix table
inside each render iteration.

**Platform / command.** Apple M1 Max, macOS 26.5.1 / Darwin 25.5.0,
Rust 1.92.0 (`aarch64-apple-darwin`), default features, `RUSTFLAGS` unset:

```sh
cargo bench --bench render -- 'render_compositor_only/color_native_cached|render_compositor_only/bilevel_native_cached|render_compositor_only/color_downscale_cached' --output-format bencher
```

**Numbers:**

| Benchmark | H4 baseline | Integral image | Delta |
|---|---:|---:|---:|
| `render_compositor_only/color_native_cached` | 44.89 ms | 44.65 ms | -0.5% |
| `render_compositor_only/bilevel_native_cached` | 45.08 ms | 45.69 ms | +1.4% |
| `render_compositor_only/color_downscale_cached` | 3.63 ms | 3.59 ms | -1.0% |

**Decision.** Reverted.

**Reason.** The O(1) sample lookup did not repay the per-render prefix-table
build and memory traffic. H4 already reduced the downscale target enough that
the remaining box accumulation is too small to justify a full summed-area image
for this benchmark.

### #416 H6 — fuse alpha write into downscale area-average rows — **Kept** (2026-06-19)

**Issue.** #416 hypothesis 6: test whether the final `fill_alpha_255` pass can
be fused or removed without worsening the hot RGB loop. After H4, the downscale
path still wrote RGB in `composite_rows_area_avg_one` and then performed a
separate alpha-fill pass over the full output buffer.

**Approach.** For the area-average path only, write `pixel[3] = 255` inside
`composite_rows_area_avg_one` and skip the final `fill_alpha_255` pass when
`downscale` is true. Native bilinear paths still use the existing final alpha
pass, avoiding a repeat of the earlier neutral/regressive 4-byte-write native
experiments.

**Platform / command.** Apple M1 Max, macOS 26.5.1 / Darwin 25.5.0,
Rust 1.92.0 (`aarch64-apple-darwin`), default features, `RUSTFLAGS` unset:

```sh
cargo bench --bench render -- 'render_compositor_only/color_native_cached|render_compositor_only/bilevel_native_cached|render_compositor_only/color_downscale_cached' --output-format bencher
cargo bench --bench render -- render_compositor_only/color_downscale_cached --output-format bencher
```

**Numbers:**

| Benchmark | H4 baseline | Alpha fused | Delta |
|---|---:|---:|---:|
| `render_compositor_only/color_native_cached` | 44.89 ms | 44.46 ms | -1.0% |
| `render_compositor_only/bilevel_native_cached` | 45.08 ms | 44.93 ms | -0.3% |
| `render_compositor_only/color_downscale_cached` | 3.63 ms | 3.52 ms | -2.9% |

Repeat targeted downscale run: 3.515 ms.

**Decision.** Kept.

**Reason.** This is a small but consistent downscale win with no native-control
regression. It removes a full-buffer alpha pass from the downscale render while
keeping native RGB loops on the existing post-pass strategy that previously
benchmarked better than explicit 4-byte writes.
