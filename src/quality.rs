//! Perceptual image-quality metrics (PSNR, SSIM) for render experiments.
//!
//! The existing pixel-diff tooling (`examples/interop_pixdiff.rs`) reports the
//! *arithmetic* per-pixel RGB difference distribution versus DjVuLibre. That
//! answers "did the output drift?" but not "is it perceptually better/worse?",
//! which is exactly the judgement the deferred quality experiments need
//! (linear-light blending, mask-upscale AA, Lanczos-vs-area-avg downscale,
//! bicubic FG upsampling, gamma-correct downscale — see PERF_EXPERIMENTS.md
//! round-5 QUALITY_AA and the round-8 QUALITY-GATED list).
//!
//! This module adds the two standard perceptual metrics so a change can be
//! judged against a reference image with a single number:
//!
//! - [`psnr`] — peak signal-to-noise ratio in dB. Higher is closer; ∞ (returned
//!   as [`f64::INFINITY`]) for identical inputs. Correlates loosely with quality
//!   but is a pure error metric.
//! - [`ssim`] — structural similarity index in `[-1, 1]` (1.0 = identical).
//!   Models luminance, contrast and structure the way the human visual system
//!   roughly does, so it catches blur / ringing / blockiness that PSNR misses.
//!
//! Both operate on the **luma** of RGBA [`Pixmap`]s (alpha ignored) so colour
//! and grayscale renders compare on the same perceptual channel. A convenience
//! [`compare`] returns both plus MSE in one pass-pair.
//!
//! # Reference-vs-ideal workflow
//!
//! The most useful mode for an *intentional* quality change is to compare the
//! render against the **pre-compression source**, not against another decoder:
//! take a known image, encode it to DjVu, render it back, and measure SSIM/PSNR
//! against the original. A change that raises SSIM-vs-source is genuinely better,
//! independent of whether it drifts from DjVuLibre. `examples/quality_harness.rs`
//! drives exactly this.

#[cfg(feature = "std")]
use crate::pixmap::{GrayPixmap, Pixmap};

/// Rec.601 luma of an RGB triple, matching [`Pixmap::to_gray8`]'s fixed-point
/// weights (306 + 601 + 117 = 1024).
#[inline]
fn luma(r: u8, g: u8, b: u8) -> f64 {
    (r as u32 * 306 + g as u32 * 601 + b as u32 * 117) as f64 / 1024.0
}

/// Extract the luma plane (`f64`, one sample per pixel) from an RGBA byte buffer.
#[cfg(feature = "std")]
fn luma_plane(rgba: &[u8]) -> Vec<f64> {
    rgba.chunks_exact(4)
        .map(|p| luma(p[0], p[1], p[2]))
        .collect()
}

/// A one-shot quality comparison of two same-size images.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QualityReport {
    /// Mean squared error over the luma channel (0.0 = identical).
    pub mse: f64,
    /// Peak signal-to-noise ratio in dB ([`f64::INFINITY`] if identical).
    pub psnr_db: f64,
    /// Structural similarity index in `[-1, 1]` (1.0 = identical).
    pub ssim: f64,
}

/// Mean squared error between two equal-length luma planes.
fn mse_planes(a: &[f64], b: &[f64]) -> f64 {
    debug_assert_eq!(a.len(), b.len());
    if a.is_empty() {
        return 0.0;
    }
    let sum: f64 = a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum();
    sum / a.len() as f64
}

/// PSNR in dB from a mean squared error over an 8-bit signal (peak = 255).
///
/// Returns [`f64::INFINITY`] when `mse == 0` (identical images).
pub fn psnr_from_mse(mse: f64) -> f64 {
    if mse <= 0.0 {
        return f64::INFINITY;
    }
    10.0 * (255.0f64 * 255.0 / mse).log10()
}

