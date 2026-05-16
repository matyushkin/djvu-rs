//! High-level page encoder — composes the codec primitives into a
//! complete `FORM:DJVU` page ready to wrap as a single-page document or
//! drop into a `FORM:DJVM` bundle.
//!
//! The encoder kit (`jb2_encode`, `iw44_encode`, `fgbz_encode`,
//! `smmr`, `bzz_encode`, `text_encode`, `navm_encode`) provides the
//! per-codec building blocks; this module orchestrates them so callers
//! don't have to hand-assemble IFF chunks.
//!
//! # Quick start
//!
//! Bilevel scan → single-page DjVu file:
//!
//! ```no_run
//! use djvu_rs::Bitmap;
//! use djvu_rs::djvu_encode::{PageEncoder, EncodeQuality};
//!
//! let mut bm = Bitmap::new(1024, 1280);
//! // … fill bm …
//! let bytes = PageEncoder::from_bitmap(&bm)
//!     .with_dpi(300)
//!     .with_quality(EncodeQuality::Lossless)
//!     .encode()
//!     .unwrap();
//! std::fs::write("scan.djvu", bytes).unwrap();
//! ```
//!
//! Color scan → layered DjVu (mask via JB2 + sub-sampled BG via IW44):
//!
//! ```no_run
//! use djvu_rs::Pixmap;
//! use djvu_rs::djvu_encode::{PageEncoder, EncodeQuality};
//!
//! let pm = Pixmap::white(1024, 1280);
//! let bytes = PageEncoder::from_pixmap(&pm)
//!     .with_dpi(300)
//!     .with_quality(EncodeQuality::Quality)
//!     .encode()
//!     .unwrap();
//! ```
//!
//! # Status
//!
//! - `Lossless` from a [`Bitmap`]: ships `INFO + Sjbz`. Pixel-exact.
//! - `Quality` from a [`Pixmap`]: ships `INFO + Sjbz + BG44… + FGbz`
//!   when foreground ink is detected. Lossy by codec definition; output
//!   is decodable end-to-end.
//! - `Archival` from a [`Pixmap`]: same layered chunk shape as `Quality`,
//!   with a denser background sample grid. This is a conservative archival
//!   profile, not a DjVuLibre-equivalent color text optimiser.
//! - `Lossless` from a [`Pixmap`] / `Quality` from a [`Bitmap`] are
//!   rejected: the combinations are mathematically meaningless
//!   (IW44 is lossy; bilevel input has nothing to put in BG44).

use crate::bitmap::Bitmap;
use crate::fgbz_encode::{FgbzColor, encode_fgbz};
use crate::iff::{Chunk, DjvuFile, emit};
use crate::iw44_encode::{Iw44EncodeOptions, encode_iw44_color};
use crate::jb2_encode;
use crate::pixmap::Pixmap;
use crate::segment::{SegmentOptions, segment_page};

// ── Errors ────────────────────────────────────────────────────────────────────

/// Errors returned by [`PageEncoder::encode`].
#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    /// The requested combination of input + quality profile is not
    /// implemented yet. The message names the missing dependency
    /// (typically a sibling issue tracking the codec layer).
    #[error("page encoder: {0}")]
    Unsupported(&'static str),
}

// ── Quality profile ───────────────────────────────────────────────────────────

/// Encoder quality profile.
///
/// The profile drives codec selection (JB2 vs IW44, mask-only vs
/// layered, optional FGbz palette) and quality knobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EncodeQuality {
    /// Pixel-exact round-trip. Requires bilevel input
    /// ([`PageEncoder::from_bitmap`]); writes `INFO + Sjbz` (JB2).
    #[default]
    Lossless,
    /// Layered foreground/background encoding. Requires color input
    /// ([`PageEncoder::from_pixmap`]); writes `INFO + Sjbz + BG44…`
    /// plus `FGbz` when foreground ink is detected.
    Quality,
    /// Conservative archival color profile. Requires color input; writes
    /// the same layered chunks as `Quality`, but keeps a denser background
    /// sample grid. Bilevel input should use `Lossless`.
    Archival,
}

// ── Encoder ──────────────────────────────────────────────────────────────────

enum Source<'a> {
    Bitmap(&'a Bitmap),
    Pixmap(&'a Pixmap),
}

impl Source<'_> {
    fn dimensions(&self) -> (u32, u32) {
        match self {
            Source::Bitmap(b) => (b.width, b.height),
            Source::Pixmap(p) => (p.width, p.height),
        }
    }
}

