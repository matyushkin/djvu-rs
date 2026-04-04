# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- **CLI tool** — `djvu info`, `djvu render`, `djvu text` subcommands
- **`no_std` support** — IFF/BZZ/JB2/IW44/ZP modules work with `alloc` only

[Unreleased]: https://github.com/matyushkin/djvu-rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/matyushkin/djvu-rs/releases/tag/v0.1.0
