# djvu-iff

Pure-Rust DjVu IFF container parser and emitter.

This crate is part of the `djvu-rs` workspace. It provides the zero-copy
`parse_form` API plus the legacy tree parser/emitter used by the umbrella
crate.

The parser builds with `default-features = false` for no_std consumers.