/// Windowed SSIM over two equal-size luma planes.
///
/// Uses 8×8 non-overlapping windows (the widely used fast approximation of the
/// canonical 11×11 Gaussian sliding window) with the standard 8-bit stabilising
/// constants `C1 = (0.01·255)²`, `C2 = (0.03·255)²`, and averages the per-window
/// SSIM. Windows that would run past the right/bottom edge are anchored back
/// inside the image so every pixel is covered. For images smaller than one
/// window the whole image is treated as a single window.
fn ssim_planes(a: &[f64], b: &[f64], width: usize, height: usize) -> f64 {
    const C1: f64 = (0.01 * 255.0) * (0.01 * 255.0);
    const C2: f64 = (0.03 * 255.0) * (0.03 * 255.0);
    const WIN: usize = 8;

    if width == 0 || height == 0 {
        return 1.0;
    }

    let win_w = WIN.min(width);
    let win_h = WIN.min(height);
    let n = (win_w * win_h) as f64;

    // Top-left origins of the non-overlapping windows, clamped so the last
    // window in each axis ends exactly at the edge (covers the remainder).
    let origins = |len: usize, win: usize| -> Vec<usize> {
        let mut v = Vec::new();
        let mut o = 0usize;
        loop {
            v.push(o.min(len - win));
            if o + win >= len {
                break;
            }
            o += win;
        }
        v
    };
    let xs = origins(width, win_w);
    let ys = origins(height, win_h);

    let mut sum_ssim = 0.0;
    let mut count = 0.0;
    for &oy in &ys {
        for &ox in &xs {
            let (mut sa, mut sb, mut saa, mut sbb, mut sab) = (0.0, 0.0, 0.0, 0.0, 0.0);
            for wy in 0..win_h {
                let row = (oy + wy) * width + ox;
                for wx in 0..win_w {
                    let va = a[row + wx];
                    let vb = b[row + wx];
                    sa += va;
                    sb += vb;
                    saa += va * va;
                    sbb += vb * vb;
                    sab += va * vb;
                }
            }
            let mu_a = sa / n;
            let mu_b = sb / n;
            // Population variance/covariance (divide by n) — standard for SSIM.
            let var_a = saa / n - mu_a * mu_a;
            let var_b = sbb / n - mu_b * mu_b;
            let cov = sab / n - mu_a * mu_b;
            let num = (2.0 * mu_a * mu_b + C1) * (2.0 * cov + C2);
            let den = (mu_a * mu_a + mu_b * mu_b + C1) * (var_a + var_b + C2);
            sum_ssim += num / den;
            count += 1.0;
        }
    }
    if count == 0.0 { 1.0 } else { sum_ssim / count }
}

/// PSNR (dB) between two RGBA [`Pixmap`]s over their luma channel.
///
/// # Panics
/// Panics if the two pixmaps differ in dimensions.
#[cfg(feature = "std")]
pub fn psnr(a: &Pixmap, b: &Pixmap) -> f64 {
    assert_eq!(
        (a.width, a.height),
        (b.width, b.height),
        "psnr: dimension mismatch"
    );
    let (pa, pb) = (luma_plane(&a.data), luma_plane(&b.data));
    psnr_from_mse(mse_planes(&pa, &pb))
}

/// SSIM in `[-1, 1]` between two RGBA [`Pixmap`]s over their luma channel.
///
/// # Panics
/// Panics if the two pixmaps differ in dimensions.
#[cfg(feature = "std")]
pub fn ssim(a: &Pixmap, b: &Pixmap) -> f64 {
    assert_eq!(
        (a.width, a.height),
        (b.width, b.height),
        "ssim: dimension mismatch"
    );
    let (pa, pb) = (luma_plane(&a.data), luma_plane(&b.data));
    ssim_planes(&pa, &pb, a.width as usize, a.height as usize)
}

/// Full [`QualityReport`] (MSE + PSNR + SSIM) for two RGBA [`Pixmap`]s.
///
/// # Panics
/// Panics if the two pixmaps differ in dimensions.
#[cfg(feature = "std")]
pub fn compare(a: &Pixmap, b: &Pixmap) -> QualityReport {
    assert_eq!(
        (a.width, a.height),
        (b.width, b.height),
        "compare: dimension mismatch"
    );
    let (pa, pb) = (luma_plane(&a.data), luma_plane(&b.data));
    let mse = mse_planes(&pa, &pb);
    QualityReport {
        mse,
        psnr_db: psnr_from_mse(mse),
        ssim: ssim_planes(&pa, &pb, a.width as usize, a.height as usize),
    }
}

