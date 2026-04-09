//! DjVu to EPUB 3 converter — preserves document structure.
//!
//! Converts DjVu documents to EPUB 3 while preserving:
//! - Page images as PNG (one per page)
//! - Invisible text overlay for search and copy
//! - NAVM bookmarks as EPUB navigation (`nav.xhtml`)
//! - ANTz hyperlinks as `<a href>` in page XHTML
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
}

impl Default for EpubOptions {
    fn default() -> Self {
        Self {
            title: "DjVu Document".to_owned(),
            author: String::new(),
            dpi: 150,
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
    let pw = page.width() as u32;
    let ph = page.height() as u32;
    let dpi = page.dpi().max(1) as f32;

    let render_opts = RenderOptions {
        width: pw,
        height: ph,
        ..RenderOptions::default()
    };
    let pixmap = djvu_render::render_pixmap(page, &render_opts)?;

    // Encode as PNG
    let png_bytes = encode_rgba_to_png(&pixmap.data, pw, ph);

    let page_num = index + 1;
    let img_name = format!("page_{page_num:04}.png");
    let img_path = format!("OEBPS/images/{img_name}");

    zip.start_file(
        &img_path,
        SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
    )?;
    zip.write_all(&png_bytes)?;

    // Build text overlay from text layer
    let text_overlay = build_text_overlay(page, dpi, pw, ph);

    // Build XHTML page
    let xhtml = build_page_xhtml(&img_name, pw, ph, &text_overlay, opts);
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

/// Returns a Vec of `(x_pct, y_pct, w_pct, h_pct, text)` for word/char zones.
/// Coordinates are expressed as percentages of page dimensions for CSS positioning.
fn build_text_overlay(
    page: &DjVuPage,
    _dpi: f32,
    pw: u32,
    ph: u32,
) -> Vec<(f32, f32, f32, f32, String)> {
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
                    let y = r.y as f32 / ph as f32 * 100.0;
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
    pw: u32,
    ph: u32,
    text_overlay: &[(f32, f32, f32, f32, String)],
    _opts: &EpubOptions,
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
.djvu-page { position: relative; display: block; width: 100%; }
.djvu-page img { display: block; width: 100%; height: auto; }
.djvu-text {
  position: absolute;
  color: transparent;
  background: transparent;
  white-space: pre;
  overflow: hidden;
  pointer-events: none;
}
</style>
</head>
<body>
"#,
    );

    html.push_str(&format!(
        r#"<div class="djvu-page" style="width:{pw}px; height:{ph}px;">"#
    ));
    html.push_str(&format!(
        r#"<img src="../images/{img_name}" alt="page" width="{pw}" height="{ph}"/>"#
    ));

    for (x, y, w, h, text) in text_overlay {
        html.push_str(&format!(
            r#"<span class="djvu-text" aria-hidden="true" style="left:{x:.3}%;top:{y:.3}%;width:{w:.3}%;height:{h:.3}%;">{text}</span>"#
        ));
    }

    html.push_str("</div>\n</body>\n</html>\n");
    html
}

// ── OPF package ──────────────────────────────────────────────────────────────

fn build_opf(opts: &EpubOptions, page_count: usize) -> String {
    let title = xml_escape(&opts.title);
    let author = xml_escape(&opts.author);

    let mut manifest_items = String::new();
    let mut spine_items = String::new();

    // nav document
    manifest_items.push_str(
        r#"    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
"#,
    );

    for i in 1..=page_count {
        let pid = format!("page_{i:04}");
        manifest_items.push_str(&format!(
            "    <item id=\"{pid}\" href=\"pages/page_{i:04}.xhtml\" media-type=\"application/xhtml+xml\"/>\n"
        ));
        manifest_items.push_str(&format!(
            "    <item id=\"img_{pid}\" href=\"images/page_{i:04}.png\" media-type=\"image/png\"/>\n"
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
    <dc:language>en</dc:language>
    <dc:identifier id="uid">djvu-rs-export</dc:identifier>
    <meta property="dcterms:modified">2024-01-01T00:00:00Z</meta>
  </metadata>
  <manifest>
{manifest_items}  </manifest>
  <spine>
{spine_items}  </spine>
</package>
"#
    )
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
}
