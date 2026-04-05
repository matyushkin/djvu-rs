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
- **Progressive rendering** — incremental BG44 wavelet refinement
- **Thumbnails** — TH44 embedded thumbnail extraction
- `no_std` compatible — IFF/BZZ/JB2/IW44/ZP modules work with `alloc` only (no `std`)

## Quick start

```rust
use djvu_rs::Document;

let doc = Document::open("file.djvu")?;
println!("{} pages", doc.page_count());

let page = doc.page(0)?;
println!("{}x{} @ {} dpi", page.width(), page.height(), page.dpi());

let pixmap = page.render()?;
// pixmap.data is RGBA bytes (4 bytes per pixel, row-major)
// pixmap.width, pixmap.height are dimensions in pixels
```

## Text extraction

```rust
use djvu_rs::Document;

let doc = Document::open("scanned.djvu")?;
let page = doc.page(0)?;
if let Some(text) = page.text()? {
    println!("Page text: {}", text);
}
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

## Feature flags

| Flag | Default | Description |
|------|---------|-------------|
| `std` | enabled | Enables `Document`, `Page`, file I/O, and rendering. Disable for `no_std` |

Without `std`, the crate provides IFF parsing, BZZ decompression, JB2/IW44 decoding, text/annotation parsing — all codec primitives that work on byte slices.

## Minimum supported Rust version (MSRV)

Rust **1.88** (edition 2024 features: let-chains stabilized in 1.88)

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
