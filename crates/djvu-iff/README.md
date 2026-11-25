# djvu-iff

Pure-Rust DjVu IFF container parser and emitter.

This crate is part of the `djvu-rs` workspace. It provides the zero-copy
`parse_form` API plus an owned chunk-tree parser/emitter (`parse`/`emit`)
used by the umbrella crate's editing layers. Both report errors as
`IffError`.

The parser builds with `default-features = false` for no_std consumers.
