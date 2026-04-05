//! hOCR and ALTO XML export for the DjVu text layer.
//!
//! Converts the structured [`TextLayer`] / [`TextZone`] hierarchy into two
//! widely-used OCR interchange formats:
//!
//! - **hOCR** — HTML micro-format used by Tesseract, Google Books, Internet Archive.
//! - **ALTO XML** — ISO 25577:2013 standard used by national libraries (LoC, Europeana, BnF).
//!
//! ## Key public types
//!
//! - [`HocrOptions`] — options for hOCR output (page selection, DPI scale)
//! - [`AltoOptions`] — options for ALTO output (page selection, DPI scale)
//! - [`to_hocr`] — generate hOCR HTML string for a document
//! - [`to_alto`] — generate ALTO XML string for a document
//! - [`OcrExportError`] — typed errors from this module

use std::fmt::Write as FmtWrite;

use crate::djvu_document::DjVuDocument;
use crate::text::{TextLayer, TextZone, TextZoneKind};

// ---- Error ------------------------------------------------------------------

/// Errors from OCR export.
#[derive(Debug, thiserror::Error)]
pub enum OcrExportError {
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

// ---- Options ----------------------------------------------------------------

/// Options for hOCR output.
#[derive(Debug, Clone, Default)]
pub struct HocrOptions {
    /// If `Some(n)`, only include page `n` (0-based). Default: all pages.
    pub page_index: Option<usize>,
    /// DPI to use for coordinate scaling. Default: native page DPI.
    pub dpi: Option<u32>,
}

/// Options for ALTO XML output.
#[derive(Debug, Clone, Default)]
pub struct AltoOptions {
    /// If `Some(n)`, only include page `n` (0-based). Default: all pages.
    pub page_index: Option<usize>,
    /// DPI to use for coordinate scaling. Default: native page DPI.
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
/// Returns [`OcrExportError`] if a page cannot be accessed or its text layer
/// cannot be decoded.
pub fn to_hocr(doc: &DjVuDocument, opts: &HocrOptions) -> Result<String, OcrExportError> {
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

    let page_range: Box<dyn Iterator<Item = usize>> = match opts.page_index {
        Some(i) => Box::new(std::iter::once(i)),
        None => Box::new(0..doc.page_count()),
    };

    for page_idx in page_range {
        let page = doc.page(page_idx)?;
        let pw = page.width() as u32;
        let ph = page.height() as u32;

        // bbox for the full page
        write!(
            out,
            r#"  <div class="ocr_page" id="page_{idx}" title="image page_{idx}.djvu; bbox 0 0 {w} {h}; ppageno {idx}">"#,
            idx = page_idx,
            w = pw,
            h = ph,
        )?;
        writeln!(out)?;

        if let Some(layer) = page.text_layer()? {
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
/// Returns [`OcrExportError`] if a page cannot be accessed or its text layer
/// cannot be decoded.
pub fn to_alto(doc: &DjVuDocument, opts: &AltoOptions) -> Result<String, OcrExportError> {
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

    let page_range: Box<dyn Iterator<Item = usize>> = match opts.page_index {
        Some(i) => Box::new(std::iter::once(i)),
        None => Box::new(0..doc.page_count()),
    };

    for page_idx in page_range {
        let page = doc.page(page_idx)?;
        let pw = page.width() as u32;
        let ph = page.height() as u32;

        writeln!(
            out,
            r#"    <Page ID="page_{idx}" WIDTH="{w}" HEIGHT="{h}" PHYSICAL_IMG_NR="{idx}">"#,
            idx = page_idx,
            w = pw,
            h = ph,
        )?;
        writeln!(
            out,
            "      <PrintSpace WIDTH=\"{w}\" HEIGHT=\"{h}\" HPOS=\"0\" VPOS=\"0\">",
            w = pw,
            h = ph
        )?;

        if let Some(layer) = page.text_layer()? {
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
) -> Result<(), OcrExportError> {
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
) -> Result<(), OcrExportError> {
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
            let text = escape_html(&zone.text);
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

fn escape_html(s: &str) -> String {
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
) -> Result<(), OcrExportError> {
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
) -> Result<(), OcrExportError> {
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
            let text = escape_xml(&zone.text);
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

fn escape_xml(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '&' => "&amp;".chars().collect::<Vec<_>>(),
            '<' => "&lt;".chars().collect(),
            '>' => "&gt;".chars().collect(),
            '"' => "&quot;".chars().collect(),
            '\'' => "&apos;".chars().collect(),
            c => vec![c],
        })
        .collect()
}
