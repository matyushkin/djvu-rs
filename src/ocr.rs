//! Pluggable OCR backend trait and error types.
//!
//! Provides [`OcrBackend`] — an abstraction over OCR engines (Tesseract, ONNX, Candle).
//! Each backend is gated behind its own feature flag:
//! - `ocr-tesseract` — system Tesseract via `tesseract-rs`
//! - `ocr-onnx` — ONNX models via `tract`
//! - `ocr-neural` — HuggingFace models via `candle`

use crate::pixmap::Pixmap;
use crate::text::TextLayer;

/// Error type for OCR operations.
#[derive(Debug, thiserror::Error)]
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

    /// I/O error (e.g. loading model file).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Configuration for an OCR run.
#[derive(Debug, Clone)]
pub struct OcrOptions {
    /// Languages to recognize (e.g. "eng", "rus+eng").
    pub languages: String,
    /// Page DPI (helps OCR engines scale internally).
    pub dpi: u32,
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
    /// Returns a [`TextLayer`] with zone hierarchy (page -> line -> word)
    /// and bounding boxes in the pixmap's coordinate system (top-left origin).
    fn recognize(&self, pixmap: &Pixmap, options: &OcrOptions) -> Result<TextLayer, OcrError>;
}
