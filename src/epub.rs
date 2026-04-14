//! DjVu to EPUB 3 converter — preserves document structure.
//!
//! Converts DjVu documents to EPUB 3 while preserving:
//! - Page images as PNG (one per page)
//! - Invisible text overlay for search and copy
//! - NAVM bookmarks as EPUB navigation (`nav.xhtml`)
//! - ANTz/ANTa hyperlinks as `<a href>` overlays on each page
//!
//! # Example
//!
//! ```no_run
//! use djvu_rs::djvu_document::DjVuDocument;
//! use djvu_rs::epub::{djvu_to_epub, EpubOptions};
//!
//! let data = std::fs::read("book.djvu").unwrap();
//! let doc = DjVuDocument::parse(&data).unwrap();
//! let epub_bytes = djvu_to_epub(&doc, &EpubOptions::default()).unwrap();
//! std::fs::write("book.epub", epub_bytes).unwrap();
//! ```

use std::io::Write;

use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::{
    annotation::{MapArea, Shape},
    djvu_document::{DjVuBookmark, DjVuDocument, DjVuPage, DocError},
    djvu_render::{self, RenderError, RenderOptions},
    text::TextZoneKind,
};

// ── Errors ────────────────────────────────────────────────────────────────────

/// Errors from EPUB conversion.
#[derive(Debug, thiserror::Error)]
pub enum EpubError {
    /// Document model error.
    #[error("document error: {0}")]
    Doc(#[from] DocError),
    /// Render error.
    #[error("render error: {0}")]
    Render(#[from] RenderError),
    /// ZIP I/O error.
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    /// I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

// ── Options ───────────────────────────────────────────────────────────────────

/// Options for EPUB conversion.
#[derive(Debug, Clone)]
pub struct EpubOptions {
    /// Title embedded in the OPF metadata. Defaults to `"DjVu Document"`.
    pub title: String,
    /// Author embedded in the OPF metadata. Defaults to empty.
    pub author: String,
    /// DPI for page rendering. Defaults to 150.
    pub dpi: u32,
    /// BCP-47 language tag for `<dc:language>`. Defaults to `"en"`.
    pub language: String,
    /// ISO 8601 timestamp for `dcterms:modified` (e.g. `"2026-04-14T00:00:00Z"`).
    /// When `None`, the current UTC time is used (computed from `std::time::SystemTime`).
    pub modified: Option<String>,
}

impl Default for EpubOptions {
    fn default() -> Self {
        Self {
            title: "DjVu Document".to_owned(),
            author: String::new(),
            dpi: 150,
            language: "en".to_owned(),
            modified: None,
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Convert a DjVu document to EPUB 3.
///
/// Returns the raw bytes of a valid EPUB file (ZIP archive).
///
/// # Errors
///
/// Returns [`EpubError`] if page rendering or ZIP writing fails.
pub fn djvu_to_epub(doc: &DjVuDocument, opts: &EpubOptions) -> Result<Vec<u8>, EpubError> {
    let buf = Vec::new();
    let cursor = std::io::Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);

    // 1. mimetype — MUST be first and STORED (no compression), per EPUB spec
    zip.start_file(
        "mimetype",
        SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
    )?;
    zip.write_all(b"application/epub+zip")?;

    // 2. META-INF/container.xml
    zip.start_file(
        "META-INF/container.xml",
        SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
    )?;
    zip.write_all(CONTAINER_XML.as_bytes())?;

    // 3. Per-page content
    let page_count = doc.page_count();
    for i in 0..page_count {
        let page = doc.page(i)?;
        write_page(&mut zip, page, i, opts)?;
    }

    // 4. Navigation document
    let nav_xhtml = build_nav(doc.bookmarks(), page_count);
    zip.start_file(
        "OEBPS/nav.xhtml",
        SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
    )?;
    zip.write_all(nav_xhtml.as_bytes())?;

    // 5. OPF package document
    let opf = build_opf(opts, page_count);
    zip.start_file(
        "OEBPS/content.opf",
        SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
    )?;
    zip.write_all(opf.as_bytes())?;

    let cursor = zip.finish()?;
    Ok(cursor.into_inner())
}

// ── Per-page writer ───────────────────────────────────────────────────────────

fn write_page(
    zip: &mut ZipWriter<std::io::Cursor<Vec<u8>>>,
    page: &DjVuPage,
    index: usize,
    opts: &EpubOptions,
) -> Result<(), EpubError> {
    // Native page dimensions in DjVu pixels
    let pw = page.width() as u32;
    let ph = page.height() as u32;
    let page_dpi = page.dpi().max(1) as f32;

    // Scale to the requested output DPI
    let scale = opts.dpi as f32 / page_dpi;
    let w = ((pw as f32 * scale).round() as u32).max(1);
    let h = ((ph as f32 * scale).round() as u32).max(1);

    let render_opts = RenderOptions {
        width: w,
        height: h,
        scale,
        ..RenderOptions::default()
    };
    let pixmap = djvu_render::render_pixmap(page, &render_opts)?;

    // Encode as PNG
    let png_bytes = encode_rgba_to_png(&pixmap.data, w, h);

    let page_num = index + 1;
    let img_name = format!("page_{page_num:04}.png");
    let img_path = format!("OEBPS/images/{img_name}");

    zip.start_file(
        &img_path,
        SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
    )?;
    zip.write_all(&png_bytes)?;

    // Text overlay (invisible selectable text)
    let text_overlay = build_text_overlay(page, pw, ph);

    // Hyperlink overlays from ANTz/ANTa annotations
    let hyperlinks = page.hyperlinks().unwrap_or_default();

    // Build XHTML page
    let xhtml = build_page_xhtml(&img_name, w, h, pw, ph, &text_overlay, &hyperlinks);
    let xhtml_path = format!("OEBPS/pages/page_{page_num:04}.xhtml");

    zip.start_file(
        &xhtml_path,
        SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
    )?;
    zip.write_all(xhtml.as_bytes())?;

    Ok(())
}

// ── PNG encoder ───────────────────────────────────────────────────────────────

fn encode_rgba_to_png(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut enc = png::Encoder::new(std::io::Cursor::new(&mut buf), width, height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        if let Ok(mut writer) = enc.write_header() {
            let _ = writer.write_image_data(rgba);
        }
    }
    buf
}

// ── Text overlay ─────────────────────────────────────────────────────────────

/// Returns `(x_pct, y_pct, w_pct, h_pct, text)` for word/char zones.
///
/// Coordinates are CSS percentages of the rendered image dimensions.
/// DjVu text zones use bottom-left origin; the y-axis is inverted for CSS.
fn build_text_overlay(page: &DjVuPage, pw: u32, ph: u32) -> Vec<(f32, f32, f32, f32, String)> {
    let text_layer = match page.text_layer() {
        Ok(Some(tl)) => tl,
        _ => return Vec::new(),
    };

    let mut spans = Vec::new();

    fn walk(
        zones: &[crate::text::TextZone],
        spans: &mut Vec<(f32, f32, f32, f32, String)>,
        pw: u32,
        ph: u32,
    ) {
        for zone in zones {
            match zone.kind {
                TextZoneKind::Word | TextZoneKind::Character => {
                    if zone.text.is_empty() {
                        continue;
                    }
                    let r = &zone.rect;
                    let x = r.x as f32 / pw as f32 * 100.0;
                    // DjVu y is bottom-left origin; invert for CSS top
                    let y = (ph.saturating_sub(r.y + r.height)) as f32 / ph as f32 * 100.0;
                    let w = r.width as f32 / pw as f32 * 100.0;
                    let h = r.height as f32 / ph as f32 * 100.0;
                    if w > 0.0 && h > 0.0 {
                        spans.push((x, y, w, h, xml_escape(&zone.text)));
                    }
                }
                _ => walk(&zone.children, spans, pw, ph),
            }
        }
    }

    walk(&text_layer.zones, &mut spans, pw, ph);
    spans
}

// ── XHTML page ────────────────────────────────────────────────────────────────

fn build_page_xhtml(
    img_name: &str,
    w: u32,
    h: u32,
    pw: u32,
    ph: u32,
    text_overlay: &[(f32, f32, f32, f32, String)],
    hyperlinks: &[MapArea],
) -> String {
    let mut html = String::new();
    html.push_str(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head>
<meta charset="UTF-8"/>
<title>Page</title>
<style>
body { margin: 0; padding: 0; }
.djvu-page { position: relative; display: block; }
.djvu-page img { display: block; width: 100%; height: auto; }
.djvu-text {
  position: absolute;
  color: transparent;
  background: transparent;
  white-space: pre;
  overflow: hidden;
  pointer-events: none;
}
.djvu-link {
  position: absolute;
  display: block;
}
</style>
</head>
<body>
"#,
    );

    html.push_str(&format!(
        r#"<div class="djvu-page" style="width:{w}px; height:{h}px;">"#
    ));
    html.push_str(&format!(
        r#"<img src="../images/{img_name}" alt="page" width="{w}" height="{h}"/>"#
    ));

    for (x, y, ww, hh, text) in text_overlay {
        html.push_str(&format!(
            r#"<span class="djvu-text" aria-hidden="true" style="left:{x:.3}%;top:{y:.3}%;width:{ww:.3}%;height:{hh:.3}%;">{text}</span>"#
        ));
    }

    for ma in hyperlinks {
        if let Some((x, y, ww, hh)) = map_area_to_css(ma, pw, ph) {
            let href = resolve_link_href(&ma.url);
            let title = xml_escape(&ma.description);
            html.push_str(&format!(
                r#"<a class="djvu-link" href="{href}" title="{title}" style="left:{x:.3}%;top:{y:.3}%;width:{ww:.3}%;height:{hh:.3}%;"></a>"#
            ));
        }
    }

    html.push_str("</div>\n</body>\n</html>\n");
    html
}

/// Convert a `MapArea` shape to CSS percentage coordinates `(left, top, width, height)`.
///
/// Returns `None` for unsupported or degenerate shapes.
/// DjVu annotation y is bottom-left origin; the result is flipped for CSS.
fn map_area_to_css(ma: &MapArea, pw: u32, ph: u32) -> Option<(f32, f32, f32, f32)> {
    let rect = match &ma.shape {
        Shape::Rect(r) | Shape::Oval(r) | Shape::Text(r) => r,
        Shape::Poly(pts) => {
            // Bounding box of polygon
            let (min_x, min_y, max_x, max_y) = pts.iter().fold(
                (u32::MAX, u32::MAX, 0u32, 0u32),
                |(mnx, mny, mxx, mxy), &(px, py)| {
                    (mnx.min(px), mny.min(py), mxx.max(px), mxy.max(py))
                },
            );
            if max_x <= min_x || max_y <= min_y {
                return None;
            }
            let w = max_x - min_x;
            let h = max_y - min_y;
            let x = (min_x as f32 / pw as f32) * 100.0;
            let y = (ph.saturating_sub(max_y) as f32 / ph as f32) * 100.0;
            let ww = (w as f32 / pw as f32) * 100.0;
            let hh = (h as f32 / ph as f32) * 100.0;
            return Some((x, y, ww, hh));
        }
        Shape::Line(x1, y1, x2, y2) => {
            let min_x = (*x1).min(*x2);
            let min_y = (*y1).min(*y2);
            let max_y = (*y1).max(*y2);
            let w = ((*x1 as i64 - *x2 as i64).unsigned_abs() as u32).max(1);
            let h = ((*y1 as i64 - *y2 as i64).unsigned_abs() as u32).max(1);
            let x = (min_x as f32 / pw as f32) * 100.0;
            let y = (ph.saturating_sub(max_y + h) as f32 / ph as f32) * 100.0;
            let ww = (w as f32 / pw as f32) * 100.0;
            let hh = (h as f32 / ph as f32) * 100.0;
            return Some((x, y, ww, hh));
        }
    };
    if rect.width == 0 || rect.height == 0 || pw == 0 || ph == 0 {
        return None;
    }
    let x = (rect.x as f32 / pw as f32) * 100.0;
    let y = (ph.saturating_sub(rect.y + rect.height) as f32 / ph as f32) * 100.0;
    let ww = (rect.width as f32 / pw as f32) * 100.0;
    let hh = (rect.height as f32 / ph as f32) * 100.0;
    Some((x, y, ww, hh))
}

/// Resolve a DjVu annotation URL to an EPUB-relative href.
fn resolve_link_href(url: &str) -> String {
    bookmark_href(url)
}

// ── OPF package ──────────────────────────────────────────────────────────────

fn build_opf(opts: &EpubOptions, page_count: usize) -> String {
    let title = xml_escape(&opts.title);
    let author = xml_escape(&opts.author);
    let language = xml_escape(&opts.language);
    let modified = opts
        .modified
        .as_deref()
        .map(str::to_owned)
        .unwrap_or_else(current_timestamp);

    let mut manifest_items = String::new();
    let mut spine_items = String::new();

    // nav document
    manifest_items.push_str(
        r#"    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
"#,
    );

    // cover image (first page)
    if page_count > 0 {
        manifest_items.push_str(
            r#"    <item id="cover-image" href="images/page_0001.png" media-type="image/png" properties="cover-image"/>
"#,
        );
    }

    for i in 1..=page_count {
        let pid = format!("page_{i:04}");
        // skip the cover-image item (already added above) but still add the page entry
        if i > 1 {
            manifest_items.push_str(&format!(
                "    <item id=\"img_{pid}\" href=\"images/page_{i:04}.png\" media-type=\"image/png\"/>\n"
            ));
        }
        manifest_items.push_str(&format!(
            "    <item id=\"{pid}\" href=\"pages/page_{i:04}.xhtml\" media-type=\"application/xhtml+xml\"/>\n"
        ));
        spine_items.push_str(&format!("    <itemref idref=\"{pid}\"/>\n"));
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" epub:type="book"
         xmlns:epub="http://www.idpf.org/2007/ops" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>{title}</dc:title>
    <dc:creator>{author}</dc:creator>
    <dc:language>{language}</dc:language>
    <dc:identifier id="uid">djvu-rs-export</dc:identifier>
    <meta property="dcterms:modified">{modified}</meta>
  </metadata>
  <manifest>
{manifest_items}  </manifest>
  <spine>
{spine_items}  </spine>
</package>
"#
    )
}

/// Return an ISO 8601 UTC timestamp for the current time.
///
/// Uses only `std::time::SystemTime` — no external crate dependency.
fn current_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Compute Y/M/D H:M:S from Unix timestamp (no leap seconds, Gregorian)
    let (y, mo, d, hh, mm, ss) = unix_secs_to_parts(secs);
    format!("{y:04}-{mo:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// Decompose a Unix timestamp (seconds since 1970-01-01 00:00:00 UTC) into
/// `(year, month, day, hour, min, sec)`.
fn unix_secs_to_parts(secs: u64) -> (u32, u32, u32, u32, u32, u32) {
    let ss = (secs % 60) as u32;
    let mins = secs / 60;
    let mm = (mins % 60) as u32;
    let hours = mins / 60;
    let hh = (hours % 24) as u32;
    let days = (hours / 24) as u32;

    // Days since 1970-01-01 → Gregorian date (algorithm by Henry Fliegel)
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    (y, mo, d, hh, mm, ss)
}

// ── Navigation document ───────────────────────────────────────────────────────

fn build_nav(bookmarks: &[DjVuBookmark], page_count: usize) -> String {
    let toc_items = if bookmarks.is_empty() {
        build_default_nav_items(page_count)
    } else {
        build_bookmark_nav_items(bookmarks)
    };

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head><meta charset="UTF-8"/><title>Navigation</title></head>
<body>
<nav epub:type="toc" id="toc">
  <h1>Contents</h1>
  <ol>
{toc_items}  </ol>
</nav>
</body>
</html>
"#
    )
}

fn build_default_nav_items(page_count: usize) -> String {
    let mut s = String::new();
    for i in 1..=page_count {
        s.push_str(&format!(
            "    <li><a href=\"pages/page_{i:04}.xhtml\">Page {i}</a></li>\n"
        ));
    }
    s
}

fn build_bookmark_nav_items(bookmarks: &[DjVuBookmark]) -> String {
    let mut s = String::new();
    for bm in bookmarks {
        let title = xml_escape(&bm.title);
        let href = bookmark_href(&bm.url);
        s.push_str(&format!("    <li><a href=\"{href}\">{title}</a>"));
        if !bm.children.is_empty() {
            s.push_str("\n    <ol>\n");
            s.push_str(&build_bookmark_nav_items_inner(&bm.children, 2));
            s.push_str("    </ol>");
        }
        s.push_str("</li>\n");
    }
    s
}

fn build_bookmark_nav_items_inner(bookmarks: &[DjVuBookmark], depth: usize) -> String {
    let indent = "  ".repeat(depth + 1);
    let mut s = String::new();
    for bm in bookmarks {
        let title = xml_escape(&bm.title);
        let href = bookmark_href(&bm.url);
        s.push_str(&format!("{indent}<li><a href=\"{href}\">{title}</a>"));
        if !bm.children.is_empty() {
            s.push_str(&format!("\n{indent}<ol>\n"));
            s.push_str(&build_bookmark_nav_items_inner(&bm.children, depth + 1));
            s.push_str(&format!("{indent}</ol>"));
        }
        s.push_str("</li>\n");
    }
    s
}

/// Convert a DjVu bookmark URL to an EPUB relative href.
/// DjVu bookmarks use `#page=N` (1-based) or bare `#anchor` format.
fn bookmark_href(url: &str) -> String {
    // Try to parse `#page=N` pattern
    if let Some(rest) = url.strip_prefix('#') {
        if let Some(n_str) = rest.strip_prefix("page=")
            && let Ok(n) = n_str.trim().parse::<usize>()
            && n >= 1
        {
            return format!("pages/page_{n:04}.xhtml");
        }
        // plain anchor — link to page 1 with anchor
        return format!("pages/page_0001.xhtml{}", xml_escape(url));
    }
    // External URL — keep as-is
    xml_escape(url)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

const CONTAINER_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf"
              media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_escape_basic() {
        assert_eq!(
            xml_escape("a&b<c>d\"e'f"),
            "a&amp;b&lt;c&gt;d&quot;e&apos;f"
        );
    }

    #[test]
    fn bookmark_href_page_number() {
        assert_eq!(bookmark_href("#page=3"), "pages/page_0003.xhtml");
        assert_eq!(bookmark_href("#page=1"), "pages/page_0001.xhtml");
    }

    #[test]
    fn bookmark_href_external() {
        assert_eq!(bookmark_href("https://example.com"), "https://example.com");
    }

    #[test]
    fn nav_has_toc_for_empty_bookmarks() {
        let nav = build_nav(&[], 2);
        assert!(nav.contains("epub:type=\"toc\""));
        assert!(nav.contains("page_0001.xhtml"));
        assert!(nav.contains("page_0002.xhtml"));
    }

    #[test]
    fn current_timestamp_looks_like_iso8601() {
        let ts = current_timestamp();
        // e.g. "2026-04-14T12:34:56Z"
        assert_eq!(ts.len(), 20);
        assert!(ts.ends_with('Z'));
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], "T");
    }

    #[test]
    fn unix_secs_epoch() {
        let (y, mo, d, hh, mm, ss) = unix_secs_to_parts(0);
        assert_eq!((y, mo, d, hh, mm, ss), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn unix_secs_known_date() {
        // 2026-04-14T00:00:00Z = 1776124800
        let (y, mo, d, hh, mm, ss) = unix_secs_to_parts(1_776_124_800);
        assert_eq!((y, mo, d, hh, mm, ss), (2026, 4, 14, 0, 0, 0));
    }

    #[test]
    fn epub_options_default_language_is_en() {
        assert_eq!(EpubOptions::default().language, "en");
    }

    #[test]
    fn epub_options_default_modified_is_none() {
        assert!(EpubOptions::default().modified.is_none());
    }

    #[test]
    fn opf_contains_cover_image_for_nonempty_doc() {
        let opf = build_opf(&EpubOptions::default(), 3);
        assert!(opf.contains("cover-image"));
        assert!(opf.contains("properties=\"cover-image\""));
    }

    #[test]
    fn opf_no_cover_image_for_empty_doc() {
        let opf = build_opf(&EpubOptions::default(), 0);
        assert!(!opf.contains("cover-image"));
    }

    #[test]
    fn opf_uses_custom_language() {
        let opts = EpubOptions {
            language: "ru".to_owned(),
            ..Default::default()
        };
        let opf = build_opf(&opts, 1);
        assert!(opf.contains("<dc:language>ru</dc:language>"));
    }

    #[test]
    fn opf_uses_custom_modified() {
        let opts = EpubOptions {
            modified: Some("2025-01-01T00:00:00Z".to_owned()),
            ..Default::default()
        };
        let opf = build_opf(&opts, 1);
        assert!(opf.contains("2025-01-01T00:00:00Z"));
    }
}
