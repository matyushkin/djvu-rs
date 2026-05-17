# djvu-rs Python bindings

Python bindings for [djvu-rs](https://github.com/matyushkin/djvu-rs), a pure-Rust DjVu decoder.

## Install

```bash
pip install djvu-rs
```

## Version policy

The Python package follows the Rust crate version. For example, Python package
`djvu-rs==0.18.0` is built from the `djvu-rs` Rust workspace at version
`0.18.0`. There is no separate Python release train.

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
