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

use crate::{bzz_new::bzz_decode, error::BzzError, info::Rotation};

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

impl TextLayer {
    /// Return a copy of this text layer with all zone rectangles transformed to
    /// match a rendered page of size `render_w × render_h`.
    ///
    /// - `page_w`, `page_h` — native page dimensions from the INFO chunk.
    /// - `rotation` — page rotation from the INFO chunk.
    /// - `render_w`, `render_h` — the pixel size of the rendered output.
    ///
    /// Applies rotation first (in native pixel space), then scales the result
    /// proportionally to the requested render size.  The text content is
    /// preserved unchanged.
    pub fn transform(
        &self,
        page_w: u32,
        page_h: u32,
        rotation: Rotation,
        render_w: u32,
        render_h: u32,
    ) -> Self {
        let (disp_w, disp_h) = match rotation {
            Rotation::Cw90 | Rotation::Ccw90 => (page_h, page_w),
            _ => (page_w, page_h),
        };
        let t = ZoneTransform {
            page_w,
            page_h,
            rotation,
            disp_w,
            disp_h,
            render_w,
            render_h,
        };
        let zones = self.zones.iter().map(|z| transform_zone(z, &t)).collect();
        TextLayer {
            text: self.text.clone(),
            zones,
        }
    }
}

// ---- Coordinate helpers -----------------------------------------------------

impl Rect {
    /// Rotate this rectangle within a `page_w × page_h` native coordinate space.
    ///
    /// Coordinates are in top-left origin.  Returns the transformed rect in the
    /// rotated display space (which has dimensions `page_h × page_w` for 90°
    /// rotations and `page_w × page_h` for 0°/180°).
    pub fn rotate(&self, page_w: u32, page_h: u32, rotation: Rotation) -> Self {
        match rotation {
            Rotation::None => self.clone(),
            Rotation::Rot180 => Rect {
                x: page_w.saturating_sub(self.x.saturating_add(self.width)),
                y: page_h.saturating_sub(self.y.saturating_add(self.height)),
                width: self.width,
                height: self.height,
            },
            // Clockwise 90°: displayed page is page_h wide × page_w tall.
            // (x, y, w, h) → (page_h - y - h,  x,  h,  w)
            Rotation::Cw90 => Rect {
                x: page_h.saturating_sub(self.y.saturating_add(self.height)),
                y: self.x,
                width: self.height,
                height: self.width,
            },
            // Counter-clockwise 90°: displayed page is page_h wide × page_w tall.
            // (x, y, w, h) → (y,  page_w - x - w,  h,  w)
            Rotation::Ccw90 => Rect {
                x: self.y,
                y: page_w.saturating_sub(self.x.saturating_add(self.width)),
                width: self.height,
                height: self.width,
            },
        }
    }

    /// Scale this rectangle from a `from_w × from_h` space to `to_w × to_h`.
    pub fn scale(&self, from_w: u32, from_h: u32, to_w: u32, to_h: u32) -> Self {
        if from_w == 0 || from_h == 0 {
            return self.clone();
        }
        Rect {
            x: (self.x as u64 * to_w as u64 / from_w as u64) as u32,
            y: (self.y as u64 * to_h as u64 / from_h as u64) as u32,
            width: (self.width as u64 * to_w as u64 / from_w as u64) as u32,
            height: (self.height as u64 * to_h as u64 / from_h as u64) as u32,
        }
    }
}

/// Parameters for `transform_zone` — groups the 7 invariants so we stay
/// under clippy's `too_many_arguments` limit.
struct ZoneTransform {
    page_w: u32,
    page_h: u32,
    rotation: Rotation,
    disp_w: u32,
    disp_h: u32,
    render_w: u32,
    render_h: u32,
}

