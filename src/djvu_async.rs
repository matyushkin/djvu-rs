//! Async render surface for [`DjVuPage`] — phase 5 extension.
//!
//! Feature-gated: `--features async` (adds `tokio` as a dependency).
//!
//! All rendering is delegated to [`tokio::task::spawn_blocking`]: the CPU-bound
//! IW44/JB2 decode work runs on the blocking thread pool and never blocks the
//! async runtime thread.
//!
//! [`DjVuPage`] implements [`Clone`], so the page is cloned into the blocking
//! closure with no unsafe code and no thread management by the caller.
//!
//! ## Key public functions
//!
//! - [`render_pixmap_async`] — async wrapper around [`djvu_render::render_pixmap`]
//! - [`render_gray8_async`] — async wrapper around [`djvu_render::render_gray8`]
//! - [`render_progressive_stream`] — streaming progressive render yielding one frame per BG44 chunk
//!
//! ## Example: concurrent multi-page rendering
//!
//! ```no_run
//! use djvu_rs::djvu_document::DjVuDocument;
//! use djvu_rs::djvu_render::RenderOptions;
//! use djvu_rs::djvu_async::render_pixmap_async;
//!
//! #[tokio::main]
//! async fn main() {
//!     let data = std::fs::read("document.djvu").unwrap();
//!     let doc = std::sync::Arc::new(DjVuDocument::parse(&data).unwrap());
//!
//!     let futures: Vec<_> = (0..doc.page_count())
//!         .filter_map(|i| doc.page(i).ok())
//!         .map(|page| {
//!             let page = page.clone();
//!             let opts = RenderOptions { width: 800, height: 600, ..Default::default() };
//!             tokio::spawn(async move { render_pixmap_async(&page, opts).await })
//!         })
//!         .collect();
//!
//!     for handle in futures {
//!         let pixmap = handle.await.unwrap().unwrap();
//!         println!("{}×{}", pixmap.width, pixmap.height);
//!     }
//! }
//! ```

use std::{ops::Range, sync::Arc};

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt},
    sync::{Mutex, OnceCell},
};

use crate::{
    djvu_document::{DjVuDocument, DjVuPage, DocError},
    djvu_render::{self, RenderError, RenderOptions},
    error::IffError,
    iff::parse_form,
    pixmap::{GrayPixmap, Pixmap},
};

// ── Error types ───────────────────────────────────────────────────────────────

