# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0](https://github.com/matyushkin/djvu-rs/compare/djvu-rs-v0.3.0...djvu-rs-v0.4.0) (2026-04-05)


### Features

* add fit_to_width/height/box to RenderOptions ([#33](https://github.com/matyushkin/djvu-rs/issues/33)) ([b371a93](https://github.com/matyushkin/djvu-rs/commit/b371a93099276cab573244cc262dc6ba093276cf))
* **api:** raw_chunk / all_chunks / chunk_ids on DjVuPage and DjVuDocument (Issue [#43](https://github.com/matyushkin/djvu-rs/issues/43)) ([#54](https://github.com/matyushkin/djvu-rs/issues/54)) ([3135627](https://github.com/matyushkin/djvu-rs/commit/31356279ad2739ae13ab36a144fbaabe5b5f63ab))
* **async:** async render API via tokio::task::spawn_blocking (Issue [#51](https://github.com/matyushkin/djvu-rs/issues/51)) ([#61](https://github.com/matyushkin/djvu-rs/issues/61)) ([452636f](https://github.com/matyushkin/djvu-rs/commit/452636f143c56f4fc674968be5d5a4d8bd15d14a))
* **bench:** add render_scaled and pdf_export benchmarks + BENCHMARKS.md (Issue [#52](https://github.com/matyushkin/djvu-rs/issues/52)) ([#62](https://github.com/matyushkin/djvu-rs/issues/62)) ([a8523c7](https://github.com/matyushkin/djvu-rs/commit/a8523c711c2fc3fc12d1089c5becf1b68af25595))
* **cli:** implement djvu info/render/text — 24/24 tests green ([eb2e9d6](https://github.com/matyushkin/djvu-rs/commit/eb2e9d61f0a12285f9c6d7de4a4665b31f19a32a))
* **cos-djvu:** benchmark suite, corpus infrastructure, BENCHMARKS.md (closes [#282](https://github.com/matyushkin/djvu-rs/issues/282)) ([#332](https://github.com/matyushkin/djvu-rs/issues/332)) ([50b6933](https://github.com/matyushkin/djvu-rs/commit/50b69330c73e5883a2954fe646f2cc7b7ec4e654))
* **cos-djvu:** phase 1 — IFF parser, typed errors, MIT skeleton (closes [#267](https://github.com/matyushkin/djvu-rs/issues/267)) ([#277](https://github.com/matyushkin/djvu-rs/issues/277)) ([1943f3f](https://github.com/matyushkin/djvu-rs/commit/1943f3f1cbef880038065ea377ccdfdff4ee33d0))
* **cos-djvu:** phase 2a — ZP arithmetic coder + BZZ decompressor (closes [#268](https://github.com/matyushkin/djvu-rs/issues/268)) ([#279](https://github.com/matyushkin/djvu-rs/issues/279)) ([4983056](https://github.com/matyushkin/djvu-rs/commit/498305678a0d13947e6fe5473d2bc6250383595d))
* **cos-djvu:** phase-5 rendering pipeline — compositing, gamma, scaling, AA (closes [#273](https://github.com/matyushkin/djvu-rs/issues/273)) ([7f5e161](https://github.com/matyushkin/djvu-rs/commit/7f5e161dd7f13c94ed1fb680966416e339eb80d5))
* **cos-djvu:** phase-6 quality — fuzz targets, benchmarks, no_std, full docs (closes [#274](https://github.com/matyushkin/djvu-rs/issues/274)) ([#324](https://github.com/matyushkin/djvu-rs/issues/324)) ([67fab7f](https://github.com/matyushkin/djvu-rs/commit/67fab7fdce945d972c574f61f9e460c32998a189))
* **cos-djvu:** text layer + annotations extraction (closes [#272](https://github.com/matyushkin/djvu-rs/issues/272)) ([#316](https://github.com/matyushkin/djvu-rs/issues/316)) ([5beeba9](https://github.com/matyushkin/djvu-rs/commit/5beeba96034a55396769d044ff1c20f20dc379a6))
* djvu render --format pdf|cbz, roadmap v0.1 finalised ([7d823f6](https://github.com/matyushkin/djvu-rs/commit/7d823f634a1efbc75f743d5d5a8a4f6056e0b0a0))
* DjVu to PDF converter with text, bookmarks, and hyperlinks ([#2](https://github.com/matyushkin/djvu-rs/issues/2)-[#6](https://github.com/matyushkin/djvu-rs/issues/6)) ([#29](https://github.com/matyushkin/djvu-rs/issues/29)) ([a6f0a74](https://github.com/matyushkin/djvu-rs/commit/a6f0a7486d85146e677ae094396e96b44675e894))
* document model — DjVuDocument, Page, DIRM, NAVM (closes [#271](https://github.com/matyushkin/djvu-rs/issues/271)) ([#283](https://github.com/matyushkin/djvu-rs/issues/283)) ([e36fd41](https://github.com/matyushkin/djvu-rs/commit/e36fd4169b591cb5f4146fc58bd3aaacf464d82b))
* import cos-djvu history, remove GPL legacy code ([0f33110](https://github.com/matyushkin/djvu-rs/commit/0f33110d1846e7114c6a46726c9e088cbef25bea))
* IW44 wavelet decoder with planar YCbCr (closes [#270](https://github.com/matyushkin/djvu-rs/issues/270)) ([#281](https://github.com/matyushkin/djvu-rs/issues/281)) ([f799e70](https://github.com/matyushkin/djvu-rs/commit/f799e7098dfbffdabada8e0c19fea2e31cdac351))
* JB2 bilevel decoder (closes [#269](https://github.com/matyushkin/djvu-rs/issues/269)) ([#280](https://github.com/matyushkin/djvu-rs/issues/280)) ([e2a6898](https://github.com/matyushkin/djvu-rs/commit/e2a6898618765fe96a07ddaac51a40b665b64efb))
* **jb2:** DJVI shared dictionary support via INCL chunks (Issue [#45](https://github.com/matyushkin/djvu-rs/issues/45)) ([#56](https://github.com/matyushkin/djvu-rs/issues/56)) ([86a63cb](https://github.com/matyushkin/djvu-rs/commit/86a63cb975303c14159340da4718ed3e23182e3e))
* mask and foreground/background layer extraction API ([#36](https://github.com/matyushkin/djvu-rs/issues/36)) ([d4c6527](https://github.com/matyushkin/djvu-rs/commit/d4c6527eb03aa9397a681ccdf03a27cf9ed77b0b))
* **metadata:** METa/METz document metadata parsing (Issue [#44](https://github.com/matyushkin/djvu-rs/issues/44)) ([#55](https://github.com/matyushkin/djvu-rs/issues/55)) ([eb4515b](https://github.com/matyushkin/djvu-rs/commit/eb4515b49819c3b75824738989c07153a7c2c0d6))
* **pdf:** DCTDecode background encoding — smaller PDF output (Issue [#49](https://github.com/matyushkin/djvu-rs/issues/49)) ([#59](https://github.com/matyushkin/djvu-rs/issues/59)) ([de90a9f](https://github.com/matyushkin/djvu-rs/commit/de90a9fd94ef98fdeb8822aef174e945ebe5a3ea))
* progressive DjVu rendering, multi-book cache, cos-diagnostics crate ([32432d8](https://github.com/matyushkin/djvu-rs/commit/32432d8c240f4a4673310a70158cf12cb9643635))
* **render:** BGjp/FGjp JPEG background/foreground decoder (Issue [#47](https://github.com/matyushkin/djvu-rs/issues/47)) ([#57](https://github.com/matyushkin/djvu-rs/issues/57)) ([b65bd81](https://github.com/matyushkin/djvu-rs/commit/b65bd817270c21248aa3ec46b140b6fb97a9b683))
* **render:** grayscale output mode — GrayPixmap + render_gray8 ([c13ebb7](https://github.com/matyushkin/djvu-rs/commit/c13ebb75ffa0673e670530bdd7ebe53f311a5044))
* **render:** grayscale output mode — GrayPixmap + render_gray8 (Issue [#15](https://github.com/matyushkin/djvu-rs/issues/15)) ([75d7b37](https://github.com/matyushkin/djvu-rs/commit/75d7b37aed6ce0a6d4d118abc8050b731089a8bf))
* **render:** Lanczos-3 separable resampling (Issue [#50](https://github.com/matyushkin/djvu-rs/issues/50)) ([#60](https://github.com/matyushkin/djvu-rs/issues/60)) ([56817d1](https://github.com/matyushkin/djvu-rs/commit/56817d162a335fe9597d2e65f84e0778ee147c65))
* **render:** permissive render mode — skip corrupted chunks ([dc5734a](https://github.com/matyushkin/djvu-rs/commit/dc5734a088f82b6c5bf7b6cfc06a599a2f342a2b))
* **render:** permissive render mode — skip corrupted chunks (Issue [#19](https://github.com/matyushkin/djvu-rs/issues/19)) ([df5a8d7](https://github.com/matyushkin/djvu-rs/commit/df5a8d715ba8534856d50f570752b8722118c2a0))
* **text:** TextLayer::transform — rotate + scale zone rects for rendered pages (Issue [#46](https://github.com/matyushkin/djvu-rs/issues/46)) ([#53](https://github.com/matyushkin/djvu-rs/issues/53)) ([c4a514e](https://github.com/matyushkin/djvu-rs/commit/c4a514e8bbdd4bccdb0032130b949580ef5306cc))
* **tiff:** TIFF export — multi-page color and bilevel modes (Issue [#48](https://github.com/matyushkin/djvu-rs/issues/48)) ([#58](https://github.com/matyushkin/djvu-rs/issues/58)) ([dc90cc0](https://github.com/matyushkin/djvu-rs/commit/dc90cc049ee7bd299ce6156c9c390c8616bdabca))
* transfer from cos-djvu, remove legacy GPL code, add PD corpus, benchmarks ([33fd496](https://github.com/matyushkin/djvu-rs/commit/33fd4969a691b5445c708b6c5f1ae2f877304f73))
* **ui:** table of contents navigation panel (closes [#60](https://github.com/matyushkin/djvu-rs/issues/60)) ([#298](https://github.com/matyushkin/djvu-rs/issues/298)) ([3fb0b2a](https://github.com/matyushkin/djvu-rs/commit/3fb0b2a94ff19891d6469f95aa381dcf66b7b712))
* user-controllable rotation in RenderOptions ([#35](https://github.com/matyushkin/djvu-rs/issues/35)) ([e0f79a8](https://github.com/matyushkin/djvu-rs/commit/e0f79a80bd3d3b2cc43933debb9a0b290aaea285))


### Bug Fixes

* add missing chunk_data binding in iw44_new doctest ([d1a210b](https://github.com/matyushkin/djvu-rs/commit/d1a210b8b61fabc5c7634282f0bfb8062503dfa3))
* apply gamma correction in all legacy render paths ([#9](https://github.com/matyushkin/djvu-rs/issues/9)) ([#22](https://github.com/matyushkin/djvu-rs/issues/22)) ([dfba614](https://github.com/matyushkin/djvu-rs/commit/dfba614c510d8da150ee76cf62f629f89cff48bc))
* apply page rotation from INFO chunk in render_pixmap and render_coarse ([#10](https://github.com/matyushkin/djvu-rs/issues/10)) ([#24](https://github.com/matyushkin/djvu-rs/issues/24)) ([adec5ee](https://github.com/matyushkin/djvu-rs/commit/adec5eed27bf5f096bad4187a2d88a5349bd07a4))
* **ci:** IJG license allowlist, no_std BTreeMap, clippy errors ([6a2a391](https://github.com/matyushkin/djvu-rs/commit/6a2a39199dffd1aa60cbe916bd2b28a516185dcd))
* clippy errors and fmt — let-chain, ref on let, line wrapping ([b4ba2f8](https://github.com/matyushkin/djvu-rs/commit/b4ba2f87acc234e7b7467f394169775b846c03b5))
* exclude .cargo/config.toml from published package (fixes docs.rs build) ([b9dd0da](https://github.com/matyushkin/djvu-rs/commit/b9dd0dafac62f0d6e12064f431d6b962b6a93e08))
* FGbz multi-color foreground palette — use per-glyph blit index ([#12](https://github.com/matyushkin/djvu-rs/issues/12)) ([#26](https://github.com/matyushkin/djvu-rs/issues/26)) ([7897164](https://github.com/matyushkin/djvu-rs/commit/789716408799e39c0e78e3caeedb051f913d26ac))
* **hard-rule:** eliminate last 5 .expect()/.unwrap() in production code (Issue [#443](https://github.com/matyushkin/djvu-rs/issues/443)) ([#444](https://github.com/matyushkin/djvu-rs/issues/444)) ([e4247ea](https://github.com/matyushkin/djvu-rs/commit/e4247eafc1eabadfac42fb04c318252ac0ebcc71))
* remove deprecated [[licenses.deny]] syntax from deny.toml (cargo-deny v2) ([7971e44](https://github.com/matyushkin/djvu-rs/commit/7971e448cdeb82d8be78910e9f275ecc1b975d65))
* replace all internal cos-djvu/cos_djvu references with djvu-rs/djvu_rs ([03fb17a](https://github.com/matyushkin/djvu-rs/commit/03fb17ae4460ca6baff50235993c6f005ddc08e4))
* update MSRV to 1.88 (let-chains stabilized in 1.88) ([8d5b94f](https://github.com/matyushkin/djvu-rs/commit/8d5b94f9d84c9065662e2464003aa75a95613421))
* vendor djvu-rs into crates/cos-djvu and fix production panics ([5f6d7fe](https://github.com/matyushkin/djvu-rs/commit/5f6d7fecadb7ea3bdcfa5cd530215ff28ea5e133)), closes [#4](https://github.com/matyushkin/djvu-rs/issues/4)


### Performance Improvements

* area-averaging downscale for better quality when rendering at reduced size ([#13](https://github.com/matyushkin/djvu-rs/issues/13)) ([#28](https://github.com/matyushkin/djvu-rs/issues/28)) ([b822ded](https://github.com/matyushkin/djvu-rs/commit/b822ded71372b0f70071f8245768d09b89e62a17))
* **bitmap:** packed bitwise dilation with ping-pong buffers ([2887689](https://github.com/matyushkin/djvu-rs/commit/288768968e910eccfe313785aa316ef8eb0fbdac))
* **bitmap:** packed bitwise dilation with ping-pong buffers (Issue [#17](https://github.com/matyushkin/djvu-rs/issues/17)) ([0814f55](https://github.com/matyushkin/djvu-rs/commit/0814f5519475073c4b426ca91360491c8ad69830))
* eliminate redundant mask sampling in 3-layer composite ([#14](https://github.com/matyushkin/djvu-rs/issues/14)) ([#27](https://github.com/matyushkin/djvu-rs/issues/27)) ([f601036](https://github.com/matyushkin/djvu-rs/commit/f601036e8988c2ea0c4f2a5742cd4b1321bc8065))
* **iw44:** SIMD YCbCr→RGB using wide::i32x8 (Issue [#1](https://github.com/matyushkin/djvu-rs/issues/1)) ([#64](https://github.com/matyushkin/djvu-rs/issues/64)) ([abceef4](https://github.com/matyushkin/djvu-rs/commit/abceef47d2bca524a20d13f4926f4dbf84e3c79b))
* **render:** eliminate redundant mask sampling in 3-layer composite (Issue [#14](https://github.com/matyushkin/djvu-rs/issues/14)) ([#37](https://github.com/matyushkin/djvu-rs/issues/37)) ([585991d](https://github.com/matyushkin/djvu-rs/commit/585991d961978277b8fdaf2b87236f6d1b825ac0))

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
