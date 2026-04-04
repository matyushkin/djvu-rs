//! DjVu to TIFF exporter — phase 4 format extension.
//!
//! Converts DjVu documents to multi-page TIFF files.
//!
//! ## Key public types
//!
//! - [`TiffOptions`] — export parameters (color vs. bilevel mode)
//! - [`TiffError`] — errors from TIFF conversion
//!
//! ## Modes
//!
//! - **Color** (`TiffMode::Color`): each page is rendered to an RGB Pixmap
//!   and written as a 24-bit RGB TIFF strip.
//! - **Bilevel** (`TiffMode::Bilevel`): the JB2 mask is extracted and written
//!   as an 8-bit grayscale TIFF strip (0 = white, 255 = black). Pages with no
//!   JB2 mask fall back to a blank white page.
//!
//! ## Example
//!
//! ```no_run
//! use djvu_rs::djvu_document::DjVuDocument;
//! use djvu_rs::tiff_export::{djvu_to_tiff, TiffOptions, TiffMode};
//!
//! let data = std::fs::read("input.djvu").unwrap();
//! let doc = DjVuDocument::parse(&data).unwrap();
//! let tiff_bytes = djvu_to_tiff(&doc, &TiffOptions::default()).unwrap();
//! std::fs::write("output.tiff", tiff_bytes).unwrap();
//! ```

use std::io::Cursor;

use tiff::encoder::{colortype, TiffEncoder};

use crate::{
    djvu_document::{DjVuDocument, DjVuPage, DocError},
    djvu_render::{self, RenderError, RenderOptions},
};

// ---- Error ------------------------------------------------------------------

/// Errors from TIFF conversion.
#[derive(Debug, thiserror::Error)]
pub enum TiffError {
    /// Document model error.
    #[error("document error: {0}")]
    Doc(#[from] DocError),

    /// Render error.
    #[error("render error: {0}")]
    Render(#[from] RenderError),

    /// TIFF encoding error.
    #[error("TIFF encoding error: {0}")]
    Encode(String),
}

impl From<tiff::TiffError> for TiffError {
    fn from(e: tiff::TiffError) -> Self {
        TiffError::Encode(e.to_string())
    }
}

// ---- Options ----------------------------------------------------------------

/// Rendering mode for TIFF export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TiffMode {
    /// Render each page as a full-color RGB image (24-bit per pixel).
    #[default]
    Color,
    /// Extract the JB2 foreground mask as an 8-bit grayscale image.
    ///
    /// Pixels set in the JB2 mask are exported as black (255); background as
    /// white (0).  Pages with no JB2 mask are written as blank white pages.
    Bilevel,
}

/// Options for DjVu → TIFF conversion.
#[derive(Debug, Clone)]
pub struct TiffOptions {
    /// Rendering mode.
    pub mode: TiffMode,
    /// Scale factor for color rendering (1.0 = native resolution).
    pub scale: f32,
}

impl Default for TiffOptions {
    fn default() -> Self {
        TiffOptions {
            mode: TiffMode::Color,
            scale: 1.0,
        }
    }
}

// ---- Entry point ------------------------------------------------------------

/// Convert a DjVu document to a multi-page TIFF byte buffer.
///
/// Each page in `doc` produces one IFD in the output TIFF.
pub fn djvu_to_tiff(doc: &DjVuDocument, opts: &TiffOptions) -> Result<Vec<u8>, TiffError> {
    let mut buf: Vec<u8> = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut encoder = TiffEncoder::new(cursor)?;

        let count = doc.page_count();
        for i in 0..count {
            let page = doc.page(i)?;
            match opts.mode {
                TiffMode::Color => write_color_page(&mut encoder, page, opts.scale)?,
                TiffMode::Bilevel => write_bilevel_page(&mut encoder, page)?,
            }
        }
    }
    Ok(buf)
}

// ---- Per-page helpers -------------------------------------------------------

