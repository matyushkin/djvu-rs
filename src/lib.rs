//! Pure-Rust DjVu decoder written from the DjVu v3 public specification.
//!
//! This crate implements the full DjVu v3 document format in safe Rust,
//! including IFF container parsing, JB2 bilevel decoding, IW44 wavelet
//! decoding, BZZ decompression, text layer extraction, and annotation parsing.
//! All algorithms are written from the public DjVu spec with no GPL code.
//!
//! # Key public types
//!
//! - [`DjVuError`] — top-level error enum (wraps [`IffError`], etc.)
//! - [`IffError`] — errors from the IFF container parser
//! - [`PageInfo`] — page metadata parsed from the INFO chunk
//! - [`Rotation`] — page rotation enum (None, Ccw90, Rot180, Cw90)
//! - [`DjVuDocument`] — high-level document model (IFF/BZZ/IW44 based)
//! - [`DjVuPage`] — lazy page handle
//! - [`DjVuBookmark`] — NAVM bookmark (table of contents)
//! - [`DocError`] — error type for the document model
//! - [`djvu_render::RenderOptions`] — render parameters
//! - [`djvu_render::RenderError`] — render pipeline error type
//! - [`text::TextLayer`] — text layer from TXTz/TXTa chunks
//! - [`text::TextZone`] — a zone node in the text layer hierarchy
//! - [`annotation::Annotation`] — page-level annotation
//! - [`annotation::MapArea`] — clickable area with URL and shape
//! - [`Pixmap`] — RGBA pixel buffer returned by render methods
//! - [`Bitmap`] — 1-bit bitmap for JB2 mask layers
//! - [`Document`] — owned DjVu document (high-level std API, requires std feature)
//! - [`Page`] — a page within a [`Document`]
//!
//! # Quick start
//!
//! ```no_run
//! use djvu_rs::Document;
//!
//! let doc = Document::open("file.djvu").unwrap();
//! println!("{} pages", doc.page_count());
//!
//! let page = doc.page(0).unwrap();
//! println!("{}x{} @ {} dpi", page.width(), page.height(), page.dpi());
//!
//! let pixmap = page.render().unwrap();
//! // pixmap.data: RGBA bytes
//! ```
//!
//! # IFF parser
//!
//! ```no_run
//! use djvu_rs::iff::parse_form;
//!
//! let data = std::fs::read("file.djvu").unwrap();
//! let form = parse_form(&data).unwrap();
//! println!("form type: {:?}", std::str::from_utf8(&form.form_type));
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#[cfg(not(feature = "std"))]
extern crate alloc;

// ---- New phase-1 modules ---------------------------------------------------
//
// These are the new clean-room implementations written from the DjVu spec.
// They are exposed under their natural names. The legacy modules that conflict
// are kept under different names below.

/// IFF container parser (phase 1, written from spec).
pub mod iff;

/// Typed error hierarchy for the new implementation (phase 1).
///
/// Key types: `DjVuError`, `IffError`, `BzzError`, `Jb2Error`, `Iw44Error`,
/// `LegacyError`. See also `text::TextError` and `annotation::AnnotationError`.
pub mod error;

/// INFO chunk parser (phase 1).
pub(crate) mod info;

/// ZP arithmetic coder — clean-room implementation (phase 2a).
///
/// Provides `ZpDecoder` for use by the new BZZ decompressor and future
/// phase decoders (JB2, IW44). Not yet wired into the legacy rendering path.
#[path = "zp/mod.rs"]
#[allow(dead_code)]
pub(crate) mod zp_impl;

/// BZZ decompressor — clean-room implementation.
///
/// Provides `bzz_new::bzz_decode` for decompressing DjVu BZZ streams
/// (DIRM, NAVM, ANTz chunks).
#[allow(dead_code)]
pub mod bzz_new;

