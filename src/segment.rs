//! Photometric foreground/background segmentation.
//!
//! Splits a full-resolution RGBA [`Pixmap`] into a bilevel mask and a
//! sub-sampled background pixmap, the inputs the layered DjVu encoder
//! needs for `Sjbz` + `BG44` (and eventually `FG44` / `FGbz`).
//!
//! # v1 scope
//!
//! - Mask: simple BT.601 luminance threshold. Pixels darker than
//!   `opts.threshold` become `1` (black, "ink") in the returned
//!   [`Bitmap`]. The result feeds the JB2 encoder unchanged.
//! - Background: block-averaged at `opts.bg_subsample` (DjVuLibre's
//!   default is 12). Each output pixel is the mean of the non-mask
//!   source pixels in its source block; blocks that are 100% mask fall
//!   back to the unmasked block mean (no inpainting yet).
//!
//! Adaptive binarisation (Sauvola/Wolf), FG palette extraction, and
//! mask-hole inpainting are tracked as #220 follow-ups — they slot in
//! behind the same [`SegmentOptions`] surface so callers don't need to
//! change.

use crate::bitmap::Bitmap;
use crate::pixmap::Pixmap;

/// Knobs for [`segment_page`].
#[derive(Debug, Clone, Copy)]
pub struct SegmentOptions {
    /// Luminance cut-off for the mask: pixels with `Y < threshold`
    /// become foreground (black, `1`). BT.601 weights.
    pub threshold: u8,
    /// Background sub-sample factor — output BG dimensions are
    /// `ceil(width / bg_subsample) × ceil(height / bg_subsample)`.
    /// Saturated to `>= 1`. DjVuLibre default: 12.
    pub bg_subsample: u32,
}

impl Default for SegmentOptions {
    fn default() -> Self {
        Self {
            threshold: 128,
            bg_subsample: 12,
        }
    }
}

/// Result of [`segment_page`].
pub struct SegmentedPage {
    /// Full-resolution bilevel mask. `true` = foreground/ink.
    pub mask: Bitmap,
    /// Sub-sampled background pixmap, mean-colour per block of the
    /// non-mask source pixels (or the full-block mean where the block
    /// is entirely masked).
    pub bg: Pixmap,
}

