# CLAUDE.md — agent memory

Lab notebook for Claude. Update this file BEFORE committing any significant
experiment. Goal: avoid re-treading already-explored paths.

---

## Hot-path architecture

```
DjVu decode pipeline:
  IFF parse → chunk dispatch
    ├─ JB2  (bilevel):  ZpDecoder → jb2.rs → bitmap
    ├─ IW44 (color):   ZpDecoder → iw44_new.rs → YCbCr tiles → RGB
    └─ BZZ  (text):    ZpDecoder → MTF/Huffman → UTF-8

ZpDecoder (src/zp/mod.rs) — hottest path:
  decode_bit() called millions of times per page
  fields: a (interval), c (code), fence (cached bound), bit_buf, bit_count
  renormalize() — called on every LPS event (~10-15% of decode_bit calls)

Composite pipeline (src/djvu_render.rs):
  JB2 bitmap + IW44 background → final pixmap
  hot path: composite_bilevel(), composite_color()
```

**Profiling:** `cargo bench --bench codecs` (Criterion, ~2 min)  
**vs DjVuLibre:** `bash scripts/bench_djvulibre.sh .`

---

## Baseline metrics (Apple M1 Max, 2026-04-16, after NEON encoder preliminary_flag_computation)

| Benchmark | Result | vs BENCHMARKS.md (v0.4.1) |
|-----------|--------|---------------------------|
| `jb2_decode` | **131.8 µs** | −42% (was 228 µs) |
| `iw44_decode_first_chunk` | **578 µs** | −21% (was 734 µs) |
| `iw44_decode_corpus_color` | **648 µs** | — |
| `iw44_to_rgb_colorbook/sub1_full_decode` | **5.46 ms** | — |
| `iw44_to_rgb_colorbook/sub2_partial_decode` | **1.31 ms** | — |
| `iw44_to_rgb_colorbook/sub4_partial_decode` | **342 µs** | — |
| `jb2_decode_corpus_bilevel` | **421 µs** | — |
| `jb2_encode` | **182 µs** | — |
| `iw44_encode_color` (boy.djvu 192×256) | **1.80 ms** | — |
| `iw44_encode_large_1024x1024` (synthetic) | **17.5 ms** sequential / **16.2 ms** parallel (−7.4%) | new |
| `render_page/dpi/72` | **240 µs** (warm cache) | (was 1.21 ms in BENCHMARKS.md — major gains since v0.4.1) |
| `render_page/dpi/300` | 4.02 ms | (from BENCHMARKS.md) |
| `render_colorbook_cold` (150 dpi, `parallel`) | **14.1 ms** | −40% vs sequential (23.6 ms before #186) |

> Criterion numbers on M1 Max. Full table with x86_64 and DjVuLibre → BENCHMARKS.md

---

## Experiment log

### ✓ Kept

| Date | Component | Change | Effect |
|------|-----------|--------|--------|
| 2026-04 | ZP/JB2 | local-copy ZP state (register alloc) + hardware CLZ | −15% JB2 |
| 2026-04 | ZP/JB2 | eliminate bounds checks in JB2 hot loops + ZP renormalize | significant |
| 2026-04 | ZP | widen a/c/fence u16→u32, remove all `as u16` casts in hot loop | jb2 −2%, iw44_color −1.8%, jb2_encode −2.2% |
| 2026-04 | IW44 | generalise row_pass SIMD to s=2/4/8 (was s=1 only) | sub2_decode −3.1% (p=0.00); sub1 noise |
| 2026-04 | BZZ | inline ZP state locals in MTF decode hot loop | significant |
| 2026-04 | render | downsampled mask pyramid for composite | 23 ms → 8 ms at 150 dpi |
| 2026-04 | render | partial BG44 decode for sub=4 (skip high-frequency bands) | skip unnecessary work |
| 2026-04 | render | chunks_exact_mut → eliminate per-pixel bounds checks | small |
| 2026-04 | render | x86_64 SSE2/SSSE3 fast paths (alpha fill, RGB→RGBA) | significant on x86_64 |
| 2026-04 | render | parallel BG+mask+FG44 decode via `rayon::join` in `render_pixmap`/`render_region` (#186) | cold render −30% (23.6→16.5 ms); warm-cache +13% overhead (240→272 µs) — rayon::join ~30 µs cost dominates when caches are warm; acceptable because cold render is the dominant real-world case |
| 2026-04 | IW44 | skip `previously_active_coefficient_decoding_pass` when `bbstate & ACTIVE == 0` | iw44_first_chunk −13% (714→623 µs); iw44_corpus_color −46% (2.30→1.25 ms) — avoids function call + ZP register flush for all-zero/UNK blocks (dominant case in sparse/early chunks) |
| 2026-04 | IW44 | local-copy ZP state in `previously_active_coefficient_decoding_pass` (same JB2 pattern) | sub1 −2.1% (13.24→12.96 ms); sub2 −1.5%; sub4 −2.4%; corpus_color −2.4% — LLVM keeps a/c/fence/bit_buf/bit_count in registers for entire coefficient refinement inner loop; small function body avoids I-cache thrash that killed the full-pass inlining attempt |
| 2026-04 | IW44 | NEON-vectorize `preliminary_flag_computation` band≠0 path: 16 i16 coefs → 16 u8 flags in ~14 NEON instructions vs 64 scalar ops | corpus_color −48% (1.25→0.67 ms); first_chunk −7% (623→582 µs); sub1 −3.2% (12.96→12.55 ms) — LLVM was scalar-unrolling the 16-iter loop; explicit NEON (vld1q×2, vceq×2, vmvn×2, vand×2, veor×2, vmovn×2, vst1q + horizontal OR) reduces per-bucket work ~3× on M1 NEON; bands 1-9 each call this per block so corpus_color (many bands) sees the largest gain |
| 2026-04 | IW44 | NEON-vectorize `preliminary_flag_computation` band-0 path: vbslq_u8 blend to handle conditional update for ZERO-state entries | corpus_color −3.9% (667→650 µs); sub1/first_chunk flat — band-0 conditional update (skip ZERO entries) done with vceqq_u8 + vmvnq_u8 mask + vbslq blend; ~20 NEON instructions vs 48 scalar |
| 2026-04 | IW44 | Extend column-pass SIMD from `s=1` to `s≤4`: `vld2q_s16`/`vld4q_s16` gather for s=2/4 loads, scatter `str h` for stores (s=2,4 can't use vst2/vst4 without extra read-back load) | sub1 −6.1% (12.84→12.06 ms); sub2 −3% (3.35→3.25 ms); sub4 −3.4% (821→793 µs) — NEON deinterleave reduces scalar i16-to-i32 widening overhead at coarser levels; scatter stores avoid extra vld2q reload that tripled memory traffic in initial vst2q approach |
| 2026-04 | IW44 | NEON-vectorize `ycbcr_row_to_rgba`: explicit `vld1q_s32`×6 + SIMD arithmetic + `vst4_u8` replaces LLVM-generated code that emitted 80+ bounds-check branches per 8 pixels | sub1 −7.3% (12.06→11.51 ms); sub2 −8.3% (3.25→2.98 ms); sub4 −7.7% (793→733 µs) — profiling (samply, 6522 samples) showed ycbcr_row_to_rgba at 12.5% self-time; assembly revealed `memset_pattern16` init + massive cmp/b.hs forest from `wide::i32x8::from([scalar...])` constructors; `vst4_u8` writes 32 interleaved RGBA bytes in one instruction vs 32 individual strb |
| 2026-04 | IW44 | `get_unchecked` in `load_rows8`/`store_rows8` (row-pass scatter/gather) | sub1 −13.3% (11.51→9.98 ms); sub2 −10% (2.98→2.68 ms); sub4 −10.5% (733→656 µs) — assembly showed 5× `cmp+b.hs` per load cluster + `fmov+mov.s×7` scalar-to-vector; removing bounds checks let LLVM eliminate conditional branches and improve instruction scheduling across the scatter loop |
| 2026-04 | IW44 | Horizontal row-pass NEON (s=1): `row_pass_neon_s1_row` replaces 8-rows-at-a-time scatter with `vld2q_s16` + `vextq_s16` sliding window per row | sub1 −5.1% (9.98→9.47 ms); sub2 −7.8% (2.68→2.47 ms); sub4 −5.6% (656→619 µs) — eliminates `8×ldrh + 7×fmov/mov.s` scatter per column position; even pass: 3 loads (`vld2q_s16` ×2 + `vld2q_s16` ahead) + 4 `vextq_s16` for all neighbors of 8 evens; odd pass: 2 loads + 4 `vextq_s16`; scalar tail handles boundary; `vst2q_s16` reinterleaves updated even/odd back in one store |
| 2026-04 | IW44 | `get_unchecked` in `load8_i32`/`store8_i32` (column-pass st0/st1/st2 temporary arrays) | sub1 −8.4% (9.47→8.67 ms); sub2 −4.8% (2.47→2.35 ms); sub4 −7.8% (619→569 µs) — profile showed `fmt::Debug`+`panic_fmt` at 6.7% self-time; identical pattern to `load_rows8` bounds-check overhead; `ci+7 < simd_cols ≤ num_cols` invariant guarantees safety at all call sites |
| 2026-04 | IW44 | Skip zero-init in `reconstruct` plane allocation (uninit Vec + set_len) | sub1 −3.7% (8.67→8.34 ms); sub2 −3.4% (2.35→2.23 ms); sub4 −2.2% (569→560 µs) — ZIGZAG_ROW/COL is a bijection: i∈[0,1024) maps to every position in a 32×32 block exactly once; for compact path, i∈[0,coeff_limit) maps to every position in sub_block² exactly once; so vec![0i16;n] is pure redundant memset (~3–9 MB/to_rgb() across 3 planes); replaced with Vec::with_capacity + set_len |
| 2026-04 | IW44 | Row-major scatter in `reconstruct` full-res path via `ZIGZAG_INV` table | sub1 −2.0% (8.34→8.20 ms); sub2/sub4 unaffected (compact path unchanged) — zigzag order spreads writes across all 32 rows of a block simultaneously, preventing write-combine buffer coalescing; row-major order fills 1 cache line (32 i16 = 64 bytes) per row before advancing; reads from 2 KB `block` array remain in L1 |
| 2026-04 | IW44 | Row-major scatter in `reconstruct` compact path via `ZIGZAG_INV_SUB2/4/8` tables | sub2 −7.2% (2.23→2.12 ms, p=0.00); sub4 −6.5% (560→540 µs, p=0.00); sub1 flat — same write-combine benefit as full-res path but larger relative gain because compact blocks are smaller (16×16/8×8/4×4): fewer open cache lines during scatter means greater contention relief; `ZIGZAG_INV_SUBn` tables use u8 (max index 255) totaling 336 bytes (fits in L1 data cache) |
| 2026-04 | IW44 | `get_unchecked` in compact scatter (after row-major rewrite; sequential writes now enable vectorization) | sub2 −6.6% (2.12→1.98 ms, p=0.00); sub4 −5.4% (540→511 µs, p=0.00); sub1 flat — profile showed 9.3% self-time in `panic_fmt`/`fmt::Debug` (bounds-check speculation overhead) from compact scatter; with row-major writes, LLVM can vectorize the sequential store side once bounds checks are removed; previous attempt (zigzag scatter, non-sequential writes) was +4.4% worse — write-side non-sequential was the blocker then |
| 2026-04 | IW44 | `const` rounding constants for `lifting_even`/`predict_inner`/`predict_avg` (replace `i32x8::splat(N)` with `const C: i32x8 = unsafe { transmute([N; 8]) }`) | sub1 −22% (8.20→6.40 ms); sub2 −20% (1.98→1.58 ms); sub4 −22% (511→399 µs) — `i32x8::splat(N)` compiled to `bl memcpy` (PLT stub, 32-byte `.rodata` copy) inside the hot k-loop for each of lifting_even/predict_inner/predict_avg; LLVM treated `splat()` as non-pure and didn't hoist or inline to `movi.4s`; `const` transmute produces a static rodata entry that LLVM loads with `ldp q`/`ldr q` (1-2 instructions) and hoists out of the loop; samply profile showed ~20% of samples in Debug/panic infra from the memcpy overhead |
| 2026-04 | IW44 | fused normalize+YCbCr: `ycbcr_neon_raw`/`ycbcr_neon_raw_half` read i16 plane data directly, inline `vrshrq_n_s16` normalization, eliminate 3 intermediate i32 buffers and separate normalize loops | sub2 −6.2% (1.58→1.51 ms, p=0.00); sub4 −0.9% (399→395 µs, p=0.03); sub1 flat (+1.2% noise, p=0.08) — colorbook.djvu has `chroma_half=false` (minor=1): sub1 uses `ycbcr_neon_raw` (non-half) but `ycbcr_neon_raw_half` is never reached by this file; parallel path also eliminates 3 `vec![0i32; pw]` allocations per row |
| 2026-04 | IW44 | i16 YCbCr arithmetic in `ycbcr_neon_raw`/`ycbcr_neon_raw_half`: after normalize+clamp all intermediates fit in i16 (r16∈[-192,445], g16∈[-126,383], b16∈[-287,541]); `vqmovun_s16` saturates i16→u8 in one op, eliminating 6 `vmovl` widenings + 12 i32 arithmetic ops + 12 i32 min/max ops + 9 narrows per 8 pixels (42→13 ops for arithmetic+clamp+narrow) | sub1 −5.0% (6.40→6.10 ms, p=0.00); sub2 −4.6% (1.51→1.45 ms, p=0.00); sub4 −8.2% (395→370 µs, p=0.00) — profiling (samply, 7087 samples) showed `ycbcr_neon_raw` at 8.8% leaf time; larger sub4 gain because YCbCr fraction grows as wavelet work shrinks at coarser levels |
| 2026-04 | IW44 | Hoist `has_n3` branch out of even-pass ci inner loop: split `while k <= kmax` into main loop (`while k+3 <= kmax`, no conditional) + tail loop (`while k <= kmax`, zero n3) | sub1/sub2/sub4 within noise — structurally cleaner but M1 branch predictor handles the original perfectly; no measurable effect |
| 2026-04 | IW44 | `load8s`/`store8s` s=1 fast path: move `if s == 1` check to top with `core::ptr::read/write<[i16;8]>` for contiguous load/store, bypassing 5-branch `match s` dispatch chain inside `load8s_neon`/`store8s_neon` | sub1 −6.2% (5.80→5.46 ms, p=0.00); sub2 −11.4% (1.48→1.31 ms, p=0.00); sub4 −9.1% (376→342 µs, p=0.00) — assembly: s=1 hot path reduced from 5-branch `match`-dispatch to single `cmp+b.ne` before `ldp d18,d19+sshll×2`; each ci-iteration saves 4 dispatch branches × 3 calls (2 loads + 1 store) = 12 fewer branches; `ldp d,d` (2×64-bit) generates same memory traffic as `ldr q` (1×128-bit); sub2/sub4 gain larger because their wavelet planes are smaller and fit better in L1 after branch reduction |
| 2026-04 | IW44 encoder | NEON-vectorize encoder's `preliminary_flag_computation` band≠0 and band-0 paths (i32 recon data): 4 × `vld1q_s32` + `vceqq_s32` + narrowing chain `vmovn_u32→vmovn_u16` + `veorq_u8`/`vandq_u8` + `vst1q_u8`; band-0 uses `vbslq_u8` blend to preserve ZERO entries | `iw44_encode_color` −5.3% (2.11→2.04 ms, p=0.00) — scalar loop not auto-vectorized by LLVM due to `bstatetmp |= ...` accumulation; i32 source requires 4 × `vld1q_s32` (vs decoder's 2 × `vld1q_s16`); ~25 NEON instructions replaces 48+ scalar ops per 16-element bucket |
| 2026-04 | IW44 encoder | NEON `forward_row_neon_s1_row` + `forward_col_predict_neon` for `forward_wavelet_transform` at s=1: mirror of decoder's `row_pass_neon_s1_row` with sign-dual ops (predict subtracts, lift adds, odd pass first then even pass); col pass inner predict uses 5 × `vld1q_s16` + `vmovl_s16`/`vshlq_n_s32`/`vshrq_n_s32`/`vsubq_s16` + `vst1q_s16` per 8 columns | `iw44_encode_color` −5.1% (2.04→1.99 ms, p=0.00) — LLVM generated pure scalar `ldrsh`/`strh` for both row and col passes; explicit NEON processes 8 positions per iteration; assembly confirmed 0 vector instructions before this change; row pass NEON matches decoder's `smull.4s + rshrn.4h` form after inlining |
| 2026-04 | IW44 encoder | NEON `forward_col_lift_neon_row` for col pass Step 2 at s=1: Vec<i32> state → Vec<i16> + 8-wide `smull.4s + rshrn.4h` + `add.8h`; state advance via 3 × `str q`; `movi.2d v18, #0` for !has_n3 case | flat on boy.djvu (192×256); assembly confirmed `smull.4s + rshrn.4h + add.8h` NEON path active — benefit proportional to image size; no regression (p=0.00) |
| 2026-04 | IW44 encoder | skip `previously_active_encoding_pass` when `bbstate & ACTIVE == 0` (mirrors decoder's `if (self.bbstate & ACTIVE) != 0` guard at iw44_new.rs:712) | `iw44_encode_color` −9.8% (1.99→1.83 ms, p=0.00) — encoder was unconditionally iterating 16×num_buckets coefficients per block doing `continue` checks; in early slices most blocks are all-UNK so the function body was pure loop overhead; decoder already skips when no ACTIVE coefficients exist |
| 2026-04 | IW44 encoder | parallel forward_wavelet_transform + gather for Y/Cb/Cr planes under `feature = "parallel"` (mirrors decoder's rayon::join of Y/Cb/Cr reconstruct): threshold guard `w*h > 512*512` to avoid rayon overhead dominating on small images | flat on boy.djvu (192×256 < threshold, falls back to sequential); expected gain ≈ 20–33% of wavelet+gather time (≈3–5% total) for large images (512×512+) where sequential Y+Cb+Cr ≈ 6×Cb vs parallel max(Y, Cb, Cr) ≈ 4×Cb |
| 2026-04 | benchmarks | Added `bench_iw44_encode_large_1024x1024`: synthetic 1024×1024 gradient pixmap, exercises parallel encoder path (w×h > 512² threshold) | sequential 17.5 ms, parallel 16.2 ms (−7.4%, M1 Max); confirms parallel encoder works for large images; available as regression baseline |
| 2026-04 | IW44 encoder | `gather()` get_unchecked: remove dead `if idx < plane.len()` branch (plane allocated as `stride×plane_h`, zigzag indices always within bounds by construction) | flat on all benchmarks — M1 branch predictor handled the always-true branch at zero cost; bounds-check removal leaves no measurable footprint; kept for code correctness/clarity |
| 2026-04 | ZP encoder | `encode_passthrough_iw44`/`encode_passthrough` !bit path: remove dead `if self.a >= 0x8000` guard (z = 0x8000 + 3a/8 ≥ 0x8000 always since `a < 0x8000` invariant) | flat on all benchmarks — LLVM already folded the always-true branch; kept for code clarity; invariant documented in comments |
| 2026-04 | JB2 encoder | pre-expand `Bitmap` to byte-per-pixel array with 2 zero rows above + 4 zero columns right before `encode_bitmap_direct` inner loop; eliminates `pix_bm` (4 bounds comparisons + bit-unpack per call, 3 calls/pixel) → direct byte array reads with no bounds checks or bit manipulation | pending stable benchmarks; assembly confirmed: LLVM uses precomputed base pointers (`x20 = row_p2.ptr+2`, `x19 = row_p1.ptr+3`) → zero bounds checks for row pixel reads in hot path; pix_bm removed entirely |
| 2026-04 | ZP tables | pad PROB/THRESHOLD/MPS_NEXT/LPS_NEXT from 251 to 256 entries (5 dummy entries at end); LLVM can now prove `state < 256` for any `u8`-cast index and eliminate the `cmp x8, #250; b.hi` PROB bounds check from both encoder and decoder `encode_bit`/`decode_bit` hot loops | assembly confirmed: `cmp #250` branch no longer appears in encode_jb2 inner loop |
| 2026-04 | JB2 encoder | `get_unchecked_mut(idx)` for ctx array in `encode_bitmap_direct` inner loop; safety: r2 ≤ 7, r1 ≤ 31, r0 ≤ 3 by the `& 0b111` / `& 0b11111` / `& 0b11` masks, so idx = (r2<<7)|(r1<<2)|r0 ≤ 1023 < 1024 = ctx.len(); eliminates `cmp x0, x1; b.hs` from hot path | assembly confirmed: encoding inner loop now has zero bounds-check branches; only legitimate ZP arithmetic branches remain (`cmp w8, #0x8000` for a≥0x8000 check) |

### ✗ Reverted

| Date | Component | What was tried | Why reverted |
|------|-----------|----------------|--------------|
| 2026-04 | render | bilevel composite fast path (#165) | regression — restored in #169 |
| 2026-04 | ZP | `#[cold] #[inline(never)]` for LPS branch + cmov-friendly context update | iw44 +4%, jb2_encode +2% — function call overhead > I-cache gain; LPS fires 10-15% of calls, too frequent for out-of-line |
| 2026-04 | IW44 | early-exit `decode_slice` when `zp.is_exhausted() && bbstate & ACTIVE == 0` (#182) | 99.2% pixel mismatch — `is_exhausted()` fires mid-stream (not end-of-decisions); skipping decode_bit corrupts ZP arithmetic state for all subsequent calls; the ZP stream is a continuous encoding of ALL block decisions; can't skip any call without desynchronising |
| 2026-04 | IW44 | local-copy ZP state + inline all 4 ZP sub-passes in `decode_slice` (macro-based, same pattern as JB2) | +7% `iw44_decode_first_chunk`, +25% `iw44_decode_corpus_color` — I-cache thrash from large inlined function body; IW44 block-loop body is much larger than JB2 row-loop, so I-cache pressure dominates any register-allocation gain |
| 2026-04 | IW44 | `any_coef_nonzero` flag to skip block-data scan in `preliminary_flag_computation` for all-zero images | +5% `iw44_decode_first_chunk` regression — adding bool to `PlaneDecoder` struct increases cache pressure; branch overhead in tight loop + `fill(UNK)` not faster than vectorized load-compare-store |
| 2026-04 | IW44 | column_pass SIMD at s=2 via runtime `s==1` dispatch + `load8_stride2`/`store8_stride2` (#184 attempt 1) | +5% `iw44_decode_first_chunk` (623→654 µs), −2.4% `iw44_decode_corpus_color`; sub1 +6.5%, sub2 +6.8% — I-cache pressure from doubled dispatch code in large column-pass body; net negative |
| 2026-04 | IW44 | column_pass SIMD at s=2 via const-generic `column_pass<const S>` monomorphization (#184 attempt 2) | sub1 +22% (13.24→16.2 ms), sub2 +25%, corpus_color −3.2% — extracting column_pass as non-inlined function loses LLVM register allocation across outer s-loop; column pass too tightly coupled to outer loop for safe extraction without inlining |
| 2026-04 | IW44 | local-copy ZP state in `bucket_decoding_pass` + `newly_active_coefficient_decoding_pass` (extending JB2 pattern) | first_chunk +4%, corpus_color +3.3% — extract/writeback overhead (14 register-move ops × 74 880 blocks ≈ 328 µs) exceeds ZP-in-register savings; breakeven requires ≥7 ZP calls/block avg; `bucket_decoding_pass` avg 1-4 calls, `newly_active` rare (most blocks are UNK/ZERO not NEW) — net negative for both |
| 2026-04 | IW44 | bucket-level early exit in `previously_active_coefficient_decoding_pass` (skip bucket if `bucketstate[boff] & ACTIVE == 0`) | corpus_color +1.5%, sub1 +1.1% — benchmark corpus files are dense (most buckets ACTIVE in later slices); branch overhead per bucket exceeds savings; only helps for very sparse images |
| 2026-04 | IW44 | `get_unchecked` on zigzag scatter in `PlaneDecoder::reconstruct` (both compact and full-res paths) | compact path +4.4% sub4 (consistent, p=0.00); full-res path flat ±0% — scatter loop is memory-bound (writes to non-sequential addresses in 3.2 MB plane); cache-miss latency dominates; no benefit from removing bounds-check branches unlike `load8_i32` arithmetic loops |
| 2026-04 | IW44 | `get_unchecked` on full-res scatter (after row-major rewrite) | sub1 +2.1% (p=0.00); sub2/sub4 flat — LLVM generates slightly worse instruction scheduling for full-res path without bounds checks; full-res scatter over 1 MB plane is still memory-latency-bound even with sequential writes; compact path benefits but full-res does not |
| 2026-04 | IW44 encoder | local-copy ZP state in `previously_active_encoding_pass` (macro-based: outbit + zemit + encode_bit + encode_passthrough_iw44 inlined) | `iw44_encode_color` +1.8% (2.11→2.15 ms, p=0.00) — I-cache pressure from expanded macro body (7 encoder state fields + zemit/outbit chains) exceeds register-allocation gain; encoder slow paths (zemit → outbit → Vec::push) generate more code than decoder's renorm; breakeven never reached even though ≤64 ZP calls/block when fully ACTIVE |
| 2026-04 | IW44 encoder | `get_unchecked` in `forward_row_pass` and `forward_col_pass` inner loops (data array + prev3/prev1/next1 Vec accesses) | flat: 0.3% improvement, p=0.10 — eliminates 35 `panic_bounds_check` symbols and enables partial LLVM NEON auto-vectorization (9 NEON instructions vs 0), but M1 branch predictor handles the original bounds-check branches at effectively zero cost; lifting sliding-window dependency (prev3/prev1/next1 sequential read-write) prevents full vectorization of the hottest loop regardless |
| 2026-04 | IW44 encoder | `#[inline(always)]` on `ZpEncoder::zemit` and `outbit` (samply profile shows zemit at 15.2% leaf time = not inlined, top=0 fast path is just `nrun++` = dominated by call overhead) | `iw44_encode_color` +3.1% (p=0.00) — same I-cache pressure as local-copy regression: zemit+outbit expand into encode_lps/encode_mps, which are already inlined into encode_bit, which is inlined into encode_slice; the expanded function body for encode_slice grows past I-cache line boundaries; call overhead for zemit is ~5 cycles/call but function body saving is outweighed by I-cache miss cost |
| 2026-04 | IW44 encoder | split zemit into `#[inline(always)]` hot path (buffer update + `if top==0 { nrun++ } else { call zemit_carry }`) + `#[cold] #[inline(never)]` zemit_carry for top≠0 | `iw44_encode_color` +2.0% (p=0.00) — same I-cache pattern as above: any form of inlining zemit's body at the encode_lps/encode_mps call sites expands the encode_slice function body past cache-line boundaries; the CPU's call predictor handles the direct function call at zero effective cost; rule: zemit must remain a non-inlined function call |
| 2026-04 | IW44 encoder | NEON `rgba_to_ycbcr_row_neon`: explicit `vld4_u8` + `vmovl_u8` + arithmetic + `vst1q_s16×3` per 8 pixels to replace scalar RGB→YCbCr loop (encode_iw44_color 8.5% leaf) | `iw44_encode_color` +2.9% (p=0.00) — LLVM already auto-vectorizes the original scalar loop with `ld4.8b` NEON; explicit NEON function is no faster and may introduce overhead from different register spilling or loop structure; assembly confirmed `ld4.8b { v1-v4 }, [x13], #32` in original code |
| 2026-04 | IW44 | split `int16x8x2_t curr_pair` into `curr_even`/`curr_odd` in even-pass loop to eliminate "redundant" ld2.8h carry across iterations | sub1 −1.3% (p=0.00) but sub2 +2.1% (p=0.00) — net negative; the "redundant" ld2 is a sequential L1 hit (~free on M1); restructuring hurts LLVM's instr scheduling for sub2; "carry-in-registers" only wins for very long row bodies, not here |
| 2026-04 | IW44 | replace `vmovl_s16×2 + vaddq_s32` with `vaddl_s16` in even-pass lift (saves 8 instr/chunk) | sub1 +1.7% (p=0.00) — `saddl` has 2/cycle throughput on M1 vs 4/cycle for `sxtl`; using a lower-throughput instruction to save instruction count is a net loss; M1's even-pass lift is throughput-bound on `add.4s`/`sxtl`-class instructions (4/cycle units) |
| 2026-04 | IW44 | `srshr5_i32x8`/`srshr4_i32x8` wrappers using `vrshrq_n_s32` to fold bias into rounding shift (save 2 `add.4s` per even/odd lifting iteration) | sub1 +1.2%, sub2 −0.9%, sub4 −0.6% — all within noise (Criterion p=0.00 but tiny magnitude); assembly confirms `srshr.4s #5` for `lifting_even` but `srshr.4s #4` for `predict_inner` is absent (LLVM absorbs it into narrowing path); ~2 M fewer instructions but M1's column pass is not instruction-count-bound at this level; complex unsafe transmute code not justified by zero measurable gain |
| 2026-04 | IW44 | `row_pass_neon_s2_all`: gather active (even physical) columns into temp buffer via `vld2q_s16`, run `row_pass_neon_s1_row` on contiguous buffer, scatter back with 8 × `vst1_lane_s16` per chunk — intended to reduce L1 pressure from 8-rows-at-a-time i32x8 path (36 KB stride span vs 6 KB) | sub2 +8.4%, sub4 +8.8%, sub1 +3.8% — all regressions; `vst1_lane_s16` scatter stores each write a single 2-byte lane to a non-sequential address (8 independent str h per chunk); this produces 8 distinct cache-line writes per 16-element group vs 1 vst2q write in the i32x8 path; M1's prefetcher handles the strided 36 KB working set efficiently; Vec allocation per to_rgb() call also adds heap overhead |
| 2026-04 | IW44 | `row_pass_neon_s2_row`: horizontal NEON row pass for s=2 using `vld4q_s16` to deinterleave active-even (.0) and active-odd (.2) from 32 consecutive physical i16s; even pass stored back with `vst4q_s16` (preserving inactive lanes); odd pass via 8 × `vst1_lane_s16` scatter stores at stride 4 | sub1 +4.6% (p=0.00), sub2 +2.7% (p=0.00), sub4 flat — regression; even pass with `vst4q` is clean, but odd pass scatter stores (8 × `str h` to non-sequential physical 6,10,...,34 within chunk) are the bottleneck; same scatter-store cost as `row_pass_neon_s2_all` kills the gain from better even-pass cache behaviour; M1 store buffer saturates on 8 independent partial-cache-line writes per 16 logical positions |
| 2026-04 | ZP encoder | `encode_passthrough_iw44`/`encode_passthrough` bit=true while→loop conversion: `while a >= 0x8000 { ... }` → `loop { ...; if a < 0x8000 { break } }` (do-while form, initial condition provably true) | `iw44_encode_large` +4% (17.5→18.2 ms, p=0.00) — LLVM generates worse code for `loop { body; if !cond { break } }` than for `while cond { body }` in this context; the `while` form enables LLVM to prove the loop terminates and schedule instructions across the exit; reverted to `while` |
| 2026-04 | JB2 encoder | local-copy ZP state in `encode_bitmap_direct` via `encode_step!` macro (mirrors JB2 decoder's pattern: copy `a` to local, inline fast path `a = z`, call `zp.encode_bit()` for slow paths with write-back/reload) | inconclusive — high system load (load avg 22) made Criterion results unreliable (first run −7.5%, subsequent runs +20-30%); theoretical basis questionable: `zp: &mut ZpEncoder` and `ctx: &mut [u8]` are provably non-aliasing to LLVM (Rust borrow checker), so LLVM can already keep `zp.a` in registers without explicit local-copy; the decoder's local-copy works because the aliasing situation is structurally different (ZpDecoder holds `&[u8]` input, not `&mut`); reverted pending stable benchmarking environment |
| 2026-04 | ZP | 8-wide SIMD `decode_bit8` across parallel lanes (#183) | not implemented — fundamental misconception in the issue: a single ZP stream has one shared `(a, c, fence, bit_buf, bit_count, pos)` register; there are no 8 independent `(a, c)` tuples to put in lanes (JB2/IW44/BZZ all use one stream per chunk, `decode_bit` N+1 has hard data-dep on N). Viable reformulation = speculative fast-path prefix-sum batching (gather `PROB[ctx]`, prefix sum, threshold vs `fence`, ctz to find first LPS, scalar tail), but: (1) fast path already ~1–2 ns on M1 (~5–6 instr); (2) batch setup (gather+prefix+ctz+dispatch) ~15–25 ns ≈ full 8-call saving; (3) avg fast-path run ~7–10 before LPS; (4) only `previously_active_coefficient_decoding_pass` has structurally batchable ctx sequences among all callers, and it's already hand-tuned with local-copy ZP state — further body expansion triggers the same I-cache regression pattern that killed `#[cold]` LPS (+4%), 4-pass inlined decode_slice (+25%), branch-free cmov, and local-copy in `bucket_decoding_pass`/`newly_active` (+4%/+3.3%). Realistic ceiling 1–2% on `iw44_decode_corpus_color`, below Criterion noise. Issue closed. |

> **Rule:** if you revert something, add a row here with the reason — otherwise it will be tried again.

### → Hypotheses (not yet measured)

| Component | Idea | Expected | Risk |
|-----------|------|----------|------|
| ZP | SIMD decode of multiple symbols in parallel (8-wide) (#183) | ✗ not viable — see log | single stream has one shared (a,c); fast-path already ~1-2 ns; I-cache pattern regresses |
| ZP | branch-free decode_bit via cmov (#179) | ✗ reverted — see log | LPS function call overhead worse than inline |
| IW44 | column_pass SIMD at s=2 (#184) | ✓ kept (attempt 3) — see log | `load8s`/`store8s` with `vld2q_s16`/`vld4q_s16` + scatter-stores within existing `use_simd = s <= 4` body; no extraction, no dispatch overhead |
| JB2 | bit-pack bitmap → smaller memory/cache footprint (#185) | medium | complex |
| render | pre-decode JB2 bitmap on a separate thread (#186) | ✓ kept — see log | −30% cold render |
| ZP | LUT for frequent states (#181) | ✓ done differently | padded tables 251→256 eliminates the `PROB[state]` bounds check from both encoder/decoder hot loops; combined struct not worth it (same ~1.5 KB total, L1 always warm) |
| IW44 | early-exit in `decode_slice` when ZP exhausted + no ACTIVE blocks (#182) | ✗ reverted — see log | ZP stream is a continuous encoding of all decisions; skipping any call desynchronises state |
| IW44 | horizontal row-pass NEON for s=2 (any approach) | ✗ reverted × 2 — see log | both `row_pass_neon_s2_all` (temp-buffer) and `row_pass_neon_s2_row` (vld4q_s16 in-place) regress; fundamental bottleneck: odd-pass must scatter 8 values at stride 4 → 8 independent `str h` per chunk regardless of how evens are loaded; M1 store buffer saturates on partial cache-line writes; the existing i32x8 8-rows-at-a-time path with NEON load8s is already well-optimised for this stride pattern |

---

## Investigations

### JB2 encoder quality baseline vs cjb2 (2026-04-24)

**Setup:** `examples/encode_quality_jb2.rs` re-encodes every Sjbz chunk from `tests/corpus/*.djvu` (36 JB2 pages, originally produced by cjb2 at archival time) via djvu-rs's `encode_jb2` and compares payload sizes.

| Metric | cjb2 (original) | djvu-rs (re-encoded) | Ratio |
|--------|-----------------|----------------------|-------|
| Total payload | 182 950 B (0.0019 bpp) | 627 218 B (0.0065 bpp) | **3.43× worse** |

**Per-page ratios range:** 1.004× (dense text) to 2.15× (near-empty pages with overhead). The gap is smaller than the 5–10× I predicted — adaptive ZP arithmetic coding alone already recovers most of the compression on redundant bilevel content.

**Root cause** of the gap, per `src/jb2_encode.rs:9`:
> «The encoder emits the entire image as a single record type 3 ("new symbol, direct, blit only") record. This produces valid output without requiring connected-component analysis or a symbol dictionary.»

A CC-analysis + symbol dictionary encoder (#188) should close ≥80% of this gap; remaining ≤20% from refinement matching + multi-page shared dict (#194).

**Side-finding: self-incompatibility bug (#198)**. All 36 re-encoded streams hit `Jb2Error::ImageTooLarge` when fed back through djvu-rs's decoder. Decoder has `MAX_SYMBOL_PIXELS = 1 MP` DoS guard (jb2.rs:362) but encoder emits full-image symbols up to 64 MP. DjVuLibre `ddjvu` likely accepts the output fine — not verified yet. Fix tracked as #198 (tiled direct-blit, few dozen LOC) — naturally obsoleted by #188 once it lands.

**Baseline to track:** total ratio **3.43×**. Any encoder change should measurably reduce this; track per-release.

---

### JB2 symbol-dictionary encoder Phase 1 (#188, 2026-04-25)

**Setup:** Same harness (`encode_quality_jb2`), now measuring both `encode_jb2` (single record-type-3, direct) and `encode_jb2_dict` (CC extraction + record types 1 + 7, exact-match dedup). `tests/corpus/*.djvu` (36 JB2 pages).

| Encoder | Total payload | bpp | Ratio vs cjb2 |
|---------|---------------|-----|---------------|
| cjb2 (original) | 182 950 B | 0.0019 | 1.000× |
| djvu-rs direct (single record-3) | 627 218 B | 0.0065 | **3.43× worse** |
| **djvu-rs dict (record 1 + 7)** | **209 024 B** | **0.0022** | **1.143× worse** |

- **3× improvement** over direct encoding (3.43× → 1.143×). Essentially meets #188 expected outcome of "80–100% of cjb2 compression" on the first phase.
- **#198 naturally bypassed**: all 36 pages roundtrip OK (0 decode errors, 0 mismatches). Individual CCs are tiny (≪ 1 MP), so the decoder's `MAX_SYMBOL_PIXELS` guard never fires.
- On sparse / low-content pages, dict encoder is already *better* than cjb2 (e.g. page 20 of `conquete_paix.djvu`: **0.947×**, page 4: 0.960×). cjb2 has fixed dict-preamble overhead; ours doesn't emit a dict record.
- On dense text pages (#188's target), we close most of the gap but not all — e.g. `conquete_paix` page 1: 6.42× → 1.36×. The remaining gap comes from:
  1. `new_line = true` coordinate coding for every symbol (Phase 1 MVP simplification; optimize with baseline-relative same-line later).
  2. No refinement matching (record type 2/4) — every near-duplicate glyph emits its own dict entry.

**Implementation:** `src/jb2_encode.rs` — `extract_ccs()` (8-connected iterative DFS on unpacked byte grid) + `encode_jb2_dict()` (dedup via `BTreeMap<(w, h, data), dict_idx>`, emit rec 1 on first sight, rec 7 on repeat). 10 new round-trip unit tests pass. ~220 LOC added.

**Next phases of #188:**
- Phase 2 (same-line coord coding): reuse `shoff`/`svoff` contexts, cluster CCs into baselines, expect −10-20% on dense text.
- Phase 3 (refinement matching, rec type 4): near-duplicate clustering (Hamming distance), expect further −10-20%, target 0.9–1.0× parity on scanned books.
- Phase 4 (multi-page shared Djbz #194): archival ratio ≤ cjb2.

---

### IW44 `to_rgb()` profile breakdown (2026-04-16)

**Setup:** 500 iters of `img.to_rgb()` on colorbook.djvu, samply @ ~1 ksample/s.

| Function | Self% | Notes |
|----------|-------|-------|
| `inverse_wavelet_transform_from` | **63.8%** | All wavelet passes inlined (~20 KB). Hot spots at +0x214c/+0x2164: NEON odd-prediction inner loop (`ld2.8h`+`smull`+`ext.16b`+`st2.8h`). |
| `PlaneDecoder::reconstruct` | **19.8%** | Scatter loop (full-res Y plane). Memory-bound; row-major ZIGZAG_INV reduced by ~2% via write-combine. |
| `Iw44Image::to_rgb_subsample` | 6.6% | YCbCr→RGBA conversion + orchestration. |
| `panic_fmt` / `Debug::fmt` | ~3% | Remaining bounds-check overhead somewhere in wavelet code. |

**Key finding:** The wavelet (63.8%) is the dominant cost. It's already NEON-optimized for s=1 column+row passes and s=2,4 column passes. Row pass for s=2 uses the vertical 8-row SIMD (not horizontal NEON). The hottest instruction-level bottleneck is the NEON `smull`/`smlal` throughput in the s=1 odd prediction pass — this is probably throughput-saturated on M1.

---

### IW44 vs JB2 "17× slower" mystery (2026-04-15)

**Question:** Why is `iw44_decode_corpus_color` (2.30 ms) ~17× slower than `jb2_decode` (131 µs)?

**TL;DR:** The comparison is mostly apples-to-oranges (173× more pixels). The remaining real gap is dominated by per-block ZP overhead on padding bytes, not algorithmic inefficiency.

#### 1. The files are completely different sizes

| Benchmark | File | Page size | Blocks (32×32) |
|-----------|------|-----------|----------------|
| `jb2_decode` | `boy_jb2.djvu` | 192×256 | 48 |
| `iw44_decode_corpus_color` | `watchmaker.djvu` | 2550×3301 | **8 320** |

Page area ratio: 8 673 300 / 49 152 = **176×**. The two benchmarks simply measure different amounts of work; the 17× wall-clock difference is mild given that fact.

#### 2. Breakdown of watchmaker.djvu first-chunk decode (2252 µs measured)

| Phase | Cost | % total |
|-------|------|---------|
| Block allocation (8320 × 1024 × i16 = 17 MB) | ~298 µs | 13% |
| ZP decode overhead on padding bytes | ~1 955 µs | 87% |

The entire first chunk is only **819 bytes** of BG44 payload. That is enough ZP data to make real block decisions, but the decoder must iterate all 8 320 blocks × 9 bands afterward anyway.

#### 3. Root cause: 74 880 forced `decode_bit` calls on 0xFF padding

In `block_band_decoding_pass` (iw44_new.rs):
```rust
let should_mark_new = bcount < 16          // false for bands 1–9 (bcount ≥ 16)
    || (self.bbstate & ACTIVE) != 0        // false: fresh image, all UNK
    || ((self.bbstate & UNK) != 0 && zp.decode_bit(&mut self.ctx_decode_bucket[0]));
```

For a freshly initialized image all blocks start as `UNK`. Bands 1–9 have `bcount ≥ 16`, so the third arm fires for every one of the 8 320 blocks, calling `decode_bit` once each. That is **9 × 8 320 = 74 880 calls**. After the 819-byte input is consumed the ZP decoder continues with deterministic 0xFF padding — still executing the full arithmetic-coder state machine each call.

At ~26 ns/call (measured): 74 880 × 26 ns ≈ **1.95 ms** — matches the observed overhead.

JB2 does not have this problem: it decodes a token stream that terminates on an end-of-image symbol, so it never iterates over all possible pixel positions.

#### 4. Optimization attempt — #182 (REVERTED 2026-04-15)

Tried early-exit in `decode_slice`: `if zp.is_exhausted() && (bbstate & ACTIVE) == 0 { continue }`.

Result: **99.2% pixel mismatch** (big_scanned, chicken). Root cause: `is_exhausted()` checks only the byte buffer (`pos >= data.len()`), but the ZP coder is a **continuous bit stream** — each `decode_bit` call advances a shared arithmetic state. Skipping any call desynchronises all subsequent calls for that chunk. `is_exhausted()` can fire mid-stream (e.g. when 0.088 bits/block compression means 819 bytes cover block 1 through ~74 000 of 74 880 total), so blocks well before the end of the sweep get the wrong decisions.

No safe early-exit is possible without changing the encoding format.

---

### IW44 encoder `encode_iw44_color` profile breakdown (2026-04-17)

**Setup:** 200 iters of `encode_iw44_color` on boy.djvu (192×256), samply @ ~1 ksample/s.

| Function | Self% | Notes |
|----------|-------|-------|
| `PlaneEncoder::encode_slice` | **56.3%** | Main ZP encoding loop. Inlined coefficient scan + ZP dispatch. |
| `ZpEncoder::zemit` | **15.2%** | Carry-propagation buffer flush. NOT inlined (LLVM skips due to slow-path Vec::push size). Hot path is top=0 (nrun++). |
| `forward_wavelet_transform` | **13.5%** | Scalar only — no NEON. 35 bounds-check calls eliminated by get_unchecked but no measured benefit. Lifting sliding-window prevents full auto-vectorization. |
| `encode_iw44_color` (orchestration) | 8.5% | RGB→YCbCr, block gather, overhead. |
| `ZpEncoder::encode_bit` | 4.3% | Fast path (a=z, no shift) — already fast. |

**Key finding:** zemit at 15.2% looks like a target, but `#[inline(always)]` causes +3.1% regression. The encoder is fundamentally I-cache-limited in its hot path — any attempt to expand the inline chain into encode_slice degrades performance. The remaining wins must come from reducing work (NEON wavelet, faster gather/scatter) rather than ZP encoding speed.

---

## Log rules

1. After reverting — **immediately** add a row to "Reverted" with the reason
2. After measuring — update "Baseline metrics" if any number changed by >5%
3. Before starting an experiment — check "Hypotheses" and "Reverted" to avoid duplicates
4. After implementing a hypothesis — move it to "Kept" or "Reverted"