/// Errors from async rendering.
#[derive(Debug, thiserror::Error)]
pub enum AsyncRenderError {
    /// The underlying render failed.
    #[error("render error: {0}")]
    Render(#[from] RenderError),

    /// The blocking task was cancelled or panicked.
    #[error("spawn_blocking join error: {0}")]
    Join(String),
}

/// Errors from async document loading.
#[derive(Debug, thiserror::Error)]
pub enum AsyncLoadError {
    /// I/O error from the underlying async reader.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The buffered bytes failed to parse as a DjVu document.
    #[error("parse error: {0}")]
    Parse(#[from] DocError),
}

/// Errors from true lazy async document loading (#233 Phase 3 PR1).
#[derive(Debug, thiserror::Error)]
pub enum AsyncLazyError {
    /// I/O error from the underlying async reader.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The fetched bytes failed to parse as a DjVu document.
    #[error("parse error: {0}")]
    Parse(#[from] DocError),

    /// IFF container parse error while inspecting lazy page bytes.
    #[error("IFF error: {0}")]
    Iff(#[from] IffError),

    /// Page index is out of range.
    #[error("page index {index} is out of range (document has {count} pages)")]
    PageOutOfRange { index: usize, count: usize },

    /// This lazy-loading slice intentionally rejects a document shape.
    #[error("unsupported lazy document shape: {0}")]
    Unsupported(&'static str),
}

// ── True lazy async document loader (#233 Phase 3 PR1) ───────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LazyComponentType {
    Shared,
    Page,
    Thumbnail,
}

#[derive(Debug, Clone)]
struct LazyDirmEntry {
    comp_type: LazyComponentType,
    offset: u32,
}

/// Native async lazy DjVu document.
///
/// This is the first Phase 3 slice for #233: it indexes a seekable async
/// reader up front, then fetches and parses each page only when
/// [`LazyDocument::page_async`] is called. Parsed pages are cached as
/// `Arc<DjVuPage>` so callers can render them concurrently without borrowing
/// the document across awaits.
///
/// Current scope:
/// - single-page `FORM:DJVU`
/// - bundled `FORM:DJVM` pages that do not contain `INCL`
///
/// Shared DJVI/INCL resolution and WASM `!Send` readers are intentionally left
/// to the next issue slices.
pub struct LazyDocument<R> {
    reader: Arc<Mutex<R>>,
    pages: Vec<LazyPageIndex>,
    cache: Vec<OnceCell<Arc<DjVuPage>>>,
}

#[derive(Debug, Clone)]
struct LazyPageIndex {
    range: Range<u64>,
}

impl<R> LazyDocument<R>
where
    R: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    /// Build a native lazy document index from an async seekable reader.
    pub async fn from_async_reader_lazy(mut reader: R) -> Result<Self, AsyncLazyError> {
        let file_len = reader.seek(std::io::SeekFrom::End(0)).await?;
        reader.seek(std::io::SeekFrom::Start(0)).await?;

        let mut head = [0u8; 16];
        reader.read_exact(&mut head).await?;
        if &head[..4] != b"AT&T" || &head[4..8] != b"FORM" {
            return Err(AsyncLazyError::Unsupported("not an AT&T FORM document"));
        }

        let form_type = &head[12..16];
        let pages = if form_type == b"DJVU" {
            vec![LazyPageIndex { range: 0..file_len }]
        } else if form_type == b"DJVM" {
            index_bundled_djvm(&mut reader).await?
        } else {
            return Err(AsyncLazyError::Unsupported(
                "lazy loader supports only FORM:DJVU and bundled FORM:DJVM",
            ));
        };

        if pages.is_empty() {
            return Err(AsyncLazyError::Unsupported(
                "document has no lazy-loadable pages",
            ));
        }

        let cache = (0..pages.len()).map(|_| OnceCell::new()).collect();
        Ok(Self {
            reader: Arc::new(Mutex::new(reader)),
            pages,
            cache,
        })
    }

    /// Number of lazy-loadable pages.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Fetch, parse, cache, and return page `index`.
    pub async fn page_async(&self, index: usize) -> Result<Arc<DjVuPage>, AsyncLazyError> {
        let page = self
            .pages
            .get(index)
            .ok_or(AsyncLazyError::PageOutOfRange {
                index,
                count: self.pages.len(),
            })?
            .clone();

        self.cache[index]
            .get_or_try_init(|| async move {
                let bytes = self.read_page_bytes(page.range).await?;
                let form = parse_form(&bytes)?;
                if form.chunks.iter().any(|c| &c.id == b"INCL") {
                    return Err(AsyncLazyError::Unsupported(
                        "lazy shared DJVI/INCL pages are not implemented yet",
                    ));
                }
                let doc = DjVuDocument::parse(&bytes)?;
                if doc.page_count() != 1 {
                    return Err(AsyncLazyError::Unsupported(
                        "lazy page chunk did not parse to exactly one page",
                    ));
                }
                Ok(Arc::new(doc.page(0)?.clone()))
            })
            .await
            .cloned()
    }

    async fn read_page_bytes(&self, range: Range<u64>) -> Result<Vec<u8>, AsyncLazyError> {
        let len = usize::try_from(range.end.saturating_sub(range.start))
            .map_err(|_| AsyncLazyError::Unsupported("page range exceeds addressable memory"))?;
        let mut bytes = Vec::with_capacity(len.saturating_add(4));
        if range.start != 0 {
            bytes.extend_from_slice(b"AT&T");
        }
        let mut reader = self.reader.lock().await;
        reader.seek(std::io::SeekFrom::Start(range.start)).await?;
        let mut chunk = vec![0u8; len];
        reader.read_exact(&mut chunk).await?;
        bytes.extend_from_slice(&chunk);
        Ok(bytes)
    }
}

/// Build a native lazy document index from an async seekable reader.
///
/// Convenience wrapper around [`LazyDocument::from_async_reader_lazy`].
pub async fn from_async_reader_lazy<R>(reader: R) -> Result<LazyDocument<R>, AsyncLazyError>
where
    R: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    LazyDocument::from_async_reader_lazy(reader).await
}

async fn index_bundled_djvm<R>(reader: &mut R) -> Result<Vec<LazyPageIndex>, AsyncLazyError>
where
    R: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    let mut chunk_hdr = [0u8; 8];
    reader.read_exact(&mut chunk_hdr).await?;
    if &chunk_hdr[..4] != b"DIRM" {
        return Err(AsyncLazyError::Unsupported(
            "lazy DJVM loader requires DIRM as the first inner chunk",
        ));
    }
    let dirm_len =
        u32::from_be_bytes([chunk_hdr[4], chunk_hdr[5], chunk_hdr[6], chunk_hdr[7]]) as usize;
    let padded = dirm_len + (dirm_len & 1);
    let mut dirm = vec![0u8; padded];
    reader.read_exact(&mut dirm).await?;

