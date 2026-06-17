//! hOCR and ALTO XML serialization for the DjVu text layer.
//!
//! This module does **not** run OCR. It serializes the structured
//! [`TextLayer`] / [`TextZone`] hierarchy already present in a document's
//! TXTz/TXTa chunk into two widely-used interchange formats:
//!
//! - **hOCR** — HTML micro-format used by Tesseract, Google Books, Internet Archive.
//! - **ALTO XML** — ISO 25577:2013 standard used by national libraries (LoC, Europeana, BnF).
//!
//! Recognition (producing a text layer from page images) lives in the
//! [`crate::ocr`] cluster; this module is purely the text-layer → markup half.
//!
//! ## Key public types
//!
//! - [`HocrOptions`] — options for hOCR output (page selection, DPI scale)
//! - [`AltoOptions`] — options for ALTO output (page selection, DPI scale)
//! - [`to_hocr`] — generate hOCR HTML string for a document
//! - [`to_alto`] — generate ALTO XML string for a document
//! - [`TextSerializeError`] — typed errors from this module
//!
//! [`HocrOptions`]: crate::text_serialize::HocrOptions
//! [`AltoOptions`]: crate::text_serialize::AltoOptions
//! [`to_hocr`]: crate::text_serialize::to_hocr
//! [`to_alto`]: crate::text_serialize::to_alto
//! [`TextSerializeError`]: crate::text_serialize::TextSerializeError

use std::fmt::Write as FmtWrite;

use crate::djvu_document::DjVuDocument;
use crate::text::{TextLayer, TextZone, TextZoneKind};

// ---- Error ------------------------------------------------------------------

