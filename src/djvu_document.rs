//! New document model for DjVu files — phase 3.
//!
//! This module provides the high-level `DjVuDocument` API built on top of the
//! clean-room IFF parser (phase 1), BZZ decompressor (phase 2a), and IW44 decoder
//! (phase 2c).
//!
//! ## Key public types
//!
//! - [`DjVuDocument`] — opened DjVu document (single-page or multi-page)
//! - [`DjVuPage`] — lazy page handle (raw chunks stored until `thumbnail()` is called)
//! - [`DjVuBookmark`] — table-of-contents entry from the NAVM chunk
//! - [`DocError`] — typed errors for this module
//!
//! ## Document kinds
//!
//! - **FORM:DJVU** — single-page document
//! - **FORM:DJVM + DIRM** — bundled multi-page document with an in-file page index
//! - **FORM:DJVM + DIRM (indirect)** — pages live in separate files; a resolver
//!   callback `fn(name: &str) -> Result<Vec<u8>, DocError>` is required
//!
//! ## Lazy decoding contract
//!
//! `DjVuPage` stores only the raw chunk bytes. No image decoding happens until
//! the caller explicitly calls `thumbnail()` (which invokes the IW44 decoder).

#[cfg(not(feature = "std"))]
use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};

use crate::{
    annotation::{Annotation, AnnotationError, MapArea},
    bzz_new::bzz_decode,
    error::{BzzError, IffError, Iw44Error, Jb2Error},
    iff::{IffChunk, parse_form},
    info::PageInfo,
    iw44_new::Iw44Image,
    metadata::{DjVuMetadata, MetadataError},
    pixmap::Pixmap,
    text::{TextError, TextLayer},
};

// ---- Error type -------------------------------------------------------------

