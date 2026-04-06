# Benchmark Results

Platform: Apple M1 Max, 10 cores, 64 GB RAM
OS: macOS 26.3.1 (Darwin 25.3)
Rust: 1.92.0 stable (edition 2024)
Profile: release (opt-level 3, lto = thin)
Date: 2026-04-06 (updated for v0.5.3 / Issue #87)

All benchmarks use `criterion` with 100 samples, 3s warm-up, 5s measurement.

---

## Codec benchmarks (`cargo bench --bench codecs`)

Test files: `references/djvujs/library/assets/` and `tests/corpus/`

| Benchmark | File | Time (median) | Notes |
|-----------|------|--------------|-------|
| `bzz_decode` | navm_fgbz.djvu NAVM/DIRM chunk | **84.0 ms** | ZP + MTF + BWT decompression |
| `jb2_decode` | boy_jb2.djvu Sjbz chunk | **189 µs** | Bilevel JB2 decode (small page, was 245 µs) |
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
| `render_corpus_color` | watchmaker.djvu full render | **3.15 ms** |
| `render_corpus_bilevel` | cable_1973_100133.djvu full render | **3.15 ms** |
| `render_scaled_0.5x/bilinear` | boy.djvu at 0.5× with bilinear filter | **1.31 ms** |
| `render_scaled_0.5x/lanczos3` | boy.djvu at 0.5× with Lanczos-3 filter | **6.19 ms** |
| `pdf_export_single_page` | Export single page to PDF bytes | **554 ms** |

---

## Document benchmarks (`cargo bench --bench document`)

Test file: `tests/corpus/pathogenic_bacteria_1896.djvu` (520 pages, 25 MB, bilevel JB2 at 600 dpi)
Text layer: `tests/corpus/watchmaker.djvu` (TXTz present)

| Benchmark | Time (median) | Notes |
|-----------|--------------|-------|
| `parse_multipage_520p` | **1.92 ms** | Parse DJVM directory + all page descriptors, 520 pages, 25 MB |
| `iterate_pages_520p` | **521 µs** | Read width/height/dpi for all 520 pages (no render) |
| `render_large_doc_first_page` | **10.5 ms** | Render page 1 of 520 at native 600 dpi (was 42.7 ms) |
| `render_large_doc_mid_page` | **36.2 ms** | Render page 260 of 520 — dense text, large JB2 bitstream (was 75.8 ms) |
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
| watchmaker.djvu | **3.15 ms** | ~217 ms | **~69× faster** |
| cable_1973.djvu | **3.15 ms** | ~47 ms | **~15× faster** |

Note: ddjvu writes full PNM output and includes fork/exec overhead. djvu-rs
numbers are pure decode+render to an in-memory RGBA buffer.

### Library-level comparison (C API vs Rust API, render-only)

Method: `clock_gettime(CLOCK_MONOTONIC)` around render call, 20 warm-up + 20 measured iterations.
C source: `scripts/djvulibre_bench.c`

| File | Output size | djvu-rs | libdjvulibre C API | Ratio |
|------|------------|---------|-------------------|-------|
| watchmaker.djvu (color IW44, 300 dpi) | 2550×3301 px | **3.15 ms** | 37.3 ms | **~12× faster** |
| cable_1973_100133.djvu (bilevel JB2, 300 dpi) | 2550×3301 px | **3.15 ms** | 36.8 ms | **~12× faster** |
| pathogenic_bacteria_1896.djvu p.1 (bilevel JB2, 600 dpi) | 2649×4530 px | **10.5 ms** | **12.2 ms** | ~1.16× (djvu-rs ~16% faster) |
| pathogenic_bacteria_1896.djvu p.260 (bilevel JB2, 600 dpi) | 2649×4530 px | **36.2 ms** | **13.8 ms** | ~0.38× (libdjvulibre wins — large JB2 bitstream) |

djvu-rs numbers are from `cargo bench --bench document` (criterion, release mode, `--features parallel`).
libdjvulibre numbers are render-only — after the page is already decoded and in memory.

**Analysis:**

- For standard 300 dpi pages, djvu-rs is ~12× faster than libdjvulibre for the render step.
- For a sparse 600 dpi bilevel page (p.1, 11 KB JB2), djvu-rs is now **faster** than libdjvulibre
  (10.5 ms vs 12.2 ms, ~16% faster) — was 3.5× slower before v0.5.2. Improvements: bulk
  `Pixmap::new` fill (v0.5.2), shared dict cache + inner-loop `split_at_mut` (v0.5.3).
- For a dense 600 dpi bilevel page (p.260, 65 KB JB2), djvu-rs is 2.6× slower (36.2 ms vs 13.8 ms).
  The ZP arithmetic decoder is inherently sequential; the caching improvement cut this from 3.2×
  slower to 2.6×. Further gains require a faster ZP implementation or SIMD acceleration.
- `open+decode` latency before render: djvu-rs ≈ 1.9 ms (parse_multipage);
  libdjvulibre C API ≈ 24–60 ms depending on page and file size — **10–30× faster open**.

### Summary

| Scenario | Winner | Margin |
|----------|--------|--------|
| Embedded in application (typical 300 dpi page) | **djvu-rs** | ~12× |
| Sparse 600 dpi bilevel page (small JB2) | **djvu-rs** | ~1.2× faster |
| Dense 600 dpi bilevel page (large JB2) | libdjvulibre | ~2.6× |
| Open + decode document | **djvu-rs** | ~10–30× |

---

## Notes

- `bzz_decode` is slow (84 ms) because the NAVM chunk in navm_fgbz.djvu is large (~6 KB compressed). BZZ is an inherently sequential algorithm (BWT inverse requires a full-block sort).
- JB2 and IW44 decode in sub-millisecond to low-millisecond range for typical pages.
- Full page render at 72 dpi takes ~1.4 ms (composite: IW44 background + JB2 mask + color).
- Corpus benchmarks use public domain files from Internet Archive.
- `render_large_doc_first_page` improved from 42.7 → 14.5 ms (-66%) in v0.5.2:
  - `Pixmap::new` used a per-pixel push loop; replaced with `vec![fill; n]` / `slice::repeat` (-18 ms)
  - `composite_bilevel` now uses row-slice writes instead of per-pixel `(y*w+x)*4` multiply
  - `apply_gamma` skipped for pure bilevel output (values are always 0 or 255)
  - Parallel row processing via rayon (`--features parallel`)
- Further improved in v0.5.3 (Issue #87): 14.5 → 10.5 ms (-27.5%), 43.9 → 36.2 ms (-17.5%):
  - Shared JB2 symbol dictionary cached via `RwLock<HashMap<usize, Arc<JB2Dict>>>` — avoids
    re-decoding the Djbz chunk on every `decode_mask()` call (520-page doc: dictionary decoded once)
  - `decode_bitmap_direct` inner loop: `split_at_mut` provides zero-copy look-ahead row access,
    eliminating per-pixel `row * width` multiply and 4-comparison bounds checks
