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

use crate::{
    djvu_document::DjVuPage,
    djvu_render::{self, RenderError, RenderOptions},
    pixmap::{GrayPixmap, Pixmap},
};

// ── Error type ────────────────────────────────────────────────────────────────

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
    let page = page.clone();
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
    let page = page.clone();
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
    let page = page.clone();
    let n_chunks = page.bg44_chunks().len();

    async_stream::stream! {
        if n_chunks == 0 {
            // No BG44 chunks (JB2-only page): yield a single frame.
            let page = page.clone();
            let opts = opts.clone();
            let result = tokio::task::spawn_blocking(move || {
                djvu_render::render_pixmap(&page, &opts).map_err(AsyncRenderError::Render)
            })
            .await
            .map_err(|e| AsyncRenderError::Join(e.to_string()));
            yield result.and_then(|r| r);
        } else {
            for chunk_n in 0..n_chunks {
                let page = page.clone();
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