    let entries = parse_lazy_dirm(&dirm[..dirm_len])?;
    let mut pages = Vec::new();
    for entry in entries {
        if entry.comp_type != LazyComponentType::Page {
            continue;
        }
        let start = entry.offset as u64;
        reader.seek(std::io::SeekFrom::Start(start + 4)).await?;
        let mut size_bytes = [0u8; 4];
        reader.read_exact(&mut size_bytes).await?;
        let size = u32::from_be_bytes(size_bytes) as u64;
        pages.push(LazyPageIndex {
            range: start..start.saturating_add(8).saturating_add(size),
        });
    }
    Ok(pages)
}

fn parse_lazy_dirm(data: &[u8]) -> Result<Vec<LazyDirmEntry>, AsyncLazyError> {
    if data.len() < 3 {
        return Err(AsyncLazyError::Unsupported("DIRM chunk too short"));
    }
    let dflags = data[0];
    if (dflags >> 7) == 0 {
        return Err(AsyncLazyError::Unsupported(
            "indirect DJVM lazy loading is not implemented yet",
        ));
    }
    let nfiles = u16::from_be_bytes([data[1], data[2]]) as usize;
    let offsets_start = 3usize;
    let offsets_end = offsets_start
        .checked_add(nfiles.saturating_mul(4))
        .ok_or(AsyncLazyError::Unsupported("DIRM offset table overflow"))?;
    if offsets_end > data.len() {
        return Err(AsyncLazyError::Unsupported("DIRM offset table truncated"));
    }

    let mut offsets = Vec::with_capacity(nfiles);
    for i in 0..nfiles {
        let base = offsets_start + i * 4;
        offsets.push(u32::from_be_bytes([
            data[base],
            data[base + 1],
            data[base + 2],
            data[base + 3],
        ]));
    }

    let meta = djvu_bzz::bzz_decode(&data[offsets_end..]).unwrap_or_default();
    let mut comp_types = Vec::with_capacity(nfiles);
    let flags_start = nfiles * 3;
    if flags_start + nfiles <= meta.len() {
        for flag in &meta[flags_start..flags_start + nfiles] {
            let comp_type = match flag & 0x3f {
                1 => LazyComponentType::Page,
                2 => LazyComponentType::Thumbnail,
                _ => LazyComponentType::Shared,
            };
            comp_types.push(comp_type);
        }
    } else {
        comp_types.resize(nfiles, LazyComponentType::Page);
    }

    Ok(offsets
        .into_iter()
        .zip(comp_types)
        .map(|(offset, comp_type)| LazyDirmEntry { comp_type, offset })
        .collect())
}

// ── Async document loader ─────────────────────────────────────────────────────

/// Asynchronously load and parse a DjVu document from any [`AsyncRead`].
///
/// **Phase 1 of #196.** Convenience constructor that buffers the full reader
/// into memory before handing the bytes to [`DjVuDocument::parse`]. Memory
/// still peaks at full file size, but removes the synchronous `std::fs::read`
/// boundary at the call site — works directly with [`tokio::fs::File`], HTTP
/// body streams, S3 GetObject, etc.
///
/// Phases 2/3 will add genuine streaming: Phase 2 reads only the IFF
/// header and DIRM up front and exposes per-page byte offsets; Phase 3
/// makes [`DjVuDocument::page`] async and fetches each page's bytes on
/// demand (HTTP Range requests, etc.).
///
/// # Example
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use djvu_rs::djvu_async::load_document_async;
/// use tokio::fs::File;
///
/// let file = File::open("document.djvu").await?;
/// let doc = load_document_async(file).await?;
/// println!("loaded {} pages", doc.page_count());
/// # Ok(()) }
/// ```
pub async fn load_document_async<R>(mut reader: R) -> Result<DjVuDocument, AsyncLoadError>
where
    R: AsyncRead + Unpin + Send,
{
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).await?;
    Ok(DjVuDocument::parse(&buf)?)
}

