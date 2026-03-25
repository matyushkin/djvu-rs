//! DjVu annotation parser — phase 4.
//!
//! Parses ANTa (plain) and ANTz (BZZ-compressed) annotation chunks into
//! typed structures.
//!
//! ## Key public types
//!
//! - `Annotation` — page-level annotation (background, zoom, mode)
//! - `MapArea` — a clickable area with URL, description, and shape
//! - `Shape` — rect / oval / poly / line / text area shape
//! - `Color` — RGB color parsed from `#rrggbb` strings
//! - `AnnotationError` — typed errors from this module
//!
//! ## Format notes
//!
//! ANTa/ANTz contain S-expression-like text:
//! ```text
//! (background #ffffff)
//! (zoom 100)
//! (mode color)
//! (maparea "url" "desc" (rect x y w h) ...)
//! ```
//!
//! This parser handles only the subset documented in the DjVu v3 spec
//! (background, zoom, mode, maparea with rect/oval/poly/line/text shapes).

#[cfg(not(feature = "std"))]
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use crate::{bzz_new::bzz_decode, error::BzzError};

// ---- Error ------------------------------------------------------------------

/// Errors from annotation parsing.
#[derive(Debug, thiserror::Error)]
pub enum AnnotationError {
    /// BZZ decompression failed.
    #[error("bzz decode failed: {0}")]
    Bzz(#[from] BzzError),

    /// A hex color string is malformed.
    #[error("invalid color value: {0}")]
    InvalidColor(String),

    /// A numeric value could not be parsed.
    #[error("invalid number: {0}")]
    InvalidNumber(String),

    /// The S-expression is malformed (missing closing paren, etc.).
    #[error("malformed s-expression: {0}")]
    Parse(String),
}

// ---- Public types -----------------------------------------------------------

/// An RGB color value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Bounding rectangle in DjVu coordinates.
///
/// Note: coordinates are in DjVu native space (bottom-left origin).
/// Integration with the text layer coordinate system requires manual remap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Shape of a maparea.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shape {
    Rect(Rect),
    Oval(Rect),
    Poly(Vec<(u32, u32)>),
    Line(u32, u32, u32, u32),
    Text(Rect),
}

/// A border style (currently stored as a raw string for forward-compat).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Border {
    pub style: String,
}

/// A highlight color for a maparea.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Highlight {
    pub color: Color,
}

/// A clickable map area (hyperlink or highlight region) in a DjVu page.
#[derive(Debug, Clone)]
pub struct MapArea {
    /// Target URL (empty string if no link).
    pub url: String,
    /// Human-readable description.
    pub description: String,
    /// Shape of the area.
    pub shape: Shape,
    /// Optional border style.
    pub border: Option<Border>,
    /// Optional highlight color.
    pub highlight: Option<Highlight>,
}

/// Page-level annotation data.
#[derive(Debug, Clone, Default)]
pub struct Annotation {
    /// Background color for the page view.
    pub background: Option<Color>,
    /// Zoom level (percentage, e.g. 100 = 100%).
    pub zoom: Option<u32>,
    /// Display mode string (e.g. "color", "bw", "fore", "back").
    pub mode: Option<String>,
}

// ---- Entry points -----------------------------------------------------------

/// Parse an ANTa (plain-text) annotation chunk.
pub fn parse_annotations(data: &[u8]) -> Result<(Annotation, Vec<MapArea>), AnnotationError> {
    let text = core::str::from_utf8(data).unwrap_or("");
    parse_annotation_text(text)
}

/// Parse an ANTz (BZZ-compressed) annotation chunk.
pub fn parse_annotations_bzz(data: &[u8]) -> Result<(Annotation, Vec<MapArea>), AnnotationError> {
    let decoded = bzz_decode(data)?;
    let text = core::str::from_utf8(&decoded).unwrap_or("");
    parse_annotation_text(text)
}

// ---- S-expression tokenizer -------------------------------------------------

/// Minimal S-expression token.
#[derive(Debug, PartialEq)]
enum Token<'a> {
    LParen,
    RParen,
    Atom(&'a str),
    Quoted(String),
}

