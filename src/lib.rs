//! Pure Rust DjVu renderer.
//!
//! # Example
//!
//! ```no_run
//! use cos_djvu::Document;
//!
//! let doc = Document::open("file.djvu").unwrap();
//! println!("{} pages", doc.page_count());
//!
//! let page = doc.page(0).unwrap();
//! println!("{}x{} @ {} dpi", page.width(), page.height(), page.dpi());
//!
//! let pixmap = page.render().unwrap();
//! // pixmap.data: RGBA, pixmap.to_rgb(): RGB
//! ```

#![forbid(unsafe_code)]

pub(crate) mod bitmap;
#[doc(hidden)]
pub mod bzz;
#[doc(hidden)]
pub mod document;
pub(crate) mod error;
#[doc(hidden)]
pub mod iff;
#[doc(hidden)]
pub mod iw44;
#[doc(hidden)]
pub mod jb2;
pub(crate) mod pixmap;
#[doc(hidden)]
pub mod render;
#[doc(hidden)]
pub mod zp;

pub use bitmap::Bitmap;
pub use document::{Bookmark, Rotation, TextLayer, TextZone, TextZoneKind};
pub use error::Error;
pub use pixmap::Pixmap;

use self_cell::self_cell;

type ParsedDocument<'a> = document::Document<'a>;

self_cell!(
    struct DocumentInner {
        owner: Box<[u8]>,

        #[covariant]
        dependent: ParsedDocument,
    }
);

/// A parsed DjVu document. Owns its data and the parsed structure.
///
/// Parsing happens once at construction time. All subsequent `page()` and
/// `render()` calls reuse the parsed chunk tree with zero re-parsing overhead.
pub struct Document {
    inner: DocumentInner,
}

impl Document {
    /// Open a DjVu file from disk.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let data = std::fs::read(path.as_ref())
            .map_err(|e| Error::FormatError(format!("failed to read file: {}", e)))?;
        Self::from_bytes(data)
    }

    /// Parse a DjVu document from a reader (reads all bytes into memory).
    pub fn from_reader(reader: impl std::io::Read) -> Result<Self, Error> {
        let mut reader = reader;
        let mut data = Vec::new();
        reader
            .read_to_end(&mut data)
            .map_err(|e| Error::FormatError(format!("failed to read: {}", e)))?;
        Self::from_bytes(data)
    }

    /// Parse a DjVu document from owned bytes.
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, Error> {
        let inner = DocumentInner::try_new(data.into_boxed_slice(), |bytes| {
            document::Document::parse(bytes)
        })?;
        Ok(Document { inner })
    }

    /// Parse the NAVM bookmarks (table of contents).
    pub fn bookmarks(&self) -> Result<Vec<Bookmark>, Error> {
        self.inner.borrow_dependent().bookmarks()
    }

    /// Number of pages.
    pub fn page_count(&self) -> usize {
        self.inner.borrow_dependent().page_count()
    }

    /// Access a page by 0-based index.
    pub fn page(&self, index: usize) -> Result<Page<'_>, Error> {
        let inner = self.inner.borrow_dependent().page(index)?;
        Ok(Page {
            width: inner.info.width,
            height: inner.info.height,
            dpi: inner.info.dpi,
            rotation: inner.info.rotation,
            index,
            doc: self,
        })
    }
}

/// A page within a DjVu document.
pub struct Page<'a> {
    width: u16,
    height: u16,
    dpi: u16,
    rotation: document::Rotation,
    index: usize,
    doc: &'a Document,
}

impl<'a> Page<'a> {
    /// Page width in pixels (before rotation).
    pub fn width(&self) -> u32 {
        self.width as u32
    }

    /// Page height in pixels (before rotation).
    pub fn height(&self) -> u32 {
        self.height as u32
    }

    /// Effective page width after rotation.
    pub fn display_width(&self) -> u32 {
        match self.rotation {
            document::Rotation::Cw90 | document::Rotation::Cw270 => self.height as u32,
            _ => self.width as u32,
        }
    }

    /// Effective page height after rotation.
    pub fn display_height(&self) -> u32 {
        match self.rotation {
            document::Rotation::Cw90 | document::Rotation::Cw270 => self.width as u32,
            _ => self.height as u32,
        }
    }

    /// Page resolution in dots per inch.
    pub fn dpi(&self) -> u16 {
        self.dpi
    }

    /// The 0-based index of this page within the document.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Page rotation from the INFO chunk.
    pub fn rotation(&self) -> Rotation {
        self.rotation
    }

