# Migration guide

This guide is for a crate that depends on djvu-rs 0.30 or later and moves to
a newer release. It lists the breaking changes of each release, with the code
before and after.

djvu-rs is at `0.x`. Cargo treats each `0.x` minor as its own compatibility
line: a requirement of `djvu-rs = "0.33"` never selects 0.34. Fixes, security
fixes included, go into the latest minor only ([`SECURITY.md`](../SECURITY.md)).

## How to read it

1. Find the release you depend on.
2. Read every section for a later release, oldest first. A release without a
   section has no breaking change.
3. Read [Fixes that change results](#fixes-that-change-results). These are
   bug fixes, so the changelog does not mark them as breaking, but your
   output can change.

| Release | It concerns code that |
|---------|-----------------------|
| [0.31.0](#0310) | matches on `djvu_encode::EncodeError` |
| [0.32.0](#0320) | matches on any djvu-rs error enum without a `_` arm |
| [0.33.0](#0330) | stores the reference returned by `DjVuPage::decoded_bg44`, `decoded_mask` or `decoded_fg44` |
| [0.34.0](#0340) | builds optimizer types with struct literals, or matches on `RewriteAction` |
| [0.36.0](#0360) | builds `MapArea` or `Annotation` with struct literals, or uses `Border::style` |
| [0.38.0](#0380) | renders pages that carry a rotation, uses `render_into` or `render_region`, writes metadata, exports PDF, or matches on `DjvmError` |
| [0.45.0](#0450) | builds with Rust older than 1.89 |
| [0.46.0](#0460) | enables the feature `ocr-neural` or `ocr-neural-candle`, or runs `djvu ocr --backend candle` |
| [1.0.0](#100) | calls an item with a deprecation note: a `render_*` function other than `render_pixmap`, `RenderOptions::scale`, `Pixmap::new` or `OcrExportError`; in the Python and npm packages, a render method other than `render` and `render_request`; builds an options struct such as `RenderOptions` with a struct literal; matches on an options enum without a `_` arm; uses the module name `bzz_new` or `iw44_new` |

The calls below did not change between 0.30 and 0.45. Code that uses nothing
else compiles after the version bump alone:

- `DjVuDocument::parse`, `parse_from_dir`, `page_count`, `page`, `bookmarks`,
  `metadata`, `component_directory`;
- `DjVuPage::width`, `height`, `dpi`, `rotation`, `text_layer`;
- `TextLayer::transform` and the `TextLayer`, `TextZone`, `TextZoneKind` and
  `text::Rect` types;
- `djvu_render::render_pixmap` and `RenderOptions::fit_to_box` (the result
  on a rotated page changed in [0.38.0](#0380));
- `DocumentEditor::apply` with `EditRequest` and `EditOperation::SetBookmarks`;
- the legacy `Document::open` and `Document::bookmarks`.

The three crates that depended on djvu-rs on crates.io in November 2026 use
exactly these calls. Each of them compiled against 0.45.0 with only the
version requirement changed
([#992](https://github.com/matyushkin/djvu-rs/issues/992)).

## 0.31.0

`djvu_encode::EncodeError` is `#[non_exhaustive]` and has a new variant,
`PageSource`. A `match` on it needs a `_` arm.

Before:

```rust,compile_fail
use djvu_rs::djvu_encode::EncodeError;

fn describe(error: &EncodeError) -> &'static str {
    match error {
        EncodeError::Unsupported(_) => "this input is not supported",
    }
}
```

After:

```rust
use djvu_rs::djvu_encode::EncodeError;

fn describe(error: &EncodeError) -> &'static str {
    match error {
        EncodeError::Unsupported(_) => "this input is not supported",
        EncodeError::PageSource(_) => "the page source failed",
        _ => "another encode error",
    }
}
```

## 0.32.0

Every public error enum is `#[non_exhaustive]`: `DocError`, `RenderError`,
`DjvmError`, `MutError`, `EditError`, `MetadataError` and the others. A
`match` that lists every variant needs a `_` arm. The variant names, their
data and the error messages did not change.

Before:

```rust,compile_fail
use djvu_rs::metadata::MetadataError;

fn describe(error: &MetadataError) -> &'static str {
    match error {
        MetadataError::InvalidUtf8 => "the metadata is not valid UTF-8",
    }
}
```

After:

```rust
use djvu_rs::metadata::MetadataError;

fn describe(error: &MetadataError) -> &'static str {
    match error {
        MetadataError::InvalidUtf8 => "the metadata is not valid UTF-8",
        _ => "another metadata error",
    }
}
```

## 0.33.0

`DjVuPage::decoded_bg44`, `decoded_bg44_partial`, `decoded_mask` and
`decoded_fg44` return `Option<Arc<T>>`. Before, they returned `Option<&T>`.
A call that only reads through the value compiles unchanged, because
`Arc<T>` dereferences to `T`. Only code that names the reference type has to
change.

Before:

```rust,compile_fail
use djvu_rs::{Bitmap, DjVuPage};

fn mask(page: &DjVuPage) -> Option<&Bitmap> {
    page.decoded_mask()
}
```

After:

```rust
use std::sync::Arc;

use djvu_rs::{Bitmap, DjVuPage};

fn mask(page: &DjVuPage) -> Option<Arc<Bitmap>> {
    page.decoded_mask()
}

// Reading through the value needs no change.
fn mask_width(page: &DjVuPage) -> Option<u32> {
    page.decoded_mask().map(|mask| mask.width)
}
```

## 0.34.0

The optimizer types `OptimizationRequest`, `RewriteAction`,
`RewrittenComponent`, `OptimizationPlan` and `OptimizationReport` are
`#[non_exhaustive]`:

- Build an `OptimizationRequest` with its constructors, not with a struct
  literal.
- Give a `match` on `RewriteAction` a `_` arm. The enum has two new
  variants, `ReencodeBackground` and `ReencodeMask`.
- `RewrittenComponent` no longer implements `Eq`. It still implements
  `PartialEq`.
- The JSON report has a new `"quality"` field per component and a new
  `"min_ssim"` field at the top level.

Before:

```rust,compile_fail
use djvu_rs::optimizer::{OptimizationPreset, OptimizationRequest};

fn request() -> OptimizationRequest {
    OptimizationRequest {
        preset: OptimizationPreset::LosslessCleanup,
        target_size: Some(1_000_000),
        max_ssim_loss: None,
    }
}
```

After:

```rust
use djvu_rs::optimizer::{OptimizationRequest, RewriteAction};

fn request() -> OptimizationRequest {
    OptimizationRequest::lossless_cleanup().with_target_size(1_000_000)
}

fn describe(action: RewriteAction) -> &'static str {
    match action {
        RewriteAction::RemoveFreeChunk => "removes a padding chunk",
        RewriteAction::ReencodeBackground => "re-encodes the background",
        RewriteAction::ReencodeMask => "re-encodes the mask",
        _ => "another rewrite",
    }
}
```

## 0.36.0

`MapArea` has two new public fields, `target` and `extra`. `Annotation` has
one new public field, `extra`. A struct literal must set them.

Before:

```rust,compile_fail
use djvu_rs::annotation::{MapArea, Rect, Shape};

fn link() -> MapArea {
    MapArea {
        url: "#12".to_string(),
        description: String::new(),
        shape: Shape::Rect(Rect { x: 10, y: 10, width: 200, height: 40 }),
        border: None,
        highlight: None,
    }
}
```

After:

```rust
use djvu_rs::annotation::{Annotation, MapArea, Rect, Shape};

fn link() -> MapArea {
    MapArea {
        url: "#12".to_string(),
        target: None,
        description: String::new(),
        shape: Shape::Rect(Rect { x: 10, y: 10, width: 200, height: 40 }),
        border: None,
        highlight: None,
        extra: Vec::new(),
    }
}

// `Annotation` implements `Default`; use it for the fields you do not set.
fn zoomed() -> Annotation {
    Annotation { zoom: Some(100), ..Annotation::default() }
}
```

`Border::style` now holds the whole DjVuLibre border option without its
parentheses. This compiles unchanged, so check each place that reads or
writes the field:

| Border in the file | `style` before 0.36.0 | `style` from 0.36.0 |
|--------------------|-----------------------|---------------------|
| `(border #FF0000)` | `#FF0000` | `border #FF0000` |
| `(xor)` | no border was read | `xor` |
| `(shadow_in 4)` | no border was read | `shadow_in 4` |

```rust
use djvu_rs::annotation::Border;

fn red_border() -> Border {
    // Before 0.36.0 this was `style: "#FF0000"`.
    Border { style: "border #FF0000".to_string() }
}
```

## 0.38.0

This release made a rotated page behave in one way everywhere. A page is
rotated when its `INFO` chunk carries a rotation, which
`DjVuPage::rotation` returns. If none of your documents has such a page,
only the metadata, `DjvmError` and PDF items below concern you.

### Fitted render size on a rotated page

On a page with a rotation of 90° or 270°, `RenderOptions::fit_to_width`,
`fit_to_height` and `fit_to_box` now fit the page as the reader sees it,
after the rotation. The `width` and `height` fields of the returned options
hold the size before the rotation, so their sides are swapped against the
output. Take the output size from the pixmap.

A page of 192 × 256 pixels with a 90° rotation, fitted into a box of
300 × 300, gives a pixmap of 225 × 300 before 0.38.0 and of 300 × 225 from
0.38.0.

```rust
use djvu_rs::DjVuPage;
use djvu_rs::djvu_render::{RenderError, RenderOptions, render_pixmap};

fn thumbnail_size(page: &DjVuPage) -> Result<(u32, u32), RenderError> {
    let options = RenderOptions::fit_to_box(page, 300, 300);
    let pixmap = render_pixmap(page, &options)?;
    // Not `(options.width, options.height)`: on a page with a 90° or 270°
    // rotation these two are swapped against the pixmap.
    Ok((pixmap.width, pixmap.height))
}
```

The same release changed these sizes for the same pages:

- `Page::size_at_dpi` (legacy API) and the WebAssembly `width_at` and
  `height_at` return the size after the rotation.
- A PDF page of a rotated DjVu page carries `/Rotate`.
- The `image` crate decoder `DjVuDecoder` decodes the page after the
  rotation. Its default size and `with_size` are sizes after the rotation.

### `render_into` refuses options that need the whole page

`render_into`, `render_into_with_limits` and `DjVuPage::render_into` return
`RenderError::UnsupportedOption` for anti-aliasing, for Lanczos-3 at a scaled
size, and for a rotation. Before, they ignored these options and wrote a
buffer without them. `RenderOptions::can_stream` tells in advance.

The example uses `RenderRequest::write_rgba`, which follows the same rule.
It replaces the two free functions, which [1.0.0](#100) removes.

```rust
use djvu_rs::DjVuPage;
use djvu_rs::djvu_render::{RenderError, RenderOptions, RenderRequest, render_pixmap};

fn rgba(page: &DjVuPage, options: &RenderOptions) -> Result<Vec<u8>, RenderError> {
    if options.can_stream(page) {
        let mut buffer = vec![0; options.width as usize * options.height as usize * 4];
        RenderRequest::new(options.clone()).write_rgba(page, &mut buffer)?;
        Ok(buffer)
    } else {
        // The pixmap can have another size than `options` asks for:
        // anti-aliasing halves it and a 90° rotation swaps its sides.
        Ok(render_pixmap(page, options)?.data)
    }
}
```

### Page regions

- `djvu_render::render_region` with `RenderOptions::aa` returns a crop of
  the anti-aliased page. That page is `width / 2 × height / 2`, and the
  region addresses it. Before, the function returned a crop of the full-size
  page without anti-aliasing.
- On a rotated page, the `x`, `y`, `w` and `h` arguments of the legacy
  `Page::render_region` (and of the Python `Page.render_region`) address the
  page after the rotation. Before, they addressed the page before the
  rotation.

A region of `RenderRequest` always addresses the page as `render_pixmap`
returns it, so it needs no special case:

```rust
use djvu_rs::{DjVuPage, Pixmap};
use djvu_rs::djvu_render::{RenderError, RenderOptions, RenderRect, RenderRequest};

fn top_left_tile(page: &DjVuPage, options: RenderOptions) -> Result<Pixmap, RenderError> {
    RenderRequest::new(options)
        .region(RenderRect { x: 0, y: 0, width: 256, height: 256 })
        .pixmap(page)
}
```

Anti-aliasing also halves progressive frames, `ProgressiveDecoder` frames
and the coarse render (`render_coarse`). The coarse render applies
Lanczos-3.

### Metadata setters return a `Result`

`DjVuDocumentMut::set_metadata` and `remove_metadata`, and the same two
methods of `PageMut`, return `Result<(), MutError>`. Before, they returned
nothing. They fail when the existing annotation chunk cannot be parsed
(`MutError::Annotation`), for an indirect multi-file document, and for a
legacy single-image file. A call that ignores the result still compiles,
with an `unused_must_use` warning, and loses the error.

```rust
use djvu_rs::djvu_mut::{DjVuDocumentMut, MutError};
use djvu_rs::metadata::DjVuMetadata;

fn set_title(bytes: &[u8], title: &str) -> Result<Vec<u8>, MutError> {
    let mut document = DjVuDocumentMut::from_bytes(bytes)?;
    let metadata = DjVuMetadata {
        title: Some(title.to_string()),
        ..DjVuMetadata::default()
    };
    // Before 0.38.0: `document.set_metadata(&metadata);`
    document.set_metadata(&metadata)?;
    Ok(document.into_bytes())
}
```

### `DjvmError::ComponentDescriptorCountMismatch` is removed

No writer can produce this error any more. Delete the `match` arm that
names it.

Before:

```rust,compile_fail
use djvu_rs::djvm::DjvmError;

fn is_caller_mistake(error: &DjvmError) -> bool {
    matches!(
        error,
        DjvmError::EmptyMerge | DjvmError::ComponentDescriptorCountMismatch { .. }
    )
}
```

After:

```rust
use djvu_rs::djvm::DjvmError;

fn is_caller_mistake(error: &DjvmError) -> bool {
    matches!(error, DjvmError::EmptyMerge)
}
```

`DjvmStreamWriter::add_component` now ignores the name bit (`0x80`) and the
title bit (`0x40`) of the component flags, and writes an unknown component
type as a shared component.

### PDF export fails on a page that cannot be rendered

`djvu_to_pdf` and the other PDF export functions return an error for a
document with a page that cannot be rendered. Before, they wrote a blank
page in its place and returned `Ok`. If a partial PDF is what you want,
export the good pages yourself.

## 0.45.0

The minimum supported Rust version is 1.89. Before, it was 1.88. Build with
Rust 1.89 or newer. If your crate declares `rust-version`, raise it to at
least `1.89`.

## 0.46.0

The features `ocr-neural` and `ocr-neural-candle` are removed, together with
the module `ocr_neural` and its type `CandleBackend`. They were a
placeholder: `CandleBackend::load` always returned `OcrError::InitFailed`,
so no working code can depend on them.

Remove the feature from your `Cargo.toml`. Cargo refuses a requirement that
names a feature the crate does not have.

Before:

```toml
djvu-rs = { version = "0.45", features = ["ocr-neural"] }
```

After:

```toml
djvu-rs = { version = "0.46", features = ["ocr-tesseract"] }
```

`ocr-tesseract` is the supported OCR backend; it needs a system Tesseract
installation. `ocr-onnx` is the experimental neural backend; it needs Rust
1.91 and the models that `scripts/fetch_ocr_models.sh` fetches.

The `djvu ocr` command no longer accepts `--backend candle`. That value
only printed an error. Use `--backend tesseract` or `--backend onnx`.

## 1.0.0

This release makes two kinds of change.

**Removals.** It removes every item that carried a deprecation note. Each
Rust replacement shipped in 0.39.0 or earlier, and each replacement in the
Python and npm packages in 0.42.0 or earlier, so you can make the change
before the upgrade. Build your crate against 0.46 first: each deprecation
warning names the call that replaces the old one.

**Types that can grow.** It marks the options structs, the report structs
and the options enums `#[non_exhaustive]`, so that a later 1.x release can
add a field or a variant without a break. No warning in 0.46 announces this
change. The sections
[Options structs are built with `with_*` methods](#options-structs-are-built-with-with_-methods)
and [A `match` on an options enum needs a `_` arm](#a-match-on-an-options-enum-needs-a-_-arm)
give a form that compiles on both 0.46 and 1.0.

### The `render_*` variants are removed

`djvu_render::RenderRequest` replaces ten functions of `djvu_render`. A
request holds the options, an optional region, the quality, the resource
limits and a cancel token. One of its methods then picks the output.
`render_pixmap`, `render_gray8`, `render_progressive_all` and the method
`DjVuPage::render_into` stay.

| Removed | Replacement |
|---------|-------------|
| `render_into(page, &opts, buf)` | `RenderRequest::new(opts).write_rgba(page, buf)` |
| `render_into_with_limits(page, &opts, limits, buf)` | `RenderRequest::new(opts).limits(limits).write_rgba(page, buf)` |
| `render_pixmap_with_limits(page, &opts, limits)` | `RenderRequest::new(opts).limits(limits).pixmap(page)` |
| `render_pixmap_with_report(page, &opts)` | `RenderRequest::new(opts).pixmap_with_report(page)` |
| `render_streaming(page, &opts, sink)` | `RenderRequest::new(opts).rows(page, sink)` |
| `render_region(page, region, &opts)` | `RenderRequest::new(opts).region(region).pixmap(page)` |
| `render_region_tiled(page, region, &opts)` | `RenderRequest::new(opts).region(region).cached(true).pixmap(page)` |
| `render_coarse(page, &opts)` | `RenderRequest::new(opts).quality(Quality::Coarse).pixmap(page)` |
| `render_progressive(page, &opts, n)` | `RenderRequest::new(opts).quality(Quality::Step(n)).pixmap(page)` |
| `render_progressive_step(page, &opts, n)` | `RenderRequest::new(opts).quality(Quality::Step(n)).pixmap(page)` |

`RenderRequest::new` takes the options by value: clone them if you need
them again. The request methods take `&self`, so one request can render
many pages.

The pixels are the same. Four details differ:

- **A region on a rotated page.** The removed `render_region` and
  `render_region_tiled` took a rectangle on the page before its rotation.
  `RenderRequest::region` takes a rectangle on the page as `render_pixmap`
  returns it, after the rotation. On a page without a rotation the two are
  equal. Otherwise convert the rectangle with the table below.
- **A coarse render of a page without a background.** `render_coarse`
  returned `Ok(None)`. The request returns `Err(RenderError::NoBackground)`.
- **A progressive step on a page without background chunks.**
  `render_progressive` and `render_progressive_step` returned the full
  render for every step. `Quality::Step` accepts step 0 only there, and
  returns `RenderError::ChunkOutOfRange` for a larger step. Take the number
  of steps from `progressive_steps(page)`.
- **The operation name in a resource-limit error.** The field
  `ResourceLimitExceeded::operation` holds `"render"` for every request.
  Before, it held the name of the function, for example `"render_into"`.

The rotation in this table is the rotation of the output: the rotation of
the `INFO` chunk plus `RenderOptions::rotation`. `W` and `H` are the width
and height of the page before the rotation. The old rectangle is
`(x, y, w, h)`.

| Rotation of the output | Rectangle for `RenderRequest::region` |
|------------------------|----------------------------------------|
| none | `(x, y, w, h)` |
| 90° clockwise | `(H − y − h, x, h, w)` |
| 180° | `(W − x − w, H − y − h, w, h)` |
| 90° counter-clockwise | `(y, W − x − w, h, w)` |

Before:

```rust,compile_fail
use djvu_rs::djvu_render::{render_coarse, render_progressive, render_region};
```

After:

```rust
use djvu_rs::{DjVuPage, Pixmap};
use djvu_rs::djvu_render::{
    Quality, RenderError, RenderOptions, RenderRect, RenderRequest, progressive_steps,
};

/// A fast blurry preview; `None` for a page without a background.
fn preview(page: &DjVuPage, options: &RenderOptions) -> Result<Option<Pixmap>, RenderError> {
    let request = RenderRequest::new(options.clone()).quality(Quality::Coarse);
    match request.pixmap(page) {
        Ok(pixmap) => Ok(Some(pixmap)),
        Err(RenderError::NoBackground) => Ok(None),
        Err(error) => Err(error),
    }
}

/// Every progressive frame, the least detailed first.
fn frames(page: &DjVuPage, options: &RenderOptions) -> Result<Vec<Pixmap>, RenderError> {
    (0..progressive_steps(page))
        .map(|step| {
            RenderRequest::new(options.clone())
                .quality(Quality::Step(step))
                .pixmap(page)
        })
        .collect()
}

/// A part of the page, as a viewer shows it while the reader pans.
fn viewport(page: &DjVuPage, options: &RenderOptions, part: RenderRect) -> Result<Pixmap, RenderError> {
    RenderRequest::new(options.clone())
        .region(part)
        .cached(true)
        .pixmap(page)
}
```

### `RenderOptions::scale` is removed

The renderer ignored this field since 0.20.1: it takes the scale from
`width` and the width of the page. Do not set it. Code that uses
`RenderOptions::fit_to_width`, `fit_to_height` or `fit_to_box` needs no
change. A `RenderOptions` literal changes in any case: see
[Options structs are built with `with_*` methods](#options-structs-are-built-with-with_-methods).

```rust
use djvu_rs::djvu_render::RenderOptions;

fn options() -> RenderOptions {
    // Before 1.0.0 a literal could also carry `scale: 1.0`.
    RenderOptions::new(800, 1000)
}
```

### `Pixmap::new` is removed

`Pixmap::new` returned an empty pixmap of 0 × 0 when the requested size was
too large. `Pixmap::try_new` (since 0.34.0) takes the same arguments and
reports that case as a `PixmapError`. `Pixmap::white` stays.

```rust
use djvu_rs::{Pixmap, PixmapError};

fn red_canvas(width: u32, height: u32) -> Result<Pixmap, PixmapError> {
    // Before: `Pixmap::new(width, height, 255, 0, 0, 255)`.
    Pixmap::try_new(width, height, 255, 0, 0, 255)
}
```

The same holds for the crate `djvu-pixmap`, which defines the type.

### `OcrExportError` is removed

`text_serialize::OcrExportError` was an alias of
`text_serialize::TextSerializeError` since 0.21.0. Use the name
`TextSerializeError`.

### The old render methods of the Python and npm packages are removed

The Python package and the npm (WebAssembly) package each keep one render
call that takes a region and a quality. The older methods are removed. In
0.46 the Python methods emit a `DeprecationWarning`, and the JavaScript
methods carry a `@deprecated` note.

Python, where `page` is a `djvu_rs.Page`. The replacement shipped in 0.40.0.

| Removed | Replacement |
|---------|-------------|
| `page.render_region(x, y, w, h)` | `page.render(region=(x, y, w, h))` |
| `page.render_region(x, y, w, h, full_width=fw, full_height=fh)` | `page.render(size=(fw, fh), region=(x, y, w, h))` |
| `page.render_coarse(dpi)` | `page.render(dpi, quality="coarse")` |
| `page.render_progressive(n, dpi)` | `page.render(dpi, quality=n)` |

`render_coarse` returned `None` for a page without a background.
`render(quality="coarse")` raises `DecodeError` for such a page. Read
`page.bg44_chunk_count` first when your code handles bilevel pages:

```python
if page.bg44_chunk_count > 0:
    preview = page.render(dpi=72, quality="coarse")
else:
    preview = page.render(dpi=72)
```

JavaScript, where `page` is a `WasmPage`. `WasmRenderRequest` is in the npm
package since 0.42.0.

| Removed | Replacement |
|---------|-------------|
| `page.render_coarse(dpi)` | `req.set_coarse(); page.render_request(req, pm)` |
| `page.render_progressive(dpi, n)` | `req.set_step(n); page.render_request(req, pm)` |
| `page.render_into_pixmap(dpi, pm)` | `page.render_request(new WasmRenderRequest(dpi), pm)` |
| `page.render_progressive_into_pixmap(dpi, n, pm)` | `req.set_step(n); page.render_request(req, pm)` |

```js
const req = new WasmRenderRequest(150);
const pm = new WasmPixmap();
req.set_coarse();
page.render_request(req, pm);
const pixels = pm.to_bytes(); // Uint8ClampedArray, as render_coarse returned
```

`render_coarse` and `render_progressive` returned a `Uint8ClampedArray`.
`render_request` fills a `WasmPixmap`: `pm.to_bytes()` gives an owned copy,
and `pm.view()` gives a view without a copy. `render_coarse` returned
`undefined` for a page without a background. `render_request` throws for
such a page; read `page.bg44_chunk_count()` first. `page.render(dpi)` and the
tile methods did not change.

### Options structs are built with `with_*` methods

These structs are `#[non_exhaustive]`. Rust refuses a struct literal of such
a type outside the crate that defines it, and it also refuses
`..Default::default()`:

| Crate | Structs |
|-------|---------|
| `djvu-rs` | `RenderOptions`, `PdfOptions`, `EpubOptions`, `CbzOptions`, `TiffOptions`, `SegmentOptions`, `ResourceLimits`, `ParseOptions`, `ValidateOptions`, `IngestPolicy`, `TileRenderControls`, `HocrOptions`, `AltoOptions`, `OcrOptions`, `CacheBudgetOptions` |
| `djvu-jb2` | `Jb2EncodeOptions` |
| `djvu-iw44` | `Iw44EncodeOptions` |

Each of them has a `Default` value and one `with_<field>` method per field.
The method takes the value of the field and returns the struct, so the calls
chain. `RenderOptions::new(width, height)` sets the two fields that every
render needs. Reading a field did not change, and neither did assigning to
one (`options.aa = true`).

Before:

```rust,compile_fail
use djvu_rs::djvu_render::{RenderOptions, Resampling};

fn options() -> RenderOptions {
    RenderOptions {
        width: 800,
        height: 1000,
        aa: true,
        resampling: Resampling::Lanczos3,
        ..Default::default()
    }
}
```

After:

```rust
use djvu_rs::djvu_render::{RenderOptions, Resampling};
use djvu_rs::pdf::PdfOptions;

fn options() -> RenderOptions {
    RenderOptions::new(800, 1000)
        .with_aa(true)
        .with_resampling(Resampling::Lanczos3)
}

fn pdf_options() -> PdfOptions {
    PdfOptions::default().with_output_dpi(150)
}
```

This form compiles on 0.46 and on 1.0, if you want one source for both:

```rust
use djvu_rs::djvu_render::RenderOptions;

fn options() -> RenderOptions {
    let mut options = RenderOptions::default();
    options.width = 800;
    options.height = 1000;
    options.aa = true;
    options
}
```

Two methods of `Jb2EncodeOptions` had these names already, as constructors.
They are now methods on a value, like the others, and `with_despeckle` takes
the `Option` that the field holds:

| Before | After |
|--------|-------|
| `Jb2EncodeOptions::with_lossy_threshold(0.04)` | `Jb2EncodeOptions::default().with_lossy_threshold(0.04)` |
| `Jb2EncodeOptions::with_despeckle(2)` | `Jb2EncodeOptions::default().with_despeckle(Some(2))` |

```rust
use djvu_rs::jb2_encode::Jb2EncodeOptions;

fn options() -> Jb2EncodeOptions {
    Jb2EncodeOptions::default()
        .with_lossy_threshold(0.04)
        .with_despeckle(Some(2))
}
```

### Report structs cannot be built outside the crate

These structs are results that djvu-rs returns. They are `#[non_exhaustive]`
too, so your code can read their fields but cannot build one with a struct
literal, and a pattern that names their fields needs `..`:

| Crate | Structs |
|-------|---------|
| `djvu-rs` | `RenderReport`, `RenderRecovery`, `ValidationReport`, `ValidationSummary`, `Finding`, `ResourceEstimate`, `QualityReport`, `ColorQualityReport`, `ResourceLimitExceeded`, `SavePatchStats`, `RewriteItem`, `TileCacheUsage`, `OptimizationResult`, `PlaneDiff`, `SemanticDiff`, `EditPlan`, `PlannedEdit`, `ComponentNode`, `ComponentDedup`, `PageRemoval`, `IndirectDocument`, `SegmentedPage` |
| `djvu-jb2` | `CcStats`, `EncodedBlit`, `SymbolBox` |
| `djvu-iff` | `ChunkRecord` |

If a test of yours builds one of them by hand, get the value from the
function that returns it.

The data structs stay as they were, and a struct literal still builds them:
`Rect`, `Color`, `RenderRect`, `TileRect`, `Pixmap`, `GrayPixmap`, `Bitmap`,
`FgbzColor`, `PageInfo`, `TextLayer`, `TextZone`, `Paragraph`,
`DjVuBookmark`, `DjVuMetadata`, `Highlight`, `Border`, `EditRequest`,
`ComponentId`, `ComponentDirectoryEntry`, `EncodedChunk`, `InfoChunk` and
`DjvuFile`. (`MapArea` and `Annotation` have constructors since
[0.36.0](#0360).)

### A `match` on an options enum needs a `_` arm

These enums are `#[non_exhaustive]`. A `match` on one of them outside its
crate needs a `_` arm. Naming a variant to build a value did not change.

| Module | Enums |
|--------|-------|
| `djvu_render` | `Resampling`, `RecoveredLayer` |
| `djvu_encode` | `EncodeQuality`, `BilevelCodec`, `FgbzPaletteOptions` |
| `iw44_encode` (crate `djvu-iw44`) | `Iw44Target` |
| `segment` | `Binarization` |
| `ingest` | `AlphaCompositing`, `IccHandling`, `DepthDownconversion` |
| `tiff_export` | `TiffMode`, `TiffBilevelCompression` |
| `validate` | `Severity`, `Layer` |
| `resource_limits` | `ResourceLimitAxis` |
| `optimizer` | `OptimizationPreset` |
| `editor` | `EditOperation`, `EditOperationKind`, `EditTarget` |
| `djvm` | `UnreachablePolicy`, `DjvmSpool` |
| crate root | `ComponentNodeKind`, `ThumbnailStrategy` |

Before:

```rust,compile_fail
use djvu_rs::validate::Severity;

fn label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Tolerated => "tolerated",
        Severity::Recovery => "recovery",
    }
}
```

After (this form also compiles on 0.46, with a warning about an arm that
cannot be reached):

```rust
use djvu_rs::validate::Severity;

fn label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Tolerated => "tolerated",
        Severity::Recovery => "recovery",
        _ => "other",
    }
}
```

These enums stay exhaustive, because the DjVu format or plain geometry
closes them: `TextZoneKind`, `Rotation`, `UserRotation`,
`annotation::Shape`, `ComponentKind`, `PlaneStatus` and `iff::Chunk`.

### The module names `bzz_new` and `iw44_new` are removed

`djvu_rs::bzz_new` and `djvu_rs::iw44_new` were second names of the modules
`djvu_rs::bzz` and `djvu_rs::iw44` since 0.23.0. Use the short name; the
items in the module did not change.

```rust
// Before: `use djvu_rs::iw44_new::Iw44Image;`
use djvu_rs::iw44::Iw44Image;

fn empty() -> Iw44Image {
    Iw44Image::new()
}
```

## Fixes that change results

These releases have no breaking change in the API. They fix decoding
errors, so the same file can give another result. Update stored reference
images, hashes and expected texts after the upgrade.

| Release | What changed | Who sees it |
|---------|--------------|-------------|
| 0.35.1 | Background and foreground images decode as in DjVuLibre: half-resolution chroma, small images, the scaling of a low-resolution background. | Pixel values of some rendered pages. In a thumbnail render of the test corpus of this repository, 2 files of 54. |
| 0.35.2 | `TextZone::text` of a zone that follows another zone on the same level is correct. Before, such zones had the wrong text or none, mostly in files written by DjVuLibre. | Code that reads `TextZone::text`. In the test corpus, 723 pages in 14 files of 54. `TextLayer::text` and the zone rectangles of these files did not change. |
| 0.35.2 | The text layer encoder writes version 1, which DjVuLibre accepts. Files that an older djvu-rs wrote carry version 0 and the old zone offsets. | Save the text layer again to repair such a file. |
| 0.35.2 | `DjVuDocument::metadata` also reads the shared annotation of a multi-page file, where DjVuLibre stores the metadata. | A file that gave `Ok(None)` can now give `Ok(Some(_))`. |

## Keeping this guide current

A change that breaks the API adds a section here in the same pull request,
under the release it will ship in: the next minor while the crate is `0.x`.
The test `tests/migration_sync.rs` fails when `CHANGELOG.md` lists breaking
changes for a release that has no section here, so a release cannot ship
without one. Every `rust` code block in this file is compiled as a doctest:
a block marked `compile_fail` must fail to build, every other block must
build.
