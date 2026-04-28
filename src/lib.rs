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

// ---- Phase-1 modules -------------------------------------------------------
//
// Clean-room implementations written from the DjVu spec.

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
/// Provides `ZpDecoder` used by BZZ, JB2, and IW44 decoders.
#[path = "zp/mod.rs"]
pub(crate) mod zp_impl;

/// BZZ decompressor — clean-room implementation.
///
/// Provides `bzz_new::bzz_decode` for decompressing DjVu BZZ streams
/// (DIRM, NAVM, ANTz chunks).
#[allow(dead_code)]
pub mod bzz_new;

/// BZZ compressor — encoding counterpart to `bzz_new`.
#[cfg(feature = "std")]
pub mod bzz_encode;

/// DJVM document merge and split operations.
#[cfg(feature = "std")]
pub mod djvm;

/// JB2 bilevel image decoder — clean-room implementation.
///
/// Decodes JB2-encoded bitonal images from DjVu Sjbz and Djbz chunks using
/// ZP adaptive arithmetic coding with a symbol dictionary.
///
/// Key public types: `jb2::Jb2Dict`, `jb2::decode`, `jb2::decode_dict`.
pub mod jb2;

/// IW44 wavelet image decoder — clean-room implementation (phase 2c).
///
/// Provides `iw44_new::Iw44Image` for decoding BG44/FG44/TH44 chunks.
/// Uses planar YCbCr storage and a ZP arithmetic coder.
/// RGB conversion happens only in `iw44_new::Iw44Image::to_rgb`.
#[path = "iw44_new.rs"]
pub mod iw44_new;

/// IW44 wavelet encoder — produces BG44/FG44/TH44 chunk payloads.
///
/// Provides [`iw44_encode::encode_iw44_color`] and [`iw44_encode::encode_iw44_gray`].
#[cfg(feature = "std")]
pub mod iw44_encode;

/// JB2 bilevel image encoder — produces Sjbz chunk payloads.
///
/// Provides [`jb2_encode::encode_jb2`] (single record-type-3 direct encoding) and
/// [`jb2_encode::encode_jb2_dict`] (connected-component symbol-dictionary encoding).
#[cfg(feature = "std")]
pub mod jb2_encode;

/// FGbz foreground-palette encoder — produces FGbz chunk payloads.
///
/// Provides [`fgbz_encode::encode_fgbz`] (palette + optional per-blit
/// index table) and [`fgbz_encode::decode_fgbz`] (inverse), plus the
/// [`fgbz_encode::FgbzColor`] palette-entry type.
#[cfg(feature = "std")]
pub mod fgbz_encode;

/// High-level page encoder — composes the codec primitives into a
/// complete `FORM:DJVU` page.
///
/// Provides [`djvu_encode::PageEncoder`] (builder-style entry point),
/// [`djvu_encode::EncodeQuality`] (Lossless / Quality / Archival
/// profiles), and [`djvu_encode::EncodeError`].
#[cfg(feature = "std")]
pub mod djvu_encode;

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

/// Pluggable OCR backend trait and error types.
///
/// Provides [`ocr::OcrBackend`] — recognize text in rendered page images.
/// Backend implementations are gated behind feature flags.
#[cfg(feature = "std")]
pub mod ocr;

/// Tesseract OCR backend (requires `ocr-tesseract` feature).
#[cfg(feature = "ocr-tesseract")]
pub mod ocr_tesseract;

/// ONNX OCR backend via tract (requires `ocr-onnx` feature).
#[cfg(feature = "ocr-onnx")]
pub mod ocr_onnx;

/// Neural OCR backend via Candle (requires `ocr-neural` feature).
#[cfg(feature = "ocr-neural")]
pub mod ocr_neural;

/// TXTa/TXTz text layer encoder — writes [`text::TextLayer`] back to DjVu binary format.
#[cfg(feature = "std")]
pub mod text_encode;

/// NAVM bookmark encoder — serializes [`djvu_document::DjVuBookmark`] trees to BZZ-compressed binary.
#[cfg(feature = "std")]
pub mod navm_encode;

/// Smmr chunk codec — ITU-T G4 (MMR) bilevel image compression.
///
/// Provides [`smmr::decode_smmr`] (chunk → [`Bitmap`]) and
/// [`smmr::encode_smmr`] ([`Bitmap`] → chunk). Useful as an alternative
/// to JB2 for fax-style scans without recurring glyph structure.
pub mod smmr;

#[cfg(feature = "wasm")]
pub mod wasm;

/// C FFI bindings for foreign language integration.
///
/// Provides `extern "C"` functions with no-panic guarantees.
/// Key functions: `djvu_doc_open`, `djvu_doc_free`, `djvu_page_render`,
/// `djvu_pixmap_free`, `djvu_page_text`.
#[cfg(feature = "std")]
#[allow(unsafe_code)]
pub mod ffi;

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

#[doc(hidden)]
pub(crate) mod pixmap;

