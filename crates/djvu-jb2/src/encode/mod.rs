//! JB2 bilevel image encoder — produces Sjbz chunk payloads.
//!
//! Encodes a [`Bitmap`] into a JB2 stream decodable by [`crate::decode`].
//!
//! ## Encoding strategy
//!
//! The encoder emits the entire image as a single **record type 3** ("new symbol,
//! direct, blit only") record.  This produces valid output without requiring
//! connected-component analysis or a symbol dictionary.
//!
//! ## Binary format summary (see the crate-root decoder for full spec)
//!
//! ```text
//! encode_num(record_type_ctx, [0,11], 0)  — start-of-image
//! encode_num(image_size_ctx,  [0,262142], width)
//! encode_num(image_size_ctx,  [0,262142], height)
//! encode_bit(flag_ctx, false)             — reserved flag
//! encode_num(record_type_ctx, [0,11], 3)  — new-symbol, direct, blit-only
//! encode_num(symbol_width_ctx, [0,262142], width)
//! encode_num(symbol_height_ctx,[0,262142], height)
//! encode_bitmap_direct(...)               — 10-bit context bitmap
//! encode_bit(offset_type_ctx, true)       — new-line positioning
//! encode_num(hoff_ctx, [-262143,262142], 1)
//! encode_num(voff_ctx, [-262143,262142], 0)
//! encode_num(record_type_ctx, [0,11], 11) — end-of-data
//! ```

use crate::NumContext;
use djvu_bitmap::Bitmap;
use djvu_zp::encoder::ZpEncoder;

use std::collections::BTreeMap;

// ── Modules ──────────────────────────────────────────────────────────────────
//
// One file per concern (#900). Every item a sibling needs is `pub(super)`:
// the same reach a private item had when this module was one file.

mod analysis;
mod cc;
mod coder;
mod dict;
mod direct;
mod emit;
mod options;
mod refine;
mod shared;

pub use analysis::*;
use cc::*;
use coder::*;
pub use dict::*;
pub use direct::*;
use emit::*;
pub use options::*;
pub use refine::*;
pub use shared::*;

#[cfg(test)]
mod tests;
