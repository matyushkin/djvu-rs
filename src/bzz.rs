//! BZZ decompressor re-export.
//!
//! The implementation lives in the standalone `djvu-bzz` crate; this module
//! re-exports it as `djvu_rs::bzz::*`. (The module was called `bzz_new`
//! before 0.23; 1.0 removed that alias.)

pub use djvu_bzz::{bzz_decode, decode};

#[cfg(feature = "parallel")]
pub use djvu_bzz::bzz_decode_parallel;
