//! DjVu text layer parser — phase 4.
//!
//! Parses TXTa (plain) and TXTz (BZZ-compressed) text layer chunks into a
//! structured zone hierarchy with remapped coordinates.
//!
//! ## Key public types
//!
//! - [`TextLayer`] — the full text content and zone hierarchy of a page
//! - [`TextZone`] — a single zone node (page/column/para/line/word/char)
//! - [`TextZoneKind`] — enum discriminating zone types
//! - `Rect` — bounding rectangle in top-left-origin coordinates
//! - `TextError` — typed errors from this module
//!
//! ## Format notes
//!
//! The TXTa/TXTz binary format stores:
//!   `[u24be text_len][utf8 text][u8 version][zone tree]`
//!
//! Zone coordinates use DjVu's bottom-left origin. This module remaps all
//! coordinates to a top-left origin using the provided page height.
//!
//! Zone fields are delta-encoded relative to a parent or previous sibling.

#[cfg(not(feature = "std"))]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{bzz_new::bzz_decode, error::BzzError};

// ---- Error ------------------------------------------------------------------

/// Errors from text layer parsing.
#[derive(Debug, thiserror::Error)]
pub enum TextError {
    /// BZZ decompression failed.
    #[error("bzz decode failed: {0}")]
    Bzz(#[from] BzzError),

    /// The binary data is too short to be a valid text layer.
    #[error("text layer data too short")]
    TooShort,

    /// A text length field points past the end of the data.
    #[error("text length overflows data")]
    TextOverflow,

    /// The text bytes are not valid UTF-8.
    #[error("invalid UTF-8 in text layer")]
    InvalidUtf8,

    /// A zone record is truncated (not enough bytes for a field).
    #[error("zone record truncated at offset {0}")]
    ZoneTruncated(usize),

    /// An unknown zone type byte was encountered.
    #[error("unknown zone type {0}")]
    UnknownZoneType(u8),
}

// ---- Public types -----------------------------------------------------------

/// Zone type discriminant in the DjVu text layer hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextZoneKind {
    Page,
    Column,
    Region,
    Para,
    Line,
    Word,
    Character,
}

/// Bounding rectangle in top-left-origin coordinates (pixels).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// A single node in the text zone hierarchy.
#[derive(Debug, Clone)]
pub struct TextZone {
    /// Zone type.
    pub kind: TextZoneKind,
    /// Bounding box (top-left origin, after coordinate remap).
    pub rect: Rect,
    /// Text covered by this zone (substring of [`TextLayer::text`]).
    pub text: String,
    /// Child zones (columns inside page, words inside line, etc.).
    pub children: Vec<TextZone>,
}

/// The complete text layer of a DjVu page.
#[derive(Debug, Clone)]
pub struct TextLayer {
    /// Full plain-text content of the page, UTF-8.
    pub text: String,
    /// Top-level zone nodes (usually a single `Page` zone).
    pub zones: Vec<TextZone>,
}

// ---- Entry points -----------------------------------------------------------

/// Parse a TXTa (uncompressed) text layer chunk.
///
/// `page_height` is used to remap DjVu bottom-left coordinates to top-left.
pub fn parse_text_layer(data: &[u8], page_height: u32) -> Result<TextLayer, TextError> {
    parse_text_layer_inner(data, page_height)
}

/// Parse a TXTz (BZZ-compressed) text layer chunk.
///
/// Decompresses with BZZ first, then delegates to [`parse_text_layer`].
pub fn parse_text_layer_bzz(data: &[u8], page_height: u32) -> Result<TextLayer, TextError> {
    let decoded = bzz_decode(data)?;
    parse_text_layer_inner(&decoded, page_height)
}

// ---- Internal parsing -------------------------------------------------------

fn parse_text_layer_inner(data: &[u8], page_height: u32) -> Result<TextLayer, TextError> {
    if data.len() < 3 {
        return Err(TextError::TooShort);
    }

    let mut pos = 0usize;

    // Read text length (u24be)
    let text_len = read_u24(data, &mut pos).ok_or(TextError::TooShort)?;

    // Read UTF-8 text
    let text_end = pos.checked_add(text_len).ok_or(TextError::TextOverflow)?;
    if text_end > data.len() {
        return Err(TextError::TextOverflow);
    }
    let text = core::str::from_utf8(data.get(pos..text_end).ok_or(TextError::TextOverflow)?)
        .map_err(|_| TextError::InvalidUtf8)?
        .to_string();
    pos = text_end;

    // Consume version byte (if present)
    if pos < data.len() {
        pos += 1; // version byte — currently unused
    }

    // Parse zone tree
    let mut zones = Vec::new();
    if pos < data.len() {
        let zone = parse_zone(data, &mut pos, None, None, &text, page_height)?;
        zones.push(zone);
    }

    Ok(TextLayer { text, zones })
}

// ---- Zone parsing -----------------------------------------------------------

/// Delta-encoding context carried from one zone parse to the next.
#[derive(Clone)]
struct ZoneCtx {
    x: i32,
    y: i32, // bottom-left y (DjVu native)
    width: i32,
    height: i32,
    text_start: i32,
    text_len: i32,
}

