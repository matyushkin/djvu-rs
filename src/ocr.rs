//! Pluggable OCR backend trait and error types.
//!
//! Provides [`OcrBackend`] — an abstraction over OCR engines.
//!
//! Backends, each behind its own feature:
//! - `ocr-tesseract` — system Tesseract via `tesseract-rs`. This is the
//!   supported backend.
//! - `ocr-onnx` — **experimental** neural PP-OCR pipeline on `tract`, with
//!   pinned, SHA-256-verified models. It ignores [`OcrOptions`].
//!
//! ## Design
//!
//! The trait is a deliberate runtime seam, not a speculative abstraction: the
//! `djvu ocr --backend` CLI selector builds a `Box<dyn OcrBackend>` and drives
//! it polymorphically. It is retained by decision. See
//! `docs/ocr-backend-seam.md` (issue #382) for the rationale and the deferred
//! plan for `OcrOptions`.
//!
//! [`OcrBackend`]: crate::ocr::OcrBackend
//! [`OcrOptions`]: crate::ocr::OcrOptions

use crate::pixmap::Pixmap;
use crate::text::TextLayer;

/// Error type for OCR operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OcrError {
    /// The OCR engine failed to initialize.
    #[error("OCR init failed: {0}")]
    InitFailed(String),

    /// Recognition failed on a page image.
    #[error("OCR recognition failed: {0}")]
    RecognitionFailed(String),

    /// The specified language or model is not available.
    #[error("OCR model/language not found: {0}")]
    ModelNotFound(String),

    /// A model file was found but failed the backend's integrity check, so it
    /// was not loaded.
    ///
    /// A backend that pins its model files (by size, checksum or signature)
    /// returns this instead of running unverified weights. It is a hard error,
    /// not a warning: fetch the model again rather than retrying.
    #[error("model verification failed for '{name}': {detail}")]
    ModelVerificationFailed {
        /// Name of the model the backend tried to load.
        name: String,
        /// What did not match, with the expected and actual values.
        detail: String,
    },

    /// I/O error (e.g. loading model file).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Configuration for an OCR run.
///
/// These are **advisory hints**: each backend honours the fields that apply to
/// it and ignores the rest. The Tesseract backend uses both; an ONNX model fixes
/// its own input size and vocabulary at load time and ignores them. A
/// model-neutral recast is deferred until a second CLI-live backend needs to be
/// configured through the trait — see `docs/ocr-backend-seam.md`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct OcrOptions {
    /// Languages to recognize (e.g. "eng", "rus+eng"). Advisory; ignored by
    /// backends whose model is not language-parameterized.
    pub languages: String,
    /// Page DPI (helps OCR engines scale internally). Advisory; ignored by
    /// backends with a fixed input resolution.
    pub dpi: u32,
}

impl OcrOptions {
    /// Sets [`languages`](Self::languages).
    #[must_use]
    pub fn with_languages(mut self, languages: String) -> Self {
        self.languages = languages;
        self
    }

    /// Sets [`dpi`](Self::dpi).
    #[must_use]
    pub fn with_dpi(mut self, dpi: u32) -> Self {
        self.dpi = dpi;
        self
    }
}

impl Default for OcrOptions {
    fn default() -> Self {
        Self {
            languages: "eng".into(),
            dpi: 300,
        }
    }
}

/// Trait for pluggable OCR backends.
///
/// Implementations receive a rendered page pixmap and return a structured
/// text layer that can be written back into the DjVu file as a TXTz chunk.
pub trait OcrBackend {
    /// Recognize text in the given page image.
    ///
    /// Returns a [`TextLayer`] whose bounding boxes are in the pixmap's
    /// coordinate system (top-left origin).
    ///
    /// **Minimum granularity guarantee.** Callers may rely only on a populated
    /// top-level [`TextLayer`]`::text` string and at least one page-level zone;
    /// the richer `page -> line -> word` hierarchy is best-effort and
    /// backend-dependent. The Tesseract backend produces the full hierarchy;
    /// the `ocr-onnx` neural pipeline emits `page -> line -> word` with
    /// *heuristic* word rects (proportional split of the line box); other
    /// experimental backends (see the module docs) may emit a coarser tree or
    /// none at all. Consumers that need word-level rects (e.g. the hOCR/ALTO
    /// exporters) must tolerate a flatter layer.
    fn recognize(&self, pixmap: &Pixmap, options: &OcrOptions) -> Result<TextLayer, OcrError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocr_options_default_values() {
        let opts = OcrOptions::default();
        assert_eq!(opts.languages, "eng");
        assert_eq!(opts.dpi, 300);
    }
}
