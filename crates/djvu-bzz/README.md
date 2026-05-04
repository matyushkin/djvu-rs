# djvu-bzz

Pure-Rust DjVu BZZ compressor and decompressor.

This crate is part of the `djvu-rs` workspace. It exposes the BZZ codec used
by DjVu metadata chunks such as DIRM, NAVM, ANTz, TXTz, and FGbz.

## Features

- `std` (default) enables BZZ encoding.
- `parallel` enables parallel inverse-BWT decoding for multi-block streams.

The decoder builds with `default-features = false` for no_std consumers that
only need decompression.