/// JB2 bilevel image decoder — clean-room implementation (phase 2b).
///
/// Decodes JB2-encoded bitonal images from DjVu Sjbz and Djbz chunks using
/// ZP adaptive arithmetic coding with a symbol dictionary.
///
/// Key public types: `jb2_new::Jb2Dict`, `jb2_new::decode`, `jb2_new::decode_dict`.
#[path = "jb2_new.rs"]
pub mod jb2_new;

/// IW44 wavelet image decoder — clean-room implementation (phase 2c).
///
/// Provides `iw44_new::Iw44Image` for decoding BG44/FG44/TH44 chunks.
/// Uses planar YCbCr storage and a ZP arithmetic coder.
/// RGB conversion happens only in `iw44_new::Iw44Image::to_rgb`.
#[path = "iw44_new.rs"]
pub mod iw44_new;

/// New document model — phase 3.
///
/// Provides [`DjVuDocument`] (high-level document API built on the new IFF/BZZ/IW44
/// clean-room implementations), [`DjVuPage`] (lazy page handle), and
/// [`DjVuBookmark`] (NAVM table-of-contents entry).
pub mod djvu_document;

/// Rendering pipeline for [`DjVuPage`] — phase 5.
///
/// Provides `djvu_render::RenderOptions`, `djvu_render::RenderRect`,
/// `djvu_render::render_into`, `djvu_render::render_pixmap`,
/// `djvu_render::render_region`, `djvu_render::render_coarse`, and
/// `djvu_render::render_progressive`.
pub mod djvu_render;

/// Text layer parser for DjVu TXTz/TXTa chunks — phase 4.
///
/// Provides [`text::parse_text_layer`] and [`text::parse_text_layer_bzz`]
/// plus typed structs [`text::TextLayer`], [`text::TextZone`],
/// [`text::TextZoneKind`], and [`text::Rect`].
pub mod text;

/// Annotation parser for DjVu ANTz/ANTa chunks — phase 4.
///
/// Provides [`annotation::parse_annotations`] and [`annotation::parse_annotations_bzz`]
/// plus typed structs [`annotation::Annotation`], [`annotation::MapArea`],
/// [`annotation::Shape`], and [`annotation::Color`].
pub mod annotation;

/// Document metadata parser for METa/METz chunks — phase 4 extension.
///
/// Provides [`metadata::parse_metadata`] and [`metadata::parse_metadata_bzz`]
/// plus [`metadata::DjVuMetadata`] and [`metadata::MetadataError`].
pub mod metadata;

/// DjVu to PDF converter — phase 6.
///
/// Converts DjVu documents to PDF preserving structure: rasterized page images,
/// invisible text layer (searchable), bookmarks (PDF outline), and hyperlinks
/// (PDF link annotations).
///
/// Key function: [`pdf::djvu_to_pdf`].
#[cfg(feature = "std")]
pub mod pdf;

/// DjVu to EPUB 3 exporter.
///
/// Converts DjVu documents to EPUB 3 while preserving page images,
/// invisible text overlay for search/copy, and NAVM bookmarks as navigation.
///
/// Key function: [`epub::djvu_to_epub`].
#[cfg(feature = "epub")]
pub mod epub;

/// DjVu to TIFF exporter — phase 4 format extension.
///
/// Converts DjVu documents to multi-page TIFF files in color (RGB8) or
/// bilevel (Gray8) modes.
///
/// Key function: [`tiff_export::djvu_to_tiff`].
#[cfg(feature = "tiff")]
pub mod tiff_export;

/// Async render surface for [`DjVuPage`] — phase 5 extension.
///
/// Wraps the synchronous render pipeline in [`tokio::task::spawn_blocking`]
/// so CPU-bound IW44/JB2 work runs on the blocking thread pool without
/// blocking the async runtime.
///
/// Key functions: [`djvu_async::render_pixmap_async`], [`djvu_async::render_gray8_async`], [`djvu_async::render_progressive_stream`].
#[cfg(feature = "async")]
pub mod djvu_async;

