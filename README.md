# djvu-rs

[![Crates.io](https://badgen.net/crates/v/djvu-rs)](https://crates.io/crates/djvu-rs)
[![docs.rs](https://docs.rs/djvu-rs/badge.svg)](https://docs.rs/djvu-rs)
[![CI](https://github.com/matyushkin/djvu-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/matyushkin/djvu-rs/actions/workflows/ci.yml)
[![Benchmarks](https://img.shields.io/badge/benchmarks-dashboard-blue)](https://matyushkin.github.io/djvu-rs/dev/bench/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Pure-Rust DjVu codec — decode and encode DjVu documents. MIT licensed, no GPL dependencies. Written from the DjVu v3 public specification.

## Features

- **IFF container parser** — zero-copy, borrowing slices from input
- **JB2 bilevel image decoder** — adaptive arithmetic coding (ZP coder) with symbol dictionary
- **JB2 bilevel image encoder** — encode any `Bitmap` into a valid `Sjbz` chunk payload
- **IW44 wavelet image decoder** — planar YCbCr storage, multiple refinement chunks
- **IW44 wavelet image encoder** — encode color (`Pixmap`) or grayscale (`GrayPixmap`) into `BG44`/`FG44` chunk payloads
- **G4/MMR bilevel image decoder** — ITU-T T.6 Group 4 fax decoder (`Smmr` chunks)
- **BZZ decompressor** — ZP arithmetic coding + MTF + BWT (DIRM, NAVM, ANTz chunks)
- **Text layer extraction** — TXTz/TXTa chunk parsing with zone hierarchy (page/column/region/paragraph/line/word/character)
- **Annotation parsing** — ANTz/ANTa chunk parsing (hyperlinks, map areas, background color)
- **Annotation encoding** — serialize `Annotation` + `MapArea` slices into ANTa or ANTz chunk payloads
- **Bookmarks** — NAVM table-of-contents parsing
- **Bookmark encoding** — serialize `DjVuBookmark` trees into NAVM chunk payloads
- **Multi-page documents** — DJVM bundle format with DIRM directory chunk; indirect DJVM creation and loading from directory
- **Page rendering** — composite foreground + background into RGBA output
- **PDF export** — selectable text, lossless IW44/JB2 embedding, bookmarks, hyperlinks (feature flag `pdf`)
- **TIFF export** — multi-page color and bilevel modes (feature flag `tiff`)
- **hOCR / ALTO XML export** — text layer as hOCR or ALTO XML for OCR toolchains and archives
- **Serde support** — `Serialize`/`Deserialize` on all public data types (feature flag `serde`)
- **EPUB 3 export** — page images + invisible text overlay + bookmarks as navigation (feature flag `epub`)
- **WebAssembly (WASM)** — `wasm-bindgen` bindings for use in browsers and Node.js (feature flag `wasm`)
- **image-rs integration** — `image::ImageDecoder` impl for use with the `image` crate (feature flag `image`)
- **Async render and lazy loading** — async render wrappers plus true per-page lazy loading over `AsyncRead + AsyncSeek` (feature flag `async`)
- **Workspace codec crates** — standalone `djvu-iff`, `djvu-bzz`, `djvu-bitmap`, `djvu-jb2`, `djvu-pixmap`, `djvu-iw44`, and `djvu-zp` crates for focused consumers
- **Fuzzing integration** — libFuzzer targets and in-tree OSS-Fuzz project files
- `no_std` compatible — IFF/BZZ/JB2/IW44/ZP codec modules work with `alloc` only

## Quick start

```rust
use djvu_rs::{DjVuDocument, djvu_render::{render_pixmap, RenderOptions}};

let data = std::fs::read("file.djvu")?;
let doc = DjVuDocument::parse(&data)?;

println!("{} pages", doc.page_count());

let page = doc.page(0)?;
println!("{}×{} @ {} dpi", page.width(), page.height(), page.dpi());

let target_dpi = 150u32;
let opts = RenderOptions {
    width: ((page.width() as u32 * target_dpi) / page.dpi() as u32).max(1),
    height: ((page.height() as u32 * target_dpi) / page.dpi() as u32).max(1),
    ..Default::default()
};
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

Requires the `pdf` feature flag: `djvu-rs = { version = "…", features = ["pdf"] }`.

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

The render entry points are synchronous and CPU-bound; run them on the
blocking thread pool with `tokio::task::spawn_blocking` so they stay off the
async runtime. The render error type stays the typed `RenderError` — there is
no wrapper enum.

```rust
use djvu_rs::{DjVuDocument, djvu_render::{self, RenderOptions}};

let data = std::fs::read("file.djvu")?;
let doc = DjVuDocument::parse(&data)?;
let page = doc.page(0)?.clone();

let target_dpi = 150u32;
let opts = RenderOptions {
    width: ((page.width() as u32 * target_dpi) / page.dpi() as u32).max(1),
    height: ((page.height() as u32 * target_dpi) / page.dpi() as u32).max(1),
    ..Default::default()
};
let pixmap = tokio::task::spawn_blocking(move || {
    djvu_render::render_pixmap(&page, &opts)
})
.await??; // outer `?`: join error (panic); inner `?`: RenderError
```

For progressive (per-BG44-chunk) rendering, `djvu_async::render_progressive_stream`
yields a `Stream` of frames, each produced on the blocking pool.

## Lazy async loading

Requires the `async` feature flag. The lazy loader keeps a seekable async
reader and fetches page/component byte ranges only when `page_async(i)` is
called. Parsed pages are cached as `Arc<DjVuPage>`.

```rust
use djvu_rs::djvu_async::from_async_reader_lazy;

let file = tokio::fs::File::open("book.djvu").await?;
let doc = from_async_reader_lazy(file).await?;
println!("{} pages", doc.page_count());

let page = doc.page_async(0).await?;
println!("first page: {}×{}", page.width(), page.height());
```

Supported shapes: single-page `FORM:DJVU` and bundled `FORM:DJVM`, including
shared `DJVI` dictionaries referenced via `INCL`. For browser-local `!Send`
readers on `wasm32`, use `from_async_reader_lazy_local`.

See [`examples/async_lazy_first_page.rs`](examples/async_lazy_first_page.rs)
for a native first-page latency probe and
[`examples/wasm/range_lazy.md`](examples/wasm/range_lazy.md) for the HTTP
`Range: bytes=start-end` integration shape.

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

## Encoding

### JB2 bilevel image encoder

```rust
use djvu_rs::{bitmap::Bitmap, jb2_encode::encode_jb2};

let mut bm = Bitmap::new(800, 1000);
// ... fill bitmap pixels ...
let sjbz_payload = encode_jb2(&bm);
// Wrap in a Sjbz IFF chunk and embed in a DjVu FORM:DJVU.
```

### IW44 wavelet encoder

```rust
use djvu_rs::{Pixmap, iw44_encode::{encode_iw44_color, Iw44EncodeOptions}};

let pixmap: Pixmap = /* ... your RGBA/YCbCr image ... */;
let chunks: Vec<Vec<u8>> = encode_iw44_color(&pixmap, &Iw44EncodeOptions::default());
// Each Vec<u8> is a BG44 chunk payload; wrap each in a BG44 IFF tag.
```

Grayscale:

```rust
use djvu_rs::{GrayPixmap, iw44_encode::{encode_iw44_gray, Iw44EncodeOptions}};

let gray: GrayPixmap = /* ... */;
let chunks: Vec<Vec<u8>> = encode_iw44_gray(&gray, &Iw44EncodeOptions::default());
```

`Iw44EncodeOptions` fields (all have sensible defaults):

| Field | Default | Description |
|-------|---------|-------------|
| `slices_per_chunk` | 10 | Slices packed into each BG44/FG44 chunk |
| `total_slices` | 100 | Total refinement slices to encode |
| `chroma_delay` | 0 | Y slices before Cb/Cr encoding begins |
| `chroma_half` | false | Legacy no-op; IW44 v1.2 always emits full-resolution chroma |

### Bookmark encoder

```rust
use djvu_rs::{djvu_document::DjVuBookmark, navm_encode::encode_navm};

let bookmarks = vec![
    DjVuBookmark { title: "Chapter 1".into(), url: "#page=1".into(), children: vec![] },
];
let navm_payload = encode_navm(&bookmarks);
```

### Annotation encoder

```rust
use djvu_rs::annotation::{Annotation, MapArea, encode_annotations, encode_annotations_bzz};

let ann = Annotation::default();
let areas: Vec<MapArea> = vec![/* ... */];

let anta_payload = encode_annotations(&ann, &areas);      // uncompressed ANTa
let antz_payload = encode_annotations_bzz(&ann, &areas);  // BZZ-compressed ANTz
```

## Indirect multi-page documents

Create an indirect DJVM index file that references per-page `.djvu` files:

```rust
use djvu_rs::djvm::create_indirect;

let index = create_indirect(&["page001.djvu", "page002.djvu", "page003.djvu"])?;
std::fs::write("book.djvu", index)?;
// Distribute book.djvu alongside the individual page files.
```

Load an indirect document by resolving component files from a directory:

```rust
use djvu_rs::DjVuDocument;

let index = std::fs::read("book.djvu")?;
let doc = DjVuDocument::parse_from_dir(&index, "/path/to/pages")?;
println!("{} pages", doc.page_count());
```

Two mutation paths cover indirect documents:
`DjVuDocumentMut::from_indirect_resolved` resolves the component files and
rebundles them into a mutable bundled document, and `IndirectRewritePlan`
rewrites individual component files on disk while keeping the document
indirect (each file is renamed atomically, but the multi-file commit as a
whole is not transactional). Opening an indirect index directly with
`DjVuDocumentMut::from_bytes` and calling `page_mut` remains unsupported; see
[`docs/indirect-djvm-mutation.md`](docs/indirect-djvm-mutation.md).

## CLI

The `djvu` binary is enabled by the `cli` feature.

```sh
# Install
cargo install djvu-rs --features cli

# Document info (--json for machine-readable output, --count for page count only)
djvu info file.djvu

# Render page 1 to PNG at 200 DPI
djvu render file.djvu --dpi 200 --output page1.png

# Render all pages to a PDF, EPUB, or CBZ
djvu render file.djvu --all --format pdf --output out.pdf
djvu render file.djvu --all --format epub --output out.epub
djvu render file.djvu --all --format cbz --output out.cbz

# Render a single layer (mask, foreground, background), with optional rotation
djvu render file.djvu --layer mask --rotate cw90 --output mask.png

# Extract text from page 2 (plain), or from all pages as hOCR / ALTO XML
djvu text file.djvu --page 2
djvu text file.djvu --all --format hocr --output out.hocr

# Merge documents into a bundled DJVM / extract a page range
djvu merge a.djvu b.djvu --output merged.djvu
djvu split book.djvu --pages 10-25 --output chapter.djvu

# Encode an image (PNG, JPEG, or TIFF) into a single-page DjVu (bilevel JB2, lossless)
djvu encode scan.png --output scan.djvu --dpi 300

# Encode into a layered lossy DjVu (JB2 mask + IW44 background + FGbz foreground color)
djvu encode scan.jpg --quality quality --output scan.djvu --dpi 300

# Use the conservative archival color profile
djvu encode scan.png --quality archival --output scan.djvu --dpi 300

# Opt into adaptive mask segmentation for uneven scans
djvu encode scan.png --quality quality --binarization sauvola --bg-inpaint --output scan.djvu

# Cap the IW44 background at a bits-per-pixel budget (smaller file, lower quality)
djvu encode scan.jpg --quality quality --bg-bpp 0.8 --output scan.djvu

# Encode a directory of images into a bundled DJVM with shared Djbz
djvu encode pages/ --output book.djvu --shared-dict-pages 2

# Embed TH44 color thumbnails while bundling (multi-page layered)
djvu encode pages/ --quality quality --thumbnails --output book.djvu

# Raw BZZ compression utilities
djvu bzz-encode notes.txt --output notes.bzz
djvu bzz-decode notes.bzz --output notes.txt
```

For single image input (PNG, JPEG, or TIFF), `--quality lossless`
luminance-thresholds the image into a JB2 mask and writes `INFO + Sjbz`;
`--quality quality` uses the layered encoder (`INFO + Sjbz + BG44...` plus
`FGbz` when colored foreground is detected) for color input. `--quality
archival` uses the same layered shape with a denser background sample grid.
Directory input supports all three profiles, and both directory paths share a
Djbz symbol dictionary across pages: `lossless` uses the shared-Djbz
multi-page JB2 path, while `quality` / `archival` bundle layered pages that
keep their own `Sjbz`, `BG44`, and optional `FGbz` chunks on top of the shared
dictionary. `--shared-dict-pages` sets the page-count threshold for promoting
a symbol into the shared dictionary on either path.

Layered `quality` / `archival` encodes default to fixed BT.601 thresholding.
`--binarization sauvola` opts into adaptive local thresholding for mixed or
uneven lighting; tune it with `--sauvola-window` and `--sauvola-k`.
`--bg-inpaint` fills fully masked background blocks from neighbouring unmasked
pixels, which can reduce dark boxes under heavy text strokes. These knobs are
opt-in, only affect layered profiles, and do not change lossless JB2 defaults.
Library callers can use the same controls with `PageEncoder::with_segment_options`.

## hOCR and ALTO XML export

```rust
use djvu_rs::{DjVuDocument, text_serialize::{to_hocr, to_alto, HocrOptions, AltoOptions}};

let data = std::fs::read("scanned.djvu")?;
let doc = DjVuDocument::parse(&data)?;

// hOCR — compatible with Tesseract, ABBYY, and most OCR toolchains
let hocr = to_hocr(&doc, &HocrOptions::default())?;
std::fs::write("output.hocr", hocr)?;

// ALTO XML — used by libraries and archives (DFG, Europeana, etc.)
let alto = to_alto(&doc, &AltoOptions::default())?;
std::fs::write("output.xml", alto)?;
```

## OCR recognition backends

The supported OCR recognition path is the `ocr-tesseract` feature, which uses a
system Tesseract installation and tessdata files. Recognized text is embedded
into the output document as a compressed `TXTz` text layer, page by page:

```sh
cargo build --features cli,ocr-tesseract
# Requires Tesseract + the requested language data, e.g. eng.traineddata.
djvu ocr scanned.djvu --backend tesseract --lang eng --output with-text.djvu
```

Library callers can attach recognized text at encode time instead, via
`PageEncoder::with_ocr_text_layer` (or `with_text_layer` for an existing
`TextLayer`).

`ocr-onnx` is an experimental library-level CTC helper; the CLI accepts
`--backend onnx --model <path>` but does not treat it as a stable backend
because no specific model family, preprocessing contract, or
fixture is guaranteed yet. `ocr-neural` is a placeholder only: `CandleBackend` now
returns a clear unsupported-backend error instead of constructing a backend that
always fails at recognition time. The compatibility feature name
`ocr-neural-candle` is a no-op and no longer pulls Candle/tokenizers into
`--all-features` builds.

## Serde support

Requires the `serde` feature flag: `djvu-rs = { version = "…", features = ["serde"] }`.

All public data types (`DjVuBookmark`, `TextZone`, `MapArea`, `PageInfo`, etc.) implement
`Serialize` and `Deserialize`.

```rust
use djvu_rs::DjVuDocument;

let data = std::fs::read("book.djvu")?;
let doc = DjVuDocument::parse(&data)?;

let json = serde_json::to_string_pretty(doc.bookmarks())?;
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

let decoder = DjVuDecoder::new(page)?.with_size(1200, 1600);
let img = DynamicImage::from_decoder(decoder)?;
img.save("page.png")?;
```

## EPUB export

Requires the `epub` feature flag: `djvu-rs = { version = "…", features = ["epub"] }`.

```rust
use djvu_rs::{DjVuDocument, epub::{djvu_to_epub, EpubOptions}};

let data = std::fs::read("book.djvu")?;
let doc = DjVuDocument::parse(&data)?;

let epub_bytes = djvu_to_epub(&doc, &EpubOptions::default())?;
std::fs::write("book.epub", epub_bytes)?;
```

CLI:

```sh
djvu render book.djvu --format epub --output book.epub
```

## WebAssembly

Build with [wasm-pack](https://rustwasm.github.io/wasm-pack/):

```sh
wasm-pack build --target bundler --features wasm
```

Then use in JavaScript/TypeScript:

```js
import init, { WasmDocument } from './pkg/djvu_rs.js';

await init();
const doc = WasmDocument.from_bytes(new Uint8Array(arrayBuffer));
console.log(doc.page_count());

const page = doc.page(0);
const pixels = page.render(150);   // Uint8ClampedArray, RGBA
const img = new ImageData(pixels, page.width_at(150), page.height_at(150));
ctx.putImageData(img, 0, 0);
```

See [`examples/wasm/`](examples/wasm/) for a complete drag-and-drop demo.

The generated npm package follows the Rust crate version; there is no separate
WASM release train. The local `pkg/` directory is ignored wasm-pack output, so
regenerate it from the checked-in `Cargo.toml` before publishing instead of
editing generated `pkg/package.json` by hand.

### WASM scalar vs simd128 benchmark

The local Node.js harness builds two `wasm-pack --target nodejs` bundles and
compares scalar wasm32 against `RUSTFLAGS="-C target-feature=+simd128"`:

```sh
ITERATIONS=50 WARMUP=10 DPI=150 ./scripts/bench_wasm_simd128.sh
```

The script uses `tests/fixtures/boy.djvu` by default and reports parse,
full-render, cached-render, and first progressive-render timings. CI
syntax-checks the harness and build-checks both wasm targets, but does not run
timing comparisons because hosted runner variance is too high for stable
regression gates.

## Feature flags

| Flag | Default | Description |
|------|---------|-------------|
| `std` | enabled | `DjVuDocument`, file I/O, rendering — the decode-only surface |
| `pdf` | disabled | PDF export via `djvu_to_pdf` (owns `miniz_oxide` + `jpeg-encoder`) |
| `cli` | disabled | Build the `djvu` command-line binary (implies `pdf` and `cbz`) |
| `cbz` | disabled | CBZ (comic-book ZIP) export — backs `render --format cbz` (owns `zip`) |
| `tiff` | disabled | TIFF export via the `tiff` crate |
| `async` | disabled | Async render API and lazy `AsyncRead + AsyncSeek` document loading |
| `parallel` | disabled | Parallel multi-page render via `rayon` (`render_pages_parallel`) |
| `jpeg` | disabled | Standalone JPEG decode without full `std` (JPEG is included in `std` by default) |
| `mmap` | disabled | Memory-mapped file I/O via `memmap2` (`MmapDocument::open`) |
| `serde` | disabled | `Serialize` + `Deserialize` for all public data types |
| `image` | disabled | `image::ImageDecoder` impl via `DjVuDecoder` — integrates with the `image` crate |
| `epub` | disabled | EPUB 3 export via `djvu_to_epub` — page images, text overlay, bookmarks as nav (owns `zip`) |
| `wasm` | disabled | WebAssembly bindings via `wasm-bindgen` (`WasmDocument`, `WasmPage`) |
| `wasm-lazy` | disabled | Lazy Range-based document loading in the browser: a JS `(offset, len)` reader fetches only the pages you open |
| `wasm-threads` | disabled | wasm32 thread pool (rayon via Web Workers); requires a nightly toolchain, not part of the stable CI gate |
| `ocr-tesseract` | disabled | OCR recognition via a system Tesseract installation (the supported OCR backend) |
| `ocr-onnx` | disabled | Experimental ONNX CTC recognition helper via `tract-onnx`; no stable model contract |
| `ocr-neural` | disabled | Placeholder backend only — `CandleBackend::load` returns a clear unsupported error |
| `ocr-neural-candle` | disabled | Deprecated no-op alias for `ocr-neural` |
| `experimental` | disabled | Experimental JB2 encoder paths used by internal example binaries |
| `iw44-probe` | disabled | IW44 encoder diagnostics probe (dev-only) |
| `alloc-profile` | disabled | dhat allocation-profiling harness for `examples/alloc_profile.rs` (dev-only) |

Without `std`, the crate provides IFF parsing, BZZ decompression, JB2/IW44 decoding,
text/annotation parsing — all codec primitives that work on byte slices.

## Performance

See [BENCHMARKS_RESULTS.md](BENCHMARKS_RESULTS.md) for Criterion numbers,
methodology, and a DjVuLibre comparison (run via
[`scripts/bench_djvulibre.sh`](scripts/bench_djvulibre.sh) +
[`scripts/djvulibre_compare.py`](scripts/djvulibre_compare.py)).
Historical multi-platform results are in [BENCHMARKS.md](BENCHMARKS.md).

Recent targeted experiments are recorded in
[PERF_EXPERIMENTS.md](PERF_EXPERIMENTS.md), including:

- **#233 lazy async loading:** a 100 MiB padded 520-page DJVM reached first
  pixel in **491.469 ms** while reading only **28,578 bytes** at simulated
  12.5 MiB/s throughput.
- **#189 x86-64-v3 AVX2 validation:** existing AVX2 decode paths showed
  `iw44_decode_corpus_color` **-18.88%** and `iw44_decode_first_chunk`
  **-4.85%** on GitHub-hosted x86_64, with one sub4 partial-decode regression
  recorded for follow-up.
- **#258 shared-Djbz clustering:** Hamming shared clustering was rejected as
  default; byte-exact shared-Djbz remains the measured safe path.

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
