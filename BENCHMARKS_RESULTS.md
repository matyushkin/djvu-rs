# Benchmark Results

Platform: Apple M1 Max, 10 cores, 64 GB RAM
OS: macOS 26.3.1 (Darwin 25.3)
Rust: 1.92.0 stable (edition 2024)
Profile: release (opt-level 3, lto = thin)
Date: 2026-04-13

All Criterion benchmarks use 100 samples, 3 s warm-up, 5 s measurement.

---

## Codec benchmarks (`cargo bench --bench codecs`)

Test files: `references/djvujs/library/assets/` and `tests/corpus/`

| Benchmark | File | Payload | Time (median) | Notes |
|-----------|------|---------|--------------|-------|
| `bzz_decode` | navm_fgbz.djvu NAVM | 89 bytes | **77.0 ns** | ZP + MTF + BWT; tiny NAVM chunk |
| `jb2_decode` | boy_jb2.djvu Sjbz | — | **183 µs** | Bilevel JB2 decode, small page |
| `iw44_decode_first_chunk` | boy.djvu BG44 | — | **751 µs** | Single IW44 wavelet chunk |
| `jb2_decode_corpus_bilevel` | cable_1973_100133.djvu Sjbz | — | **3.36 ms** | Larger bilevel scan (State Dept cable) |
| `iw44_decode_corpus_color` | watchmaker.djvu first BG44 | — | **2.34 ms** | Color IW44 chunk |

Note: `bzz_decode` measures the 89-byte NAVM payload in navm_fgbz.djvu. BZZ performance
scales with payload size (BWT requires a full-block sort); large payloads (e.g. 6 KB DIRM
in a 520-page document) take ~1–5 ms.

---

## Render benchmarks (`cargo bench --bench render`)

Test file: `references/djvujs/library/assets/boy.djvu` (192×256 px, 100 dpi)
Corpus files: `tests/corpus/`, colorbook: `references/djvujs/library/assets/colorbook.djvu`

### DPI scaling — boy.djvu (IW44 color page, 192×256 native)

| DPI | Approx output size | Time (median) |
|-----|--------------------|--------------|
| 72 dpi | ~138×184 px | **580 µs** |
| 144 dpi | ~276×368 px | **951 µs** |
| 300 dpi | ~576×768 px | **3.27 ms** |
| 600 dpi | ~1152×1536 px | **12.5 ms** |

### Full-resolution corpus render

| Benchmark | File | Native size | Time (median) |
|-----------|------|-------------|--------------|
| `render_coarse` | boy.djvu | 192×256 | **1.34 ms** |
| `render_colorbook` | colorbook.djvu | 2260×3669 (400 dpi) | **34.5 ms** |
| `render_corpus_color` | watchmaker.djvu | 2550×3301 | **73.9 ms** |
| `render_corpus_bilevel` | cable_1973_100133.djvu | 2550×3301 | **69.1 ms** |
| `pdf_export_single_page` | watchmaker.djvu | — | **1.88 s** |

Note: djvu-rs always performs a **full IW44 decode** before scaling to the target
output size. For downscaled output (e.g. 150 dpi from a 400 dpi source), DjVuLibre
uses **partial IW44 band decode** and is significantly faster (see comparison below).
Progressive IW44 decode is a planned optimization.

---

## Document benchmarks (`cargo bench --bench document`)

Test file: `tests/corpus/pathogenic_bacteria_1896.djvu` (520 pages, 25 MB, bilevel JB2 at 600 dpi)
Text layer: `tests/corpus/watchmaker.djvu` (TXTz present)

