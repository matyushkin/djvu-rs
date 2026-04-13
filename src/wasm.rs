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
    djvu_render::{
        RenderOptions, Resampling, UserRotation, render_coarse, render_pixmap, render_progressive,
    },
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

    /// Return text zone data for this page, scaled to match a render at `target_dpi`.
    ///
    /// Returns a JSON string — array of `{"t":"…","x":N,"y":N,"w":N,"h":N}` objects,
    /// one per leaf text zone, with pixel coordinates identical to the canvas produced
    /// by `render(target_dpi)`.  Leaf zones are the finest granularity stored in the
    /// text layer (word-level for richly OCR'd files, line-level otherwise).
    ///
    /// Returns `null` if the page has no text layer.
    /// Throws a JavaScript `Error` on decode failure.
    pub fn text_zones_json(&self, target_dpi: u32) -> Result<Option<String>, JsError> {
        let page = self
            .doc
            .page(self.index)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let scale = target_dpi as f32 / page.dpi() as f32;
        let render_w = ((page.width() as f32 * scale).round() as u32).max(1);
        let render_h = ((page.height() as f32 * scale).round() as u32).max(1);

        let Some(layer) = page
            .text_layer_at_size(render_w, render_h)
            .map_err(|e| JsError::new(&e.to_string()))?
        else {
            return Ok(None);
        };

        let mut buf = String::from("[");
        let mut first = true;
        for zone in &layer.zones {
            collect_leaf_zones(zone, &mut buf, &mut first);
        }
        buf.push(']');
        Ok(Some(buf))
    }

    /// Number of BG44 background chunks on this page.
    ///
    /// Determines how many refinement steps are available via
    /// [`render_progressive`]. Returns `0` for bilevel-only pages.
    pub fn bg44_chunk_count(&self) -> u32 {
        self.doc
            .page(self.index)
            .map(|p| p.bg44_chunks().len() as u32)
            .unwrap_or(0)
    }

    /// Fast coarse render — decodes only the first BG44 chunk (~5 ms for a
    /// typical color page).
    ///
    /// Returns `null` for bilevel-only pages (no BG44 data); use
    /// [`render`] for those.  For color pages the result is a blurry but
    /// instantly visible preview; call [`render_progressive`] or [`render`]
    /// on a Web Worker to produce the final image.
    ///
    /// Throws on decode error.
    pub fn render_coarse(
        &self,
        target_dpi: u32,
    ) -> Result<Option<js_sys::Uint8ClampedArray>, JsError> {
        let page = self
            .doc
            .page(self.index)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let opts = render_opts_for_dpi(page, target_dpi);
        let pm = render_coarse(page, &opts).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(pm.map(|p| {
            let arr = js_sys::Uint8ClampedArray::new_with_length(p.data.len() as u32);
            arr.copy_from(&p.data);
            arr
        }))
    }

    /// Progressive render — decodes BG44 chunks 0..=`chunk_n` plus all
    /// foreground layers (JB2 mask, text).
    ///
    /// `chunk_n = 0` is equivalent to [`render_coarse`] but also composites
    /// the mask. Each subsequent call with `chunk_n += 1` adds one more
    /// wavelet refinement pass. After the last chunk the result is identical
    /// to [`render`].
    ///
    /// Use [`bg44_chunk_count`] to find the maximum valid `chunk_n`
    /// (`bg44_chunk_count() - 1`).
    ///
    /// Throws on decode error or if `chunk_n` is out of range.
    pub fn render_progressive(
        &self,
        target_dpi: u32,
        chunk_n: u32,
    ) -> Result<js_sys::Uint8ClampedArray, JsError> {
        let page = self
            .doc
            .page(self.index)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let opts = render_opts_for_dpi(page, target_dpi);
        let pm = render_progressive(page, &opts, chunk_n as usize)
            .map_err(|e| JsError::new(&e.to_string()))?;
        let arr = js_sys::Uint8ClampedArray::new_with_length(pm.data.len() as u32);
        arr.copy_from(&pm.data);
        Ok(arr)
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

        let opts = render_opts_for_dpi(page, target_dpi);
        let pm = render_pixmap(page, &opts).map_err(|e| JsError::new(&e.to_string()))?;
        // Allocate a new JS-side Uint8ClampedArray and copy the RGBA bytes
        // into it.  Using `Uint8ClampedArray::from(&[u8])` (which creates a
        // view into WASM linear memory) causes incorrect `length` values in
        // Node.js and with the externref ABI because the backing memory may
        // be freed before the caller reads the length.
        let arr = js_sys::Uint8ClampedArray::new_with_length(pm.data.len() as u32);
        arr.copy_from(&pm.data);
        Ok(arr)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build [`RenderOptions`] for a given page and target DPI.
///
/// AA is disabled so `pixels.length == width_at(dpi) * height_at(dpi) * 4`
/// always holds (see [`WasmPage::render`] for details).
fn render_opts_for_dpi(page: &crate::djvu_document::DjVuPage, target_dpi: u32) -> RenderOptions {
    let scale = target_dpi as f32 / page.dpi() as f32;
    let w = ((page.width() as f32 * scale).round() as u32).max(1);
    let h = ((page.height() as f32 * scale).round() as u32).max(1);
    RenderOptions {
        width: w,
        height: h,
        scale,
        bold: 0,
        aa: false,
        rotation: UserRotation::None,
        permissive: true,
        resampling: Resampling::Bilinear,
    }
}

// ── Text zone helpers ─────────────────────────────────────────────────────────

/// Recursively collect leaf zones (zones without children) into a JSON array.
fn collect_leaf_zones(zone: &crate::text::TextZone, buf: &mut String, first: &mut bool) {
    if zone.children.is_empty() {
        let t = zone.text.trim();
        if t.is_empty() {
            return;
        }
        if !*first {
            buf.push(',');
        }
        *first = false;
        buf.push_str("{\"t\":\"");
        json_escape_into(t, buf);
        buf.push_str("\",\"x\":");
        buf.push_str(&zone.rect.x.to_string());
        buf.push_str(",\"y\":");
        buf.push_str(&zone.rect.y.to_string());
        buf.push_str(",\"w\":");
        buf.push_str(&zone.rect.width.to_string());
        buf.push_str(",\"h\":");
        buf.push_str(&zone.rect.height.to_string());
        buf.push('}');
    } else {
        for child in &zone.children {
            collect_leaf_zones(child, buf, first);
        }
    }
}

/// Append `s` to `buf` with JSON string escaping (no surrounding quotes).
fn json_escape_into(s: &str, buf: &mut String) {
    for ch in s.chars() {
        match ch {
            '"' => buf.push_str("\\\""),
            '\\' => buf.push_str("\\\\"),
            '\n' => buf.push_str("\\n"),
            '\r' => buf.push_str("\\r"),
            '\t' => buf.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                buf.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => buf.push(c),
        }
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

    /// render_coarse on a color page returns Some with correct pixel count.
    #[test]
    fn wasm_render_coarse_color_page() {
        let bytes = boy_bytes();
        let doc = DjVuDocument::parse(&bytes).unwrap();
        let page = doc.page(0).unwrap();
        let scale = 150_f32 / page.dpi() as f32;
        let w = ((page.width() as f32 * scale).round() as u32).max(1);
        let h = ((page.height() as f32 * scale).round() as u32).max(1);
        let opts = render_opts_for_dpi(page, 150);
        let result = render_coarse(page, &opts).expect("render_coarse failed");
        // boy.djvu has BG44 chunks — coarse result must be Some
        let pm = result.expect("expected Some for color page");
        assert_eq!(pm.data.len(), (w * h * 4) as usize);
    }

    /// bg44_chunk_count is > 0 for a color page.
    #[test]
    fn wasm_bg44_chunk_count_color_page() {
        let bytes = boy_bytes();
        let doc = DjVuDocument::parse(&bytes).unwrap();
        assert!(doc.page(0).unwrap().bg44_chunks().len() > 0);
    }

    /// render_progressive with chunk_n = 0 succeeds and returns correct pixel count.
    #[test]
    fn wasm_render_progressive_chunk0() {
        let bytes = boy_bytes();
        let doc = DjVuDocument::parse(&bytes).unwrap();
        let page = doc.page(0).unwrap();
        let scale = 150_f32 / page.dpi() as f32;
        let w = ((page.width() as f32 * scale).round() as u32).max(1);
        let h = ((page.height() as f32 * scale).round() as u32).max(1);
        let opts = render_opts_for_dpi(page, 150);
        let pm = render_progressive(page, &opts, 0).expect("render_progressive failed");
        assert_eq!(pm.data.len(), (w * h * 4) as usize);
    }
}

// WASM browser tests (wasm-pack test --headless --firefox) are defined in
// tests/wasm_browser.rs to keep them separate from the build path.
// See examples/wasm/README.md for how to run them.
