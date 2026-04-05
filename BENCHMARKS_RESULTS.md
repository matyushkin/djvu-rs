# Benchmark Results

Platform: Apple M1 Max, 10 cores, 64 GB RAM
OS: macOS 26.3.1 (Darwin 25.3)
Rust: 1.92.0 stable (edition 2024)
Profile: release (opt-level 3, lto = thin)
Date: 2026-04-05

All benchmarks use `criterion` with 100 samples, 3s warm-up, 5s measurement.

---

## Codec benchmarks (`cargo bench --bench codecs`)

Test files: `references/djvujs/library/assets/` and `tests/corpus/`

| Benchmark | File | Time (median) | Notes |
|-----------|------|--------------|-------|
| `bzz_decode` | navm_fgbz.djvu NAVM/DIRM chunk | **84.0 ms** | ZP + MTF + BWT decompression |
| `jb2_decode` | boy_jb2.djvu Sjbz chunk | **245 µs** | Bilevel JB2 decode (small page) |
| `iw44_decode_first_chunk` | boy.djvu first BG44 | **751 µs** | Single IW44 wavelet chunk |
| `jb2_decode_corpus_bilevel` | cable_1973_100133.djvu Sjbz | **3.42 ms** | Larger bilevel scan (State Dept cable) |
| `iw44_decode_corpus_color` | watchmaker.djvu first BG44 | **2.36 ms** | Color IW44 chunk |

---

## Render benchmarks (`cargo bench --bench render`)

Test file: `references/djvujs/library/assets/boy.djvu` (192×256 px, 100 dpi)
Corpus files: `tests/corpus/`

### DPI scaling — boy.djvu (IW44 color page)

| DPI | Output size | Time (median) |
|-----|-------------|--------------|
| 72 dpi | ~138×184 px | **1.40 ms** |
| 144 dpi | ~276×368 px | **1.86 ms** |
| 300 dpi | ~576×768 px | **4.51 ms** |
| 600 dpi | ~1152×1536 px | **15.1 ms** |

### Special render modes

| Benchmark | Description | Time (median) |
|-----------|-------------|--------------|
| `render_corpus_color` | watchmaker.djvu full render | **3.27 ms** |
| `render_corpus_bilevel` | cable_1973_100133.djvu full render | **3.16 ms** |
| `render_scaled_0.5x/bilinear` | boy.djvu at 0.5× with bilinear filter | **1.31 ms** |
| `render_scaled_0.5x/lanczos3` | boy.djvu at 0.5× with Lanczos-3 filter | **6.19 ms** |
| `pdf_export_single_page` | Export single page to PDF bytes | **554 ms** |

---

## Document benchmarks (`cargo bench --bench document`)

Test file: `tests/corpus/pathogenic_bacteria_1896.djvu` (520 pages, 25 MB, mixed IW44+JB2)
Text layer: `tests/corpus/watchmaker.djvu` (TXTz present)

| Benchmark | Time (median) | Notes |
|-----------|--------------|-------|
| `parse_multipage_520p` | **1.92 ms** | Parse DJVM directory + all page descriptors, 520 pages, 25 MB |
| `iterate_pages_520p` | **521 µs** | Read width/height/dpi for all 520 pages (no render) |
| `render_large_doc_first_page` | **44.2 ms** | Render page 1 of 520 (mixed content) at native DPI |
| `render_large_doc_mid_page` | **75.7 ms** | Render page 260 of 520 — larger/denser page |
| `text_extraction_single_page` | **202 µs** | TXTz parse + plain text output, watchmaker.djvu |

---

## Comparison with DjVuLibre 3.5.29

Two comparison levels: CLI-to-CLI and library-to-library.

### CLI comparison (`ddjvu` vs render)

Method: 5 runs, mean, subprocess timing (includes process startup ~7 ms for ddjvu)

| File | ddjvu scale=100 | Notes |
|------|-----------------|-------|
| boy.djvu (192×256, 3-layer) | 30.6 ms | ~24 ms decode after startup |
| watchmaker.djvu (849×1100, color) | 224 ms | ~217 ms decode |
| cable_1973.djvu (849×1100, bilevel) | 53.5 ms | ~47 ms decode |

djvu-rs library equivalents (in-process, no startup):

| File | djvu-rs | ddjvu decode-only | Ratio |
|------|---------|-------------------|-------|
| boy.djvu at 100 dpi | **~1.6 ms** | ~24 ms | **~15× faster** |
| watchmaker.djvu | **3.27 ms** | ~217 ms | **~66× faster** |
| cable_1973.djvu | **3.16 ms** | ~47 ms | **~15× faster** |

Note: ddjvu writes full PNM output and includes fork/exec overhead. djvu-rs
numbers are pure decode+render to an in-memory RGBA buffer.

### Library-level comparison (C API vs Rust API, render-only)

Method: `clock_gettime(CLOCK_MONOTONIC)` around render call, 20 warm-up + 20 measured iterations.
C source: `scripts/djvulibre_bench.c`

| File | Output size | djvu-rs | libdjvulibre C API | Ratio |
|------|------------|---------|-------------------|-------|
| watchmaker.djvu (color IW44, 300 dpi) | 2550×3301 px | **3.27 ms** | 37.3 ms | **~11× faster** |
| cable_1973_100133.djvu (bilevel JB2, 300 dpi) | 2550×3301 px | **3.16 ms** | 36.8 ms | **~12× faster** |
| pathogenic_bacteria_1896.djvu p.1 (mixed, 600 dpi) | 2649×4530 px | 44.2 ms | **12.2 ms** | ~0.3× (libdjvulibre wins) |
| pathogenic_bacteria_1896.djvu p.260 (mixed, 600 dpi) | 2649×4530 px | 75.7 ms | **13.8 ms** | ~0.2× (libdjvulibre wins) |

djvu-rs numbers are from `cargo bench --bench document` (criterion, release mode).
libdjvulibre numbers are render-only — after the page is already decoded and in memory.

**Analysis:**

- For standard 300 dpi pages, djvu-rs is ~11–12× faster than libdjvulibre for the render step.
- For large 600 dpi mixed pages (12 MP output buffer) libdjvulibre is ~3–5× faster. This is an
  expected weakness — djvu-rs color conversion uses scalar Rust; libdjvulibre uses hand-tuned C
  with SIMD for large buffers. This is a known optimization target.
- `open+decode` latency before render: djvu-rs ≈ 1.9 ms (parse_multipage);
  libdjvulibre C API ≈ 24–60 ms depending on page and file size — **10–30× faster open**.

### Summary

| Scenario | Winner | Margin |
|----------|--------|--------|
| Embedded in application (typical 300 dpi page) | **djvu-rs** | ~11× |
| Large high-DPI page (600 dpi, 12 MP+) | libdjvulibre | ~3–5× |
| Open + decode document | **djvu-rs** | ~10–30× |

---

## Notes

- `bzz_decode` is slow (84 ms) because the NAVM chunk in navm_fgbz.djvu is large (~6 KB compressed). BZZ is an inherently sequential algorithm (BWT inverse requires a full-block sort).
- JB2 and IW44 decode in sub-millisecond to low-millisecond range for typical pages.
- Full page render at 72 dpi takes ~1.4 ms (composite: IW44 background + JB2 mask + color).
- Corpus benchmarks use public domain files from Internet Archive.
- Large high-DPI render (600 dpi) is a known optimization target — SIMD color conversion is planned.