/// Errors from text-layer serialization.
#[derive(Debug, thiserror::Error)]
pub enum TextSerializeError {
    /// Accessing a page failed.
    #[error("document error: {0}")]
    Doc(#[from] crate::djvu_document::DocError),

    /// Text layer extraction failed.
    #[error("text layer error: {0}")]
    Text(#[from] crate::text::TextError),

    /// String formatting error (infallible in practice).
    #[error("format error: {0}")]
    Fmt(#[from] std::fmt::Error),
}

/// Deprecated alias for [`TextSerializeError`], kept so code written against the
/// former `ocr_export` module name still compiles.
#[deprecated(since = "0.21.0", note = "renamed to `TextSerializeError`")]
pub type OcrExportError = TextSerializeError;

// ---- Options ----------------------------------------------------------------

/// Options for hOCR output.
#[derive(Debug, Clone, Default)]
pub struct HocrOptions {
    /// If `Some(n)`, only include page `n` (0-based). Default: all pages.
    pub page_index: Option<usize>,
    /// Target DPI for coordinate scaling. When set, page dimensions and all
    /// text zone coordinates are scaled so that 1 pixel equals 1/dpi inches
    /// at the specified resolution. Useful when the hOCR output must align
    /// with a rendered image at a specific DPI. When `None`, coordinates are
    /// emitted in native page pixels.
    pub dpi: Option<u32>,
}

/// Options for ALTO XML output.
#[derive(Debug, Clone, Default)]
pub struct AltoOptions {
    /// If `Some(n)`, only include page `n` (0-based). Default: all pages.
    pub page_index: Option<usize>,
    /// Target DPI for coordinate scaling. When set, page dimensions and all
    /// text zone coordinates are scaled so that 1 pixel equals 1/dpi inches
    /// at the specified resolution. Useful when the ALTO output must align
    /// with a rendered image at a specific DPI. When `None`, coordinates are
    /// emitted in native page pixels.
    pub dpi: Option<u32>,
}

// ---- Public API -------------------------------------------------------------

/// Generate hOCR HTML for the text layer of a [`DjVuDocument`].
///
/// Returns the complete HTML document as a `String`. Pages without a text
/// layer produce an empty `ocr_page` div (with correct dimensions) so that
/// the page count in the output always matches the document.
///
/// # Errors
///
/// Returns [`TextSerializeError`] if a page cannot be accessed or its text layer
/// cannot be decoded.
pub fn to_hocr(doc: &DjVuDocument, opts: &HocrOptions) -> Result<String, TextSerializeError> {
    let mut out = String::with_capacity(4096);

    writeln!(out, "<!DOCTYPE html>")?;
    writeln!(out, r#"<html xmlns="http://www.w3.org/1999/xhtml">"#)?;
    writeln!(out, "<head>")?;
    writeln!(out, r#"  <meta charset="utf-8"/>"#)?;
    writeln!(out, r#"  <meta name="ocr-system" content="djvu-rs"/>"#)?;
    writeln!(
        out,
        r#"  <meta name="ocr-capabilities" content="ocr_page ocr_block ocr_par ocr_line ocrx_word"/>"#
    )?;
    writeln!(out, "</head>")?;
    writeln!(out, "<body>")?;

    for page_idx in crate::export_common::page_indices(doc, opts.page_index) {
        let page = doc.page(page_idx)?;
        let (out_w, out_h) = match opts.dpi {
            Some(target_dpi) => crate::export_common::size_at_dpi(page, target_dpi as f32),
            None => (page.width() as u32, page.height() as u32),
        };

        // bbox for the full page
        write!(
            out,
            r#"  <div class="ocr_page" id="page_{idx}" title="image page_{idx}.djvu; bbox 0 0 {w} {h}; ppageno {idx}">"#,
            idx = page_idx,
            w = out_w,
            h = out_h,
        )?;
        writeln!(out)?;

        let layer_opt = if opts.dpi.is_some() {
            page.text_layer_at_size(out_w, out_h)?
        } else {
            page.text_layer()?
        };
        if let Some(layer) = layer_opt {
            write_hocr_zones(&mut out, &layer, page_idx)?;
        }

        writeln!(out, "  </div>")?;
    }

    writeln!(out, "</body>")?;
    writeln!(out, "</html>")?;

    Ok(out)
}

/// Generate ALTO XML for the text layer of a [`DjVuDocument`].
///
/// Returns a complete ALTO 4.x XML document as a `String`.
///
/// # Errors
///
/// Returns [`TextSerializeError`] if a page cannot be accessed or its text layer
/// cannot be decoded.
pub fn to_alto(doc: &DjVuDocument, opts: &AltoOptions) -> Result<String, TextSerializeError> {
    let mut out = String::with_capacity(4096);

    writeln!(out, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
    writeln!(
        out,
        r#"<alto xmlns="http://www.loc.gov/standards/alto/ns-v4#""#
    )?;
    writeln!(
        out,
        r#"      xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance""#
    )?;
    writeln!(
        out,
        r#"      xsi:schemaLocation="http://www.loc.gov/standards/alto/ns-v4# https://www.loc.gov/standards/alto/v4/alto.xsd">"#
    )?;
    writeln!(out, "  <Description>")?;
    writeln!(out, "    <MeasurementUnit>pixel</MeasurementUnit>")?;
    writeln!(out, "    <sourceImageInformation>")?;
    writeln!(out, "      <fileName>document.djvu</fileName>")?;
    writeln!(out, "    </sourceImageInformation>")?;
    writeln!(out, "  </Description>")?;
    writeln!(out, "  <Layout>")?;

    for page_idx in crate::export_common::page_indices(doc, opts.page_index) {
        let page = doc.page(page_idx)?;
        let (out_w, out_h) = match opts.dpi {
            Some(target_dpi) => crate::export_common::size_at_dpi(page, target_dpi as f32),
            None => (page.width() as u32, page.height() as u32),
        };

        writeln!(
            out,
            r#"    <Page ID="page_{idx}" WIDTH="{w}" HEIGHT="{h}" PHYSICAL_IMG_NR="{idx}">"#,
            idx = page_idx,
            w = out_w,
            h = out_h,
        )?;
        writeln!(
            out,
            "      <PrintSpace WIDTH=\"{w}\" HEIGHT=\"{h}\" HPOS=\"0\" VPOS=\"0\">",
            w = out_w,
            h = out_h
        )?;

        let layer_opt = if opts.dpi.is_some() {
            page.text_layer_at_size(out_w, out_h)?
        } else {
            page.text_layer()?
        };
        if let Some(layer) = layer_opt {
            write_alto_zones(&mut out, &layer, page_idx)?;
        }

        writeln!(out, "      </PrintSpace>")?;
        writeln!(out, "    </Page>")?;
    }

    writeln!(out, "  </Layout>")?;
    writeln!(out, "</alto>")?;

    Ok(out)
}

// ---- hOCR helpers -----------------------------------------------------------

fn write_hocr_zones(
    out: &mut String,
    layer: &TextLayer,
    page_idx: usize,
) -> Result<(), TextSerializeError> {
    let mut block_id = 0usize;
    let mut line_id = 0usize;
    let mut word_id = 0usize;

    for zone in &layer.zones {
        write_hocr_zone(
            out,
            zone,
            page_idx,
            &mut block_id,
            &mut line_id,
            &mut word_id,
            3,
        )?;
    }
    Ok(())
}

fn write_hocr_zone(
    out: &mut String,
    zone: &TextZone,
    page_idx: usize,
    block_id: &mut usize,
    line_id: &mut usize,
    word_id: &mut usize,
    indent: usize,
) -> Result<(), TextSerializeError> {
    let pad = " ".repeat(indent);
    let r = &zone.rect;
    let bbox = format!("bbox {} {} {} {}", r.x, r.y, r.x + r.width, r.y + r.height);

    match zone.kind {
        TextZoneKind::Page => {
            // Page zone is handled by the caller
            for child in &zone.children {
                write_hocr_zone(out, child, page_idx, block_id, line_id, word_id, indent)?;
            }
        }
        TextZoneKind::Column | TextZoneKind::Region => {
            let id = *block_id;
            *block_id += 1;
            writeln!(
                out,
                r#"{pad}<div class="ocr_block" id="block_{page}_{id}" title="{bbox}">"#,
                page = page_idx
            )?;
            for child in &zone.children {
                write_hocr_zone(out, child, page_idx, block_id, line_id, word_id, indent + 2)?;
            }
            writeln!(out, "{pad}</div>")?;
        }
        TextZoneKind::Para => {
            let id = *block_id;
            *block_id += 1;
            writeln!(
                out,
                r#"{pad}<p class="ocr_par" id="par_{page}_{id}" title="{bbox}">"#,
                page = page_idx
            )?;
            for child in &zone.children {
                write_hocr_zone(out, child, page_idx, block_id, line_id, word_id, indent + 2)?;
            }
            writeln!(out, "{pad}</p>")?;
        }
        TextZoneKind::Line => {
            let id = *line_id;
            *line_id += 1;
            writeln!(
                out,
                r#"{pad}<span class="ocr_line" id="line_{page}_{id}" title="{bbox}">"#,
                page = page_idx
            )?;
            for child in &zone.children {
                write_hocr_zone(out, child, page_idx, block_id, line_id, word_id, indent + 2)?;
            }
            writeln!(out, "{pad}</span>")?;
        }
        TextZoneKind::Word => {
            let id = *word_id;
            *word_id += 1;
            let text = escape_markup(&zone.text);
            writeln!(
                out,
                r#"{pad}<span class="ocrx_word" id="word_{page}_{id}" title="{bbox}">{text}</span>"#,
                page = page_idx
            )?;
            // Words may have character children — skip sub-word nesting in hOCR
        }
        TextZoneKind::Character => {
            // Characters are not a standard hOCR class; skip.
        }
    }
    Ok(())
}

/// Escape text for embedding in hOCR (XHTML) or ALTO (XML) markup.
///
/// Uses the numeric character reference `&#39;` for the apostrophe rather than
/// the named `&apos;`: `&#39;` is valid in both XHTML and XML, so one escaper
/// serves both writers (the two previously disagreed on this single character).
fn escape_markup(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '&' => "&amp;".chars().collect::<Vec<_>>(),
            '<' => "&lt;".chars().collect(),
            '>' => "&gt;".chars().collect(),
            '"' => "&quot;".chars().collect(),
            '\'' => "&#39;".chars().collect(),
            c => vec![c],
        })
        .collect()
}

// ---- ALTO helpers -----------------------------------------------------------

fn write_alto_zones(
    out: &mut String,
    layer: &TextLayer,
    page_idx: usize,
) -> Result<(), TextSerializeError> {
    let mut block_id = 0usize;
    let mut line_id = 0usize;
    let mut word_id = 0usize;

    for zone in &layer.zones {
        write_alto_zone(
            out,
            zone,
            page_idx,
            &mut block_id,
            &mut line_id,
            &mut word_id,
            4,
        )?;
    }
    Ok(())
}

fn write_alto_zone(
    out: &mut String,
    zone: &TextZone,
    page_idx: usize,
    block_id: &mut usize,
    line_id: &mut usize,
    word_id: &mut usize,
    indent: usize,
) -> Result<(), TextSerializeError> {
    let pad = " ".repeat(indent);
    let r = &zone.rect;

    match zone.kind {
        TextZoneKind::Page => {
            for child in &zone.children {
                write_alto_zone(out, child, page_idx, block_id, line_id, word_id, indent)?;
            }
        }
        TextZoneKind::Column | TextZoneKind::Region | TextZoneKind::Para => {
            let id = *block_id;
            *block_id += 1;
            writeln!(
                out,
                r#"{pad}<TextBlock ID="block_{page}_{id}" HPOS="{hpos}" VPOS="{vpos}" WIDTH="{w}" HEIGHT="{h}">"#,
                page = page_idx,
                hpos = r.x,
                vpos = r.y,
                w = r.width,
                h = r.height,
            )?;
            for child in &zone.children {
                write_alto_zone(out, child, page_idx, block_id, line_id, word_id, indent + 2)?;
            }
            writeln!(out, "{pad}</TextBlock>")?;
        }
        TextZoneKind::Line => {
            let id = *line_id;
            *line_id += 1;
            writeln!(
                out,
                r#"{pad}<TextLine ID="line_{page}_{id}" HPOS="{hpos}" VPOS="{vpos}" WIDTH="{w}" HEIGHT="{h}">"#,
                page = page_idx,
                hpos = r.x,
                vpos = r.y,
                w = r.width,
                h = r.height,
            )?;
            for child in &zone.children {
                write_alto_zone(out, child, page_idx, block_id, line_id, word_id, indent + 2)?;
            }
            writeln!(out, "{pad}</TextLine>")?;
        }
        TextZoneKind::Word => {
            let id = *word_id;
            *word_id += 1;
            let text = escape_markup(&zone.text);
            writeln!(
                out,
                r#"{pad}<String ID="word_{page}_{id}" HPOS="{hpos}" VPOS="{vpos}" WIDTH="{w}" HEIGHT="{h}" CONTENT="{text}"/>"#,
                page = page_idx,
                hpos = r.x,
                vpos = r.y,
                w = r.width,
                h = r.height,
            )?;
        }
        TextZoneKind::Character => {
            // Glyph-level elements not included in the basic ALTO export.
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{Rect, TextLayer, TextZone, TextZoneKind};

    fn word_zone(text: &str, x: u32, y: u32, w: u32, h: u32) -> TextZone {
        TextZone {
            kind: TextZoneKind::Word,
            rect: Rect {
                x,
                y,
                width: w,
                height: h,
            },
            text: text.to_string(),
            children: vec![],
        }
    }

    fn line_zone(words: Vec<TextZone>, x: u32, y: u32, w: u32, h: u32) -> TextZone {
        TextZone {
            kind: TextZoneKind::Line,
            rect: Rect {
                x,
                y,
                width: w,
                height: h,
            },
            text: words
                .iter()
                .map(|z| z.text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            children: words,
        }
    }

    fn page_zone(children: Vec<TextZone>) -> TextZone {
        TextZone {
            kind: TextZoneKind::Page,
            rect: Rect {
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            },
            text: String::new(),
            children,
        }
    }

    fn simple_layer(words: &[(&str, u32, u32, u32, u32)]) -> TextLayer {
        let word_zones: Vec<_> = words
            .iter()
            .map(|&(t, x, y, w, h)| word_zone(t, x, y, w, h))
            .collect();
        let line = line_zone(word_zones, 0, 0, 800, 40);
        TextLayer {
            text: words.iter().map(|w| w.0).collect::<Vec<_>>().join(" "),
            zones: vec![page_zone(vec![line])],
        }
    }

    // ---- escape_markup -------------------------------------------------------

    #[test]
    fn escape_ampersand() {
        assert_eq!(escape_markup("a & b"), "a &amp; b");
    }

    #[test]
    fn escape_less_than() {
        assert_eq!(escape_markup("<tag>"), "&lt;tag&gt;");
    }

    #[test]
    fn escape_quote() {
        assert_eq!(escape_markup(r#"say "hi""#), "say &quot;hi&quot;");
    }

    #[test]
    fn escape_apostrophe_uses_numeric_ref() {
        assert_eq!(escape_markup("it's"), "it&#39;s");
    }

    #[test]
    fn escape_plain_text_unchanged() {
        assert_eq!(escape_markup("hello world"), "hello world");
    }

    #[test]
    fn escape_empty() {
        assert_eq!(escape_markup(""), "");
    }

    // ---- hOCR ---------------------------------------------------------------

    #[test]
    fn hocr_word_contains_class_and_text() {
        let layer = simple_layer(&[("hello", 10, 20, 50, 15)]);
        let mut out = String::new();
        write_hocr_zones(&mut out, &layer, 0).unwrap();
        assert!(out.contains("ocrx_word"), "expected ocrx_word class");
        assert!(out.contains("hello"), "expected word text");
    }

    #[test]
    fn hocr_word_bbox_format() {
        // bbox in hOCR is "x1 y1 x2 y2"
        let layer = simple_layer(&[("foo", 10, 20, 30, 10)]);
        let mut out = String::new();
        write_hocr_zones(&mut out, &layer, 0).unwrap();
        // x2 = x + width = 10 + 30 = 40, y2 = 20 + 10 = 30
        assert!(
            out.contains("bbox 10 20 40 30"),
            "expected bbox 10 20 40 30, got: {out}"
        );
    }

    #[test]
    fn hocr_word_ids_increment() {
        let layer = simple_layer(&[("a", 0, 0, 10, 10), ("b", 20, 0, 10, 10)]);
        let mut out = String::new();
        write_hocr_zones(&mut out, &layer, 0).unwrap();
        assert!(out.contains("word_0_0"));
        assert!(out.contains("word_0_1"));
    }

    #[test]
    fn hocr_page_index_in_ids() {
        let layer = simple_layer(&[("x", 0, 0, 10, 10)]);
        let mut out = String::new();
        write_hocr_zones(&mut out, &layer, 3).unwrap();
        assert!(
            out.contains("word_3_0"),
            "page index should appear in id: {out}"
        );
    }

    #[test]
    fn hocr_escapes_special_chars_in_text() {
        let layer = simple_layer(&[("a&b", 0, 0, 10, 10)]);
        let mut out = String::new();
        write_hocr_zones(&mut out, &layer, 0).unwrap();
        assert!(out.contains("a&amp;b"), "expected escaped ampersand: {out}");
        assert!(!out.contains(" a&b "), "unescaped text must not appear");
    }

    #[test]
    fn hocr_line_zone_has_ocr_line_class() {
        let layer = simple_layer(&[("w", 0, 0, 50, 20)]);
        let mut out = String::new();
        write_hocr_zones(&mut out, &layer, 0).unwrap();
        assert!(out.contains("ocr_line"), "expected ocr_line class: {out}");
    }

    // ---- ALTO ---------------------------------------------------------------

    #[test]
    fn alto_word_has_string_element() {
        let layer = simple_layer(&[("hello", 5, 10, 40, 12)]);
        let mut out = String::new();
        write_alto_zones(&mut out, &layer, 0).unwrap();
        assert!(out.contains("<String"), "expected String element: {out}");
        assert!(
            out.contains(r#"CONTENT="hello""#),
            "expected CONTENT attr: {out}"
        );
    }

    #[test]
    fn alto_word_hpos_vpos_width_height() {
        let layer = simple_layer(&[("w", 5, 10, 40, 12)]);
        let mut out = String::new();
        write_alto_zones(&mut out, &layer, 0).unwrap();
        assert!(out.contains(r#"HPOS="5""#));
        assert!(out.contains(r#"VPOS="10""#));
        assert!(out.contains(r#"WIDTH="40""#));
        assert!(out.contains(r#"HEIGHT="12""#));
    }

    #[test]
    fn alto_word_ids_increment() {
        let layer = simple_layer(&[("a", 0, 0, 10, 10), ("b", 20, 0, 10, 10)]);
        let mut out = String::new();
        write_alto_zones(&mut out, &layer, 0).unwrap();
        assert!(out.contains(r#"ID="word_0_0""#));
        assert!(out.contains(r#"ID="word_0_1""#));
    }

    #[test]
    fn alto_line_has_textline_element() {
        let layer = simple_layer(&[("w", 0, 0, 50, 20)]);
        let mut out = String::new();
        write_alto_zones(&mut out, &layer, 0).unwrap();
        assert!(
            out.contains("<TextLine"),
            "expected TextLine element: {out}"
        );
        assert!(
            out.contains("</TextLine>"),
            "expected closing TextLine: {out}"
        );
    }

    #[test]
    fn alto_escapes_special_chars_in_content() {
        let layer = simple_layer(&[("it's", 0, 0, 30, 10)]);
        let mut out = String::new();
        write_alto_zones(&mut out, &layer, 0).unwrap();
        assert!(out.contains("&#39;"), "expected escaped apostrophe: {out}");
    }

    #[test]
    fn alto_page_index_in_word_id() {
        let layer = simple_layer(&[("x", 0, 0, 10, 10)]);
        let mut out = String::new();
        write_alto_zones(&mut out, &layer, 5).unwrap();
        assert!(
            out.contains(r#"ID="word_5_0""#),
            "page index in word id: {out}"
        );
    }
}
