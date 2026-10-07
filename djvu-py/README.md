# djvu-rs for Python

[![PyPI](https://img.shields.io/pypi/v/djvu-rs)](https://pypi.org/project/djvu-rs/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/matyushkin/djvu-rs/blob/main/LICENSE)

Read, render, convert, and edit DjVu files from Python. Bindings for the
pure-Rust [djvu-rs](https://github.com/matyushkin/djvu-rs) library: no
DjVuLibre, no GPL, no system dependencies. Rendering and conversion release
the GIL.

| Your task | How |
|-----------|-----|
| Render a page to PIL / numpy | `page.render(dpi=150).to_pil()` / `.to_numpy()` — [rendering](https://github.com/matyushkin/djvu-rs/blob/main/djvu-py/GUIDE.md#rendering) |
| Extract text | `page.text()`, or `page.text_layer()` for words with coordinates |
| Convert DjVu → PDF, EPUB, CBZ, TIFF | `doc.write_pdf(path)` and friends — [export](https://github.com/matyushkin/djvu-rs/blob/main/djvu-py/GUIDE.md#export) |
| Build a viewer (previews, viewport crops) | `page.render(quality='coarse')`, `page.render(region=(x, y, w, h))` |
| Edit metadata, bookmarks, links, text layer | `djvu.Editor` — [editing](https://github.com/matyushkin/djvu-rs/blob/main/djvu-py/GUIDE.md#editing) |
| Create DjVu from images | Not in Python — use the [`djvu encode` CLI](https://github.com/matyushkin/djvu-rs/blob/main/docs/cli.md) or the Rust crate |

## Install

```bash
pip install djvu-rs                    # or djvu-rs[numpy,pillow]
```

Wheels for CPython 3.9–3.13 on manylinux/musllinux, macOS, and Windows. The
package version matches the `djvu-rs` crate version.

## Quick start

```python
import djvu_rs as djvu

doc = djvu.Document.open('scan.djvu')
print(f'{doc.page_count()} pages')

page = doc.page(0)
print(f'{page.width}x{page.height} @ {page.dpi} dpi')

page.render(dpi=150).to_pil().save('page.png')   # needs Pillow
text = page.text()                                # None if the page has no text layer

doc.write_pdf('scan.pdf')                         # selectable text, bookmarks, links

editor = djvu.Editor.open('scan.djvu')
editor.set_metadata({'title': 'Scan'})
editor.save('scan.djvu')                          # validated, atomic replace
```

Errors are typed: `DecodeError`, `IoError`, `ExportError`, `EditError`, and
`PageIndexError` (also an `IndexError`), all under `djvu_rs.Error`. The
package ships a type stub and `TypedDict` shapes for every edit value.

## Status & limitations

- **No encoding.** Creating DjVu from images, merge, split, and page
  insertion stay on the Rust crate / `djvu` CLI.
- **Bookmarks need a bundled multi-page file;** on a single-page file
  `set_bookmarks` raises `EditError`.
- **Legacy `BM44` / `PM44` pages are read-only:** they render, but editing
  them raises `EditError`.

## Documentation

| Topic | Where |
|-------|-------|
| Rendering, export options, editing, type hints, errors, build from source | [`GUIDE.md`](https://github.com/matyushkin/djvu-rs/blob/main/djvu-py/GUIDE.md) |
| Exact signatures | [`djvu_rs.pyi`](https://github.com/matyushkin/djvu-rs/blob/main/djvu-py/djvu_rs.pyi) |
| The Rust library, CLI, and WebAssembly bindings | [djvu-rs README](https://github.com/matyushkin/djvu-rs#readme) |
| Release and versioning contract | [`docs/packaging.md`](https://github.com/matyushkin/djvu-rs/blob/main/docs/packaging.md) |

MIT licensed. Written from the public DjVu v3 specification.
