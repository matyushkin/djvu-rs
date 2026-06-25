# Experiments Index

Navigation map for `PERF_EXPERIMENTS.md`. Read this first; open the full entry only when you need numbers or code. Updated: 2026-06-24.

## Legend

Status: **K** = Kept · **R** = Reverted · **X** = Rejected · **D** = Diagnostic/infra · **F** = Needs follow-up

---

## Experiment Table

| ID | Date | Component | Status | Effect | Notes / Related |
|----|------|-----------|--------|--------|-----------------|
| JB2_PAGE_SYM_CAP | 2026-06-25 | JB2 decode (robustness) | **K** | fuzz_jb2 timeout fixed; per-page decode bounded 256→32 MP | 409 B input did 23 K refinement records = 47.7 MP (~626 ms→10 s ASAN). Split cap: 32 MP page / 256 MP dict. Regression test + seed corpus added |
| IW44_SWARM_REST | 2026-06-25 | IW44 encoder (size) | R/X | dead-end / interop-break | #8 ycbcr-round (PSNR worse), #6/7 early-term (chunks not null), #3/4/5/10/12 (change normative tables/ctx → break DjVuLibre interop), #2 (quality knob) |
| IW44_ACT_THRESH (IW44-1) | 2026-06-25 | IW44 encoder (size) | **K** | **−9.1% BG44, PSNR-identical** | Activation prediction s/2→11s/16 matches real gate; removes wasted NEW bits. Encoder-only, interop-safe. Gap 14.3%→3.9% |
| ENC_SIZE_DIAG | 2026-06-25 | encoder (size) | D | mask=parity, IW44 BG44=1.143× | Chunk breakdown: BG44/FG44 dominate colour/photo docs (94–99.9%); our IW44 is ~14% larger → next size lever |
| LAYERED_SHARED_DJBZ (#452) | 2026-06-24 | JB2 encoder (size) | **K** | **−34% (spec) / −5.5% (watchmaker)** | Shared Djbz for layered multi-page encode; closes the real DjVuLibre size gap (per-page dict dup). Round-trip verified |
| CHROMA_BILINEAR (#422) | 2026-06-24 | IW44 YCbCr | **K** | quality (chroma_half) | Bilinear chroma upsampling via per-row pre-upsample + reuse full-res SIMD kernel; only IW44 v2 pages, common path byte-identical |
| REFROW_REG (#445) | 2026-06-24 | JB2 decode | R | no benefit | Rolling m_r2 register; col_shift unbounded so guard can't drop; safe version just relocates read |
| IDWT_S2_NEON (#442) | 2026-06-24 | IW44 IDWT NEON | X | incorrect premise | Reuse s1_row for s==2 fails correctness: s==2 ⟹ sd==1 (stride-2 cols), s1_row assumes stride-1 |
| IDWT_SPLAT (#441) | 2026-06-24 | IW44 IDWT NEON | R | unmeasurable | Hoist vdupq_n_s32 splat; movi is free in ALU-bound loop, no benefit. C16/C8 pattern doesn't transfer |
| F2_GAMMA (#443) | 2026-06-24 | color 1:1 bilinear | **K** | narrow (non-identity gamma) | F2 all-bg fast path for non-identity gamma (LUT pass); free-on-miss, byte-identical. Identity path unchanged |
| PDF_STREAM (#449) | 2026-06-24 | PDF export | **K** | peak RSS O(pages)→O(1) bodies | Stream render→emit→drop in sequential path; byte-identical, strictly ≤ peak. Unmeasurable on corpus (no many-large-body doc) |
| LANCZOS_HOIST (#448) | 2026-06-24 | Lanczos resample | **K** | **−22.5% Lanczos** | Hoist sin weights + row-major accumulate in vertical pass; bit-identical |
| ROTATE_TILE (#447) | 2026-06-24 | rotate_pixmap | **K** | ~2–6% rotated (transpose much faster) | 32×32 tiled transpose for Cw90/Ccw90; cache-local vs strided. Byte-identical |
| CLUSTER_DEDUP (#446) | 2026-06-24 | JB2 encoder | **K** | O(P²)→O(P) | pages_seen.contains→last() under monotonic page order; strictly ≤ cost, helps large multi-page encodes |
| FGPAL_CACHE (#444) | 2026-06-24 | decode pipeline | R | +1% regression | Cache parsed FGbz palette; navm_fgbz chunk is 21 bytes (trivial parse) → cache overhead > saving |
| PAR_DEC (#440) | 2026-06-24 | decode pipeline | **K** | **−22% cold render** | rayon::join BG/FG layer decode (parallel feature); FG overlaps BG ZP phase. Warm free, cold first-open faster |
| COLOR_AA (#439) | 2026-06-24 | color area-avg | **K** | quality (68 vs 28 colors) | Proportional fg/bg blend via mask_box_coverage; AA color downscale, removes blocky halos. mask_box_any deleted |
| AREAAVG_ALLBG (#438) | 2026-06-24 | color area-avg | **K** | ~skip mask_box_any (50–71% fire) | All-bg band short-circuit skips footprint scan; byte-identical, #428 analog for color downscale |
| EXACT_SLICE (#437) | 2026-06-24 | color 1:1 bilinear | R | unmeasurable | Exact-length row slices for bounds-check elision; unwrap_or(&[]) defeats fg length-tracking; machine too hot to verify |
| BSERIES_MASKEXP (#436) | 2026-06-24 | B-series upscale | R | unmeasurable + overhead | MASK_EXPAND in B-series (G1b retry on upscale); expansion overhead not worth it on resampling-bound path |
| BSERIES_ALLBG (#435) | 2026-06-24 | B-series bilinear | **K** | ~3–8% (52.6% fire) | All-bg row short-circuit skips is_fg; byte-identical, F2 analog. Benchmark noisy, firing-rate evidence |
| FG_FX_ACCUM (#434) | 2026-06-24 | B-series bilinear | R | +13–27% regression | Q48 accumulator for fg_fx; advance is unconditional (100%) but fg multiply was conditional (~20%). Inverse of B1 |
| P2_REGION (#433) | 2026-06-24 | bilevel 1:1 | **K** | compositor faster, e2e within noise | Generalize P2 to byte-aligned offset_x (render_region); safe ext, zero regression on offset_x==0 |
| MASK_ANY_BYTE (#432) | 2026-06-24 | color area-avg | R | +5.1% regression | Byte-scan mask_box_any; early-exit + small footprint → setup overhead > saving. POPCNT doesn't transfer |
| TIFF_LUT (#431) | 2026-06-24 | TIFF export | **K** | **~18% bilevel export** | LUT byte-expansion in extract_bilevel_pixels (Gray8 analog of P2 table) |
| TIFF_DEFLATE (#430) | 2026-06-24 | TIFF export | **K** | **~149× size** | Deflate-compressed bilevel TIFF (1-bit not supported by tiff 0.9; Deflate beats it) |
| CROP_BYTECOPY (#429) | 2026-06-24 | JB2 direct encoder | **K** | **~14–15% multitile** | Byte-aligned crop_bitmap row-copy; direct encode_jb2 path only (non-shipping) |
| I3_DOWNSCALE (#428) | 2026-06-24 | bilevel downscale | **K** | **~6.5% (94.4% fire)** | Blank-band fill(255) in AA downscale path; benchmark thermal-noisy, evidence from fire rate + cost model |
| MASK_IDX_CACHE (#427) | 2026-06-24 | decode pipeline | **K** | **~5% palette** | Cache indexed JB2 mask+blit-map; avoids 33.6 MB re-alloc + JB2 re-decode per render |
| BG_CACHE_S2 (#426) | 2026-06-24 | decode pipeline | **K** | **−7–9% downscale** | Cache decoded BG44 RGB at sub=2; mirror of BG_CACHE for 150 DPI |
| C2c | 2026-06-24 | area-avg compositor | R | 0% | LLVM LICM already hoists fg_fy. Related: C2b |
| C2b | 2026-06-24 | B-series bilinear | R | 0% | LLVM LICM already hoists fg_fy. See dead-end §1 |
| P2 | 2026-06-24 | bilevel 1:1 | **K** | **+18–24% bilevel** | BILEVEL_RGBA 256×32B table; NEON `ldp q0,q1/stp q0,q1`. Replaces E1 |
| Q1 | 2026-06-24 | bilevel 1:1 | R | 0% | Clamp removal; LLVM still scalar. Superseded by P2 |
| G1b | 2026-06-24 | B-series bilinear | R | −10% regression | MASK_EXPAND in downscale path; overhead > saving at 72 DPI |
| I3 | 2026-06-24 | bilevel 1:1 | **K** | **+2.3% bilevel** | All-zero mask row → `row_buf.fill(255)`. Fires before P2 |
| G2 | 2026-06-24 | color 1:1 bilinear | R | 0% | bg-fill-then-fg-fixup; LLVM already pipelines loads. See dead-end §2 |
| G1 | 2026-06-24 | color 1:1 bilinear | **K** | **+25% color** | MASK_EXPAND to 4096B stack buf; 1 byte-load replaces 7-op bit-extract |
| F2 | 2026-06-24 | color 1:1 bilinear | **K** | **+3.4% color** | All-bg mask row → bulk `copy_from_slice`. Mirrors I3 |
| E1 | 2026-06-24 | bilevel 1:1 | R | 0% | MASK_EXPAND+fill in bilevel path; LLVM auto-vec already optimal |
| C3 | 2026-06-24 | color 1:1 bilinear | **K** | ~0% (cleanup) | Mask-row hoist; LLVM did LICM; kept for code clarity |
| POPCNT | 2026-06-23 | bilevel downscale | **K** | **−24% @72DPI** | Byte-level popcount in `mask_box_coverage`; replaces per-pixel bits |
| AA | 2026-06-23 | bilevel downscale | **K** | quality ↑ | Anti-aliased bilevel via `mask_box_coverage`; was binary `mask_box_any` |
| C2 | 2026-06-23 | color 1:1 bilinear | **K** | −1.4% color | FG44 y-row + bg-row hoist in general 1:1 path |
| BILINEAR | 2026-06-22 | color 1:1 bilinear | **K** | ~0% (+0.4% noise) | quality: `sample_nearest` → `sample_bilinear` for FG44 |
| ZEROED | 2026-06-22 | output allocation | X | −1% seq / **+62% par** | `Pixmap::zeroed` triggers VM page-fault storm in parallel path |
| COW_FG | 2026-06-21 | decode pipeline | **K** | −1.7% seq / −6.1% par | Cow for FG44+Mask; avoids 4.75 MB clone each warm render |
| FILL_OVR | 2026-06-21 | color 1:1 bilinear | X | +3.3% regression | Fill+overlay 2-pass; doubles output bandwidth vs 1-pass |
| COW_BG | 2026-06-21 | decode pipeline | **K** | −1.2% seq / −7.2% par | Cow for BG44; avoids 33.6 MB clone each warm render |
| ALPHA_INL | 2026-06-21 | color 1:1 bilinear | **K** | −2.3% seq / **−12.6% par** | Inline alpha=255 per pixel; removes 33.6 MB sequential post-pass |
| BG_CACHE | 2026-06-21 | decode pipeline | **K** | −5.1% color | Cache decoded BG44 RGB Pixmap; avoids 2.8 ms IDWT+YCbCr per call |
| BILEVEL_NEON | 2026-06-21 | bilevel 1:1 | X | +1.8% regression | Manual NEON `vst4_u8`; LLVM auto-vec already optimal. See dead-end §3 |
| COLOR_NEON | 2026-06-21 | color 1:1 bilinear | X | +0.9–1.3% regression | Manual NEON for A2 path; inner loop not bottleneck |
| SIMD_AREA | 2026-06-21 | area-avg downscale | X | +19.9% regression | NEON/SSSE3 accumulate; 2×2 footprint too small for SIMD threshold |
| PARALLEL | 2026-06-18 | all paths | **K** | **3.8× parallel** | `par_chunks_exact_mut` in `composite_into`; opt-in via `parallel` feature |
| IW44_PAR | 2026-06-21 | IW44 decode | **K** | **2.2× IDWT** | Parallel Y/Cb/Cr IDWT via `rayon::join`; opt-in |
| AREA_FIX | 2026-06-18 | area-avg downscale | **K** | **−52% colorbook** | Fix off-by-one exclusive bound; was 3×3 at 2×, now correct 2×2 |
| JB2_CROSS6 | 2026-06-15 | JB2 encoder | R | +4.37% size | Cross-size record-6; geometric misalign makes context mispredict |
| IW44_DIAG | 2026-06-15 | IW44 encoder | D | — | Loss localized to fine-band quantization; transform is bit-exact |
| RENDER_DIRECT | 2026-05-16 | render pipeline | **K** | **−18–20%** | `render_pixmap` direct → `render_into`; removes row-copy adapter |
| TIFF_STREAM | 2026-05-16 | TIFF export | **K** | −116 MB peak RSS | `render_streaming` feeds TIFF rows; no full RGBA pixmap |
| ADAPTIVE_SEG | 2026-05-16 | encoder segmentation | **K** | quality ↑ | Sauvola binarization + BG inpainting; opt-in via `SegmentOptions` |
| DJVM_MULTI | 2026-05-16 | encoder | **K** | — | Layered multi-page DJVM via CLI `--quality` |
| JB2_QUALITY | 2026-05-17 | JB2 encoder | F | dict=1.356× orig | Round-trip ok; byte cost remains too high vs original |
| WASM_SIMD | 2026-05-17 | wasm | D | −7% render | Harness: scalar vs simd128; simd128 confirmed win from existing IW44 code |
| ROW_SCRATCH | 2026-05-17 | render pipeline | X | inconsistent | Noisy A/B; rejected as production heuristic |
| COMP_BASELINES | 2026-05-17 | infrastructure | D | — | Added `render_compositor_only` bench group |

---

## Render Path Map

```
render_pixmap (RENDER_DIRECT: kept)
└── composite_into / composite_rows
    ├── [parallel] composite_rows via par_chunks_exact_mut   (PARALLEL: kept)
    ├── composite_rows_bilevel_one                            ← bilevel 1:1 path
    │   ├── I3: all-zero row → fill(255)                     KEPT
    │   ├── P2: BILEVEL_RGBA table, offset_x==0             KEPT  ← hot path
    │   └── scalar fallback (bit-extract per pixel)
    ├── composite_rows_bilinear_one
    │   ├── [1:1] general fast path                          ← color 1:1 path
    │   │   ├── F2: all-bg mask row → copy_from_slice        KEPT
    │   │   ├── G1: MASK_EXPAND pre-expand to stack buf      KEPT  ← hot path
    │   │   └── per-pixel bilinear blend
    │   └── [B-series] non-1:1 path                         ← color downscale
    │       ├── sample_bilinear(fg, fg_fx, fg_fy)            (C2b: LLVM already LICM)
    │       └── mask_box_coverage / POPCNT                   KEPT
    └── composite_rows_area_avg_one                          ← area-avg downscale
        ├── AREA_FIX: correct 2×2 exclusive bound            KEPT
        └── sample_area_avg_bounds                           (SIMD: rejected)

Decode pipeline:
  BG44 → RGB cache (BG_CACHE) → Cow borrow (COW_BG) → compositor
  FG44 → Cow borrow (COW_FG) → compositor
  JB2  → Cow borrow (COW_FG) → compositor
  IW44 IDWT: parallel Y/Cb/Cr (IW44_PAR)
  Output alpha: inlined per-pixel (ALPHA_INL)
```

---

## Current Baselines (cool state, M1 Max, aarch64, Rust 1.88)

| Benchmark | Time | Path exercised |
|-----------|------|----------------|
| `bilevel_native_cached` | ~37 ms | bilevel 1:1 (P2 + I3 active) |
| `color_native_cached` | ~47 ms | color 1:1 (G1 + F2 active) |
| `color_downscale_mixed_cached` | ~15–20 ms | B-series bilinear (watchmaker @150/300) |
| `render_corpus_bilevel_dpi/72` | ~7 ms | bilevel downscale (POPCNT) |
| `render_corpus_bilevel_dpi/150` | ~23 ms | bilevel downscale (POPCNT) |
| `render_colorbook` | ~3.5 ms | area-avg downscale (AREA_FIX) |

Thermal methodology: use intra-session ratio `target / control` to cancel M1 Max throttling.
Control for bilevel experiments: `color_native_cached`. Control for color: `bilevel_native_cached`.

---

## Known Dead Ends (do not retry without new evidence)

1. **LLVM LICM** covers all y-row hoists in B-series / area-avg paths (C2b, C2c). Confirmed by intra-session ratio showing zero consistent trend. Any explicit hoisting adds Option wrapping that may inhibit other opts.
2. **LLVM out-of-order pipelining** covers independent bg_row + g1_mask loads in the G1 inner loop (G2). A second pass over g1_mask to find fg pixels adds sequential dependency that prevents the same pipelining.
3. **LLVM auto-vectorization** of the bilevel scalar loop is already optimal on aarch64-apple-darwin (BILEVEL_NEON, E1, Q1). Manual NEON / `#[target_feature]` on stable Rust cannot use `#[inline(always)]` (issue #145574).
4. **SIMD for small footprints** (SIMD_AREA): at 2×2 downscale, each `sample_area_avg_bounds` row has 2 pixels = 8 bytes, below any SIMD threshold. Falls through to scalar tail every call.
5. **`Pixmap::zeroed`** (lazy VM pages): causes page-fault storm in parallel compositor on macOS; 21–62% regression in parallel path.

---

## Open Hypotheses (tracked as GitHub issues)

Generated 2026-06-24 by the `perf-experiment-swarm` workflow (14 discovery agents →
adversarial validation → synthesis). Each survived a skeptical validator that checked
against all 37 prior experiments and the 5 dead-ends. Pick one, run the experiment,
record the result in `PERF_EXPERIMENTS.md`, close the issue.

**P1 (highest leverage):**
- ~~#426 — cache decoded BG44 RGB Pixmap at subsample=2~~ **DONE (Kept, −7–9%)** → BG_CACHE_S2
- ~~#427 — cache decoded JB2 indexed mask in PageLayers~~ **DONE (Kept, ~5%)** → MASK_IDX_CACHE
- ~~#428 — all-white band fast path in bilevel downscale compositor~~ **DONE (Kept, ~6.5%)** → I3_DOWNSCALE
- ~~#429 — crop_bitmap aligned byte-copy fast path in JB2 encoder~~ **DONE (Kept, ~14–15% multitile)** → CROP_BYTECOPY
- ~~#430 — 1-bit TIFF output for bilevel pages~~ **DONE (Kept, ~149× via Deflate; 1-bit unsupported by tiff 0.9)** → TIFF_DEFLATE

**P2:**
- ~~#431 LUT byte-expansion in bilevel TIFF~~ **DONE (~18%)** → TIFF_LUT · ~~#432 byte-level early-exit in `mask_box_any`~~ **REVERTED (+5% regression)** → MASK_ANY_BYTE
- ~~#433 extend P2 to byte-aligned offset_x~~ **DONE (Kept, safe ext)** → P2_REGION · ~~#434 B-series fg_fx Q48 accumulator~~ **REVERTED (+13–27% regression)** → FG_FX_ACCUM
- ~~#435 B-series all-bg row fast path~~ **DONE (Kept, 52.6% fire)** → BSERIES_ALLBG · ~~#436 MASK_EXPAND in B-series upscale~~ **REVERTED (overhead, unmeasurable)** → BSERIES_MASKEXP
- ~~#437 exact-length FG44/BG44 row slices~~ **REVERTED (unmeasurable, mechanism defeated)** → EXACT_SLICE · ~~#438 area-avg all-bg row fast path~~ **DONE (Kept, 50–71% fire)** → AREAAVG_ALLBG
- ~~#439 proportional fg/bg blend in color area-avg~~ **DONE (Kept, quality)** → COLOR_AA · ~~#440 PAR-DEC parallel layer decode~~ **DONE (Kept, −22% cold)** → PAR_DEC

**P3:**
- #441 hoist NEON splat consts in IDWT · ~~#442 NEON s=2 row pass in IW44 IDWT~~ **REJECTED (incorrect premise: s==2 implies sd==1)** → IDWT_S2_NEON
- ~~#443 F2 fast path for non-identity gamma~~ **DONE (Kept, safe ext)** → F2_GAMMA · ~~#444 cache parsed FGbz palette~~ **REVERTED (+1%, 21-byte chunk)** → FGPAL_CACHE
- ~~#445 rolling 1-bit register in decode_ref_row~~ **REVERTED (col_shift unbounded, no benefit)** → REFROW_REG · ~~#446 O(1) dedup in cluster_shared_symbols~~ **DONE (Kept, O(P²)→O(P))** → CLUSTER_DEDUP
- ~~#447 cache-tiled transpose in rotate_pixmap~~ **DONE (Kept, ~2–6%)** → ROTATE_TILE · ~~#448 Lanczos3 vertical-pass weight precompute~~ **DONE (Kept, −22.5%)** → LANCZOS_HOIST
- ~~#449 PDF streaming render-and-emit~~ **DONE (Kept, strictly ≤ peak)** → PDF_STREAM

**Pre-existing (not from swarm):** #422 bilinear chroma upsampling · #423 Lanczos-3 resampling option

**Still open from prior analysis (no issue yet):** JB2 byte-cost estimator before cross-size emit
(#301; JB2_CROSS6 was +4% size — need cost model first) · IW44 quantization/slice-budget increase
for fine bands (IW44_DIAG: starved in 100 slices, quality floor avg_abs=8.72).
