# Benchmark Results

Platform: Apple Silicon (`arm64`)
OS: macOS 26.3.1 (Darwin 25.3)
Rust: 1.92.0 stable (edition 2024)
Profile: release (opt-level 3, lto = thin)
Date: 2026-05-04

All Criterion benchmarks use 100 samples, 3 s warm-up, 5 s measurement.

---

## Codec benchmarks (`cargo bench --bench codecs`)

Test files: `references/djvujs/library/assets/` and `tests/corpus/`

| Benchmark | File | Payload | Time (median) | Notes |
|-----------|------|---------|--------------|-------|
| `bzz_decode` | navm_fgbz.djvu NAVM | 89 bytes | **76 ns** | ZP + MTF + BWT; tiny NAVM chunk |
| `jb2_decode` | boy_jb2.djvu Sjbz | — | **134 µs** | Bilevel JB2 decode, small page |
| `iw44_decode_first_chunk` | boy.djvu BG44 | — | **599 µs** | Single IW44 wavelet chunk |
| `jb2_decode_corpus_bilevel` | cable_1973_100133.djvu Sjbz | — | **438 µs** | Larger bilevel scan |
| `iw44_decode_corpus_color` | watchmaker.djvu first BG44 | — | **671 µs** | Color IW44 chunk |
| `jb2_decode_large_600dpi` | pathogenic_bacteria_1896.djvu page mask | 11,438 bytes | **2.20 µs** | Large-page JB2 mask fast path |
| `iw44_to_rgb_colorbook/sub1_full_decode` | colorbook.djvu | — | **5.68 ms** | Full decode + RGB |
| `iw44_to_rgb_colorbook/sub2_partial_decode` | colorbook.djvu | — | **1.34 ms** | Partial decode at sub=2 |
| `iw44_to_rgb_colorbook/sub4_partial_decode` | colorbook.djvu | — | **348 µs** | Partial decode at sub=4 |
| `iw44_encode_color` | synthetic color page | — | **1.84 ms** | IW44 color encode |
| `iw44_encode_large_1024x1024` | synthetic 1024×1024 page | — | **17.4 ms** | IW44 large encode |
| `jb2_encode` | synthetic bilevel page | — | **174 µs** | JB2 encode |

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
| 72 dpi | ~138×184 px | **216 µs** |
| 144 dpi | ~276×368 px | **895 µs** |
| 300 dpi | ~576×768 px | **3.42 ms** |
| 600 dpi | ~1152×1536 px | **13.0 ms** |

### Full-resolution corpus render

| Benchmark | File | Native size | Time (median) | Notes |
|-----------|------|-------------|--------------|-------|
| `render_coarse` | boy.djvu | 192×256 | **1.11 ms** | |
| `render_colorbook` | colorbook.djvu | 2260×3669 (400 dpi) | **7.29 ms** | 150 dpi, warm (sub=4 mask + partial BG44) |
| `render_colorbook_stages/full_render` | colorbook.djvu | 2260×3669 (400 dpi) | **7.25 ms** | Warm full render stage |
| `render_colorbook_stages/mask_decode` | colorbook.djvu | 2260×3669 (400 dpi) | **4.30 ms** | JB2 mask decode stage |
| `render_colorbook_cold` | colorbook.djvu | 2260×3669 (400 dpi) | **17.9 ms** | cold (ZP + wavelet + RGB, first render) |
| `render_corpus_color` | watchmaker.djvu | 2550×3301 | **71.2 ms** | native 600 dpi, full IW44 |
| `render_corpus_bilevel` | cable_1973_100133.djvu | 2550×3301 | **70.4 ms** | native 600 dpi, bilevel JB2 |
| `render_scaled_0.5x/bilinear` | boy.djvu | 0.5× output | **128 µs** | Built-in bilinear downscale |
| `render_scaled_0.5x/lanczos3` | boy.djvu | 0.5× output | **3.81 ms** | Higher quality separable Lanczos3 |
| `pdf_export_sequential` | watchmaker.djvu | — | **844 ms** | 12 pages, `output_dpi=150`, DCTDecode JPEG-80 |
| `pdf_export_parallel` | watchmaker.djvu | — | — | Not measured in this run |

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
| `parse_multipage_520p` | **2.27 ms** | Parse DJVM directory + all page descriptors, 520 pages |
| `iterate_pages_520p` | **1.54 µs** | Read width/height/dpi for all 520 pages (no render) |
| `render_large_doc_first_page` | **12.8 ms** | Render page 1 of 520 at native 600 dpi |
| `render_large_doc_mid_page` | **12.7 ms** | Render page 260 of 520 — dense text, large JB2 |
| `decode_mask_large_600dpi` | **2.55 ms** | Decode JB2 mask only, page 1 |
| `decode_mask_mid_600dpi` | **16.1 ms** | Decode JB2 mask only, page 260 |
| `text_extraction_single_page` | **182 µs** | TXTz parse + plain text output, watchmaker.djvu |

---

## Comparison with DjVuLibre 3.5.29

The benchmark workflow keeps this comparison active:
`.github/workflows/bench.yml` runs `scripts/bench_djvulibre.sh` on the same
machine as Criterion and formats the result with `scripts/djvulibre_compare.py`.
The benchmark dashboard workflow also publishes a DjVuLibre overlay.

Current local run (2026-05-04):

- `boy.djvu`: small color IW44 downscale
- `colorbook.djvu`: large color IW44 downscale
- `watchmaker.djvu`: native-resolution color corpus page
- `cable_1973_100133.djvu`: native-resolution bilevel JB2 corpus page

> libdjvulibre C API is render-only with the page already decoded in memory.
> `ddjvu` CLI includes process startup and PPM output to `/dev/null`.

| Benchmark | djvu-rs | libdjvulibre C API | ddjvu CLI | Ratio |
|-----------|--------:|-------------------:|----------:|------:|
| `boy.djvu` @ 72 dpi, small color IW44 | **217 µs** | **122 µs** | **30.20 ms** | djvu-rs **1.8x slower** |
| `colorbook.djvu` @ 150 dpi, color IW44 | **7.29 ms** | **6.00 ms** | **67.10 ms** | djvu-rs **1.2x slower** |
| `watchmaker.djvu` @ 300 dpi, native color corpus | **73.28 ms** | **36.47 ms** | **81.00 ms** | djvu-rs **2.0x slower** |
| `cable_1973_100133.djvu` @ 300 dpi, native bilevel JB2 corpus | **72.33 ms** | **35.26 ms** | **74.10 ms** | djvu-rs **2.1x slower** |

For the closest cold-path djvu-rs Criterion comparison,
`render_colorbook_cold` is **17.9 ms**. That benchmark includes document
parsing and first render work, but it is not identical to libdjvulibre's
open+decode measurement. The libdjvulibre C API harness intentionally avoids
upscale cases because `ddjvu_page_render` can return a zero buffer when the
requested output rectangle is larger than the native page.

### Summary

| Scenario | Winner | Margin |
|----------|--------|--------|
| Downscaled render (< native DPI), warm | **DjVuLibre** | 1.2-1.8x faster in this matrix |
| Native-resolution corpus render | **DjVuLibre** | 2.0-2.1x faster |
| `ddjvu` CLI subprocess baseline | comparable to slower than djvu-rs render-only | 30.20-81.00 ms across measured cases |
| djvu-rs cold colorbook render | — | 17.9 ms; not directly equivalent to libdjvulibre open+decode |
| Document open / parse | **djvu-rs** | `parse_multipage_520p`: 2.27 ms |

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
