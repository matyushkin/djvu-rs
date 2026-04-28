//! `#![no_std]` smoke test for the codec entry points called out in #227.
//!
//! Goal: prove that `iff::parse_form`, `bzz_new::bzz_decode`, `jb2::decode_dict`,
//! and `iw44_new::Iw44Image::decode_chunk` are callable from a consumer crate
//! that has `default-features = false` (no `std`). If any of these public
//! signatures grow a `std::*` type, this crate fails to compile.
//!
//! The bytes are embedded via `include_bytes!` so this crate has no I/O —
//! only `core` + `alloc` + `djvu-rs` (no_std build).
//!
//! Built in CI by the `wasm` job. Not run as a test (the lib never links a
//! binary). Compile-only verification is sufficient: a leaked `std::*` would
//! refuse to type-check here.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use djvu_rs::{bzz_new, iff, iw44_new, jb2};

/// Minimal IFF FORM:DJVU containing one INFO chunk. Hand-crafted to
/// exercise `iff::parse_form` without pulling a fixture file.
const TINY_FORM: &[u8] = &[
    b'A', b'T', b'&', b'T', // "AT&T" magic
    b'F', b'O', b'R', b'M', // FORM
    0x00, 0x00, 0x00, 0x10, // FORM length = 16 (4 type + 12 INFO chunk)
    b'D', b'J', b'V', b'U', // form type
    b'I', b'N', b'F', b'O', // INFO chunk id
    0x00, 0x00, 0x00, 0x04, // INFO length
    0x00, 0x10, 0x00, 0x10, // 4 bytes of INFO body (truncated header — fine for the parser)
];

/// Embedded BZZ stream from `tests/golden/bzz/test_short.bzz` (36 bytes).
const TINY_BZZ: &[u8] = include_bytes!("../../golden/bzz/test_short.bzz");

/// Calls each of the four codec entry points named in #227.
///
/// Returns `()` on success. Errors are folded into `Result<(), ()>` so the
/// signature stays free of any third-party error type — this keeps the
/// no_std contract focused on the codec APIs themselves.
#[allow(clippy::result_unit_err)]
pub fn smoke() -> Result<(), ()> {
    iff::parse_form(TINY_FORM).map_err(|_| ())?;

    let _decoded: Vec<u8> = bzz_new::bzz_decode(TINY_BZZ).map_err(|_| ())?;

    // jb2::decode_dict on an empty buffer returns Err — that still proves
    // the symbol is reachable from a no_std consumer. We don't care about
    // the value, only that the signature compiles.
    let _ = jb2::decode_dict(&[], None);

    // Iw44Image::new() + decode_chunk on a 2-byte header returns Err —
    // same rationale: signature reachability, not value correctness.
    let mut img = iw44_new::Iw44Image::new();
    let _ = img.decode_chunk(&[0u8, 0u8]);

    Ok(())
}
