# djvu-bitmap

Packed 1-bit bitmap type used by DjVu codecs.

This crate is part of the `djvu-rs` workspace. It exposes the `Bitmap` type
used by JB2, SMMR, render, and export code while allowing codec crates to avoid
depending on the umbrella `djvu-rs` crate.

The type builds with `default-features = false` for no_std consumers.
