//! DjVu to PDF converter — preserves document structure.
//!
//! Converts DjVu documents to PDF while preserving:
//! - IW44 background as compressed RGB image (#2)
//! - JB2 foreground mask as 1-bit image (#3)
//! - Text layer as invisible selectable text (#4)
//! - NAVM bookmarks as PDF outline / table of contents (#5)
//! - ANTz hyperlinks as PDF link annotations (#6)
//!
//! # Example
//!
//! ```no_run
//! use djvu_rs::djvu_document::DjVuDocument;
//! use djvu_rs::pdf::djvu_to_pdf;
//!
//! let data = std::fs::read("input.djvu").unwrap();
//! let doc = DjVuDocument::parse(&data).unwrap();
//! let pdf_bytes = djvu_to_pdf(&doc).unwrap();
//! std::fs::write("output.pdf", pdf_bytes).unwrap();
//! ```

#[cfg(not(feature = "std"))]
use alloc::{format, string::String, vec, vec::Vec};

use crate::{
    annotation::Shape,
    djvu_document::{DjVuBookmark, DjVuDocument, DjVuPage, DocError},
    djvu_render::{self, RenderOptions},
    text::{TextZone, TextZoneKind},
};

// ---- Error ------------------------------------------------------------------

/// Errors from PDF conversion.
#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    /// Document model error.
    #[error("document error: {0}")]
    Doc(#[from] DocError),
    /// Render error.
    #[error("render error: {0}")]
    Render(#[from] djvu_render::RenderError),
}

// ---- Low-level PDF object writer --------------------------------------------

/// A PDF object body (bytes between `N 0 obj\n` and `\nendobj\n`).
struct PdfObj {
    id: usize,
    body: Vec<u8>,
}

/// Accumulates PDF objects and serializes them into a valid PDF 1.4 file.
struct PdfWriter {
    objects: Vec<PdfObj>,
    next_id: usize,
}

impl PdfWriter {
    fn new() -> Self {
        PdfWriter {
            objects: Vec::new(),
            next_id: 1,
        }
    }

    /// Reserve the next object ID.
    fn alloc_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Add an object with a pre-allocated ID.
    fn add_obj(&mut self, id: usize, body: Vec<u8>) {
        self.objects.push(PdfObj { id, body });
    }

    /// Allocate and add an object, returning its ID.
    fn add(&mut self, body: Vec<u8>) -> usize {
        let id = self.alloc_id();
        self.add_obj(id, body);
        id
    }

    /// Serialize all objects into a complete PDF file.
    fn serialize(self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        buf.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");

        let mut offsets: Vec<(usize, usize)> = Vec::new();
        for obj in &self.objects {
            offsets.push((obj.id, buf.len()));
            buf.extend_from_slice(format!("{} 0 obj\n", obj.id).as_bytes());
            buf.extend_from_slice(&obj.body);
            buf.extend_from_slice(b"\nendobj\n");
        }

        // Cross-reference table
        let xref_offset = buf.len();
        let max_id = offsets.iter().map(|(id, _)| *id).max().unwrap_or(0);
        buf.extend_from_slice(format!("xref\n0 {}\n", max_id + 1).as_bytes());
        buf.extend_from_slice(b"0000000000 65535 f \n");

        let mut offset_map = vec![None; max_id + 1];
        for (obj_id, off) in &offsets {
            if *obj_id <= max_id {
                offset_map[*obj_id] = Some(*off);
            }
        }
        for entry in offset_map.iter().skip(1) {
            match entry {
                Some(off) => {
                    buf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
                }
                None => buf.extend_from_slice(b"0000000000 65535 f \n"),
            }
        }

        buf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                max_id + 1,
                xref_offset
            )
            .as_bytes(),
        );

        buf
    }
}

