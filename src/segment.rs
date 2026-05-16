//! Photometric foreground/background segmentation.
//!
//! Splits a full-resolution RGBA [`Pixmap`] into a bilevel mask and a
//! sub-sampled background pixmap, the inputs the layered DjVu encoder
//! needs for `Sjbz` + `BG44` (and eventually `FG44` / `FGbz`).
//!
//! The default remains the original deterministic fixed-luminance threshold.
//! Optional knobs add adaptive Sauvola binarisation and conservative background
//! inpainting for fully masked BG blocks.

use crate::bitmap::Bitmap;
use crate::pixmap::Pixmap;

/// Binarisation method used by [`segment_page`].
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Binarization {
    /// Fixed BT.601 luminance threshold from [`SegmentOptions::threshold`].
    #[default]
    Fixed,
    /// Sauvola local adaptive threshold.
    ///
    /// `window` is clamped to at least 3 pixels. `k` is typically in
    /// `0.2..=0.5`; non-finite values fall back to `0.34`.
    Sauvola { window: u32, k: f32 },
}

/// Knobs for [`segment_page`].
#[derive(Debug, Clone, Copy)]
pub struct SegmentOptions {
    /// Luminance cut-off for fixed-threshold masks: pixels with `Y < threshold`
    /// become foreground (black, `1`). BT.601 weights.
    ///
    /// Ignored by [`Binarization::Sauvola`].
    pub threshold: u8,
    /// Background sub-sample factor — output BG dimensions are
    /// `ceil(width / bg_subsample) × ceil(height / bg_subsample)`.
    /// Saturated to `>= 1`. DjVuLibre default: 12.
    pub bg_subsample: u32,
    /// Mask-generation method. Defaults to [`Binarization::Fixed`] to preserve
    /// the deterministic historical encoder output.
    pub binarization: Binarization,
    /// When true, a BG block that is fully covered by foreground mask is filled
    /// from the nearest neighbouring unmasked pixels instead of falling back to
    /// the masked block mean. This prevents solid ink from becoming a black BG
    /// cell under text strokes.
    pub bg_inpaint: bool,
}

impl Default for SegmentOptions {
    fn default() -> Self {
        Self {
            threshold: 128,
            bg_subsample: 12,
            binarization: Binarization::Fixed,
            bg_inpaint: false,
        }
    }
}

/// Result of [`segment_page`].
pub struct SegmentedPage {
    /// Full-resolution bilevel mask. `true` = foreground/ink.
    pub mask: Bitmap,
    /// Sub-sampled background pixmap, mean-colour per block of the non-mask
    /// source pixels. Fully masked blocks either fall back to their full-block
    /// mean (default) or, with [`SegmentOptions::bg_inpaint`], to neighbouring
    /// unmasked pixels.
    pub bg: Pixmap,
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

    fn color(self) -> Option<(u8, u8, u8)> {
        if self.n == 0 {
            return None;
        }
        Some((
            (self.r / self.n) as u8,
            (self.g / self.n) as u8,
            (self.b / self.n) as u8,
        ))
    }
}

#[inline]
fn luminance(r: u8, g: u8, b: u8) -> u8 {
    (((r as u32) * 306 + (g as u32) * 601 + (b as u32) * 117) >> 10) as u8
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

    match opts.binarization {
        Binarization::Fixed => fill_fixed_mask(&mut mask, rgba, opts.threshold),
        Binarization::Sauvola { window, k } => {
            let luma = luminance_plane(rgba);
            fill_sauvola_mask(&mut mask, &luma, w, h, window, k);
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

            let color = block_mean(rgba, &mask, x0, x1, y0, y1, true)
                .or_else(|| {
                    opts.bg_inpaint
                        .then(|| inpaint_block_mean(rgba, &mask, bx, by, sub, bw, bh))
                        .flatten()
                })
                .or_else(|| block_mean(rgba, &mask, x0, x1, y0, y1, false))
                .unwrap_or((255, 255, 255));
            bg.set_rgb(bx, by, color.0, color.1, color.2);
        }
    }

    SegmentedPage { mask, bg }
}