fn transform_zone(zone: &TextZone, t: &ZoneTransform) -> TextZone {
    let rotated = zone.rect.rotate(t.page_w, t.page_h, t.rotation);
    let scaled = rotated.scale(t.disp_w, t.disp_h, t.render_w, t.render_h);
    let children = zone.children.iter().map(|c| transform_zone(c, t)).collect();
    TextZone {
        kind: zone.kind,
        rect: scaled,
        text: zone.text.clone(),
        children,
    }
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

    let mut x = read_i16_biased(data, pos).ok_or(TextError::ZoneTruncated(*pos))?;
    let mut y = read_i16_biased(data, pos).ok_or(TextError::ZoneTruncated(*pos))?;
    let width = read_i16_biased(data, pos).ok_or(TextError::ZoneTruncated(*pos))?;
    let height = read_i16_biased(data, pos).ok_or(TextError::ZoneTruncated(*pos))?;
    let mut text_start = read_i16_biased(data, pos).ok_or(TextError::ZoneTruncated(*pos))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── Low-level reader tests ──────────────────────────────────────────────

    #[test]
    fn test_read_u24() {
        let data = [0x01, 0x02, 0x03];
        let mut pos = 0;
        assert_eq!(read_u24(&data, &mut pos), Some(0x010203));
        assert_eq!(pos, 3);
    }

    #[test]
    fn test_read_u24_truncated() {
        let data = [0x01, 0x02];
        let mut pos = 0;
        assert_eq!(read_u24(&data, &mut pos), None);
    }

    #[test]
    fn test_read_i16_biased() {
        let data = [0x80, 0x00]; // 0x8000 - 0x8000 = 0
        let mut pos = 0;
        assert_eq!(read_i16_biased(&data, &mut pos), Some(0));
        assert_eq!(pos, 2);
    }

    #[test]
    fn test_read_i16_biased_negative() {
        let data = [0x00, 0x00]; // 0x0000 - 0x8000 = -32768
        let mut pos = 0;
        assert_eq!(read_i16_biased(&data, &mut pos), Some(-0x8000));
    }

    #[test]
    fn test_read_i16_biased_truncated() {
        let data = [0x80];
        let mut pos = 0;
        assert_eq!(read_i16_biased(&data, &mut pos), None);
    }

    #[test]
    fn test_read_i24() {
        let data = [0x00, 0x01, 0x00];
        let mut pos = 0;
        assert_eq!(read_i24(&data, &mut pos), Some(256));
    }

    // ── extract_text_slice ──────────────────────────────────────────────────

    #[test]
    fn test_extract_text_slice_basic() {
        assert_eq!(extract_text_slice("hello world", 0, 5), "hello");
        assert_eq!(extract_text_slice("hello world", 6, 5), "world");
    }

    #[test]
    fn test_extract_text_slice_out_of_bounds() {
        assert_eq!(extract_text_slice("hello", 10, 5), "");
        assert_eq!(extract_text_slice("hello", 0, 100), "hello");
    }

    #[test]
    fn test_extract_text_slice_utf8_boundary() {
        // Multi-byte char: each char is 2 bytes
        let s = "\u{00e9}\u{00e8}"; // é è — 2 bytes each
        // Slicing at byte 1 (mid-char) should snap to boundary
        let result = extract_text_slice(s, 1, 2);
        assert!(result.is_char_boundary(0));
    }

    #[test]
    fn test_extract_text_slice_empty() {
        assert_eq!(extract_text_slice("", 0, 0), "");
        assert_eq!(extract_text_slice("abc", 1, 0), "");
    }

    // ── Error paths ─────────────────────────────────────────────────────────

    #[test]
    fn test_too_short_data() {
        assert!(matches!(
            parse_text_layer(&[0x00], 100),
            Err(TextError::TooShort)
        ));
        assert!(matches!(
            parse_text_layer(&[], 100),
            Err(TextError::TooShort)
        ));
    }

    #[test]
    fn test_text_overflow() {
        // text_len = 0x00_00_FF (255) but only 3+1 bytes available
        let data = [0x00, 0x00, 0xFF, 0x41];
        assert!(matches!(
            parse_text_layer(&data, 100),
            Err(TextError::TextOverflow)
        ));
    }

    #[test]
    fn test_invalid_utf8() {
        // text_len = 2, then 2 invalid bytes
        let data = [0x00, 0x00, 0x02, 0xFF, 0xFE];
        assert!(matches!(
            parse_text_layer(&data, 100),
            Err(TextError::InvalidUtf8)
        ));
    }

    #[test]
    fn test_unknown_zone_type() {
        // text_len=1, text="A", version=0, then zone type=99 (invalid)
        let data = [
            0x00, 0x00, 0x01, // text_len = 1
            b'A', // text
            0x00, // version
            99,   // invalid zone type
        ];
        assert!(matches!(
            parse_text_layer(&data, 100),
            Err(TextError::UnknownZoneType(99))
        ));
    }

    #[test]
    fn test_zone_truncated() {
        // text_len=1, text="A", version=0, zone type=1 (Page), then truncated
        let data = [
            0x00, 0x00, 0x01, // text_len = 1
            b'A', // text
            0x00, // version
            0x01, // zone type = Page
            0x80, 0x00, // x (only partial fields)
        ];
        assert!(matches!(
            parse_text_layer(&data, 100),
            Err(TextError::ZoneTruncated(_))
        ));
    }

    // ── Successful parse ────────────────────────────────────────────────────

    #[test]
    fn test_empty_text_no_zones() {
        // text_len=0, no zones after that
        let data = [0x00, 0x00, 0x00];
        let result = parse_text_layer(&data, 100).unwrap();
        assert_eq!(result.text, "");
        assert!(result.zones.is_empty());
    }

    #[test]
    fn test_text_only_no_zones() {
        // text_len=5, text="Hello", version byte, then no zone data
        let data = [
            0x00, 0x00, 0x05, // text_len = 5
            b'H', b'e', b'l', b'l', b'o', // text
            0x00, // version
        ];
        let result = parse_text_layer(&data, 100).unwrap();
        assert_eq!(result.text, "Hello");
        assert!(result.zones.is_empty());
    }

    // ── TextLayer::transform ─────────────────────────────────────────────────

    fn make_layer(x: u32, y: u32, w: u32, h: u32) -> TextLayer {
        TextLayer {
            text: "test".to_string(),
            zones: vec![TextZone {
                kind: TextZoneKind::Page,
                rect: Rect {
                    x,
                    y,
                    width: w,
                    height: h,
                },
                text: "test".to_string(),
                children: vec![],
            }],
        }
    }

    fn rect0(layer: &TextLayer) -> &Rect {
        &layer.zones[0].rect
    }

    #[test]
    fn transform_none_identity() {
        // No rotation, 1:1 scale — rects unchanged
        let layer = make_layer(10, 20, 30, 40);
        let out = layer.transform(100, 200, Rotation::None, 100, 200);
        assert_eq!(
            *rect0(&out),
            Rect {
                x: 10,
                y: 20,
                width: 30,
                height: 40
            }
        );
    }

    #[test]
    fn transform_none_scale_2x() {
        let layer = make_layer(10, 20, 30, 40);
        let out = layer.transform(100, 200, Rotation::None, 200, 400);
        assert_eq!(
            *rect0(&out),
            Rect {
                x: 20,
                y: 40,
                width: 60,
                height: 80
            }
        );
    }

    #[test]
    fn transform_rot180() {
        // page 100×200, rect (10, 20, 30, 40)
        // new_x = 100 - 10 - 30 = 60
        // new_y = 200 - 20 - 40 = 140
        let layer = make_layer(10, 20, 30, 40);
        let out = layer.transform(100, 200, Rotation::Rot180, 100, 200);
        assert_eq!(
            *rect0(&out),
            Rect {
                x: 60,
                y: 140,
                width: 30,
                height: 40
            }
        );
    }

    #[test]
    fn transform_cw90() {
        // page 100×200, rect (x=10, y=20, w=30, h=40)
        // displayed: 200 wide × 100 tall
        // new_x = page_h - y - h = 200 - 20 - 40 = 140
        // new_y = x = 10
        // new_w = h = 40,  new_h = w = 30
        let layer = make_layer(10, 20, 30, 40);
        let out = layer.transform(100, 200, Rotation::Cw90, 200, 100);
        assert_eq!(
            *rect0(&out),
            Rect {
                x: 140,
                y: 10,
                width: 40,
                height: 30
            }
        );
    }

    #[test]
    fn transform_ccw90() {
        // page 100×200, rect (x=10, y=20, w=30, h=40)
        // displayed: 200 wide × 100 tall
        // new_x = y = 20
        // new_y = page_w - x - w = 100 - 10 - 30 = 60
        // new_w = h = 40,  new_h = w = 30
        let layer = make_layer(10, 20, 30, 40);
        let out = layer.transform(100, 200, Rotation::Ccw90, 200, 100);
        assert_eq!(
            *rect0(&out),
            Rect {
                x: 20,
                y: 60,
                width: 40,
                height: 30
            }
        );
    }

    #[test]
    fn transform_cw90_then_scale() {
        // page 100×200, rect (10, 20, 30, 40), render at 2× (400×200)
        // After Cw90: (140, 10, 40, 30) in 200×100 space
        // Scale ×2: (280, 20, 80, 60)
        let layer = make_layer(10, 20, 30, 40);
        let out = layer.transform(100, 200, Rotation::Cw90, 400, 200);
        assert_eq!(
            *rect0(&out),
            Rect {
                x: 280,
                y: 20,
                width: 80,
                height: 60
            }
        );
    }

    #[test]
    fn transform_text_preserved() {
        let layer = make_layer(0, 0, 10, 10);
        let out = layer.transform(100, 100, Rotation::Cw90, 100, 100);
        assert_eq!(out.text, "test");
        assert_eq!(out.zones[0].text, "test");
    }

    #[test]
    fn test_single_word_zone() {
        // Build a minimal text layer with one Page zone containing "Hi"
        let text = b"Hi";
        let mut data = Vec::new();
        // text_len = 2 (u24be)
        data.extend_from_slice(&[0x00, 0x00, 0x02]);
        data.extend_from_slice(text);
        data.push(0x00); // version

        // Page zone (type=1)
        data.push(0x01);
        // x=0, y=0, w=100, h=50 (biased i16: value + 0x8000)
        data.extend_from_slice(&0x8000u16.to_be_bytes()); // x=0
        data.extend_from_slice(&0x8000u16.to_be_bytes()); // y=0
        data.extend_from_slice(&(100u16 + 0x8000u16).wrapping_add(0).to_be_bytes()); // w=100
        let h_val = 50i32 + 0x8000;
        data.extend_from_slice(&(h_val as u16).to_be_bytes()); // h=50
        data.extend_from_slice(&0x8000u16.to_be_bytes()); // text_start=0
        // text_len = 2 (i24)
        data.extend_from_slice(&[0x00, 0x00, 0x02]);
        // children_count = 0 (i24)
        data.extend_from_slice(&[0x00, 0x00, 0x00]);

        let result = parse_text_layer(&data, 100).unwrap();
        assert_eq!(result.text, "Hi");
        assert_eq!(result.zones.len(), 1);
        assert_eq!(result.zones[0].kind, TextZoneKind::Page);
        assert_eq!(result.zones[0].text, "Hi");
        assert_eq!(result.zones[0].rect.width, 100);
        assert_eq!(result.zones[0].rect.height, 50);
    }
}
