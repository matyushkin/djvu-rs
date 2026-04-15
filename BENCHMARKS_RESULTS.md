# Benchmark Results

Platform: Apple M1 Max, 10 cores, 64 GB RAM
OS: macOS 26.3.1 (Darwin 25.3)
Rust: 1.92.0 stable (edition 2024)
Profile: release (opt-level 3, lto = thin)
Date: 2026-04-15

All Criterion benchmarks use 100 samples, 3 s warm-up, 5 s measurement.

---

## Codec benchmarks (`cargo bench --bench codecs`)

Test files: `references/djvujs/library/assets/` and `tests/corpus/`

| Benchmark | File | Payload | Time (median) | Notes |
|-----------|------|---------|--------------|-------|
| `bzz_decode` | navm_fgbz.djvu NAVM | 89 bytes | **65 ns** | ZP + MTF + BWT; tiny NAVM chunk |
| `jb2_decode` | boy_jb2.djvu Sjbz | — | **132 µs** | Bilevel JB2 decode, small page |
| `iw44_decode_first_chunk` | boy.djvu BG44 | — | **715 µs** | Single IW44 wavelet chunk |
| `jb2_decode_corpus_bilevel` | cable_1973_100133.djvu Sjbz | — | **423 µs** | Larger bilevel scan; 8× faster after ZP u32 (#180) |
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
| 72 dpi | ~138×184 px | **236 µs** |
| 144 dpi | ~276×368 px | **920 µs** |
| 300 dpi | ~576×768 px | **3.10 ms** |
| 600 dpi | ~1152×1536 px | **11.7 ms** |

### Full-resolution corpus render

| Benchmark | File | Native size | Time (median) | Notes |
|-----------|------|-------------|--------------|-------|
| `render_coarse` | boy.djvu | 192×256 | **1.31 ms** | |
| `render_colorbook` | colorbook.djvu | 2260×3669 (400 dpi) | **6.75 ms** | 150 dpi, warm (sub=4 mask + partial BG44) |
| `render_colorbook_cold` | colorbook.djvu | 2260×3669 (400 dpi) | **22.5 ms** | cold (ZP + wavelet + RGB, first render) |
| `render_corpus_color` | watchmaker.djvu | 2550×3301 | **67 ms** | native 600 dpi, full IW44 |
| `render_corpus_bilevel` | cable_1973_100133.djvu | 2550×3301 | **66 ms** | native 600 dpi, bilevel JB2 |
| `pdf_export_sequential` | watchmaker.djvu | — | **868 ms** | 12 pages, `output_dpi=150`, DCTDecode JPEG-80 |
| `pdf_export_parallel` | watchmaker.djvu | — | **162 ms** | same, `--features parallel` (rayon), **5.4× faster** |

Note: The `render_colorbook` benchmark renders at 150 dpi (848×1376 output). For sub=4
renders djvu-rs applies a cascade of optimizations: (1) partial BG44 decode (first chunk
only), (2) cached 1/4-res max-pool mask pyramid (single bit lookup per pixel), (3)
bit-shift instead of UDIV for bg/mask coordinate transforms.  Cold render includes full
ZP arithmetic decode for the first BG44 chunk (~20 ms) + wavelet + composite.

---

## Document benchmarks (`cargo bench --bench document`)

Test file: `tests/corpus/pathogenic_bacteria_1896.djvu` (520 pages, 25 MB, bilevel JB2 at 600 dpi)
Text layer: `tests/corpus/watchmaker.djvu` (TXTz present)

| Benchmark | Time (median) | Notes |
|-----------|--------------|-------|
| `parse_multipage_520p` | **2.18 ms** | Parse DJVM directory + all page descriptors, 520 pages |
| `iterate_pages_520p` | **1.5 µs** | Read width/height/dpi for all 520 pages (no render) |
| `render_large_doc_first_page` | **10.4 ms** | Render page 1 of 520 at native 600 dpi |
| `render_large_doc_mid_page` | **10.8 ms** | Render page 260 of 520 — dense text, large JB2; 3.3× faster after ZP u32 (#180) |
| `decode_mask_large_600dpi` | **2.38 ms** | Decode JB2 mask only, page 1; 3.5× faster after ZP u32 (#180) |
| `decode_mask_mid_600dpi` | **15.0 ms** | Decode JB2 mask only, page 260; 2× faster after ZP u32 (#180) |
| `text_extraction_single_page` | **186 µs** | TXTz parse + plain text output, watchmaker.djvu |

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
| colorbook, 150 dpi (848×1376 output) — warm | **6.75 ms** | 6.13 ms | djvu-rs within **~10%** of DjVuLibre |
| colorbook, 150 dpi (848×1376 output) — cold | **22.5 ms** | — | first render (ZP + wavelet + composite) |

**Progressive IW44 optimizations implemented (issue #144):**
- Partial BG44 decode: only the first chunk (coarsest wavelet bands) for sub=4 renders
- 1/4-resolution max-pool mask pyramid: single bit lookup per pixel vs 4–9 lookups in the full-res mask
- Bit-shift coordinate transforms: replaced `fx / bg_subsample` (UDIV) with `fx >> bg_shift` in the hot composite loop
- Cumulative speedup: 37.4 ms → 6.5 ms for 150 dpi warm render (5.75× faster)

The remaining ~6% gap vs DjVuLibre is due to DjVuLibre using C with platform-specific
SIMD in the YCbCr→RGB conversion and the wavelet inverse transform.

**djvu-rs advantage: document open latency.**  
`parse_multipage_520p`: djvu-rs ≈ 1.9 ms vs DjVuLibre ≈ 24–60 ms → **10–30× faster open**.

### Summary

| Scenario | Winner | Margin |
|----------|--------|--------|
| CLI (process startup included) | **djvu-rs** | ~2.5–10× |
| Native-resolution render | comparable | — |
| Downscaled render (< native DPI), warm | **djvu-rs** | within 6% of DjVuLibre |
| Downscaled render (< native DPI), cold | **DjVuLibre** | ~4.4× (cold ZP decode) |
| Dense 600 dpi bilevel (large JB2) | comparable | ZP decoder widened to u32 in #180 closed the gap |
| Document open / parse | **djvu-rs** | ~10–30× |

---

## Notes

- JB2 and IW44 pure decode are sub-millisecond to low-millisecond for typical pages.
- Full native-resolution render (2550×3301 px): ~67–70 ms.
- Corpus benchmarks use public domain files from Internet Archive.
- `render_large_doc_first_page` improved from ~43 ms → 10.4 ms across v0.5.2–v0.5.3:
  - `Pixmap::new` fill changed from per-pixel push to `vec![fill; n]`
  - JB2 symbol dictionary cached via `RwLock<HashMap<usize, Arc<JB2Dict>>>`
  - `decode_bitmap_direct` inner loop: `split_at_mut` eliminates per-pixel multiply

**ZP decoder u32 widening (#180, 2026-04-15):** widening `a`, `c`, `fence` from u16 to u32
eliminates casts in the inner decode loop, enabling better register allocation and removing
the `leading_ones()` u16 truncation in `renormalize`. Results:
- `jb2_decode_corpus_bilevel`: 3.36 ms → 423 µs (**8× faster**)
- `decode_mask_large_600dpi`: 8.43 ms → 2.38 ms (**3.5× faster**)
- `decode_mask_mid_600dpi`: 30.8 ms → 15.0 ms (**2× faster**)
- `render_large_doc_mid_page`: 35.6 ms → 10.8 ms (**3.3× faster**)
