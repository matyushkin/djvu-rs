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
//! # Status
//!
//! v1 ships the **`Lossless` profile for bilevel input only**: writes
//! `INFO + Sjbz` into a `FORM:DJVU`. The other profiles
//! (`Quality`, `Archival`) and color/gray input require the FG/BG
//! segmentation pass tracked by issue #220 and the FG44 layered-mask
//! integration; they currently return [`EncodeError::Unsupported`].

use crate::bitmap::Bitmap;
use crate::iff::{Chunk, DjvuFile, emit};
use crate::jb2_encode;

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
/// layered, optional FGbz palette) and quality knobs. Sizes are
/// indicative — actual output depends heavily on input content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EncodeQuality {
    /// Pixel-exact round-trip. For bilevel input this is `INFO + Sjbz`
    /// (JB2). For color/gray input — *not yet supported* (would emit
    /// IW44 at maximum quality, which is still mathematically lossy;
    /// genuine lossless color requires a different codec path).
    #[default]
    Lossless,
    /// Layered foreground/background encoding tuned for readable text
    /// at low bit rates. *Not yet supported* — needs FG/BG
    /// segmentation (#220).
    Quality,
    /// Archival profile with FGbz palette + aggressive lossy JB2
    /// refinement matching. *Not yet supported* — needs #220 + the
    /// per-CC profitability model (#194 Phase 2.5).
    Archival,
}

// ── Encoder ──────────────────────────────────────────────────────────────────

/// Builder-style page encoder.
///
/// Constructed from a [`Bitmap`] (bilevel) and configured via the
/// `with_*` methods, then finalised with [`encode`](Self::encode).
pub struct PageEncoder<'a> {
    bitmap: &'a Bitmap,
    dpi: u16,
    quality: EncodeQuality,
}

impl<'a> PageEncoder<'a> {
    /// Start encoding a bilevel page. Defaults: 300 dpi, `Lossless`.
    pub fn from_bitmap(bitmap: &'a Bitmap) -> Self {
        Self {
            bitmap,
            dpi: 300,
            quality: EncodeQuality::Lossless,
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
        match self.quality {
            EncodeQuality::Lossless => self.encode_lossless_bilevel(),
            EncodeQuality::Quality => Err(EncodeError::Unsupported(
                "Quality profile requires FG/BG segmentation (#220) and layered mask integration",
            )),
            EncodeQuality::Archival => Err(EncodeError::Unsupported(
                "Archival profile requires FG/BG segmentation (#220) plus the per-CC profitability model (#194 Phase 2.5)",
            )),
        }
    }

    fn encode_lossless_bilevel(&self) -> Result<Vec<u8>, EncodeError> {
        let w = u16::try_from(self.bitmap.width).map_err(|_| {
            EncodeError::Unsupported("page width exceeds INFO chunk limit (65 535 px)")
        })?;
        let h = u16::try_from(self.bitmap.height).map_err(|_| {
            EncodeError::Unsupported("page height exceeds INFO chunk limit (65 535 px)")
        })?;

        let info = encode_info(w, h, self.dpi);
        let sjbz = jb2_encode::encode_jb2(self.bitmap);

        let file = DjvuFile {
            root: Chunk::Form {
                secondary_id: *b"DJVU",
                length: 0, // recomputed by emit
                children: vec![
                    Chunk::Leaf {
                        id: *b"INFO",
                        data: info,
                    },
                    Chunk::Leaf {
                        id: *b"Sjbz",
                        data: sjbz,
                    },
                ],
            },
        };
        Ok(emit(&file))
    }
}

// ── Internal helpers ─────────────────────────────────────────────────────────

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

        // Parse back
        let form = parse_form(&bytes).expect("parse_form");
        assert_eq!(&form.form_type, b"DJVU");

        // Find INFO + Sjbz
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

        // INFO dimensions
        assert_eq!(u16::from_be_bytes([info[0], info[1]]), 64);
        assert_eq!(u16::from_be_bytes([info[2], info[3]]), 48);
        assert_eq!(u16::from_le_bytes([info[6], info[7]]), 150);

        // Sjbz round-trip → bit-exact bitmap
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
    fn defaults_are_300_dpi_lossless() {
        let bm = Bitmap::new(8, 8);
        let enc = PageEncoder::from_bitmap(&bm);
        assert_eq!(enc.dpi, 300);
        assert_eq!(enc.quality, EncodeQuality::Lossless);
    }

    #[test]
    fn with_dpi_clamps_zero_to_one() {
        let bm = Bitmap::new(8, 8);
        let enc = PageEncoder::from_bitmap(&bm).with_dpi(0);
        assert_eq!(enc.dpi, 1);
    }

    #[test]
    fn quality_profile_unsupported_until_segmentation() {
        let bm = Bitmap::new(16, 16);
        let err = PageEncoder::from_bitmap(&bm)
            .with_quality(EncodeQuality::Quality)
            .encode()
            .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("Quality"));
    }

    #[test]
    fn archival_profile_unsupported_until_segmentation() {
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
}
