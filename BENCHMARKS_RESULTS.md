# Benchmark Results

Platform: Apple Silicon (`arm64`)
OS: macOS 26.3.1 (Darwin 25.3)
Rust: 1.92.0 stable (edition 2024)
Profile: release (opt-level 3, lto = thin)
Date: 2026-05-16

Command: `cargo bench --workspace --features cli,tiff`

All Criterion benchmarks use 100 samples, 3 s warm-up, 5 s measurement.
Criterion's local baseline comparison reported improvements for most codec,
document, and native-render benches; `pdf_export_sequential` was unchanged;
small thumbnail paths (`render_page/dpi/72` and `render_scaled_0.5x/bilinear`)
were ~10–12% slower.

---

## Cross-architecture benchmark matrix

Use this table for architecture-sensitive benchmark updates. Every row must
identify the operating system, CPU, Rust toolchain, `target_arch`, relevant
`target_feature`s, and `RUSTFLAGS`; missing architecture cells are kept explicit
so follow-up SIMD work can fill them without changing the schema.

### Platform metadata template

```md
**Platform.**
- OS:
- CPU:
- target_arch:
- target_feature(s):
- Rust:
- RUSTFLAGS:
- Source artifact:
```

### Seed matrix (2026-05-17)

| Target family | OS | CPU / runner | target_arch | target_feature(s) | Rust | RUSTFLAGS | Source artifact | Status |
|---------------|----|--------------|-------------|-------------------|------|-----------|-----------------|--------|
| Apple ARM64 | macOS 26.3.1 (Darwin 25.3) | Apple M1 Max, 10 cores | `aarch64` | ARM64 baseline; NEON available on Apple Silicon | 1.92.0 stable | unset | Current local `cargo bench --workspace --features cli,tiff` summary below | Current broad baseline |
| Linux x86_64 baseline | Ubuntu GitHub-hosted runner | `ubuntu-latest` | `x86_64` | baseline x86-64 codegen | stable from workflow | unset | #189 artifact run `25299920836` from `.github/workflows/bench.yml` `bench-x86-64-v3` validation | Current selected IW44/render baseline |
| Linux x86_64-v3 / AVX2 | Ubuntu GitHub-hosted runner | `ubuntu-latest` | `x86_64` | `avx2` via x86-64-v3 codegen | stable from workflow | `-C target-cpu=x86-64-v3` | `.github/workflows/bench.yml` `bench-x86-64-v3` job; #189 artifact run `25299920836` | Current AVX2 validation exists for selected IW44/render benches |
| wasm32 scalar | — | — | `wasm32` | scalar | — | — | Blocked pending #306 harness | Missing |
| wasm32 simd128 | — | — | `wasm32` | `simd128` | — | `-C target-feature=+simd128` | Blocked pending #306 harness | Missing |
| Linux aarch64 | — | — | `aarch64` | NEON | — | — | No trustworthy current artifact | Missing |

### Architecture-sensitive seed numbers

These rows intentionally cover only measurements that already have trustworthy
source artifacts. Later issues should add rows instead of changing the platform
metadata format.

| Benchmark | Apple ARM64 local | Linux x86_64 baseline | Linux x86_64-v3 / AVX2 | wasm32 scalar | wasm32 simd128 | Linux aarch64 |
|-----------|------------------:|----------------------:|-----------------------:|--------------:|----------------:|--------------:|
| `iw44_decode_corpus_color` | **637 µs** | 1,385,461 ns | 1,123,865 ns | missing | missing | missing |
| `iw44_decode_first_chunk` | **571 µs** | 765,703 ns | 728,565 ns | missing | missing | missing |
| `iw44_to_rgb_colorbook/sub1_full_decode` | **5.39 ms** | 9,231,033 ns | 9,129,333 ns | missing | missing | missing |
| `iw44_to_rgb_colorbook/sub2_partial_decode` | **1.29 ms** | 2,164,523 ns | 2,199,280 ns | missing | missing | missing |
| `iw44_to_rgb_colorbook/sub4_partial_decode` | **337 µs** | 565,640 ns | 583,519 ns | missing | missing | missing |
| `render_colorbook` | **6.90 ms** | 13,072,440 ns | 12,826,562 ns | missing | missing | missing |
| `render_colorbook_cold` | **17.3 ms** | 28,127,606 ns | 27,105,326 ns | missing | missing | missing |
| `render_colorbook_stages/mask_decode` | **4.13 ms** | 5,325,125 ns | 5,107,550 ns | missing | missing | missing |
| `render_corpus_color` | **68.7 ms** | 133,813,976 ns | 133,185,634 ns | missing | missing | missing |