pub use bitmap::Bitmap;
pub use pixmap::{GrayPixmap, Pixmap};

// Re-export text types from the new pipeline
#[cfg(feature = "std")]
pub use text::{TextLayer, TextZone, TextZoneKind};

// Bookmark type alias — same shape as DjVuBookmark
#[cfg(feature = "std")]
pub type Bookmark = DjVuBookmark;

// Legacy error type (re-exported from legacy_error module included via error.rs)
#[cfg(feature = "std")]
pub use error::LegacyError as Error;

/// A parsed DjVu document. Owns the parsed structure.
///
/// Parsing happens once at construction time. All subsequent `page()` and
/// `render()` calls reuse the parsed chunk tree with zero re-parsing overhead.
#[cfg(feature = "std")]
pub struct Document {
    doc: DjVuDocument,
}

#[cfg(feature = "std")]
impl Document {
    /// Open a DjVu file from disk.
    ///
    /// For **indirect** multi-page DJVM files (where component pages live in
    /// separate files next to the index), use [`Document::open_dir`] instead.
    /// This method uses `DjVuDocument::parse` which only handles bundled
    /// (self-contained) files; it will return an error for indirect documents.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let data = std::fs::read(path.as_ref())
            .map_err(|e| Error::FormatError(format!("failed to read file: {}", e)))?;
        Self::from_bytes(data)
    }

    /// Open an indirect DJVM document from disk, resolving component pages
    /// from the same directory as the index file.
    ///
    /// Use this when the DjVu file is an *indirect* multi-page document where
    /// individual page files (e.g. `page001.djvu`) live alongside the index.
    /// For self-contained (bundled) files, [`Document::open`] is sufficient.
    pub fn open_dir(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let data = std::fs::read(path)
            .map_err(|e| Error::FormatError(format!("failed to read file: {}", e)))?;
        let base_dir = path.parent().unwrap_or(std::path::Path::new("."));
        let doc = DjVuDocument::parse_from_dir(&data, base_dir)
            .map_err(|e| Error::FormatError(e.to_string()))?;
        Ok(Document { doc })
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
        let doc = DjVuDocument::parse(&data).map_err(|e| Error::FormatError(e.to_string()))?;
        Ok(Document { doc })
    }

    /// Parse the NAVM bookmarks (table of contents).
    pub fn bookmarks(&self) -> Result<Vec<Bookmark>, Error> {
        Ok(self.doc.bookmarks().to_vec())
    }

    /// Number of pages.
    pub fn page_count(&self) -> usize {
        self.doc.page_count()
    }

    /// Access a page by 0-based index.
    pub fn page(&self, index: usize) -> Result<Page<'_>, Error> {
        let page = self
            .doc
            .page(index)
            .map_err(|e| Error::FormatError(e.to_string()))?;
        Ok(Page { page, index })
    }
}

/// A page within a DjVu document.
#[cfg(feature = "std")]
pub struct Page<'a> {
    page: &'a DjVuPage,
    index: usize,
}

#[cfg(feature = "std")]
impl<'a> Page<'a> {
    /// Page width in pixels (before rotation).
    pub fn width(&self) -> u32 {
        self.page.width() as u32
    }

    /// Page height in pixels (before rotation).
    pub fn height(&self) -> u32 {
        self.page.height() as u32
    }

    /// Effective page width after rotation.
    pub fn display_width(&self) -> u32 {
        self.display_dims().0
    }

    /// Effective page height after rotation.
    pub fn display_height(&self) -> u32 {
        self.display_dims().1
    }

    fn display_dims(&self) -> (u32, u32) {
        let w = self.page.width() as u32;
        let h = self.page.height() as u32;
        match self.page.rotation() {
            info::Rotation::Cw90 | info::Rotation::Ccw90 => (h, w),
            _ => (w, h),
        }
    }

    /// Page resolution in dots per inch.
    pub fn dpi(&self) -> u16 {
        self.page.dpi()
    }

    /// The 0-based index of this page within the document.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Page rotation from the INFO chunk.
    pub fn rotation(&self) -> info::Rotation {
        self.page.rotation()
    }

    fn render_err(e: djvu_render::RenderError) -> Error {
        Error::FormatError(e.to_string())
    }

    /// Decode the JB2/G4 mask layer only (no compositing).
    ///
    /// Returns `None` when the page has no mask chunk (pure IW44 background page).
    pub fn decode_mask(&self) -> Result<Option<Bitmap>, Error> {
        self.page
            .extract_mask()
            .map_err(|e| Error::FormatError(e.to_string()))
    }

    /// Render the page to an RGBA pixmap at native resolution.
    pub fn render(&self) -> Result<Pixmap, Error> {
        let (w, h) = self.display_dims();
        let opts = djvu_render::RenderOptions {
            width: w,
            height: h,
            scale: 1.0,
            ..Default::default()
        };
        djvu_render::render_pixmap(self.page, &opts).map_err(Self::render_err)
    }

