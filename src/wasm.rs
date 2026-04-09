//! WebAssembly bindings for djvu-rs.
//!
//! Exposes a minimal browser-friendly API via `wasm-bindgen`:
//!
//! - [`WasmDocument`] — parsed DjVu document, created from raw bytes.
//! - [`WasmPage`]     — a single page, capable of rendering to RGBA pixels.
//!
//! ## Usage (JavaScript)
//!
//! ```js
//! import init, { WasmDocument } from 'djvu-rs';
//! await init();
//! const doc = WasmDocument.from_bytes(new Uint8Array(buffer));
//! const page = doc.page(0);
//! const pixels = page.render(150);   // Uint8ClampedArray, RGBA
//! const img = new ImageData(pixels, page.width_at(150), page.height_at(150));
//! ctx.putImageData(img, 0, 0);
//! ```

use std::sync::Arc;

use wasm_bindgen::prelude::*;

use crate::{
    djvu_document::DjVuDocument,
    djvu_render::{RenderOptions, Resampling, UserRotation, render_pixmap},
};

// ── WasmDocument ─────────────────────────────────────────────────────────────

/// A parsed DjVu document.
///
/// Created from raw bytes via [`WasmDocument::from_bytes`].
#[wasm_bindgen]
pub struct WasmDocument {
    inner: Arc<DjVuDocument>,
}

#[wasm_bindgen]
impl WasmDocument {
    /// Parse a DjVu document from a byte slice.
    ///
    /// Throws a JavaScript `Error` if the bytes are not a valid DjVu file.
    pub fn from_bytes(data: &[u8]) -> Result<WasmDocument, JsError> {
        let doc = DjVuDocument::parse(data).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(WasmDocument {
            inner: Arc::new(doc),
        })
    }

    /// Total number of pages in the document.
    pub fn page_count(&self) -> u32 {
        self.inner.page_count() as u32
    }

    /// Return a handle to page `index` (0-based).
    ///
    /// Throws if `index >= page_count()`.
    pub fn page(&self, index: u32) -> Result<WasmPage, JsError> {
        let count = self.inner.page_count();
        if index as usize >= count {
            return Err(JsError::new(&format!(
                "page index {index} out of range (document has {count} pages)"
            )));
        }
        Ok(WasmPage {
            doc: Arc::clone(&self.inner),
            index: index as usize,
        })
    }
}

// ── WasmPage ─────────────────────────────────────────────────────────────────

/// A single page within a [`WasmDocument`].
#[wasm_bindgen]
pub struct WasmPage {
    doc: Arc<DjVuDocument>,
    index: usize,
}

#[wasm_bindgen]
impl WasmPage {
    /// Native DPI stored in the INFO chunk.
    pub fn dpi(&self) -> u32 {
        self.doc
            .page(self.index)
            .map(|p| p.dpi() as u32)
            .unwrap_or(300)
    }

    /// Output width in pixels when rendered at `target_dpi`.
    pub fn width_at(&self, target_dpi: u32) -> u32 {
        self.doc
            .page(self.index)
            .map(|p| {
                let scale = target_dpi as f32 / p.dpi() as f32;
                ((p.width() as f32 * scale).round() as u32).max(1)
            })
            .unwrap_or(1)
    }

    /// Output height in pixels when rendered at `target_dpi`.
    pub fn height_at(&self, target_dpi: u32) -> u32 {
        self.doc
            .page(self.index)
            .map(|p| {
                let scale = target_dpi as f32 / p.dpi() as f32;
                ((p.height() as f32 * scale).round() as u32).max(1)
            })
            .unwrap_or(1)
    }

    /// Extract the plain text content of this page from the TXTz/TXTa layer.
    ///
    /// Returns `undefined` (JS `None`) if the page has no text layer.
    /// Throws a JavaScript `Error` on decode failure.
    pub fn text(&self) -> Result<Option<String>, JsError> {
        let page = self
            .doc
            .page(self.index)
            .map_err(|e| JsError::new(&e.to_string()))?;
        page.text().map_err(|e| JsError::new(&e.to_string()))
    }