/// Tokenize an S-expression string into a flat Vec of tokens.
fn tokenize(input: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        match bytes.get(i) {
            Some(b'(') => {
                tokens.push(Token::LParen);
                i += 1;
            }
            Some(b')') => {
                tokens.push(Token::RParen);
                i += 1;
            }
            Some(b'"') => {
                i += 1;
                let start = i;
                let mut s = String::new();
                while i < bytes.len() {
                    match bytes.get(i) {
                        Some(b'\\') if i + 1 < bytes.len() => {
                            i += 1;
                            if let Some(&c) = bytes.get(i) {
                                s.push(c as char);
                            }
                            i += 1;
                        }
                        Some(b'"') => {
                            i += 1;
                            break;
                        }
                        Some(&c) => {
                            s.push(c as char);
                            i += 1;
                        }
                        None => break,
                    }
                }
                let _ = start; // consumed above
                tokens.push(Token::Quoted(s));
            }
            Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r') => {
                i += 1;
            }
            Some(b';') => {
                // line comment
                while i < bytes.len() && bytes.get(i) != Some(&b'\n') {
                    i += 1;
                }
            }
            _ => {
                let start = i;
                while i < bytes.len() {
                    match bytes.get(i) {
                        Some(b'(') | Some(b')') | Some(b'"') | Some(b' ') | Some(b'\t')
                        | Some(b'\n') | Some(b'\r') => break,
                        _ => i += 1,
                    }
                }
                if let Some(slice) = input.get(start..i)
                    && !slice.is_empty()
                {
                    tokens.push(Token::Atom(slice));
                }
            }
        }
    }

    tokens
}

// ---- S-expression tree ------------------------------------------------------

/// A node in the parsed S-expression tree.
#[derive(Debug)]
enum SExpr {
    Atom(String),
    List(Vec<SExpr>),
}