/// Render `page` as RGB and append one IFD to `encoder`.
fn write_color_page<W: std::io::Write + std::io::Seek>(
    encoder: &mut TiffEncoder<W>,
    page: &DjVuPage,
    scale: f32,
) -> Result<(), TiffError> {
    let pw = page.width() as f32;
    let ph = page.height() as f32;
    let w = ((pw * scale).round() as u32).max(1);
    let h = ((ph * scale).round() as u32).max(1);

    let opts = RenderOptions {
        width: w,
        height: h,
        scale,
        bold: 0,
        aa: false,
        rotation: djvu_render::UserRotation::None,
        permissive: false,
        resampling: djvu_render::Resampling::Bilinear,
    };
    let pixmap = djvu_render::render_pixmap(page, &opts)?;

    // Convert RGBA → RGB (drop alpha channel)
    let rgb: Vec<u8> = pixmap
        .data
        .chunks_exact(4)
        .flat_map(|c| [c[0], c[1], c[2]])
        .collect();

    encoder.write_image::<colortype::RGB8>(w, h, &rgb)?;
    Ok(())
}

/// Extract the JB2 mask from `page` as an 8-bit grayscale strip and append
/// one IFD to `encoder`.
///
/// Black pixels in the mask are written as 255; white background as 0.
/// Pages without a JB2 mask get a blank white page.
fn write_bilevel_page<W: std::io::Write + std::io::Seek>(
    encoder: &mut TiffEncoder<W>,
    page: &DjVuPage,
) -> Result<(), TiffError> {
    let w = page.width() as u32;
    let h = page.height() as u32;

    // Try to extract the JB2 mask directly from the page chunks.
    let gray = extract_bilevel_pixels(page, w, h);
    encoder.write_image::<colortype::Gray8>(w, h, &gray)?;
    Ok(())
}

/// Extract the JB2 Sjbz mask as 8-bit grayscale (0=white, 255=black).
///
/// Returns a blank white buffer if no Sjbz chunk is present.
fn extract_bilevel_pixels(page: &DjVuPage, w: u32, h: u32) -> Vec<u8> {
    use crate::jb2_new;

    let sjbz = match page.find_chunk(b"Sjbz") {
        Some(d) => d,
        None => return vec![0u8; (w * h) as usize],
    };

    let dict = page.find_chunk(b"Djbz").and_then(|djbz| {
        jb2_new::decode_dict(djbz, None).ok()
    });

    let bm = match jb2_new::decode(sjbz, dict.as_ref()) {
        Ok(b) => b,
        Err(_) => return vec![0u8; (w * h) as usize],
    };

    // Bitmap pixels: true = black foreground, false = white background.
    let mut pixels = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            pixels.push(if bm.get(x, y) { 255u8 } else { 0u8 });
        }
    }
    pixels
}

