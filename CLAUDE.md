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

## Baseline metrics (Apple M1 Max, 2026-04-15, after ZP u16→u32)

| Benchmark | Result | vs BENCHMARKS.md (v0.4.1) |
|-----------|--------|---------------------------|
| `jb2_decode` | **131.8 µs** | −42% (was 228 µs) |
| `iw44_decode_first_chunk` | **725 µs** | −1.2% (was 734 µs) |
| `iw44_decode_corpus_color` | **2.30 ms** | — |
| `jb2_decode_corpus_bilevel` | **421 µs** | — |
| `jb2_encode` | **182 µs** | — |
| `iw44_encode_color` | **2.16 ms** | — |
| `render_page/dpi/72` | 1.21 ms | (from BENCHMARKS.md) |
| `render_page/dpi/300` | 4.02 ms | (from BENCHMARKS.md) |

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

### ✗ Reverted

| Date | Component | What was tried | Why reverted |
|------|-----------|----------------|--------------|
| 2026-04 | render | bilevel composite fast path (#165) | regression — restored in #169 |
| 2026-04 | ZP | `#[cold] #[inline(never)]` for LPS branch + cmov-friendly context update | iw44 +4%, jb2_encode +2% — function call overhead > I-cache gain; LPS fires 10-15% of calls, too frequent for out-of-line |
| 2026-04 | IW44 | early-exit `decode_slice` when `zp.is_exhausted() && bbstate & ACTIVE == 0` (#182) | 99.2% pixel mismatch — `is_exhausted()` fires mid-stream (not end-of-decisions); skipping decode_bit corrupts ZP arithmetic state for all subsequent calls; the ZP stream is a continuous encoding of ALL block decisions; can't skip any call without desynchronising |

> **Rule:** if you revert something, add a row here with the reason — otherwise it will be tried again.

### → Hypotheses (not yet measured)

| Component | Idea | Expected | Risk |
|-----------|------|----------|------|
| ZP | SIMD decode of multiple symbols in parallel (8-wide) (#183) | large | complex, breaking |
| ZP | branch-free decode_bit via cmov (#179) | ✗ reverted — see log | LPS function call overhead worse than inline |
| IW44 | column_pass SIMD at s=2 (stride-2 gather, follow-up to #180) (#184) | small | needs load8_strided (vld2q_s16 on NEON) |
| JB2 | bit-pack bitmap → smaller memory/cache footprint (#185) | medium | complex |
| render | pre-decode JB2 bitmap on a separate thread (#186) | medium | requires Arc |
| ZP | LUT for frequent states (#181) | small | cache pressure |
| IW44 | early-exit in `decode_slice` when ZP exhausted + no ACTIVE blocks (#182) | ✗ reverted — see log | ZP stream is a continuous encoding of all decisions; skipping any call desynchronises state |

---

## Investigations

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