/// `image::ImageDecoder` integration — allows DjVu pages to be used as
/// first-class image sources in the `image` crate ecosystem.
///
/// Key types: [`image_compat::DjVuDecoder`], [`image_compat::ImageCompatError`].
#[cfg(feature = "image")]
pub mod image_compat;

/// hOCR and ALTO XML export for the text layer.
///
/// Key functions: [`ocr_export::to_hocr`], [`ocr_export::to_alto`].
/// Key types: [`ocr_export::HocrOptions`], [`ocr_export::AltoOptions`],
/// [`ocr_export::OcrExportError`].
#[cfg(feature = "std")]
pub mod ocr_export;

#[cfg(feature = "wasm")]
pub mod wasm;

// Re-export new phase-1 error types
pub use error::{BzzError, DjVuError, IffError, Iw44Error, Jb2Error};

// Re-export new phase-3 document model
pub use djvu_document::{DjVuBookmark, DjVuDocument, DjVuPage, DocError};

// Re-export new phase-1 page info types
pub use info::{PageInfo, Rotation};

// ---- Rendering / document modules ------------------------------------------
//
// These modules implement the rendering pipeline. They depend on bitmap,
// pixmap, iw44, jb2, bzz. They require std (std::io, std::path, Vec, etc.)
// so they are gated behind #[cfg(feature = "std")].

#[doc(hidden)]
pub(crate) mod bitmap;

#[cfg(feature = "std")]
#[doc(hidden)]
pub mod document;

#[cfg(feature = "std")]
#[doc(hidden)]
pub mod iw44;

#[cfg(feature = "std")]
#[doc(hidden)]
pub mod jb2;

#[doc(hidden)]
pub(crate) mod pixmap;

#[cfg(feature = "std")]
#[doc(hidden)]
pub mod render;

#[cfg(feature = "std")]
#[doc(hidden)]
#[path = "zp_legacy/mod.rs"]
pub mod zp;

// Re-export types needed by both legacy and new phase modules
pub use bitmap::Bitmap;
pub use pixmap::{GrayPixmap, Pixmap};

// Re-export legacy types (only with std feature)
#[cfg(feature = "std")]
pub use document::{Bookmark, TextLayer, TextZone, TextZoneKind};

// Legacy error type (re-exported from legacy_error module included via error.rs)
#[cfg(feature = "std")]
pub use error::LegacyError as Error;

/// A parsed DjVu document. Owns the parsed structure.
#[cfg(feature = "std")]
///
/// Parsing happens once at construction time. All subsequent `page()` and
/// `render()` calls reuse the parsed chunk tree with zero re-parsing overhead.
pub struct Document {
    doc: document::Document,
}

#[cfg(feature = "std")]
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
        let doc = document::Document::parse(&data)?;
        Ok(Document { doc })
    }

    /// Parse the NAVM bookmarks (table of contents).
    pub fn bookmarks(&self) -> Result<Vec<Bookmark>, Error> {
        self.doc.bookmarks()
    }

    /// Number of pages.
    pub fn page_count(&self) -> usize {
        self.doc.page_count()
    }

    /// Access a page by 0-based index.
    pub fn page(&self, index: usize) -> Result<Page<'_>, Error> {
        let inner = self.doc.page(index)?;
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
#[cfg(feature = "std")]
pub struct Page<'a> {
    width: u16,
    height: u16,
    dpi: u16,
    rotation: document::Rotation,
    index: usize,
    doc: &'a Document,
}

