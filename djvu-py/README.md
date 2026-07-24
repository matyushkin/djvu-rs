# djvu-rs Python bindings

Python bindings for [djvu-rs](https://github.com/matyushkin/djvu-rs), a
pure-Rust DjVu decoder and encoder. The bindings cover the reading surface:
open documents, render pages (to PIL or numpy), and extract the text layer.

Encode, document mutation, and PDF/EPUB/TIFF/CBZ export are **not** exposed
here — use the [Rust crate](https://crates.io/crates/djvu-rs) or the `djvu`
CLI. See [docs/packaging.md](../docs/packaging.md) for the release contract.

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