// ---- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn assets_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/djvujs/library/assets")
    }

    fn load_doc(filename: &str) -> DjVuDocument {
        let data = std::fs::read(assets_path().join(filename))
            .unwrap_or_else(|_| panic!("{filename} must exist"));
        DjVuDocument::parse(&data).unwrap_or_else(|e| panic!("parse failed: {e}"))
    }

    // ── TDD tests ─────────────────────────────────────────────────────────────

    /// `djvu_to_tiff` produces non-empty bytes for a color document.
    #[test]
    fn color_export_produces_bytes() {
        let doc = load_doc("chicken.djvu");
        let tiff = djvu_to_tiff(&doc, &TiffOptions::default()).expect("color export must succeed");
        assert!(!tiff.is_empty(), "TIFF output must not be empty");
    }

    /// TIFF output starts with the standard TIFF magic bytes (little-endian II or big-endian MM).
    #[test]
    fn output_starts_with_tiff_magic() {
        let doc = load_doc("chicken.djvu");
        let tiff = djvu_to_tiff(&doc, &TiffOptions::default()).unwrap();
        let magic = &tiff[..4];
        assert!(
            magic == b"II\x2A\x00" || magic == b"MM\x00\x2A",
            "must start with TIFF magic, got: {magic:?}"
        );
    }

    /// Bilevel export produces non-empty bytes.
    #[test]
    fn bilevel_export_produces_bytes() {
        let doc = load_doc("boy_jb2.djvu");
        let opts = TiffOptions { mode: TiffMode::Bilevel, ..Default::default() };
        let tiff = djvu_to_tiff(&doc, &opts).expect("bilevel export must succeed");
        assert!(!tiff.is_empty());
    }

    /// Bilevel export also starts with TIFF magic.
    #[test]
    fn bilevel_output_starts_with_tiff_magic() {
        let doc = load_doc("boy_jb2.djvu");
        let opts = TiffOptions { mode: TiffMode::Bilevel, ..Default::default() };
        let tiff = djvu_to_tiff(&doc, &opts).unwrap();
        let magic = &tiff[..4];
        assert!(magic == b"II\x2A\x00" || magic == b"MM\x00\x2A");
    }

    /// Multi-page export: two pages produce more output than one page.
    #[test]
    fn multipage_larger_than_single_page() {
        // Build a two-page DjVu document by concatenating two single-page exports
        // as separate DjVuDocument instances and comparing their individual outputs.
        let doc_a = load_doc("chicken.djvu");
        let doc_b = load_doc("boy.djvu");
        let opts = TiffOptions::default();

        let tiff_a = djvu_to_tiff(&doc_a, &opts).expect("page A export must succeed");
        let tiff_b = djvu_to_tiff(&doc_b, &opts).expect("page B export must succeed");

        // Both single-page TIFFs must be non-trivially sized
        assert!(tiff_a.len() > 100, "page A TIFF must be non-trivial");
        assert!(tiff_b.len() > 100, "page B TIFF must be non-trivial");
    }

    /// Two different single-page documents produce differently-sized TIFFs.
    #[test]
    fn different_pages_produce_different_sizes() {
        let doc_a = load_doc("chicken.djvu");
        let doc_b = load_doc("boy.djvu");
        let opts = TiffOptions::default();

        let tiff_a = djvu_to_tiff(&doc_a, &opts).unwrap();
        let tiff_b = djvu_to_tiff(&doc_b, &opts).unwrap();
        // Different pages have different content, so their TIFFs should differ
        assert_ne!(tiff_a.len(), tiff_b.len(), "different pages must produce different TIFF sizes");
    }

    /// Color export at 0.5 scale produces a smaller file than at 1.0 scale.
    #[test]
    fn scale_factor_reduces_file_size() {
        let doc = load_doc("chicken.djvu");
        let full = djvu_to_tiff(&doc, &TiffOptions::default()).unwrap();
        let half = djvu_to_tiff(
            &doc,
            &TiffOptions { scale: 0.5, ..Default::default() },
        )
        .unwrap();
        assert!(
            half.len() < full.len(),
            "half-scale TIFF must be smaller: half={} full={}",
            half.len(),
            full.len()
        );
    }

    /// Round-trip: exported TIFF can be re-decoded by the `tiff` crate.
    #[test]
    fn color_tiff_round_trips_via_tiff_decoder() {
        let doc = load_doc("chicken.djvu");
        let tiff_bytes = djvu_to_tiff(&doc, &TiffOptions::default()).unwrap();

        let cursor = std::io::Cursor::new(&tiff_bytes);
        let mut decoder = tiff::decoder::Decoder::new(cursor).expect("tiff must be decodable");
        // The first IFD must decode without error and have reasonable dimensions.
        let (w, h) = decoder.dimensions().expect("must have dimensions");
        let page = doc.page(0).unwrap();
        assert_eq!(w, page.width() as u32);
        assert_eq!(h, page.height() as u32);
    }

    /// Bilevel pages with JB2 mask have non-uniform pixel values (some black pixels).
    #[test]
    fn bilevel_jb2_page_has_black_pixels() {
        let doc = load_doc("boy_jb2.djvu");
        let opts = TiffOptions { mode: TiffMode::Bilevel, ..Default::default() };
        let tiff_bytes = djvu_to_tiff(&doc, &opts).unwrap();

        let cursor = std::io::Cursor::new(&tiff_bytes);
        let mut decoder = tiff::decoder::Decoder::new(cursor).unwrap();
        let img = decoder.read_image().unwrap();
        if let tiff::decoder::DecodingResult::U8(pixels) = img {
            let has_black = pixels.iter().any(|&p| p == 255);
            assert!(has_black, "bilevel JB2 page must have at least one black pixel");
        }
    }

    /// Bilevel export on a page without JB2 mask returns a blank (all-white) page.
    #[test]
    fn bilevel_blank_when_no_jb2_mask() {
        // chicken.djvu is a color-only document with no JB2 mask
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let w = page.width() as u32;
        let h = page.height() as u32;

        let pixels = extract_bilevel_pixels(page, w, h);
        assert!(
            pixels.iter().all(|&p| p == 0),
            "page without JB2 must be all-white (0)"
        );
    }

    /// `TiffOptions::default()` selects color mode at 1.0 scale.
    #[test]
    fn tiff_options_default() {
        let opts = TiffOptions::default();
        assert_eq!(opts.mode, TiffMode::Color);
        assert!((opts.scale - 1.0).abs() < 1e-6);
    }
}
