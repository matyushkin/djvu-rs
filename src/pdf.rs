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
    let mut body = format!("<< /Length {len}{dict_extra} >>\nstream\n").into_bytes();
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

/// Encode RGB bytes as JPEG and return the compressed bytes.
///
/// `quality` is in range 1–100. Values around 75–85 give excellent
/// perceptual quality for typical DjVu backgrounds at a fraction of the
/// FlateDecode+RGB size.
fn encode_rgb_to_jpeg(rgb: &[u8], width: u32, height: u32, quality: u8) -> Vec<u8> {
    use jpeg_encoder::{ColorType, Encoder};
    let mut out = Vec::new();
    let enc = Encoder::new(&mut out, quality);
    // Ignore encoding errors — fallback to empty, which will be caught at
    // the caller and downgraded to FlateDecode.
    let _ = enc.encode(rgb, width as u16, height as u16, ColorType::Rgb);
    out
}

/// Helper: make a DCTDecode (JPEG) stream object.
fn make_dct_stream(dict_extra: &str, jpeg_bytes: &[u8]) -> Vec<u8> {
    let extra = format!(" /Filter /DCTDecode{dict_extra}");
    make_stream(&extra, jpeg_bytes)
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

/// Compute render dimensions for a page given `output_dpi` option.
///
/// Returns `(render_w, render_h)` in pixels. When `output_dpi == 0` the native
/// page resolution is returned unchanged.
fn render_dims(native_w: u32, native_h: u32, native_dpi: f32, output_dpi: u32) -> (u32, u32) {
    if output_dpi == 0 || output_dpi as f32 >= native_dpi {
        return (native_w, native_h);
    }
    let scale = output_dpi as f32 / native_dpi;
    let rw = ((native_w as f32 * scale).round() as u32).max(1);
    let rh = ((native_h as f32 * scale).round() as u32).max(1);
    (rw, rh)
}

/// Pre-rendered page data — all expensive compute done, ready for sequential PDF emit.
///
/// # Memory note
///
/// `djvu_to_pdf_impl` collects `RenderedPage` for every page before emitting any PDF
/// objects (because `PdfWriter` is not `Send`). For large bilevel documents at native
/// DPI (e.g. 520 pages × ~1 MB deflated mask each) peak RAM can be significant.
/// A streaming/chunked approach is tracked in a separate issue.
struct RenderedPage {
    pt_w: f32,
    pt_h: f32,
    is_bilevel_only: bool,
    /// Fully encoded XObject body written as PDF resource `/Im0`.
    ///
    /// For bilevel-only pages this is the 1-bit JB2 mask; for mixed pages it is the
    /// RGB background image.
    img0_body: Option<Vec<u8>>,
    /// Fully encoded XObject body for the JB2 mask overlay (`/Mask0`).
    /// Only set for non-bilevel pages that have a Sjbz chunk.
    mask_obj_body: Option<Vec<u8>>,
    /// PDF content stream text operators (invisible text layer).
    text_ops: String,
    /// Pre-built annotation object bodies, one per hyperlink.
    link_annot_bodies: Vec<Vec<u8>>,
}

/// Render one page into a [`RenderedPage`].
///
/// This is the expensive step (pixel render, JPEG encode, JB2 decode, deflate)
/// and can safely run in parallel across pages.
fn render_page_data(page: &DjVuPage, opts: &PdfOptions) -> Result<RenderedPage, PdfError> {
    let pw = page.width() as u32;
    let ph = page.height() as u32;
    let dpi = page.dpi().max(1) as f32;
    let pt_w = px_to_pt(pw as f32, dpi);
    let pt_h = px_to_pt(ph as f32, dpi);

    let is_bilevel_only = page.find_chunk(b"Sjbz").is_some() && page.find_chunk(b"BG44").is_none();

    let (img0_body, mask_obj_body) = if is_bilevel_only {
        // Bilevel fast path: embed the 1-bit JB2 mask as the sole XObject.
        let mask = collect_mask_stream(page);
        (mask, None)
    } else {
        let (rw, rh) = render_dims(pw, ph, dpi, opts.output_dpi);
        let render_opts = RenderOptions {
            width: rw,
            height: rh,
            ..RenderOptions::default()
        };
        let pixmap = djvu_render::render_pixmap(page, &render_opts)?;
        let rgb = pixmap.to_rgb();

        let img_dict = format!(
            " /Type /XObject /Subtype /Image /Width {rw} /Height {rh}\
             /ColorSpace /DeviceRGB /BitsPerComponent 8"
        );
        let img_body = match opts.jpeg_quality {
            Some(quality) => {
                let jpeg = encode_rgb_to_jpeg(&rgb, rw, rh, quality);
                if jpeg.is_empty() {
                    make_deflate_stream(&img_dict, &rgb)
                } else {
                    make_dct_stream(&img_dict, &jpeg)
                }
            }
            None => make_deflate_stream(&img_dict, &rgb),
        };

        let mask = collect_mask_stream(page);
        (Some(img_body), mask)
    };

    let text_ops = build_text_content(page, dpi, pt_h);
    let link_annot_bodies = collect_link_annot_bodies(page, dpi, pt_h);

    Ok(RenderedPage {
        pt_w,
        pt_h,
        is_bilevel_only,
        img0_body,
        mask_obj_body,
        text_ops,
        link_annot_bodies,
    })
}

/// Decode and deflate the JB2 foreground mask into a PDF ImageMask XObject body.
fn collect_mask_stream(page: &DjVuPage) -> Option<Vec<u8>> {
    let sjbz = page.find_chunk(b"Sjbz")?;
    let dict = page
        .find_chunk(b"Djbz")
        .and_then(|djbz| crate::jb2::decode_dict(djbz, None).ok());
    let bitmap = crate::jb2::decode(sjbz, dict.as_ref()).ok()?;
    let bw = bitmap.width;
    let bh = bitmap.height;
    // Bitmap data is already packed 1-bit MSB-first, which is what PDF expects
    // for an ImageMask with /Decode [1 0] (1=black=marked).
    let dict_extra = format!(
        " /Type /XObject /Subtype /Image /Width {bw} /Height {bh}\
         /ImageMask true /BitsPerComponent 1 /Decode [1 0]"
    );
    Some(make_deflate_stream(&dict_extra, &bitmap.data))
}

/// Build pre-serialized annotation bodies for all hyperlinks on a page.
fn collect_link_annot_bodies(page: &DjVuPage, dpi: f32, pt_h: f32) -> Vec<Vec<u8>> {
    let hyperlinks = match page.hyperlinks() {
        Ok(links) => links,
        Err(_) => return Vec::new(),
    };
    hyperlinks
        .iter()
        .filter_map(|link| {
            let rect = shape_to_pdf_rect(&link.shape, dpi, pt_h)?;
            let url_escaped = pdf_escape_string(&link.url);
            Some(
                format!(
                    "<< /Type /Annot /Subtype /Link\n\
                       /Rect [{:.4} {:.4} {:.4} {:.4}]\n\
                       /Border [0 0 0]\n\
                       /A << /S /URI /URI ({url_escaped}) >> >>",
                    rect.0, rect.1, rect.2, rect.3
                )
                .into_bytes(),
            )
        })
        .collect()
}

/// Emit a pre-rendered page into `PdfWriter` (sequential). Returns the page object ID.
fn emit_page_objects(
    w: &mut PdfWriter,
    data: RenderedPage,
    pages_id: usize,
    font_id: usize,
) -> usize {
    let pt_w = data.pt_w;
    let pt_h = data.pt_h;

    let img_id = data.img0_body.map(|body| w.add(body));
    let mask_img_id = data.mask_obj_body.map(|body| w.add(body));

    let mut content = String::new();

    if data.is_bilevel_only {
        // img0 may still be None if JB2 decode failed at render time — render gracefully.
        if img_id.is_some() {
            content.push_str("1 1 1 rg\n");
            content.push_str(&format!("q {pt_w:.4} 0 0 {pt_h:.4} 0 0 cm /Im0 Do Q\n"));
        }
    } else {
        if img_id.is_some() {
            content.push_str(&format!("q {pt_w:.4} 0 0 {pt_h:.4} 0 0 cm /Im0 Do Q\n"));
        }
        if mask_img_id.is_some() {
            content.push_str(&format!(
                "q 0 0 0 rg {pt_w:.4} 0 0 {pt_h:.4} 0 0 cm /Mask0 Do Q\n"
            ));
        }
    }

    if !data.text_ops.is_empty() {
        content.push_str(&data.text_ops);
    }

    let content_body = make_deflate_stream("", content.as_bytes());
    let content_id = w.add(content_body);

    let mut resources = String::from("/XObject <<");
    if let Some(id) = img_id {
        resources.push_str(&format!(" /Im0 {id} 0 R"));
    }
    if let Some(mid) = mask_img_id {
        resources.push_str(&format!(" /Mask0 {mid} 0 R"));
    }
    resources.push_str(" >>");
    if !data.text_ops.is_empty() {
        resources.push_str(&format!(" /Font << /F1 {font_id} 0 R >>"));
    }

    let annot_ids: Vec<usize> = data
        .link_annot_bodies
        .into_iter()
        .map(|body| w.add(body))
        .collect();
    let mut annots_str = String::new();
    if !annot_ids.is_empty() {
        annots_str.push_str(" /Annots [");
        for aid in &annot_ids {
            annots_str.push_str(&format!(" {aid} 0 R"));
        }
        annots_str.push_str(" ]");
    }

    w.add(
        format!(
            "<< /Type /Page /Parent {pages_id} 0 R\n\
               /MediaBox [0 0 {pt_w:.4} {pt_h:.4}]\n\
               /Contents {content_id} 0 R\n\
               /Resources << {resources} >>{annots_str} >>"
        )
        .into_bytes(),
    )
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
            // Sum per-character advance widths using Helvetica metrics.
            let natural_width: f32 = zone
                .text
                .chars()
                .map(|c| helvetica_advance(c) * font_size)
                .sum::<f32>()
                .max(0.01);
            let h_scale = if natural_width > 0.01 {
                (w / natural_width) * 100.0
            } else {
                100.0
            };

            ops.push_str(&format!(
                "{font_size:.2} 0 0 {font_size:.2} {x:.4} {y:.4} Tm\n"
            ));
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

/// Return the normalized advance width (fraction of em) for `c` in Helvetica.
///
/// Uses standard Helvetica metrics for ASCII, and Unicode-block heuristics
/// for non-ASCII ranges.  CJK, full-width, and Hangul characters are
/// treated as full-width (1.0).  Everything else falls back to 0.556 (the
/// Helvetica average for Latin lowercase).
fn helvetica_advance(c: char) -> f32 {
    let cp = c as u32;
    match c {
        // ASCII control / non-printing — zero width
        '\x00'..='\x1f' | '\x7f' => 0.0,
        // Space
        ' ' => 0.278,
        // Digits
        '0'..='9' => 0.556,
        // Common punctuation
        ',' | '.' | ':' | ';' | '!' | '?' => 0.278,
        '\'' | '"' => 0.222,
        '(' | ')' | '[' | ']' | '{' | '}' => 0.333,
        '-' | '\u{2013}' | '\u{2014}' => 0.333,
        // Uppercase ASCII — broad average for Helvetica
        'A'..='Z' => 0.667,
        // Lowercase ASCII
        'a'..='z' => 0.556,
        _ => {
            // CJK Unified Ideographs and common CJK blocks → full-width
            if matches!(cp,
                0x1100..=0x11FF  // Hangul Jamo
                | 0x2E80..=0x2EFF  // CJK Radicals Supplement
                | 0x2F00..=0x2FDF  // Kangxi Radicals
                | 0x3000..=0x303F  // CJK Symbols and Punctuation
                | 0x3040..=0x309F  // Hiragana
                | 0x30A0..=0x30FF  // Katakana
                | 0x3100..=0x312F  // Bopomofo
                | 0x3130..=0x318F  // Hangul Compatibility Jamo
                | 0x3190..=0x31FF  // various CJK
                | 0x3200..=0x32FF  // Enclosed CJK
                | 0x3300..=0x33FF  // CJK Compatibility
                | 0x3400..=0x4DBF  // CJK Extension A
                | 0x4E00..=0x9FFF  // CJK Unified Ideographs
                | 0xA000..=0xA48F  // Yi Syllables
                | 0xA490..=0xA4CF  // Yi Radicals
                | 0xAC00..=0xD7AF  // Hangul Syllables
                | 0xF900..=0xFAFF  // CJK Compatibility Ideographs
                | 0xFE10..=0xFE1F  // Vertical Forms
                | 0xFE30..=0xFE4F  // CJK Compatibility Forms
                | 0xFF00..=0xFFEF  // Halfwidth and Fullwidth Forms
                | 0x1B000..=0x1B0FF // Kana Supplement
                | 0x20000..=0x2A6DF // CJK Extension B
                | 0x2A700..=0x2CEAF // CJK Extensions C/D/E
                | 0x2CEB0..=0x2EBEF // CJK Extension F
                | 0x30000..=0x3134F // CJK Extension G
            ) {
                1.0
            } else {
                // Latin Extended, Cyrillic, Greek, Arabic, Hebrew, etc.
                0.556
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
        format!("<< /Type /Outlines /First {first} 0 R /Last {last} 0 R /Count {count} >>")
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
        if let Some(page_str) = stripped.strip_prefix("page")
            && let Ok(page_num) = page_str.trim_start_matches('_').parse::<usize>()
        {
            let idx = page_num.saturating_sub(1);
            if let Some(&pid) = page_ids.get(idx) {
                return format!(" /Dest [{pid} 0 R /Fit]");
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
/// Options for DjVu → PDF conversion.
///
/// Use `PdfOptions::default()` for sensible defaults:
/// - 150 DPI output (screen-quality, ~16× fewer pixels than native 600 DPI)
/// - DCTDecode (JPEG quality 80) for color backgrounds
/// - 1-bit FlateDecode for bilevel masks
/// - Bilevel-only pages skip RGB render entirely (direct 1-bit embed)
#[derive(Debug, Clone)]
pub struct PdfOptions {
    /// JPEG quality for background image encoding (1–100).
    ///
    /// Higher values produce better quality at larger file sizes.
    /// Set to `None` to use lossless FlateDecode (PNG-like, larger output).
    pub jpeg_quality: Option<u8>,

    /// Output resolution in DPI.
    ///
    /// Controls the pixel dimensions of embedded images. Lower values produce
    /// smaller files and faster exports; higher values preserve more detail.
    ///
    /// - `150` — screen quality (default); ~16× fewer pixels than native 600 DPI
    /// - `300` — print quality
    /// - `0` — use native page DPI (maximum quality, slowest)
    pub output_dpi: u32,
}

impl Default for PdfOptions {
    fn default() -> Self {
        PdfOptions {
            jpeg_quality: Some(80),
            output_dpi: 150,
        }
    }
}

impl PdfOptions {
    /// High-quality archival preset: native DPI, JPEG quality 90.
    pub fn archival() -> Self {
        PdfOptions {
            jpeg_quality: Some(90),
            output_dpi: 0,
        }
    }
}

/// Convert a DjVu document to PDF bytes using custom options.
///
/// See [`PdfOptions`] for available settings.
pub fn djvu_to_pdf_with_options(
    doc: &DjVuDocument,
    opts: &PdfOptions,
) -> Result<Vec<u8>, PdfError> {
    djvu_to_pdf_impl(doc, opts)
}

/// This produces a PDF 1.4 file with:
/// - Rasterized page images (IW44 background + JB2 mask composite)
/// - Invisible text layer for search and selection
/// - Bookmarks (PDF outline) from NAVM
/// - Hyperlink annotations from ANTz
///
/// Background images are encoded as DCTDecode (JPEG at quality 80) by default,
/// producing significantly smaller files than the legacy FlateDecode path.
/// Use [`djvu_to_pdf_with_options`] with `jpeg_quality: None` for lossless output.
///
/// # Errors
///
/// Returns `PdfError` if page rendering or text layer parsing fails.
pub fn djvu_to_pdf(doc: &DjVuDocument) -> Result<Vec<u8>, PdfError> {
    djvu_to_pdf_impl(doc, &PdfOptions::default())
}

fn djvu_to_pdf_impl(doc: &DjVuDocument, opts: &PdfOptions) -> Result<Vec<u8>, PdfError> {
    let mut w = PdfWriter::new();

    // Reserve IDs for catalog and pages
    let catalog_id = w.alloc_id(); // 1
    let pages_id = w.alloc_id(); // 2

    // Reserve a font object ID
    let font_id = w.alloc_id(); // 3
    w.add_obj(font_id, font_dict());

    let page_count = doc.page_count();

    // Render all pages (expensive: pixel render, encode, deflate).
    // With the `parallel` feature, pages are rendered concurrently via rayon.
    #[cfg(feature = "parallel")]
    let rendered_pages: Vec<Option<RenderedPage>> = {
        use rayon::prelude::*;
        (0..page_count)
            .into_par_iter()
            .map(|i| {
                doc.page(i)
                    .ok()
                    .and_then(|p| render_page_data(p, opts).ok())
            })
            .collect()
    };

    #[cfg(not(feature = "parallel"))]
    let rendered_pages: Vec<Option<RenderedPage>> = (0..page_count)
        .map(|i| {
            doc.page(i)
                .ok()
                .and_then(|p| render_page_data(p, opts).ok())
        })
        .collect();

    // Emit page objects sequentially (PdfWriter is not Send).
    let mut page_obj_ids = Vec::with_capacity(page_count);
    for (i, rendered) in rendered_pages.into_iter().enumerate() {
        let page_id = match rendered {
            Some(data) => emit_page_objects(&mut w, data, pages_id, font_id),
            None => {
                // Fallback: blank page at native dimensions
                let page = doc.page(i)?;
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

    #[test]
    fn test_deflate_roundtrip() {
        let data = b"hello world, this is a test of deflate compression";
        let compressed = deflate(data);
        // Compressed data should be non-empty
        assert!(!compressed.is_empty());
        // Decompress and verify
        let decompressed = miniz_oxide::inflate::decompress_to_vec_zlib(&compressed).unwrap();
        assert_eq!(&decompressed, data);
    }

    #[test]
    fn test_make_deflate_stream() {
        let body = make_deflate_stream(" /Type /XObject", b"test data");
        let s = String::from_utf8_lossy(&body);
        assert!(s.contains("/Filter /FlateDecode"));
        assert!(s.contains("/Type /XObject"));
        assert!(s.contains("stream\n"));
        assert!(s.contains("\nendstream"));
    }

    #[test]
    fn test_font_dict() {
        let d = font_dict();
        let s = String::from_utf8_lossy(&d);
        assert!(s.contains("/Type /Font"));
        assert!(s.contains("/BaseFont /Helvetica"));
    }

    #[test]
    fn test_pdf_writer_alloc_ids() {
        let mut w = PdfWriter::new();
        let id1 = w.alloc_id();
        let id2 = w.alloc_id();
        let id3 = w.alloc_id();
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[test]
    fn test_pdf_writer_multiple_objects() {
        let mut w = PdfWriter::new();
        w.add(b"<< /Type /Catalog >>".to_vec());
        w.add(b"<< /Type /Pages >>".to_vec());
        let pdf = w.serialize();
        let s = String::from_utf8_lossy(&pdf);
        assert!(s.contains("1 0 obj"));
        assert!(s.contains("2 0 obj"));
        assert!(s.contains("/Size 3")); // 0, 1, 2
    }

    #[test]
    fn test_resolve_bookmark_dest_page_prefix() {
        let page_ids = vec![10, 20, 30];
        let dest = resolve_bookmark_dest("#page2", &page_ids);
        assert!(dest.contains("20 0 R"));
        assert!(dest.contains("/Fit"));
    }

    #[test]
    fn test_resolve_bookmark_dest_page_underscore() {
        let page_ids = vec![10, 20, 30];
        let dest = resolve_bookmark_dest("#page_3", &page_ids);
        assert!(dest.contains("30 0 R"));
    }

    #[test]
    fn test_resolve_bookmark_dest_out_of_range() {
        let page_ids = vec![10];
        let dest = resolve_bookmark_dest("#page99", &page_ids);
        // Should fall through to bare number parse or be empty
        assert!(!dest.contains("10 0 R"));
    }

    #[test]
    fn test_resolve_bookmark_dest_external_url() {
        let page_ids = vec![10];
        let dest = resolve_bookmark_dest("http://example.com", &page_ids);
        assert!(dest.contains("/S /URI"));
        assert!(dest.contains("http://example.com"));
    }

    #[test]
    fn test_resolve_bookmark_dest_empty_url() {
        let page_ids = vec![10];
        let dest = resolve_bookmark_dest("", &page_ids);
        assert!(dest.is_empty());
    }

    #[test]
    fn test_pdf_escape_special_chars() {
        assert_eq!(pdf_escape_string("a(b)c\\d"), "a\\(b\\)c\\\\d");
    }

    #[test]
    fn test_pdf_escape_non_ascii() {
        // Non-ASCII chars should be replaced with ?
        let result = pdf_escape_string("caf\u{00e9}");
        assert_eq!(result, "caf?");
    }

    #[test]
    fn test_shape_to_pdf_rect_rect() {
        use crate::annotation;
        let shape = annotation::Shape::Rect(annotation::Rect {
            x: 0,
            y: 0,
            width: 300,
            height: 300,
        });
        let rect = shape_to_pdf_rect(&shape, 300.0, 72.0).unwrap();
        assert!((rect.0 - 0.0).abs() < 0.01); // x1
        assert!((rect.2 - 72.0).abs() < 0.01); // x2 = 300 * 72/300
    }

    #[test]
    fn test_shape_to_pdf_rect_poly() {
        use crate::annotation;
        let shape = annotation::Shape::Poly(vec![(0, 0), (300, 0), (300, 300), (0, 300)]);
        let rect = shape_to_pdf_rect(&shape, 300.0, 72.0).unwrap();
        assert!((rect.0 - 0.0).abs() < 0.01);
        assert!((rect.2 - 72.0).abs() < 0.01);
    }

    #[test]
    fn test_shape_to_pdf_rect_empty_poly() {
        use crate::annotation;
        let shape = annotation::Shape::Poly(vec![]);
        assert!(shape_to_pdf_rect(&shape, 300.0, 72.0).is_none());
    }

    #[test]
    fn test_shape_to_pdf_rect_line() {
        use crate::annotation;
        let shape = annotation::Shape::Line(0, 0, 150, 150);
        let rect = shape_to_pdf_rect(&shape, 150.0, 72.0).unwrap();
        assert!((rect.0 - 0.0).abs() < 0.01);
        assert!((rect.2 - 72.0).abs() < 0.01);
    }

    #[test]
    fn test_count_outline_items_empty() {
        let bookmarks: Vec<crate::djvu_document::DjVuBookmark> = vec![];
        assert_eq!(count_outline_items(&bookmarks), 0);
    }

    #[test]
    fn test_count_outline_items_nested() {
        use crate::djvu_document::DjVuBookmark;
        let bookmarks = vec![DjVuBookmark {
            title: "Chapter 1".into(),
            url: "#1".into(),
            children: vec![
                DjVuBookmark {
                    title: "Section 1.1".into(),
                    url: "#2".into(),
                    children: vec![],
                },
                DjVuBookmark {
                    title: "Section 1.2".into(),
                    url: "#3".into(),
                    children: vec![],
                },
            ],
        }];
        assert_eq!(count_outline_items(&bookmarks), 3);
    }

    // ── DCTDecode / PdfOptions tests ──────────────────────────────────────────

    fn assets_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/djvujs/library/assets")
    }

    fn load_doc(name: &str) -> crate::djvu_document::DjVuDocument {
        let data =
            std::fs::read(assets_path().join(name)).unwrap_or_else(|_| panic!("{name} must exist"));
        crate::djvu_document::DjVuDocument::parse(&data)
            .unwrap_or_else(|e| panic!("parse failed: {e}"))
    }

    /// `PdfOptions::default()` uses jpeg_quality = Some(80).
    #[test]
    fn pdf_options_default_is_jpeg80() {
        let opts = PdfOptions::default();
        assert_eq!(opts.jpeg_quality, Some(80));
    }

    /// JPEG encoding roundtrip: `encode_rgb_to_jpeg` returns a non-empty JPEG.
    #[test]
    fn encode_rgb_to_jpeg_returns_jpeg() {
        // 4×4 solid red image
        let rgb = [255u8, 0, 0].repeat(16); // 16 pixels * 3 channels
        let jpeg = encode_rgb_to_jpeg(&rgb, 4, 4, 80);
        assert!(!jpeg.is_empty(), "JPEG output must not be empty");
        // JPEG starts with FF D8
        assert_eq!(jpeg[0], 0xFF);
        assert_eq!(jpeg[1], 0xD8);
    }

    /// `make_dct_stream` embeds /Filter /DCTDecode in the PDF stream dict.
    #[test]
    fn make_dct_stream_has_dctdecode_filter() {
        let fake_jpeg = b"\xFF\xD8\xFF\xD9"; // minimal JPEG markers
        let stream = make_dct_stream(" /Type /XObject", fake_jpeg);
        let s = String::from_utf8_lossy(&stream);
        assert!(
            s.contains("/Filter /DCTDecode"),
            "must contain DCTDecode filter"
        );
        assert!(s.contains("/Type /XObject"));
    }

    /// DCT PDF is smaller than deflate PDF for the same page.
    #[test]
    fn dct_pdf_is_smaller_than_deflate_pdf() {
        let doc = load_doc("chicken.djvu");
        let dct_pdf = djvu_to_pdf_with_options(
            &doc,
            &PdfOptions {
                jpeg_quality: Some(75),
                output_dpi: 150,
            },
        )
        .expect("DCT conversion must succeed");
        let flat_pdf = djvu_to_pdf_with_options(
            &doc,
            &PdfOptions {
                jpeg_quality: None,
                output_dpi: 150,
            },
        )
        .expect("FlateDecode conversion must succeed");
        assert!(
            dct_pdf.len() < flat_pdf.len(),
            "DCT PDF ({} bytes) must be smaller than FlateDecode PDF ({} bytes)",
            dct_pdf.len(),
            flat_pdf.len()
        );
    }

    /// Output PDF contains /DCTDecode when jpeg_quality is set.
    #[test]
    fn pdf_with_dct_contains_dctdecode_marker() {
        let doc = load_doc("chicken.djvu");
        let pdf = djvu_to_pdf_with_options(
            &doc,
            &PdfOptions {
                jpeg_quality: Some(80),
                output_dpi: 150,
            },
        )
        .unwrap();
        let has_dct = pdf.windows(9).any(|w| w == b"DCTDecode");
        assert!(has_dct, "PDF must contain DCTDecode");
    }

    /// Output PDF does NOT contain /DCTDecode when jpeg_quality is None.
    #[test]
    fn pdf_without_dct_has_no_dctdecode() {
        let doc = load_doc("chicken.djvu");
        let pdf = djvu_to_pdf_with_options(
            &doc,
            &PdfOptions {
                jpeg_quality: None,
                output_dpi: 150,
            },
        )
        .unwrap();
        let has_dct = pdf.windows(9).any(|w| w == b"DCTDecode");
        assert!(!has_dct, "FlateDecode PDF must not contain DCTDecode");
    }

    /// `djvu_to_pdf` (default, DCT at 80) is smaller than FlateDecode.
    #[test]
    fn default_djvu_to_pdf_is_dct() {
        let doc = load_doc("chicken.djvu");
        let default_pdf = djvu_to_pdf(&doc).unwrap();
        let flat_pdf = djvu_to_pdf_with_options(
            &doc,
            &PdfOptions {
                jpeg_quality: None,
                output_dpi: 150,
            },
        )
        .unwrap();
        assert!(
            default_pdf.len() < flat_pdf.len(),
            "default PDF must use DCT and be smaller than FlateDecode"
        );
    }
}
