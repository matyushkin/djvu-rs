# djvu-zp

Pure-Rust clean-room implementation of the **ZP adaptive binary arithmetic
coder** specified in the [DjVu v3 specification](https://www.sndjvu.org/spec.html).

ZP is the entropy coder underlying every codec in a DjVu file:
[BZZ](https://crates.io/crates/djvu-rs) (text-chunk compression),
[JB2](https://crates.io/crates/djvu-rs) (bilevel image),
and [IW44](https://crates.io/crates/djvu-rs) (wavelet color/grey image).

This crate exposes the coder primitives so codec implementations and other
downstream consumers (e.g. format-conformance tools, fuzz harnesses) can
share one well-tested implementation.

## Status

Extracted from the [`djvu-rs`](https://crates.io/crates/djvu-rs) umbrella
crate as part of issue [#229](https://github.com/matyushkin/djvu-rs/issues/229).
The decoder is the same code shipped in `djvu-rs ≥ 0.14`, vetted by the
project's full corpus + property + fuzz suite.

## Usage

### Decoder (no-std capable, no allocations)

```rust
use djvu_zp::ZpDecoder;

let compressed: &[u8] = &[0x00, 0x00];
let mut dec = ZpDecoder::new(compressed)?;
let mut ctx = 0u8;
let _bit = dec.decode_bit(&mut ctx);
# Ok::<(), djvu_zp::ZpError>(())
```

### Encoder (requires `std`, default-on)

```rust
use djvu_zp::encoder::ZpEncoder;

let mut enc = ZpEncoder::new();
let mut ctx = 0u8;
enc.encode_bit(&mut ctx, true);
let bytes: Vec<u8> = enc.finish();
```

## Features

| Feature | Default | Effect |
| ------- | ------- | ------ |
| `std`   | ✓       | Enables [`encoder::ZpEncoder`]. The decoder works either way and never allocates. |

## License

MIT — see the upstream [`djvu-rs`](https://github.com/matyushkin/djvu-rs)
repository.