/// Errors that can occur when working with the DjVuDocument API.
#[derive(Debug, thiserror::Error)]
pub enum DocError {
    /// IFF container parse error.
    #[error("IFF error: {0}")]
    Iff(#[from] IffError),

    /// BZZ decompression error.
    #[error("BZZ error: {0}")]
    Bzz(#[from] BzzError),

    /// IW44 wavelet decoding error.
    #[error("IW44 error: {0}")]
    Iw44(#[from] Iw44Error),

    /// JB2 bilevel image decoding error.
    #[error("JB2 error: {0}")]
    Jb2(#[from] Jb2Error),

    /// The file is not a supported DjVu format.
    #[error("not a DjVu file: found form type {0:?}")]
    NotDjVu([u8; 4]),

    /// A required chunk is missing.
    #[error("missing required chunk: {0}")]
    MissingChunk(&'static str),

    /// The document is malformed (description included).
    #[error("malformed DjVu document: {0}")]
    Malformed(&'static str),

    /// An indirect page reference could not be resolved.
    #[error("failed to resolve indirect page '{0}'")]
    IndirectResolve(String),

    /// Page index is out of range.
    #[error("page index {index} is out of range (document has {count} pages)")]
    PageOutOfRange { index: usize, count: usize },

    /// Invalid UTF-8 in a string field.
    #[error("invalid UTF-8 in DjVu metadata")]
    InvalidUtf8,

    /// The resolver callback is required for indirect documents but was not provided.
    #[error("indirect DjVu document requires a resolver callback")]
    NoResolver,

    /// I/O error when reading file data (only with `std` feature).
    #[cfg(feature = "std")]
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Text layer parse error.
    #[error("text layer error: {0}")]
    Text(#[from] TextError),

    /// Annotation parse error.
    #[error("annotation error: {0}")]
    Annotation(#[from] AnnotationError),

    /// Metadata parse error.
    #[error("metadata error: {0}")]
    Metadata(#[from] MetadataError),
}

// ---- Bookmark ---------------------------------------------------------------

/// A table-of-contents entry from the NAVM chunk.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DjVuBookmark {
    /// Display title.
    pub title: String,
    /// Target URL (DjVu internal URL format).
    pub url: String,
    /// Nested child entries.
    pub children: Vec<DjVuBookmark>,
}

// ---- Page -------------------------------------------------------------------

/// Component type in the DIRM directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComponentType {
    Shared,
    Page,
    Thumbnail,
}

/// A raw chunk extracted from a page FORM:DJVU.
#[derive(Debug, Clone)]
struct RawChunk {
    id: [u8; 4],
    data: Vec<u8>,
}

/// A lazy DjVu page handle.
///
/// Raw chunk data is stored on construction. No image decoding is performed
/// until the caller invokes `thumbnail()` or a render function.
///
/// The fully decoded BG44 wavelet image is cached after the first render so
/// that subsequent renders skip the expensive ZP arithmetic decode and only
/// run the wavelet inverse-transform and compositor.
#[derive(Debug, Clone)]
pub struct DjVuPage {
    /// Page info parsed from the INFO chunk.
    info: PageInfo,
    /// All raw chunks from this page's FORM:DJVU, in order.
    chunks: Vec<RawChunk>,
    /// Page index within the document (0-based).
    index: usize,
    /// Raw Djbz data from the DJVI shared dictionary component referenced via
    /// the page's INCL chunk, if present.  Stored here so that `extract_mask`
    /// can decode it without access to the parent document.
    shared_djbz: Option<Vec<u8>>,
    /// Lazily decoded BG44 background wavelet image.  Populated on first use;
    /// subsequent renders call `to_rgb_subsample` directly on the cached image.
    /// Only available when the `std` feature is enabled (`OnceLock` requires std).
    #[cfg(feature = "std")]
    bg44_decoded: std::sync::OnceLock<Option<Iw44Image>>,
}

impl DjVuPage {
    /// Page width in pixels.
    pub fn width(&self) -> u16 {
        self.info.width
    }

    /// Page height in pixels.
    pub fn height(&self) -> u16 {
        self.info.height
    }

    /// Page resolution in dots per inch.
    pub fn dpi(&self) -> u16 {
        self.info.dpi
    }

    /// Display gamma from the INFO chunk.
    pub fn gamma(&self) -> f32 {
        self.info.gamma
    }

    /// Page rotation from the INFO chunk.
    pub fn rotation(&self) -> crate::info::Rotation {
        self.info.rotation
    }

    /// 0-based page index within the document.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Dimensions as `(width, height)`.
    pub fn dimensions(&self) -> (u16, u16) {
        (self.info.width, self.info.height)
    }

    /// Decode the thumbnail for this page from TH44 chunks, if present.
    ///
    /// No image data is decoded until this method is called (lazy contract).
    ///
    /// Returns `Ok(None)` if the page has no TH44 thumbnail.
    pub fn thumbnail(&self) -> Result<Option<Pixmap>, DocError> {
        let th44_chunks: Vec<&[u8]> = self
            .chunks
            .iter()
            .filter(|c| &c.id == b"TH44")
            .map(|c| c.data.as_slice())
            .collect();

        if th44_chunks.is_empty() {
            return Ok(None);
        }

        let mut img = Iw44Image::new();
        for chunk_data in &th44_chunks {
            img.decode_chunk(chunk_data)?;
        }
        let pixmap = img.to_rgb()?;
        Ok(Some(pixmap))
    }

    /// Return the raw bytes of the first chunk with the given 4-byte ID.
    ///
    /// Returns `None` if no chunk with that ID exists.  The returned slice
    /// points into the owned chunk storage — zero copy.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let sjbz = page.raw_chunk(b"Sjbz").expect("page must have a JB2 chunk");
    /// ```
    pub fn raw_chunk(&self, id: &[u8; 4]) -> Option<&[u8]> {
        self.chunks
            .iter()
            .find(|c| &c.id == id)
            .map(|c| c.data.as_slice())
    }

    /// Return the raw bytes of all chunks with the given 4-byte ID, in order.
    ///
    /// Returns an empty `Vec` if no such chunk exists.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let bg44_chunks = page.all_chunks(b"BG44");
    /// assert!(!bg44_chunks.is_empty(), "colour page must have BG44 data");
    /// ```
    pub fn all_chunks(&self, id: &[u8; 4]) -> Vec<&[u8]> {
        self.chunks
            .iter()
            .filter(|c| &c.id == id)
            .map(|c| c.data.as_slice())
            .collect()
    }

    /// Return the IDs of all chunks present on this page, in order.
    ///
    /// Duplicate IDs appear multiple times (once per chunk).
    pub fn chunk_ids(&self) -> Vec<[u8; 4]> {
        self.chunks.iter().map(|c| c.id).collect()
    }

    /// Find the first chunk with the given 4-byte ID.
    ///
    /// Equivalent to [`Self::raw_chunk`]; kept for internal use.
    pub fn find_chunk(&self, id: &[u8; 4]) -> Option<&[u8]> {
        self.raw_chunk(id)
    }

    /// Find all chunks with the given 4-byte ID.
    ///
    /// Equivalent to [`Self::all_chunks`]; kept for internal use.
    pub fn find_chunks(&self, id: &[u8; 4]) -> Vec<&[u8]> {
        self.all_chunks(id)
    }

    /// Return all BG44 background chunk data slices, in order.
    pub fn bg44_chunks(&self) -> Vec<&[u8]> {
        self.find_chunks(b"BG44")
    }

    /// Return the fully decoded BG44 wavelet image, decoding and caching on first call.
    ///
    /// Returns `None` if the page has no BG44 chunks.  On decode error the error
    /// is swallowed and `None` is returned (same semantics as the permissive render
    /// path), so this method is infallible.
    ///
    /// The result is computed once (all ZP arithmetic decode + block assembly) and
    /// then cached inside the page.  Subsequent calls return the cached value
    /// immediately.  The wavelet inverse-transform and YCbCr→RGB conversion are
    /// **not** cached; they are applied at each render at the appropriate subsample
    /// level via [`Iw44Image::to_rgb_subsample`].
    #[cfg(feature = "std")]
    pub fn decoded_bg44(&self) -> Option<&Iw44Image> {
        self.bg44_decoded
            .get_or_init(|| {
                let chunks = self.bg44_chunks();
                if chunks.is_empty() {
                    return None;
                }
                let mut img = Iw44Image::new();
                for chunk_data in &chunks {
                    if img.decode_chunk(chunk_data).is_err() {
                        break;
                    }
                }
                if img.width == 0 { None } else { Some(img) }
            })
            .as_ref()
    }

    #[cfg(not(feature = "std"))]
    pub fn decoded_bg44(&self) -> Option<&Iw44Image> {
        None
    }

    /// Return all FG44 foreground chunk data slices, in order.
    pub fn fg44_chunks(&self) -> Vec<&[u8]> {
        self.find_chunks(b"FG44")
    }

    /// Extract the text layer from TXTz (BZZ-compressed) or TXTa (plain) chunks.
    ///
    /// Returns `Ok(None)` if the page has no text layer.
    pub fn text_layer(&self) -> Result<Option<TextLayer>, DocError> {
        let page_height = self.info.height as u32;

        if let Some(txtz) = self.find_chunk(b"TXTz") {
            if txtz.is_empty() {
                return Ok(None);
            }
            let layer = crate::text::parse_text_layer_bzz(txtz, page_height)?;
            return Ok(Some(layer));
        }

        if let Some(txta) = self.find_chunk(b"TXTa") {
            if txta.is_empty() {
                return Ok(None);
            }
            let layer = crate::text::parse_text_layer(txta, page_height)?;
            return Ok(Some(layer));
        }

        Ok(None)
    }

    /// Parse the text layer and transform all zone rectangles to match a
    /// rendered page of size `render_w × render_h`.
    ///
    /// This is a convenience wrapper around [`Self::text_layer`] followed by
    /// [`TextLayer::transform`].  It applies the page's own rotation (from the
    /// INFO chunk) and scales coordinates proportionally to the requested
    /// render size, so callers can use the returned rects directly for text
    /// selection / copy-paste overlays without any additional maths.
    ///
    /// Returns `Ok(None)` if the page has no text layer.
    pub fn text_layer_at_size(
        &self,
        render_w: u32,
        render_h: u32,
    ) -> Result<Option<TextLayer>, DocError> {
        let page_w = self.info.width as u32;
        let page_h = self.info.height as u32;
        let rotation = self.info.rotation;
        Ok(self
            .text_layer()?
            .map(|tl| tl.transform(page_w, page_h, rotation, render_w, render_h)))
    }

    /// Extract the plain text content of the page (convenience wrapper).
    ///
    /// Returns `Ok(None)` if the page has no text layer.
    pub fn text(&self) -> Result<Option<String>, DocError> {
        Ok(self.text_layer()?.map(|tl| tl.text))
    }

    /// Parse the annotation layer from ANTz (BZZ-compressed) or ANTa (plain) chunks.
    ///
    /// Returns `Ok(None)` if the page has no annotation chunk.
    pub fn annotations(&self) -> Result<Option<(Annotation, Vec<MapArea>)>, DocError> {
        if let Some(antz) = self.find_chunk(b"ANTz") {
            if antz.is_empty() {
                return Ok(None);
            }
            let result = crate::annotation::parse_annotations_bzz(antz)?;
            return Ok(Some(result));
        }

        if let Some(anta) = self.find_chunk(b"ANTa") {
            if anta.is_empty() {
                return Ok(None);
            }
            let result = crate::annotation::parse_annotations(anta)?;
            return Ok(Some(result));
        }

        Ok(None)
    }

    /// Return all hyperlinks (MapAreas with a non-empty URL) on this page.
    pub fn hyperlinks(&self) -> Result<Vec<MapArea>, DocError> {
        match self.annotations()? {
            None => Ok(Vec::new()),
            Some((_, mapareas)) => Ok(mapareas.into_iter().filter(|m| !m.url.is_empty()).collect()),
        }
    }

    /// Decode the JB2 foreground mask as a 1-bit [`Bitmap`](crate::bitmap::Bitmap).
    ///
    /// Returns `Ok(None)` if the page has no Sjbz (JB2 mask) chunk.
    pub fn extract_mask(&self) -> Result<Option<crate::bitmap::Bitmap>, DocError> {
        let sjbz = match self.find_chunk(b"Sjbz") {
            Some(data) => data,
            None => return Ok(None),
        };

        // Prefer an inline Djbz chunk; fall back to the shared DJVI dictionary
        // that was resolved from the INCL chunk during document parse.
        let dict = if let Some(djbz) = self.find_chunk(b"Djbz") {
            Some(crate::jb2_new::decode_dict(djbz, None)?)
        } else if let Some(djbz) = self.shared_djbz.as_deref() {
            Some(crate::jb2_new::decode_dict(djbz, None)?)
        } else {
            None
        };

        let bm = crate::jb2_new::decode(sjbz, dict.as_ref())?;
        Ok(Some(bm))
    }

    /// Decode the IW44 foreground layer (FG44 chunks) if present.
    ///
    /// Returns `Ok(None)` if the page has no FG44 chunks.
    pub fn extract_foreground(&self) -> Result<Option<Pixmap>, DocError> {
        let chunks = self.fg44_chunks();
        if chunks.is_empty() {
            return Ok(None);
        }

        let mut img = Iw44Image::new();
        for chunk_data in &chunks {
            img.decode_chunk(chunk_data)?;
        }
        let pixmap = img.to_rgb()?;
        Ok(Some(pixmap))
    }

    /// Decode the IW44 background layer (BG44 chunks) if present.
    ///
    /// Returns `Ok(None)` if the page has no BG44 chunks.
    pub fn extract_background(&self) -> Result<Option<Pixmap>, DocError> {
        let chunks = self.bg44_chunks();
        if chunks.is_empty() {
            return Ok(None);
        }

        let mut img = Iw44Image::new();
        for chunk_data in &chunks {
            img.decode_chunk(chunk_data)?;
        }
        let pixmap = img.to_rgb()?;
        Ok(Some(pixmap))
    }

    /// Render this page into a pre-allocated RGBA buffer using the given options.
    ///
    /// This is the zero-allocation render path: no heap allocation occurs when
    /// `buf` is already sized to `opts.width * opts.height * 4` bytes.
    ///
    /// # Errors
    ///
    /// - [`crate::djvu_render::RenderError::BufTooSmall`] if buffer is too small
    /// - [`crate::djvu_render::RenderError::InvalidDimensions`] if width/height is 0
    /// - Propagates IW44 / JB2 decode errors
    pub fn render_into(
        &self,
        opts: &crate::djvu_render::RenderOptions,
        buf: &mut [u8],
    ) -> Result<(), crate::djvu_render::RenderError> {
        crate::djvu_render::render_into(self, opts, buf)
    }
}

// ---- Document ---------------------------------------------------------------

/// An opened DjVu document.
///
/// Supports single-page FORM:DJVU, bundled multi-page FORM:DJVM, and indirect
/// multi-page FORM:DJVM (via resolver callback).
#[derive(Debug)]
pub struct DjVuDocument {
    /// All pages, indexed by 0-based page number.
    pages: Vec<DjVuPage>,
    /// Parsed NAVM bookmarks, or empty if none.
    bookmarks: Vec<DjVuBookmark>,
    /// Raw document-level chunks (NAVM, DIRM, etc.) from the DJVM container,
    /// or from the top-level DJVU form for single-page documents.
    global_chunks: Vec<RawChunk>,
}

impl DjVuDocument {
    /// Parse a DjVu document from a byte slice.
    ///
    /// For indirect documents (INCL references to external files), a resolver
    /// must be supplied via [`DjVuDocument::parse_with_resolver`].
    ///
    /// # Errors
    ///
    /// Returns `DocError::NoResolver` if the document is indirect and no resolver
    /// was provided.
    pub fn parse(data: &[u8]) -> Result<Self, DocError> {
        Self::parse_with_resolver(data, None::<fn(&str) -> Result<Vec<u8>, DocError>>)
    }

    /// Parse a DjVu document with an optional resolver for indirect pages.
    ///
    /// The resolver receives the `name` field from each INCL chunk and must
    /// return the raw bytes of that external component file.
    pub fn parse_with_resolver<R>(data: &[u8], resolver: Option<R>) -> Result<Self, DocError>
    where
        R: Fn(&str) -> Result<Vec<u8>, DocError>,
    {
        let form = parse_form(data)?;

        match &form.form_type {
            b"DJVU" => {
                // Single-page document — expose all top-level chunks as global
                let global_chunks: Vec<RawChunk> = form
                    .chunks
                    .iter()
                    .map(|c| RawChunk {
                        id: c.id,
                        data: c.data.to_vec(),
                    })
                    .collect();
                let page = parse_page_from_chunks(&form.chunks, 0, None)?;
                Ok(DjVuDocument {
                    pages: vec![page],
                    bookmarks: vec![],
                    global_chunks,
                })
            }
            b"DJVM" => {
                // Multi-page document — parse DIRM first
                let dirm_chunk = form
                    .chunks
                    .iter()
                    .find(|c| &c.id == b"DIRM")
                    .ok_or(DocError::MissingChunk("DIRM"))?;

                let (entries, is_bundled) = parse_dirm(dirm_chunk.data)?;

                // Collect NAVM bookmarks (BZZ-compressed)
                let bookmarks = parse_navm_bookmarks(&form.chunks)?;

                // Store non-FORM global chunks (DIRM, NAVM, etc.)
                let global_chunks: Vec<RawChunk> = form
                    .chunks
                    .iter()
                    .filter(|c| &c.id != b"FORM")
                    .map(|c| RawChunk {
                        id: c.id,
                        data: c.data.to_vec(),
                    })
                    .collect();

                if is_bundled {
                    // Bundled: FORM:DJVU / FORM:DJVI sub-forms follow DIRM in sequence.
                    let sub_forms: Vec<&IffChunk<'_>> =
                        form.chunks.iter().filter(|c| &c.id == b"FORM").collect();

                    // Build a map of DJVI component ID → raw Djbz bytes for
                    // shared symbol dictionaries (referenced via INCL chunks).
                    // Use BTreeMap so this compiles in no_std (alloc::collections::BTreeMap
                    // is available; std::collections::HashMap is not).
                    #[cfg(not(feature = "std"))]
                    use alloc::collections::BTreeMap;
                    #[cfg(feature = "std")]
                    use std::collections::BTreeMap;
                    let djvi_djbz: BTreeMap<String, Vec<u8>> = entries
                        .iter()
                        .enumerate()
                        .filter(|(_, e)| e.comp_type == ComponentType::Shared)
                        .filter_map(|(comp_idx, entry)| {
                            let sf = sub_forms.get(comp_idx)?;
                            let chunks = parse_sub_form(sf.data).ok()?;
                            let djbz = chunks.iter().find(|c| &c.id == b"Djbz")?;
                            Some((entry.id.clone(), djbz.data.to_vec()))
                        })
                        .collect();

                    let mut pages = Vec::new();
                    let mut page_idx = 0usize;
                    for (comp_idx, entry) in entries.iter().enumerate() {
                        if entry.comp_type != ComponentType::Page {
                            continue;
                        }
                        let sub_form = sub_forms.get(comp_idx).ok_or(DocError::Malformed(
                            "DIRM entry count exceeds FORM children",
                        ))?;
                        let sub_chunks = parse_sub_form(sub_form.data)?;

                        // Resolve INCL reference to a shared DJVI dictionary.
                        let shared_djbz = sub_chunks
                            .iter()
                            .find(|c| &c.id == b"INCL")
                            .and_then(|incl| core::str::from_utf8(incl.data.trim_ascii_end()).ok())
                            .and_then(|name| djvi_djbz.get(name))
                            .cloned();

                        let page = parse_page_from_chunks(&sub_chunks, page_idx, shared_djbz)?;
                        pages.push(page);
                        page_idx += 1;
                    }

                    Ok(DjVuDocument {
                        pages,
                        bookmarks,
                        global_chunks,
                    })
                } else {
                    // Indirect: pages must be resolved by name
                    let resolver = resolver.ok_or(DocError::NoResolver)?;

                    let mut pages = Vec::new();
                    let mut page_idx = 0usize;
                    for entry in &entries {
                        if entry.comp_type != ComponentType::Page {
                            continue;
                        }
                        let resolved_data = resolver(&entry.id)
                            .map_err(|_| DocError::IndirectResolve(entry.id.clone()))?;
                        let sub_form = parse_form(&resolved_data)?;
                        let page = parse_page_from_chunks(&sub_form.chunks, page_idx, None)?;
                        pages.push(page);
                        page_idx += 1;
                    }

                    Ok(DjVuDocument {
                        pages,
                        bookmarks,
                        global_chunks,
                    })
                }
            }
            other => Err(DocError::NotDjVu(*other)),
        }
    }

    /// Number of pages.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Access a page by 0-based index.
    ///
    /// # Errors
    ///
    /// Returns `DocError::PageOutOfRange` if `index >= page_count()`.
    pub fn page(&self, index: usize) -> Result<&DjVuPage, DocError> {
        self.pages.get(index).ok_or(DocError::PageOutOfRange {
            index,
            count: self.pages.len(),
        })
    }

    /// The NAVM table of contents, or an empty slice if not present.
    pub fn bookmarks(&self) -> &[DjVuBookmark] {
        &self.bookmarks
    }

    /// Parse document-level metadata from a METz (BZZ-compressed) or METa
    /// (plain text) chunk.
    ///
    /// Returns `Ok(None)` if no METa/METz chunk is present.
    pub fn metadata(&self) -> Result<Option<DjVuMetadata>, DocError> {
        if let Some(metz) = self.raw_chunk(b"METz") {
            if metz.is_empty() {
                return Ok(None);
            }
            return Ok(Some(crate::metadata::parse_metadata_bzz(metz)?));
        }
        if let Some(meta) = self.raw_chunk(b"METa") {
            if meta.is_empty() {
                return Ok(None);
            }
            return Ok(Some(crate::metadata::parse_metadata(meta)?));
        }
        Ok(None)
    }

    /// Return the raw bytes of the first document-level chunk with the given
    /// 4-byte ID.
    ///
    /// For single-page DJVU files this covers all top-level chunks (INFO,
    /// Sjbz, BG44, …).  For multi-page DJVM files this covers non-page chunks
    /// such as DIRM and NAVM.  Per-page chunks are accessed via
    /// [`DjVuPage::raw_chunk`].
    ///
    /// Returns `None` if no such chunk exists.
    pub fn raw_chunk(&self, id: &[u8; 4]) -> Option<&[u8]> {
        self.global_chunks
            .iter()
            .find(|c| &c.id == id)
            .map(|c| c.data.as_slice())
    }

    /// Return the raw bytes of all document-level chunks with the given ID.
    ///
    /// Returns an empty `Vec` if no such chunk exists.
    pub fn all_chunks(&self, id: &[u8; 4]) -> Vec<&[u8]> {
        self.global_chunks
            .iter()
            .filter(|c| &c.id == id)
            .map(|c| c.data.as_slice())
            .collect()
    }

    /// Return the IDs of all document-level chunks, in order.
    ///
    /// For multi-page DJVM files this is the sequence of non-page chunks
    /// (DIRM, NAVM, …).  Duplicate IDs appear once per chunk.
    pub fn chunk_ids(&self) -> Vec<[u8; 4]> {
        self.global_chunks.iter().map(|c| c.id).collect()
    }
}

// ---- Memory-mapped document -------------------------------------------------

/// A DjVu document backed by a memory-mapped file.
///
/// Instead of copying the entire file into a `Vec<u8>`, this type maps the file
/// into the process address space using the OS virtual-memory subsystem.  The
/// kernel pages data from disk on demand, which can significantly reduce peak
/// memory usage for large multi-volume scans (100+ MB).
///
/// # Safety contract
///
/// **The underlying file must not be modified or truncated while the mapping is
/// alive.**  Mutating a memory-mapped file is undefined behaviour on most
/// platforms (SIGBUS on Linux/macOS, access violation on Windows).  The caller
/// is responsible for ensuring file immutability for the lifetime of this
/// struct.
///
/// Requires the `mmap` feature flag.
#[cfg(feature = "mmap")]
pub struct MmapDocument {
    /// The memory mapping — kept alive so the parsed document's borrowed data
    /// (pages, chunks) remain valid.  In practice `DjVuDocument` owns `Vec`
    /// copies of all chunk data, so the mmap is only needed during `parse`.
    _mmap: memmap2::Mmap,
    doc: DjVuDocument,
}

#[cfg(feature = "mmap")]
impl MmapDocument {
    /// Open a DjVu file via memory-mapped I/O.
    ///
    /// # Safety contract
    ///
    /// The file at `path` **must not be modified or truncated** while the
    /// returned `MmapDocument` is alive.  See the struct-level documentation
    /// for details.
    ///
    /// # Errors
    ///
    /// Returns `DocError::Io` if the file cannot be opened or mapped, or any
    /// parse error from [`DjVuDocument::parse`].
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, DocError> {
        let file = std::fs::File::open(path.as_ref())?;

        // SAFETY: The caller guarantees the file is not modified while mapped.
        // memmap2::Mmap provides a &[u8] view of the file contents.
        #[allow(unsafe_code)]
        let mmap = unsafe { memmap2::Mmap::map(&file) }?;

        let doc = DjVuDocument::parse(&mmap)?;
        Ok(MmapDocument { _mmap: mmap, doc })
    }

    /// Access the parsed [`DjVuDocument`].
    pub fn document(&self) -> &DjVuDocument {
        &self.doc
    }

    /// Number of pages in the document.
    pub fn page_count(&self) -> usize {
        self.doc.page_count()
    }

    /// Access a page by 0-based index.
    pub fn page(&self, index: usize) -> Result<&DjVuPage, DocError> {
        self.doc.page(index)
    }
}

#[cfg(feature = "mmap")]
impl core::ops::Deref for MmapDocument {
    type Target = DjVuDocument;
    fn deref(&self) -> &DjVuDocument {
        &self.doc
    }
}

// ---- Internal parsing helpers -----------------------------------------------

/// Parse a `DjVuPage` from the chunks of a FORM:DJVU.
///
/// `shared_djbz` is the raw `Djbz` data from a referenced DJVI component
/// (resolved from the page's INCL chunk by the caller); pass `None` if no
/// shared dictionary is available.
fn parse_page_from_chunks(
    chunks: &[IffChunk<'_>],
    index: usize,
    shared_djbz: Option<Vec<u8>>,
) -> Result<DjVuPage, DocError> {
    let info_chunk = chunks
        .iter()
        .find(|c| &c.id == b"INFO")
        .ok_or(DocError::MissingChunk("INFO"))?;

    let info = PageInfo::parse(info_chunk.data)?;

    // Copy all chunks to owned storage for lazy decode later.
    let raw_chunks: Vec<RawChunk> = chunks
        .iter()
        .map(|c| RawChunk {
            id: c.id,
            data: c.data.to_vec(),
        })
        .collect();

    Ok(DjVuPage {
        info,
        chunks: raw_chunks,
        index,
        shared_djbz,
        #[cfg(feature = "std")]
        bg44_decoded: std::sync::OnceLock::new(),
    })
}

/// Parse sub-form chunks from the data portion of a FORM chunk.
///
/// The `data` bytes start with a 4-byte form type (e.g. `DJVU`), followed by
/// sequential IFF chunks.
fn parse_sub_form(data: &[u8]) -> Result<Vec<IffChunk<'_>>, DocError> {
    if data.len() < 4 {
        return Err(DocError::Malformed("sub-form data too short"));
    }
    // data[0..4] = form type (DJVU / DJVI / THUM …)
    // data[4..] = sequential chunks
    let body = data
        .get(4..)
        .ok_or(DocError::Malformed("sub-form body missing"))?;
    let chunks = parse_iff_body_chunks(body)?;
    Ok(chunks)
}

/// Parse sequential IFF chunks from a raw byte slice (no AT&T / FORM wrapper).
fn parse_iff_body_chunks(mut buf: &[u8]) -> Result<Vec<IffChunk<'_>>, DocError> {
    let mut chunks = Vec::new();

    while buf.len() >= 8 {
        let id: [u8; 4] = buf
            .get(0..4)
            .and_then(|s| s.try_into().ok())
            .ok_or(IffError::Truncated)?;
        let data_len = buf
            .get(4..8)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_be_bytes)
            .map(|n| n as usize)
            .ok_or(IffError::Truncated)?;

        let data_start = 8usize;
        let data_end = data_start
            .checked_add(data_len)
            .ok_or(IffError::Truncated)?;

        if data_end > buf.len() {
            return Err(DocError::Iff(IffError::ChunkTooLong {
                id,
                claimed: data_len as u32,
                available: buf.len().saturating_sub(data_start),
            }));
        }

        let chunk_data = buf.get(data_start..data_end).ok_or(IffError::Truncated)?;

        // If this is a nested FORM, expose it as a FORM chunk with raw data
        // (form_type + children) so callers can handle FORM:DJVU sub-forms.
        chunks.push(IffChunk {
            id,
            data: chunk_data,
        });

        let padded_len = data_len + (data_len & 1);
        let next = data_start
            .checked_add(padded_len)
            .ok_or(IffError::Truncated)?;
        buf = buf.get(next.min(buf.len())..).ok_or(IffError::Truncated)?;
    }

    Ok(chunks)
}

/// A DIRM component entry.
#[derive(Debug, Clone)]
struct DirmEntry {
    comp_type: ComponentType,
    id: String,
}

/// Parse the DIRM chunk (directory of files in FORM:DJVM).
///
/// Returns `(entries, is_bundled)`.
fn parse_dirm(data: &[u8]) -> Result<(Vec<DirmEntry>, bool), DocError> {
    if data.len() < 3 {
        return Err(DocError::Malformed("DIRM chunk too short"));
    }

    let dflags = *data.first().ok_or(DocError::Malformed("DIRM empty"))?;
    let is_bundled = (dflags >> 7) != 0;
    let nfiles = u16::from_be_bytes([
        *data.get(1).ok_or(DocError::Malformed("DIRM too short"))?,
        *data.get(2).ok_or(DocError::Malformed("DIRM too short"))?,
    ]) as usize;

    let mut pos = 3usize;

    // Bundled documents embed 4-byte offsets (skipped; we rely on in-order FORM children).
    if is_bundled {
        let offsets_size = nfiles * 4;
        pos = pos
            .checked_add(offsets_size)
            .ok_or(DocError::Malformed("DIRM offset arithmetic overflow"))?;
        if pos > data.len() {
            return Err(DocError::Malformed("DIRM offset table truncated"));
        }
    }

    // Remaining bytes are BZZ-compressed metadata.
    let bzz_data = data
        .get(pos..)
        .ok_or(DocError::Malformed("DIRM bzz data missing"))?;
    let meta = bzz_decode(bzz_data)?;

    // Layout: sizes(3 bytes × N), flags(1 byte × N), then null-terminated IDs…
    let mut mpos = nfiles * 3; // skip per-component sizes

    if mpos + nfiles > meta.len() {
        return Err(DocError::Malformed("DIRM meta too short for flags"));
    }
    let flags: Vec<u8> = meta
        .get(mpos..mpos + nfiles)
        .ok_or(DocError::Malformed("DIRM flags truncated"))?
        .to_vec();
    mpos += nfiles;

    let mut entries = Vec::with_capacity(nfiles);
    for &flag in flags.iter().take(nfiles) {
        let id = read_str_nt(&meta, &mut mpos)?;

        // Optional name and title fields
        if (flag & 0x80) != 0 {
            let _ = read_str_nt(&meta, &mut mpos)?;
        }
        if (flag & 0x40) != 0 {
            let _ = read_str_nt(&meta, &mut mpos)?;
        }

        let comp_type = match flag & 0x3f {
            1 => ComponentType::Page,
            2 => ComponentType::Thumbnail,
            _ => ComponentType::Shared,
        };

        entries.push(DirmEntry { comp_type, id });
    }

    Ok((entries, is_bundled))
}

/// Read a null-terminated UTF-8 string from `data` at `*pos`, advancing `*pos`.
fn read_str_nt(data: &[u8], pos: &mut usize) -> Result<String, DocError> {
    let start = *pos;
    while *pos < data.len() && *data.get(*pos).ok_or(DocError::Malformed("str read OOB"))? != 0 {
        *pos += 1;
    }
    if *pos >= data.len() {
        return Err(DocError::Malformed(
            "null terminator missing in DIRM string",
        ));
    }
    let s = core::str::from_utf8(
        data.get(start..*pos)
            .ok_or(DocError::Malformed("str slice OOB"))?,
    )
    .map_err(|_| DocError::InvalidUtf8)?
    .to_string();
    *pos += 1; // consume null terminator
    Ok(s)
}

/// Parse NAVM bookmarks from the chunk list of a FORM:DJVM.
///
/// Returns an empty Vec if there is no NAVM chunk.
fn parse_navm_bookmarks(chunks: &[IffChunk<'_>]) -> Result<Vec<DjVuBookmark>, DocError> {
    let navm_data = match chunks.iter().find(|c| &c.id == b"NAVM") {
        Some(c) => c.data,
        None => return Ok(vec![]),
    };

    let decoded = bzz_decode(navm_data)?;

    if decoded.len() < 2 {
        return Ok(vec![]);
    }

    let b0 = *decoded
        .first()
        .ok_or(DocError::Malformed("NAVM total count byte 0"))?;
    let b1 = *decoded
        .get(1)
        .ok_or(DocError::Malformed("NAVM total count byte 1"))?;
    let total_count = u16::from_be_bytes([b0, b1]) as usize;

    let mut pos = 2usize;
    let mut bookmarks = Vec::new();
    let mut decoded_count = 0usize;

    while decoded_count < total_count {
        let bm = parse_bookmark_entry(&decoded, &mut pos, &mut decoded_count)?;
        bookmarks.push(bm);
    }

    Ok(bookmarks)
}

/// Recursively parse one bookmark entry and its children.
///
/// `total_counter` is a shared counter for ALL bookmark nodes across all recursion
/// levels, matching the DjVu NAVM format's flat total-count field.
fn parse_bookmark_entry(
    data: &[u8],
    pos: &mut usize,
    total_counter: &mut usize,
) -> Result<DjVuBookmark, DocError> {
    if *pos >= data.len() {
        return Err(DocError::Malformed("NAVM bookmark entry truncated"));
    }

    // n_children is a single byte in the NAVM format
    let n_children = *data
        .get(*pos)
        .ok_or(DocError::Malformed("NAVM children count"))? as usize;
    *pos += 1;

    let title = read_navm_str(data, pos)?;
    let url = read_navm_str(data, pos)?;
    *total_counter += 1;

    // Children: fixed count, recurse with the same global total_counter
    let mut children = Vec::with_capacity(n_children);
    for _ in 0..n_children {
        let child = parse_bookmark_entry(data, pos, total_counter)?;
        children.push(child);
    }

    Ok(DjVuBookmark {
        title,
        url,
        children,
    })
}

/// Read a length-prefixed UTF-8 string from NAVM data.
///
/// Format: `[be_u24 length][utf8 bytes]`
fn read_navm_str(data: &[u8], pos: &mut usize) -> Result<String, DocError> {
    if *pos + 3 > data.len() {
        return Err(DocError::Malformed("NAVM string length truncated"));
    }
    let len = ((*data.get(*pos).ok_or(DocError::Malformed("NAVM str"))? as usize) << 16)
        | ((*data.get(*pos + 1).ok_or(DocError::Malformed("NAVM str"))? as usize) << 8)
        | (*data.get(*pos + 2).ok_or(DocError::Malformed("NAVM str"))? as usize);
    *pos += 3;

    let bytes = data
        .get(*pos..*pos + len)
        .ok_or(DocError::Malformed("NAVM string bytes truncated"))?;
    *pos += len;

    core::str::from_utf8(bytes)
        .map(|s| s.to_string())
        .map_err(|_| DocError::InvalidUtf8)
}

// ---- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn assets_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/djvujs/library/assets")
    }

    // ---- TDD: failing tests written first (Red phase) -----------------------

    /// Single-page FORM:DJVU — basic parse, page count, dimensions, DPI.
    #[test]
    fn single_page_parse_and_metadata() {
        let data =
            std::fs::read(assets_path().join("chicken.djvu")).expect("chicken.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("parse should succeed");

        assert_eq!(doc.page_count(), 1);
        let page = doc.page(0).expect("page 0 must exist");
        assert_eq!(page.width(), 181);
        assert_eq!(page.height(), 240);
        assert_eq!(page.dpi(), 100);
        assert!((page.gamma() - 2.2).abs() < 0.01, "gamma should be ~2.2");
    }

    /// Single-page document: page index out of range.
    #[test]
    fn single_page_out_of_range() {
        let data =
            std::fs::read(assets_path().join("chicken.djvu")).expect("chicken.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("parse should succeed");
        let err = doc.page(1).expect_err("page 1 should be out of range");
        assert!(
            matches!(err, DocError::PageOutOfRange { index: 1, count: 1 }),
            "unexpected error: {err:?}"
        );
    }

    /// Single-page document: no thumbnails expected.
    #[test]
    fn single_page_no_thumbnail() {
        let data =
            std::fs::read(assets_path().join("chicken.djvu")).expect("chicken.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("parse should succeed");
        let page = doc.page(0).expect("page 0 must exist");
        // Data is not decoded until thumbnail() is called — verify lazy contract
        let thumb = page.thumbnail().expect("thumbnail() should not error");
        assert!(
            thumb.is_none(),
            "single-page chicken.djvu has no TH44 chunks"
        );
    }

    /// Single-page: dimensions helper.
    #[test]
    fn single_page_dimensions() {
        let data =
            std::fs::read(assets_path().join("chicken.djvu")).expect("chicken.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("parse should succeed");
        let page = doc.page(0).unwrap();
        assert_eq!(page.dimensions(), (181, 240));
    }

    /// Bundled multi-page FORM:DJVM — page count and DIRM parsing.
    #[test]
    fn multipage_bundled_page_count() {
        let data = std::fs::read(assets_path().join("DjVu3Spec_bundled.djvu"))
            .expect("DjVu3Spec_bundled.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("bundled parse should succeed");
        // The bundled spec PDF has many pages — just check > 1
        assert!(
            doc.page_count() > 1,
            "bundled document should have more than 1 page, got {}",
            doc.page_count()
        );
    }

    /// Bundled multi-page: each page should have valid metadata.
    #[test]
    fn multipage_bundled_page_metadata() {
        let data = std::fs::read(assets_path().join("DjVu3Spec_bundled.djvu"))
            .expect("DjVu3Spec_bundled.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("bundled parse should succeed");

        let page0 = doc.page(0).expect("page 0 must exist");
        assert!(page0.width() > 0, "page width must be non-zero");
        assert!(page0.height() > 0, "page height must be non-zero");
        assert!(page0.dpi() > 0, "page dpi must be non-zero");
    }

    /// NAVM bookmarks from a document that contains them.
    #[test]
    fn navm_bookmarks_present() {
        let data =
            std::fs::read(assets_path().join("navm_fgbz.djvu")).expect("navm_fgbz.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("parse should succeed");
        // navm_fgbz.djvu has NAVM chunk — should return at least one bookmark
        let bm = doc.bookmarks();
        assert!(
            !bm.is_empty(),
            "navm_fgbz.djvu should have at least one bookmark"
        );
    }

    /// Documents without NAVM should return empty bookmark list.
    #[test]
    fn no_navm_returns_empty_bookmarks() {
        let data =
            std::fs::read(assets_path().join("chicken.djvu")).expect("chicken.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("parse should succeed");
        assert!(
            doc.bookmarks().is_empty(),
            "chicken.djvu has no NAVM — bookmarks should be empty"
        );
    }

    /// Indirect document: parse with resolver callback.
    ///
    /// We simulate an indirect document by constructing a DJVM DIRM that marks
    /// entries as non-bundled and supplying a resolver that returns the bytes of
    /// the real chicken.djvu page.
    #[test]
    fn indirect_document_with_resolver() {
        // Load chicken.djvu — we'll use it as the "resolved" page.
        let chicken_data =
            std::fs::read(assets_path().join("chicken.djvu")).expect("chicken.djvu must exist");
        // Build a minimal indirect DJVM document referencing "chicken.djvu"
        let djvm_data = build_indirect_djvm_bytes("chicken.djvu");

        let resolver = |name: &str| -> Result<Vec<u8>, DocError> {
            if name == "chicken.djvu" {
                Ok(chicken_data.clone())
            } else {
                Err(DocError::IndirectResolve(name.to_string()))
            }
        };

        let doc = DjVuDocument::parse_with_resolver(&djvm_data, Some(resolver))
            .expect("indirect parse should succeed");

        assert_eq!(doc.page_count(), 1);
        let page = doc.page(0).unwrap();
        assert_eq!(page.width(), 181);
        assert_eq!(page.height(), 240);
    }

    /// Indirect document without resolver must return NoResolver error.
    #[test]
    fn indirect_document_no_resolver_returns_error() {
        let djvm_data = build_indirect_djvm_bytes("chicken.djvu");
        let err = DjVuDocument::parse(&djvm_data).expect_err("should fail without resolver");
        assert!(
            matches!(err, DocError::NoResolver),
            "expected NoResolver, got {err:?}"
        );
    }

    /// Page must not decode image data before thumbnail() is called.
    ///
    /// We verify laziness by confirming that constructing the document and
    /// accessing `page()` without calling `thumbnail()` does not involve
    /// any IW44 decoder side-effects.  We test this by calling thumbnail()
    /// on a page with no TH44 chunks and verifying we get Ok(None).
    #[test]
    fn page_is_lazy_no_decode_before_thumbnail() {
        let data =
            std::fs::read(assets_path().join("boy_jb2.djvu")).expect("boy_jb2.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("parse should succeed");
        let page = doc.page(0).expect("page 0 must exist");

        // page.chunks should be populated but no decoding has happened
        assert!(!page.chunks.is_empty(), "chunks must be stored (lazy)");

        // thumbnail() triggers decode — but there's no TH44 chunk in boy_jb2.djvu
        let thumb = page.thumbnail().expect("thumbnail() should not error");
        assert!(thumb.is_none());
    }

    /// Non-DjVu file returns NotDjVu error.
    #[test]
    fn not_djvu_returns_error() {
        // Construct a valid IFF with a non-DjVu form type
        let mut data = Vec::new();
        data.extend_from_slice(b"AT&T");
        data.extend_from_slice(b"FORM");
        data.extend_from_slice(&8u32.to_be_bytes());
        data.extend_from_slice(b"XXXXXXXX"); // form_type = XXXX + 4 dummy bytes
        let err = DjVuDocument::parse(&data).expect_err("should fail");
        assert!(
            matches!(err, DocError::NotDjVu(_) | DocError::Iff(_)),
            "expected NotDjVu or Iff error, got {err:?}"
        );
    }

    // ---- Helpers: build minimal DJVM documents for indirect tests -----------

    /// Build a minimal indirect FORM:DJVM with 1 page component named "chicken.djvu".
    ///
    /// DIRM format: flags=0x00 (not bundled), nfiles=1, followed by BZZ-compressed
    /// metadata. The BZZ bytes below were pre-computed using the reference `bzz -e`
    /// tool encoding the metadata:
    ///   `\x00\x00\x00` (size, 3 bytes) + `\x01` (Page flag) + `chicken.djvu\x00`
    fn build_indirect_djvm_bytes(_page_name: &str) -> Vec<u8> {
        // BZZ-encoded DIRM metadata for 1 Page component named "chicken.djvu".
        // Generated with: printf '\x00\x00\x00\x01chicken.djvu\x00' | bzz -e - file.bzz
        // Verified to decode back to the original 17-byte meta block.
        let bzz_meta: &[u8] = &[
            0xff, 0xff, 0xed, 0xbf, 0x8a, 0x1f, 0xbe, 0xad, 0x14, 0x57, 0x10, 0xc9, 0x63, 0x19,
            0x11, 0xf0, 0x85, 0x28, 0x12, 0x8a, 0xbf,
        ];

        let mut dirm_data = Vec::new();
        dirm_data.push(0x00); // flags: not bundled (is_bundled bit = 0)
        dirm_data.push(0x00); // nfiles high byte
        dirm_data.push(0x01); // nfiles low byte = 1
        dirm_data.extend_from_slice(bzz_meta);

        build_djvm_with_dirm(&dirm_data)
    }

    fn build_djvm_with_dirm(dirm_data: &[u8]) -> Vec<u8> {
        // DIRM chunk
        let mut dirm_chunk = Vec::new();
        dirm_chunk.extend_from_slice(b"DIRM");
        dirm_chunk.extend_from_slice(&(dirm_data.len() as u32).to_be_bytes());
        dirm_chunk.extend_from_slice(dirm_data);
        if !dirm_data.len().is_multiple_of(2) {
            dirm_chunk.push(0); // pad to even
        }

        // FORM:DJVM body
        let mut form_body = Vec::new();
        form_body.extend_from_slice(b"DJVM");
        form_body.extend_from_slice(&dirm_chunk);

        // Full file
        let mut file = Vec::new();
        file.extend_from_slice(b"AT&T");
        file.extend_from_slice(b"FORM");
        file.extend_from_slice(&(form_body.len() as u32).to_be_bytes());
        file.extend_from_slice(&form_body);
        file
    }

    // ── raw chunk API (Issue #43) ────────────────────────────────────────────

    /// `DjVuPage::raw_chunk` returns bytes for known chunk types.
    #[test]
    fn page_raw_chunk_info_present() {
        let data =
            std::fs::read(assets_path().join("chicken.djvu")).expect("chicken.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("parse must succeed");
        let page = doc.page(0).expect("page 0 must exist");

        // INFO chunk must be present
        let info = page.raw_chunk(b"INFO").expect("INFO chunk must be present");
        assert_eq!(info.len(), 10, "INFO chunk is always 10 bytes");
    }

    /// `DjVuPage::raw_chunk` returns None for absent chunk types.
    #[test]
    fn page_raw_chunk_absent() {
        let data =
            std::fs::read(assets_path().join("chicken.djvu")).expect("chicken.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("parse must succeed");
        let page = doc.page(0).expect("page 0 must exist");

        assert!(
            page.raw_chunk(b"XXXX").is_none(),
            "unknown chunk type must return None"
        );
    }

    /// `DjVuPage::all_chunks` returns multiple BG44 chunks in order.
    #[test]
    fn page_all_chunks_bg44_multiple() {
        // big-scanned-page.djvu has 4 progressive BG44 chunks
        let data = std::fs::read(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/big-scanned-page.djvu"),
        )
        .expect("big-scanned-page.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("parse must succeed");
        let page = doc.page(0).expect("page 0 must exist");

        let bg44 = page.all_chunks(b"BG44");
        assert!(
            bg44.len() >= 2,
            "colour page must have ≥2 BG44 chunks, got {}",
            bg44.len()
        );

        // Chunks must be non-empty
        for (i, chunk) in bg44.iter().enumerate() {
            assert!(!chunk.is_empty(), "BG44 chunk {i} must not be empty");
        }
    }

    /// `DjVuPage::chunk_ids` lists all chunk IDs in order.
    #[test]
    fn page_chunk_ids_includes_info() {
        let data =
            std::fs::read(assets_path().join("chicken.djvu")).expect("chicken.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("parse must succeed");
        let page = doc.page(0).expect("page 0 must exist");

        let ids = page.chunk_ids();
        assert!(!ids.is_empty(), "chunk_ids must not be empty");
        assert!(
            ids.contains(b"INFO"),
            "chunk_ids must include INFO, got: {:?}",
            ids.iter()
                .map(|id| std::str::from_utf8(id).unwrap_or("????"))
                .collect::<Vec<_>>()
        );
    }

    /// `DjVuDocument::raw_chunk` works for single-page DJVU files.
    #[test]
    fn document_raw_chunk_single_page() {
        let data =
            std::fs::read(assets_path().join("chicken.djvu")).expect("chicken.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("parse must succeed");

        // Single-page DJVU exposes all top-level chunks at document level too
        let info = doc
            .raw_chunk(b"INFO")
            .expect("document must expose INFO chunk");
        assert_eq!(info.len(), 10);
    }

    // ── DJVI shared dictionary / INCL chunks (Issue #45) ────────────────────

    /// DjVu3Spec_bundled.djvu has shared DJVI symbol dictionaries.
    /// Parsing must succeed and pages with INCL references must carry the dict.
    #[test]
    fn djvi_shared_dict_parsed_from_bundled_djvm() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/DjVu3Spec_bundled.djvu");
        let data = std::fs::read(&path).expect("DjVu3Spec_bundled.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("parse must succeed");

        assert!(doc.page_count() > 0, "document must have pages");

        // At least one page should have a shared dict loaded (shared_djbz Some)
        let pages_with_dict = doc.pages.iter().filter(|p| p.shared_djbz.is_some()).count();
        assert!(
            pages_with_dict > 0,
            "at least one page must have a resolved shared DJVI dict"
        );
    }

    /// Pages with INCL references must render their mask without error.
    #[test]
    fn djvi_incl_page_mask_renders_ok() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/DjVu3Spec_bundled.djvu");
        let data = std::fs::read(&path).expect("DjVu3Spec_bundled.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("parse must succeed");

        // Find first page with a shared dict and render its mask
        let page = doc
            .pages
            .iter()
            .find(|p| p.shared_djbz.is_some())
            .expect("at least one page must have a shared dict");

        let mask = page
            .extract_mask()
            .expect("extract_mask must succeed for INCL page");
        assert!(mask.is_some(), "INCL page must have a JB2 mask");
        let bm = mask.unwrap();
        assert!(
            bm.width > 0 && bm.height > 0,
            "mask must have non-zero dimensions"
        );
    }

    /// Pages without INCL still render correctly (no regression).
    #[test]
    fn no_regression_non_incl_pages() {
        // boy_jb2.djvu has a Sjbz mask and no INCL reference
        let data = std::fs::read(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/boy_jb2.djvu"),
        )
        .expect("boy_jb2.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("parse must succeed");
        let page = doc.page(0).expect("page 0 must exist");
        assert!(
            page.shared_djbz.is_none(),
            "single-page DJVU has no shared dict"
        );
        let mask = page.extract_mask().expect("extract_mask must succeed");
        assert!(mask.is_some(), "boy_jb2.djvu page must have a JB2 mask");
    }

    /// Round-trip: bytes from `raw_chunk` re-parse to the same metadata.
    #[test]
    fn page_raw_chunk_info_roundtrip() {
        let data =
            std::fs::read(assets_path().join("chicken.djvu")).expect("chicken.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("parse must succeed");
        let page = doc.page(0).expect("page 0 must exist");

        let raw_info = page.raw_chunk(b"INFO").expect("INFO chunk must be present");
        let reparsed = crate::info::PageInfo::parse(raw_info).expect("re-parse must succeed");
        assert_eq!(reparsed.width, page.width() as u16);
        assert_eq!(reparsed.height, page.height() as u16);
        assert_eq!(reparsed.dpi, page.dpi());
    }

    /// MmapDocument opens a file and parses identically to in-memory parse.
    #[test]
    #[cfg(feature = "mmap")]
    fn mmap_document_matches_parse() {
        let path = assets_path().join("chicken.djvu");
        let mmap_doc = MmapDocument::open(&path).expect("mmap open should succeed");
        let data = std::fs::read(&path).expect("read should succeed");
        let mem_doc = DjVuDocument::parse(&data).expect("parse should succeed");

        assert_eq!(mmap_doc.page_count(), mem_doc.page_count());
        for i in 0..mmap_doc.page_count() {
            let mp = mmap_doc.page(i).unwrap();
            let pp = mem_doc.page(i).unwrap();
            assert_eq!(mp.width(), pp.width());
            assert_eq!(mp.height(), pp.height());
            assert_eq!(mp.dpi(), pp.dpi());
        }
    }
}