Notes:

- Apple ARM64 values come from the current broad local benchmark summary in
  this file.
- Linux x86_64 baseline and x86_64-v3 values come from the #189 AVX2 validation
  artifact recorded in `PERF_EXPERIMENTS.md`.
- The wasm32 and Linux aarch64 cells are explicitly missing. #306 and #308 are
  expected to fill them using the metadata template above.

---

## Codec benchmarks (`cargo bench --bench codecs`)

Test files: `references/djvujs/library/assets/` and `tests/corpus/`

| Benchmark | File | Payload | Time (median) | Notes |
|-----------|------|---------|--------------|-------|
| `bzz_decode` | navm_fgbz.djvu NAVM | 89 bytes | **68.6 ns** | ZP + MTF + BWT; tiny NAVM chunk |
| `jb2_decode` | boy_jb2.djvu Sjbz | — | **128 µs** | Bilevel JB2 decode, small page |
| `iw44_decode_first_chunk` | boy.djvu BG44 | — | **571 µs** | Single IW44 wavelet chunk |
| `jb2_decode_corpus_bilevel` | cable_1973_100133.djvu Sjbz | — | **417 µs** | Larger bilevel scan |
| `iw44_decode_corpus_color` | watchmaker.djvu first BG44 | — | **637 µs** | Color IW44 chunk |
| `jb2_decode_large_600dpi` | pathogenic_bacteria_1896.djvu page mask | 11,438 bytes | **2.13 µs** | Large-page JB2 mask fast path |
| `iw44_to_rgb_colorbook/sub1_full_decode` | colorbook.djvu | — | **5.39 ms** | Full decode + RGB |
| `iw44_to_rgb_colorbook/sub2_partial_decode` | colorbook.djvu | — | **1.29 ms** | Partial decode at sub=2 |
| `iw44_to_rgb_colorbook/sub4_partial_decode` | colorbook.djvu | — | **337 µs** | Partial decode at sub=4 |
| `iw44_encode_color` | synthetic color page | — | **1.75 ms** | IW44 color encode |
| `iw44_encode_large_1024x1024` | synthetic 1024×1024 page | — | **16.2 ms** | IW44 large encode |
| `jb2_encode` | synthetic bilevel page | — | **168 µs** | JB2 encode |

Note: `bzz_decode` measures the 89-byte NAVM payload in navm_fgbz.djvu. BZZ performance
scales with payload size (BWT requires a full-block sort); large payloads (e.g. 6 KB DIRM
in a 520-page document) take ~1–5 ms.

### JB2 encoder quality baseline (#295)

Measured with `examples/encode_quality_jb2.rs` and
`examples/encode_quality_djbz.rs` on 2026-05-17. See `PERF_EXPERIMENTS.md`
for commands, platform metadata, and failure buckets.

Page-level JB2 corpus refresh:

| Mode | Pages | Bytes | bpp | vs original | Round-trip |
|------|------:|------:|----:|------------:|------------|
| Original `Sjbz` | 692 | 26,569,542 | 0.0263 | 1.000x | source |
| Direct `encode_jb2` | 692 | 46,252,033 | 0.0457 | 1.741x | 464 ok, 228 decode errors |
| Dict `encode_jb2_dict` | 692 | 36,016,741 | 0.0356 | 1.356x | 692 ok |

Shared-Djbz multi-page refresh:

| Mode | Files/pages | Bytes | bpp | vs original | Round-trip |
|------|------------:|------:|----:|------------:|------------|
| Original `Sjbz` totals | 6 / 688 | 26,424,220 | 0.0262 | 1.000x | source |
| Independent dict pages | 6 / 688 | 35,963,419 | 0.0356 | 1.361x | all pages ok |
| Bundled shared-Djbz | 6 / 688 | 34,986,136 | 0.0347 | 1.324x | all bundles ok |