    /// Render the page to an RGBA pixmap at native resolution.
    pub fn render(&self) -> Result<Pixmap, Error> {
        let page = self.doc.inner.borrow_dependent().page(self.index)?;
        render::render(&page)
    }

    /// Render the page to an RGBA pixmap at a target size.
    ///
    /// Layers are composited directly at the target resolution --
    /// no intermediate full-size buffer is allocated.
    pub fn render_to_size(&self, width: u32, height: u32) -> Result<Pixmap, Error> {
        let page = self.doc.inner.borrow_dependent().page(self.index)?;
        render::render_to_size(&page, width, height)
    }

    /// Render the page at native resolution with mask dilation for bolder text.
    ///
    /// Each dilation pass thickens every stroke by ~1 pixel in each direction.
    /// Typically 1 pass is enough for improved legibility at reduced display sizes.
    pub fn render_bold(&self, dilate_passes: u32) -> Result<Pixmap, Error> {
        let page = self.doc.inner.borrow_dependent().page(self.index)?;
        render::render_to_size_bold(
            &page,
            page.info.width as u32,
            page.info.height as u32,
            dilate_passes,
        )
    }

    /// Render the page to a target size with mask dilation for bolder text.
    pub fn render_to_size_bold(
        &self,
        width: u32,
        height: u32,
        dilate_passes: u32,
    ) -> Result<Pixmap, Error> {
        let page = self.doc.inner.borrow_dependent().page(self.index)?;
        render::render_to_size_bold(&page, width, height, dilate_passes)
    }

    /// Render the page at a target size with anti-aliased downscaling.
    ///
    /// Renders internally at native resolution, then box-downsamples to the
    /// target size with a contrast curve that darkens anti-aliased edges.
    /// `boldness` controls how aggressively edges are darkened:
    /// 0.0 = neutral, 0.5 = moderate, 1.0 = strong.
    pub fn render_aa(&self, width: u32, height: u32, boldness: f32) -> Result<Pixmap, Error> {
        let page = self.doc.inner.borrow_dependent().page(self.index)?;
        render::render_aa(&page, width, height, boldness)
    }

    /// Decode the page thumbnail, if available.
    ///
    /// Returns `Ok(None)` if no thumbnail exists for this page.
    pub fn thumbnail(&self) -> Result<Option<Pixmap>, Error> {
        self.doc.inner.borrow_dependent().thumbnail(self.index)
    }

    /// Extract the text layer (TXTz/TXTa) with zone hierarchy.
    ///
    /// Returns `Ok(None)` if the page has no text layer.
    pub fn text_layer(&self) -> Result<Option<TextLayer>, Error> {
        let page = self.doc.inner.borrow_dependent().page(self.index)?;
        page.text_layer()
    }

    /// Extract the plain text content of the page.
    ///
    /// Returns `Ok(None)` if the page has no text layer.
    pub fn text(&self) -> Result<Option<String>, Error> {
        Ok(self.text_layer()?.map(|tl| tl.text))
    }

    /// Render the page scaled by a factor (e.g. 0.5 = half size, 2.0 = double).
    pub fn render_scaled(&self, scale: f32) -> Result<Pixmap, Error> {
        let dw = self.display_width();
        let dh = self.display_height();
        let w = ((dw as f32 * scale).round() as u32).max(1);
        let h = ((dh as f32 * scale).round() as u32).max(1);
        // render_to_size works in pre-rotation coords
        let (tw, th) = match self.rotation {
            document::Rotation::Cw90 | document::Rotation::Cw270 => (h, w),
            _ => (w, h),
        };
        let page = self.doc.inner.borrow_dependent().page(self.index)?;
        render::render_to_size(&page, tw, th)
    }
}

// Compile-time assertions: Document is Send + Sync.
#[allow(dead_code)]
const _: () = {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    fn assertions() {
        assert_send::<Document>();
        assert_sync::<Document>();
    }
};

#[cfg(test)]
mod tests {
    use super::*;

