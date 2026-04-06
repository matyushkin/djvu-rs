# djvu-rs

[![Crates.io](https://img.shields.io/crates/v/djvu-rs.svg)](https://crates.io/crates/djvu-rs)
[![docs.rs](https://docs.rs/djvu-rs/badge.svg)](https://docs.rs/djvu-rs)
[![CI](https://github.com/matyushkin/djvu-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/matyushkin/djvu-rs/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Pure-Rust DjVu decoder. MIT licensed. Written from the DjVu v3 public specification.

## Features

- **IFF container parser** — zero-copy, borrowing slices from input
- **JB2 bilevel image decoder** — adaptive arithmetic coding (ZP coder) with symbol dictionary
- **IW44 wavelet image decoder** — planar YCbCr storage, multiple refinement chunks
- **BZZ decompressor** — ZP arithmetic coding + MTF + BWT (DIRM, NAVM, ANTz chunks)
- **Text layer extraction** — TXTz/TXTa chunk parsing with zone hierarchy (page/column/region/paragraph/line/word/character)
- **Annotation parsing** — ANTz/ANTa chunk parsing (hyperlinks, map areas, background color)
- **Bookmarks** — NAVM table-of-contents parsing
- **Multi-page documents** — DJVM bundle format with DIRM directory chunk
- **Page rendering** — composite foreground + background into RGBA output
- **PDF export** — selectable text, lossless IW44/JB2 embedding, bookmarks, hyperlinks
- **TIFF export** — multi-page color and bilevel modes (feature flag `tiff`)
- **hOCR / ALTO XML export** — text layer as hOCR or ALTO XML for OCR toolchains and archives
- **Serde support** — `Serialize`/`Deserialize` on all public data types (feature flag `serde`)
- **image-rs integration** — `image::ImageDecoder` impl for use with the `image` crate (feature flag `image`)
- **Async render** — `tokio::task::spawn_blocking` wrapper (feature flag `async`)
- `no_std` compatible — IFF/BZZ/JB2/IW44/ZP modules work with `alloc` only

## Quick start

```rust
use djvu_rs::{DjVuDocument, djvu_render::{render_pixmap, RenderOptions}};

let data = std::fs::read("file.djvu")?;
let doc = DjVuDocument::parse(&data)?;

println!("{} pages", doc.page_count());

let page = doc.page(0)?;
println!("{}×{} @ {} dpi", page.width(), page.height(), page.dpi());

let opts = RenderOptions { dpi: 150.0, ..Default::default() };
let pixmap = render_pixmap(page, &opts)?;
// pixmap.data — RGBA bytes (width × height × 4), row-major
```

## Text extraction

```rust
use djvu_rs::DjVuDocument;

let data = std::fs::read("scanned.djvu")?;
let doc = DjVuDocument::parse(&data)?;
let page = doc.page(0)?;

if let Some(text) = page.text()? {
    println!("{text}");
}
```

## PDF export

```rust
use djvu_rs::{DjVuDocument, pdf::djvu_to_pdf};

let data = std::fs::read("book.djvu")?;
let doc = DjVuDocument::parse(&data)?;

let pdf_bytes = djvu_to_pdf(&doc)?;
std::fs::write("book.pdf", pdf_bytes)?;
```

## TIFF export

Requires the `tiff` feature flag: `djvu-rs = { version = "…", features = ["tiff"] }`.

```rust
use djvu_rs::{DjVuDocument, tiff_export::{djvu_to_tiff, TiffOptions}};

let data = std::fs::read("scan.djvu")?;
let doc = DjVuDocument::parse(&data)?;

let tiff_bytes = djvu_to_tiff(&doc, &TiffOptions::default())?;
std::fs::write("scan.tiff", tiff_bytes)?;
```

## Async render

Requires the `async` feature flag: `djvu-rs = { version = "…", features = ["async"] }`.

```rust
use djvu_rs::{DjVuDocument, djvu_render::RenderOptions, djvu_async::render_pixmap_async};

let data = std::fs::read("file.djvu")?;
let doc = DjVuDocument::parse(&data)?;
let page = doc.page(0)?;

let opts = RenderOptions { dpi: 150.0, ..Default::default() };
let pixmap = render_pixmap_async(page, opts).await?;
```

## Low-level IFF access

```rust
use djvu_rs::iff::parse_form;

let data = std::fs::read("file.djvu")?;
let form = parse_form(&data)?;
println!("FORM type: {:?}", std::str::from_utf8(&form.form_type));
for chunk in &form.chunks {
    println!("  chunk {:?} ({} bytes)", std::str::from_utf8(&chunk.id), chunk.data.len());
}
```

## CLI

The `djvu` binary is included when the `std` feature is enabled (the default).

```sh
# Install
cargo install djvu-rs

# Document info
djvu info file.djvu

# Render page 1 to PNG at 200 DPI
djvu render file.djvu --dpi 200 --output page1.png

# Render all pages to a PDF
djvu render file.djvu --all --format pdf --output out.pdf

# Export all pages to CBZ
djvu render file.djvu --all --format cbz --output out.cbz

# Extract text from page 2
djvu text file.djvu --page 2

# Extract text from all pages
djvu text file.djvu --all
```

## hOCR and ALTO XML export

```rust
use djvu_rs::{DjVuDocument, ocr_export::{to_hocr, to_alto, HocrOptions, AltoOptions}};

let data = std::fs::read("scanned.djvu")?;
let doc = DjVuDocument::parse(&data)?;

// hOCR — compatible with Tesseract, ABBYY, and most OCR toolchains
let hocr = to_hocr(&doc, &HocrOptions::default())?;
std::fs::write("output.hocr", hocr)?;

// ALTO XML — used by libraries and archives (DFG, Europeana, etc.)
let alto = to_alto(&doc, &AltoOptions::default())?;
std::fs::write("output.xml", alto)?;
```

## Serde support

Requires the `serde` feature flag: `djvu-rs = { version = "…", features = ["serde"] }`.

All public data types (`DjVuBookmark`, `TextZone`, `MapArea`, `PageInfo`, etc.) implement
`Serialize` and `Deserialize`.

```rust
use djvu_rs::DjVuDocument;

let data = std::fs::read("book.djvu")?;
let doc = DjVuDocument::parse(&data)?;
let info = doc.page(0)?.info()?;

let json = serde_json::to_string_pretty(&info)?;
println!("{json}");
```

## image-rs integration

Requires the `image` feature flag: `djvu-rs = { version = "…", features = ["image"] }`.

```rust
use djvu_rs::{DjVuDocument, image_compat::DjVuDecoder};
use image::DynamicImage;

let data = std::fs::read("file.djvu")?;
let doc = DjVuDocument::parse(&data)?;
let page = doc.page(0)?;

let decoder = DjVuDecoder::new(&page, 150.0)?;
let img = DynamicImage::from_decoder(decoder)?;
img.save("page.png")?;
```

## Feature flags

| Flag | Default | Description |
|------|---------|-------------|
| `std` | enabled | `DjVuDocument`, file I/O, rendering, PDF export, CLI |
| `tiff` | disabled | TIFF export via the `tiff` crate |
| `async` | disabled | Async render API via `tokio::task::spawn_blocking` |
| `parallel` | disabled | Parallel multi-page render via `rayon` (`render_pages_parallel`) |
| `jpeg` | disabled | Standalone JPEG decode without full `std` (JPEG is included in `std` by default) |
| `mmap` | disabled | Memory-mapped file I/O via `memmap2` (`DjVuDocument::from_mmap`) |
| `serde` | disabled | `Serialize` + `Deserialize` for all public data types |
| `image` | disabled | `image::ImageDecoder` impl via `DjVuDecoder` — integrates with the `image` crate |

Without `std`, the crate provides IFF parsing, BZZ decompression, JB2/IW44 decoding,
text/annotation parsing — all codec primitives that work on byte slices.

## Performance

Measured on Apple M1 Max (Rust 1.92, release profile). Compared to DjVuLibre 3.5.29 C library:

| Page type | djvu-rs | libdjvulibre | Ratio |
|-----------|---------|--------------|-------|
| Color IW44, 300 dpi (849×1100 px) | 3.3 ms | 37 ms | **~11× faster** |
| Bilevel JB2, 300 dpi (849×1100 px) | 3.2 ms | 37 ms | **~12× faster** |
| Mixed, 600 dpi (2649×4530 px) | 42 ms | 12 ms | ~0.3× (libdjvulibre wins) |

Document open + parse is 10–30× faster than the C library. The 600 dpi regression is a
known target: djvu-rs uses scalar color conversion for large buffers; SIMD is in progress.

See [BENCHMARKS_RESULTS.md](BENCHMARKS_RESULTS.md) for full details.

## Minimum supported Rust version (MSRV)

Rust **1.88** (edition 2024 — let-chains stabilized in 1.88)

## Roadmap

See [GitHub milestones](https://github.com/matyushkin/djvu-rs/milestones) for the full roadmap and progress tracking.

## License

MIT. See [LICENSE](LICENSE).

## Specification

Written from the public DjVu v3 specification:
- https://www.sndjvu.org/spec.html
- https://djvu.sourceforge.net/spec/DjVu3Spec.djvu (the spec is itself a DjVu file)

No code derived from GPL-licensed DjVuLibre or any other GPL source.
All algorithms are independent implementations from the spec.