Shared-Djbz is `0.973x` of independent dict output on this run, but the
current safe encoder family remains larger than the source `Sjbz` corpus
overall. The old `483/553` dict round-trip number is stale: dict encoding now
round-trips every refreshed page.

---

## Render benchmarks (`cargo bench --bench render`)

Test file: `references/djvujs/library/assets/boy.djvu` (192×256 px, 100 dpi)
Corpus files: `tests/corpus/`, colorbook: `references/djvujs/library/assets/colorbook.djvu`

### DPI scaling — boy.djvu (IW44 color page, 192×256 native)

| DPI | Approx output size | Time (median) |
|-----|--------------------|--------------|
| 72 dpi | ~138×184 px | **238 µs** |
| 144 dpi | ~276×368 px | **904 µs** |
| 300 dpi | ~576×768 px | **3.44 ms** |
| 600 dpi | ~1152×1536 px | **13.4 ms** |

### Full-resolution corpus render

| Benchmark | File | Native size | Time (median) | Notes |
|-----------|------|-------------|--------------|-------|
| `render_coarse` | boy.djvu | 192×256 | **1.09 ms** | |
| `render_colorbook` | colorbook.djvu | 2260×3669 (400 dpi) | **6.90 ms** | 150 dpi, warm (sub=4 mask + partial BG44) |
| `render_colorbook_stages/full_render` | colorbook.djvu | 2260×3669 (400 dpi) | **6.90 ms** | Warm full render stage |
| `render_colorbook_stages/mask_decode` | colorbook.djvu | 2260×3669 (400 dpi) | **4.13 ms** | JB2 mask decode stage |
| `render_colorbook_cold` | colorbook.djvu | 2260×3669 (400 dpi) | **17.3 ms** | cold (ZP + wavelet + RGB, first render) |
| `render_corpus_color` | watchmaker.djvu | 2550×3301 | **68.7 ms** | native 600 dpi, full IW44 |
| `render_corpus_bilevel` | cable_1973_100133.djvu | 2550×3301 | **69.7 ms** | native 600 dpi, bilevel JB2 |
| `render_scaled_0.5x/bilinear` | boy.djvu | 0.5× output | **144 µs** | Built-in bilinear downscale |
| `render_scaled_0.5x/lanczos3` | boy.djvu | 0.5× output | **3.75 ms** | Higher quality separable Lanczos3 |
| `pdf_export_sequential` | watchmaker.djvu | — | **848 ms** | 12 pages, `output_dpi=150`, DCTDecode JPEG-80 |
| `pdf_export_parallel` | watchmaker.djvu | — | — | Not measured in this run |

Note: The `render_colorbook` benchmark renders at 150 dpi (848×1376 output). For sub=4
renders djvu-rs applies a cascade of optimizations: (1) partial BG44 decode (first chunk
only), (2) cached 1/4-res max-pool mask pyramid (single bit lookup per pixel), (3)
bit-shift instead of UDIV for bg/mask coordinate transforms.  Cold render includes full
ZP arithmetic decode for the first BG44 chunk (~20 ms) + wavelet + composite.

### Native render stage breakdown (#281)

Measured as part of the full `cargo bench --workspace --features cli,tiff` run.
These benches are diagnostic: `render_pixmap` is the public allocation-returning
API, `render_into_reuse_buffer` composites into a caller-owned buffer, and
`render_streaming_discard` measures row generation without retaining output.

| Benchmark | Page | Time (median) | Notes |
|-----------|------|--------------:|-------|
| `render_native_stages/render_pixmap` | watchmaker color | **68.7 ms** | public `Pixmap` path, warm decode caches |
| `render_native_stages/render_into_reuse_buffer` | watchmaker color | **68.3 ms** | caller-owned RGBA buffer |
| `render_native_stages/render_streaming_discard` | watchmaker color | **65.1 ms** | row streaming, no retained output |
| `render_native_stages/mask_decode` | watchmaker color | **2.61 ms** | JB2 mask decode only |
| `render_native_stages/bg_to_rgb_warm` | watchmaker color | **2.77 ms** | cached IW44 inverse + RGB |
| `render_native_stages/render_pixmap` | cable mixed bilevel | **69.8 ms** | public `Pixmap` path, warm decode caches |
| `render_native_stages/render_into_reuse_buffer` | cable mixed bilevel | **69.4 ms** | caller-owned RGBA buffer |
| `render_native_stages/render_streaming_discard` | cable mixed bilevel | **66.0 ms** | row streaming, no retained output |
| `render_native_stages/mask_decode` | cable mixed bilevel | **408 µs** | JB2 mask decode only |
| `render_native_stages/bg_to_rgb_warm` | cable mixed bilevel | **2.81 ms** | cached IW44 inverse + RGB |