fn luminance_plane(rgba: &Pixmap) -> Vec<u8> {
    let mut luma = Vec::with_capacity((rgba.width * rgba.height) as usize);
    for y in 0..rgba.height {
        for x in 0..rgba.width {
            let (r, g, b) = rgba.get_rgb(x, y);
            luma.push(luminance(r, g, b));
        }
    }
    luma
}

fn fill_fixed_mask(mask: &mut Bitmap, rgba: &Pixmap, threshold: u8) {
    let threshold = u32::from(threshold);
    for y in 0..mask.height {
        for x in 0..mask.width {
            let (r, g, b) = rgba.get_rgb(x, y);
            if u32::from(luminance(r, g, b)) < threshold {
                mask.set(x, y, true);
            }
        }
    }
}

fn fill_sauvola_mask(mask: &mut Bitmap, luma: &[u8], w: u32, h: u32, window: u32, k: f32) {
    let window = window.max(3);
    let radius = window / 2;
    let k = if k.is_finite() { k } else { 0.34 };
    let k = k.clamp(0.0, 1.0);
    let (sum, sum_sq) = integral_luma(luma, w, h);
    let stride = w as usize + 1;

    for y in 0..h {
        let y0 = y.saturating_sub(radius);
        let y1 = (y + radius + 1).min(h);
        for x in 0..w {
            let x0 = x.saturating_sub(radius);
            let x1 = (x + radius + 1).min(w);
            let area = f64::from((x1 - x0) * (y1 - y0));
            let s = rect_sum(&sum, stride, x0, y0, x1, y1) as f64;
            let ss = rect_sum(&sum_sq, stride, x0, y0, x1, y1) as f64;
            let mean = s / area;
            let variance = (ss / area - mean * mean).max(0.0);
            let stddev = variance.sqrt();
            let threshold = mean * (1.0 + f64::from(k) * (stddev / 128.0 - 1.0));
            let idx = (y * w + x) as usize;
            if f64::from(luma[idx]) < threshold {
                mask.set(x, y, true);
            }
        }
    }
}

fn integral_luma(luma: &[u8], w: u32, h: u32) -> (Vec<u64>, Vec<u64>) {
    let stride = w as usize + 1;
    let len = stride * (h as usize + 1);
    let mut sum = vec![0u64; len];
    let mut sum_sq = vec![0u64; len];

    for y in 0..h as usize {
        let mut row_sum = 0u64;
        let mut row_sum_sq = 0u64;
        for x in 0..w as usize {
            let v = u64::from(luma[y * w as usize + x]);
            row_sum += v;
            row_sum_sq += v * v;
            let dst = (y + 1) * stride + x + 1;
            sum[dst] = sum[dst - stride] + row_sum;
            sum_sq[dst] = sum_sq[dst - stride] + row_sum_sq;
        }
    }

    (sum, sum_sq)
}

fn rect_sum(integral: &[u64], stride: usize, x0: u32, y0: u32, x1: u32, y1: u32) -> u64 {
    let (x0, y0, x1, y1) = (x0 as usize, y0 as usize, x1 as usize, y1 as usize);
    integral[y1 * stride + x1] + integral[y0 * stride + x0]
        - integral[y0 * stride + x1]
        - integral[y1 * stride + x0]
}

fn block_mean(
    rgba: &Pixmap,
    mask: &Bitmap,
    x0: u32,
    x1: u32,
    y0: u32,
    y1: u32,
    unmasked_only: bool,
) -> Option<(u8, u8, u8)> {
    let mut acc = ColorAccum::default();
    for y in y0..y1 {
        for x in x0..x1 {
            if unmasked_only && mask.get(x, y) {
                continue;
            }
            let (r, g, b) = rgba.get_rgb(x, y);
            acc.add(r, g, b);
        }
    }
    acc.color()
}

