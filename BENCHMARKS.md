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

This file is the broad baseline table. Smaller issue-driven experiments and
kept/reverted decisions are recorded in [`PERF_EXPERIMENTS.md`](PERF_EXPERIMENTS.md),
including recent async lazy loading and x86-64-v3 AVX2 validation results.

The latest full local Criterion run is summarized in
[`BENCHMARKS_RESULTS.md`](BENCHMARKS_RESULTS.md) (2026-05-04, macOS arm64,
Rust 1.92.0). Selected results from that run:

| Benchmark | Time |
|-----------|-----:|
| `render_page/dpi/72` | 216 µs |
| `render_page/dpi/144` | 895 µs |
| `render_page/dpi/300` | 3.42 ms |
| `render_colorbook` | 7.29 ms |
| `render_colorbook_cold` | 17.9 ms |
| `render_corpus_color` | 71.2 ms |
| `render_corpus_bilevel` | 70.4 ms |
| `jb2_decode` | 134 µs |
| `iw44_decode_first_chunk` | 599 µs |
| `iw44_decode_corpus_color` | 671 µs |
| `parse_multipage_520p` | 2.27 ms |
| `render_large_doc_first_page` | 12.8 ms |

## Contributing results

To add results for a new platform, run:

```sh
cargo bench 2>&1 | tee bench_output.txt
```

Then open a PR updating this file with the new column. Please include CPU model, OS, and Rust version.

---

## Multi-platform comparison

Historical key benchmarks across platforms (Criterion mean, release profile,
v0.4.1 tag). Do not compare these directly with the latest local results above
without checking benchmark definitions; several render benches now measure
different corpus paths or output sizes.

| Benchmark | Apple M1 Max | macOS CI (M-series) | x86_64 Linux (CI) |
|-----------|-------------|---------------------|-------------------|
| `render_page/dpi/72` | **1.21 ms** | 1.41 ms | 2.26 ms |
| `render_page/dpi/144` | **1.74 ms** | 2.02 ms | 3.62 ms |
| `render_page/dpi/300` | **4.02 ms** | 5.08 ms | 10.04 ms |
| `render_scaled_0.5x/bilinear` | **1.17 ms** | 1.39 ms | 1.99 ms |
| `render_scaled_0.5x/lanczos3` | **5.68 ms** | 6.34 ms | 12.40 ms |
| `render_corpus_color` | **3.15 ms** | 3.70 ms | 4.93 ms |
| `render_corpus_bilevel` | **3.12 ms** | 3.47 ms | 4.91 ms |
| `jb2_decode` | **228 µs** | 232 µs | 422 µs |
| `iw44_decode_first_chunk` | **734 µs** | 739 µs | 1.10 ms |
| `bzz_decode` | **82.6 ms** | 86.3 ms | 118.4 ms |
| `pdf_export_single_page` | — | **625 ms** | 1 122 ms |

macOS CI runner: `macos-latest` (GitHub-hosted Apple Silicon).
Linux CI runner: `ubuntu-latest` (GitHub-hosted x86_64).

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

## Detailed results — Linux x86_64 (CI, v0.4.1)

### Platform

| | |
|---|---|
| **CPU** | GitHub-hosted `ubuntu-latest` runner (x86_64) |
| **OS** | Ubuntu 24.04 |
| **Rust** | stable (edition 2024) |
| **Profile** | release |
| **Date** | 2026-04-05 |

### Codec benchmarks

| Benchmark | Time (mean) |
|-----------|------------|
| `bzz_decode` | 118.4 ms |
| `jb2_decode` | 422 µs |
| `jb2_decode_corpus_bilevel` | 4.65 ms |
| `iw44_decode_first_chunk` | 1.10 ms |
| `iw44_decode_corpus_color` | 4.39 ms |

### Render benchmarks