The diagnostic split shows the native-resolution gap is dominated by compositor
sampling and output materialization, not by warm JB2/IW44 decode alone.

---

## Document benchmarks (`cargo bench --bench document`)

Test file: `tests/corpus/pathogenic_bacteria_1896.djvu` (520 pages, 25 MB, bilevel JB2 at 600 dpi)
Text layer: `tests/corpus/watchmaker.djvu` (TXTz present)

| Benchmark | Time (median) | Notes |
|-----------|--------------|-------|
| `parse_multipage_520p` | **2.19 ms** | Parse DJVM directory + all page descriptors, 520 pages |
| `iterate_pages_520p` | **1.49 µs** | Read width/height/dpi for all 520 pages (no render) |
| `render_large_doc_first_page` | **10.6 ms** | Render page 1 of 520 at native 600 dpi |
| `render_large_doc_mid_page` | **10.5 ms** | Render page 260 of 520 — dense text, large JB2 |
| `decode_mask_large_600dpi` | **2.41 ms** | Decode JB2 mask only, page 1 |
| `decode_mask_mid_600dpi` | **15.2 ms** | Decode JB2 mask only, page 260 |
| `text_extraction_single_page` | **171 µs** | TXTz parse + plain text output, watchmaker.djvu |

---

## Comparison with DjVuLibre 3.5.29

The benchmark workflow keeps this comparison active:
`.github/workflows/bench.yml` runs `scripts/bench_djvulibre.sh` on the same
machine as Criterion and formats the result with `scripts/djvulibre_compare.py`.
The benchmark dashboard workflow also publishes a DjVuLibre overlay.

Current local run (2026-05-16):

- `boy.djvu`: small color IW44 downscale
- `colorbook.djvu`: large color IW44 downscale
- `watchmaker.djvu`: native-resolution color corpus page
- `cable_1973_100133.djvu`: native-resolution bilevel JB2 corpus page

> libdjvulibre C API is render-only with the page already decoded in memory.
> `ddjvu` CLI includes process startup and PPM output to `/dev/null`.

| Benchmark | djvu-rs | libdjvulibre C API | ddjvu CLI | Ratio |
|-----------|--------:|-------------------:|----------:|------:|
| `boy.djvu` @ 72 dpi, small color IW44 | **238 µs** | **179 µs** | **27.6 ms** | djvu-rs **1.3x slower** |
| `colorbook.djvu` @ 150 dpi, color IW44 | **6.90 ms** | **5.87 ms** | **62.3 ms** | djvu-rs **1.2x slower** |
| `watchmaker.djvu` @ 300 dpi, native color corpus | **68.71 ms** | **35.13 ms** | **74.1 ms** | djvu-rs **2.0x slower** |
| `cable_1973_100133.djvu` @ 300 dpi, native bilevel JB2 corpus | **69.73 ms** | **33.89 ms** | **69.7 ms** | djvu-rs **2.1x slower** |

For the closest cold-path djvu-rs Criterion comparison,
`render_colorbook_cold` is **17.3 ms**. That benchmark includes document
parsing and first render work, but it is not identical to libdjvulibre's
open+decode measurement. The libdjvulibre C API harness intentionally avoids
upscale cases because `ddjvu_page_render` can return a zero buffer when the
requested output rectangle is larger than the native page.

### Summary

| Scenario | Winner | Margin |
|----------|--------|--------|
| Downscaled render (< native DPI), warm | **DjVuLibre** | 1.2-1.3x faster in this matrix |
| Native-resolution corpus render | **DjVuLibre** | 2.0-2.1x faster |
| `ddjvu` CLI subprocess baseline | comparable to slower than djvu-rs render-only | 27.6-74.1 ms across measured cases |
| djvu-rs cold colorbook render | — | 17.3 ms; not directly equivalent to libdjvulibre open+decode |
| Document open / parse | **djvu-rs** | `parse_multipage_520p`: 2.19 ms |

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
