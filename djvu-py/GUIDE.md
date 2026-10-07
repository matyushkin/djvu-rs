# djvu-rs Python guide

Reference for the `djvu_rs` package beyond the [README quick start](README.md#quick-start).
The authoritative signatures are in the type stub [`djvu_rs.pyi`](djvu_rs.pyi).
Every ```` ```python ```` block here and in the README runs in CI
(`tests/test_doc_examples.py`), so the examples cannot drift from the module.

## Rendering

`Page.render` covers every render shape through keyword arguments:

```python
import djvu_rs as djvu

page = djvu.Document.open('scan.djvu').page(0)

page.render()                                    # native resolution
page.render(dpi=150)                             # scaled by DPI
page.render(size=(1200, 1600))                   # exact output size
page.render(dpi=150, region=(0, 400, 800, 600))  # (x, y, w, h) crop of that render
page.render(dpi=72, quality='coarse')            # fast preview
page.render(dpi=150, quality=0)                  # background chunks 0..=n (progressive)
```

A `region` is in the displayed (rotated) orientation and is served from the
tile cache. `quality='coarse'` raises `DecodeError` on a page without a
background, and a chunk index must be below `page.bg44_chunk_count`. The
result is a `Pixmap`: `.to_pil()`, `.to_numpy()` (shape `(height, width, 4)`,
`uint8`), their `_zerocopy` variants backed by the same buffer, `.data()` for
raw RGBA bytes, and the buffer protocol (`memoryview`).
The old `render_region`, `render_coarse` and `render_progressive` still work
but emit a `DeprecationWarning`.

## Export

Every format has two forms. `to_x()` returns the bytes; `write_x(path)`
streams the same output into a file and holds only one page at a time. Use
`write_x` for a long book. Both release the GIL for the whole conversion.

```python
doc = djvu.Document.open('scan.djvu')

# PDF with an invisible text layer, the bookmarks and the links.
doc.write_pdf('scan.pdf', dpi=300, jpeg_quality=85)
pdf_bytes = doc.to_pdf()                       # same output, in memory

# EPUB 3.
doc.write_epub('scan.epub', title='Scan', author='Nobody', dpi=150)

# CBZ: one PNG per page. `pages` picks a subset, 0-based.
doc.write_cbz('scan.cbz', dpi=200, pages=[0, 1, 2])

# Multi-page TIFF. 'bilevel' with 'g4' is the archival choice for scans.
doc.write_tiff('scan.tiff', mode='bilevel', bilevel_compression='g4')
```

| Method | Main arguments |
|---|---|
| `to_pdf` / `write_pdf` | `dpi=150`, `jpeg_quality=80`, `adaptive=False`, `ccitt_g4=False`, `mrc=False` |
| `to_epub` / `write_epub` | `dpi=150`, `title`, `author`, `language='en'`, `modified=None`, `reflowable_text=False`, `jpeg_quality=None`, `adaptive=False` |
| `to_cbz` / `write_cbz` | `dpi=150`, `rotation=0`, `pages=None` |
| `to_tiff` / `write_tiff` | `mode='color'`, `scale=1.0`, `bilevel_compression='deflate'` |

`dpi=0` in `to_pdf` means the page's own resolution — the largest and slowest
output. `jpeg_quality=None` gives lossless page images and larger files.
`adaptive=True` encodes each page both ways and keeps the smaller one.

## Editing

`Editor` changes the annotations, text layer, metadata and bookmarks of an
existing file. Every value is a plain dict or list in the same shape as the
Rust models' serde form, so you read a value, change it, and write it back.

```python
editor = djvu.Editor.open('book.djvu')

# Metadata: missing keys become None. DjVuLibre tools such as
# `djvused -e print-meta` see the result.
editor.set_metadata({'title': 'Atlas', 'extra': [('isbn', '978-0')]})

# Annotations of page 0: a dict of page settings plus a list of map areas.
# Annotation rectangles use DjVu coordinates (origin at the bottom left).
annotation, areas = editor.page_annotations(0) or ({}, [])
areas.append({
    'url': 'https://example.org',
    'description': 'Example',
    'shape': {'Rect': {'x': 10, 'y': 20, 'width': 100, 'height': 30}},
})
editor.set_page_annotations(0, annotation, areas)

# Bookmarks (bundled multi-page files only). [] removes them.
editor.set_bookmarks([{'title': 'Chapter 1', 'url': '#1'}])

# Text layer: text plus a zone tree; zone rectangles have a top-left origin.
layer = editor.page_text_layer(0)

editor.save('book.djvu')          # validates, then replaces the file atomically
data = editor.to_bytes()          # or keep the result in memory
```

| Method | Effect |
|---|---|
| `Editor.open(path)` / `Editor.from_bytes(data)` | Start an edit session |
| `metadata()` / `set_metadata(dict)` / `remove_metadata()` | Document metadata |
| `bookmarks()` / `set_bookmarks(list)` | Document outline |
| `page_annotations(i)` / `set_page_annotations(i, annotation, areas=None)` / `remove_page_annotations(i)` | Page annotations and links |
| `page_text_layer(i)` / `set_page_text_layer(i, layer)` / `remove_page_text_layer(i)` | Page text layer |
| `modified` | `True` after any change |
| `document()` | A read-only `Document` of the current, unsaved state |
| `to_bytes()` / `save(path)` | Write the result |

The getters show unsaved changes. `Document.metadata()`, `Document.bookmarks()`,
`Page.annotations()` and `Page.text_layer()` return the same shapes for a
read-only document. A getter returns `None` (or `[]` for bookmarks) when the
file has no such data.

A dict of the wrong shape raises `ValueError` and leaves the document
unchanged. An out-of-range page raises `PageIndexError`. Legacy `BM44` /
`PM44` pages and bookmarks on a single-page file raise `EditError`.

## Type hints

The package ships a type stub (`djvu_rs.pyi`) and a `py.typed` marker, so
mypy, pyright and IDEs check calls and complete names. The dict shapes above
have `TypedDict` types: `Metadata`, `Bookmark`, `Annotation`, `MapArea`,
`Shape`, `TextLayer`, `TextZone` and others. The editor also accepts the
`...Input` variants, where keys left out take their defaults.

These types exist only for type checkers. Import them under `TYPE_CHECKING`:

```python
from __future__ import annotations
from typing import TYPE_CHECKING

import djvu_rs as djvu

if TYPE_CHECKING:
    from djvu_rs import Metadata

def title(path: str) -> str | None:
    meta: Metadata | None = djvu.Document.open(path).metadata()
    return meta["title"] if meta else None
```

A shape is a dict with one key; `if "Rect" in shape:` narrows it to
`RectShape`. `make py-stubtest` checks the stub against the built module.

## Typed errors

| Exception | Meaning |
|-----------|---------|
| `djvu_rs.Error` | Base class |
| `djvu_rs.DecodeError` | Parse / decode / render failure |
| `djvu_rs.IoError` | Filesystem failure from `Document.open` |
| `djvu_rs.ExportError` | PDF / EPUB / CBZ / TIFF conversion failure |
| `djvu_rs.EditError` | An edit or `Editor.save` the file cannot take |
| `djvu_rs.PageIndexError` | Out-of-range page index (also an `IndexError`) |

## Build from source

Requires a Rust toolchain (see repository `rust-version`) and maturin:

```bash
pip install ./djvu-py
# or, for development:
pip install maturin
cd djvu-py && maturin develop --release
```

The package version is read from `djvu-py/Cargo.toml`, which must match the
`djvu-rs` crate version; there is no separate Python release train. See
[`docs/packaging.md`](../docs/packaging.md) for the release contract.
