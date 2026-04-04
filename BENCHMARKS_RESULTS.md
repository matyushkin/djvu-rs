# Benchmark Results

Platform: macOS 15 (Darwin 25.3), Apple Silicon (M-series)
Rust: stable (edition 2024)
Profile: release (opt-level 3, lto = thin)
Date: 2026-04-04

All benchmarks use `criterion` with 100 samples, 3s warm-up, 5s measurement.

---

## Codec benchmarks (`cargo bench --bench codecs`)

Test files: `references/djvujs/library/assets/` and `tests/corpus/`

| Benchmark | File | Time (median) | Notes |
|-----------|------|--------------|-------|
| `bzz_decode` | navm_fgbz.djvu NAVM/DIRM chunk | **82.6 ms** | ZP + MTF + BWT decompression |
| `jb2_decode` | boy_jb2.djvu Sjbz chunk | **228 µs** | Bilevel JB2 decode (small page) |
| `iw44_decode_first_chunk` | boy.djvu first BG44 | **734 µs** | Single IW44 wavelet chunk |
| `jb2_decode_corpus_bilevel` | cable_1973_100133.djvu Sjbz | **3.30 ms** | Larger bilevel scan (State Dept cable) |
| `iw44_decode_corpus_color` | watchmaker.djvu first BG44 | **2.33 ms** | Color IW44 chunk |

---

## Render benchmarks (`cargo bench --bench render`)

Test file: `references/djvujs/library/assets/boy.djvu` (181×240 px, 100 dpi)
Corpus files: `tests/corpus/`

### DPI scaling — boy.djvu (IW44 color page)

| DPI | Output size | Time (median) |
|-----|-------------|--------------|
| 72 dpi | ~130×173 px | **1.21 ms** |
| 144 dpi | ~260×346 px | **1.74 ms** |
| 300 dpi | ~543×720 px | **4.02 ms** |

### Special render modes

| Benchmark | Description | Time (median) |
|-----------|-------------|--------------|
| `render_coarse` | First BG44 chunk only (blurry preview) | **1.36 ms** |
| `render_corpus_color` | watchmaker.djvu full render | **3.15 ms** |
| `render_corpus_bilevel` | cable_1973_100133.djvu full render | **3.12 ms** |

---

## Comparison with DjVuLibre 3.5.29

Tool: `ddjvu -format=ppm -page=1` (CLI utility, process-per-call)
Method: `hyperfine --warmup 3 --runs 10`
Platform: same machine (Apple M-series, macOS 15)

| File | djvu-rs render | ddjvu CLI | Ratio |
|------|---------------|-----------|-------|
| watchmaker.djvu (color IW44) | **3.1 ms** | 145.2 ms | ~47× faster |
| cable_1973_100133.djvu (bilevel JB2) | **3.1 ms** | 103.0 ms | ~33× faster |

**Caveat:** `ddjvu` is a subprocess — timings include fork/exec overhead (~50–80 ms on macOS) and PPM file write. The comparison reflects real-world CLI usage, not pure decode time. A library-level comparison via `libdjvulibre` C API would be more apples-to-apples for the decode kernel itself.

Even so, djvu-rs embedded in a Rust application avoids all subprocess overhead entirely.

---

## Notes

- `bzz_decode` is slow (82 ms) because the NAVM chunk in navm_fgbz.djvu is large (~6 KB compressed). BZZ is an inherently sequential algorithm (BWT inverse requires a full-block sort).
- JB2 and IW44 decode in sub-millisecond to low-millisecond range for typical pages.
- Full page render at 72 dpi takes ~1.2 ms (composite: IW44 background + JB2 mask + color).
- Corpus benchmarks use public domain files from Internet Archive.
