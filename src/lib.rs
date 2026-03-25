//! Pure-Rust DjVu decoder written from the DjVu v3 public specification.
//!
//! This crate provides a zero-copy IFF container parser, typed error types,
//! and page metadata extraction. Higher-level decoding (JB2, IW44, BZZ) is
//! provided by phase 2+ modules. The legacy implementation is kept for
//! backward compatibility while the clean-room rewrite is in progress.
//!
//! # Key public types
//!
//! - [`DjVuError`] — top-level error enum (wraps [`IffError`], etc.)
//! - [`IffError`] — errors from the IFF container parser
//! - [`PageInfo`] — page metadata parsed from the INFO chunk
//! - [`Rotation`] — page rotation enum (None, Ccw90, Rot180, Cw90)
//! - [`DjVuDocument`] — new phase-3 document model (IFF/BZZ/IW44 based)
//! - [`DjVuPage`] — lazy page handle (new phase-3)
//! - [`DjVuBookmark`] — NAVM bookmark (new phase-3)
//! - [`DocError`] — error type for the new document model
//! - [`djvu_render::RenderOptions`] — render parameters (phase 5)
//! - [`djvu_render::RenderError`] — render pipeline error type (phase 5)
//! - [`text::TextLayer`] — text layer from TXTz/TXTa chunks (phase 4)
//! - [`text::TextZone`] — a zone node in the text layer hierarchy (phase 4)
//! - [`annotation::Annotation`] — page-level annotation (phase 4)
//! - [`annotation::MapArea`] — clickable area with URL and shape (phase 4)
//! - [`Pixmap`] — RGBA pixel buffer returned by render methods
//! - [`Bitmap`] — 1-bit bitmap for JB2 mask layers
//!
//! # Legacy API (std only)
//!
//! - `Document` — owned DjVu document (full rendering, from legacy impl)
//! - `Page` — a page within a `Document`
//! - `Bookmark` — table-of-contents entry from NAVM chunk (legacy)
//! - `TextLayer` — text layer extracted from TXTz/TXTa chunks (legacy)
//! - `TextZone`, `TextZoneKind` — zone types (legacy)
//! - `error::LegacyError` — the original error type (legacy)
//!
//! # IFF parser (phase 1)
//!
//! ```no_run
//! use cos_djvu::iff::parse_form;
//!
//! let data = std::fs::read("file.djvu").unwrap();
//! let form = parse_form(&data).unwrap();
//! println!("form type: {:?}", std::str::from_utf8(&form.form_type));
//! ```
//!
//! # Full document rendering (phase 5)
//!
//! ```no_run
//! use cos_djvu::{DjVuDocument, djvu_render::{render_pixmap, RenderOptions}};
//!
//! let data = std::fs::read("file.djvu").unwrap();
//! let doc = DjVuDocument::parse(&data).unwrap();
//! println!("{} pages", doc.page_count());
//!
//! let page = doc.page(0).unwrap();
//! println!("{}x{} @ {} dpi", page.width(), page.height(), page.dpi());
//!
//! let opts = RenderOptions { width: page.width() as u32, height: page.height() as u32, scale: 1.0, bold: 0, aa: false };
//! let pixmap = render_pixmap(page, &opts).unwrap();
//! // pixmap.data: RGBA bytes
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
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

/// BZZ decompressor — clean-room implementation (phase 2a).
///
/// Provides `bzz_new::bzz_decode` for decompressing DjVu BZZ streams
/// (DIRM, NAVM, ANTz chunks). Will replace the legacy bzz module in phase 2b.
#[path = "bzz.rs"]
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
/// Provides `djvu_render::RenderOptions`, `djvu_render::render_into`,
/// `djvu_render::render_pixmap`, `djvu_render::render_coarse`, and
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

// Re-export new phase-1 error types
pub use error::{BzzError, DjVuError, IffError, Iw44Error, Jb2Error};

// Re-export new phase-3 document model
pub use djvu_document::{DjVuBookmark, DjVuDocument, DjVuPage, DocError};

// Re-export new phase-1 page info types
pub use info::{PageInfo, Rotation};