fn parse_zone(
    data: &[u8],
    pos: &mut usize,
    parent: Option<&ZoneCtx>,
    prev: Option<&ZoneCtx>,
    full_text: &str,
    page_height: u32,
) -> Result<TextZone, TextError> {
    if *pos >= data.len() {
        return Err(TextError::ZoneTruncated(*pos));
    }

    let type_byte = *data.get(*pos).ok_or(TextError::ZoneTruncated(*pos))?;
    *pos += 1;

    let kind = match type_byte {
        1 => TextZoneKind::Page,
        2 => TextZoneKind::Column,
        3 => TextZoneKind::Region,
        4 => TextZoneKind::Para,
        5 => TextZoneKind::Line,
        6 => TextZoneKind::Word,
        7 => TextZoneKind::Character,
        other => return Err(TextError::UnknownZoneType(other)),
    };

    let mut x = read_i16_biased(data, pos).ok_or(TextError::ZoneTruncated(*pos))? as i32;
    let mut y = read_i16_biased(data, pos).ok_or(TextError::ZoneTruncated(*pos))? as i32;
    let width = read_i16_biased(data, pos).ok_or(TextError::ZoneTruncated(*pos))? as i32;
    let height = read_i16_biased(data, pos).ok_or(TextError::ZoneTruncated(*pos))? as i32;
    let mut text_start = read_i16_biased(data, pos).ok_or(TextError::ZoneTruncated(*pos))? as i32;
    let text_len = read_i24(data, pos).ok_or(TextError::ZoneTruncated(*pos))?;

    // Apply delta encoding (matches djvujs DjVuText.js decodeZone logic)
    if let Some(prev) = prev {
        match type_byte {
            1 | 4 | 5 => {
                // PAGE, PARAGRAPH, LINE
                x += prev.x;
                y = prev.y - (y + height);
            }
            _ => {
                // COLUMN, REGION, WORD, CHARACTER
                x += prev.x + prev.width;
                y += prev.y;
            }
        }
        text_start += prev.text_start + prev.text_len;
    } else if let Some(parent) = parent {
        x += parent.x;
        y = parent.y + parent.height - (y + height);
        text_start += parent.text_start;
    }

    // Remap y from DjVu bottom-left to top-left
    // top_left_y = page_height - (bl_y + height)
    let tl_y = (page_height as i32)
        .saturating_sub(y.saturating_add(height))
        .max(0) as u32;
    let tl_x = x.max(0) as u32;
    let tl_w = width.max(0) as u32;
    let tl_h = height.max(0) as u32;

    let rect = Rect {
        x: tl_x,
        y: tl_y,
        width: tl_w,
        height: tl_h,
    };

    // Extract zone text
    let ts = text_start.max(0) as usize;
    let tl = text_len.max(0) as usize;
    let zone_text = extract_text_slice(full_text, ts, tl);

    let children_count = read_i24(data, pos)
        .ok_or(TextError::ZoneTruncated(*pos))?
        .max(0) as usize;

    let ctx = ZoneCtx {
        x,
        y,
        width,
        height,
        text_start,
        text_len,
    };

    let mut children = Vec::with_capacity(children_count);
    let mut prev_child: Option<ZoneCtx> = None;

    for _ in 0..children_count {
        let child = parse_zone(
            data,
            pos,
            Some(&ctx),
            prev_child.as_ref(),
            full_text,
            page_height,
        )?;
        prev_child = Some(ZoneCtx {
            x: child.rect.x as i32,
            y: {
                // We need to store the original bottom-left y for delta calc.
                // Inverse remap: bl_y = page_height - (tl_y + height)
                (page_height as i32).saturating_sub(child.rect.y as i32 + child.rect.height as i32)
            },
            width: child.rect.width as i32,
            height: child.rect.height as i32,
            text_start: ts as i32,
            text_len: tl as i32,
        });
        children.push(child);
    }

    Ok(TextZone {
        kind,
        rect,
        text: zone_text,
        children,
    })
}

/// Extract a substring from `full_text` starting at byte offset `start` with byte length `len`.
///
/// Clamps to valid char boundaries to avoid panics on multi-byte UTF-8.
fn extract_text_slice(full_text: &str, start: usize, len: usize) -> String {
    let end = start.saturating_add(len).min(full_text.len());
    let start = start.min(end);
    // Walk back to a valid char boundary
    let safe_start = (0..=start)
        .rev()
        .find(|&i| full_text.is_char_boundary(i))
        .unwrap_or(0);
    let safe_end = (end..=full_text.len())
        .find(|&i| full_text.is_char_boundary(i))
        .unwrap_or(full_text.len());
    full_text[safe_start..safe_end].to_string()
}

// ---- Low-level readers (no indexing, no unwrap) -----------------------------

/// Read 3 bytes as a u24 big-endian value; advance `pos` by 3. Returns None if truncated.
fn read_u24(data: &[u8], pos: &mut usize) -> Option<usize> {
    let b0 = *data.get(*pos)?;
    let b1 = *data.get(*pos + 1)?;
    let b2 = *data.get(*pos + 2)?;
    *pos += 3;
    Some(((b0 as usize) << 16) | ((b1 as usize) << 8) | (b2 as usize))
}

/// Read 2 bytes as a biased i16 (raw u16 − 0x8000). Returns None if truncated.
fn read_i16_biased(data: &[u8], pos: &mut usize) -> Option<i32> {
    let b0 = *data.get(*pos)?;
    let b1 = *data.get(*pos + 1)?;
    *pos += 2;
    let raw = u16::from_be_bytes([b0, b1]);
    Some(raw as i32 - 0x8000)
}

/// Read 3 bytes as a signed i24 big-endian. Returns None if truncated.
fn read_i24(data: &[u8], pos: &mut usize) -> Option<i32> {
    let b0 = *data.get(*pos)? as i32;
    let b1 = *data.get(*pos + 1)? as i32;
    let b2 = *data.get(*pos + 2)? as i32;
    *pos += 3;
    Some((b0 << 16) | (b1 << 8) | b2)
}