/// Async loader that reads the IFF + FORM + DIRM head separately from the
/// page bodies (#196 Phase 2).
///
/// **Phase 2 of #196.** Issues two `read_exact` calls for the document head
/// (IFF magic + FORM length + form_type, then the DIRM chunk header + payload),
/// then a single `read_to_end` for the remainder. The total bytes received
/// match Phase 1 — this constructor still returns an in-memory
/// [`DjVuDocument`] — but a bandwidth-instrumented `AsyncRead` implementation
/// can observe the head-first read pattern, and the resulting document
/// exposes [`DjVuDocument::page_byte_range`] for any caller that wants to
/// fan out per-page byte fetches via HTTP `Range` requests on a separate
/// connection.
///
/// For documents that aren't bundled DJVM (single-page DJVU, indirect DJVM,
/// or anything without a DIRM in the first chunk), this falls back to the
/// Phase 1 buffered-read behavior — there's nothing useful to stream.
///
/// # Errors
///
/// - `AsyncLoadError::Io` — any underlying read fails
/// - `AsyncLoadError::Parse` — the assembled buffer fails [`DjVuDocument::parse`]
pub async fn load_document_async_streaming<R>(mut reader: R) -> Result<DjVuDocument, AsyncLoadError>
where
    R: AsyncRead + Unpin + Send,
{
    // 1) IFF outer header: 4-byte magic "AT&T" + "FORM" + 4-byte length + 4-byte form_type = 16 bytes.
    let mut head = [0u8; 16];
    reader.read_exact(&mut head).await?;

    // If it isn't a DJVM bundle, the rest of the file is just page payload —
    // no per-chunk streaming benefit, so fall back to bulk read.
    let is_djvm = &head[..4] == b"AT&T" && &head[4..8] == b"FORM" && &head[12..16] == b"DJVM";

    let mut buf = Vec::with_capacity(if is_djvm {
        // Pre-size: 1 MB head guess; Vec grows as needed.
        1 << 20
    } else {
        16 * 1024
    });
    buf.extend_from_slice(&head);

    if is_djvm {
        // 2) Next chunk header: 4-byte id + 4-byte BE length.
        let mut chunk_hdr = [0u8; 8];
        reader.read_exact(&mut chunk_hdr).await?;
        buf.extend_from_slice(&chunk_hdr);

        // If the first inner chunk is DIRM, read its payload separately so
        // a recording reader sees the head-first pattern. Otherwise just
        // continue with read_to_end — the document layout is non-canonical
        // and Phase 2's offset map wouldn't apply anyway.
        if &chunk_hdr[..4] == b"DIRM" {
            let dirm_len =
                u32::from_be_bytes([chunk_hdr[4], chunk_hdr[5], chunk_hdr[6], chunk_hdr[7]])
                    as usize;
            // IFF chunks pad to 2-byte boundary; the parser handles this, but
            // we must read those padding bytes too to keep alignment.
            let padded = dirm_len + (dirm_len & 1);
            let mut dirm_buf = vec![0u8; padded];
            reader.read_exact(&mut dirm_buf).await?;
            buf.extend_from_slice(&dirm_buf);
        }
    }

    // 3) Bulk-read the remainder.
    reader.read_to_end(&mut buf).await?;

    Ok(DjVuDocument::parse(&buf)?)
}

// ── Async render functions ────────────────────────────────────────────────────

/// Render `page` to an RGBA [`Pixmap`] asynchronously.
///
/// Clones the page and delegates to [`djvu_render::render_pixmap`] via
/// [`tokio::task::spawn_blocking`]. The render runs on the blocking thread
/// pool and does not block the async runtime.
///
/// # Example
///
/// ```no_run
/// # async fn example() {
/// use djvu_rs::djvu_document::DjVuDocument;
/// use djvu_rs::djvu_render::RenderOptions;
/// use djvu_rs::djvu_async::render_pixmap_async;
///
/// let data = std::fs::read("file.djvu").unwrap();
/// let doc = DjVuDocument::parse(&data).unwrap();
/// let page = doc.page(0).unwrap();
/// let opts = RenderOptions { width: 400, height: 300, ..Default::default() };
/// let pixmap = render_pixmap_async(page, opts).await.unwrap();
/// println!("{}×{}", pixmap.width, pixmap.height);
/// # }
/// ```
pub async fn render_pixmap_async(
    page: &DjVuPage,
    opts: RenderOptions,
) -> Result<Pixmap, AsyncRenderError> {
    let page = Arc::new(page.clone());
    tokio::task::spawn_blocking(move || {
        djvu_render::render_pixmap(&page, &opts).map_err(AsyncRenderError::Render)
    })
    .await
    .map_err(|e| AsyncRenderError::Join(e.to_string()))?
}

