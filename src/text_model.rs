//! DjVu text layer data model.
//!
//! Defines the types shared by the text parser ([`crate::text`]), the text
//! encoder ([`crate::text_encode`]), the serialisers ([`crate::text_serialize`]),
//! and every OCR backend.  No parsing logic lives here — only types and their
//! coordinate-transform methods.
//!
//! ## Key types
//!
//! - [`TextLayer`] — the full text content and zone hierarchy of a page
//! - [`TextZone`] — a single zone node (page/column/para/line/word/char)
//! - [`TextZoneKind`] — enum discriminating zone types
//! - [`Rect`] — bounding rectangle in top-left-origin coordinates
//! - [`Paragraph`] — a reflowable paragraph extracted from a [`TextLayer`]
//! - [`TextError`] — typed errors from text layer parsing

#[cfg(not(feature = "std"))]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::info::Rotation;

// ---- Error ------------------------------------------------------------------

/// Errors from text layer parsing.
#[derive(Debug, thiserror::Error)]
pub enum TextError {
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// A single node in the text zone hierarchy.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextLayer {
    /// Full plain-text content of the page, UTF-8.
    pub text: String,
    /// Top-level zone nodes (usually a single `Page` zone).
    pub zones: Vec<TextZone>,
}

/// A reflowable paragraph: the original lines as they appear on the page,
/// plus a single joined `text` string with line-break and hyphenation rules
/// applied (see [`TextLayer::reflowable_text`]).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Paragraph {
    /// Each physical line as it appeared on the page (trimmed of trailing
    /// whitespace, ordered top-to-bottom).
    pub lines: Vec<String>,
    /// Lines joined into a single string. Hyphenated line breaks (line ends
    /// with `-` and next line starts with a lowercase letter) are joined with
    /// no separator and the hyphen is dropped. Other line breaks become a
    /// single ASCII space.
    pub text: String,
}

// ---- TextLayer methods ------------------------------------------------------

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

    /// Group the page text into reading-order paragraphs (#228).
    ///
    /// Uses the DjVu zone separator characters carried in `self.text`:
    ///
    /// - `\x00` (NUL), `\x0b` (VT), `\x1d` (GS), `\x1f` (US) — paragraph /
    ///   region / column / page boundary. Each starts a new [`Paragraph`].
    /// - `\x0a` (LF) — line break within a paragraph.
    ///
    /// Line joining: trailing whitespace is dropped from each line. If a line
    /// ends with `-` and the next line starts with an ASCII lowercase letter,
    /// the hyphen is dropped and the lines are joined with no separator
    /// (soft-hyphen at line end). Otherwise lines are joined with a single
    /// ASCII space.
    ///
    /// Empty paragraphs (only whitespace) are skipped. The returned vector
    /// preserves zone-stream order — for the typical OCR'd single-column page
    /// this is reading order.
    pub fn reflowable_text(&self) -> Vec<Paragraph> {
        let mut out = Vec::new();
        for chunk in self
            .text
            .split(['\u{0000}', '\u{000b}', '\u{001d}', '\u{001f}'])
        {
            let lines: Vec<String> = chunk
                .split('\n')
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            if lines.is_empty() {
                continue;
            }
            let text = join_paragraph_lines(&lines);
            out.push(Paragraph { lines, text });
        }
        out
    }
}

fn join_paragraph_lines(lines: &[String]) -> String {
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            out.push_str(line);
            continue;
        }
        let prev_hyphen =
            out.ends_with('-') && line.chars().next().is_some_and(|c| c.is_ascii_lowercase());
        if prev_hyphen {
            out.pop();
            out.push_str(line);
        } else {
            out.push(' ');
            out.push_str(line);
        }
    }
    out
}

// ---- Rect methods -----------------------------------------------------------

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

// ---- Private zone transform helpers -----------------------------------------

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