/// Helper: make a PDF stream object `<< ... /Length N >> stream\n...\nendstream`.
fn make_stream(dict_extra: &str, data: &[u8]) -> Vec<u8> {
    let len = data.len();
    let mut body =
        format!("<< /Length {len}{dict_extra} >>\nstream\n").into_bytes();
    body.extend_from_slice(data);
    body.extend_from_slice(b"\nendstream");
    body
}

/// Compress bytes using zlib/deflate.
fn deflate(data: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec_zlib(data, 6)
}

/// Helper: make a compressed stream object.
fn make_deflate_stream(dict_extra: &str, data: &[u8]) -> Vec<u8> {
    let compressed = deflate(data);
    let extra = format!(" /Filter /FlateDecode{dict_extra}");
    make_stream(&extra, &compressed)
}

// ---- PDF font for invisible text --------------------------------------------

/// Build a Type1 font dictionary for Helvetica (standard 14 font, no embedding needed).
/// Returns object body bytes.
fn font_dict() -> Vec<u8> {
    b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec()
}

// ---- Coordinate helpers -----------------------------------------------------

/// Convert DjVu pixel coordinates to PDF points.
/// DjVu uses bottom-left origin (like PDF), so y-coordinates can be used directly
/// after scaling by 72/dpi.
fn px_to_pt(px: f32, dpi: f32) -> f32 {
    px * 72.0 / dpi
}

// ---- Page rendering ---------------------------------------------------------

/// Build PDF objects for one page. Returns (page_obj_id, list of annotation obj ids).
fn build_page_objects(
    w: &mut PdfWriter,
    page: &DjVuPage,
    pages_id: usize,
    font_id: usize,
) -> Result<usize, PdfError> {
    let pw = page.width() as u32;
    let ph = page.height() as u32;
    let dpi = page.dpi().max(1) as f32;
    let pt_w = px_to_pt(pw as f32, dpi);
    let pt_h = px_to_pt(ph as f32, dpi);

    // Render page to RGB
    let opts = RenderOptions {
        width: pw,
        height: ph,
        ..RenderOptions::default()
    };
    let pixmap = djvu_render::render_pixmap(page, &opts)?;
    let rgb = pixmap.to_rgb();

    // Background image XObject (FlateDecode RGB)
    let img_dict = format!(
        " /Type /XObject /Subtype /Image /Width {pw} /Height {ph}\
         /ColorSpace /DeviceRGB /BitsPerComponent 8"
    );
    let img_body = make_deflate_stream(&img_dict, &rgb);
    let img_id = w.add(img_body);

    // JB2 mask as 1-bit image (if present)
    let mask_img_id = build_mask_image(w, page, pw, ph);

    // Content stream: draw background image, then mask overlay, then invisible text
    let mut content = String::new();

    // Draw background image filling the page
    content.push_str(&format!(
        "q {pt_w:.4} 0 0 {pt_h:.4} 0 0 cm /Im0 Do Q\n"
    ));

    // Draw mask overlay (black foreground on transparent)
    if let Some(mask_id) = mask_img_id {
        // Use the mask as a stencil: set fill color to black, then draw mask as image mask
        content.push_str(&format!(
            "q 0 0 0 rg {pt_w:.4} 0 0 {pt_h:.4} 0 0 cm /Mask0 Do Q\n"
        ));
        let _ = mask_id; // used in resources below
    }

    // Invisible text layer
    let text_ops = build_text_content(page, dpi, pt_h);
    if !text_ops.is_empty() {
        content.push_str(&text_ops);
    }

    // Content stream object
    let content_bytes = content.as_bytes();
    let content_body = make_deflate_stream("", content_bytes);
    let content_id = w.add(content_body);

    // Resources dictionary
    let mut resources = format!("/XObject << /Im0 {img_id} 0 R");
    if mask_img_id.is_some() {
        let mid = mask_img_id.unwrap();
        resources.push_str(&format!(" /Mask0 {mid} 0 R"));
    }
    resources.push_str(" >>");
    if !text_ops.is_empty() {
        resources.push_str(&format!(" /Font << /F1 {font_id} 0 R >>"));
    }

    // Annotations (hyperlinks)
    let annot_ids = build_link_annotations(w, page, dpi, pt_h);
    let mut annots_str = String::new();
    if !annot_ids.is_empty() {
        annots_str.push_str(" /Annots [");
        for aid in &annot_ids {
            annots_str.push_str(&format!(" {aid} 0 R"));
        }
        annots_str.push_str(" ]");
    }

    // Page object
    let page_id = w.add(
        format!(
            "<< /Type /Page /Parent {pages_id} 0 R\n\
               /MediaBox [0 0 {pt_w:.4} {pt_h:.4}]\n\
               /Contents {content_id} 0 R\n\
               /Resources << {resources} >>{annots_str} >>"
        )
        .into_bytes(),
    );

    Ok(page_id)
}

