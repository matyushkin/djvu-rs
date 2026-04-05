//! Typed error hierarchy for the djvu-rs crate.
//!
//! This module provides:
//! - [`DjVuError`] — the new top-level error type for phase-1+ code
//! - [`IffError`] — errors from the new IFF container parser
//! - [`BzzError`] — errors from the BZZ decompressor (phase 2a)
//! - [`Jb2Error`], [`Iw44Error`] — stubs for future decoders
//! - `LegacyError` — the original error type, kept for backward compatibility
//! - `TextError` — errors from the text layer parser (phase 4, see `text` module)
//! - `AnnotationError` — errors from the annotation parser (phase 4, see `annotation` module)

#[cfg(not(feature = "std"))]
use alloc::{borrow::Cow, string::String};

// ---- New phase-1 typed errors -----------------------------------------------

/// Top-level error type for all DjVu decoding operations.
#[derive(Debug, thiserror::Error)]
pub enum DjVuError {
    /// An error in the IFF container format.
    #[error("IFF error: {0}")]
    Iff(#[from] IffError),

    /// A JB2 bitonal image decoding error (stub).
    #[error("JB2 error: {0}")]
    Jb2(#[from] Jb2Error),

    /// An IW44 wavelet image decoding error (stub).
    #[error("IW44 error: {0}")]
    Iw44(#[from] Iw44Error),

    /// A BZZ compression decoding error.
    #[error("BZZ error: {0}")]
    Bzz(#[from] BzzError),

    /// A page number was not found in the document.
    #[error("page {0} not found")]
    PageNotFound(usize),

    /// The document structure is invalid or unexpected.
    #[error("invalid structure: {0}")]
    InvalidStructure(&'static str),

    /// A feature or format variant that is not yet supported.
    #[error("unsupported: {0}")]
    #[cfg(feature = "std")]
    Unsupported(std::borrow::Cow<'static, str>),
    /// A feature or format variant that is not yet supported.
    #[error("unsupported: {0}")]
    #[cfg(not(feature = "std"))]
    Unsupported(Cow<'static, str>),

    /// An I/O error (only available with the `std` feature).
    #[cfg(feature = "std")]
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors that can occur while parsing the IFF container.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IffError {
    /// Input data is too short to contain a valid IFF file.
    #[error("input is too short to be a valid IFF file")]
    TooShort,

    /// The `AT&T` magic bytes were not found at the start of the file.
    #[error("bad magic bytes: expected AT&T, got {got:?}")]
    BadMagic { got: [u8; 4] },

    /// The FORM type identifier is not a recognised DjVu type.
    ///
    /// Note: this is *not* an error — callers may encounter unknown form types
    /// in bundled documents and should handle them gracefully.
    #[error("unknown FORM type: {id:?}")]
    UnknownFormType { id: [u8; 4] },

    /// A chunk header claims more bytes than are available in the buffer.
    #[error(
        "chunk {:?} claims {} bytes but only {} are available",
        id,
        claimed,
        available
    )]
    ChunkTooLong {
        id: [u8; 4],
        claimed: u32,
        available: usize,
    },

    /// The input ended unexpectedly in the middle of a chunk.
    #[error("unexpected end of input (truncated IFF data)")]
    Truncated,
}

/// JB2 bitonal image decoding errors.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Jb2Error {
    /// Input ended before the JB2 stream was complete.
    #[error("JB2 stream is truncated")]
    Truncated,

    /// A flag bit in the image/dict header was set when it must be zero.
    #[error("JB2: bad flag bit in header")]
    BadHeaderFlag,

    /// The inherited dictionary length exceeds the shared dictionary size.
    #[error("JB2: inherited dict length exceeds shared dict size")]
    InheritedDictTooLarge,

    /// The stream references a shared dictionary but none was provided.
    #[error("JB2: stream requires shared dict but none provided")]
    MissingSharedDict,

    /// Image dimensions exceed the safety limit (~64M pixels).
    #[error("JB2: image dimensions too large")]
    ImageTooLarge,

    /// A record references a dictionary symbol but the dictionary is empty.
    #[error("JB2: dict reference with empty dict")]
    EmptyDictReference,

    /// A decoded symbol index is out of range for the current dictionary.
    #[error("JB2: decoded symbol index out of dictionary range")]
    InvalidSymbolIndex,

    /// An unrecognized record type was encountered in the image stream.
    #[error("JB2: unknown record type")]
    UnknownRecordType,

    /// An unexpected record type was encountered in a dictionary stream.
    #[error("JB2: unexpected record type in dict stream")]
    UnexpectedDictRecordType,

    /// The ZP arithmetic coder could not be initialized (insufficient input).
    #[error("JB2: insufficient data to initialize ZP coder")]
    ZpInitFailed,

    /// Stream contains more records than the safety limit allows.
    #[error("JB2: record count exceeds safety limit")]
    TooManyRecords,
}

/// IW44 wavelet image decoding errors.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Iw44Error {
    /// Input ended before the IW44 stream was complete.
    #[error("IW44 stream is truncated")]
    Truncated,

    /// The IW44 stream contains invalid data.
    #[error("IW44 stream contains invalid data")]
    Invalid,

    /// A BG44/FG44/TH44 chunk is too short (fewer than 2 bytes).
    #[error("IW44 chunk is too short")]
    ChunkTooShort,

    /// The first chunk header is too short (needs at least 9 bytes).
    #[error("IW44 first chunk header too short (need ≥ 9 bytes)")]
    HeaderTooShort,

    /// Image width or height is zero.
    #[error("IW44 image has zero dimension")]
    ZeroDimension,

    /// Image dimensions exceed the safety limit.
    #[error("IW44 image dimensions too large")]
    ImageTooLarge,

    /// A subsequent chunk was encountered before the first chunk.
    #[error("IW44 subsequent chunk received before first chunk")]
    MissingFirstChunk,

    /// The subsample parameter must be >= 1.
    #[error("IW44 subsample must be >= 1")]
    InvalidSubsample,

    /// No codec has been initialized (no chunks decoded yet).
    #[error("IW44 codec not yet initialized")]
    MissingCodec,

    /// The ZP arithmetic coder stream is too short.
    #[error("IW44 ZP coder stream too short")]
    ZpTooShort,
}

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
}

// ---- Legacy error type (kept for backward compatibility) --------------------

/// Original error type used by the legacy implementation.
///
/// Kept at `crate::error::LegacyError` (and re-exported as `crate::Error`)
/// so that djvu-rs and other dependents continue to compile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyError {
    /// Input data is shorter than expected.
    UnexpectedEof,
    /// A required magic number or tag was not found.
    InvalidMagic,
    /// A chunk or field has an invalid length.
    InvalidLength,
    /// A required chunk is missing.
    MissingChunk(&'static str),
    /// An unsupported feature or version was encountered.
    Unsupported(&'static str),
    /// Generic format violation.
    FormatError(String),
}

impl core::fmt::Display for LegacyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LegacyError::UnexpectedEof => write!(f, "unexpected end of input"),
            LegacyError::InvalidMagic => write!(f, "invalid magic number"),
            LegacyError::InvalidLength => write!(f, "invalid length"),
            LegacyError::MissingChunk(id) => write!(f, "missing required chunk: {}", id),
            LegacyError::Unsupported(msg) => write!(f, "unsupported: {}", msg),
            LegacyError::FormatError(msg) => write!(f, "format error: {}", msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LegacyError {}

/// Alias for [`LegacyError`] at the path `crate::error::Error`.
///
/// This allows the legacy modules (document.rs, render.rs) which use
/// `crate::error::Error` to continue resolving correctly.
pub use LegacyError as Error;
