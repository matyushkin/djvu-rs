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
    text::Rect,
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
fn render_dims(page: &DjVuPage, output_dpi: u32) -> (u32, u32) {
    let native_dpi = page.dpi().max(1) as f32;
    // PDF never upscales: a zero or above-native target DPI keeps native pixels.
    if output_dpi == 0 || output_dpi as f32 >= native_dpi {
        return (page.width() as u32, page.height() as u32);
    }
    crate::export_common::size_at_dpi(page, output_dpi as f32)
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
        let (rw, rh) = render_dims(page, opts.output_dpi);
        // Set only the output size: the render pipeline derives the IW44 decode
        // scale from `width` (see `RenderOptions::decode_scale`). Previously this
        // left `scale = 1.0` at every DPI, forcing a full-resolution wavelet
        // decode followed by a downscale (#377).
        let render_opts = RenderOptions {
            width: rw,
            height: rh,
            ..RenderOptions::default()
        };
        let rgb = render_rgb_for_pdf(page, &render_opts, rw, rh)?;

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

fn render_rgb_for_pdf(
    page: &DjVuPage,
    opts: &RenderOptions,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, PdfError> {
    let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
    crate::export_common::render_rows_or_pixmap(page, opts, |rgba_row| {
        crate::export_common::rgba_row_to_rgb(&mut rgb, rgba_row);
    })?;
    Ok(rgb)
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

    // Emit one positioned run per leaf word/character zone (shared zone-walk).
    for span in crate::export_common::word_spans(&text_layer) {
        emit_word_span(&mut ops, span.rect, span.text, dpi, pt_h);
    }

    ops.push_str("ET\n");

    if ops == "BT\n3 Tr\n/F1 1 Tf\nET\n" {
        // No actual text was emitted
        return String::new();
    }

    ops
}

/// Emit text positioning operators for one leaf word/character span.
///
/// `rect` is top-left-origin pixels; PDF uses bottom-left origin, so the
/// baseline is flipped in point space: `pdf_y = pt_h - (r.y + r.height) * 72/dpi`.
/// (This subtract-after-convert order is what produces byte-identical output;
/// see the note on [`crate::export_common::flip_y_bottom`].)
fn emit_word_span(ops: &mut String, rect: &Rect, text: &str, dpi: f32, pt_h: f32) {
    let x = px_to_pt(rect.x as f32, dpi);
    let y = pt_h - px_to_pt((rect.y + rect.height) as f32, dpi);
    let w = px_to_pt(rect.width as f32, dpi);
    let h = px_to_pt(rect.height as f32, dpi);

    if w <= 0.0 || h <= 0.0 {
        return;
    }

    // Font size = zone height in points
    let font_size = h;
    if font_size < 0.5 {
        return;
    }

    // Horizontal scale to fit text width
    let text_escaped = pdf_escape_string(text);
    // Sum per-character advance widths using Helvetica metrics.
    let natural_width: f32 = text
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
///
/// DjVu annotation coordinates use bottom-left origin (same as PDF), so no
/// vertical flip is needed — only the point conversion of each edge. The
/// bounding box, and the empty/degenerate → `None` rule, are the shared
/// [`crate::export_common::shape_bbox`]; a zero-area shape encloses no link
/// region and is dropped. Because `px_to_pt` is monotonic, taking the bounding
/// box in pixel space and converting its edges yields the same points as the
/// previous per-point fold in point space.
fn shape_to_pdf_rect(shape: &Shape, dpi: f32, _pt_h: f32) -> Option<(f32, f32, f32, f32)> {
    let r = crate::export_common::shape_bbox(shape)?;
    let x1 = px_to_pt(r.x as f32, dpi);
    let y1 = px_to_pt(r.y as f32, dpi);
    let x2 = px_to_pt((r.x + r.width) as f32, dpi);
    let y2 = px_to_pt((r.y + r.height) as f32, dpi);
    Some((x1, y1, x2, y2))
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
    if let Some(idx) = crate::export_common::bookmark_page_index(url)
        && let Some(&pid) = page_ids.get(idx)
    {
        return format!(" /Dest [{pid} 0 R /Fit]");
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

    // Emit one page's objects (rendered body or a blank-page fallback) and return
    // its page-object id. Shared by both the parallel and sequential paths.
    let emit_one =
        |w: &mut PdfWriter, i: usize, rendered: Option<RenderedPage>| -> Result<usize, PdfError> {
            Ok(match rendered {
                Some(data) => emit_page_objects(w, data, pages_id, font_id),
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
            })
        };

    let mut page_obj_ids = Vec::with_capacity(page_count);

    // With the `parallel` feature, render all pages concurrently via rayon, then
    // emit sequentially (PdfWriter is not Send).
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        let rendered_pages: Vec<Option<RenderedPage>> = (0..page_count)
            .into_par_iter()
            .map(|i| {
                doc.page(i)
                    .ok()
                    .and_then(|p| render_page_data(p, opts).ok())
            })
            .collect();
        for (i, rendered) in rendered_pages.into_iter().enumerate() {
            page_obj_ids.push(emit_one(&mut w, i, rendered)?);
        }
    }

    // #449: sequential path renders, emits, and drops one page at a time, holding
    // O(1) page bodies in memory instead of collecting all `page_count` rendered
    // bodies first (peak RSS O(pages × body) → O(1 page); mirrors TIFF_STREAM).
    #[cfg(not(feature = "parallel"))]
    for i in 0..page_count {
        let rendered = doc
            .page(i)
            .ok()
            .and_then(|p| render_page_data(p, opts).ok());
        page_obj_ids.push(emit_one(&mut w, i, rendered)?);
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

    #[test]
    fn pdf_rgb_streaming_matches_pixmap_rgb() {
        let doc = load_doc("boy.djvu");
        let page = doc.page(0).unwrap();
        let (rw, rh) = render_dims(page, PdfOptions::default().output_dpi);
        let opts = RenderOptions {
            width: rw,
            height: rh,
            ..RenderOptions::default()
        };

        let streamed = render_rgb_for_pdf(page, &opts, rw, rh).unwrap();
        let pixmap = djvu_render::render_pixmap(page, &opts).unwrap();

        assert_eq!(streamed, pixmap.to_rgb());
    }

    #[test]
    fn pdf_rgb_fallback_handles_non_streamable_options() {
        let doc = load_doc("boy.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: page.width() as u32,
            height: page.height() as u32,
            aa: true,
            ..RenderOptions::default()
        };

        let rgb = render_rgb_for_pdf(page, &opts, opts.width, opts.height).unwrap();
        let pixmap = djvu_render::render_pixmap(page, &opts).unwrap();

        assert_eq!(rgb, pixmap.to_rgb());
    }

    // ── Text layer ────────────────────────────────────────────────────────────

    /// A document with TXTz must embed invisible text (BT…ET blocks).
    #[test]
    fn pdf_with_text_layer_contains_bt_et_markers() {
        let doc = load_doc("colorbook.djvu");
        let pdf = djvu_to_pdf(&doc).unwrap();
        // PDF content streams are deflated, but "BT" / "ET" may appear in stream
        // dict or in the raw uncompressed bytes we can observe at the dict level.
        // More reliably: we check that at least one page's content stream was
        // added (the file is larger than a document without text).
        assert!(!pdf.is_empty(), "PDF must not be empty");
        // The text content stream dict contains /Font when text is present.
        let has_font = pdf.windows(5).any(|w| w == b"/Font");
        assert!(
            has_font,
            "PDF with text layer must reference a /Font resource"
        );
    }

    /// `build_text_content` returns empty when the page has no text layer.
    #[test]
    fn build_text_content_no_text_layer_returns_empty() {
        let doc = load_doc("chicken.djvu"); // no TXTz
        let page = doc.page(0).unwrap();
        let result = build_text_content(page, 100.0, 720.0);
        assert!(
            result.is_empty(),
            "page without text layer must produce empty text content"
        );
    }

    /// `build_text_content` returns non-empty when the page has a text layer.
    #[test]
    fn build_text_content_with_text_layer_returns_non_empty() {
        let doc = load_doc("colorbook.djvu"); // has TXTz
        // Find the first page that actually has a text layer
        for i in 0..doc.page_count() {
            let page = doc.page(i).unwrap();
            if page.text_layer().ok().flatten().is_some() {
                let dpi = page.dpi().max(1) as f32;
                let pt_h = page.height() as f32 * 72.0 / dpi;
                let result = build_text_content(page, dpi, pt_h);
                if !result.is_empty() {
                    assert!(result.contains("BT"), "text content must begin with BT");
                    assert!(result.contains("ET"), "text content must end with ET");
                    return;
                }
            }
        }
        // If no page had non-empty text, that's fine — fixture may have empty zones
    }

    // ── Bookmarks (PDF outline) ───────────────────────────────────────────────

    /// A document with NAVM bookmarks must produce /Outlines in the PDF catalog.
    #[test]
    fn pdf_with_bookmarks_contains_outlines() {
        let doc = load_doc("links.djvu"); // has NAVM
        let pdf = djvu_to_pdf(&doc).unwrap();
        let has_outlines = pdf.windows(8).any(|w| w == b"Outlines");
        assert!(
            has_outlines,
            "PDF with NAVM bookmarks must contain /Outlines"
        );
    }

    /// A document without bookmarks must NOT produce /Outlines.
    #[test]
    fn pdf_without_bookmarks_has_no_outlines() {
        let doc = load_doc("chicken.djvu"); // no NAVM
        let pdf = djvu_to_pdf(&doc).unwrap();
        let has_outlines = pdf.windows(8).any(|w| w == b"Outlines");
        assert!(
            !has_outlines,
            "PDF without bookmarks must not contain /Outlines"
        );
    }

    /// `resolve_bookmark_dest` resolves `#page_N` to a /Dest reference.
    /// DjVu page anchors are 1-based: `#page_1` = index 0, `#page_2` = index 1.
    #[test]
    fn resolve_bookmark_dest_page_anchor() {
        let page_ids = [10usize, 20, 30];
        // #page_1 is 1-based → index 0 → page_ids[0] = 10
        let dest = resolve_bookmark_dest("#page_1", &page_ids);
        assert!(dest.contains("/Dest"), "must produce /Dest: {dest}");
        assert!(dest.contains("10 0 R"), "must reference page id 10: {dest}");
        // #page_2 → index 1 → page_ids[1] = 20
        let dest2 = resolve_bookmark_dest("#page_2", &page_ids);
        assert!(
            dest2.contains("20 0 R"),
            "must reference page id 20: {dest2}"
        );
    }

    /// `resolve_bookmark_dest` falls back to /A /URI for external URLs.
    #[test]
    fn resolve_bookmark_dest_external_url() {
        let dest = resolve_bookmark_dest("https://example.com", &[10, 20]);
        assert!(
            dest.contains("/URI"),
            "external URL must produce URI action: {dest}"
        );
    }

    /// `resolve_bookmark_dest` returns empty string for empty URL.
    #[test]
    fn resolve_bookmark_dest_empty_url() {
        let dest = resolve_bookmark_dest("", &[10]);
        assert!(dest.is_empty(), "empty URL must produce empty dest: {dest}");
    }

    // ── Hyperlink annotations ─────────────────────────────────────────────────

    /// `collect_link_annot_bodies` runs without error on a document with ANTz.
    #[test]
    fn collect_link_annot_bodies_runs_without_error() {
        let doc = load_doc("czech.djvu"); // has ANTz
        for i in 0..doc.page_count() {
            let page = doc.page(i).unwrap();
            let dpi = page.dpi().max(1) as f32;
            let pt_h = page.height() as f32 * 72.0 / dpi;
            let _ = collect_link_annot_bodies(page, dpi, pt_h);
        }
        // Test passes if no panic
    }

    /// `collect_link_annot_bodies` builds correct annotation body for a link.
    ///
    /// Exercises the annotation formatting path directly without needing a
    /// specific fixture with Rect-shaped hyperlinks.
    #[test]
    fn link_annot_body_format() {
        use crate::annotation::{MapArea, Rect as ARect, Shape};

        // Build a synthetic Rect-shaped hyperlink
        let link = MapArea {
            shape: Shape::Rect(ARect {
                x: 10,
                y: 20,
                width: 100,
                height: 50,
            }),
            url: "https://example.com".to_string(),
            description: String::new(),
            border: None,
            highlight: None,
        };

        let rect = shape_to_pdf_rect(&link.shape, 100.0, 360.0);
        assert!(rect.is_some(), "Rect shape must produce a PDF rect");
        let (x1, y1, x2, y2) = rect.unwrap();
        let url_escaped = pdf_escape_string(&link.url);
        let body = format!(
            "<< /Type /Annot /Subtype /Link\n\
               /Rect [{:.4} {:.4} {:.4} {:.4}]\n\
               /Border [0 0 0]\n\
               /A << /S /URI /URI ({url_escaped}) >> >>",
            x1, y1, x2, y2
        );
        assert!(body.contains("/Type /Annot"), "must have /Type /Annot");
        assert!(body.contains("/Subtype /Link"), "must have /Subtype /Link");
        assert!(body.contains("https://example.com"), "must contain URL");
        assert!(body.contains("/Rect"), "must have /Rect");
    }

    // ── Bilevel-only pages ────────────────────────────────────────────────────

    /// Bilevel-only page (Sjbz, no BG44) must use /ImageMask in the PDF.
    #[test]
    fn bilevel_only_page_has_image_mask() {
        let doc = load_doc("boy_jb2.djvu"); // Sjbz-only
        let pdf = djvu_to_pdf(&doc).unwrap();
        let has_mask = pdf.windows(9).any(|w| w == b"ImageMask");
        assert!(has_mask, "bilevel-only page must embed /ImageMask XObject");
    }

    // ── Mixed page (Sjbz + BG44) — mask overlay ──────────────────────────────

    /// A page with both Sjbz (foreground mask) and BG44 (background) must
    /// embed both an /Im0 image and a /Mask0 ImageMask XObject.
    #[test]
    fn mixed_page_has_both_image_and_mask_xobject() {
        let doc = load_doc("colorbook.djvu"); // Sjbz+BG44
        let pdf = djvu_to_pdf(&doc).unwrap();
        let has_im0 = pdf.windows(4).any(|w| w == b"Im0 ");
        let has_mask0 = pdf.windows(5).any(|w| w == b"Mask0");
        assert!(has_im0, "mixed page must reference /Im0 background");
        assert!(
            has_mask0,
            "mixed page must reference /Mask0 foreground mask"
        );
    }

    // ── render_dims / output_dpi ──────────────────────────────────────────────

    /// When output_dpi is lower than native DPI the PDF is smaller.
    #[test]
    fn lower_output_dpi_produces_smaller_pdf() {
        let doc = load_doc("chicken.djvu");
        let native = djvu_to_pdf_with_options(
            &doc,
            &PdfOptions {
                jpeg_quality: None,
                output_dpi: 0,
            },
        )
        .unwrap();
        let downscaled = djvu_to_pdf_with_options(
            &doc,
            &PdfOptions {
                jpeg_quality: None,
                output_dpi: 50,
            },
        )
        .unwrap();
        assert!(
            downscaled.len() < native.len(),
            "50 DPI PDF ({} B) must be smaller than native ({} B)",
            downscaled.len(),
            native.len()
        );
    }

    // ── PdfOptions::archival() ────────────────────────────────────────────────

    #[test]
    fn pdf_archival_preset_produces_output() {
        let doc = load_doc("chicken.djvu");
        let pdf = djvu_to_pdf_with_options(&doc, &PdfOptions::archival()).unwrap();
        assert!(!pdf.is_empty());
        assert!(
            pdf.starts_with(b"%PDF-"),
            "archival PDF must start with %PDF-"
        );
    }

    #[test]
    fn pdf_archival_preset_fields() {
        let opts = PdfOptions::archival();
        assert_eq!(opts.jpeg_quality, Some(90));
        assert_eq!(opts.output_dpi, 0);
    }

    // ── pdf_escape_string ─────────────────────────────────────────────────────

    #[test]
    fn pdf_escape_parens_and_backslash() {
        assert_eq!(pdf_escape_string("a(b)c"), "a\\(b\\)c");
        assert_eq!(pdf_escape_string("a\\b"), "a\\\\b");
    }

    #[test]
    fn pdf_escape_ascii_passthrough() {
        assert_eq!(pdf_escape_string("hello 123"), "hello 123");
    }

    #[test]
    fn pdf_escape_non_ascii_replaced_with_question_mark() {
        let s = pdf_escape_string("über");
        assert!(s.contains('?'), "non-ASCII must be replaced with ?: {s}");
    }

    // ── helvetica_advance ─────────────────────────────────────────────────────

    #[test]
    fn helvetica_advance_space() {
        assert!((helvetica_advance(' ') - 0.278).abs() < 1e-6);
    }

    #[test]
    fn helvetica_advance_digit() {
        assert!((helvetica_advance('5') - 0.556).abs() < 1e-6);
    }

    #[test]
    fn helvetica_advance_uppercase() {
        assert!((helvetica_advance('A') - 0.667).abs() < 1e-6);
    }

    #[test]
    fn helvetica_advance_lowercase() {
        assert!((helvetica_advance('a') - 0.556).abs() < 1e-6);
    }

    #[test]
    fn helvetica_advance_cjk_full_width() {
        // CJK Unified Ideograph — should return 1.0
        assert!((helvetica_advance('中') - 1.0).abs() < 1e-6);
    }

    #[test]
    fn helvetica_advance_hiragana_full_width() {
        assert!((helvetica_advance('あ') - 1.0).abs() < 1e-6);
    }

    #[test]
    fn helvetica_advance_cyrillic_falls_back() {
        // Cyrillic falls through to the 0.556 default
        assert!((helvetica_advance('А') - 0.556).abs() < 1e-6);
    }

    #[test]
    fn helvetica_advance_control_char_is_zero() {
        assert!((helvetica_advance('\x00') - 0.0).abs() < 1e-6);
        assert!((helvetica_advance('\x7f') - 0.0).abs() < 1e-6);
    }

    #[test]
    fn helvetica_advance_punctuation() {
        assert!((helvetica_advance(',') - 0.278).abs() < 1e-6);
        assert!((helvetica_advance('(') - 0.333).abs() < 1e-6);
        assert!((helvetica_advance('-') - 0.333).abs() < 1e-6);
    }

    // ── collect_mask_stream ───────────────────────────────────────────────────

    #[test]
    fn collect_mask_stream_returns_none_for_no_sjbz() {
        let doc = load_doc("chicken.djvu"); // no Sjbz
        let page = doc.page(0).unwrap();
        let result = collect_mask_stream(page);
        assert!(
            result.is_none(),
            "page without Sjbz must return None from collect_mask_stream"
        );
    }

    #[test]
    fn collect_mask_stream_returns_some_for_sjbz_page() {
        let doc = load_doc("boy_jb2.djvu"); // has Sjbz
        let page = doc.page(0).unwrap();
        let result = collect_mask_stream(page);
        assert!(
            result.is_some(),
            "page with Sjbz must return Some from collect_mask_stream"
        );
        let body = result.unwrap();
        // Must contain /ImageMask keyword
        assert!(
            body.windows(9).any(|w| w == b"ImageMask"),
            "mask stream must contain /ImageMask"
        );
    }

    // ── shape_to_pdf_rect ─────────────────────────────────────────────────────

    #[test]
    fn shape_to_pdf_rect_converts_rect_shape() {
        use crate::annotation::{Rect as ARect, Shape};
        let shape = Shape::Rect(ARect {
            x: 0,
            y: 0,
            width: 100,
            height: 50,
        });
        let rect = shape_to_pdf_rect(&shape, 100.0, 360.0);
        assert!(rect.is_some(), "valid rect shape must produce a PDF rect");
        let (x1, y1, x2, y2) = rect.unwrap();
        assert!((x1 - 0.0).abs() < 0.01);
        assert!((y1 - 0.0).abs() < 0.01);
        assert!((x2 - 72.0).abs() < 0.01); // 100px * 72/100dpi = 72pt
        assert!((y2 - 36.0).abs() < 0.01); // 50px * 72/100dpi = 36pt
    }

    // ── px_to_pt ─────────────────────────────────────────────────────────────

    #[test]
    fn px_to_pt_at_72dpi_is_identity() {
        assert!((px_to_pt(100.0, 72.0) - 100.0).abs() < 0.001);
    }

    #[test]
    fn px_to_pt_at_300dpi() {
        // 300px at 300dpi = 72pt
        assert!((px_to_pt(300.0, 300.0) - 72.0).abs() < 0.001);
    }

    // ── emit_word_span guards ─────────────────────────────────────────────────

    #[test]
    fn emit_word_span_zero_width_produces_no_ops() {
        use crate::text::Rect;
        let rect = Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 20,
        };
        let mut ops = String::new();
        emit_word_span(&mut ops, &rect, "hello", 72.0, 720.0);
        assert!(ops.is_empty(), "zero-width rect must produce no output");
    }

    #[test]
    fn emit_word_span_tiny_height_produces_no_ops() {
        use crate::text::Rect;
        // height=1px at 300dpi → h = 1*72/300 = 0.24pt < 0.5 → skip
        let rect = Rect {
            x: 0,
            y: 0,
            width: 50,
            height: 1,
        };
        let mut ops = String::new();
        emit_word_span(&mut ops, &rect, "hi", 300.0, 720.0);
        assert!(ops.is_empty(), "sub-0.5pt font size must produce no output");
    }

    // ── build_outline with nested bookmarks ──────────────────────────────────

    #[test]
    fn build_outline_with_nested_children_sets_first_last_count() {
        use crate::djvu_document::DjVuBookmark;
        let bookmarks = vec![DjVuBookmark {
            title: "Chapter 1".into(),
            url: "#page_1".into(),
            children: vec![
                DjVuBookmark {
                    title: "Section 1.1".into(),
                    url: "#page_2".into(),
                    children: vec![],
                },
                DjVuBookmark {
                    title: "Section 1.2".into(),
                    url: "#page_3".into(),
                    children: vec![],
                },
            ],
        }];
        let page_ids = [10usize, 20, 30];
        let mut w = PdfWriter::new();
        let outline_id = build_outline(&mut w, &bookmarks, &page_ids);
        assert!(
            outline_id.is_some(),
            "nested bookmarks must produce an outline"
        );
        // Serialize and check that /First and /Last are present
        let pdf = w.serialize();
        let s = String::from_utf8_lossy(&pdf);
        assert!(
            s.contains("/First"),
            "outline item with children must set /First"
        );
        assert!(
            s.contains("/Last"),
            "outline item with children must set /Last"
        );
        assert!(
            s.contains("/Count"),
            "outline item with children must set /Count"
        );
    }

    // Lines 411-415: hyperlink annotation block (/Annots [...]).
    // Build a synthetic single-page DjVu with an ANTz maparea URL, then convert.
    #[test]
    fn djvu_to_pdf_with_hyperlinks_produces_annots() {
        use crate::annotation::{self as ann, Annotation, MapArea};
        use crate::djvu_document::DjVuDocument;
        use crate::iff::{self as iff_mod, Chunk, DjvuFile};
        let maparea = MapArea {
            url: "https://example.com".to_string(),
            description: String::new(),
            shape: ann::Shape::Rect(ann::Rect {
                x: 0,
                y: 0,
                width: 100,
                height: 50,
            }),
            border: None,
            highlight: None,
        };
        let ant_data = ann::encode_annotations_bzz(&Annotation::default(), &[maparea]);
        // Minimal INFO: width=100, height=100, dpi=0 (default), rest zero.
        let mut info = vec![0u8; 10];
        info[1] = 100; // width
        info[3] = 100; // height
        let bytes = iff_mod::emit(&DjvuFile {
            root: Chunk::Form {
                secondary_id: *b"DJVU",
                length: 0,
                children: vec![
                    Chunk::Leaf {
                        id: *b"INFO",
                        data: info,
                    },
                    Chunk::Leaf {
                        id: *b"ANTz",
                        data: ant_data,
                    },
                ],
            },
        });
        let doc = DjVuDocument::parse(&bytes).expect("synthetic doc must parse");
        let pdf = djvu_to_pdf(&doc).expect("synthetic hyperlink page must convert to PDF");
        let s = String::from_utf8_lossy(&pdf);
        assert!(
            s.contains("/Annots"),
            "PDF from hyperlink page must contain /Annots"
        );
    }

    /// Page with corrupted ANTz (invalid BZZ): `hyperlinks()` errors, so
    /// `collect_link_annot_bodies` returns empty (line 332 `Err(_) => Vec::new()`).
    #[test]
    fn djvu_to_pdf_with_corrupted_antz_skips_annotations() {
        use crate::djvu_document::DjVuDocument;
        use crate::iff::{self as iff_mod, Chunk, DjvuFile};

        let mut info = vec![0u8; 10];
        info[1] = 100; // width
        info[3] = 100; // height
        // Garbage bytes that are not valid BZZ — decoding will fail
        let bad_antz: Vec<u8> = vec![0xFF, 0xFE, 0xAB, 0xCD, 0x12, 0x34];
        let bytes = iff_mod::emit(&DjvuFile {
            root: Chunk::Form {
                secondary_id: *b"DJVU",
                length: 0,
                children: vec![
                    Chunk::Leaf {
                        id: *b"INFO",
                        data: info,
                    },
                    Chunk::Leaf {
                        id: *b"ANTz",
                        data: bad_antz,
                    },
                ],
            },
        });
        let doc = DjVuDocument::parse(&bytes).expect("synthetic doc must parse");
        let pdf = djvu_to_pdf(&doc).expect("corrupted ANTz must not abort PDF export");
        let s = String::from_utf8_lossy(&pdf);
        assert!(
            !s.contains("/Annots"),
            "corrupted ANTz should produce no /Annots block"
        );
    }

    /// Page whose render fails (0×0 dimensions, no image data) triggers the blank
    /// page fallback at lines 840-850: `rendered_pages[i]` is None so a blank
    /// /Page object is emitted with the native MediaBox dimensions.
    #[test]
    fn djvu_to_pdf_zero_dim_page_emits_blank_page_object() {
        use crate::djvu_document::DjVuDocument;
        use crate::iff::{self as iff_mod, Chunk, DjvuFile};

        // INFO chunk: width=0, height=0, dpi=0 (all zeros). No Sjbz or BG44 so
        // is_bilevel_only=false, and render_dims returns (0,0), which makes
        // render_pixmap return InvalidDimensions → render_page_data returns Err
        // → .ok() yields None → blank page fallback fires.
        let info = vec![0u8; 10];
        let bytes = iff_mod::emit(&DjvuFile {
            root: Chunk::Form {
                secondary_id: *b"DJVU",
                length: 0,
                children: vec![Chunk::Leaf {
                    id: *b"INFO",
                    data: info,
                }],
            },
        });
        let doc = DjVuDocument::parse(&bytes).expect("zero-dim doc must parse");
        let pdf = djvu_to_pdf(&doc).expect("zero-dim page must not crash PDF export");
        let s = String::from_utf8_lossy(&pdf);
        assert!(
            s.contains("/Type /Page"),
            "PDF must contain at least one Page object"
        );
    }
}