/// Render `page` to an 8-bit grayscale [`GrayPixmap`] asynchronously.
///
/// Clones the page and delegates to [`djvu_render::render_gray8`] via
/// [`tokio::task::spawn_blocking`].
pub async fn render_gray8_async(
    page: &DjVuPage,
    opts: RenderOptions,
) -> Result<GrayPixmap, AsyncRenderError> {
    let page = Arc::new(page.clone());
    tokio::task::spawn_blocking(move || {
        djvu_render::render_gray8(&page, &opts).map_err(AsyncRenderError::Render)
    })
    .await
    .map_err(|e| AsyncRenderError::Join(e.to_string()))?
}

/// Render a `DjVuPage` as a lazy progressive stream of [`Pixmap`] frames.
///
/// Yields one frame per BG44 wavelet refinement chunk: the first frame is the
/// coarsest (fastest to produce), and each subsequent frame adds detail. The
/// final frame is equivalent to [`render_pixmap`][djvu_render::render_pixmap].
///
/// If the page has no BG44 chunks (bilevel JB2-only pages), exactly one frame
/// is yielded via [`render_pixmap`][djvu_render::render_pixmap].
///
/// Each frame is produced via [`tokio::task::spawn_blocking`] just before it is
/// yielded, so the stream never blocks the async runtime thread.
///
/// # Example
///
/// ```no_run
/// # async fn example() {
/// use djvu_rs::djvu_document::DjVuDocument;
/// use djvu_rs::djvu_render::RenderOptions;
/// use djvu_rs::djvu_async::render_progressive_stream;
/// use futures::StreamExt;
///
/// let data = std::fs::read("file.djvu").unwrap();
/// let doc = DjVuDocument::parse(&data).unwrap();
/// let page = doc.page(0).unwrap();
/// let opts = RenderOptions { width: 800, height: 600, ..Default::default() };
///
/// let stream = render_progressive_stream(page, opts);
/// futures::pin_mut!(stream);
/// while let Some(pixmap) = stream.next().await {
///     let pixmap = pixmap.unwrap();
///     println!("{}×{}", pixmap.width, pixmap.height);
/// }
/// # }
/// ```
pub fn render_progressive_stream(
    page: &DjVuPage,
    opts: RenderOptions,
) -> impl futures_core::Stream<Item = Result<Pixmap, AsyncRenderError>> {
    // Single clone wrapped in Arc — all spawn_blocking closures share
    // this one allocation instead of cloning the full page each time.
    let page = Arc::new(page.clone());
    let n_chunks = page.bg44_chunks().len();

    async_stream::stream! {
        if n_chunks == 0 {
            let page = Arc::clone(&page);
            let opts = opts.clone();
            let result = tokio::task::spawn_blocking(move || {
                djvu_render::render_pixmap(&page, &opts).map_err(AsyncRenderError::Render)
            })
            .await
            .map_err(|e| AsyncRenderError::Join(e.to_string()));
            yield result.and_then(|r| r);
        } else {
            for chunk_n in 0..n_chunks {
                let page = Arc::clone(&page);
                let opts = opts.clone();
                let result = tokio::task::spawn_blocking(move || {
                    djvu_render::render_progressive(&page, &opts, chunk_n)
                        .map_err(AsyncRenderError::Render)
                })
                .await
                .map_err(|e| AsyncRenderError::Join(e.to_string()));
                yield result.and_then(|r| r);
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::djvu_document::DjVuDocument;

    fn assets_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/djvujs/library/assets")
    }

    fn load_doc(name: &str) -> DjVuDocument {
        let data =
            std::fs::read(assets_path().join(name)).unwrap_or_else(|_| panic!("{name} must exist"));
        DjVuDocument::parse(&data).unwrap_or_else(|e| panic!("{e}"))
    }

    /// `render_pixmap_async` returns a pixmap with correct dimensions.
    #[tokio::test]
    async fn render_pixmap_async_correct_dims() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let pw = page.width() as u32;
        let ph = page.height() as u32;

        let opts = RenderOptions {
            width: pw,
            height: ph,
            ..Default::default()
        };
        let pm = render_pixmap_async(page, opts)
            .await
            .expect("async render must succeed");
        assert_eq!(pm.width, pw);
        assert_eq!(pm.height, ph);
    }

    /// `render_gray8_async` returns a grayscale pixmap with the right size.
    #[tokio::test]
    async fn render_gray8_async_correct_dims() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let pw = page.width() as u32;
        let ph = page.height() as u32;

        let opts = RenderOptions {
            width: pw,
            height: ph,
            ..Default::default()
        };
        let gm = render_gray8_async(page, opts)
            .await
            .expect("async gray render must succeed");
        assert_eq!(gm.width, pw);
        assert_eq!(gm.height, ph);
        assert_eq!(gm.data.len(), (pw * ph) as usize);
    }

    /// Async and sync renders produce identical results.
    #[tokio::test]
    async fn async_matches_sync() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let pw = page.width() as u32;
        let ph = page.height() as u32;

        let opts = RenderOptions {
            width: pw,
            height: ph,
            ..Default::default()
        };
        let sync_pm = djvu_render::render_pixmap(page, &opts).expect("sync render must succeed");
        let async_pm = render_pixmap_async(page, opts.clone())
            .await
            .expect("async render must succeed");

        assert_eq!(
            sync_pm.data, async_pm.data,
            "async and sync renders must match"
        );
    }

    /// Concurrent rendering of multiple instances of the same page succeeds.
    #[tokio::test]
    async fn concurrent_render_multiple_tasks() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let pw = page.width() as u32;
        let ph = page.height() as u32;
        let opts = RenderOptions {
            width: pw / 2,
            height: ph / 2,
            scale: 0.5,
            ..Default::default()
        };

        // Spawn 4 concurrent renders of the same page.
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let page_clone = page.clone();
                let opts_clone = opts.clone();
                tokio::spawn(async move { render_pixmap_async(&page_clone, opts_clone).await })
            })
            .collect();

        for handle in handles {
            let pm = handle
                .await
                .expect("task must not panic")
                .expect("render must succeed");
            assert_eq!(pm.width, pw / 2);
            assert_eq!(pm.height, ph / 2);
        }
    }

    /// `AsyncRenderError::Render` wraps `RenderError`.
    #[test]
    fn async_render_error_display() {
        let err = AsyncRenderError::Render(crate::djvu_render::RenderError::InvalidDimensions {
            width: 0,
            height: 0,
        });
        let s = err.to_string();
        assert!(
            s.contains("render error"),
            "error must mention 'render error'"
        );
    }

    // ── render_progressive_stream tests ──────────────────────────────────────

    /// Last frame from the progressive stream matches `render_pixmap`.
    #[tokio::test]
    async fn progressive_stream_last_frame_matches_pixmap() {
        use futures::StreamExt;
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 100,
            height: 80,
            ..Default::default()
        };

        let stream = render_progressive_stream(page, opts.clone());
        futures::pin_mut!(stream);

        let mut frames: Vec<Pixmap> = Vec::new();
        while let Some(result) = stream.next().await {
            frames.push(result.expect("frame should succeed"));
        }

        assert!(!frames.is_empty(), "stream must yield at least one frame");

        let expected = djvu_render::render_pixmap(page, &opts).expect("render_pixmap must succeed");
        assert_eq!(
            frames.last().unwrap().data,
            expected.data,
            "last frame must match render_pixmap"
        );
    }

    /// Each successive frame has the same dimensions.
    #[tokio::test]
    async fn progressive_stream_consistent_dimensions() {
        use futures::StreamExt;
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let n_chunks = page.bg44_chunks().len();
        let opts = RenderOptions {
            width: 100,
            height: 80,
            ..Default::default()
        };

        let stream = render_progressive_stream(page, opts);
        futures::pin_mut!(stream);

        let mut count = 0usize;
        while let Some(result) = stream.next().await {
            let frame = result.expect("frame should succeed");
            assert_eq!(frame.width, 100);
            assert_eq!(frame.height, 80);
            count += 1;
        }

        let expected_count = if n_chunks == 0 { 1 } else { n_chunks };
        assert_eq!(
            count, expected_count,
            "frame count must equal BG44 chunk count"
        );
    }

    // ── load_document_async tests ────────────────────────────────────────────

    /// `load_document_async` over `tokio::fs::File` matches `DjVuDocument::parse`.
    #[tokio::test]
    async fn load_document_async_matches_sync_parse() {
        let path = assets_path().join("chicken.djvu");
        let file = tokio::fs::File::open(&path)
            .await
            .expect("open must succeed");
        let async_doc = load_document_async(file)
            .await
            .expect("async load must succeed");

        let sync_data = std::fs::read(&path).expect("sync read must succeed");
        let sync_doc = DjVuDocument::parse(&sync_data).expect("sync parse must succeed");

        assert_eq!(async_doc.page_count(), sync_doc.page_count());
        for i in 0..sync_doc.page_count() {
            let a = async_doc.page(i).expect("async page");
            let s = sync_doc.page(i).expect("sync page");
            assert_eq!(a.width(), s.width());
            assert_eq!(a.height(), s.height());
        }
    }

    /// `load_document_async` works with an in-memory `&[u8]` reader (e.g. HTTP body).
    #[tokio::test]
    async fn load_document_async_from_in_memory_reader() {
        let path = assets_path().join("chicken.djvu");
        let bytes = std::fs::read(&path).expect("read");

        // `&[u8]` implements AsyncRead via tokio's blanket impl on slices.
        let reader = std::io::Cursor::new(bytes.clone());
        let doc = load_document_async(reader)
            .await
            .expect("async load from cursor must succeed");
        assert!(doc.page_count() > 0);
    }

    /// Truncated / non-DjVu bytes surface as `AsyncLoadError::Parse`, not panic.
    #[tokio::test]
    async fn load_document_async_propagates_parse_error() {
        let bogus = b"not a djvu file at all".to_vec();
        let reader = std::io::Cursor::new(bogus);
        let err = load_document_async(reader)
            .await
            .expect_err("must fail to parse garbage");
        assert!(
            matches!(err, AsyncLoadError::Parse(_)),
            "expected Parse error, got {err:?}"
        );
    }

    /// `LazyDocument` fetches and parses a single-page document only when
    /// `page_async` is called, then returns the cached `Arc` on repeat access.
    #[tokio::test]
    async fn lazy_document_single_page_caches_arc_page() {
        let path = assets_path().join("chicken.djvu");
        let bytes = std::fs::read(&path).expect("read");
        let sync_doc = DjVuDocument::parse(&bytes).expect("sync parse");

        let lazy = from_async_reader_lazy(std::io::Cursor::new(bytes))
            .await
            .expect("lazy index");
        assert_eq!(lazy.page_count(), 1);

        let page_a = lazy.page_async(0).await.expect("lazy page");
        let page_b = lazy.page_async(0).await.expect("lazy cached page");
        assert!(
            Arc::ptr_eq(&page_a, &page_b),
            "repeat access must reuse cache"
        );

        let sync_page = sync_doc.page(0).expect("sync page");
        assert_eq!(page_a.width(), sync_page.width());
        assert_eq!(page_a.height(), sync_page.height());
    }

    /// `LazyDocument` indexes bundled DJVM ranges up front and can fetch a
    /// no-INCL page without reading/parsing the full document body.
    #[tokio::test]
    async fn lazy_document_bundled_page_without_incl_matches_sync() {
        let path = assets_path().join("colorbook.djvu");
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("skip: {} missing", path.display());
            return;
        };
        let sync_doc = DjVuDocument::parse(&bytes).expect("sync parse");
        let lazy = from_async_reader_lazy(std::io::Cursor::new(bytes))
            .await
            .expect("lazy index");

        assert_eq!(lazy.page_count(), sync_doc.page_count());
        let page_index = (0..sync_doc.page_count())
            .find(|&i| {
                sync_doc
                    .page(i)
                    .expect("sync page")
                    .chunk_ids()
                    .iter()
                    .all(|id| id != b"INCL")
            })
            .expect("fixture must contain at least one page without INCL");

        let lazy_page = lazy.page_async(page_index).await.expect("lazy page");
        let sync_page = sync_doc.page(page_index).expect("sync page");
        assert_eq!(lazy_page.width(), sync_page.width());
        assert_eq!(lazy_page.height(), sync_page.height());
    }

    #[tokio::test]
    async fn lazy_document_page_out_of_range() {
        let path = assets_path().join("chicken.djvu");
        let bytes = std::fs::read(&path).expect("read");
        let lazy = from_async_reader_lazy(std::io::Cursor::new(bytes))
            .await
            .expect("lazy index");

        let err = lazy
            .page_async(1)
            .await
            .expect_err("page 1 is out of range");
        assert!(
            matches!(err, AsyncLazyError::PageOutOfRange { index: 1, count: 1 }),
            "unexpected error: {err:?}"
        );
    }

    /// `load_document_async_streaming` produces the same document as
    /// the buffered Phase 1 loader on a bundled DJVM.
    #[tokio::test]
    async fn streaming_loader_matches_buffered() {
        let path = assets_path().join("DjVu3Spec_bundled.djvu");
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("skip: {} missing", path.display());
            return;
        };
        let streamed = load_document_async_streaming(std::io::Cursor::new(bytes.clone()))
            .await
            .expect("streaming load must succeed");
        let buffered = DjVuDocument::parse(&bytes).expect("buffered parse");

        assert_eq!(streamed.page_count(), buffered.page_count());
        for i in 0..buffered.page_count() {
            assert_eq!(streamed.page_byte_range(i), buffered.page_byte_range(i));
        }
    }

    /// `load_document_async_streaming` reads the head before the body
    /// (#196 Phase 2 DoD).
    ///
    /// A custom `AsyncRead` records every requested read size. The first
    /// three calls must be small and bounded (IFF head 16 B, chunk header
    /// 8 B, DIRM payload — typically a few KB on a real document).
    #[tokio::test]
    async fn streaming_loader_reads_head_before_body() {
        use std::sync::{Arc, Mutex};

        let path = assets_path().join("DjVu3Spec_bundled.djvu");
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("skip: {} missing", path.display());
            return;
        };

        struct RecordingReader {
            inner: std::io::Cursor<Vec<u8>>,
            sizes: Arc<Mutex<Vec<usize>>>,
        }
        impl tokio::io::AsyncRead for RecordingReader {
            fn poll_read(
                mut self: std::pin::Pin<&mut Self>,
                _cx: &mut std::task::Context<'_>,
                buf: &mut tokio::io::ReadBuf<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                let want = buf.remaining();
                let pos = self.inner.position() as usize;
                let src = self.inner.get_ref();
                let n = want.min(src.len().saturating_sub(pos));
                if n > 0 {
                    buf.put_slice(&src[pos..pos + n]);
                    self.inner.set_position((pos + n) as u64);
                }
                self.sizes.lock().unwrap().push(n);
                std::task::Poll::Ready(Ok(()))
            }
        }

        let sizes = Arc::new(Mutex::new(Vec::new()));
        let reader = RecordingReader {
            inner: std::io::Cursor::new(bytes.clone()),
            sizes: Arc::clone(&sizes),
        };
        let _ = load_document_async_streaming(reader)
            .await
            .expect("streaming load must succeed");

        let sizes = sizes.lock().unwrap().clone();
        // Strip 0-byte tail reads (EOF signals from read_to_end).
        let nonzero: Vec<usize> = sizes.into_iter().filter(|&n| n > 0).collect();

        // First read: the 16-byte IFF + FORM + form_type head.
        assert_eq!(nonzero[0], 16, "first read must be 16-byte IFF head");
        // Second read: the 8-byte DIRM chunk header.
        assert_eq!(nonzero[1], 8, "second read must be 8-byte chunk header");
        // Third read: the DIRM payload — must be smaller than the full body.
        assert!(
            nonzero[2] < bytes.len() / 4,
            "third read should be the DIRM payload, well under the full body \
             (got {} bytes for a {} byte file)",
            nonzero[2],
            bytes.len()
        );
    }

    /// I/O failure surfaces as `AsyncLoadError::Io`, not panic.
    #[tokio::test]
    async fn load_document_async_propagates_io_error() {
        struct FailingReader;
        impl tokio::io::AsyncRead for FailingReader {
            fn poll_read(
                self: std::pin::Pin<&mut Self>,
                _cx: &mut std::task::Context<'_>,
                _buf: &mut tokio::io::ReadBuf<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                std::task::Poll::Ready(Err(std::io::Error::other("simulated I/O failure")))
            }
        }
        let err = load_document_async(FailingReader)
            .await
            .expect_err("must fail on I/O error");
        assert!(
            matches!(err, AsyncLoadError::Io(_)),
            "expected Io error, got {err:?}"
        );
    }

    /// A JB2-only page (no BG44 chunks) yields exactly one frame.
    #[tokio::test]
    async fn progressive_stream_jb2_only_yields_one_frame() {
        use futures::StreamExt;
        let doc = load_doc("boy_jb2.djvu");
        let page = doc.page(0).unwrap();
        if !page.bg44_chunks().is_empty() {
            // Page is not JB2-only; skip
            return;
        }
        let opts = RenderOptions {
            width: 80,
            height: 60,
            ..Default::default()
        };

        let stream = render_progressive_stream(page, opts);
        futures::pin_mut!(stream);

        let mut count = 0;
        while let Some(result) = stream.next().await {
            result.expect("frame should succeed");
            count += 1;
        }
        assert_eq!(count, 1, "JB2-only page must yield exactly one frame");
    }
}
