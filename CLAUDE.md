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

> **Rule:** if you revert something, add a row here with the reason — otherwise it will be tried again.

### → Hypotheses (not yet measured)

| Component | Idea | Expected | Risk |
|-----------|------|----------|------|
| ZP | SIMD decode of multiple symbols in parallel (8-wide) | large | complex, breaking |
| ZP | branch-free decode_bit via cmov (#179) | ✗ reverted — see log | LPS function call overhead worse than inline |
| IW44 | column_pass SIMD at s=2 (stride-2 gather, follow-up to #180) | small | needs load8_strided (vld2q_s16 on NEON) |
| JB2 | bit-pack bitmap → smaller memory/cache footprint | medium | complex |
| render | pre-decode JB2 bitmap on a separate thread | medium | requires Arc |
| ZP | LUT for frequent states (#181) | small | cache pressure |

---

## Log rules

1. After reverting — **immediately** add a row to "Reverted" with the reason
2. After measuring — update "Baseline metrics" if any number changed by >5%
3. Before starting an experiment — check "Hypotheses" and "Reverted" to avoid duplicates
4. After implementing a hypothesis — move it to "Kept" or "Reverted"
