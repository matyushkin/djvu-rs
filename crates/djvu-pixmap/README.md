# djvu-pixmap

RGBA and grayscale pixmap types used by DjVu rendering.

This crate is part of the `djvu-rs` workspace. It exposes `Pixmap` and
`GrayPixmap` so codec/render crates can avoid depending on the umbrella
`djvu-rs` crate.

The types build with `default-features = false` for no_std consumers.