/// [`QualityReport`] for two grayscale [`GrayPixmap`]s (one byte per pixel).
///
/// # Panics
/// Panics if the two pixmaps differ in dimensions.
#[cfg(feature = "std")]
pub fn compare_gray(a: &GrayPixmap, b: &GrayPixmap) -> QualityReport {
    assert_eq!(
        (a.width, a.height),
        (b.width, b.height),
        "compare_gray: dimension mismatch"
    );
    let pa: Vec<f64> = a.data.iter().map(|&v| v as f64).collect();
    let pb: Vec<f64> = b.data.iter().map(|&v| v as f64).collect();
    let mse = mse_planes(&pa, &pb);
    QualityReport {
        mse,
        psnr_db: psnr_from_mse(mse),
        ssim: ssim_planes(&pa, &pb, a.width as usize, a.height as usize),
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, r: u8, g: u8, b: u8) -> Pixmap {
        Pixmap::new(w, h, r, g, b, 255)
    }

    #[test]
    fn identical_images_are_perfect() {
        let a = solid(40, 30, 120, 130, 140);
        let rep = compare(&a, &a);
        assert_eq!(rep.mse, 0.0);
        assert!(rep.psnr_db.is_infinite());
        assert!((rep.ssim - 1.0).abs() < 1e-9, "ssim {}", rep.ssim);
    }

    #[test]
    fn psnr_from_mse_known_value() {
        // MSE = 1 → PSNR = 10·log10(65025) ≈ 48.13 dB.
        assert!((psnr_from_mse(1.0) - 48.1308).abs() < 1e-3);
        assert!(psnr_from_mse(0.0).is_infinite());
    }

    #[test]
    fn constant_offset_lowers_psnr_predictably() {
        // Two solid fields differing by a constant luma of 10 → MSE = 100.
        let a = solid(32, 32, 100, 100, 100);
        let b = solid(32, 32, 110, 110, 110);
        let rep = compare(&a, &b);
        assert!((rep.mse - 100.0).abs() < 1e-6, "mse {}", rep.mse);
        assert!((rep.psnr_db - psnr_from_mse(100.0)).abs() < 1e-9);
        // A pure luminance shift on flat fields still has high structural
        // similarity (contrast + structure terms are unchanged).
        assert!(rep.ssim > 0.9, "ssim {}", rep.ssim);
    }

    #[test]
    fn noise_reduces_ssim_more_than_a_flat_shift() {
        let w = 64u32;
        let h = 64u32;
        let base = solid(w, h, 128, 128, 128);

        // A flat +20 shift everywhere.
        let shifted = solid(w, h, 148, 148, 148);

        // Structured corruption: a pixel-level checkerboard so every window
        // sees high-variance damage (none stay unchanged).
        let mut noisy = base.clone();
        for y in 0..h {
            for x in 0..w {
                if (x + y) % 2 == 0 {
                    noisy.set_rgb(x, y, 20, 20, 20);
                } else {
                    noisy.set_rgb(x, y, 235, 235, 235);
                }
            }
        }

        let ssim_shift = ssim(&base, &shifted);
        let ssim_noise = ssim(&base, &noisy);
        assert!(
            ssim_noise < ssim_shift,
            "structural damage {ssim_noise} should score below a flat shift {ssim_shift}"
        );
        assert!(
            ssim_noise < 0.2,
            "pixel-checkerboard damage ssim {ssim_noise}"
        );
    }

    #[test]
    fn tiny_images_handled() {
        // Smaller than one 8×8 window — must not panic and must be perfect.
        let a = solid(3, 2, 10, 20, 30);
        let rep = compare(&a, &a);
        assert!((rep.ssim - 1.0).abs() < 1e-9);
        assert!(rep.psnr_db.is_infinite());
    }

    #[test]
    fn gray_and_rgba_luma_agree() {
        let a = solid(24, 24, 90, 90, 90);
        let b = solid(24, 24, 100, 100, 100);
        let rgba = compare(&a, &b);
        let gray = compare_gray(&a.to_gray8(), &b.to_gray8());
        assert!((rgba.mse - gray.mse).abs() < 1.0);
        assert!((rgba.ssim - gray.ssim).abs() < 1e-3);
    }
}