// ---- Legacy implementation (kept for cos-render compatibility) --------------
//
// The legacy modules use `crate::legacy_error` and `crate::legacy_iff` as their
// renamed equivalents of the old `crate::error` and `crate::iff`.
// Internally they depend on: bitmap, pixmap, iw44, jb2, zp, document, bzz.
// We keep those at their original crate paths.
//
// NOTE: legacy/document.rs and legacy/iff.rs reference `crate::error::Error`
// and `crate::iff::*` — those `crate::` paths now resolve to the NEW modules.
// To avoid breakage we insert compatibility re-exports into the new modules.
// See error.rs (re-exports legacy_error::Error) and iff.rs has separate types.
//
// The legacy iff types (DjvuFile, etc.) are exposed under `crate::legacy_iff`.
// The legacy files are patched to reference their own module names via
// absolute paths — but since we cannot modify legacy/ files, we instead
// expose shims at the paths they expect.
//
// NOTE: Legacy modules require std (they use std::io, std::path, vec!/format!
// macros and self_cell). They are gated behind #[cfg(feature = "std")] for
// no_std compatibility. The new phase-1+ decoder modules (iff, bzz, jb2_new,
// iw44_new) are alloc-only and work without std.

#[doc(hidden)]
#[path = "legacy/bitmap.rs"]
pub(crate) mod bitmap;

#[cfg(feature = "std")]
#[doc(hidden)]
#[path = "legacy/bzz.rs"]
pub mod bzz;

#[cfg(feature = "std")]
#[doc(hidden)]
#[path = "legacy/document.rs"]
pub mod document;

#[cfg(feature = "std")]
#[doc(hidden)]
#[path = "legacy/iw44.rs"]
pub mod iw44;

#[cfg(feature = "std")]
#[doc(hidden)]
#[path = "legacy/jb2.rs"]
pub mod jb2;

#[doc(hidden)]
#[path = "legacy/pixmap.rs"]
pub(crate) mod pixmap;

#[cfg(feature = "std")]
#[doc(hidden)]
#[path = "legacy/render.rs"]
pub mod render;

#[cfg(feature = "std")]
#[doc(hidden)]
#[path = "legacy/zp/mod.rs"]
pub mod zp;

// Re-export types needed by both legacy and new phase modules
pub use bitmap::Bitmap;
pub use pixmap::Pixmap;

// Re-export legacy types (only with std feature)
#[cfg(feature = "std")]
pub use document::{Bookmark, TextLayer, TextZone, TextZoneKind};

// Legacy error type (re-exported from legacy_error module included via error.rs)
#[cfg(feature = "std")]
pub use error::LegacyError as Error;

#[cfg(feature = "std")]
use self_cell::self_cell;

#[cfg(feature = "std")]
type ParsedDocument<'a> = document::Document<'a>;

#[cfg(feature = "std")]
self_cell!(
    struct DocumentInner {
        owner: Box<[u8]>,

        #[covariant]
        dependent: ParsedDocument,
    }
);

/// A parsed DjVu document. Owns its data and the parsed structure.
#[cfg(feature = "std")]
///
/// Parsing happens once at construction time. All subsequent `page()` and
/// `render()` calls reuse the parsed chunk tree with zero re-parsing overhead.
pub struct Document {
    inner: DocumentInner,
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

    /// Render the page to an RGBA pixmap at native resolution.
    pub fn render(&self) -> Result<Pixmap, Error> {
        let page = self.doc.inner.borrow_dependent().page(self.index)?;
        render::render(&page)
    }

    /// Render the page to an RGBA pixmap at a target size.
    pub fn render_to_size(&self, width: u32, height: u32) -> Result<Pixmap, Error> {
        let page = self.doc.inner.borrow_dependent().page(self.index)?;
        render::render_to_size(&page, width, height)
    }

    /// Render the page at native resolution with mask dilation for bolder text.
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
    pub fn render_aa(&self, width: u32, height: u32, boldness: f32) -> Result<Pixmap, Error> {
        let page = self.doc.inner.borrow_dependent().page(self.index)?;
        render::render_aa(&page, width, height, boldness)
    }

    /// Decode the page thumbnail, if available.
    pub fn thumbnail(&self) -> Result<Option<Pixmap>, Error> {
        self.doc.inner.borrow_dependent().thumbnail(self.index)
    }

    /// Extract the text layer (TXTz/TXTa) with zone hierarchy.
    pub fn text_layer(&self) -> Result<Option<TextLayer>, Error> {
        let page = self.doc.inner.borrow_dependent().page(self.index)?;
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
        let page = self.doc.inner.borrow_dependent().page(self.index)?;
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
        let page = self.doc.inner.borrow_dependent().page(self.index)?;
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
        let page = self.doc.inner.borrow_dependent().page(self.index)?;
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