#[cfg(feature = "std")]
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
    pub fn rotation(&self) -> document::Rotation {
        self.rotation
    }

    /// Decode the JB2 mask layer only (no compositing).
    ///
    /// Returns `None` when the page has no Sjbz chunk (pure IW44 background page).
    /// Useful for benchmarking the decode phase in isolation.
    pub fn decode_mask(&self) -> Result<Option<Bitmap>, Error> {
        let page = self.doc.doc.page(self.index)?;
        page.decode_mask()
    }

    /// Render the page to an RGBA pixmap at native resolution.
    pub fn render(&self) -> Result<Pixmap, Error> {
        let page = self.doc.doc.page(self.index)?;
        render::render(&page)
    }

    /// Render the page to an RGBA pixmap at a target size.
    pub fn render_to_size(&self, width: u32, height: u32) -> Result<Pixmap, Error> {
        let page = self.doc.doc.page(self.index)?;
        render::render_to_size(&page, width, height)
    }

    /// Render the page at native resolution with mask dilation for bolder text.
    pub fn render_bold(&self, dilate_passes: u32) -> Result<Pixmap, Error> {
        let page = self.doc.doc.page(self.index)?;
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
        let page = self.doc.doc.page(self.index)?;
        render::render_to_size_bold(&page, width, height, dilate_passes)
    }

    /// Render the page at a target size with anti-aliased downscaling.
    pub fn render_aa(&self, width: u32, height: u32, boldness: f32) -> Result<Pixmap, Error> {
        let page = self.doc.doc.page(self.index)?;
        render::render_aa(&page, width, height, boldness)
    }

    /// Decode the page thumbnail, if available.
    pub fn thumbnail(&self) -> Result<Option<Pixmap>, Error> {
        self.doc.doc.thumbnail(self.index)
    }

    /// Extract the text layer (TXTz/TXTa) with zone hierarchy.
    pub fn text_layer(&self) -> Result<Option<TextLayer>, Error> {
        let page = self.doc.doc.page(self.index)?;
        page.text_layer()
    }

    /// Extract the plain text content of the page.
    pub fn text(&self) -> Result<Option<String>, Error> {
        Ok(self.text_layer()?.map(|tl| tl.text))
    }

    /// Fast coarse render: decode only the first BG44 chunk (blurry preview).
    pub fn render_scaled_coarse(&self, scale: f32) -> Result<Option<Pixmap>, Error> {
        let dw = self.display_width();
        let dh = self.display_height();
        let w = ((dw as f32 * scale).round() as u32).max(1);
        let h = ((dh as f32 * scale).round() as u32).max(1);
        let (tw, th) = match self.rotation {
            document::Rotation::Cw90 | document::Rotation::Cw270 => (h, w),
            _ => (w, h),
        };
        let page = self.doc.doc.page(self.index)?;
        render::render_to_size_coarse(&page, tw, th)
    }

    /// Progressive rendering: returns increasingly refined pixmaps.
    pub fn render_scaled_progressive(&self, scale: f32) -> Result<Vec<Pixmap>, Error> {
        let dw = self.display_width();
        let dh = self.display_height();
        let w = ((dw as f32 * scale).round() as u32).max(1);
        let h = ((dh as f32 * scale).round() as u32).max(1);
        let (tw, th) = match self.rotation {
            document::Rotation::Cw90 | document::Rotation::Cw270 => (h, w),
            _ => (w, h),
        };
        let page = self.doc.doc.page(self.index)?;
        render::render_to_size_progressive(&page, tw, th)
    }

    /// Render the page scaled by a factor (e.g. 0.5 = half size, 2.0 = double).
    pub fn render_scaled(&self, scale: f32) -> Result<Pixmap, Error> {
        let dw = self.display_width();
        let dh = self.display_height();
        let w = ((dw as f32 * scale).round() as u32).max(1);
        let h = ((dh as f32 * scale).round() as u32).max(1);
        let (tw, th) = match self.rotation {
            document::Rotation::Cw90 | document::Rotation::Cw270 => (h, w),
            _ => (w, h),
        };
        let page = self.doc.doc.page(self.index)?;
        render::render_to_size(&page, tw, th)
    }
}

// Compile-time assertions: Document is Send + Sync.
#[cfg(feature = "std")]
#[allow(dead_code)]
const _: () = {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    fn assertions() {
        assert_send::<Document>();
        assert_sync::<Document>();
    }
};