/// Segment an RGBA page into a bilevel mask + sub-sampled background.
///
/// Empty input (`width == 0` or `height == 0`) returns empty outputs.
pub fn segment_page(rgba: &Pixmap, opts: &SegmentOptions) -> SegmentedPage {
    let w = rgba.width;
    let h = rgba.height;
    let sub = opts.bg_subsample.max(1);

    let mut mask = Bitmap::new(w, h);
    if w == 0 || h == 0 {
        return SegmentedPage {
            mask,
            bg: Pixmap::default(),
        };
    }

    let threshold = opts.threshold as u32;
    for y in 0..h {
        for x in 0..w {
            let (r, g, b) = rgba.get_rgb(x, y);
            // BT.601 fixed-point luminance, matches Pixmap::to_gray8.
            let lum = ((r as u32) * 306 + (g as u32) * 601 + (b as u32) * 117) >> 10;
            if lum < threshold {
                mask.set(x, y, true);
            }
        }
    }

    let bw = w.div_ceil(sub);
    let bh = h.div_ceil(sub);
    let mut bg = Pixmap::white(bw, bh);

    for by in 0..bh {
        let y0 = by * sub;
        let y1 = (y0 + sub).min(h);
        for bx in 0..bw {
            let x0 = bx * sub;
            let x1 = (x0 + sub).min(w);

            let (mut r_unmasked, mut g_unmasked, mut b_unmasked, mut n_unmasked) =
                (0u32, 0u32, 0u32, 0u32);
            let (mut r_all, mut g_all, mut b_all, mut n_all) = (0u32, 0u32, 0u32, 0u32);

            for y in y0..y1 {
                for x in x0..x1 {
                    let (r, g, b) = rgba.get_rgb(x, y);
                    r_all += r as u32;
                    g_all += g as u32;
                    b_all += b as u32;
                    n_all += 1;
                    if !mask.get(x, y) {
                        r_unmasked += r as u32;
                        g_unmasked += g as u32;
                        b_unmasked += b as u32;
                        n_unmasked += 1;
                    }
                }
            }

            let (r, g, b) = if n_unmasked > 0 {
                (
                    (r_unmasked / n_unmasked) as u8,
                    (g_unmasked / n_unmasked) as u8,
                    (b_unmasked / n_unmasked) as u8,
                )
            } else if n_all > 0 {
                (
                    (r_all / n_all) as u8,
                    (g_all / n_all) as u8,
                    (b_all / n_all) as u8,
                )
            } else {
                (255, 255, 255)
            };
            bg.set_rgb(bx, by, r, g, b);
        }
    }

    SegmentedPage { mask, bg }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fill(pm: &mut Pixmap, r: u8, g: u8, b: u8) {
        for y in 0..pm.height {
            for x in 0..pm.width {
                pm.set_rgb(x, y, r, g, b);
            }
        }
    }

    #[test]
    fn all_white_page_yields_empty_mask() {
        let pm = Pixmap::white(24, 24);
        let seg = segment_page(&pm, &SegmentOptions::default());
        assert_eq!(seg.mask.width, 24);
        assert_eq!(seg.mask.height, 24);
        for y in 0..24 {
            for x in 0..24 {
                assert!(
                    !seg.mask.get(x, y),
                    "white pixel at ({x},{y}) should not be mask"
                );
            }
        }
        assert_eq!(seg.bg.width, 2);
        assert_eq!(seg.bg.height, 2);
        for chunk in seg.bg.data.chunks_exact(4) {
            assert_eq!(&chunk[..3], &[255, 255, 255]);
        }
    }

    #[test]
    fn all_black_page_yields_full_mask_and_black_bg_fallback() {
        let mut pm = Pixmap::white(12, 12);
        fill(&mut pm, 0, 0, 0);
        let seg = segment_page(&pm, &SegmentOptions::default());
        for y in 0..12 {
            for x in 0..12 {
                assert!(seg.mask.get(x, y));
            }
        }
        // Block fully masked → falls back to full-block mean (here all black).
        assert_eq!(seg.bg.width, 1);
        assert_eq!(seg.bg.height, 1);
        assert_eq!(&seg.bg.data[..3], &[0, 0, 0]);
    }

    #[test]
    fn threshold_boundary_is_strict() {
        let mut pm = Pixmap::white(4, 1);
        // Set lums: 0, 127, 128, 255 (gray triples)
        pm.set_rgb(0, 0, 0, 0, 0);
        pm.set_rgb(1, 0, 127, 127, 127);
        pm.set_rgb(2, 0, 128, 128, 128);
        pm.set_rgb(3, 0, 255, 255, 255);
        let seg = segment_page(
            &pm,
            &SegmentOptions {
                threshold: 128,
                bg_subsample: 1,
            },
        );
        assert!(seg.mask.get(0, 0));
        assert!(seg.mask.get(1, 0));
        assert!(!seg.mask.get(2, 0));
        assert!(!seg.mask.get(3, 0));
    }

    #[test]
    fn bg_excludes_mask_pixels() {
        // 4x4 block, sub=4: 1 ink pixel (value 0) in a sea of pale yellow
        // (BT.601 lum ≈ 222, above default threshold). Unmasked mean must
        // equal the BG colour exactly, not be pulled toward 0.
        let mut pm = Pixmap::white(4, 4);
        fill(&mut pm, 240, 230, 100);
        pm.set_rgb(1, 1, 0, 0, 0);
        let seg = segment_page(
            &pm,
            &SegmentOptions {
                threshold: 128,
                bg_subsample: 4,
            },
        );
        assert!(seg.mask.get(1, 1));
        assert!(!seg.mask.get(0, 0));
        assert_eq!(seg.bg.width, 1);
        assert_eq!(seg.bg.height, 1);
        let (r, g, b) = (seg.bg.data[0], seg.bg.data[1], seg.bg.data[2]);
        assert_eq!(
            (r, g, b),
            (240, 230, 100),
            "ink pixel should not contaminate BG mean"
        );
    }

    #[test]
    fn empty_input_returns_empty_outputs() {
        let pm = Pixmap::default();
        let seg = segment_page(&pm, &SegmentOptions::default());
        assert_eq!(seg.mask.width, 0);
        assert_eq!(seg.mask.height, 0);
        assert_eq!(seg.bg.width, 0);
        assert_eq!(seg.bg.height, 0);
    }

    #[test]
    fn bg_dims_round_up() {
        let pm = Pixmap::white(13, 7);
        let seg = segment_page(
            &pm,
            &SegmentOptions {
                threshold: 128,
                bg_subsample: 12,
            },
        );
        assert_eq!(seg.bg.width, 2);
        assert_eq!(seg.bg.height, 1);
    }

    #[test]
    fn bg_subsample_zero_is_clamped_to_one() {
        let pm = Pixmap::white(3, 3);
        let seg = segment_page(
            &pm,
            &SegmentOptions {
                threshold: 128,
                bg_subsample: 0,
            },
        );
        assert_eq!(seg.bg.width, 3);
        assert_eq!(seg.bg.height, 3);
    }
}