    /// Render the page at `target_dpi` and return raw RGBA pixels
    /// (`Uint8ClampedArray`, suitable for `new ImageData(pixels, w, h)`).
    ///
    /// Throws on decode error.
    pub fn render(&self, target_dpi: u32) -> Result<js_sys::Uint8ClampedArray, JsError> {
        let page = self
            .doc
            .page(self.index)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let scale = target_dpi as f32 / page.dpi() as f32;
        let w = ((page.width() as f32 * scale).round() as u32).max(1);
        let h = ((page.height() as f32 * scale).round() as u32).max(1);

        let opts = RenderOptions {
            width: w,
            height: h,
            scale,
            bold: 0,
            aa: true,
            rotation: UserRotation::None,
            permissive: true,
            resampling: Resampling::Bilinear,
        };

        let pm = render_pixmap(page, &opts).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(js_sys::Uint8ClampedArray::from(pm.data.as_slice()))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────
//
// Native tests (`#[cfg(not(target_arch = "wasm32"))]`) exercise the underlying
// DjVuDocument/render_pixmap logic directly, bypassing JsError (which panics
// outside a WASM runtime).
//
// WASM tests (`#[cfg(target_arch = "wasm32")]`) use `#[wasm_bindgen_test]` and
// run with `wasm-pack test --node` or `--headless --firefox/chrome`.

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod native_tests {
    use super::*;

    fn boy_bytes() -> Vec<u8> {
        // boy.djvu: 192×256 px, 300 dpi, color IW44.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/boy.djvu");
        std::fs::read(&path).expect("boy.djvu not found in tests/fixtures/")
    }

    /// Valid DjVu bytes must parse to a 1-page document.
    #[test]
    fn wasm_document_from_bytes_page_count() {
        let bytes = boy_bytes();
        let doc = DjVuDocument::parse(&bytes).expect("parse failed");
        assert_eq!(doc.page_count(), 1);
    }

    /// Garbage bytes must produce a parse error.
    #[test]
    fn wasm_document_from_bytes_invalid_returns_error() {
        assert!(DjVuDocument::parse(b"not a djvu file").is_err());
    }

    /// Page 0 of boy.djvu must report 100 dpi (value in INFO chunk).
    #[test]
    fn wasm_page_dpi() {
        let bytes = boy_bytes();
        let doc = DjVuDocument::parse(&bytes).unwrap();
        assert_eq!(doc.page(0).unwrap().dpi(), 100);
    }

    /// boy.djvu (192×256 px @ 100 dpi) rendered at 50 dpi → 96×128 px.
    #[test]
    fn wasm_page_dimensions_at_dpi() {
        let bytes = boy_bytes();
        let doc = DjVuDocument::parse(&bytes).unwrap();
        let page = doc.page(0).unwrap();
        let scale = 50_f32 / page.dpi() as f32; // 50/100 = 0.5
        let w = ((page.width() as f32 * scale).round() as u32).max(1);
        let h = ((page.height() as f32 * scale).round() as u32).max(1);
        assert_eq!(w, 96);
        assert_eq!(h, 128);
    }

    /// Rendering must produce exactly w×h×4 RGBA bytes.
    #[test]
    fn wasm_page_render_pixel_count() {
        let bytes = boy_bytes();
        let doc = DjVuDocument::parse(&bytes).unwrap();
        let page = doc.page(0).unwrap();
        let scale = 150_f32 / page.dpi() as f32;
        let w = ((page.width() as f32 * scale).round() as u32).max(1);
        let h = ((page.height() as f32 * scale).round() as u32).max(1);
        let opts = RenderOptions {
            width: w,
            height: h,
            scale,
            bold: 0,
            aa: false,
            rotation: UserRotation::None,
            permissive: false,
            resampling: Resampling::Bilinear,
        };
        let pm = render_pixmap(page, &opts).expect("render failed");
        assert_eq!(pm.data.len(), (w * h * 4) as usize);
    }
}

// WASM browser tests (wasm-pack test --headless --firefox) are defined in
// tests/wasm_browser.rs to keep them separate from the build path.
// See examples/wasm/README.md for how to run them.
