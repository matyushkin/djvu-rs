# djvu-rs Benchmark Results

## How to reproduce

```sh
cargo bench                          # all benchmarks
cargo bench --bench codecs           # codec decode throughput
cargo bench --bench render           # render pipeline
cargo bench --bench document         # document-level operations
```

Results require `tests/corpus/` files for document and corpus benchmarks. See `CONTRIBUTING.md` for how to obtain them.

CI benchmarks run automatically on every release tag via [`.github/workflows/bench.yml`](.github/workflows/bench.yml).
Full Criterion HTML reports are uploaded as workflow artifacts (90-day retention).

## Contributing results

To add results for a new platform, run:

```sh
cargo bench 2>&1 | tee bench_output.txt
```

Then open a PR updating this file with the new column. Please include CPU model, OS, and Rust version.

---

## Multi-platform comparison

Key render benchmarks across platforms (time = Criterion median, release profile).

| Benchmark | Apple M1 Max | x86_64 Linux (CI) |
|-----------|-------------|-------------------|
| `render_page/dpi/72` (boy.djvu) | **1.21 ms** | *(see CI artifacts)* |
| `render_page/dpi/144` (boy.djvu) | **1.74 ms** | *(see CI artifacts)* |
| `render_page/dpi/300` (boy.djvu) | **4.02 ms** | *(see CI artifacts)* |
| `render_scaled_0.5x/bilinear` | **1.17 ms** | *(see CI artifacts)* |
| `render_scaled_0.5x/lanczos3` | **5.68 ms** | *(see CI artifacts)* |
| `render_corpus_color` (watchmaker.djvu) | **3.15 ms** | *(see CI artifacts)* |
| `render_corpus_bilevel` (cable_1973.djvu) | **3.12 ms** | *(see CI artifacts)* |
| `jb2_decode` (boy_jb2.djvu) | **228 µs** | *(see CI artifacts)* |
| `iw44_decode_first_chunk` (boy.djvu) | **734 µs** | *(see CI artifacts)* |
| `bzz_decode` (navm_fgbz.djvu) | **82.6 ms** | *(see CI artifacts)* |

Linux x86_64 results are collected automatically on every release tag and available as GitHub Actions artifacts.

---

## Detailed results — Apple M1 Max

### Platform

| | |
|---|---|
| **CPU** | Apple M1 Max, 10 cores |
| **RAM** | 64 GB |
| **OS** | macOS 26.3.1 (Darwin 25.3) |
| **Rust** | 1.92.0 stable (edition 2024) |
| **Profile** | release (opt-level 3, lto = thin) |
| **Date** | 2026-04-04 |

All benchmarks use Criterion with 100 samples, 3 s warm-up, 5 s measurement.

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

### Resampling — boy.djvu at 0.5× scale (90×120 px output)

| Resampling | Time (median) | Notes |
|------------|--------------|-------|
| `Bilinear` | **1.17 ms** | Built-in bilinear compositor in IW44 |
| `Lanczos3` | **5.68 ms** | Native render + two-pass separable 6-tap kernel |

Lanczos3 is ~5× slower at 0.5× but produces visibly sharper output. For thumbnails, Bilinear is the default.

### Special render modes

| Benchmark | Description | Time (median) |
|-----------|-------------|--------------|
| `render_coarse` | First BG44 chunk only (blurry preview) | **1.36 ms** |
| `render_corpus_color` | watchmaker.djvu full render (native res) | **3.15 ms** |
| `render_corpus_bilevel` | cable_1973_100133.djvu full render | **3.12 ms** |
| `pdf_export_single_page` | DjVu→PDF pipeline (render + DCTDecode JPEG) | **see below** |

> **pdf_export** requires `tests/corpus/watchmaker.djvu`. Run `cargo bench --bench render -- pdf_export` to measure.

---

## Document benchmarks (`cargo bench --bench document`)

Test file: `tests/corpus/pathogenic_bacteria_1896.djvu` (520 pages, 25 MB, mixed IW44+JB2)