    /// Render the page to an RGBA pixmap at a target size.
    pub fn render_to_size(&self, width: u32, height: u32) -> Result<Pixmap, Error> {
        let (dw, dh) = self.display_dims();
        let scale = if dw > 0 {
            width as f32 / dw as f32
        } else {
            1.0
        };
        let _ = dh;
        let opts = djvu_render::RenderOptions {
            width,
            height,
            scale,
            ..Default::default()
        };
        djvu_render::render_pixmap(self.page, &opts).map_err(Self::render_err)
    }

    /// Render the page at native resolution with mask dilation for bolder text.
    pub fn render_bold(&self, dilate_passes: u32) -> Result<Pixmap, Error> {
        let (w, h) = self.display_dims();
        let opts = djvu_render::RenderOptions {
            width: w,
            height: h,
            scale: 1.0,
            bold: dilate_passes.min(255) as u8,
            ..Default::default()
        };
        djvu_render::render_pixmap(self.page, &opts).map_err(Self::render_err)
    }

    /// Render the page to a target size with mask dilation for bolder text.
    pub fn render_to_size_bold(
        &self,
        width: u32,
        height: u32,
        dilate_passes: u32,
    ) -> Result<Pixmap, Error> {
        let (dw, _dh) = self.display_dims();
        let scale = if dw > 0 {
            width as f32 / dw as f32
        } else {
            1.0
        };
        let opts = djvu_render::RenderOptions {
            width,
            height,
            scale,
            bold: dilate_passes.min(255) as u8,
            ..Default::default()
        };
        djvu_render::render_pixmap(self.page, &opts).map_err(Self::render_err)
    }

    /// Render the page at a target size with anti-aliased downscaling.
    pub fn render_aa(&self, width: u32, height: u32, _boldness: f32) -> Result<Pixmap, Error> {
        let (dw, _dh) = self.display_dims();
        let scale = if dw > 0 {
            width as f32 / dw as f32
        } else {
            1.0
        };
        let opts = djvu_render::RenderOptions {
            width,
            height,
            scale,
            aa: true,
            ..Default::default()
        };
        djvu_render::render_pixmap(self.page, &opts).map_err(Self::render_err)
    }

    /// Decode the page thumbnail, if available.
    pub fn thumbnail(&self) -> Result<Option<Pixmap>, Error> {
        self.page
            .thumbnail()
            .map_err(|e| Error::FormatError(e.to_string()))
    }

    /// Extract the text layer (TXTz/TXTa) with zone hierarchy.
    pub fn text_layer(&self) -> Result<Option<TextLayer>, Error> {
        self.page
            .text_layer()
            .map_err(|e| Error::FormatError(e.to_string()))
    }

    /// Extract the plain text content of the page.
    pub fn text(&self) -> Result<Option<String>, Error> {
        Ok(self.text_layer()?.map(|tl| tl.text))
    }

    /// Fast coarse render: decode only the first BG44 chunk (blurry preview).
    pub fn render_scaled_coarse(&self, scale: f32) -> Result<Option<Pixmap>, Error> {
        let (dw, dh) = self.display_dims();
        let w = ((dw as f32 * scale).round() as u32).max(1);
        let h = ((dh as f32 * scale).round() as u32).max(1);
        let opts = djvu_render::RenderOptions {
            width: w,
            height: h,
            scale,
            ..Default::default()
        };
        djvu_render::render_coarse(self.page, &opts).map_err(Self::render_err)
    }

    /// Progressive rendering: returns increasingly refined pixmaps.
    pub fn render_scaled_progressive(&self, scale: f32) -> Result<Vec<Pixmap>, Error> {
        let (dw, dh) = self.display_dims();
        let w = ((dw as f32 * scale).round() as u32).max(1);
        let h = ((dh as f32 * scale).round() as u32).max(1);
        let opts = djvu_render::RenderOptions {
            width: w,
            height: h,
            scale,
            ..Default::default()
        };
        let n_bg44 = self.page.bg44_chunks().len();
        if n_bg44 == 0 {
            let pixmap = djvu_render::render_pixmap(self.page, &opts).map_err(Self::render_err)?;
            return Ok(vec![pixmap]);
        }
        let mut result = Vec::with_capacity(n_bg44);
        for chunk_n in 0..n_bg44 {
            let pixmap = djvu_render::render_progressive(self.page, &opts, chunk_n)
                .map_err(Self::render_err)?;
            result.push(pixmap);
        }
        Ok(result)
    }

    /// Render the page scaled by a factor (e.g. 0.5 = half size, 2.0 = double).
    pub fn render_scaled(&self, scale: f32) -> Result<Pixmap, Error> {
        let (dw, dh) = self.display_dims();
        let w = ((dw as f32 * scale).round() as u32).max(1);
        let h = ((dh as f32 * scale).round() as u32).max(1);
        let opts = djvu_render::RenderOptions {
            width: w,
            height: h,
            scale,
            ..Default::default()
        };
        djvu_render::render_pixmap(self.page, &opts).map_err(Self::render_err)
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