fn inpaint_block_mean(
    rgba: &Pixmap,
    mask: &Bitmap,
    bx: u32,
    by: u32,
    sub: u32,
    bw: u32,
    bh: u32,
) -> Option<(u8, u8, u8)> {
    let max_radius = bw.max(bh);
    for radius in 1..=max_radius {
        let bx0 = bx.saturating_sub(radius);
        let by0 = by.saturating_sub(radius);
        let bx1 = (bx + radius + 1).min(bw);
        let by1 = (by + radius + 1).min(bh);
        let mut acc = ColorAccum::default();

        for ny in by0..by1 {
            for nx in bx0..bx1 {
                let dx = nx.abs_diff(bx);
                let dy = ny.abs_diff(by);
                if dx.max(dy) != radius {
                    continue;
                }
                let x0 = nx * sub;
                let x1 = (x0 + sub).min(rgba.width);
                let y0 = ny * sub;
                let y1 = (y0 + sub).min(rgba.height);
                for y in y0..y1 {
                    for x in x0..x1 {
                        if !mask.get(x, y) {
                            let (r, g, b) = rgba.get_rgb(x, y);
                            acc.add(r, g, b);
                        }
                    }
                }
            }
        }

        if let Some(color) = acc.color() {
            return Some(color);
        }
    }
    None
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
        // Block fully masked → default remains the historical full-block mean.
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
                ..SegmentOptions::default()
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
                ..SegmentOptions::default()
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
    fn sauvola_handles_dark_background_and_light_ink() {
        // Synthetic mixed scan strip: left half is dark paper, right half is
        // bright paper. A fixed 128 threshold masks the dark paper and misses
        // the light-gray ink; Sauvola keys off local contrast instead.
        let mut pm = Pixmap::white(16, 8);
        for y in 0..8 {
            for x in 0..16 {
                let v = if x < 8 { 80 } else { 220 };
                pm.set_rgb(x, y, v, v, v);
            }
        }
        pm.set_rgb(3, 3, 40, 40, 40);
        pm.set_rgb(11, 3, 140, 140, 140);

        let fixed = segment_page(&pm, &SegmentOptions::default());
        let adaptive = segment_page(
            &pm,
            &SegmentOptions {
                binarization: Binarization::Sauvola { window: 7, k: 0.34 },
                ..SegmentOptions::default()
            },
        );

        let fixed_count = count_mask(&fixed.mask);
        let adaptive_count = count_mask(&adaptive.mask);
        assert!(fixed_count > 50, "fixed threshold masks the dark paper");
        assert!(
            adaptive_count < fixed_count / 2,
            "adaptive mask should be much sparser than fixed ({adaptive_count} vs {fixed_count})"
        );
        assert!(adaptive.mask.get(3, 3), "dark ink on dark paper");
        assert!(adaptive.mask.get(11, 3), "light ink on light paper");
        assert!(!adaptive.mask.get(1, 1), "dark paper is background");
        assert!(!adaptive.mask.get(9, 1), "bright paper is background");
    }

    fn count_mask(mask: &Bitmap) -> u32 {
        let mut n = 0;
        for y in 0..mask.height {
            for x in 0..mask.width {
                n += u32::from(mask.get(x, y));
            }
        }
        n
    }

    #[test]
    fn inpaint_fully_masked_bg_block_from_neighbors() {
        let mut pm = Pixmap::white(8, 4);
        for y in 0..4 {
            for x in 0..4 {
                pm.set_rgb(x, y, 0, 0, 0);
            }
            for x in 4..8 {
                pm.set_rgb(x, y, 210, 200, 160);
            }
        }

        let opts = SegmentOptions {
            threshold: 128,
            bg_subsample: 4,
            bg_inpaint: true,
            ..SegmentOptions::default()
        };
        let seg = segment_page(&pm, &opts);
        assert_eq!(seg.bg.width, 2);
        assert_eq!(seg.bg.height, 1);
        assert_eq!(seg.bg.get_rgb(0, 0), (210, 200, 160));
        assert_eq!(seg.bg.get_rgb(1, 0), (210, 200, 160));
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
                ..SegmentOptions::default()
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
                ..SegmentOptions::default()
            },
        );
        assert_eq!(seg.bg.width, 3);
        assert_eq!(seg.bg.height, 3);
    }
}
