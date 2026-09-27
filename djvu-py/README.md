# djvu-rs Python bindings

Python bindings for [djvu-rs](https://github.com/matyushkin/djvu-rs), a
pure-Rust DjVu decoder and encoder. The bindings cover reading, export and
editing: open documents, render pages (to PIL or numpy), extract the text
layer, convert a whole document to PDF, EPUB, CBZ or TIFF, and change the
annotations, text layer, metadata and bookmarks of an existing file.

Encoding new DjVu files is **not** exposed here — use the
[Rust crate](https://crates.io/crates/djvu-rs) or the `djvu` CLI. See
[docs/packaging.md](../docs/packaging.md) for the release contract.

## Install

```bash
pip install djvu-rs
```

Wheels are published for manylinux/musllinux, macOS, and Windows (CPython
3.9–3.13). The package version matches the `djvu-rs` crate version.

### Build from source

Requires a Rust toolchain (see repository `rust-version`) and maturin:

```bash
pip install ./djvu-py
# or, for development:
pip install maturin
cd djvu-py && maturin develop --release
```

## Version policy

The Python package follows the Rust crate version: maturin reads the version
from `djvu-py/Cargo.toml`, which must match the workspace crate. There is no
separate Python release train.

## Typed errors

| Exception | Meaning |
|-----------|---------|
| `djvu_rs.Error` | Base class |
| `djvu_rs.DecodeError` | Parse / decode / render failure |
| `djvu_rs.IoError` | Filesystem failure from `Document.open` |
| `djvu_rs.ExportError` | PDF / EPUB / CBZ / TIFF conversion failure |
| `djvu_rs.EditError` | An edit or `Editor.save` the file cannot take |
| `djvu_rs.PageIndexError` | Out-of-range page index (also an `IndexError`) |

## Usage

```python
import djvu_rs as djvu

doc = djvu.Document.open('scan.djvu')
print(f'{doc.page_count()} pages')

page = doc.page(0)
print(f'{page.width}x{page.height} @ {page.dpi} dpi')

# Render to PIL Image
img = page.render(dpi=150).to_pil()
img.save('page.png')

# Render to numpy array
arr = page.render(dpi=150).to_numpy()
print(arr.shape)  # (height, width, 4)

# Extract text
text = page.text()
if text:
    print(text)
```

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
