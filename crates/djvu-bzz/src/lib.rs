//! BZZ compressor and decompressor for DjVu documents.
//!
//! BZZ combines ZP adaptive arithmetic coding, move-to-front coding, and the
//! Burrows-Wheeler transform. DjVu uses it for compressed metadata chunks such
//! as DIRM, NAVM, ANTz, TXTz, and FGbz.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

#[cfg(not(feature = "std"))]
extern crate alloc;

mod decode;
#[cfg(feature = "std")]
mod encode;

#[cfg(feature = "parallel")]
pub use decode::bzz_decode_parallel;
pub use decode::{bzz_decode, decode};
#[cfg(feature = "std")]
pub use encode::bzz_encode;

/// BZZ compression decoding errors.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BzzError {
    /// Input is too short to be a valid BZZ stream (fewer than 2 bytes).
    #[error("BZZ input is too short")]
    TooShort,

    /// The block size field in the BZZ stream is invalid or out of range.
    #[error("BZZ stream contains an invalid block size")]
    InvalidBlockSize,

    /// The BWT sort index embedded in the stream is out of range.
    #[error("BZZ stream contains an invalid BWT index")]
    InvalidBwtIndex,

    /// The ZP arithmetic coder encountered an error.
    #[error("ZP coder error in BZZ stream")]
    ZpError,

    /// The BWT block did not contain an end-of-block marker.
    #[error("BZZ block is missing the end-of-block marker")]
    MissingMarker,

    /// A single block's decoded size exceeds the safety limit (4 MB).
    #[error("BZZ block size exceeds maximum allowed ({0} > 4 MB)")]
    BlockSizeTooLarge(usize),

    /// The total decompressed output exceeds the safety limit (256 MB).
    #[error("BZZ total output size exceeds maximum allowed (256 MB)")]
    OutputTooLarge,
}

/// Map ZP-coder init errors into [`BzzError`] so callers using `?` keep working.
impl From<djvu_zp::ZpError> for BzzError {
    fn from(_: djvu_zp::ZpError) -> Self {
        BzzError::TooShort
    }
}