| Benchmark | Time (mean) |
|-----------|------------|
| `render_page/dpi/72` | 2.26 ms |
| `render_page/dpi/144` | 3.62 ms |
| `render_page/dpi/300` | 10.04 ms |
| `render_scaled_0.5x/bilinear` | 1.99 ms |
| `render_scaled_0.5x/lanczos3` | 12.40 ms |
| `render_coarse` | 2.63 ms |
| `render_corpus_color` | 4.93 ms |
| `render_corpus_bilevel` | 4.91 ms |
| `pdf_export_single_page` | 1 122 ms |

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
case where libdjvulibre's hand-tuned SIMD color conversion dominates. YCbCr→RGB SIMD
was implemented in v0.4.0 ([#1](https://github.com/matyushkin/djvu-rs/issues/1)); further gains
at 600 dpi require SIMD in the wavelet transform itself.

---

## Comparison with other DjVu libraries

### Library landscape

| Library | Language | License | Actively maintained | Notes |
|---------|----------|---------|---------------------|-------|
| **djvu-rs** | Rust | MIT | ✓ | This crate. Pure Rust, no_std codec layer |
| [DjVuLibre](https://djvu.sourceforge.net/) | C++ | GPL-2.0 | ✓ | Reference implementation, 25+ years old |
| [python-djvulibre](https://jwilk.net/software/python-djvulibre) | Python (C bindings) | GPL-2.0 | △ | Thin wrapper around DjVuLibre; performance = DjVuLibre |
| [djvu.js](https://github.com/RussCoder/djvujs) | JavaScript | MIT | △ | Browser-first viewer; no server-side decode |
| [LizardTech DjVu SDK](https://www.celartem.com/) | C++ | Proprietary | ? | Commercial, rarely used outside enterprise |

No other pure-Rust DjVu decoder exists as of v0.4.

### Numerical comparison

Measured numbers are available only for **djvu-rs vs DjVuLibre** (see section above).
python-djvulibre delegates all decode work to the underlying DjVuLibre C++ library,
so its performance equals DjVuLibre's.

djvu.js operates in a browser context (WASM/JS) and is not designed for server-side
batch processing. A direct comparison is not meaningful.

Contributions with measurements for other libraries are welcome — see **Contributing results** above.

### Feature comparison

| Feature | djvu-rs | DjVuLibre | python-djvulibre | djvu.js |
|---------|---------|-----------|-----------------|---------|
| Decode IW44 | ✓ | ✓ | ✓ | ✓ |
| Decode JB2 | ✓ | ✓ | ✓ | ✓ |
| Text layer | ✓ | ✓ | ✓ | ✓ |
| PDF export | ✓ | — | — | — |
| TIFF export | ✓ | ✓ | ✓ | — |
| Async render | ✓ | — | — | — |
| no_std | ✓ | — | — | — |
| License | MIT | GPL-2.0 | GPL-2.0 | MIT |
| Rust crate | ✓ | — | — | — |

---

## Encoder quality (vs DjVuLibre)

Quality benchmarks compare djvu-rs's encoder output size to the original DjVu
file (typically produced by `cjb2`/`c44` from DjVuLibre) on `tests/corpus/*`.
Run with `cargo run --release --example encode_quality_jb2 -- tests/corpus/*.djvu`.

### JB2 — Sjbz payload size (553 pages across 4 books)

| Encoder | Total bytes | bpp | Ratio vs cjb2 | Round-trip |
|---------|-------------|-----|---------------|------------|
| `cjb2` (original) | 25 032 792 | 0.0288 | 1.00× | — |
| `encode_jb2` (tiled direct) | 42 809 783 | 0.0492 | **1.71×** | 553 / 553 ok |
| `encode_jb2_dict` (CC + rec 1+7) | 35 301 664 | 0.0406 | **1.41×** | 483 / 553 ok |

- `encode_jb2` (#198): direct-blit, tiled into ≤ 1024×1024 records to stay
  under the decoder's 1 MP per-symbol cap. 100% round-trip across all corpus
  pages including 4267×6853 (29 MP) scans.
- `encode_jb2_dict` (#188 phase 1): connected-component extraction + exact-match
  symbol dictionary. **1.41×** total ratio is mostly recovered already; the
  remaining gap closes with refinement matching (#188 phase 2/3) and shared
  Djbz across pages (#194). Round-trip fails on 70 dense scans whose CC
  extraction yields a single component > 1 MP (large halftone / contiguous
  artwork region) — the dict path needs to fall back to tiled direct-blit
  for such CCs (tracked under #188 phase 2).

Per-book ratios:

| Corpus | Pages | dict bytes | orig bytes | dict ratio | direct ratio |
|--------|-------|------------|------------|------------|--------------|
| `cable_1973_100133.djvu` | 2 | 10 082 | 8 020 | 1.257× | 3.886× |
| `conquete_paix.djvu` | 22 | 54 919 | 52 007 | 1.056× | 1.906× |
| `pathogenic_bacteria_1896.djvu` | 517 | 35 092 640 | 24 849 842 | 1.412× | 1.696× |
| `watchmaker.djvu` | 12 | 144 023 | 122 923 | 1.172× | 4.349× |

### IW44 — BG44 payload size + PSNR (23 colour pages, 4 books)

Re-encoded via `cargo run --release --example encode_quality_iw44 --
tests/corpus/*.djvu`. Reference for PSNR is djvu-rs's own decode of the
original BG44 (the encoder is the unit under test).

| Metric | Value |
|--------|-------|
| Total pages | 23 |
| Total orig BG44 size | 1 607 509 B |
| Total djvu-rs BG44 size | 1 834 621 B (1.14× orig) |
| Avg PSNR (luma) | 19.52 dB |
| Min PSNR (luma) | 9.75 dB |
| `watchmaker.djvu` pages (small bg) | 45–47 dB ✓ |
| `conquete_paix.djvu` pages | 9–17 dB ✗ |

`watchmaker.djvu` results are near-perfect (45+ dB on a near-empty bg).
`conquete_paix.djvu` results are catastrophic — sub-20 dB indicates the
encoder loses substantial structure on those pages. Worth opening a follow-up
to investigate; the harness exists now to track it.

Apple M1 Max, 2026-04-26.

---

## Notes

- `bzz_decode` is slow (82 ms) because the NAVM chunk in navm_fgbz.djvu is large (~6 KB compressed). BZZ is inherently sequential (BWT inverse requires a full-block sort).
- JB2 and IW44 decode in sub-millisecond to low-millisecond range for typical pages.
- Full page render at 72 dpi takes ~1.2 ms (composite: IW44 background + JB2 mask + color conversion).
- Corpus benchmarks use public domain files from Internet Archive.
- Large high-DPI render (600 dpi): SIMD YCbCr→RGB was added in v0.4.0. Further gains require SIMD in the IW44 wavelet transform.
- Lanczos3 is available via `RenderOptions { resampling: Resampling::Lanczos3, .. }` for higher-quality downscaling at the cost of ~5× render time.
- YCbCr→RGB conversion uses `wide::i32x8` SIMD (8 pixels per iteration). On the M1 Max the wavelet transform dominates; the SIMD path shows most benefit on large high-DPI pages (600 dpi, ≥ 12 MP) where color conversion is a larger fraction of total time.
