# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/matyushkin/djvu-rs/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/matyushkin/djvu-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/matyushkin/djvu-rs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/matyushkin/djvu-rs/releases/tag/v0.1.0