| Benchmark | Time (median) | Notes |
|-----------|--------------|-------|
| `parse_multipage_520p` | **912 µs** | Parse DJVM directory + all page descriptors |
| `iterate_pages_520p` | **482 µs** | Read width/height/dpi for all 520 pages (no render) |
| `render_large_doc_first_page` | **30.5 ms** | Render page 1 at native DPI (mixed content) |
| `render_large_doc_mid_page` | **61.8 ms** | Render page 260 of 520 |
| `text_extraction_single_page` | — | watchmaker.djvu TXTz extraction |

---

## Comparison with DjVuLibre 3.5.29

### CLI comparison (`ddjvu` vs `djvu render`)

Tool: `ddjvu -format=ppm -page=1` vs `djvu render -o out.png`
Method: `hyperfine --warmup 3 --runs 10`

| File | djvu-rs CLI | ddjvu CLI | Ratio |
|------|------------|-----------|-------|
| watchmaker.djvu (color IW44) | ~73 ms | 145.2 ms | **~2× faster** |
| cable_1973_100133.djvu (bilevel JB2) | ~73 ms | 103.0 ms | **~1.4× faster** |
| pathogenic_bacteria_1896.djvu p.1 | ~73 ms | 248.3 ms | **~3.4× faster** |

Both CLIs include process startup. djvu-rs outputs PNG (lossless); ddjvu outputs PPM (uncompressed).
PNG encoding adds ~30 ms overhead for large pages, so the raw decode advantage is larger than shown.

### Library-level comparison (C API vs Rust API)

Method: `clock_gettime(CLOCK_MONOTONIC)` around render call, 20 warm-up + 20 measured iterations.
C source: `scripts/djvulibre_bench.c`

| File | Output size | djvu-rs | libdjvulibre C API | Ratio |
|------|------------|---------|-------------------|-------|
| watchmaker.djvu (300 dpi) | 2550×3301 px | **3.15 ms** | 35.4 ms | **~11× faster** |
| cable_1973_100133.djvu (300 dpi) | 2550×3301 px | **3.12 ms** | 34.7 ms | **~11× faster** |
| pathogenic_bacteria_1896.djvu p.1 (600 dpi) | 2649×4530 px | 30.5 ms | **11.1 ms** | ~0.4× *(libdjvulibre wins)* |

### Summary

| Scenario | Winner | Margin |
|----------|--------|--------|
| Standard 300 dpi page (embedded) | **djvu-rs** | ~11× |
| Large 600 dpi page (12 MP+) | libdjvulibre | ~3× |
| CLI usage (process startup included) | **djvu-rs** | ~2–3× |
| Parse + open document | **djvu-rs** | ~25–50× |

**Analysis:** djvu-rs wins decisively at typical resolutions. The only deficit is the 600 dpi large-page
case where libdjvulibre's hand-tuned SIMD color conversion dominates. This is a known optimization
target tracked in [Issue #1](https://github.com/matyushkin/djvu-rs/issues/1).

---

## Notes

- `bzz_decode` is slow (82 ms) because the NAVM chunk in navm_fgbz.djvu is large (~6 KB compressed). BZZ is inherently sequential (BWT inverse requires a full-block sort).
- JB2 and IW44 decode in sub-millisecond to low-millisecond range for typical pages.
- Full page render at 72 dpi takes ~1.2 ms (composite: IW44 background + JB2 mask + color conversion).
- Corpus benchmarks use public domain files from Internet Archive.
- Large high-DPI render (600 dpi) is a known optimization target — SIMD color conversion is planned (Issue #1).
- Lanczos3 is available via `RenderOptions { resampling: Resampling::Lanczos3, .. }` for higher-quality downscaling at the cost of ~5× render time.
- YCbCr→RGB conversion uses `wide::i32x8` SIMD (8 pixels per iteration). On the M1 Max the wavelet transform dominates; the SIMD path shows most benefit on large high-DPI pages (600 dpi, ≥ 12 MP) where color conversion is a larger fraction of total time.
