# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] — 2026-04-05

### Added

- **TIFF export** — `djvu_to_tiff` converts DjVu to multi-page TIFF in color (RGB8) or bilevel
  (Gray8) modes; CLI: `djvu render --format tiff`; feature-gated: `--features tiff`
- **BGjp/FGjp JPEG decoder** — DjVu pages with JPEG-encoded background or foreground now render
  correctly; uses `zune-jpeg` (pure Rust, no libjpeg)
- **Async render API** — `djvu_async::render_pixmap_async` / `render_gray8_async` delegate
  CPU-bound IW44/JB2 work to `tokio::task::spawn_blocking`; feature-gated: `--features async`
- **Document metadata** — `metadata::parse_metadata` / `parse_metadata_bzz` extract METa/METz
  chunks; `DjVuMetadata` struct with title, author, date, and arbitrary key-value fields
- **Chunk introspection API** — `DjVuPage::raw_chunk`, `all_chunks`, `chunk_ids`;
  `DjVuDocument::raw_chunk`, `all_chunks` for direct access to IFF chunk data
- **DJVI shared dictionary** — `Sjbz` pages that reference a shared JB2 dictionary via `INCL`
  chunks now decode correctly; fixes rendering of multi-page documents with shared symbol sets
- **TextLayer coordinate transform** — `TextLayer::transform(scale, rotation)` maps zone rectangles
  to the rendered page coordinate system; simplifies hit-testing in viewer applications
- **DCTDecode PDF export** — `pdf::djvu_to_pdf_with_options` encodes page images as JPEG
  (DCTDecode) instead of raw RGB (FlateDecode); typically 5–10× smaller output; `PdfOptions`
  controls JPEG quality (default 80)
- **Lanczos-3 resampling** — `RenderOptions { resampling: Resampling::Lanczos3, .. }` applies a
  two-pass separable 6-tap Lanczos kernel after rendering; sharper thumbnails at the cost of ~5×
  render time vs `Bilinear`
- **Grayscale output** — `djvu_render::render_gray8` returns a `GrayPixmap` (1 byte/pixel);
  CLI: `djvu render --gray`
- **Permissive render mode** — `RenderOptions::permissive = true` skips corrupted or unsupported
  chunks instead of returning an error; useful for broken files in the wild
- **Benchmark suite** — `benches/render.rs` gains `render_scaled` (Bilinear vs Lanczos-3) and
  `pdf_export` benchmarks; `BENCHMARKS.md` documents results on Apple M1 Max and comparison vs
  DjVuLibre 3.5.29
- **Benchmark CI** — `.github/workflows/bench.yml` runs `cargo bench` on `ubuntu-latest` and
  `macos-latest` on every release tag; Criterion HTML reports uploaded as artifacts

### Performance

- **SIMD YCbCr→RGB** — `Iw44Image::to_rgb` now processes 8 pixels per iteration using
  `wide::i32x8` (maps to AVX2 on x86_64, NEON on ARM64, scalar fallback elsewhere); eliminates
  per-pixel overhead in the full-resolution color conversion hot path
- **Packed bitwise mask dilation** — `Bitmap::dilate` uses bitwise OR on packed `u64` words
  instead of per-pixel loops; 2–4× faster for bold-text rendering
- **Composite optimisation** — eliminated redundant mask sampling in the 3-layer composite loop

### Fixed

- **Permissive mode robustness** — decode pipeline no longer panics on documents with missing or
  truncated BG44/FG44 chunks when `permissive = true`

## [0.2.1] — 2026-04-04

### Fixed

- Exclude `.cargo/config.toml` from published package — it contained `-D warnings` which caused
  docs.rs builds to fail silently

## [0.2.0] — 2026-04-04

### Added

- **Structural PDF export** — `djvu render --format pdf` now produces searchable PDFs with selectable
  text (from TXTz/TXTa), bookmarks (NAVM → PDF outline), and hyperlinks (ANTz → PDF link annotations)
- **Mask / layer extraction API** — `DjVuPage::extract_mask()`, `extract_foreground()`,
  `extract_background()`; CLI: `djvu render --layer mask|fg|bg`
- **`RenderOptions::fit_to_width` / `fit_to_height` / `fit_to_box`** — aspect-preserving smart scaling
  helpers that respect page rotation
- **User-controllable rotation** — `RenderOptions::rotation` overrides the INFO chunk value

### Fixed

- **Gamma correction** — gamma LUT now applied in all render paths (`render_pixmap`, `render_coarse`,
  `render_progressive`, rotation branches)
- **Page rotation** — `render_pixmap` and `render_coarse` now apply the rotation from the INFO chunk;
  output dimensions swap correctly for 90°/270° pages
- **FGbz multi-color foreground** — per-glyph blit index is now used when compositing; documents with
  multi-color foreground (stamps, colored annotations) render correctly

### Performance

- **Area-averaging downscale** — render at scale < 1.0 now uses box-filter averaging instead of
  bilinear; better anti-aliasing and ~2× faster for thumbnail/overview sizes
- **Composite optimisation** — eliminated redundant mask sampling in 3-layer composite loop

### Refactored

- **Removed `ouroboros` dependency** — `Document` is now a fully owned struct; no self-referential
  proc-macro required; `lib.rs` is now truly `unsafe`-free

## [0.1.0] — 2026-04-04

### Added

- **IFF container parser** — zero-copy, borrowing slices from input (`iff::parse_form`)
- **JB2 bilevel image decoder** — ZP adaptive arithmetic coding with symbol dictionary (`jb2_new`)
- **IW44 wavelet image decoder** — planar YCbCr storage, progressive multi-chunk refinement (`iw44_new`)
- **BZZ decompressor** — ZP + MTF + BWT for DIRM, NAVM, ANTz chunks (`bzz_new`)
- **Text layer extraction** — TXTz/TXTa chunk parsing with full zone hierarchy (`text`)
- **Annotation parsing** — ANTz/ANTa chunks: hyperlinks, map areas, background color (`annotation`)
- **Bookmarks** — NAVM table-of-contents parsing (`DjVuDocument::bookmarks`)
- **Multi-page documents** — DJVM bundle format with DIRM directory chunk
- **Page rendering** — composite foreground mask + background wavelet into RGBA output
- **Progressive rendering** — incremental BG44 wavelet refinement (`Page::render_scaled_progressive`)
- **Thumbnails** — TH44 embedded thumbnail extraction (`Page::thumbnail`)
- **High-level API** — `Document` / `Page` (requires `std` feature)
- **New document model** — `DjVuDocument` / `DjVuPage` built on clean-room codecs
- **CLI tool** — `djvu info`, `djvu render --format png|pdf|cbz`, `djvu text` subcommands
- **Rasterized PDF export** — `djvu render --format pdf` embeds each page as an RGB image (FlateDecode)
- **CBZ export** — `djvu render --format cbz` produces a comic-book ZIP with PNG pages
- **`no_std` support** — IFF/BZZ/JB2/IW44/ZP modules work with `alloc` only

[Unreleased]: https://github.com/matyushkin/djvu-rs/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/matyushkin/djvu-rs/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/matyushkin/djvu-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/matyushkin/djvu-rs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/matyushkin/djvu-rs/releases/tag/v0.1.0
