//! JB2 bilevel image decoder — clean-room implementation (phase 2b).
//!
//! Decodes JB2-encoded bitonal images from DjVu Sjbz and Djbz chunks.
//! The JB2 format uses a ZP adaptive arithmetic coder with 262 context variables
//! and a symbol dictionary for run-length compression of recurring glyphs.
//!
//! # Key public types
//!
//! - `Jb2Dict` — shared symbol dictionary decoded from a Djbz chunk
//! - `decode` — decode a Sjbz image stream to a `Bitmap`
//! - `decode_dict` — decode a Djbz dictionary stream to a `Jb2Dict`
//!
//! # Record types
//!
//! | Code | Meaning |
//! |------|---------|
//! | 0    | start-of-image |
//! | 1    | new-symbol, add to dict AND blit to page |
//! | 2    | new-symbol, add to dict only |
//! | 3    | new-symbol (direct), blit only (not added to dict) |
//! | 4    | matched-refine, add to dict AND blit |
//! | 5    | matched-refine, add to dict only |
//! | 6    | matched-refine, blit only |
//! | 7    | matched-copy (no refinement), blit only |
//! | 8    | non-symbol (halftone block), blit only |
//! | 9    | required-dict-or-reset |
//! | 10   | comment |
//! | 11   | end-of-data |

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};
#[cfg(feature = "std")]
use std::{vec, vec::Vec};

use djvu_bitmap::Bitmap;
use djvu_zp::ZpDecoder;

/// JB2 bilevel image encoder (`std`-only). Produces `Sjbz`/`Djbz` payloads
/// decodable by this crate. Kept behind `std` so the decoder stays
/// `no_std`-capable.
#[cfg(feature = "std")]
pub mod encode;

/// JB2 bitonal image decoding errors.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
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

// ── Modules ─────────────────────────────────────────────────────────────────
//
// One file per concern (#904). Every item a sibling needs is `pub(super)`:
// the same reach a private item had when this crate root was one file.

mod blit;
mod dict;
mod direct;
mod image;
mod jbm;
mod layout;
mod num;
mod refine;

use blit::*;
use dict::*;
use direct::*;
use image::*;
use jbm::*;
use layout::*;
use num::*;
use refine::*;

// ────────────────────────────────────────────────────────────────────────────
// Public API
// ────────────────────────────────────────────────────────────────────────────

/// A shared JB2 symbol dictionary decoded from a Djbz chunk.
///
/// Pass this to [`decode`] when the Sjbz stream references an external dict
/// via a "required-dict-or-reset" (type 9) record.
pub struct Jb2Dict {
    symbols: Vec<Jbm>,
}

/// Decode a JB2 image stream (Sjbz chunk data) into a [`Bitmap`].
///
/// `shared_dict` must be provided when the Sjbz stream begins with a
/// "required-dict-or-reset" record that references an external dictionary.
///
/// # Errors
///
/// Returns [`Jb2Error`] on malformed input, missing dictionary, or oversized image.
pub fn decode(data: &[u8], shared_dict: Option<&Jb2Dict>) -> Result<Bitmap, Jb2Error> {
    decode_image(data, shared_dict)
}

/// Decode a JB2 image stream directly into a `1/2^shift`-resolution
/// [`Bitmap`], OR-reducing (max-pooling) each decoded pixel into its
/// downsampled cell as it is blitted, instead of allocating a full-resolution
/// canvas and downsampling it afterward.
///
/// `shift = 0` is identical to [`decode`]. For `shift >= 1` this is
/// semantically identical to decoding at full resolution and then
/// max-pool-downsampling by `2^shift` (block-OR reduction, floor-aligned
/// blocks, `div_ceil` output size) — it exists purely to skip the
/// full-resolution canvas allocation and the full-canvas downsample scan
/// when only a coarse mask is needed (e.g. a thumbnail render composited at
/// IW44 subsample >= 4). The arithmetic decode of the symbol dictionary and
/// page instructions is unavoidable either way — this only shrinks the
/// output canvas the decoded symbols are blitted into.
///
/// # Errors
///
/// Returns [`Jb2Error`] on malformed input, missing dictionary, or oversized image.
pub fn decode_downsampled(
    data: &[u8],
    shared_dict: Option<&Jb2Dict>,
    shift: u32,
) -> Result<Bitmap, Jb2Error> {
    let mut pool = Vec::new();
    decode_image_with_pool(data, shared_dict, &mut pool, shift)
}

/// Decode a JB2 image stream with per-pixel blit index tracking.
///
/// Returns the bitmap and a blit map (`Vec<i32>`) of the same pixel dimensions.
/// `blit_map[y * width + x]` holds the blit record index for each foreground
/// pixel, or `-1` for background. This is used by the FGbz palette to assign
/// per-glyph colors.
pub fn decode_indexed(
    data: &[u8],
    shared_dict: Option<&Jb2Dict>,
) -> Result<(Bitmap, Vec<i32>), Jb2Error> {
    decode_image_indexed(data, shared_dict)
}

/// Decode a JB2 dictionary stream (Djbz chunk data) into a [`Jb2Dict`].
///
/// The returned dict can then be passed to [`decode`] for Sjbz streams that
/// reference it via an INCL or "required-dict-or-reset" record.
///
/// # Errors
///
/// Returns [`Jb2Error`] on malformed input.
pub fn decode_dict(data: &[u8], inherited: Option<&Jb2Dict>) -> Result<Jb2Dict, Jb2Error> {
    decode_dictionary(data, inherited)
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;

#[cfg(test)]
mod regression_fuzz2;
