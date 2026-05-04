//! Backwards-compatible BZZ decompressor module.
//!
//! The implementation lives in the standalone `djvu-bzz` crate. This module
//! preserves the historical `djvu_rs::bzz_new::*` path.

pub use djvu_bzz::{bzz_decode, decode};

#[cfg(feature = "parallel")]
pub use djvu_bzz::bzz_decode_parallel;
