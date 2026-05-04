# djvu-iw44

Pure-Rust DjVu IW44 wavelet image decoder.

This crate is part of the `djvu-rs` workspace. It decodes `BG44`, `FG44`, and
`TH44` chunks using the standalone `djvu-zp` arithmetic coder and returns
`djvu-pixmap` images.

The decoder builds with `default-features = false` for no_std consumers.