| Benchmark | Time (median) | Notes |
|-----------|--------------|-------|
| `parse_multipage_520p` | **1.89 ms** | Parse DJVM directory + all page descriptors, 520 pages |
| `iterate_pages_520p` | **496 µs** | Read width/height/dpi for all 520 pages (no render) |
| `render_large_doc_first_page` | **10.4 ms** | Render page 1 of 520 at native 600 dpi |
| `render_large_doc_mid_page` | **35.6 ms** | Render page 260 of 520 — dense text, large JB2 bitstream |
| `decode_mask_large_600dpi` | **8.43 ms** | Decode JB2 mask only, page 1 (sparse, 11 KB bitstream) |
| `decode_mask_mid_600dpi` | **30.8 ms** | Decode JB2 mask only, page 260 (dense, 65 KB bitstream) |
| `text_extraction_single_page` | **194 µs** | TXTz parse + plain text output, watchmaker.djvu |

---

## Comparison with DjVuLibre 3.5.29

### CLI comparison (`djvu render` vs `ddjvu`)

Method: 10 runs, mean, subprocess timing in Python.
djvu-rs outputs PNG; ddjvu outputs PPM (uncompressed). Both include process startup.
Output resolution: 150 dpi for all files.

| File | djvu-rs CLI | ddjvu CLI | Ratio |
|------|-------------|-----------|-------|
| watchmaker.djvu (color IW44, 2550×3301) | **35.8 ms** | 355.3 ms | **~10× faster** |
| cable_1973_100133.djvu (bilevel JB2, 2550×3301) | **29.5 ms** | 75.0 ms | **~2.5× faster** |

djvu-rs process startup is ~5 ms; ddjvu startup is ~25–35 ms. For very large files the
CLI margin narrows toward the library-level ratio.

### Library-level comparison (render-only, no process overhead)

Method: 20 warm-up + 20 measured iterations via `clock_gettime` (DjVuLibre) and
`std::time::Instant` (djvu-rs). Page already parsed and in memory; only render step timed.

Test file: `colorbook.djvu` — 2260×3669 px at 400 dpi, rendered to 848×1376 px at 150 dpi.

| | djvu-rs | DjVuLibre C API | Ratio |
|-|---------|-----------------|-------|
| colorbook, 150 dpi (848×1376 output) | 34.5 ms | **6.13 ms** | DjVuLibre ~5.6× faster |

**Why DjVuLibre wins here:** DjVuLibre uses **progressive IW44 decode** — it only
decodes the low-frequency wavelet bands needed for the target output resolution.
For 150 dpi output from a 400 dpi source (37.5% scale, ~14% of pixels), it skips
the high-frequency bands entirely. djvu-rs always decodes all IW44 bands and then
resamples — doing ~7× more decode work.

**djvu-rs advantage: document open latency.**  
`parse_multipage_520p`: djvu-rs ≈ 1.9 ms vs DjVuLibre ≈ 24–60 ms → **10–30× faster open**.

### Summary

| Scenario | Winner | Margin |
|----------|--------|--------|
| CLI (process startup included) | **djvu-rs** | ~2.5–10× |
| Native-resolution render | comparable | — |
| Downscaled render (< native DPI) | **DjVuLibre** | ~5.6× (partial IW44 decode) |
| Dense 600 dpi bilevel (large JB2) | **DjVuLibre** | ~2.6× (sequential ZP decoder) |
| Document open / parse | **djvu-rs** | ~10–30× |

---

## Notes

- **Partial IW44 decode** is the main gap vs DjVuLibre for typical viewer use (thumbnails,
  paginated readers rendering at screen DPI). It is a planned optimization.
- JB2 and IW44 pure decode are sub-millisecond to low-millisecond for typical pages.
- Full native-resolution render (2550×3301 px): ~70–74 ms.
- Corpus benchmarks use public domain files from Internet Archive.
- `render_large_doc_first_page` improved from ~43 ms → 10.4 ms across v0.5.2–v0.5.3:
  - `Pixmap::new` fill changed from per-pixel push to `vec![fill; n]`
  - `composite_bilevel` uses row-slice writes
  - JB2 symbol dictionary cached via `RwLock<HashMap<usize, Arc<JB2Dict>>>`
  - `decode_bitmap_direct` inner loop: `split_at_mut` eliminates per-pixel multiply