/// Builder-style page encoder.
///
/// Constructed from a [`Bitmap`] (bilevel) or [`Pixmap`] (RGBA) and
/// configured via the `with_*` methods, then finalised with
/// [`encode`](Self::encode).
pub struct PageEncoder<'a> {
    source: Source<'a>,
    dpi: u16,
    quality: EncodeQuality,
    segment_options: Option<SegmentOptions>,
}

impl<'a> PageEncoder<'a> {
    /// Start encoding a bilevel page. Defaults: 300 dpi, `Lossless`.
    pub fn from_bitmap(bitmap: &'a Bitmap) -> Self {
        Self {
            source: Source::Bitmap(bitmap),
            dpi: 300,
            quality: EncodeQuality::Lossless,
            segment_options: None,
        }
    }

    /// Start encoding a colour page. Defaults: 300 dpi, `Quality` (the
    /// only sensible profile for colour input — `Lossless` requires a
    /// `Bitmap`).
    pub fn from_pixmap(pixmap: &'a Pixmap) -> Self {
        Self {
            source: Source::Pixmap(pixmap),
            dpi: 300,
            quality: EncodeQuality::Quality,
            segment_options: None,
        }
    }

    /// Set the page resolution stored in the `INFO` chunk.
    ///
    /// Clamped to `[1, 65 535]` (the wire-format range of the dpi
    /// field). Values outside that range are silently saturated.
    pub fn with_dpi(mut self, dpi: u16) -> Self {
        self.dpi = dpi.max(1);
        self
    }

    /// Select an encoding profile. See [`EncodeQuality`] for the
    /// per-variant trade-offs and current support status.
    pub fn with_quality(mut self, quality: EncodeQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Override the segmentation knobs used by `Quality` / `Archival` color
    /// encodes. Defaults remain profile-specific and fixed-threshold.
    pub fn with_segment_options(mut self, opts: SegmentOptions) -> Self {
        self.segment_options = Some(opts);
        self
    }

    /// Produce the bytes of a single-page DjVu file (`FORM:DJVU`
    /// wrapped in the `AT&T` IFF container).
    pub fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let (w, h) = self.source.dimensions();
        let w = u16::try_from(w).map_err(|_| {
            EncodeError::Unsupported("page width exceeds INFO chunk limit (65 535 px)")
        })?;
        let h = u16::try_from(h).map_err(|_| {
            EncodeError::Unsupported("page height exceeds INFO chunk limit (65 535 px)")
        })?;
        let info = encode_info(w, h, self.dpi);

        match (&self.source, self.quality) {
            (Source::Bitmap(bm), EncodeQuality::Lossless) => Ok(encode_form_djvu(vec![
                Chunk::Leaf {
                    id: *b"INFO",
                    data: info,
                },
                Chunk::Leaf {
                    id: *b"Sjbz",
                    data: jb2_encode::encode_jb2(bm),
                },
            ])),
            (Source::Pixmap(pm), EncodeQuality::Quality | EncodeQuality::Archival) => {
                let segment_options = self.segment_options.unwrap_or_else(|| match self.quality {
                    EncodeQuality::Quality => SegmentOptions::default(),
                    EncodeQuality::Archival => SegmentOptions {
                        bg_subsample: 6,
                        ..SegmentOptions::default()
                    },
                    EncodeQuality::Lossless => unreachable!(),
                });
                let seg = segment_page(pm, &segment_options);
                // Use the dictionary encoder for color profiles so FGbz can
                // address foreground colors per blitted component.
                let sjbz = jb2_encode::encode_jb2_dict(&seg.mask);
                let bg44_chunks = encode_iw44_color(&seg.bg, &Iw44EncodeOptions::default());
                let fgbz = foreground_fgbz(pm, &seg.mask, &sjbz);

                let mut chunks =
                    Vec::with_capacity(2 + bg44_chunks.len() + usize::from(fgbz.is_some()));
                chunks.push(Chunk::Leaf {
                    id: *b"INFO",
                    data: info,
                });
                chunks.push(Chunk::Leaf {
                    id: *b"Sjbz",
                    data: sjbz,
                });
                for body in bg44_chunks {
                    chunks.push(Chunk::Leaf {
                        id: *b"BG44",
                        data: body,
                    });
                }
                if let Some(data) = fgbz {
                    chunks.push(Chunk::Leaf { id: *b"FGbz", data });
                }
                Ok(encode_form_djvu(chunks))
            }
            (Source::Pixmap(_), EncodeQuality::Lossless) => Err(EncodeError::Unsupported(
                "Lossless requires bilevel input — use from_bitmap or switch to Quality",
            )),
            (Source::Bitmap(_), EncodeQuality::Quality) => Err(EncodeError::Unsupported(
                "Quality requires colour input — use from_pixmap or switch to Lossless",
            )),
            (Source::Bitmap(_), EncodeQuality::Archival) => Err(EncodeError::Unsupported(
                "Archival requires colour input — use from_pixmap or switch to Lossless",
            )),
        }
    }
}