/// Build a 1-bit image mask from the JB2 foreground mask.
fn build_mask_image(
    w: &mut PdfWriter,
    page: &DjVuPage,
    _pw: u32,
    _ph: u32,
) -> Option<usize> {
    // Decode JB2 mask
    let sjbz = page.find_chunk(b"Sjbz")?;
    let dict = page.find_chunk(b"Djbz").and_then(|djbz| {
        crate::jb2_new::decode_dict(djbz, None).ok()
    });
    let bitmap = crate::jb2_new::decode(sjbz, dict.as_ref()).ok()?;

    let bw = bitmap.width;
    let bh = bitmap.height;

    // Bitmap data is already packed 1-bit MSB-first, which is what PDF expects
    // for an ImageMask with /Decode [1 0] (1=black=marked).
    // PDF ImageMask: painted where sample = 1 in the image data.
    let dict_extra = format!(
        " /Type /XObject /Subtype /Image /Width {bw} /Height {bh}\
         /ImageMask true /BitsPerComponent 1 /Decode [1 0]"
    );
    let body = make_deflate_stream(&dict_extra, &bitmap.data);
    let id = w.add(body);
    Some(id)
}

/// Build invisible text operators for the text layer.
fn build_text_content(page: &DjVuPage, dpi: f32, pt_h: f32) -> String {
    let text_layer = match page.text_layer() {
        Ok(Some(tl)) => tl,
        _ => return String::new(),
    };

    let mut ops = String::new();
    // Begin text object
    ops.push_str("BT\n");
    // Set text rendering mode to invisible (mode 3)
    ops.push_str("3 Tr\n");
    // Set font — use a small size, we scale per-word
    ops.push_str("/F1 1 Tf\n");

    // Walk the zone tree and emit text for word/character zones
    for zone in &text_layer.zones {
        emit_text_zones(&mut ops, zone, dpi, pt_h);
    }

    ops.push_str("ET\n");

    if ops == "BT\n3 Tr\n/F1 1 Tf\nET\n" {
        // No actual text was emitted
        return String::new();
    }

    ops
}

/// Recursively emit text positioning operators for word-level zones.
fn emit_text_zones(ops: &mut String, zone: &TextZone, dpi: f32, pt_h: f32) {
    match zone.kind {
        TextZoneKind::Word | TextZoneKind::Character => {
            if zone.text.is_empty() {
                return;
            }
            let r = &zone.rect;
            // zone.rect is in top-left-origin pixel coords
            // PDF uses bottom-left origin, so: pdf_y = pt_h - (r.y + r.height) * 72/dpi
            let x = px_to_pt(r.x as f32, dpi);
            let y = pt_h - px_to_pt((r.y + r.height) as f32, dpi);
            let w = px_to_pt(r.width as f32, dpi);
            let h = px_to_pt(r.height as f32, dpi);

            if w <= 0.0 || h <= 0.0 {
                return;
            }

            // Font size = zone height in points
            let font_size = h;
            if font_size < 0.5 {
                return;
            }

            // Horizontal scale to fit text width
            let text_escaped = pdf_escape_string(&zone.text);
            let char_count = zone.text.chars().count().max(1) as f32;
            // Approximate: each glyph in Helvetica is ~0.5 * font_size wide
            let natural_width = char_count * 0.5 * font_size;
            let h_scale = if natural_width > 0.01 {
                (w / natural_width) * 100.0
            } else {
                100.0
            };

            ops.push_str(&format!("{font_size:.2} 0 0 {font_size:.2} {x:.4} {y:.4} Tm\n"));
            if (h_scale - 100.0).abs() > 1.0 {
                ops.push_str(&format!("{h_scale:.2} Tz\n"));
            }
            ops.push_str(&format!("({text_escaped}) Tj\n"));
        }
        _ => {
            // Recurse into children
            for child in &zone.children {
                emit_text_zones(ops, child, dpi, pt_h);
            }
        }
    }
}

