//! IW44 wavelet image decoder — pure-Rust clean-room implementation (phase 2c).
//!
//! Implements the IW44 progressive wavelet codec used by DjVu BG44, FG44, and
//! TH44 chunks.  Each BG44 chunk may carry one or more *slices*; the ZP coder
//! state persists across all chunks so that progressive refinement works correctly.
//!
//! ## Key public types
//!
//! - `Iw44Image` — progressive decoder; call `Iw44Image::decode_chunk` for
//!   each BG44/FG44/TH44 chunk, then `Iw44Image::to_rgb` to obtain an RGB
//!   pixmap.
//! - `Iw44Error` — typed error enum (re-exported from
//!   this crate).
//!
//! ## Architecture
//!
//! YCbCr planes are kept separate (`y: Vec<i16>`, `cb: Vec<i16>`, `cr: Vec<i16>`)
//! until `to_rgb()` is called.  This allows future SIMD processing on each plane
//! independently.  No interleaved buffers exist inside this module.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};
#[cfg(feature = "std")]
use std::{vec, vec::Vec};

use djvu_pixmap::{GrayPixmap, Pixmap, PixmapError};
use djvu_zp::ZpDecoder;
use wide::i32x8;

/// IW44 wavelet image encoder — produces BG44/FG44/TH44 chunk payloads (std-only).
///
/// Shares the band/quant/state-flag/zigzag spec data with the decoder (this
/// module) instead of re-declaring it. Requires `std` for the ZP encoder and the
/// SIMD forward-transform paths.
#[cfg(feature = "std")]
pub mod encode;

/// IW44 wavelet image decoding errors.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
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

    /// A chunk's serial number does not match the expected next value
    /// (0, 1, 2, … in document order). Mirrors DjVuLibre's
    /// `IW44Image.wrong_serial`/`wrong_serial2` check in
    /// `IWBitmap::decode_chunk`/`IWPixmap::decode_chunk` — a corrupted or
    /// desynced chunk sequence (e.g. from a bit-flip landing on the serial
    /// byte itself, or a dropped/duplicated chunk) is rejected instead of
    /// silently decoded into the wrong refinement slot.
    #[error("IW44 chunk does not bear expected serial number")]
    UnexpectedSerial,
}

/// The decoder's output pixmap is bounded like its input planes.
///
/// [`Pixmap::try_new`] refuses more than [`Pixmap::MAX_PIXELS`]; the decoder
/// already rejects an image that large at the header, so this is the same
/// limit reported from the other end.
impl From<PixmapError> for Iw44Error {
    fn from(_: PixmapError) -> Self {
        Iw44Error::ImageTooLarge
    }
}

// ---- Modules ----------------------------------------------------------------
//
// One file per concern (#898). Every item a sibling needs is `pub(super)`:
// the same reach a private item had when this crate root was one file.

mod color;
mod image;
mod plane;
mod tables;
mod wavelet;

use color::*;
pub use image::*;
use plane::*;
use tables::*;
use wavelet::*;

// ---- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests;