// ── Internal helpers ─────────────────────────────────────────────────────────

fn encode_form_djvu(children: Vec<Chunk>) -> Vec<u8> {
    let file = DjvuFile {
        root: Chunk::Form {
            secondary_id: *b"DJVU",
            length: 0, // recomputed by emit
            children,
        },
    };
    emit(&file)
}

/// Build the 10-byte `INFO` chunk body.
///
/// Mirrors the layout parsed by `crate::info::PageInfo::parse` — note
/// the mixed endianness: width/height are big-endian, dpi is
/// little-endian (per DjVu spec).
fn encode_info(width: u16, height: u16, dpi: u16) -> Vec<u8> {
    let mut b = vec![0u8; 10];
    b[0..2].copy_from_slice(&width.to_be_bytes());
    b[2..4].copy_from_slice(&height.to_be_bytes());
    b[4] = 0x18; // minor version
    b[5] = 0x00; // major version
    b[6..8].copy_from_slice(&dpi.to_le_bytes()); // dpi: little-endian
    b[8] = 22; // gamma byte: 22 → 2.2
    b[9] = 0x00; // flags: no rotation
    b
}

#[derive(Debug, Clone, Copy, Default)]
struct ColorAccum {
    r: u64,
    g: u64,
    b: u64,
    n: u64,
}

impl ColorAccum {
    fn add(&mut self, r: u8, g: u8, b: u8) {
        self.r += u64::from(r);
        self.g += u64::from(g);
        self.b += u64::from(b);
        self.n += 1;
    }

    fn color(self) -> Option<FgbzColor> {
        if self.n == 0 {
            return None;
        }
        Some(FgbzColor {
            r: (self.r / self.n) as u8,
            g: (self.g / self.n) as u8,
            b: (self.b / self.n) as u8,
        })
    }
}

