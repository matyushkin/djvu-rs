//! Integration tests for the Tesseract OCR backend.
//!
//! These tests require the `ocr-tesseract` feature and a working Tesseract
//! installation with English language data (`tesseract-ocr-eng`).

#[cfg(feature = "ocr-tesseract")]
mod tesseract_tests {
    use std::path::PathBuf;

    use djvu_rs::{
        DjVuDocument,
        djvu_render::{RenderOptions, render_pixmap},
        ocr::{OcrBackend, OcrOptions},
        ocr_tesseract::TesseractBackend,
    };

    fn assets_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/djvujs/library/assets")
    }

    #[test]
    fn tesseract_recognize_returns_ok() {
        // Render the first page of a small test file and run OCR on it.
        let data = std::fs::read(assets_path().join("boy.djvu")).expect("boy.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("parse must succeed");
        let page = doc.page(0).expect("page 0 must exist");
        let opts = RenderOptions {
            width: page.width() as u32,
            height: page.height() as u32,
            ..RenderOptions::default()
        };
        let pixmap = render_pixmap(page, &opts).expect("render must succeed");

        let backend = TesseractBackend::new();
        let ocr_opts = OcrOptions::default();
        let result = backend.recognize(&pixmap, &ocr_opts);
        assert!(
            result.is_ok(),
            "TesseractBackend::recognize must not return an error: {:?}",
            result.err()
        );
    }

    #[test]
    fn tesseract_result_has_text_layer() {
        let data = std::fs::read(assets_path().join("boy.djvu")).expect("boy.djvu must exist");
        let doc = DjVuDocument::parse(&data).expect("parse must succeed");
        let page = doc.page(0).expect("page 0 must exist");
        let opts = RenderOptions {
            width: page.width() as u32,
            height: page.height() as u32,
            ..RenderOptions::default()
        };
        let pixmap = render_pixmap(page, &opts).expect("render must succeed");

        let backend = TesseractBackend::new();
        let layer = backend
            .recognize(&pixmap, &OcrOptions::default())
            .expect("recognize must succeed");

        // text field is present (may be empty for a photographic image)
        let _ = layer.text;
        // zones field is present and does not panic
        let _ = layer.zones;
    }
}