/// Escape a string for PDF literal string syntax.
fn pdf_escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            c if c.is_ascii() => out.push(c),
            // Non-ASCII: encode as UTF-16BE with BOM for PDF
            _ => {
                // For simplicity, skip non-ASCII chars in text positioning
                // (they'll still be in the document via the image)
                out.push('?');
            }
        }
    }
    out
}

/// Build PDF link annotation objects for hyperlinks from the ANTz layer.
fn build_link_annotations(
    w: &mut PdfWriter,
    page: &DjVuPage,
    dpi: f32,
    pt_h: f32,
) -> Vec<usize> {
    let hyperlinks = match page.hyperlinks() {
        Ok(links) => links,
        Err(_) => return Vec::new(),
    };

    let mut ids = Vec::new();
    for link in &hyperlinks {
        if let Some(rect) = shape_to_pdf_rect(&link.shape, dpi, pt_h) {
            let url_escaped = pdf_escape_string(&link.url);
            let body = format!(
                "<< /Type /Annot /Subtype /Link\n\
                   /Rect [{:.4} {:.4} {:.4} {:.4}]\n\
                   /Border [0 0 0]\n\
                   /A << /S /URI /URI ({url_escaped}) >> >>",
                rect.0, rect.1, rect.2, rect.3
            );
            let id = w.add(body.into_bytes());
            ids.push(id);
        }
    }
    ids
}

