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

## Baseline metrics (Apple M1 Max, 2026-04-17, after i16 YCbCr arithmetic)

| Benchmark | Result | vs BENCHMARKS.md (v0.4.1) |
|-----------|--------|---------------------------|
| `jb2_decode` | **131.8 µs** | −42% (was 228 µs) |
| `iw44_decode_first_chunk` | **578 µs** | −21% (was 734 µs) |
| `iw44_decode_corpus_color` | **650 µs** | — |
| `iw44_to_rgb_colorbook/sub1_full_decode` | **6.10 ms** | — |
| `iw44_to_rgb_colorbook/sub2_partial_decode` | **1.45 ms** | — |
| `iw44_to_rgb_colorbook/sub4_partial_decode` | **370 µs** | — |
| `jb2_decode_corpus_bilevel` | **421 µs** | — |
| `jb2_encode` | **182 µs** | — |
| `iw44_encode_color` | **2.13 ms** | — |
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
| 2026-04 | IW44 | split `int16x8x2_t curr_pair` into `curr_even`/`curr_odd` in even-pass loop to eliminate "redundant" ld2.8h carry across iterations | sub1 −1.3% (p=0.00) but sub2 +2.1% (p=0.00) — net negative; the "redundant" ld2 is a sequential L1 hit (~free on M1); restructuring hurts LLVM's instr scheduling for sub2; "carry-in-registers" only wins for very long row bodies, not here |
| 2026-04 | IW44 | replace `vmovl_s16×2 + vaddq_s32` with `vaddl_s16` in even-pass lift (saves 8 instr/chunk) | sub1 +1.7% (p=0.00) — `saddl` has 2/cycle throughput on M1 vs 4/cycle for `sxtl`; using a lower-throughput instruction to save instruction count is a net loss; M1's even-pass lift is throughput-bound on `add.4s`/`sxtl`-class instructions (4/cycle units) |
| 2026-04 | IW44 | `srshr5_i32x8`/`srshr4_i32x8` wrappers using `vrshrq_n_s32` to fold bias into rounding shift (save 2 `add.4s` per even/odd lifting iteration) | sub1 +1.2%, sub2 −0.9%, sub4 −0.6% — all within noise (Criterion p=0.00 but tiny magnitude); assembly confirms `srshr.4s #5` for `lifting_even` but `srshr.4s #4` for `predict_inner` is absent (LLVM absorbs it into narrowing path); ~2 M fewer instructions but M1's column pass is not instruction-count-bound at this level; complex unsafe transmute code not justified by zero measurable gain |
| 2026-04 | IW44 | `row_pass_neon_s2_all`: gather active (even physical) columns into temp buffer via `vld2q_s16`, run `row_pass_neon_s1_row` on contiguous buffer, scatter back with 8 × `vst1_lane_s16` per chunk — intended to reduce L1 pressure from 8-rows-at-a-time i32x8 path (36 KB stride span vs 6 KB) | sub2 +8.4%, sub4 +8.8%, sub1 +3.8% — all regressions; `vst1_lane_s16` scatter stores each write a single 2-byte lane to a non-sequential address (8 independent str h per chunk); this produces 8 distinct cache-line writes per 16-element group vs 1 vst2q write in the i32x8 path; M1's prefetcher handles the strided 36 KB working set efficiently; Vec allocation per to_rgb() call also adds heap overhead |

> **Rule:** if you revert something, add a row here with the reason — otherwise it will be tried again.

### → Hypotheses (not yet measured)

| Component | Idea | Expected | Risk |
|-----------|------|----------|------|
| ZP | SIMD decode of multiple symbols in parallel (8-wide) (#183) | large | complex, breaking |
| ZP | branch-free decode_bit via cmov (#179) | ✗ reverted — see log | LPS function call overhead worse than inline |
| IW44 | column_pass SIMD at s=2 (#184) | ✓ kept (attempt 3) — see log | `load8s`/`store8s` with `vld2q_s16`/`vld4q_s16` + scatter-stores within existing `use_simd = s <= 4` body; no extraction, no dispatch overhead |
| JB2 | bit-pack bitmap → smaller memory/cache footprint (#185) | medium | complex |
| render | pre-decode JB2 bitmap on a separate thread (#186) | ✓ kept — see log | −30% cold render |
| ZP | LUT for frequent states (#181) | small | cache pressure |
| IW44 | early-exit in `decode_slice` when ZP exhausted + no ACTIVE blocks (#182) | ✗ reverted — see log | ZP stream is a continuous encoding of all decisions; skipping any call desynchronises state |
| IW44 | horizontal row-pass NEON for s=2: gather/scatter via temp buffer | ✗ reverted — see log | `vst1_lane_s16` scatter produces 8 independent cache-line writes per chunk; worse than i32x8 path which M1 prefetcher handles fine |

---

## Investigations

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

## Log rules

1. After reverting — **immediately** add a row to "Reverted" with the reason
2. After measuring — update "Baseline metrics" if any number changed by >5%
3. Before starting an experiment — check "Hypotheses" and "Reverted" to avoid duplicates
4. After implementing a hypothesis — move it to "Kept" or "Reverted"