fn foreground_fgbz(pm: &Pixmap, mask: &Bitmap, sjbz: &[u8]) -> Option<Vec<u8>> {
    let (decoded_mask, blit_map) = crate::jb2::decode_indexed(sjbz, None).ok()?;
    if decoded_mask.width != mask.width || decoded_mask.height != mask.height {
        return None;
    }

    let max_blit = blit_map.iter().copied().filter(|&i| i >= 0).max()? as usize;
    let mut by_blit = vec![ColorAccum::default(); max_blit + 1];
    let w = mask.width as usize;
    for y in 0..mask.height {
        for x in 0..mask.width {
            if mask.get(x, y) {
                let idx = y as usize * w + x as usize;
                let blit_idx = blit_map.get(idx).copied().unwrap_or(-1);
                if blit_idx < 0 {
                    continue;
                }
                let (pr, pg, pb) = pm.get_rgb(x, y);
                by_blit[blit_idx as usize].add(pr, pg, pb);
            }
        }
    }

    let mut palette: Vec<FgbzColor> = Vec::new();
    let mut indices: Vec<i16> = Vec::with_capacity(by_blit.len());
    for accum in by_blit {
        let color = accum.color().unwrap_or_default();
        let color_idx = match palette.iter().position(|&c| c == color) {
            Some(i) => i,
            None => {
                if palette.len() >= i16::MAX as usize {
                    return None;
                }
                palette.push(color);
                palette.len() - 1
            }
        };
        indices.push(color_idx as i16);
    }

    if palette.is_empty() || palette.iter().all(|c| c.r == 0 && c.g == 0 && c.b == 0) {
        return None;
    }

    let index_payload = if palette.len() > 1 {
        Some(indices.as_slice())
    } else {
        None
    };
    Some(encode_fgbz(&palette, index_payload))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iff::parse_form;
    use crate::jb2;

    fn checkerboard(w: u32, h: u32) -> Bitmap {
        let mut bm = Bitmap::new(w, h);
        for y in 0..h {
            for x in 0..w {
                if (x + y) % 2 == 0 {
                    bm.set_black(x, y);
                }
            }
        }
        bm
    }

    #[test]
    fn lossless_bilevel_round_trips() {
        let bm = checkerboard(64, 48);
        let bytes = PageEncoder::from_bitmap(&bm)
            .with_dpi(150)
            .with_quality(EncodeQuality::Lossless)
            .encode()
            .expect("encode");

        let form = parse_form(&bytes).expect("parse_form");
        assert_eq!(&form.form_type, b"DJVU");

        let mut info_data: Option<&[u8]> = None;
        let mut sjbz_data: Option<&[u8]> = None;
        for chunk in &form.chunks {
            match &chunk.id {
                b"INFO" => info_data = Some(chunk.data),
                b"Sjbz" => sjbz_data = Some(chunk.data),
                _ => {}
            }
        }
        let info = info_data.expect("INFO chunk present");
        let sjbz = sjbz_data.expect("Sjbz chunk present");

        assert_eq!(u16::from_be_bytes([info[0], info[1]]), 64);
        assert_eq!(u16::from_be_bytes([info[2], info[3]]), 48);
        assert_eq!(u16::from_le_bytes([info[6], info[7]]), 150);

        let decoded = jb2::decode(sjbz, None).expect("jb2 decode");
        assert_eq!(decoded.width, bm.width);
        assert_eq!(decoded.height, bm.height);
        for y in 0..bm.height {
            for x in 0..bm.width {
                assert_eq!(decoded.get(x, y), bm.get(x, y), "mismatch at ({x},{y})");
            }
        }
    }

    #[test]
    fn defaults_are_300_dpi_lossless_for_bitmap() {
        let bm = Bitmap::new(8, 8);
        let enc = PageEncoder::from_bitmap(&bm);
        assert_eq!(enc.dpi, 300);
        assert_eq!(enc.quality, EncodeQuality::Lossless);
    }

    #[test]
    fn defaults_are_300_dpi_quality_for_pixmap() {
        let pm = Pixmap::white(8, 8);
        let enc = PageEncoder::from_pixmap(&pm);
        assert_eq!(enc.dpi, 300);
        assert_eq!(enc.quality, EncodeQuality::Quality);
        assert!(enc.segment_options.is_none());
    }

    #[test]
    fn with_dpi_clamps_zero_to_one() {
        let bm = Bitmap::new(8, 8);
        let enc = PageEncoder::from_bitmap(&bm).with_dpi(0);
        assert_eq!(enc.dpi, 1);
    }

    #[test]
    fn archival_bitmap_rejected() {
        let bm = Bitmap::new(16, 16);
        let err = PageEncoder::from_bitmap(&bm)
            .with_quality(EncodeQuality::Archival)
            .encode()
            .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("Archival"));
    }

    #[test]
    fn empty_bitmap_round_trips() {
        let bm = Bitmap::new(1, 1);
        let bytes = PageEncoder::from_bitmap(&bm).encode().expect("encode");
        let form = parse_form(&bytes).expect("parse");
        assert_eq!(&form.form_type, b"DJVU");
    }

    #[test]
    fn quality_color_emits_info_sjbz_bg44() {
        // 64×64 page: white background with a black 16×16 ink square.
        let mut pm = Pixmap::white(64, 64);
        for y in 16..32 {
            for x in 16..32 {
                pm.set_rgb(x, y, 0, 0, 0);
            }
        }

        let bytes = PageEncoder::from_pixmap(&pm)
            .with_dpi(200)
            .with_quality(EncodeQuality::Quality)
            .encode()
            .expect("encode");

        let form = parse_form(&bytes).expect("parse_form");
        assert_eq!(&form.form_type, b"DJVU");

        let mut has_info = false;
        let mut has_sjbz = false;
        let mut bg44_count = 0;
        for chunk in &form.chunks {
            match &chunk.id {
                b"INFO" => has_info = true,
                b"Sjbz" => has_sjbz = true,
                b"BG44" => bg44_count += 1,
                _ => {}
            }
        }
        assert!(has_info, "INFO chunk missing");
        assert!(has_sjbz, "Sjbz chunk missing");
        assert!(
            bg44_count > 0,
            "expected at least one BG44 chunk, got {bg44_count}"
        );
    }

    #[test]
    fn quality_color_emits_fgbz_for_colored_foreground() {
        let mut pm = Pixmap::white(64, 64);
        for y in 16..32 {
            for x in 16..32 {
                pm.set_rgb(x, y, 180, 20, 20);
            }
        }

        let bytes = PageEncoder::from_pixmap(&pm)
            .with_quality(EncodeQuality::Quality)
            .encode()
            .expect("encode");

        let form = parse_form(&bytes).expect("parse_form");
        let fgbz = form
            .chunks
            .iter()
            .find(|chunk| &chunk.id == b"FGbz")
            .expect("FGbz chunk present");
        let (palette, indices) = crate::fgbz_encode::decode_fgbz(fgbz.data).expect("decode FGbz");
        assert_eq!(palette.len(), 1);
        assert!(indices.is_empty());
        assert!(palette[0].r > 0, "foreground red should be preserved");
    }

    #[test]
    fn quality_color_emits_per_blit_fgbz_indices() {
        let mut pm = Pixmap::white(80, 40);
        for y in 8..24 {
            for x in 8..24 {
                pm.set_rgb(x, y, 180, 20, 20);
            }
            for x in 48..64 {
                pm.set_rgb(x, y, 20, 40, 180);
            }
        }

        let bytes = PageEncoder::from_pixmap(&pm)
            .with_quality(EncodeQuality::Quality)
            .encode()
            .expect("encode");
        let doc = crate::djvu_document::DjVuDocument::parse(&bytes).expect("parse");
        let page = doc.page(0).expect("page");
        let fgbz = page.raw_chunk(b"FGbz").expect("FGbz present");
        let (palette, indices) = crate::fgbz_encode::decode_fgbz(fgbz).expect("decode FGbz");

        assert!(
            palette.len() >= 2,
            "expected at least two foreground colors, got {palette:?}"
        );
        assert!(
            indices.len() >= 2,
            "expected per-blit indices for two foreground components"
        );
        assert_ne!(
            indices[0], indices[1],
            "separate colored components should point at distinct palette entries"
        );

        let rendered = crate::Document::from_bytes(bytes)
            .expect("document")
            .page(0)
            .expect("page")
            .render()
            .expect("render");
        let left = rendered.get_rgb(12, 12);
        let right = rendered.get_rgb(52, 12);
        assert!(
            left.0 > left.2,
            "left foreground should render red-dominant, got {left:?}"
        );
        assert!(
            right.2 > right.0,
            "right foreground should render blue-dominant, got {right:?}"
        );
    }

    #[test]
    fn quality_color_accepts_adaptive_segment_options() {
        let pm = mixed_lighting_fixture();

        let bytes = PageEncoder::from_pixmap(&pm)
            .with_quality(EncodeQuality::Quality)
            .with_segment_options(adaptive_segment_options())
            .encode()
            .expect("encode");

        let doc = crate::djvu_document::DjVuDocument::parse(&bytes).expect("parse");
        let page = doc.page(0).expect("page");
        assert!(page.raw_chunk(b"Sjbz").is_some());
        assert!(!page.all_chunks(b"BG44").is_empty());
    }

    #[test]
    fn adaptive_segment_options_improve_decoded_mixed_lighting_fixture() {
        let pm = mixed_lighting_fixture();
        let fixed = PageEncoder::from_pixmap(&pm)
            .with_quality(EncodeQuality::Quality)
            .with_segment_options(SegmentOptions {
                bg_subsample: 6,
                ..SegmentOptions::default()
            })
            .encode()
            .expect("fixed encode");
        let adaptive = PageEncoder::from_pixmap(&pm)
            .with_quality(EncodeQuality::Quality)
            .with_segment_options(SegmentOptions {
                bg_subsample: 6,
                ..adaptive_segment_options()
            })
            .encode()
            .expect("adaptive encode");

        let fixed_render = render_encoded(&fixed);
        let adaptive_render = render_encoded(&adaptive);
        let fixed_err = mean_abs_rgb_diff(&pm, &fixed_render);
        let adaptive_err = mean_abs_rgb_diff(&pm, &adaptive_render);

        assert!(
            adaptive_err < fixed_err * 0.70,
            "adaptive decoded render should be closer to source ({adaptive_err:.2} vs {fixed_err:.2})"
        );
    }

    fn adaptive_segment_options() -> SegmentOptions {
        SegmentOptions {
            binarization: crate::segment::Binarization::Sauvola { window: 9, k: 0.34 },
            bg_inpaint: true,
            ..SegmentOptions::default()
        }
    }

    fn mixed_lighting_fixture() -> Pixmap {
        let mut pm = Pixmap::white(48, 24);
        for y in 0..24 {
            for x in 0..48 {
                let v = if x < 24 { 80 } else { 220 };
                pm.set_rgb(x, y, v, v, v);
            }
        }

        // Dark ink on dark paper.
        for y in 6..18 {
            pm.set_rgb(9, y, 40, 40, 40);
            pm.set_rgb(14, y, 40, 40, 40);
        }
        for x in 9..=14 {
            pm.set_rgb(x, 6, 40, 40, 40);
            pm.set_rgb(x, 12, 40, 40, 40);
        }

        // Light-gray ink on bright paper. Fixed threshold treats this as BG,
        // so the thin strokes wash into the BG44 sample cells.
        for y in 6..18 {
            pm.set_rgb(33, y, 140, 140, 140);
            pm.set_rgb(40, y, 140, 140, 140);
        }
        for x in 33..=40 {
            pm.set_rgb(x, 6, 140, 140, 140);
            pm.set_rgb(x, 12, 140, 140, 140);
            pm.set_rgb(x, 17, 140, 140, 140);
        }
        pm
    }

    fn render_encoded(bytes: &[u8]) -> Pixmap {
        let doc = crate::djvu_document::DjVuDocument::parse(bytes).expect("parse encoded doc");
        let page = doc.page(0).expect("page");
        let (width, height) = page.dimensions();
        let opts = crate::djvu_render::RenderOptions {
            width: u32::from(width),
            height: u32::from(height),
            ..crate::djvu_render::RenderOptions::default()
        };
        crate::djvu_render::render_pixmap(page, &opts).expect("render encoded page")
    }

    fn mean_abs_rgb_diff(expected: &Pixmap, actual: &Pixmap) -> f64 {
        assert_eq!(
            (expected.width, expected.height),
            (actual.width, actual.height)
        );
        let mut sum = 0u64;
        let mut n = 0u64;
        for (a, b) in expected
            .data
            .chunks_exact(4)
            .zip(actual.data.chunks_exact(4))
        {
            for c in 0..3 {
                sum += a[c].abs_diff(b[c]) as u64;
                n += 1;
            }
        }
        sum as f64 / n as f64
    }

    #[test]
    fn archival_color_emits_layered_djvu_with_fgbz() {
        let mut pm = Pixmap::white(48, 48);
        for y in 12..24 {
            for x in 12..24 {
                pm.set_rgb(x, y, 0, 90, 180);
            }
        }

        let bytes = PageEncoder::from_pixmap(&pm)
            .with_quality(EncodeQuality::Archival)
            .encode()
            .expect("encode");

        let doc = crate::djvu_document::DjVuDocument::parse(&bytes).expect("parse");
        let page = doc.page(0).expect("page");
        assert!(page.raw_chunk(b"Sjbz").is_some());
        assert!(!page.all_chunks(b"BG44").is_empty());
        assert!(page.raw_chunk(b"FGbz").is_some());
    }

    #[test]
    fn lossless_pixmap_rejected() {
        let pm = Pixmap::white(8, 8);
        let err = PageEncoder::from_pixmap(&pm)
            .with_quality(EncodeQuality::Lossless)
            .encode()
            .unwrap_err();
        assert!(format!("{err}").contains("Lossless"));
    }

    #[test]
    fn quality_bitmap_rejected() {
        let bm = Bitmap::new(8, 8);
        let err = PageEncoder::from_bitmap(&bm)
            .with_quality(EncodeQuality::Quality)
            .encode()
            .unwrap_err();
        assert!(format!("{err}").contains("Quality"));
    }

    #[test]
    fn quality_color_round_trips_through_document() {
        // End-to-end: encode a colour page at Quality, parse it back
        // through the high-level Document API, and confirm dimensions
        // + that the page has both an Sjbz and at least one BG44 chunk.
        let pm = Pixmap::white(32, 24);
        let bytes = PageEncoder::from_pixmap(&pm)
            .with_dpi(150)
            .with_quality(EncodeQuality::Quality)
            .encode()
            .expect("encode");

        let doc = crate::djvu_document::DjVuDocument::parse(&bytes).expect("parse");
        let page = doc.page(0).expect("page 0");
        assert_eq!(page.width(), 32);
        assert_eq!(page.height(), 24);
        assert_eq!(page.dpi(), 150);
        assert!(page.raw_chunk(b"Sjbz").is_some());
        assert!(!page.all_chunks(b"BG44").is_empty());
    }
}