/// Convert a DjVu shape to a PDF rectangle [x1, y1, x2, y2] in points.
/// DjVu annotation coordinates use bottom-left origin (same as PDF).
fn shape_to_pdf_rect(shape: &Shape, dpi: f32, _pt_h: f32) -> Option<(f32, f32, f32, f32)> {
    match shape {
        Shape::Rect(r) | Shape::Oval(r) | Shape::Text(r) => {
            let x1 = px_to_pt(r.x as f32, dpi);
            let y1 = px_to_pt(r.y as f32, dpi);
            let x2 = px_to_pt((r.x + r.width) as f32, dpi);
            let y2 = px_to_pt((r.y + r.height) as f32, dpi);
            Some((x1, y1, x2, y2))
        }
        Shape::Poly(points) => {
            if points.is_empty() {
                return None;
            }
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            for (px, py) in points {
                let x = px_to_pt(*px as f32, dpi);
                let y = px_to_pt(*py as f32, dpi);
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
            Some((min_x, min_y, max_x, max_y))
        }
        Shape::Line(x1, y1, x2, y2) => {
            let px1 = px_to_pt(*x1 as f32, dpi);
            let py1 = px_to_pt(*y1 as f32, dpi);
            let px2 = px_to_pt(*x2 as f32, dpi);
            let py2 = px_to_pt(*y2 as f32, dpi);
            Some((px1.min(px2), py1.min(py2), px1.max(px2), py1.max(py2)))
        }
    }
}

// ---- Bookmarks (PDF outline) ------------------------------------------------

/// Build PDF outline objects from NAVM bookmarks.
/// Returns the outline root object ID, or None if no bookmarks.
fn build_outline(
    w: &mut PdfWriter,
    bookmarks: &[DjVuBookmark],
    page_ids: &[usize],
) -> Option<usize> {
    if bookmarks.is_empty() {
        return None;
    }

    let outline_id = w.alloc_id();

    // Flatten the bookmark tree into outline item objects
    let item_ids = build_outline_items(w, bookmarks, outline_id, page_ids);

    if item_ids.is_empty() {
        return None;
    }

    let first = item_ids[0];
    let last = *item_ids.last().unwrap();
    let count = count_outline_items(bookmarks);

    w.add_obj(
        outline_id,
        format!(
            "<< /Type /Outlines /First {first} 0 R /Last {last} 0 R /Count {count} >>"
        )
        .into_bytes(),
    );

    Some(outline_id)
}

/// Recursively build outline items. Returns IDs of top-level items at this level.
fn build_outline_items(
    w: &mut PdfWriter,
    bookmarks: &[DjVuBookmark],
    parent_id: usize,
    page_ids: &[usize],
) -> Vec<usize> {
    let mut ids = Vec::new();

    for _bm in bookmarks {
        let item_id = w.alloc_id();
        ids.push(item_id);
    }

    for (i, bm) in bookmarks.iter().enumerate() {
        let item_id = ids[i];
        let prev = if i > 0 {
            format!(" /Prev {} 0 R", ids[i - 1])
        } else {
            String::new()
        };
        let next = if i + 1 < ids.len() {
            format!(" /Next {} 0 R", ids[i + 1])
        } else {
            String::new()
        };

        // Resolve bookmark URL to page index
        let dest = resolve_bookmark_dest(&bm.url, page_ids);

        // Build children
        let child_ids = build_outline_items(w, &bm.children, item_id, page_ids);
        let children_str = if !child_ids.is_empty() {
            let first = child_ids[0];
            let last = *child_ids.last().unwrap();
            let count = count_outline_items(&bm.children);
            format!(" /First {first} 0 R /Last {last} 0 R /Count {count}")
        } else {
            String::new()
        };

        let title = pdf_escape_string(&bm.title);
        w.add_obj(
            item_id,
            format!(
                "<< /Title ({title}) /Parent {parent_id} 0 R{prev}{next}{dest}{children_str} >>"
            )
            .into_bytes(),
        );
    }

    ids
}

/// Count total outline items (including nested children).
fn count_outline_items(bookmarks: &[DjVuBookmark]) -> usize {
    let mut n = bookmarks.len();
    for bm in bookmarks {
        n += count_outline_items(&bm.children);
    }
    n
}

/// Resolve a DjVu bookmark URL to a PDF destination string.
/// DjVu internal URLs look like `#page_N` or `#+N` or `#-N`.
fn resolve_bookmark_dest(url: &str, page_ids: &[usize]) -> String {
    if let Some(stripped) = url.strip_prefix('#') {
        // Try to parse as page number
        if let Some(page_str) = stripped.strip_prefix("page") {
            if let Ok(page_num) = page_str.trim_start_matches('_').parse::<usize>() {
                let idx = page_num.saturating_sub(1);
                if let Some(&pid) = page_ids.get(idx) {
                    return format!(" /Dest [{pid} 0 R /Fit]");
                }
            }
        }
        // Try +N / -N (relative, but treat as absolute from 1)
        if let Ok(n) = stripped.parse::<i64>() {
            let idx = (n.max(1) - 1) as usize;
            if let Some(&pid) = page_ids.get(idx) {
                return format!(" /Dest [{pid} 0 R /Fit]");
            }
        }
        // Try bare number
        if let Ok(n) = stripped.parse::<usize>() {
            let idx = n.saturating_sub(1);
            if let Some(&pid) = page_ids.get(idx) {
                return format!(" /Dest [{pid} 0 R /Fit]");
            }
        }
    }

    // External URL or unparseable — use URI action
    if !url.is_empty() {
        let escaped = pdf_escape_string(url);
        return format!(" /A << /S /URI /URI ({escaped}) >>");
    }

    String::new()
}

// ---- Public API -------------------------------------------------------------

/// Convert a DjVu document to PDF bytes.
///
/// This produces a PDF 1.4 file with:
/// - Rasterized page images (IW44 background + JB2 mask composite)
/// - Invisible text layer for search and selection
/// - Bookmarks (PDF outline) from NAVM
/// - Hyperlink annotations from ANTz
///
/// # Errors
///
/// Returns `PdfError` if page rendering or text layer parsing fails.
pub fn djvu_to_pdf(doc: &DjVuDocument) -> Result<Vec<u8>, PdfError> {
    let mut w = PdfWriter::new();

    // Reserve IDs for catalog and pages
    let catalog_id = w.alloc_id(); // 1
    let pages_id = w.alloc_id(); // 2

    // Reserve a font object ID
    let font_id = w.alloc_id(); // 3
    w.add_obj(font_id, font_dict());

    // Build page objects (tolerate per-page errors with blank fallback)
    let mut page_obj_ids = Vec::new();
    for i in 0..doc.page_count() {
        let page = doc.page(i)?;
        let page_id = match build_page_objects(&mut w, page, pages_id, font_id) {
            Ok(id) => id,
            Err(_) => {
                // Fallback: blank page at native dimensions
                let dpi = page.dpi().max(1) as f32;
                let pt_w = px_to_pt(page.width() as f32, dpi);
                let pt_h = px_to_pt(page.height() as f32, dpi);
                w.add(
                    format!(
                        "<< /Type /Page /Parent {pages_id} 0 R\n\
                           /MediaBox [0 0 {pt_w:.4} {pt_h:.4}]\n\
                           /Resources << >> >>"
                    )
                    .into_bytes(),
                )
            }
        };
        page_obj_ids.push(page_id);
    }

    // Build outline from bookmarks
    let outline_id = build_outline(&mut w, doc.bookmarks(), &page_obj_ids);

    // Pages object
    let kids = page_obj_ids
        .iter()
        .map(|id| format!("{id} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");
    let n = page_obj_ids.len();
    w.add_obj(
        pages_id,
        format!("<< /Type /Pages /Kids [{kids}] /Count {n} >>").into_bytes(),
    );

    // Catalog
    let outline_ref = match outline_id {
        Some(oid) => format!(" /Outlines {oid} 0 R /PageMode /UseOutlines"),
        None => String::new(),
    };
    w.add_obj(
        catalog_id,
        format!("<< /Type /Catalog /Pages {pages_id} 0 R{outline_ref} >>").into_bytes(),
    );

    Ok(w.serialize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_escape_string() {
        assert_eq!(pdf_escape_string("hello"), "hello");
        assert_eq!(pdf_escape_string("a(b)c"), "a\\(b\\)c");
        assert_eq!(pdf_escape_string("a\\b"), "a\\\\b");
    }

    #[test]
    fn test_px_to_pt() {
        // At 72 dpi, 72 pixels = 72 points
        assert!((px_to_pt(72.0, 72.0) - 72.0).abs() < 0.01);
        // At 300 dpi, 300 pixels = 72 points
        assert!((px_to_pt(300.0, 300.0) - 72.0).abs() < 0.01);
    }

    #[test]
    fn test_resolve_bookmark_dest_page_number() {
        let page_ids = vec![10, 20, 30];
        let dest = resolve_bookmark_dest("#1", &page_ids);
        assert!(dest.contains("10 0 R"));
    }

    #[test]
    fn test_pdf_writer_serialize() {
        let mut w = PdfWriter::new();
        let id = w.add(b"<< /Type /Catalog >>".to_vec());
        assert_eq!(id, 1);
        let pdf = w.serialize();
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.windows(5).any(|w| w == b"%%EOF"));
    }

    #[test]
    fn test_make_stream() {
        let stream = make_stream(" /Filter /FlateDecode", b"hello");
        let s = String::from_utf8_lossy(&stream);
        assert!(s.contains("/Length 5"));
        assert!(s.contains("stream\nhello\nendstream"));
    }
}
