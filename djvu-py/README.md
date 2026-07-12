# djvu-rs Python bindings

Python bindings for [djvu-rs](https://github.com/matyushkin/djvu-rs), a
pure-Rust DjVu decoder and encoder. The bindings cover the reading surface:
open documents, render pages (to PIL or numpy), and extract the text layer.
For conversion (PDF/EPUB/TIFF) and encoding, see the
[main project README](https://github.com/matyushkin/djvu-rs#readme).

## Install

Not published to PyPI yet — build from the repository checkout (requires a
Rust toolchain):

```bash
pip install ./djvu-py
# or, for development:
pip install maturin
cd djvu-py && maturin develop --release
```

## Version policy

The Python package follows the Rust crate version: it is built from the
`djvu-rs` Rust workspace at the same version, with no separate Python release
train. The same policy will apply once the package is published to PyPI.

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
