# djvu-rs Python bindings

Python bindings for [djvu-rs](https://github.com/matyushkin/djvu-rs), a
pure-Rust DjVu decoder and encoder. The bindings cover reading and export:
open documents, render pages (to PIL or numpy), extract the text layer, and
convert a whole document to PDF, EPUB, CBZ or TIFF.

Encode and document mutation are **not** exposed here — use the
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
