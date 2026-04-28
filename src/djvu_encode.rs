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
//! - `Quality` from a [`Pixmap`]: ships `INFO + Sjbz + BG44…` —
//!   layered mask + sub-sampled IW44 background. Lossy by codec
//!   definition; output is decodable end-to-end. FG layer and FGbz
//!   palette are #220 follow-ups.
//! - `Archival`: still [`EncodeError::Unsupported`] — wants the per-CC
//!   profitability model from #194 Phase 2.5.
//! - `Lossless` from a [`Pixmap`] / `Quality` from a [`Bitmap`] are
//!   rejected: the combinations are mathematically meaningless
//!   (IW44 is lossy; bilevel input has nothing to put in BG44).

use crate::bitmap::Bitmap;
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
    /// (mask + sub-sampled IW44 background). FG layer and FGbz palette
    /// are #220 follow-ups.
    Quality,
    /// Archival profile with FGbz palette + aggressive lossy JB2
    /// refinement matching. *Not yet supported* — needs the per-CC
    /// profitability model (#194 Phase 2.5).
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
}

impl<'a> PageEncoder<'a> {
    /// Start encoding a bilevel page. Defaults: 300 dpi, `Lossless`.
    pub fn from_bitmap(bitmap: &'a Bitmap) -> Self {
        Self {
            source: Source::Bitmap(bitmap),
            dpi: 300,
            quality: EncodeQuality::Lossless,
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
            (Source::Pixmap(pm), EncodeQuality::Quality) => {
                let seg = segment_page(pm, &SegmentOptions::default());
                let sjbz = jb2_encode::encode_jb2(&seg.mask);
                let bg44_chunks = encode_iw44_color(&seg.bg, &Iw44EncodeOptions::default());
                let mut chunks = Vec::with_capacity(2 + bg44_chunks.len());
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
                Ok(encode_form_djvu(chunks))
            }
            (Source::Pixmap(_), EncodeQuality::Lossless) => Err(EncodeError::Unsupported(
                "Lossless requires bilevel input — use from_bitmap or switch to Quality",
            )),
            (Source::Bitmap(_), EncodeQuality::Quality) => Err(EncodeError::Unsupported(
                "Quality requires colour input — use from_pixmap or switch to Lossless",
            )),
            (_, EncodeQuality::Archival) => Err(EncodeError::Unsupported(
                "Archival profile requires the per-CC profitability model (#194 Phase 2.5)",
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
    }

    #[test]
    fn with_dpi_clamps_zero_to_one() {
        let bm = Bitmap::new(8, 8);
        let enc = PageEncoder::from_bitmap(&bm).with_dpi(0);
        assert_eq!(enc.dpi, 1);
    }

    #[test]
    fn archival_profile_still_unsupported() {
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