    fn assets_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/djvujs/library/assets")
    }

    fn golden_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/composite")
    }

    #[test]
    fn open_and_page_count() {
        let cases: &[(&str, usize)] = &[
            ("boy_jb2.djvu", 1),
            ("chicken.djvu", 1),
            ("navm_fgbz.djvu", 6),
            ("DjVu3Spec_bundled.djvu", 71),
            ("colorbook.djvu", 62),
        ];
        for (file, expected) in cases {
            let doc = Document::open(assets_path().join(file)).unwrap();
            assert_eq!(doc.page_count(), *expected, "{}", file);
        }
    }

    #[test]
    fn page_dimensions() {
        let doc = Document::open(assets_path().join("chicken.djvu")).unwrap();
        let page = doc.page(0).unwrap();
        assert_eq!(page.width(), 181);
        assert_eq!(page.height(), 240);
        assert_eq!(page.dpi(), 100);
    }

    #[test]
    fn render_all_test_files() {
        let files = [
            "boy_jb2.djvu",
            "boy.djvu",
            "chicken.djvu",
            "carte.djvu",
            "navm_fgbz.djvu",
            "DjVu3Spec_bundled.djvu",
            "colorbook.djvu",
            "big-scanned-page.djvu",
        ];
        for file in &files {
            let doc = Document::open(assets_path().join(file)).unwrap();
            let page = doc.page(0).unwrap();
            let pixmap = page.render().unwrap();
            assert!(pixmap.width > 0, "{}: zero width", file);
            assert!(pixmap.height > 0, "{}: zero height", file);
            assert_eq!(
                pixmap.data.len(),
                pixmap.width as usize * pixmap.height as usize * 4,
                "{}: data length mismatch",
                file
            );
        }
    }

    #[test]
    fn render_pixel_exact_chicken() {
        let doc = Document::open(assets_path().join("chicken.djvu")).unwrap();
        let pixmap = doc.page(0).unwrap().render().unwrap();
        let expected = std::fs::read(golden_path().join("chicken.ppm")).unwrap();
        assert_eq!(pixmap.to_ppm(), expected);
    }

    #[test]
    fn render_pixel_exact_boy_jb2() {
        let doc = Document::open(assets_path().join("boy_jb2.djvu")).unwrap();
        let pixmap = doc.page(0).unwrap().render().unwrap();
        let expected = std::fs::read(golden_path().join("boy_jb2.ppm")).unwrap();
        assert_eq!(pixmap.to_ppm(), expected);
    }

    #[test]
    fn to_rgb_output() {
        let doc = Document::open(assets_path().join("chicken.djvu")).unwrap();
        let pixmap = doc.page(0).unwrap().render().unwrap();
        let rgb = pixmap.to_rgb();
        assert_eq!(
            rgb.len(),
            pixmap.width as usize * pixmap.height as usize * 3
        );
    }

    #[test]
    fn from_bytes() {
        let data = std::fs::read(assets_path().join("boy_jb2.djvu")).unwrap();
        let doc = Document::from_bytes(data).unwrap();
        assert_eq!(doc.page_count(), 1);
    }

    #[test]
    fn display_dimensions_with_rotation() {
        let doc = Document::open(assets_path().join("boy_jb2_rotate90.djvu")).unwrap();
        let page = doc.page(0).unwrap();
        let w = page.width();
        let h = page.height();
        // Rotated 90 degrees swaps dimensions
        assert_eq!(page.display_width(), h);
        assert_eq!(page.display_height(), w);
    }

    #[test]
    fn render_boy_jb2_rotate180() {
        let doc = Document::open(assets_path().join("boy_jb2_rotate180.djvu")).unwrap();
        let page = doc.page(0).unwrap();
        // 180 degrees keeps dimensions the same
        assert_eq!(page.display_width(), page.width());
        assert_eq!(page.display_height(), page.height());
        let pm = page.render().unwrap();
        assert!(pm.width > 0 && pm.height > 0);
    }

    #[test]
    fn render_boy_jb2_rotate270() {
        let doc = Document::open(assets_path().join("boy_jb2_rotate270.djvu")).unwrap();
        let page = doc.page(0).unwrap();
        // 270 degrees swaps dimensions like 90 degrees
        assert_eq!(page.display_width(), page.height());
        assert_eq!(page.display_height(), page.width());
        let pm = page.render().unwrap();
        assert!(pm.width > 0 && pm.height > 0);
    }

    #[test]
    fn parse_ccitt2_empty_last_chunk() {
        let doc = Document::open(assets_path().join("ccitt_2.djvu")).unwrap();
        assert!(doc.page_count() >= 1);
        let page = doc.page(0).unwrap();
        let pm = page.render().unwrap();
        assert!(pm.width > 0 && pm.height > 0);
    }

    #[test]
    fn parse_links_minimal_file() {
        let doc = Document::open(assets_path().join("links.djvu")).unwrap();
        assert!(doc.page_count() >= 1);
    }

    #[test]
    fn render_problem_page_jb2_edge_case() {
        let doc = Document::open(assets_path().join("problem_page.djvu")).unwrap();
        let page = doc.page(0).unwrap();
        let pm = page.render().unwrap();
        assert!(pm.width > 0 && pm.height > 0);
    }

    #[test]
    fn render_vega_jb2_empty_edges() {
        let doc = Document::open(assets_path().join("vega.djvu")).unwrap();
        let page = doc.page(0).unwrap();
        let pm = page.render().unwrap();
        assert!(pm.width > 0 && pm.height > 0);
    }

    #[test]
    fn render_malliavin_empty_page() {
        let doc = Document::open(assets_path().join("malliavin.djvu")).unwrap();
        // Page 6 (index 5) is reported as empty in djvujs
        assert!(doc.page_count() >= 6);
        // All pages should at least parse without panic
        for i in 0..doc.page_count() {
            let page = doc.page(i).unwrap();
            let _ = page.render();
        }
    }

    #[test]
    fn parse_irish_multi_bzz() {
        let doc = Document::open(assets_path().join("irish.djvu")).unwrap();
        assert!(doc.page_count() >= 1);
        let page = doc.page(0).unwrap();
        let pm = page.render().unwrap();
        assert!(pm.width > 0 && pm.height > 0);
    }

    #[test]
    fn parse_czech_text_layer() {
        let doc = Document::open(assets_path().join("czech.djvu")).unwrap();
        assert!(doc.page_count() >= 7);
        let page = doc.page(0).unwrap();
        let pm = page.render().unwrap();
        assert!(pm.width > 0 && pm.height > 0);
    }

    #[test]
    fn parse_history_cyrillic_ids() {
        let doc = Document::open(assets_path().join("history.djvu")).unwrap();
        assert!(doc.page_count() >= 1);
        let page = doc.page(0).unwrap();
        let pm = page.render().unwrap();
        assert!(pm.width > 0 && pm.height > 0);
    }

    #[test]
    fn bookmarks_navm_fgbz() {
        let doc = Document::open(assets_path().join("navm_fgbz.djvu")).unwrap();
        let bm = doc.bookmarks().unwrap();
        assert_eq!(bm.len(), 4);
        assert_eq!(bm[2].title, "Stamps");
        assert_eq!(bm[2].children.len(), 2);
    }

    #[test]
    fn bookmarks_empty_when_absent() {
        let doc = Document::open(assets_path().join("boy_jb2.djvu")).unwrap();
        assert!(doc.bookmarks().unwrap().is_empty());
    }

    #[test]
    fn text_extraction_djvu3spec() {
        let doc = Document::open(assets_path().join("DjVu3Spec_bundled.djvu")).unwrap();
        let page = doc.page(0).unwrap();

        let tl = page.text_layer().unwrap().unwrap();
        assert!(!tl.text.is_empty());
        assert!(tl.text.contains("Introduction"));

        let root = tl.root.as_ref().unwrap();
        assert_eq!(root.kind, TextZoneKind::Page);
    }

    #[test]
    fn text_plain_convenience() {
        let doc = Document::open(assets_path().join("DjVu3Spec_bundled.djvu")).unwrap();
        let text = doc.page(0).unwrap().text().unwrap().unwrap();
        assert!(text.contains("Introduction"));
    }

    #[test]
    fn text_none_for_image_only() {
        let doc = Document::open(assets_path().join("chicken.djvu")).unwrap();
        assert!(doc.page(0).unwrap().text().unwrap().is_none());
    }

    #[test]
    fn thumbnail_carte() {
        let doc = Document::open(assets_path().join("carte.djvu")).unwrap();
        let thumb = doc
            .page(0)
            .unwrap()
            .thumbnail()
            .unwrap()
            .expect("carte should have thumbnail");
        assert!(thumb.width > 0 && thumb.width < 500);
        assert!(thumb.height > 0 && thumb.height < 500);
    }

    #[test]
    fn thumbnail_none_for_image_only() {
        let doc = Document::open(assets_path().join("chicken.djvu")).unwrap();
        assert!(doc.page(0).unwrap().thumbnail().unwrap().is_none());
    }

    #[test]
    fn render_slow_large_document() {
        let doc = Document::open(assets_path().join("slow.djvu")).unwrap();
        assert!(doc.page_count() >= 1);
        // Just render first page -- this is a perf stress test
        let page = doc.page(0).unwrap();
        let pm = page.render().unwrap();
        assert!(pm.width > 0 && pm.height > 0);
    }
}