/// Parse tokens into a list of top-level S-expressions.
fn parse_sexprs(tokens: &[Token<'_>]) -> Vec<SExpr> {
    let mut result = Vec::new();
    let mut pos = 0usize;
    while pos < tokens.len() {
        if let Some(expr) = parse_one(tokens, &mut pos) {
            result.push(expr);
        }
    }
    result
}

fn parse_one(tokens: &[Token<'_>], pos: &mut usize) -> Option<SExpr> {
    match tokens.get(*pos) {
        Some(Token::LParen) => {
            *pos += 1;
            let mut items = Vec::new();
            loop {
                match tokens.get(*pos) {
                    Some(Token::RParen) => {
                        *pos += 1;
                        break;
                    }
                    None => break,
                    _ => {
                        if let Some(child) = parse_one(tokens, pos) {
                            items.push(child);
                        } else {
                            break;
                        }
                    }
                }
            }
            Some(SExpr::List(items))
        }
        Some(Token::RParen) => {
            // Unexpected RParen — skip
            *pos += 1;
            None
        }
        Some(Token::Atom(s)) => {
            let s = s.to_string();
            *pos += 1;
            Some(SExpr::Atom(s))
        }
        Some(Token::Quoted(s)) => {
            let s = s.clone();
            *pos += 1;
            Some(SExpr::Atom(s))
        }
        None => None,
    }
}

// ---- Annotation builder from S-expressions ----------------------------------

fn parse_annotation_text(text: &str) -> Result<(Annotation, Vec<MapArea>), AnnotationError> {
    if text.trim().is_empty() {
        return Ok((Annotation::default(), Vec::new()));
    }

    let tokens = tokenize(text);
    let exprs = parse_sexprs(&tokens);

    let mut annotation = Annotation::default();
    let mut mapareas = Vec::new();

    for expr in &exprs {
        if let SExpr::List(items) = expr {
            let head = match items.first() {
                Some(SExpr::Atom(s)) => s.as_str(),
                _ => continue,
            };

            match head {
                "background" => {
                    if let Some(SExpr::Atom(color_str)) = items.get(1) {
                        annotation.background = Some(parse_color(color_str)?);
                    }
                }
                "zoom" => {
                    if let Some(SExpr::Atom(n)) = items.get(1) {
                        annotation.zoom = Some(parse_uint(n)?);
                    }
                }
                "mode" => {
                    if let Some(SExpr::Atom(m)) = items.get(1) {
                        annotation.mode = Some(m.clone());
                    }
                }
                "maparea" => {
                    if let Some(ma) = parse_maparea(items)? {
                        mapareas.push(ma);
                    }
                }
                _ => {} // ignore unknown top-level forms
            }
        }
    }

    Ok((annotation, mapareas))
}

fn parse_maparea(items: &[SExpr]) -> Result<Option<MapArea>, AnnotationError> {
    // (maparea "url" "desc" (shape ...) [options...])
    let url = match items.get(1) {
        Some(SExpr::Atom(s)) => s.clone(),
        _ => String::new(),
    };
    let description = match items.get(2) {
        Some(SExpr::Atom(s)) => s.clone(),
        _ => String::new(),
    };

    let shape_expr = match items.get(3) {
        Some(SExpr::List(l)) => l,
        _ => return Ok(None),
    };

    let shape = parse_shape(shape_expr)?;

    // Optional border / highlight (items[4..])
    let mut border = None;
    let mut highlight = None;
    for item in items.get(4..).unwrap_or(&[]) {
        if let SExpr::List(opts) = item {
            match opts.first() {
                Some(SExpr::Atom(s)) if s == "border" => {
                    if let Some(SExpr::Atom(style)) = opts.get(1) {
                        border = Some(Border {
                            style: style.clone(),
                        });
                    }
                }
                Some(SExpr::Atom(s)) if s == "hilite" => {
                    if let Some(SExpr::Atom(color)) = opts.get(1) {
                        highlight = Some(Highlight {
                            color: parse_color(color)?,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    Ok(Some(MapArea {
        url,
        description,
        shape,
        border,
        highlight,
    }))
}

fn parse_shape(items: &[SExpr]) -> Result<Shape, AnnotationError> {
    let kind = match items.first() {
        Some(SExpr::Atom(s)) => s.as_str(),
        _ => return Err(AnnotationError::Parse("shape has no kind".to_string())),
    };

    match kind {
        "rect" => {
            let x = get_uint(items, 1)?;
            let y = get_uint(items, 2)?;
            let w = get_uint(items, 3)?;
            let h = get_uint(items, 4)?;
            Ok(Shape::Rect(Rect {
                x,
                y,
                width: w,
                height: h,
            }))
        }
        "oval" => {
            let x = get_uint(items, 1)?;
            let y = get_uint(items, 2)?;
            let w = get_uint(items, 3)?;
            let h = get_uint(items, 4)?;
            Ok(Shape::Oval(Rect {
                x,
                y,
                width: w,
                height: h,
            }))
        }
        "text" => {
            let x = get_uint(items, 1)?;
            let y = get_uint(items, 2)?;
            let w = get_uint(items, 3)?;
            let h = get_uint(items, 4)?;
            Ok(Shape::Text(Rect {
                x,
                y,
                width: w,
                height: h,
            }))
        }
        "line" => {
            let x1 = get_uint(items, 1)?;
            let y1 = get_uint(items, 2)?;
            let x2 = get_uint(items, 3)?;
            let y2 = get_uint(items, 4)?;
            Ok(Shape::Line(x1, y1, x2, y2))
        }
        "poly" => {
            // (poly x1 y1 x2 y2 ...)
            let mut pts = Vec::new();
            let mut i = 1usize;
            while i + 1 < items.len() {
                let x = get_uint(items, i)?;
                let y = get_uint(items, i + 1)?;
                pts.push((x, y));
                i += 2;
            }
            Ok(Shape::Poly(pts))
        }
        other => Err(AnnotationError::Parse(format!(
            "unknown shape kind: {other}"
        ))),
    }
}

// ---- Helpers ----------------------------------------------------------------

fn get_uint(items: &[SExpr], idx: usize) -> Result<u32, AnnotationError> {
    match items.get(idx) {
        Some(SExpr::Atom(s)) => parse_uint(s),
        _ => Err(AnnotationError::Parse(format!(
            "expected uint at position {idx}"
        ))),
    }
}

fn parse_uint(s: &str) -> Result<u32, AnnotationError> {
    s.parse::<u32>()
        .map_err(|_| AnnotationError::InvalidNumber(s.to_string()))
}

fn parse_color(s: &str) -> Result<Color, AnnotationError> {
    let hex = s.strip_prefix('#').unwrap_or(s);
    if hex.len() != 6 {
        return Err(AnnotationError::InvalidColor(s.to_string()));
    }
    let r = u8::from_str_radix(&hex[0..2], 16)
        .map_err(|_| AnnotationError::InvalidColor(s.to_string()))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|_| AnnotationError::InvalidColor(s.to_string()))?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .map_err(|_| AnnotationError::InvalidColor(s.to_string()))?;
    Ok(Color { r, g, b })
}
