# djvu-jb2

Pure-Rust DjVu JB2 bilevel image decoder.

This crate is part of the `djvu-rs` workspace. It decodes `Sjbz` image streams
and `Djbz` shared dictionaries using the standalone `djvu-zp` arithmetic coder
and `djvu-bitmap` image type.

The decoder builds with `default-features = false` for no_std consumers.
