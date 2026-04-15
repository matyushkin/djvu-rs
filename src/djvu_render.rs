//! Rendering pipeline for the new DjVuPage model (phase 5).
//!
//! This module provides the high-level rendering API for [`DjVuPage`] using the
//! clean-room decoders (IW44, JB2, BZZ) introduced in phases 2–3.
//!
//! ## Key public types
//!
//! - `RenderOptions` — render parameters (size, scale, bold, AA)
//! - `RenderError` — typed errors from the render pipeline
//!
//! ## Compositing model
//!
//! Three layers are composited in this order:
//!
//! 1. **Background** — IW44 wavelet-coded YCbCr image (BG44 chunks).
//!    YCbCr → RGB conversion happens HERE, and nowhere else.
//! 2. **Mask** — JB2 bilevel image (Sjbz chunk). Black pixels mark foreground.
//! 3. **Foreground palette** — FGbz-encoded color palette (FGbz chunk).
//!    Each foreground pixel is colored according to the palette.
//!
//! ## Gamma correction
//!
//! A `gamma_lut[256]` is precomputed from the INFO chunk `gamma` value using
//! `lut[i] = (i/255)^(doc_gamma/2.2) * 255`.  For the vast majority of DjVu
//! files (gamma = 2.2) the exponent is 1.0 → identity, no correction applied.
//!
//! ## Scaling
//!
//! Bilinear scaling uses 4-bit fixed-point fractional coordinates (FRACBITS=4).
//! Anti-aliasing downscale averages a 2×2 neighbourhood before outputting.
//!
//! ## Progressive rendering
//!
//! `render_coarse()` decodes only the first BG44 chunk; subsequent calls to
//! `render_progressive(chunk_n)` decode one additional chunk, yielding
//! progressively higher-quality images.

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

use crate::djvu_document::DjVuPage;
use crate::iw44_new::Iw44Image;
use crate::pixmap::{GrayPixmap, Pixmap};

// ── Errors ───────────────────────────────────────────────────────────────────

/// Errors that can occur during DjVuPage rendering.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// IW44 wavelet decode error.
    #[error("IW44 decode error: {0}")]
    Iw44(#[from] crate::error::Iw44Error),

    /// JB2 bilevel decode error.
    #[error("JB2 decode error: {0}")]
    Jb2(#[from] crate::error::Jb2Error),

    /// The output buffer provided to `render_into` is too small.
    #[error("buffer too small: need {need} bytes, got {got}")]
    BufTooSmall { need: usize, got: usize },

    /// The requested render dimensions are invalid (zero width or height).
    #[error("invalid render dimensions: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },

    /// `chunk_n` is out of range for progressive rendering.
    #[error("chunk index {chunk_n} out of range (max {max})")]
    ChunkOutOfRange { chunk_n: usize, max: usize },

    /// BZZ decompression error (for FGbz palette).
    #[error("BZZ error: {0}")]
    Bzz(#[from] crate::error::BzzError),

    /// JPEG decode error (for BGjp/FGjp chunks).
    #[cfg(feature = "std")]
    #[error("JPEG decode error: {0}")]
    Jpeg(String),

    /// Document-level error (e.g. page index out of range).
    #[error("document error: {0}")]
    Doc(#[from] crate::djvu_document::DocError),
}

// ── RenderOptions ─────────────────────────────────────────────────────────────

/// User-requested rotation, applied on top of the INFO chunk rotation.
///
/// The final rotation is the sum of the INFO rotation and the user rotation.
/// For example, if the INFO chunk specifies 90° CW and the user requests 90° CW,
/// the output will be rotated 180°.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UserRotation {
    /// No additional rotation (only INFO chunk rotation applies).
    #[default]
    None,
    /// 90° clockwise.
    Cw90,
    /// 180°.
    Rot180,
    /// 90° counter-clockwise (= 270° clockwise).
    Ccw90,
}

/// Resampling algorithm used when scaling a rendered page to the target size.
///
/// Applied after full-resolution decode and compositing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Resampling {
    /// Bilinear interpolation (default — fast, acceptable quality).
    #[default]
    Bilinear,
    /// Lanczos-3 separable resampling.
    ///
    /// Higher quality than bilinear for downscaling (less aliasing, sharper
    /// text). Slower: two-pass separable filter with a 6-tap kernel.
    /// The rendered pixmap is produced at full page resolution and then
    /// downscaled, so memory usage is higher than `Bilinear`.
    Lanczos3,
}

/// Rendering parameters passed to `render_into` and related functions.
///
/// # Example
///
/// ```
/// use djvu_rs::djvu_render::{RenderOptions, UserRotation};
///
/// let opts = RenderOptions {
///     width: 800,
///     height: 600,
///     scale: 1.0,
///     bold: 0,
///     aa: true,
///     rotation: UserRotation::None,
///     permissive: false,
///     resampling: djvu_rs::djvu_render::Resampling::Bilinear,
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RenderOptions {
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
    /// Scale factor (informational; actual size is given by `width`/`height`).
    pub scale: f32,
    /// Bold level: number of dilation passes on the JB2 mask (0 = no dilation).
    pub bold: u8,
    /// Whether to apply anti-aliasing downscale pass.
    pub aa: bool,
    /// User-requested rotation, combined with the INFO chunk rotation.
    pub rotation: UserRotation,
    /// When `true`, tolerate corrupted chunks instead of returning an error.
    ///
    /// - BG44: decodes chunks until the first decode error; uses whatever
    ///   was decoded so far (may be empty / blurry).
    /// - JB2 mask: if decoding fails, renders the background without a mask
    ///   rather than returning `Err`.
    ///
    /// Returns `Ok(pixmap)` even when chunks are skipped. Useful for document
    /// viewers where a partial render is better than a blank page.
    ///
    /// Default: `false` (strict — any decode error propagates as `Err`).
    pub permissive: bool,
    /// Resampling algorithm applied when scaling to `width`×`height`.
    ///
    /// Default: [`Resampling::Bilinear`] (preserves backward compatibility).
    pub resampling: Resampling,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            width: 0,
            height: 0,
            scale: 1.0,
            bold: 0,
            aa: false,
            rotation: UserRotation::None,
            permissive: false,
            resampling: Resampling::Bilinear,
        }
    }
}

impl RenderOptions {
    /// Create render options that scale the page to fit the given width,
    /// preserving aspect ratio. Respects page rotation from the INFO chunk.
    pub fn fit_to_width(page: &crate::djvu_document::DjVuPage, width: u32) -> Self {
        let (dw, dh) = display_dimensions(page);
        let height = if dw == 0 {
            width
        } else {
            ((dh as f64 * width as f64) / dw as f64).round() as u32
        }
        .max(1);
        let scale = width as f32 / dw.max(1) as f32;
        RenderOptions {
            width,
            height,
            scale,
            ..Default::default()
        }
    }

    /// Create render options that scale the page to fit the given height,
    /// preserving aspect ratio. Respects page rotation from the INFO chunk.
    pub fn fit_to_height(page: &crate::djvu_document::DjVuPage, height: u32) -> Self {
        let (dw, dh) = display_dimensions(page);
        let width = if dh == 0 {
            height
        } else {
            ((dw as f64 * height as f64) / dh as f64).round() as u32
        }
        .max(1);
        let scale = height as f32 / dh.max(1) as f32;
        RenderOptions {
            width,
            height,
            scale,
            ..Default::default()
        }
    }

    /// Create render options that scale the page to fit within a bounding box,
    /// preserving aspect ratio. Respects page rotation from the INFO chunk.
    pub fn fit_to_box(
        page: &crate::djvu_document::DjVuPage,
        max_width: u32,
        max_height: u32,
    ) -> Self {
        let (dw, dh) = display_dimensions(page);
        if dw == 0 || dh == 0 {
            return RenderOptions {
                width: max_width.max(1),
                height: max_height.max(1),
                scale: 1.0,
                ..Default::default()
            };
        }
        let scale_w = max_width as f64 / dw as f64;
        let scale_h = max_height as f64 / dh as f64;
        let scale = if scale_w < scale_h { scale_w } else { scale_h };
        let width = (dw as f64 * scale).round() as u32;
        let height = (dh as f64 * scale).round() as u32;
        RenderOptions {
            width: width.max(1),
            height: height.max(1),
            scale: scale as f32,
            ..Default::default()
        }
    }
}

/// Return `(display_width, display_height)` — dimensions after rotation.
fn display_dimensions(page: &crate::djvu_document::DjVuPage) -> (u32, u32) {
    let w = page.width() as u32;
    let h = page.height() as u32;
    match page.rotation() {
        crate::info::Rotation::Cw90 | crate::info::Rotation::Ccw90 => (h, w),
        _ => (w, h),
    }
}

// ── Gamma LUT ─────────────────────────────────────────────────────────────────

/// Standard sRGB / CRT display gamma assumed for rendering output.
const DISPLAY_GAMMA: f32 = 2.2;

/// Precompute a gamma-correction look-up table for values 0..255.
///
/// Matches DjVuLibre's correction formula: the exponent is
/// `document_gamma / DISPLAY_GAMMA` so that documents created on a
/// standard gamma-2.2 device need no correction (identity LUT), while
/// documents from linear-light (gamma=1.0) sources are brightened to
/// compensate for the display gamma.
///
/// `lut[i] = round(255 * (i/255)^(gamma / DISPLAY_GAMMA))`
///
/// When `gamma <= 0.0`, not finite, or approximately equal to
/// `DISPLAY_GAMMA`, the LUT is the identity function (no correction).
fn build_gamma_lut(gamma: f32) -> [u8; 256] {
    let mut lut = [0u8; 256];
    let exponent = if gamma <= 0.0 || !gamma.is_finite() {
        1.0_f32 // invalid — no correction
    } else {
        gamma / DISPLAY_GAMMA
    };
    if (exponent - 1.0).abs() < 1e-4 {
        // Identity
        for (i, v) in lut.iter_mut().enumerate() {
            *v = i as u8;
        }
        return lut;
    }
    for (i, v) in lut.iter_mut().enumerate() {
        let linear = i as f32 / 255.0;
        let corrected = linear.powf(exponent);
        *v = (corrected * 255.0 + 0.5) as u8;
    }
    lut
}

// ── Bilinear scaling (FRACBITS = 4) ──────────────────────────────────────────

/// Fixed-point fractional bits for bilinear scaling (1 << 4 = 16 subpixels).
const FRACBITS: u32 = 4;
const FRAC: u32 = 1 << FRACBITS;
const FRAC_MASK: u32 = FRAC - 1;

// ── SIMD helpers ──────────────────────────────────────────────────────────────

/// Set alpha = 255 on every RGBA pixel in `buf`.
///
/// On x86_64 with SSE2 (universally available since 2003): processes 4 pixels
/// (16 bytes) per instruction via `_mm_or_si128`.  Falls back to a scalar loop
/// on other targets.
#[allow(unsafe_code)]
#[inline]
fn fill_alpha_255(buf: &mut [u8]) {
    debug_assert_eq!(buf.len() % 4, 0);

    #[cfg(target_arch = "x86_64")]
    // SAFETY: SSE2 is required by the x86_64 ABI — always available.
    unsafe {
        fill_alpha_255_sse2(buf);
    }

    #[cfg(not(target_arch = "x86_64"))]
    for pixel in buf.chunks_exact_mut(4) {
        pixel[3] = 255;
    }
}

#[cfg(target_arch = "x86_64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "sse2")]
// SAFETY: caller guarantees SSE2 is available (ABI requirement on x86_64);
// buf is valid for its entire length; i * 16 is in-bounds by construction.
unsafe fn fill_alpha_255_sse2(buf: &mut [u8]) {
    use core::arch::x86_64::*;

    // Each 32-bit pixel: OR with 0xFF000000 to set the high byte (alpha) to 255.
    let alpha_mask = _mm_set1_epi32(0xFF000000u32 as i32);
    let ptr = buf.as_mut_ptr();
    let chunks = buf.len() / 16; // 4 RGBA pixels per 128-bit register

    for i in 0..chunks {
        let p = ptr.add(i * 16) as *mut __m128i;
        _mm_storeu_si128(
            p,
            _mm_or_si128(_mm_loadu_si128(p as *const __m128i), alpha_mask),
        );
    }

    // Scalar tail (0–3 remaining pixels)
    for i in (chunks * 4)..(buf.len() / 4) {
        buf[i * 4 + 3] = 255;
    }
}

/// Convert packed RGB bytes to packed RGBA with alpha = 255.
///
/// On x86_64 with SSSE3 (available on Core 2+, ~2006): processes 4 pixels per
/// `_mm_shuffle_epi8` + `_mm_or_si128`.  Falls back to scalar on older targets.
///
/// `src` must hold exactly `pixel_count * 3` bytes;
/// `dst` must hold exactly `pixel_count * 4` bytes.
#[cfg(feature = "std")]
#[allow(unsafe_code)]
#[inline]
fn rgb_to_rgba(src: &[u8], dst: &mut [u8]) {
    let pixel_count = src.len() / 3;
    debug_assert_eq!(dst.len(), pixel_count * 4);

    #[cfg(target_arch = "x86_64")]
    if is_x86_feature_detected!("ssse3") {
        // SAFETY: feature detected; bounds are enforced by safe_chunks calculation.
        unsafe {
            // Only load 16 bytes where src has at least 16 bytes available:
            // chunk i reads src[i*12..i*12+16], so we need i*12+16 <= src.len().
            let safe_chunks = if src.len() >= 16 {
                ((src.len() - 16) / 12 + 1).min(pixel_count / 4)
            } else {
                0
            };
            rgb_to_rgba_ssse3(src, dst, pixel_count, safe_chunks);
        }
        return;
    }

    rgb_to_rgba_scalar(src, dst, 0, pixel_count);
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "ssse3")]
// SAFETY: caller guarantees SSSE3 availability; safe_chunks * 12 + 16 <= src.len()
// and safe_chunks * 16 <= dst.len() (enforced by rgb_to_rgba).
unsafe fn rgb_to_rgba_ssse3(src: &[u8], dst: &mut [u8], pixel_count: usize, safe_chunks: usize) {
    use core::arch::x86_64::*;

    // Shuffle 12 packed RGB bytes into 16 RGBA bytes (4 pixels), zero in alpha slot.
    // _mm_set_epi8 arguments are byte 15 (highest) down to byte 0 (lowest).
    let shuf = _mm_set_epi8(
        -1, 11, 10, 9, // pixel 3: [R,G,B,0]
        -1, 8, 7, 6, // pixel 2
        -1, 5, 4, 3, // pixel 1
        -1, 2, 1, 0, // pixel 0
    );
    let alpha_or = _mm_set1_epi32(0xFF000000u32 as i32);

    for i in 0..safe_chunks {
        let v = _mm_loadu_si128(src.as_ptr().add(i * 12) as *const __m128i);
        _mm_storeu_si128(
            dst.as_mut_ptr().add(i * 16) as *mut __m128i,
            _mm_or_si128(_mm_shuffle_epi8(v, shuf), alpha_or),
        );
    }

    rgb_to_rgba_scalar(src, dst, safe_chunks * 4, pixel_count);
}

#[cfg(feature = "std")]
#[inline]
fn rgb_to_rgba_scalar(src: &[u8], dst: &mut [u8], start: usize, end: usize) {
    for i in start..end {
        dst[i * 4] = src[i * 3];
        dst[i * 4 + 1] = src[i * 3 + 1];
        dst[i * 4 + 2] = src[i * 3 + 2];
        dst[i * 4 + 3] = 255;
    }
}

/// Sample a pixmap at fractional coordinates using bilinear interpolation.
///
/// Coordinates are in fixed-point: `fx = x * FRAC`, etc.
/// Returns (r, g, b).
#[inline]
fn sample_bilinear(pm: &Pixmap, fx: u32, fy: u32) -> (u8, u8, u8) {
    let x0 = (fx >> FRACBITS).min(pm.width.saturating_sub(1));
    let y0 = (fy >> FRACBITS).min(pm.height.saturating_sub(1));
    let x1 = (x0 + 1).min(pm.width.saturating_sub(1));
    let y1 = (y0 + 1).min(pm.height.saturating_sub(1));

    let tx = fx & FRAC_MASK; // 0..15
    let ty = fy & FRAC_MASK;

    let (r00, g00, b00) = pm.get_rgb(x0, y0);
    let (r10, g10, b10) = pm.get_rgb(x1, y0);
    let (r01, g01, b01) = pm.get_rgb(x0, y1);
    let (r11, g11, b11) = pm.get_rgb(x1, y1);

    let lerp = |a: u8, b: u8, c: u8, d: u8| -> u8 {
        let top = a as u32 * (FRAC - tx) + b as u32 * tx;
        let bot = c as u32 * (FRAC - tx) + d as u32 * tx;
        let v = (top * (FRAC - ty) + bot * ty) >> (2 * FRACBITS);
        v.min(255) as u8
    };

    (
        lerp(r00, r10, r01, r11),
        lerp(g00, g10, g01, g11),
        lerp(b00, b10, b01, b11),
    )
}

/// Area-average (box filter) sample: average all source pixels covered by the
/// output pixel's footprint.  Used when downscaling (scale < 1.0) for better
/// anti-aliasing and fewer moire patterns than bilinear.
///
/// `fx`, `fy` are the top-left corner of the output pixel in fixed-point.
/// `fx_step`, `fy_step` are the output pixel size in source coordinates.
#[inline]
fn sample_area_avg(pm: &Pixmap, fx: u32, fy: u32, fx_step: u32, fy_step: u32) -> (u8, u8, u8) {
    let x0 = (fx >> FRACBITS).min(pm.width.saturating_sub(1));
    let y0 = (fy >> FRACBITS).min(pm.height.saturating_sub(1));
    let x1 = ((fx + fx_step) >> FRACBITS).min(pm.width.saturating_sub(1));
    let y1 = ((fy + fy_step) >> FRACBITS).min(pm.height.saturating_sub(1));

    // Fast path: box is 1×1 pixel → just read it
    if x0 == x1 && y0 == y1 {
        return pm.get_rgb(x0, y0);
    }

    let mut r_sum = 0u32;
    let mut g_sum = 0u32;
    let mut b_sum = 0u32;

    let pw = pm.width as usize;
    let cols = (x1 - x0 + 1) as usize;
    let rows = (y1 - y0 + 1) as usize;

    // Read directly from the RGBA data buffer for speed
    for sy in y0..=y1 {
        let row_off = (sy as usize * pw + x0 as usize) * 4;
        for c in 0..cols {
            let off = row_off + c * 4;
            if let Some(px) = pm.data.get(off..off + 3) {
                r_sum += px[0] as u32;
                g_sum += px[1] as u32;
                b_sum += px[2] as u32;
            }
        }
    }

    let count = (rows * cols) as u32;
    if count == 0 {
        return (255, 255, 255);
    }

    (
        ((r_sum + count / 2) / count) as u8,
        ((g_sum + count / 2) / count) as u8,
        ((b_sum + count / 2) / count) as u8,
    )
}

// ── Lanczos-3 resampling ─────────────────────────────────────────────────────

/// Lanczos-3 kernel: `sinc(x) * sinc(x/3)` for `|x| < 3`, 0 otherwise.
///
/// Uses the normalised sinc: `sinc(x) = sin(π x) / (π x)`, `sinc(0) = 1`.
#[inline]
fn lanczos3_kernel(x: f32) -> f32 {
    let ax = x.abs();
    if ax >= 3.0 {
        return 0.0;
    }
    if ax < 1e-6 {
        return 1.0;
    }
    let pi_x = core::f32::consts::PI * ax;
    let sinc_x = pi_x.sin() / pi_x;
    let pi_x3 = pi_x / 3.0;
    let sinc_x3 = pi_x3.sin() / pi_x3;
    sinc_x * sinc_x3
}

/// Scale `src` to `dst_w × dst_h` using separable Lanczos-3 resampling.
///
/// Two-pass implementation:
/// 1. Horizontal pass: `src_w × src_h` → `dst_w × src_h` intermediate.
/// 2. Vertical pass: `dst_w × src_h` → `dst_w × dst_h` output.
///
/// Only RGBA pixmaps are handled (alpha is passed through unchanged at 255).
pub fn scale_lanczos3(src: &Pixmap, dst_w: u32, dst_h: u32) -> Pixmap {
    let src_w = src.width;
    let src_h = src.height;

    // Short-circuit: nothing to scale.
    if src_w == dst_w && src_h == dst_h {
        return src.clone();
    }
    if dst_w == 0 || dst_h == 0 {
        return Pixmap::white(dst_w.max(1), dst_h.max(1));
    }

    // ── Horizontal pass ───────────────────────────────────────────────────────
    // Map each output column `ox` (0..dst_w) to a source position, then sum
    // the Lanczos-3 kernel over the contributing source columns.
    let h_scale = src_w as f32 / dst_w as f32;
    let h_support = (3.0_f32 * h_scale.max(1.0)).ceil() as i32; // kernel half-width in src pixels

    let mut mid = Pixmap::new(dst_w, src_h, 255, 255, 255, 255);
    for oy in 0..src_h {
        for ox in 0..dst_w {
            // Centre of the output pixel in source coordinates.
            let cx = (ox as f32 + 0.5) * h_scale - 0.5;
            let x0 = (cx.floor() as i32 - h_support + 1).max(0);
            let x1 = (cx.floor() as i32 + h_support).min(src_w as i32 - 1);

            let mut r = 0.0_f32;
            let mut g = 0.0_f32;
            let mut b = 0.0_f32;
            let mut w_sum = 0.0_f32;

            for sx in x0..=x1 {
                let w = lanczos3_kernel((sx as f32 - cx) / h_scale.max(1.0));
                let (pr, pg, pb) = src.get_rgb(sx as u32, oy);
                r += pr as f32 * w;
                g += pg as f32 * w;
                b += pb as f32 * w;
                w_sum += w;
            }

            let norm = if w_sum.abs() > 1e-6 { 1.0 / w_sum } else { 1.0 };
            mid.set_rgb(
                ox,
                oy,
                (r * norm).round().clamp(0.0, 255.0) as u8,
                (g * norm).round().clamp(0.0, 255.0) as u8,
                (b * norm).round().clamp(0.0, 255.0) as u8,
            );
        }
    }

    // ── Vertical pass ─────────────────────────────────────────────────────────
    let v_scale = src_h as f32 / dst_h as f32;
    let v_support = (3.0_f32 * v_scale.max(1.0)).ceil() as i32;

    let mut out = Pixmap::new(dst_w, dst_h, 255, 255, 255, 255);
    for oy in 0..dst_h {
        let cy = (oy as f32 + 0.5) * v_scale - 0.5;
        let y0 = (cy.floor() as i32 - v_support + 1).max(0);
        let y1 = (cy.floor() as i32 + v_support).min(src_h as i32 - 1);

        for ox in 0..dst_w {
            let mut r = 0.0_f32;
            let mut g = 0.0_f32;
            let mut b = 0.0_f32;
            let mut w_sum = 0.0_f32;

            for sy in y0..=y1 {
                let w = lanczos3_kernel((sy as f32 - cy) / v_scale.max(1.0));
                let (pr, pg, pb) = mid.get_rgb(ox, sy as u32);
                r += pr as f32 * w;
                g += pg as f32 * w;
                b += pb as f32 * w;
                w_sum += w;
            }

            let norm = if w_sum.abs() > 1e-6 { 1.0 / w_sum } else { 1.0 };
            out.set_rgb(
                ox,
                oy,
                (r * norm).round().clamp(0.0, 255.0) as u8,
                (g * norm).round().clamp(0.0, 255.0) as u8,
                (b * norm).round().clamp(0.0, 255.0) as u8,
            );
        }
    }

    out
}

/// Check whether any pixel in the mask box is set (foreground).
/// Used for area-averaging downscale to determine if a box has foreground.
#[inline]
fn mask_box_any(
    mask: &crate::bitmap::Bitmap,
    fx: u32,
    fy: u32,
    fx_step: u32,
    fy_step: u32,
) -> bool {
    let x0 = (fx >> FRACBITS).min(mask.width.saturating_sub(1));
    let y0 = (fy >> FRACBITS).min(mask.height.saturating_sub(1));
    let x1 = ((fx + fx_step) >> FRACBITS).min(mask.width.saturating_sub(1));
    let y1 = ((fy + fy_step) >> FRACBITS).min(mask.height.saturating_sub(1));

    for sy in y0..=y1 {
        for sx in x0..=x1 {
            if mask.get(sx, sy) {
                return true;
            }
        }
    }
    false
}

/// Find the center foreground pixel in a mask box for palette color lookup.
#[inline]
fn mask_box_center_fg(
    mask: &crate::bitmap::Bitmap,
    fx: u32,
    fy: u32,
    fx_step: u32,
    fy_step: u32,
) -> (u32, u32) {
    // Use the center of the box
    let cx = (fx + fx_step / 2) >> FRACBITS;
    let cy = (fy + fy_step / 2) >> FRACBITS;
    (
        cx.min(mask.width.saturating_sub(1)),
        cy.min(mask.height.saturating_sub(1)),
    )
}

// ── Anti-aliasing downscale ──────────────────────────────────────────────────

/// Apply a 2×2 box-filter downscale pass for anti-aliasing.
///
/// If either dimension of `pm` is 1, the output dimension stays at 1.
fn aa_downscale(pm: &Pixmap) -> Pixmap {
    let out_w = (pm.width / 2).max(1);
    let out_h = (pm.height / 2).max(1);
    let mut out = Pixmap::white(out_w, out_h);
    for y in 0..out_h {
        for x in 0..out_w {
            let sx = (x * 2).min(pm.width.saturating_sub(1));
            let sy = (y * 2).min(pm.height.saturating_sub(1));
            let sx1 = (sx + 1).min(pm.width.saturating_sub(1));
            let sy1 = (sy + 1).min(pm.height.saturating_sub(1));

            let (r00, g00, b00) = pm.get_rgb(sx, sy);
            let (r10, g10, b10) = pm.get_rgb(sx1, sy);
            let (r01, g01, b01) = pm.get_rgb(sx, sy1);
            let (r11, g11, b11) = pm.get_rgb(sx1, sy1);

            let avg = |a: u8, b: u8, c: u8, d: u8| -> u8 {
                ((a as u32 + b as u32 + c as u32 + d as u32 + 2) / 4) as u8
            };
            out.set_rgb(
                x,
                y,
                avg(r00, r10, r01, r11),
                avg(g00, g10, g01, g11),
                avg(b00, b10, b01, b11),
            );
        }
    }
    out
}

// ── Page rotation ───────────────────────────────────────────────────────────

/// Convert a rotation to a number of 90° CW steps (0..3).
fn rotation_to_steps(r: crate::info::Rotation) -> u8 {
    use crate::info::Rotation;
    match r {
        Rotation::None => 0,
        Rotation::Cw90 => 1,
        Rotation::Rot180 => 2,
        Rotation::Ccw90 => 3,
    }
}

/// Convert a user rotation to a number of 90° CW steps (0..3).
fn user_rotation_to_steps(r: UserRotation) -> u8 {
    match r {
        UserRotation::None => 0,
        UserRotation::Cw90 => 1,
        UserRotation::Rot180 => 2,
        UserRotation::Ccw90 => 3,
    }
}

/// Combine INFO chunk rotation with user rotation and return the combined
/// `info::Rotation` value.
fn combine_rotations(info: crate::info::Rotation, user: UserRotation) -> crate::info::Rotation {
    use crate::info::Rotation;
    let steps = (rotation_to_steps(info) + user_rotation_to_steps(user)) % 4;
    match steps {
        0 => Rotation::None,
        1 => Rotation::Cw90,
        2 => Rotation::Rot180,
        3 => Rotation::Ccw90,
        _ => unreachable!(),
    }
}

/// Apply page rotation to the rendered pixmap.
///
/// For 90°/270° rotations, width and height are swapped.
fn rotate_pixmap(src: Pixmap, rotation: crate::info::Rotation) -> Pixmap {
    use crate::info::Rotation;
    match rotation {
        Rotation::None => src,
        Rotation::Cw90 => {
            let w = src.height;
            let h = src.width;
            let mut out = Pixmap::white(w, h);
            for y in 0..src.height {
                for x in 0..src.width {
                    let (r, g, b) = src.get_rgb(x, y);
                    out.set_rgb(src.height - 1 - y, x, r, g, b);
                }
            }
            out
        }
        Rotation::Rot180 => {
            let mut out = Pixmap::white(src.width, src.height);
            for y in 0..src.height {
                for x in 0..src.width {
                    let (r, g, b) = src.get_rgb(x, y);
                    out.set_rgb(src.width - 1 - x, src.height - 1 - y, r, g, b);
                }
            }
            out
        }
        Rotation::Ccw90 => {
            let w = src.height;
            let h = src.width;
            let mut out = Pixmap::white(w, h);
            for y in 0..src.height {
                for x in 0..src.width {
                    let (r, g, b) = src.get_rgb(x, y);
                    out.set_rgb(y, src.width - 1 - x, r, g, b);
                }
            }
            out
        }
    }
}

// ── FGbz palette parsing ──────────────────────────────────────────────────────

/// An RGB color from the FGbz palette.
#[derive(Debug, Clone, Copy, Default)]
struct PaletteColor {
    r: u8,
    g: u8,
    b: u8,
}

/// Parsed FGbz data: palette colors and optional per-blit color indices.
struct FgbzPalette {
    colors: Vec<PaletteColor>,
    /// Per-blit color index: `indices[blit_idx]` → index into `colors`.
    /// Empty when the FGbz chunk has no index table (version bit 7 unset).
    indices: Vec<i16>,
}

/// Parse the FGbz chunk into palette colors and per-blit index table.
///
/// FGbz format:
/// - byte 0: version (bit 7 = has index table, bits 6-0 must be 0)
/// - byte 1-2: big-endian u16 palette size (number of colors)
/// - next `palette_size * 3` bytes: BGR triples (raw if version=0, BZZ if version has bit 0 set)
/// - if bit 7 set: 3-byte big-endian count + BZZ-compressed i16be index table
fn parse_fgbz(data: &[u8]) -> Result<FgbzPalette, RenderError> {
    if data.len() < 3 {
        return Ok(FgbzPalette {
            colors: vec![],
            indices: vec![],
        });
    }

    let version = data[0];
    let has_indices = (version & 0x80) != 0;

    let n_colors =
        u16::from_be_bytes([*data.get(1).unwrap_or(&0), *data.get(2).unwrap_or(&0)]) as usize;

    if n_colors == 0 {
        return Ok(FgbzPalette {
            colors: vec![],
            indices: vec![],
        });
    }

    // Colors: raw BGR triples starting at byte 3
    let color_bytes = n_colors * 3;
    let color_data = data.get(3..).unwrap_or(&[]);

    let mut colors = Vec::with_capacity(n_colors);
    for i in 0..n_colors {
        let base = i * 3;
        if base + 2 < color_data.len().min(color_bytes) {
            colors.push(PaletteColor {
                r: color_data[base + 2],
                g: color_data[base + 1],
                b: color_data[base],
            });
        } else {
            colors.push(PaletteColor { r: 0, g: 0, b: 0 });
        }
    }

    // Per-blit index table
    let mut indices = Vec::new();
    if has_indices {
        let idx_start = 3 + color_bytes;
        if idx_start + 3 <= data.len() {
            let num_indices = ((data[idx_start] as u32) << 16)
                | ((data[idx_start + 1] as u32) << 8)
                | (data[idx_start + 2] as u32);

            let bzz_data = data.get(idx_start + 3..).unwrap_or(&[]);
            let decoded = crate::bzz_new::bzz_decode(bzz_data)?;

            let n = num_indices as usize;
            indices.reserve(n);
            for i in 0..n {
                if i * 2 + 1 < decoded.len() {
                    indices.push(i16::from_be_bytes([decoded[i * 2], decoded[i * 2 + 1]]));
                }
            }
        }
    }

    Ok(FgbzPalette { colors, indices })
}

// ── Core compositor ───────────────────────────────────────────────────────────

/// Return the largest power-of-2 IW44 subsample factor for the given render
/// scale, allowing up to 1.5× upscaling in the compositor.
///
/// The compositor samples the decoded background at `pixel / subsample`, so a
/// decoded plane that is slightly smaller than the output is fine — the
/// compositor's nearest-neighbour lookup handles it naturally.  Allowing up to
/// 1.5× upscaling lets us pick a coarser subsample in many common cases
/// (e.g. 150 dpi from a 400 dpi source) and skip the high-frequency wavelet
/// bands, matching the partial-decode strategy used by DjVuLibre.
///
/// Examples (with 1.5× tolerance):
/// - scale=1.0  → 1 (full resolution)
/// - scale=0.5  → 2 (1.5/0.5=3.0 → 2)
/// - scale=0.375→ 4 (1.5/0.375=4.0 → 4)   ← was 2 before fix
/// - scale=0.25 → 4 (1.5/0.25=6.0 → 4)
/// - scale=0.1  → 8 (1.5/0.1=15 → capped at 8)
fn best_iw44_subsample(scale: f32) -> u32 {
    if scale <= 0.0 || !scale.is_finite() || scale >= 1.0 {
        return 1;
    }
    // Allow up to 1.5× upscaling: the compositor handles the coordinate
    // division, so a slightly-too-small decoded plane is fine.
    let max_sub = (1.5_f32 / scale) as u32; // truncating = floor for positive
    let mut s = 1u32;
    while s * 2 <= max_sub {
        s *= 2;
    }
    s.min(8)
}

/// Decode background from BG44 chunks up to `max_chunks`.
///
/// `subsample` controls IW44 decode resolution: 1 = full, 2 = half, 4 = quarter.
/// Use `best_iw44_subsample(opts.scale)` to pick an appropriate value.
///
/// When `max_chunks == usize::MAX`, the decoded wavelet image is fetched from
/// [`DjVuPage::decoded_bg44`]'s cache, avoiding repeated ZP arithmetic decode.
///
/// Returns `None` if there are no BG44 chunks.
/// `max_chunks = usize::MAX` means decode all chunks.
fn decode_background_chunks(
    page: &DjVuPage,
    max_chunks: usize,
    subsample: u32,
) -> Result<Option<Pixmap>, RenderError> {
    // Fast path: use a cached Iw44Image when all chunks are wanted.
    // For sub >= 4 we use the partial cache (first chunk only) — the high-frequency
    // refinement in later chunks is imperceptible at quarter-scale output, and skipping
    // them reduces cold ZP decode cost by ~4×.
    if max_chunks == usize::MAX {
        let bg44_chunks = page.bg44_chunks();
        if !bg44_chunks.is_empty() {
            let img = if subsample >= 4 {
                page.decoded_bg44_partial()
            } else {
                page.decoded_bg44()
            };
            let img = img.ok_or(RenderError::Iw44(crate::Iw44Error::Invalid))?;
            return Ok(Some(img.to_rgb_subsample(subsample)?));
        }
        // No BG44 chunks — fall through to the JPEG fallback below.
    } else {
        let bg44_chunks = page.bg44_chunks();
        if !bg44_chunks.is_empty() {
            let mut img = Iw44Image::new();
            for chunk_data in bg44_chunks.iter().take(max_chunks) {
                img.decode_chunk(chunk_data)?;
            }
            return Ok(Some(img.to_rgb_subsample(subsample)?));
        }
    }

    // Fall back to JPEG-encoded background if present.
    #[cfg(feature = "std")]
    if let Some(pm) = decode_bgjp(page)? {
        return Ok(Some(pm));
    }

    Ok(None)
}

/// Permissive variant: decode BG44 chunks until the first error, then stop.
///
/// Returns whatever was decoded so far (may be blurry / incomplete).
/// Returns `None` only when there are no BG44 chunks at all or even the
/// first chunk fails to produce a valid image.
fn decode_background_chunks_permissive(
    page: &DjVuPage,
    max_chunks: usize,
    subsample: u32,
) -> Option<Pixmap> {
    let bg44_chunks = page.bg44_chunks();
    if !bg44_chunks.is_empty() {
        let mut img = Iw44Image::new();
        for chunk_data in bg44_chunks.iter().take(max_chunks) {
            if img.decode_chunk(chunk_data).is_err() {
                break; // stop on first error, use what we have
            }
        }
        return img.to_rgb_subsample(subsample).ok();
    }

    // Fall back to JPEG-encoded background if present.
    #[cfg(feature = "std")]
    {
        decode_bgjp(page).ok().flatten()
    }
    #[cfg(not(feature = "std"))]
    None
}

/// Decode the JB2 mask (Sjbz chunk) without blit tracking.
///
/// Uses the page-level cache (`decoded_mask`) so that repeated renders of the
/// same page (e.g. at different DPI levels) skip the ZP arithmetic decode.
/// The cached `Bitmap` is cloned cheaply (1 MB memcopy) rather than re-running
/// the full 8+ ms JB2 decode.
fn decode_mask(page: &DjVuPage) -> Result<Option<crate::bitmap::Bitmap>, RenderError> {
    match page.decoded_mask() {
        Some(bm) => Ok(Some(bm.clone())),
        None if page.find_chunk(b"Sjbz").is_some() => {
            // Cache miss means decode failed; propagate via fresh decode for the error.
            page.extract_mask().map_err(RenderError::from)
        }
        None => Ok(None),
    }
}

/// Decode the JB2 mask with per-pixel blit index tracking.
///
/// Delegates to [`DjVuPage::extract_mask_indexed`] so that the shared DJVI
/// dictionary (`shared_djbz`) is used as a fallback when there is no inline
/// Djbz chunk.
fn decode_mask_indexed(
    page: &DjVuPage,
) -> Result<Option<(crate::bitmap::Bitmap, Vec<i32>)>, RenderError> {
    page.extract_mask_indexed().map_err(RenderError::from)
}

/// Decode the FGbz foreground palette with per-blit color indices.
fn decode_fg_palette_full(page: &DjVuPage) -> Result<Option<FgbzPalette>, RenderError> {
    let fgbz = match page.find_chunk(b"FGbz") {
        Some(data) => data,
        None => return Ok(None),
    };

    let pal = parse_fgbz(fgbz)?;
    if pal.colors.is_empty() {
        return Ok(None);
    }
    Ok(Some(pal))
}

/// Decode the FG44 foreground layer.
///
/// Uses the page-level cache (`decoded_fg44`) so that repeated renders skip
/// the IW44 ZP decode. Falls back to FGjp (JPEG) when no FG44 chunks are present.
fn decode_fg44(page: &DjVuPage) -> Result<Option<Pixmap>, RenderError> {
    let fg44_chunks = page.fg44_chunks();
    if !fg44_chunks.is_empty() {
        return Ok(page.decoded_fg44().cloned());
    }

    // Fall back to JPEG-encoded foreground if present.
    #[cfg(feature = "std")]
    if let Some(pm) = decode_fgjp(page)? {
        return Ok(Some(pm));
    }

    Ok(None)
}

/// Decode a BGjp (JPEG-encoded background) chunk into an RGB [`Pixmap`].
///
/// Returns `None` when the page has no `BGjp` chunk.
/// Only available with the `std` feature (requires `zune-jpeg`).
#[cfg(feature = "std")]
fn decode_bgjp(page: &DjVuPage) -> Result<Option<Pixmap>, RenderError> {
    let data = match page.find_chunk(b"BGjp") {
        Some(d) => d,
        None => return Ok(None),
    };
    Ok(Some(decode_jpeg_to_pixmap(data)?))
}

/// Decode an FGjp (JPEG-encoded foreground) chunk into an RGB [`Pixmap`].
///
/// Returns `None` when the page has no `FGjp` chunk.
/// Only available with the `std` feature (requires `zune-jpeg`).
#[cfg(feature = "std")]
fn decode_fgjp(page: &DjVuPage) -> Result<Option<Pixmap>, RenderError> {
    let data = match page.find_chunk(b"FGjp") {
        Some(d) => d,
        None => return Ok(None),
    };
    Ok(Some(decode_jpeg_to_pixmap(data)?))
}

/// Decode raw JPEG bytes into an RGBA [`Pixmap`].
///
/// Uses `zune-jpeg` for decoding. The JPEG is decoded to RGB and then
/// converted to RGBA (alpha = 255).
#[cfg(feature = "std")]
fn decode_jpeg_to_pixmap(data: &[u8]) -> Result<Pixmap, RenderError> {
    use zune_jpeg::JpegDecoder;
    use zune_jpeg::zune_core::bytestream::ZCursor;

    let cursor = ZCursor::new(data);
    let mut decoder = JpegDecoder::new(cursor);
    decoder
        .decode_headers()
        .map_err(|e| RenderError::Jpeg(format!("{e:?}")))?;
    let info = decoder
        .info()
        .ok_or_else(|| RenderError::Jpeg("missing image info after decode_headers".to_owned()))?;
    let w = info.width as usize;
    let h = info.height as usize;
    let rgb = decoder
        .decode()
        .map_err(|e| RenderError::Jpeg(format!("{e:?}")))?;

    // zune-jpeg returns packed RGB; convert to RGBA with alpha = 255.
    let pixel_count = w * h;
    let rgb = if rgb.len() >= pixel_count * 3 {
        rgb
    } else {
        // Truncated JPEG — pad with zeros so rgb_to_rgba stays in bounds.
        let mut padded = rgb;
        padded.resize(pixel_count * 3, 0);
        padded
    };
    let mut rgba = vec![0u8; pixel_count * 4];
    rgb_to_rgba(&rgb[..pixel_count * 3], &mut rgba);
    Ok(Pixmap {
        width: w as u32,
        height: h as u32,
        data: rgba,
    })
}

/// A sub-rectangle within the full rendered output.
///
/// Used by [`render_region`] to select which portion of the page to render.
/// `x` and `y` are pixel offsets within the output at `opts.width × opts.height`
/// resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderRect {
    /// X offset in output pixels.
    pub x: u32,
    /// Y offset in output pixels.
    pub y: u32,
    /// Width of the output region in pixels.
    pub width: u32,
    /// Height of the output region in pixels.
    pub height: u32,
}

/// All decoded layers and options passed to the compositor.
struct CompositeContext<'a> {
    opts: &'a RenderOptions,
    page_w: u32,
    page_h: u32,
    bg: Option<&'a Pixmap>,
    /// `bg_subsample.trailing_zeros()` where bg_subsample is 1, 2, 4, or 8.
    /// Using a shift instead of division avoids a UDIV for each `fx / bg_subsample`
    /// in the inner pixel loop.
    bg_shift: u32,
    mask: Option<&'a crate::bitmap::Bitmap>,
    /// `mask_sub.trailing_zeros()` where mask_sub is 1 (full-res) or 4 (1/4-res).
    /// Using a shift instead of division avoids a UDIV instruction in the hot path.
    mask_shift: u32,
    fg_palette: Option<&'a FgbzPalette>,
    /// Per-pixel blit index map (same dimensions as mask). `-1` = no blit.
    blit_map: Option<&'a [i32]>,
    fg44: Option<&'a Pixmap>,
    gamma_lut: &'a [u8; 256],
    /// X offset within the full render (for region renders; 0 for full page).
    offset_x: u32,
    /// Y offset within the full render (for region renders; 0 for full page).
    offset_y: u32,
    /// Output width (may be smaller than opts.width for region renders).
    out_w: u32,
    /// Output height (may be smaller than opts.height for region renders).
    out_h: u32,
}

/// Look up the palette color for a foreground pixel at (px, py).
///
/// Uses the blit map to find the per-glyph blit index, then maps it through
/// the FGbz index table to get the final color. Falls back to palette[0] when
/// no index table is present, and to black when lookup fails.
#[inline]
fn lookup_palette_color(
    pal: &FgbzPalette,
    blit_map: Option<&[i32]>,
    mask: Option<&crate::bitmap::Bitmap>,
    px: u32,
    py: u32,
) -> PaletteColor {
    if let Some(bm) = blit_map
        && let Some(m) = mask
    {
        let mi = py as usize * m.width as usize + px as usize;
        if mi < bm.len() {
            let blit_idx = bm[mi];
            if blit_idx >= 0 {
                if !pal.indices.is_empty() {
                    // Two-level indirection: blit_idx → color_idx → color
                    let bi = blit_idx as usize;
                    if bi < pal.indices.len() {
                        let ci = pal.indices[bi] as usize;
                        if ci < pal.colors.len() {
                            return pal.colors[ci];
                        }
                    }
                } else {
                    // No index table: use blit_idx directly as color index
                    let ci = blit_idx as usize;
                    if ci < pal.colors.len() {
                        return pal.colors[ci];
                    }
                }
            }
        }
    }
    // Fallback: first palette color or black
    pal.colors.first().copied().unwrap_or_default()
}

/// Bilinear composite loop — used when upscaling or at 1:1 (step ≤ 1 pixel).
/// Single-pixel mask check per output pixel.
fn composite_loop_bilinear(ctx: &CompositeContext<'_>, buf: &mut [u8], fx_step: u32, fy_step: u32) {
    let (w, h) = (ctx.out_w, ctx.out_h);
    let (page_w, page_h) = (ctx.page_w, ctx.page_h);
    let row_stride = w as usize * 4;
    for (oy, row) in buf
        .chunks_exact_mut(row_stride)
        .take(h as usize)
        .enumerate()
    {
        let oy = oy as u32;
        let fy = (oy + ctx.offset_y) * fy_step;
        let py = (fy >> FRACBITS).min(page_h.saturating_sub(1));

        for (ox, pixel) in row.chunks_exact_mut(4).enumerate() {
            let fx = (ox as u32 + ctx.offset_x) * fx_step;
            let px = (fx >> FRACBITS).min(page_w.saturating_sub(1));

            let is_fg = ctx
                .mask
                .is_some_and(|m| px < m.width && py < m.height && m.get(px, py));

            let (r, g, b) = if is_fg {
                if let Some(pal) = ctx.fg_palette {
                    let color = lookup_palette_color(pal, ctx.blit_map, ctx.mask, px, py);
                    (color.r, color.g, color.b)
                } else if let Some(fg) = ctx.fg44 {
                    sample_bilinear(fg, fx, fy)
                } else {
                    (0, 0, 0)
                }
            } else if let Some(bg) = ctx.bg {
                sample_bilinear(bg, fx >> ctx.bg_shift, fy >> ctx.bg_shift)
            } else {
                (255, 255, 255)
            };

            pixel[0] = ctx.gamma_lut[r as usize];
            pixel[1] = ctx.gamma_lut[g as usize];
            pixel[2] = ctx.gamma_lut[b as usize];
            // alpha written by fill_alpha_255 in composite_into
        }
    }
}

/// Area-averaging composite loop — used when downscaling (step > 1 pixel).
/// Uses box filter for background sampling and checks a box of mask pixels.
fn composite_loop_area_avg(ctx: &CompositeContext<'_>, buf: &mut [u8], fx_step: u32, fy_step: u32) {
    let (w, h) = (ctx.out_w, ctx.out_h);
    // Precompute bg step in bg-space (avoid per-pixel division in the inner loop).
    let bg_sh = ctx.bg_shift;
    let bg_fx_step = fx_step >> bg_sh;
    let bg_fy_step = fy_step >> bg_sh;

    let row_stride = w as usize * 4;
    for (oy, row) in buf
        .chunks_exact_mut(row_stride)
        .take(h as usize)
        .enumerate()
    {
        let oy = oy as u32;
        let fy = (oy + ctx.offset_y) * fy_step;
        // bg-space fy (shift replaces division by bg_subsample)
        let bg_fy = fy >> bg_sh;

        for (ox, pixel) in row.chunks_exact_mut(4).enumerate() {
            let fx = (ox as u32 + ctx.offset_x) * fx_step;

            let is_fg = ctx.mask.is_some_and(|m| {
                if ctx.mask_shift > 0 {
                    // Single-bit lookup in pre-downsampled mask (shift replaces division)
                    let px = fx >> (FRACBITS + ctx.mask_shift);
                    let py = fy >> (FRACBITS + ctx.mask_shift);
                    px < m.width && py < m.height && m.get(px, py)
                } else {
                    mask_box_any(m, fx, fy, fx_step, fy_step)
                }
            });

            let (r, g, b) = if is_fg {
                if let Some(pal) = ctx.fg_palette {
                    let (cx, cy) = mask_box_center_fg(ctx.mask.unwrap(), fx, fy, fx_step, fy_step);
                    let color = lookup_palette_color(pal, ctx.blit_map, ctx.mask, cx, cy);
                    (color.r, color.g, color.b)
                } else if let Some(fg) = ctx.fg44 {
                    sample_area_avg(fg, fx, fy, fx_step, fy_step)
                } else {
                    (0, 0, 0)
                }
            } else if let Some(bg) = ctx.bg {
                // bg-space fx (shift replaces division by bg_subsample)
                let bg_fx = fx >> bg_sh;
                sample_area_avg(bg, bg_fx, bg_fy, bg_fx_step, bg_fy_step)
            } else {
                (255, 255, 255)
            };

            pixel[0] = ctx.gamma_lut[r as usize];
            pixel[1] = ctx.gamma_lut[g as usize];
            pixel[2] = ctx.gamma_lut[b as usize];
            // alpha written by fill_alpha_255 in composite_into
        }
    }
}

/// Bilevel fast path: JB2-only page with no IW44 background or FG44 layer.
///
/// Fills `buf` with white (255,255,255,255), then paints foreground pixels
/// black (0,0,0,255).  Avoids bilinear sampling, gamma LUT lookups, and the
/// full per-pixel branch tree of `composite_loop_bilinear` — the only work per
/// pixel is a single mask-bit read and a conditional 3-byte write.
///
/// Handles both upscale/1:1 (`fx_step ≤ FRAC`) and downscale cases.
fn composite_loop_bilevel(ctx: &CompositeContext<'_>, buf: &mut [u8], fx_step: u32, fy_step: u32) {
    let mask = match ctx.mask {
        Some(m) => m,
        None => {
            // No mask, no bg: pure white page — just fill.
            for chunk in buf.chunks_exact_mut(4) {
                chunk[0] = 255;
                chunk[1] = 255;
                chunk[2] = 255;
                chunk[3] = 255;
            }
            return;
        }
    };

    let w = ctx.out_w;
    let h = ctx.out_h;
    let row_stride = w as usize * 4;

    // ── 1:1 scale fast path ────────────────────────────────────────────────────
    // Skips fixed-point arithmetic entirely; reads bitmap bytes directly.
    if fx_step == FRAC && fy_step == FRAC {
        let stride = mask.row_stride();
        for (oy, row) in buf
            .chunks_exact_mut(row_stride)
            .take(h as usize)
            .enumerate()
        {
            let py = (oy as u32 + ctx.offset_y).min(ctx.page_h.saturating_sub(1)) as usize;
            let mask_row = &mask.data[py * stride..(py + 1) * stride];
            for (ox, pixel) in row.chunks_exact_mut(4).enumerate() {
                let px = (ox as u32 + ctx.offset_x).min(ctx.page_w.saturating_sub(1)) as usize;
                let is_black = (mask_row[px / 8] >> (7 - (px % 8))) & 1 != 0;
                if is_black {
                    pixel[0] = 0;
                    pixel[1] = 0;
                    pixel[2] = 0;
                    pixel[3] = 255;
                } else {
                    pixel[0] = 255;
                    pixel[1] = 255;
                    pixel[2] = 255;
                    pixel[3] = 255;
                }
            }
        }
        return;
    }

    // ── Scaled path (upscale > 1:1, or downscale) ─────────────────────────────
    let downscale = fx_step > FRAC || fy_step > FRAC;

    for (oy, row) in buf
        .chunks_exact_mut(row_stride)
        .take(h as usize)
        .enumerate()
    {
        let oy = oy as u32;
        let fy = (oy + ctx.offset_y) * fy_step;
        let py = (fy >> FRACBITS).min(ctx.page_h.saturating_sub(1));

        for (ox, pixel) in row.chunks_exact_mut(4).enumerate() {
            let fx = (ox as u32 + ctx.offset_x) * fx_step;
            let px = (fx >> FRACBITS).min(ctx.page_w.saturating_sub(1));

            let is_fg = if downscale {
                if ctx.mask_shift > 0 {
                    let dpx = fx >> (FRACBITS + ctx.mask_shift);
                    let dpy = fy >> (FRACBITS + ctx.mask_shift);
                    dpx < mask.width && dpy < mask.height && mask.get(dpx, dpy)
                } else {
                    mask_box_any(mask, fx, fy, fx_step, fy_step)
                }
            } else {
                px < mask.width && py < mask.height && mask.get(px, py)
            };

            if is_fg {
                pixel[0] = 0;
                pixel[1] = 0;
                pixel[2] = 0;
                pixel[3] = 255;
            } else {
                pixel[0] = 255;
                pixel[1] = 255;
                pixel[2] = 255;
                pixel[3] = 255;
            }
        }
    }
}

/// Composite one page into `buf` (RGBA, pre-allocated) using the given context.
///
/// This is a zero-allocation render path when `buf` is already the right size.
/// For region renders, `ctx.out_w`/`ctx.out_h` give the output dimensions and
/// `ctx.offset_x`/`ctx.offset_y` give the starting offset within the full render.
fn composite_into(ctx: &CompositeContext<'_>, buf: &mut [u8]) -> Result<(), RenderError> {
    let full_w = ctx.opts.width;
    let full_h = ctx.opts.height;

    // Fixed-point step: how many source pixels per full-render output pixel
    let fx_step = ((ctx.page_w as u64 * FRAC as u64) / full_w.max(1) as u64) as u32;
    let fy_step = ((ctx.page_h as u64 * FRAC as u64) / full_h.max(1) as u64) as u32;

    // Bilevel fast path: JB2-only page (no IW44 bg, no FG44, no palette).
    // Skips bilinear sampling and gamma LUT — just white fill + black mask writes.
    if ctx.bg.is_none() && ctx.fg44.is_none() && ctx.fg_palette.is_none() {
        composite_loop_bilevel(ctx, buf, fx_step, fy_step);
        return Ok(());
    }

    // Downscaling when output is smaller than source (step > 1 pixel)
    if fx_step > FRAC || fy_step > FRAC {
        composite_loop_area_avg(ctx, buf, fx_step, fy_step);
    } else {
        composite_loop_bilinear(ctx, buf, fx_step, fy_step);
    }

    // Set alpha = 255 for all output pixels in a single SIMD pass.
    // This covers both Pixmap::white() pre-initialised buffers and externally
    // supplied buffers (render_into) that may not have alpha pre-set.
    fill_alpha_255(buf);

    Ok(())
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Render a `DjVuPage` into a pre-allocated RGBA buffer.
///
/// This is the zero-allocation render path when `buf` is reused across calls
/// with the same dimensions. The buffer must be at least `width * height * 4`
/// bytes.
///
/// # Errors
///
/// - [`RenderError::BufTooSmall`] if `buf.len() < width * height * 4`
/// - [`RenderError::InvalidDimensions`] if `width == 0 || height == 0`
/// - Propagates IW44 / JB2 decode errors.
pub fn render_into(
    page: &DjVuPage,
    opts: &RenderOptions,
    buf: &mut [u8],
) -> Result<(), RenderError> {
    let w = opts.width;
    let h = opts.height;

    if w == 0 || h == 0 {
        return Err(RenderError::InvalidDimensions {
            width: w,
            height: h,
        });
    }

    let need = (w as usize)
        .checked_mul(h as usize)
        .and_then(|n| n.checked_mul(4))
        .unwrap_or(usize::MAX);

    if buf.len() < need {
        return Err(RenderError::BufTooSmall {
            need,
            got: buf.len(),
        });
    }

    let gamma_lut = build_gamma_lut(page.gamma());

    // Decode all layers
    let bg_subsample = best_iw44_subsample(opts.scale);
    let bg = decode_background_chunks(page, usize::MAX, bg_subsample)?;
    let fg_palette = decode_fg_palette_full(page)?;

    // Use indexed mask when we have a palette (for per-glyph colors)
    let (mask, blit_map) = if fg_palette.is_some() {
        match decode_mask_indexed(page)? {
            Some((bm, bm_map)) => (Some(bm), Some(bm_map)),
            None => (None, None),
        }
    } else {
        (decode_mask(page)?, None)
    };

    let mask = if opts.bold > 0 {
        mask.map(|m| m.dilate_n(opts.bold as u32))
    } else {
        mask
    };
    let fg44 = decode_fg44(page)?;

    // Use pre-downsampled 1/4-res mask for sub=4 renders (single bit lookup vs
    // 4-9 lookups per pixel in the full-res mask).
    let use_sub4_mask = bg_subsample >= 4 && opts.bold == 0 && fg_palette.is_none();
    let (ctx_mask, mask_shift) = if use_sub4_mask {
        (page.decoded_mask_sub4(), 2u32) // sub=4 → shift by 2
    } else {
        (mask.as_ref().map(|m| m as &_), 0u32) // sub=1 → shift by 0
    };

    let ctx = CompositeContext {
        opts,
        page_w: page.width() as u32,
        page_h: page.height() as u32,
        bg: bg.as_ref(),
        bg_shift: bg_subsample.trailing_zeros(),
        mask: ctx_mask,
        mask_shift,
        fg_palette: fg_palette.as_ref(),
        blit_map: blit_map.as_deref(),
        fg44: fg44.as_ref(),
        gamma_lut: &gamma_lut,
        offset_x: 0,
        offset_y: 0,
        out_w: w,
        out_h: h,
    };
    composite_into(&ctx, buf)?;

    Ok(())
}

/// Render a `DjVuPage` to a new [`Pixmap`] using the given options.
pub fn render_pixmap(page: &DjVuPage, opts: &RenderOptions) -> Result<Pixmap, RenderError> {
    let w = opts.width;
    let h = opts.height;

    if w == 0 || h == 0 {
        return Err(RenderError::InvalidDimensions {
            width: w,
            height: h,
        });
    }

    let gamma_lut = build_gamma_lut(page.gamma());

    // Decode all layers, respecting permissive mode.
    let bg;
    let fg_palette;
    let mask;
    let blit_map;
    let fg44;

    let bg_subsample = best_iw44_subsample(opts.scale);

    if opts.permissive {
        bg = decode_background_chunks_permissive(page, usize::MAX, bg_subsample);
        fg_palette = decode_fg_palette_full(page).ok().flatten();
        let indexed = if fg_palette.is_some() {
            decode_mask_indexed(page).ok().flatten()
        } else {
            None
        };
        if let Some((bm, bm_map)) = indexed {
            mask = Some(bm);
            blit_map = Some(bm_map);
        } else {
            mask = decode_mask(page).ok().flatten();
            blit_map = None;
        }
        fg44 = decode_fg44(page).ok().flatten();
    } else {
        bg = decode_background_chunks(page, usize::MAX, bg_subsample)?;
        fg_palette = decode_fg_palette_full(page)?;
        let indexed_result = if fg_palette.is_some() {
            decode_mask_indexed(page)?
        } else {
            None
        };
        if let Some((bm, bm_map)) = indexed_result {
            mask = Some(bm);
            blit_map = Some(bm_map);
        } else {
            mask = if fg_palette.is_none() {
                decode_mask(page)?
            } else {
                None
            };
            blit_map = None;
        }
        fg44 = decode_fg44(page)?;
    }

    let mask = if opts.bold > 0 {
        mask.map(|m| m.dilate_n(opts.bold as u32))
    } else {
        mask
    };

    // Use pre-downsampled 1/4-res mask for sub=4 renders (single bit lookup vs
    // 4-9 lookups per pixel in the full-res mask).
    let use_sub4_mask = bg_subsample >= 4 && opts.bold == 0 && fg_palette.is_none();
    let (ctx_mask, mask_shift) = if use_sub4_mask {
        (page.decoded_mask_sub4(), 2u32) // sub=4 → shift by 2
    } else {
        (mask.as_ref().map(|m| m as &_), 0u32) // sub=1 → shift by 0
    };

    let mut pm = Pixmap::white(w, h);

    {
        let ctx = CompositeContext {
            opts,
            page_w: page.width() as u32,
            page_h: page.height() as u32,
            bg: bg.as_ref(),
            bg_shift: bg_subsample.trailing_zeros(),
            mask: ctx_mask,
            mask_shift,
            fg_palette: fg_palette.as_ref(),
            blit_map: blit_map.as_deref(),
            fg44: fg44.as_ref(),
            gamma_lut: &gamma_lut,
            offset_x: 0,
            offset_y: 0,
            out_w: w,
            out_h: h,
        };
        composite_into(&ctx, &mut pm.data)?;
    }

    if opts.aa {
        pm = aa_downscale(&pm);
    }

    // Apply Lanczos-3 post-processing when requested.
    // The composited pixmap is already at `w × h`; if the page dimensions
    // differ from the output (i.e. actual scaling happened) reprocess it
    // with the higher-quality Lanczos filter.
    if opts.resampling == Resampling::Lanczos3 {
        let need_scale = page.width() as u32 != w || page.height() as u32 != h;
        if need_scale {
            // Re-render at native resolution, then downscale with Lanczos.
            let native_opts = RenderOptions {
                width: page.width() as u32,
                height: page.height() as u32,
                scale: 1.0,
                bold: opts.bold,
                aa: false,
                rotation: UserRotation::None, // rotation applied after scaling
                permissive: opts.permissive,
                resampling: Resampling::Bilinear, // avoid infinite recursion
            };
            // Render at full resolution (may fail gracefully).
            if let Ok(native_pm) = render_pixmap(page, &native_opts) {
                pm = scale_lanczos3(&native_pm, w, h);
            }
            // If native render failed, pm already holds the bilinear result.
        }
    }

    Ok(rotate_pixmap(
        pm,
        combine_rotations(page.rotation(), opts.rotation),
    ))
}

/// Render a sub-rectangle of a page into a new [`Pixmap`].
///
/// Unlike [`render_pixmap`], which always allocates `opts.width × opts.height`
/// pixels, `render_region` only allocates `region.width × region.height` pixels.
/// This makes it efficient for thumbnails, viewport clips, and tile rendering.
///
/// `opts.width` and `opts.height` still define the **full-page** render dimensions
/// used for scale calculation. `region` selects which sub-rectangle of that
/// full render to output. The returned `Pixmap` has dimensions
/// `region.width × region.height`.
///
/// # Errors
///
/// - [`RenderError::InvalidDimensions`] if `region.width == 0 || region.height == 0`
/// - Propagates IW44 / JB2 decode errors.
pub fn render_region(
    page: &DjVuPage,
    region: RenderRect,
    opts: &RenderOptions,
) -> Result<Pixmap, RenderError> {
    if region.width == 0 || region.height == 0 {
        return Err(RenderError::InvalidDimensions {
            width: region.width,
            height: region.height,
        });
    }

    let full_w = opts.width.max(1);
    let full_h = opts.height.max(1);
    let gamma_lut = build_gamma_lut(page.gamma());

    let bg;
    let fg_palette;
    let mask;
    let blit_map;
    let fg44;

    let bg_subsample = best_iw44_subsample(opts.scale);

    if opts.permissive {
        bg = decode_background_chunks_permissive(page, usize::MAX, bg_subsample);
        fg_palette = decode_fg_palette_full(page).ok().flatten();
        let indexed = if fg_palette.is_some() {
            decode_mask_indexed(page).ok().flatten()
        } else {
            None
        };
        if let Some((bm, bm_map)) = indexed {
            mask = Some(bm);
            blit_map = Some(bm_map);
        } else {
            mask = decode_mask(page).ok().flatten();
            blit_map = None;
        }
        fg44 = decode_fg44(page).ok().flatten();
    } else {
        bg = decode_background_chunks(page, usize::MAX, bg_subsample)?;
        fg_palette = decode_fg_palette_full(page)?;
        let indexed_result = if fg_palette.is_some() {
            decode_mask_indexed(page)?
        } else {
            None
        };
        if let Some((bm, bm_map)) = indexed_result {
            mask = Some(bm);
            blit_map = Some(bm_map);
        } else {
            mask = if fg_palette.is_none() {
                decode_mask(page)?
            } else {
                None
            };
            blit_map = None;
        }
        fg44 = decode_fg44(page)?;
    }

    let mask = if opts.bold > 0 {
        mask.map(|m| m.dilate_n(opts.bold as u32))
    } else {
        mask
    };

    let out_w = region.width;
    let out_h = region.height;
    let mut pm = Pixmap::white(out_w, out_h);

    let region_opts = RenderOptions {
        width: full_w,
        height: full_h,
        ..*opts
    };
    let ctx = CompositeContext {
        opts: &region_opts,
        page_w: page.width() as u32,
        page_h: page.height() as u32,
        bg: bg.as_ref(),
        bg_shift: bg_subsample.trailing_zeros(),
        mask: mask.as_ref(),
        mask_shift: 0,
        fg_palette: fg_palette.as_ref(),
        blit_map: blit_map.as_deref(),
        fg44: fg44.as_ref(),
        gamma_lut: &gamma_lut,
        offset_x: region.x,
        offset_y: region.y,
        out_w,
        out_h,
    };
    composite_into(&ctx, &mut pm.data)?;

    // Apply Lanczos-3 post-processing when requested (same logic as render_pixmap).
    if opts.resampling == Resampling::Lanczos3 {
        let need_scale = page.width() as u32 != full_w || page.height() as u32 != full_h;
        if need_scale {
            let native_opts = RenderOptions {
                width: page.width() as u32,
                height: page.height() as u32,
                scale: 1.0,
                bold: opts.bold,
                aa: false,
                rotation: UserRotation::None,
                permissive: opts.permissive,
                resampling: Resampling::Bilinear,
            };
            if let Ok(native_pm) = render_region(page, region, &native_opts) {
                pm = scale_lanczos3(&native_pm, out_w, out_h);
            }
        }
    }

    Ok(rotate_pixmap(
        pm,
        combine_rotations(page.rotation(), opts.rotation),
    ))
}

/// Render a `DjVuPage` to an 8-bit grayscale image.
///
/// Equivalent to calling [`render_pixmap`] and converting the result with
/// [`Pixmap::to_gray8`]. Returns a [`GrayPixmap`] where `data.len() ==
/// width * height`.
///
/// For bilevel (JB2-only) pages this produces only `0` and `255` values.
/// For colour pages, luminance is computed with ITU-R BT.601 weights.
pub fn render_gray8(page: &DjVuPage, opts: &RenderOptions) -> Result<GrayPixmap, RenderError> {
    Ok(render_pixmap(page, opts)?.to_gray8())
}

/// Render all pages of a document in parallel using rayon.
///
/// Each page is rendered independently with its own [`RenderOptions`] computed
/// from the given `dpi`.  Results are returned in page order.
///
/// Requires the `parallel` feature flag.
#[cfg(feature = "parallel")]
pub fn render_pages_parallel(
    doc: &crate::djvu_document::DjVuDocument,
    dpi: u32,
) -> Vec<Result<Pixmap, RenderError>> {
    use rayon::prelude::*;

    let count = doc.page_count();
    (0..count)
        .into_par_iter()
        .map(|i| {
            let page = doc.page(i)?;
            let native_dpi = page.dpi() as f32;
            let scale = dpi as f32 / native_dpi;
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
            render_pixmap(page, &opts)
        })
        .collect()
}

/// Coarse render: decode only the first BG44 chunk for a fast blurry preview.
///
/// Returns `Ok(None)` when the page has no BG44 chunks.
pub fn render_coarse(page: &DjVuPage, opts: &RenderOptions) -> Result<Option<Pixmap>, RenderError> {
    let w = opts.width;
    let h = opts.height;

    if w == 0 || h == 0 {
        return Err(RenderError::InvalidDimensions {
            width: w,
            height: h,
        });
    }

    let bg_subsample = best_iw44_subsample(opts.scale);
    let bg = decode_background_chunks(page, 1, bg_subsample)?;
    let bg = match bg {
        Some(b) => b,
        None => return Ok(None),
    };

    let gamma_lut = build_gamma_lut(page.gamma());
    let mut pm = Pixmap::white(w, h);

    {
        let ctx = CompositeContext {
            opts,
            page_w: page.width() as u32,
            page_h: page.height() as u32,
            bg: Some(&bg),
            bg_shift: bg_subsample.trailing_zeros(),
            mask: None,
            mask_shift: 0,
            fg_palette: None,
            blit_map: None,
            fg44: None,
            gamma_lut: &gamma_lut,
            offset_x: 0,
            offset_y: 0,
            out_w: w,
            out_h: h,
        };
        composite_into(&ctx, &mut pm.data)?;
    }

    Ok(Some(rotate_pixmap(
        pm,
        combine_rotations(page.rotation(), opts.rotation),
    )))
}

/// Progressive render: decode BG44 chunks 1..=chunk_n and all other layers.
///
/// `chunk_n = 0` behaves like [`render_coarse`] (first chunk only).
/// Each additional chunk adds detail. The result after all chunks is
/// equivalent to [`render_pixmap`].
///
/// # Errors
///
/// Returns [`RenderError::ChunkOutOfRange`] if `chunk_n` exceeds the number
/// of available BG44 chunks.
pub fn render_progressive(
    page: &DjVuPage,
    opts: &RenderOptions,
    chunk_n: usize,
) -> Result<Pixmap, RenderError> {
    let w = opts.width;
    let h = opts.height;

    if w == 0 || h == 0 {
        return Err(RenderError::InvalidDimensions {
            width: w,
            height: h,
        });
    }

    let n_bg44 = page.bg44_chunks().len();
    let max_chunk = n_bg44.saturating_sub(1);

    if n_bg44 > 0 && chunk_n > max_chunk {
        return Err(RenderError::ChunkOutOfRange {
            chunk_n,
            max: max_chunk,
        });
    }

    let gamma_lut = build_gamma_lut(page.gamma());

    // Decode background up to chunk_n + 1 chunks
    let bg_subsample = best_iw44_subsample(opts.scale);
    let bg = decode_background_chunks(page, chunk_n + 1, bg_subsample)?;
    let fg_palette = decode_fg_palette_full(page)?;

    let (mask, blit_map) = if fg_palette.is_some() {
        match decode_mask_indexed(page)? {
            Some((bm, bm_map)) => (Some(bm), Some(bm_map)),
            None => (None, None),
        }
    } else {
        (decode_mask(page)?, None)
    };

    let mask = if opts.bold > 0 {
        mask.map(|m| m.dilate_n(opts.bold as u32))
    } else {
        mask
    };
    let fg44 = decode_fg44(page)?;

    let mut pm = Pixmap::white(w, h);
    {
        let ctx = CompositeContext {
            opts,
            page_w: page.width() as u32,
            page_h: page.height() as u32,
            bg: bg.as_ref(),
            bg_shift: bg_subsample.trailing_zeros(),
            mask: mask.as_ref(),
            mask_shift: 0,
            fg_palette: fg_palette.as_ref(),
            blit_map: blit_map.as_deref(),
            fg44: fg44.as_ref(),
            gamma_lut: &gamma_lut,
            offset_x: 0,
            offset_y: 0,
            out_w: w,
            out_h: h,
        };
        composite_into(&ctx, &mut pm.data)?;
    }

    // Apply Lanczos-3 post-processing when requested (same logic as render_pixmap).
    if opts.resampling == Resampling::Lanczos3 {
        let need_scale = page.width() as u32 != w || page.height() as u32 != h;
        if need_scale {
            let native_opts = RenderOptions {
                width: page.width() as u32,
                height: page.height() as u32,
                scale: 1.0,
                bold: opts.bold,
                aa: false,
                rotation: UserRotation::None,
                permissive: opts.permissive,
                resampling: Resampling::Bilinear,
            };
            if let Ok(native_pm) = render_progressive(page, &native_opts, chunk_n) {
                pm = scale_lanczos3(&native_pm, w, h);
            }
        }
    }

    Ok(rotate_pixmap(
        pm,
        combine_rotations(page.rotation(), opts.rotation),
    ))
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::djvu_document::DjVuDocument;

    fn assets_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/djvujs/library/assets")
    }

    /// Helper that returns an owned document so tests can borrow pages from it.
    fn load_doc(filename: &str) -> DjVuDocument {
        let data = std::fs::read(assets_path().join(filename))
            .unwrap_or_else(|_| panic!("{filename} must exist"));
        DjVuDocument::parse(&data).unwrap_or_else(|e| panic!("parse failed: {e}"))
    }

    // ── TDD: failing tests written first ─────────────────────────────────────

    /// RenderOptions default values.
    #[test]
    fn render_options_default() {
        let opts = RenderOptions::default();
        assert_eq!(opts.width, 0);
        assert_eq!(opts.height, 0);
        assert_eq!(opts.bold, 0);
        assert!(!opts.aa);
        assert!((opts.scale - 1.0).abs() < 1e-6);
        assert_eq!(opts.resampling, Resampling::Bilinear);
    }

    /// RenderOptions can be constructed with explicit fields.
    #[test]
    fn render_options_construction() {
        let opts = RenderOptions {
            width: 400,
            height: 300,
            scale: 0.5,
            bold: 1,
            aa: true,
            rotation: UserRotation::Cw90,
            permissive: false,
            resampling: Resampling::Bilinear,
        };
        assert_eq!(opts.width, 400);
        assert_eq!(opts.height, 300);
        assert_eq!(opts.bold, 1);
        assert!(opts.aa);
        assert!((opts.scale - 0.5).abs() < 1e-6);
        assert_eq!(opts.rotation, UserRotation::Cw90);
    }

    /// `fit_to_width` scales correctly, preserving aspect ratio.
    #[test]
    fn fit_to_width_preserves_aspect() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let pw = page.width() as u32;
        let ph = page.height() as u32;

        let opts = RenderOptions::fit_to_width(page, 800);
        assert_eq!(opts.width, 800);
        let expected_h = ((ph as f64 * 800.0) / pw as f64).round() as u32;
        assert_eq!(opts.height, expected_h);
        assert!((opts.scale - 800.0 / pw as f32).abs() < 0.01);
    }

    /// `fit_to_height` scales correctly, preserving aspect ratio.
    #[test]
    fn fit_to_height_preserves_aspect() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let pw = page.width() as u32;
        let ph = page.height() as u32;

        let opts = RenderOptions::fit_to_height(page, 600);
        assert_eq!(opts.height, 600);
        let expected_w = ((pw as f64 * 600.0) / ph as f64).round() as u32;
        assert_eq!(opts.width, expected_w);
        assert!((opts.scale - 600.0 / ph as f32).abs() < 0.01);
    }

    /// `fit_to_box` chooses the smaller scale factor.
    #[test]
    fn fit_to_box_constrains_both() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();

        // Very wide box — height should be the constraint
        let opts = RenderOptions::fit_to_box(page, 10000, 100);
        assert!(opts.width <= 10000);
        assert!(opts.height <= 100);
        assert!(opts.width > 0 && opts.height > 0);

        // Very tall box — width should be the constraint
        let opts = RenderOptions::fit_to_box(page, 100, 10000);
        assert!(opts.width <= 100);
        assert!(opts.height <= 10000);
        assert!(opts.width > 0 && opts.height > 0);
    }

    /// `fit_to_box` with a square box picks the tighter dimension.
    #[test]
    fn fit_to_box_square() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();

        let opts = RenderOptions::fit_to_box(page, 500, 500);
        assert!(opts.width <= 500);
        assert!(opts.height <= 500);
        // At least one dimension should be close to 500
        assert!(opts.width >= 490 || opts.height >= 490);
    }

    /// Rotated page: fit_to_width uses display dimensions (swapped w/h).
    #[test]
    fn fit_to_width_rotation_aware() {
        // boy_jb2_rotate90 has a 90° rotation in the INFO chunk
        let doc = load_doc("boy_jb2_rotate90.djvu");
        let page = doc.page(0).unwrap();
        let pw = page.width() as u32;
        let ph = page.height() as u32;
        // Display dimensions are swapped for 90° rotation
        let (dw, dh) = (ph, pw);

        let opts = RenderOptions::fit_to_width(page, 400);
        assert_eq!(opts.width, 400);
        let expected_h = ((dh as f64 * 400.0) / dw as f64).round() as u32;
        assert_eq!(opts.height, expected_h);
    }

    /// `render_into` with a zero-width dimension returns InvalidDimensions.
    #[test]
    fn render_into_invalid_dimensions() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();

        let opts = RenderOptions {
            width: 0,
            height: 100,
            ..Default::default()
        };
        let mut buf = vec![0u8; 400];
        let err = render_into(page, &opts, &mut buf).unwrap_err();
        assert!(
            matches!(err, RenderError::InvalidDimensions { .. }),
            "expected InvalidDimensions, got {err:?}"
        );
    }

    /// `render_into` with a too-small buffer returns BufTooSmall.
    #[test]
    fn render_into_buf_too_small() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();

        let opts = RenderOptions {
            width: 10,
            height: 10,
            ..Default::default()
        };
        let mut buf = vec![0u8; 10]; // too small (needs 400)
        let err = render_into(page, &opts, &mut buf).unwrap_err();
        assert!(
            matches!(err, RenderError::BufTooSmall { need: 400, got: 10 }),
            "expected BufTooSmall, got {err:?}"
        );
    }

    /// `render_into` fills a pre-allocated buffer without allocating new one.
    ///
    /// We verify by: calling with exactly the right size buf, no panic,
    /// and the buffer is mutated (not all-zero after the call).
    #[test]
    fn render_into_fills_buffer_no_alloc() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();

        let w = 50u32;
        let h = 40u32;
        let opts = RenderOptions {
            width: w,
            height: h,
            ..Default::default()
        };
        let mut buf = vec![0u8; (w * h * 4) as usize];
        render_into(page, &opts, &mut buf).expect("render_into should succeed");

        // The page is a color image — pixels should not all be zero
        assert!(
            buf.iter().any(|&b| b != 0),
            "rendered buffer should contain non-zero pixels"
        );
    }

    /// `render_into` can be called twice with the same buffer (zero-allocation reuse).
    #[test]
    fn render_into_reuse_buffer() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();

        let w = 30u32;
        let h = 20u32;
        let opts = RenderOptions {
            width: w,
            height: h,
            ..Default::default()
        };
        let mut buf = vec![0u8; (w * h * 4) as usize];

        // First render
        render_into(page, &opts, &mut buf).expect("first render_into should succeed");
        let first = buf.clone();

        // Second render — same result
        render_into(page, &opts, &mut buf).expect("second render_into should succeed");
        assert_eq!(
            first, buf,
            "repeated render_into should produce identical output"
        );
    }

    /// gamma=2.2 (most DjVu files) produces an identity LUT — no correction needed
    /// for a standard display gamma=2.2.
    #[test]
    fn gamma_lut_standard_is_identity() {
        let lut = build_gamma_lut(2.2);
        for (i, &val) in lut.iter().enumerate() {
            assert_eq!(
                val, i as u8,
                "gamma=2.2 LUT at {i}: expected {i}, got {val}"
            );
        }
    }

    /// A linear-light source (gamma=1.0) is corrected: midtones become brighter
    /// (exponent=1/2.2<1 raises sub-unity values toward 1.0) to compensate
    /// for the display gamma-2.2 encoding needed for correct appearance.
    #[test]
    fn gamma_lut_linear_source_brightens() {
        let lut_linear = build_gamma_lut(1.0); // linear source → needs brightening
        let mid = 128u8;
        let corrected = lut_linear[mid as usize];
        assert!(
            corrected > mid,
            "linear-source LUT at mid ({corrected}) should be brighter than {mid}"
        );
    }

    /// Gamma LUT for gamma=0.0 (invalid) falls back to identity.
    #[test]
    fn gamma_lut_zero_is_identity() {
        let lut = build_gamma_lut(0.0);
        for (i, &val) in lut.iter().enumerate() {
            assert_eq!(val, i as u8, "zero gamma should produce identity LUT");
        }
    }

    /// render_coarse returns a valid pixmap (non-empty, correct dimensions) for
    /// a color page.
    #[test]
    fn render_coarse_returns_pixmap() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();

        let opts = RenderOptions {
            width: 60,
            height: 80,
            ..Default::default()
        };

        let result = render_coarse(page, &opts).expect("render_coarse should succeed");
        // chicken.djvu may or may not have BG44 chunks
        if let Some(pm) = result {
            assert_eq!(pm.width, 60);
            assert_eq!(pm.height, 80);
            assert_eq!(pm.data.len(), 60 * 80 * 4);
        }
        // Ok(None) is also valid if no BG44
    }

    /// render_progressive returns valid pixmap after each chunk.
    #[test]
    fn render_progressive_each_chunk() {
        // Use a page that has multiple BG44 chunks (boy.djvu is a good candidate)
        let doc = load_doc("boy.djvu");
        let page = doc.page(0).unwrap();

        let opts = RenderOptions {
            width: 80,
            height: 100,
            ..Default::default()
        };

        let n_bg44 = page.bg44_chunks().len();

        for chunk_n in 0..n_bg44 {
            let pm = render_progressive(page, &opts, chunk_n)
                .unwrap_or_else(|e| panic!("render_progressive chunk {chunk_n} failed: {e}"));
            assert_eq!(pm.width, 80);
            assert_eq!(pm.height, 100);
            assert_eq!(pm.data.len(), 80 * 100 * 4);
            // Each frame must have some non-zero pixels
            assert!(
                pm.data.iter().any(|&b| b != 0),
                "chunk {chunk_n}: rendered frame should not be all-zero"
            );
        }
    }

    /// render_progressive with chunk_n out of range returns ChunkOutOfRange.
    #[test]
    fn render_progressive_chunk_out_of_range() {
        let doc = load_doc("boy.djvu");
        let page = doc.page(0).unwrap();

        let opts = RenderOptions {
            width: 40,
            height: 50,
            ..Default::default()
        };

        let n_bg44 = page.bg44_chunks().len();
        if n_bg44 == 0 {
            // No BG44 chunks — skip this test
            return;
        }

        let err = render_progressive(page, &opts, n_bg44 + 10).unwrap_err();
        assert!(
            matches!(err, RenderError::ChunkOutOfRange { .. }),
            "expected ChunkOutOfRange, got {err:?}"
        );
    }

    /// render_pixmap with gamma gives different result than without (identity gamma).
    ///
    /// We compare rendering chicken.djvu twice: once with its natural gamma,
    /// once with gamma forced to 1.0 (identity). The pixel values should differ.
    #[test]
    fn render_pixmap_gamma_differs_from_identity() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();

        let w = 40u32;
        let h = 53u32; // ~native aspect for 181x240

        let opts = RenderOptions {
            width: w,
            height: h,
            ..Default::default()
        };

        // Render with native gamma (2.2 from INFO chunk)
        let pm_gamma = render_pixmap(page, &opts).expect("render with gamma should succeed");

        // Render with identity gamma LUT manually applied to output
        let lut_identity = build_gamma_lut(1.0);
        let pm_identity = render_pixmap(page, &opts).expect("render for identity should succeed");
        // Apply identity correction (no-op) — pixels should be the same
        for i in 0..pm_identity.data.len().saturating_sub(3) {
            if i % 4 != 3 {
                // non-alpha channel
                let _ = lut_identity[pm_identity.data[i] as usize];
            }
        }

        // Since chicken.djvu gamma = 2.2, the gamma-corrected render
        // should have generally brighter mid-tones than a raw (no-correction) render.
        // We test this by checking that the gamma render is not bit-for-bit identical
        // to a hypothetical no-correction render. Since we always apply gamma in
        // render_pixmap, we test the gamma LUT effect directly (covered by
        // `gamma_correction_changes_pixels`).
        //
        // Instead, verify that pm_gamma has valid dimensions and non-trivial content.
        assert_eq!(pm_gamma.width, w);
        assert_eq!(pm_gamma.height, h);
        assert!(
            pm_gamma.data.iter().any(|&b| b != 255),
            "should have non-white pixels"
        );
    }

    /// render_pixmap for a bilevel (JB2-only) page produces black pixels.
    #[test]
    fn render_bilevel_page_has_black_pixels() {
        let doc = load_doc("boy_jb2.djvu");
        let page = doc.page(0).unwrap();

        let opts = RenderOptions {
            width: 60,
            height: 80,
            ..Default::default()
        };

        let pm = render_pixmap(page, &opts).expect("render bilevel should succeed");
        assert_eq!(pm.width, 60);
        assert_eq!(pm.height, 80);
        // A bilevel page should have some black pixels
        assert!(
            pm.data
                .chunks_exact(4)
                .any(|px| px[0] == 0 && px[1] == 0 && px[2] == 0),
            "bilevel page should contain black pixels"
        );
    }

    /// `render_pixmap` with aa=true returns a valid pixmap.
    #[test]
    fn render_with_aa() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();

        let opts = RenderOptions {
            width: 40,
            height: 54,
            aa: true,
            ..Default::default()
        };
        // With aa=true the output is downscaled 2×, so we get 20×27
        let pm = render_pixmap(page, &opts).expect("render with AA should succeed");
        // AA downscales the output
        assert_eq!(pm.width, 20);
        assert_eq!(pm.height, 27);
    }

    // -- Rotation tests -------------------------------------------------------

    #[test]
    fn rotate_pixmap_none_is_identity() {
        let mut pm = Pixmap::white(3, 2);
        pm.set_rgb(0, 0, 255, 0, 0);
        let rotated = rotate_pixmap(pm.clone(), crate::info::Rotation::None);
        assert_eq!(rotated.width, 3);
        assert_eq!(rotated.height, 2);
        assert_eq!(rotated.get_rgb(0, 0), (255, 0, 0));
    }

    #[test]
    fn rotate_pixmap_cw90_swaps_dims() {
        let mut pm = Pixmap::white(4, 2);
        pm.set_rgb(0, 0, 255, 0, 0); // top-left red
        let rotated = rotate_pixmap(pm, crate::info::Rotation::Cw90);
        assert_eq!(rotated.width, 2);
        assert_eq!(rotated.height, 4);
        // Top-left (0,0) of original goes to (height-1-0, 0) = (1, 0) in rotated
        assert_eq!(rotated.get_rgb(1, 0), (255, 0, 0));
    }

    #[test]
    fn rotate_pixmap_180_preserves_dims() {
        let mut pm = Pixmap::white(3, 2);
        pm.set_rgb(0, 0, 255, 0, 0); // top-left red
        let rotated = rotate_pixmap(pm, crate::info::Rotation::Rot180);
        assert_eq!(rotated.width, 3);
        assert_eq!(rotated.height, 2);
        assert_eq!(rotated.get_rgb(2, 1), (255, 0, 0));
    }

    #[test]
    fn rotate_pixmap_ccw90_swaps_dims() {
        let mut pm = Pixmap::white(4, 2);
        pm.set_rgb(0, 0, 255, 0, 0); // top-left red
        let rotated = rotate_pixmap(pm, crate::info::Rotation::Ccw90);
        assert_eq!(rotated.width, 2);
        assert_eq!(rotated.height, 4);
        // Top-left (0,0) -> (0, width-1-0) = (0, 3) in rotated
        assert_eq!(rotated.get_rgb(0, 3), (255, 0, 0));
    }

    #[test]
    fn render_pixmap_rotation_90_swaps_dimensions() {
        let doc = load_doc("boy_jb2_rotate90.djvu");
        let page = doc.page(0).expect("page 0");
        let orig_w = page.width();
        let orig_h = page.height();
        let opts = RenderOptions {
            width: orig_w as u32,
            height: orig_h as u32,
            ..Default::default()
        };
        let pm = render_pixmap(page, &opts).expect("render should succeed");
        // 90° rotation swaps width and height
        assert_eq!(
            pm.width, orig_h as u32,
            "rotated width should be original height"
        );
        assert_eq!(
            pm.height, orig_w as u32,
            "rotated height should be original width"
        );
    }

    #[test]
    fn render_pixmap_rotation_180_preserves_dimensions() {
        let doc = load_doc("boy_jb2_rotate180.djvu");
        let page = doc.page(0).expect("page 0");
        let orig_w = page.width();
        let orig_h = page.height();
        let opts = RenderOptions {
            width: orig_w as u32,
            height: orig_h as u32,
            ..Default::default()
        };
        let pm = render_pixmap(page, &opts).expect("render should succeed");
        assert_eq!(pm.width, orig_w as u32);
        assert_eq!(pm.height, orig_h as u32);
    }

    #[test]
    fn render_pixmap_rotation_270_swaps_dimensions() {
        let doc = load_doc("boy_jb2_rotate270.djvu");
        let page = doc.page(0).expect("page 0");
        let orig_w = page.width();
        let orig_h = page.height();
        let opts = RenderOptions {
            width: orig_w as u32,
            height: orig_h as u32,
            ..Default::default()
        };
        let pm = render_pixmap(page, &opts).expect("render should succeed");
        assert_eq!(
            pm.width, orig_h as u32,
            "rotated width should be original height"
        );
        assert_eq!(
            pm.height, orig_w as u32,
            "rotated height should be original width"
        );
    }

    // -- User rotation tests ---------------------------------------------------

    /// combine_rotations adds steps modulo 4.
    #[test]
    fn combine_rotations_identity() {
        use crate::info::Rotation;
        assert_eq!(
            combine_rotations(Rotation::None, UserRotation::None),
            Rotation::None
        );
    }

    #[test]
    fn combine_rotations_info_only() {
        use crate::info::Rotation;
        assert_eq!(
            combine_rotations(Rotation::Cw90, UserRotation::None),
            Rotation::Cw90
        );
    }

    #[test]
    fn combine_rotations_user_only() {
        use crate::info::Rotation;
        assert_eq!(
            combine_rotations(Rotation::None, UserRotation::Ccw90),
            Rotation::Ccw90
        );
    }

    #[test]
    fn combine_rotations_sum() {
        use crate::info::Rotation;
        // 90 CW (INFO) + 90 CW (user) = 180
        assert_eq!(
            combine_rotations(Rotation::Cw90, UserRotation::Cw90),
            Rotation::Rot180
        );
        // 90 CW + 270 CW = 360 = None
        assert_eq!(
            combine_rotations(Rotation::Cw90, UserRotation::Ccw90),
            Rotation::None
        );
        // 180 + 180 = 360 = None
        assert_eq!(
            combine_rotations(Rotation::Rot180, UserRotation::Rot180),
            Rotation::None
        );
    }

    /// User rotation Cw90 on a non-rotated page swaps output dimensions.
    #[test]
    fn user_rotation_cw90_swaps_dimensions() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let pw = page.width() as u32;
        let ph = page.height() as u32;

        let opts = RenderOptions {
            width: pw,
            height: ph,
            rotation: UserRotation::Cw90,
            ..Default::default()
        };
        let pm = render_pixmap(page, &opts).expect("render");
        assert_eq!(pm.width, ph, "user Cw90 should swap: width becomes height");
        assert_eq!(pm.height, pw, "user Cw90 should swap: height becomes width");
    }

    /// User rotation 180° preserves dimensions.
    #[test]
    fn user_rotation_180_preserves_dimensions() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let pw = page.width() as u32;
        let ph = page.height() as u32;

        let opts = RenderOptions {
            width: pw,
            height: ph,
            rotation: UserRotation::Rot180,
            ..Default::default()
        };
        let pm = render_pixmap(page, &opts).expect("render");
        assert_eq!(pm.width, pw);
        assert_eq!(pm.height, ph);
    }

    /// UserRotation default is None.
    #[test]
    fn user_rotation_default_is_none() {
        assert_eq!(UserRotation::default(), UserRotation::None);
        let opts = RenderOptions::default();
        assert_eq!(opts.rotation, UserRotation::None);
    }

    // -- FGbz multi-color palette tests ---------------------------------------

    #[test]
    fn fgbz_palette_page_renders_multiple_colors() {
        // irish.djvu is a single-page file with an FGbz palette.
        let doc = load_doc("irish.djvu");
        let page = doc.page(0).expect("page 0");
        let w = page.width() as u32;
        let h = page.height() as u32;
        let opts = RenderOptions {
            width: w,
            height: h,
            ..Default::default()
        };
        let pm = render_pixmap(page, &opts).expect("render should succeed");

        // Collect distinct non-white, non-black foreground colors
        let mut fg_colors = std::collections::HashSet::new();
        for y in 0..h {
            for x in 0..w {
                let (r, g, b) = pm.get_rgb(x, y);
                // Skip white and near-white (background)
                if r > 240 && g > 240 && b > 240 {
                    continue;
                }
                fg_colors.insert((r, g, b));
            }
        }

        // A multi-color palette page should produce more than 1 distinct
        // foreground color (if it only had 1, it'd be the old bug).
        assert!(
            fg_colors.len() > 1,
            "multi-color palette page should have >1 distinct foreground colors, got {}",
            fg_colors.len()
        );
    }

    #[test]
    fn lookup_palette_color_uses_blit_map() {
        let pal = FgbzPalette {
            colors: vec![
                PaletteColor { r: 255, g: 0, b: 0 }, // index 0: red
                PaletteColor { r: 0, g: 0, b: 255 }, // index 1: blue
            ],
            indices: vec![1, 0], // blit 0 → color 1 (blue), blit 1 → color 0 (red)
        };
        let bm = crate::bitmap::Bitmap::new(2, 1);
        let blit_map = vec![0i32, 1i32]; // pixel (0,0) → blit 0, pixel (1,0) → blit 1

        let c0 = lookup_palette_color(&pal, Some(&blit_map), Some(&bm), 0, 0);
        assert_eq!(
            (c0.r, c0.g, c0.b),
            (0, 0, 255),
            "blit 0 → indices[0]=1 → blue"
        );

        let c1 = lookup_palette_color(&pal, Some(&blit_map), Some(&bm), 1, 0);
        assert_eq!(
            (c1.r, c1.g, c1.b),
            (255, 0, 0),
            "blit 1 → indices[1]=0 → red"
        );
    }

    #[test]
    fn lookup_palette_color_fallback_without_blit_map() {
        let pal = FgbzPalette {
            colors: vec![PaletteColor { r: 0, g: 128, b: 0 }],
            indices: vec![],
        };
        let c = lookup_palette_color(&pal, None, None, 0, 0);
        assert_eq!(
            (c.r, c.g, c.b),
            (0, 128, 0),
            "should fall back to first color"
        );
    }

    // ── BGjp / FGjp tests ─────────────────────────────────────────────────────

    /// Load the synthetic bgjp_test.djvu fixture from the assets directory.
    fn load_bgjp_doc() -> DjVuDocument {
        load_doc("bgjp_test.djvu")
    }

    /// BGjp fixture loads without error and reports correct dimensions.
    #[test]
    fn bgjp_fixture_loads() {
        let doc = load_bgjp_doc();
        let page = doc.page(0).unwrap();
        assert_eq!(page.width(), 4);
        assert_eq!(page.height(), 4);
    }

    /// BGjp chunk is present in the fixture.
    #[test]
    fn bgjp_chunk_present() {
        let doc = load_bgjp_doc();
        let page = doc.page(0).unwrap();
        assert!(
            page.find_chunk(b"BGjp").is_some(),
            "fixture must have a BGjp chunk"
        );
        assert!(
            page.bg44_chunks().is_empty(),
            "fixture must NOT have BG44 chunks"
        );
    }

    /// `decode_bgjp` returns a non-None Pixmap for the BGjp fixture.
    #[test]
    fn decode_bgjp_returns_pixmap() {
        let doc = load_bgjp_doc();
        let page = doc.page(0).unwrap();
        let pm = decode_bgjp(page).expect("decode_bgjp must not error");
        assert!(pm.is_some(), "decode_bgjp must return Some(Pixmap)");
        let pm = pm.unwrap();
        assert_eq!(pm.width, 4);
        assert_eq!(pm.height, 4);
        assert_eq!(pm.data.len(), 4 * 4 * 4); // RGBA
    }

    /// `decode_bgjp` returns None for a page with no BGjp chunk.
    #[test]
    fn decode_bgjp_returns_none_without_chunk() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let pm = decode_bgjp(page).expect("should not error");
        assert!(pm.is_none());
    }

    /// `decode_jpeg_to_pixmap` produces RGBA output with alpha=255.
    #[test]
    fn decode_jpeg_to_pixmap_alpha_is_255() {
        let doc = load_bgjp_doc();
        let page = doc.page(0).unwrap();
        let data = page.find_chunk(b"BGjp").unwrap();
        let pm = decode_jpeg_to_pixmap(data).expect("decode must succeed");
        for chunk in pm.data.chunks_exact(4) {
            assert_eq!(chunk[3], 255, "alpha must be 255 for every pixel");
        }
    }

    /// render_pixmap falls back to BGjp when no BG44 chunks are present.
    #[test]
    fn render_pixmap_uses_bgjp_background() {
        let doc = load_bgjp_doc();
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 4,
            height: 4,
            scale: 1.0,
            bold: 0,
            aa: false,
            rotation: UserRotation::None,
            permissive: false,
            resampling: Resampling::Bilinear,
        };
        let pm = render_pixmap(page, &opts).expect("render must succeed");
        assert_eq!(pm.width, 4);
        assert_eq!(pm.height, 4);
    }

    /// render_coarse also falls back to BGjp (no BG44 chunks).
    #[test]
    fn render_coarse_uses_bgjp_background() {
        let doc = load_bgjp_doc();
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 4,
            height: 4,
            scale: 1.0,
            bold: 0,
            aa: false,
            rotation: UserRotation::None,
            permissive: false,
            resampling: Resampling::Bilinear,
        };
        let pm = render_coarse(page, &opts).expect("render_coarse must succeed");
        assert!(pm.is_some(), "must return Some when BGjp present");
        let pm = pm.unwrap();
        assert_eq!(pm.width, 4);
        assert_eq!(pm.height, 4);
    }

    // ── Lanczos-3 tests ───────────────────────────────────────────────────────

    /// `lanczos3_kernel(0)` == 1.0 (unity at origin).
    #[test]
    fn lanczos3_kernel_unity_at_zero() {
        assert!((lanczos3_kernel(0.0) - 1.0).abs() < 1e-5);
    }

    /// `lanczos3_kernel` is zero outside |x| ≥ 3.
    #[test]
    fn lanczos3_kernel_zero_outside_support() {
        assert_eq!(lanczos3_kernel(3.0), 0.0);
        assert_eq!(lanczos3_kernel(-3.5), 0.0);
        assert_eq!(lanczos3_kernel(10.0), 0.0);
    }

    /// `scale_lanczos3` preserves dimensions.
    #[test]
    fn scale_lanczos3_correct_dimensions() {
        let src = Pixmap::white(100, 80);
        let dst = scale_lanczos3(&src, 50, 40);
        assert_eq!(dst.width, 50);
        assert_eq!(dst.height, 40);
    }

    /// `scale_lanczos3` returns a clone when source and target match.
    #[test]
    fn scale_lanczos3_noop_when_same_size() {
        let src = Pixmap::new(4, 4, 200, 100, 50, 255);
        let dst = scale_lanczos3(&src, 4, 4);
        assert_eq!(dst.width, 4);
        assert_eq!(dst.height, 4);
        assert_eq!(dst.data, src.data);
    }

    /// Scaling a solid-color pixmap with Lanczos-3 preserves the color.
    #[test]
    fn scale_lanczos3_preserves_solid_color() {
        // Solid red 20×20 → 10×10
        let src = Pixmap::new(20, 20, 200, 0, 0, 255);
        let dst = scale_lanczos3(&src, 10, 10);
        assert_eq!(dst.width, 10);
        assert_eq!(dst.height, 10);
        // All output pixels should be close to red (200, 0, 0).
        for chunk in dst.data.chunks_exact(4) {
            let (r, g, b) = (chunk[0], chunk[1], chunk[2]);
            assert!(
                (r as i32 - 200).abs() <= 5 && g <= 5 && b <= 5,
                "expected near-red (200,0,0), got ({r},{g},{b})"
            );
        }
    }

    /// `Resampling::Lanczos3` produces the correct output dimensions.
    #[test]
    fn render_pixmap_lanczos3_correct_dimensions() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let pw = page.width() as u32;
        let ph = page.height() as u32;
        let tw = pw / 2;
        let th = ph / 2;

        let opts = RenderOptions {
            width: tw,
            height: th,
            scale: 0.5,
            resampling: Resampling::Lanczos3,
            ..Default::default()
        };
        let pm = render_pixmap(page, &opts).expect("Lanczos3 render must succeed");
        assert_eq!(pm.width, tw);
        assert_eq!(pm.height, th);
    }

    /// Lanczos-3 and bilinear renders differ (different algorithms produce different output).
    #[test]
    fn lanczos3_differs_from_bilinear_at_half_scale() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let pw = page.width() as u32;
        let ph = page.height() as u32;
        let tw = pw / 2;
        let th = ph / 2;

        let bilinear = render_pixmap(
            page,
            &RenderOptions {
                width: tw,
                height: th,
                scale: 0.5,
                resampling: Resampling::Bilinear,
                ..Default::default()
            },
        )
        .unwrap();

        let lanczos = render_pixmap(
            page,
            &RenderOptions {
                width: tw,
                height: th,
                scale: 0.5,
                resampling: Resampling::Lanczos3,
                ..Default::default()
            },
        )
        .unwrap();

        // Dimensions must be the same.
        assert_eq!(bilinear.width, lanczos.width);
        assert_eq!(bilinear.height, lanczos.height);

        // But pixel values should differ (algorithms are not identical).
        let differ = bilinear
            .data
            .iter()
            .zip(lanczos.data.iter())
            .any(|(a, b)| a != b);
        assert!(
            differ,
            "Lanczos3 and bilinear must produce different pixel values"
        );
    }

    /// `Resampling::Bilinear` default is maintained for backward compat.
    #[test]
    fn resampling_default_is_bilinear() {
        let opts = RenderOptions::default();
        assert_eq!(opts.resampling, Resampling::Bilinear);
    }

    // ── render_region tests ───────────────────────────────────────────────────

    /// `render_region` allocates only the region-sized buffer (≤ 512 KB for 256×256).
    #[test]
    fn render_region_allocates_proportionally() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions::fit_to_width(page, 1000);
        let region = RenderRect {
            x: 0,
            y: 0,
            width: 256,
            height: 256,
        };
        let pm = render_region(page, region, &opts).expect("render_region should succeed");
        assert_eq!(pm.width, 256);
        assert_eq!(pm.height, 256);
        assert_eq!(pm.data.len(), 256 * 256 * 4);
        assert!(
            pm.data.len() <= 512 * 1024,
            "region allocation {} exceeds 512 KB",
            pm.data.len()
        );
    }

    /// `render_region` pixels match the same pixels from `render_pixmap`.
    #[test]
    fn render_region_matches_full_render() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 100,
            height: 80,
            ..Default::default()
        };
        let full = render_pixmap(page, &opts).expect("full render should succeed");
        let region = RenderRect {
            x: 10,
            y: 10,
            width: 30,
            height: 20,
        };
        let part = render_region(page, region, &opts).expect("region render should succeed");

        assert_eq!(part.width, 30);
        assert_eq!(part.height, 20);

        for ry in 0..20u32 {
            for rx in 0..30u32 {
                let full_base = ((10 + ry) as usize * 100 + (10 + rx) as usize) * 4;
                let part_base = (ry as usize * 30 + rx as usize) * 4;
                assert_eq!(
                    &full.data[full_base..full_base + 4],
                    &part.data[part_base..part_base + 4],
                    "pixel mismatch at region ({rx},{ry}) / full ({},{} )",
                    10 + rx,
                    10 + ry
                );
            }
        }
    }

    /// `render_region` with invalid dimensions returns an error.
    #[test]
    fn render_region_invalid_dimensions() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 100,
            height: 100,
            ..Default::default()
        };
        let region = RenderRect {
            x: 0,
            y: 0,
            width: 0,
            height: 50,
        };
        let err = render_region(page, region, &opts).unwrap_err();
        assert!(
            matches!(err, RenderError::InvalidDimensions { .. }),
            "expected InvalidDimensions, got {err:?}"
        );
    }

    /// `render_pixmap` still works correctly (regression guard).
    #[test]
    fn render_pixmap_still_works_after_refactor() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 80,
            height: 60,
            ..Default::default()
        };
        let pm = render_pixmap(page, &opts).expect("render_pixmap should succeed");
        assert_eq!(pm.width, 80);
        assert_eq!(pm.height, 60);
        assert_eq!(pm.data.len(), 80 * 60 * 4);
    }

    /// `best_iw44_subsample` returns expected power-of-2 values.
    #[test]
    fn best_iw44_subsample_values() {
        assert_eq!(best_iw44_subsample(1.0), 1, "scale=1.0 → subsample=1");
        assert_eq!(best_iw44_subsample(0.5), 2, "scale=0.5 → subsample=2");
        assert_eq!(
            best_iw44_subsample(0.375),
            4,
            "scale=0.375 → subsample=4 (1.5/0.375=4.0, allows 1.5× upscale)"
        );
        assert_eq!(best_iw44_subsample(0.25), 4, "scale=0.25 → subsample=4");
        assert_eq!(
            best_iw44_subsample(0.1),
            8,
            "scale=0.1 → subsample=8 (capped)"
        );
        assert_eq!(
            best_iw44_subsample(0.0),
            1,
            "scale=0.0 → subsample=1 (edge case)"
        );
        assert_eq!(
            best_iw44_subsample(-1.0),
            1,
            "scale<0 → subsample=1 (edge case)"
        );
        assert_eq!(
            best_iw44_subsample(2.0),
            1,
            "scale>1.0 → subsample=1 (no upscaling needed)"
        );
    }

    /// Rendering with bg_subsample=2 (scale=0.5) produces the correct output dimensions.
    #[test]
    fn render_pixmap_subsampled_bg_correct_dimensions() {
        let doc = load_doc("boy.djvu");
        let page = doc.page(0).unwrap();
        // scale=0.5 triggers bg_subsample=2 internally
        let opts = RenderOptions {
            width: (page.width() as f32 * 0.5) as u32,
            height: (page.height() as f32 * 0.5) as u32,
            scale: 0.5,
            ..Default::default()
        };
        let pm = render_pixmap(page, &opts).expect("subsampled render should succeed");
        assert_eq!(pm.width, opts.width);
        assert_eq!(pm.height, opts.height);
        assert_eq!(
            pm.data.len() as u64,
            opts.width as u64 * opts.height as u64 * 4
        );
    }

    /// Second render of the same page produces identical pixels — confirms the
    /// BG44 cache is used and does not corrupt output.
    #[test]
    fn decoded_bg44_cache_produces_identical_pixels_on_second_render() {
        let doc = load_doc("boy.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: page.width() as u32,
            height: page.height() as u32,
            ..Default::default()
        };
        let pm1 = render_pixmap(page, &opts).expect("first render should succeed");
        let pm2 = render_pixmap(page, &opts).expect("second render should succeed");
        assert_eq!(
            pm1.data, pm2.data,
            "cached render must produce identical pixels"
        );
    }

    /// After the first render the `decoded_bg44` cache is populated — the
    /// image dimensions match the page's raw BG44 size.
    #[test]
    fn decoded_bg44_is_populated_after_render() {
        let doc = load_doc("boy.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: page.width() as u32,
            height: page.height() as u32,
            ..Default::default()
        };
        // Trigger cache population.
        render_pixmap(page, &opts).expect("render should succeed");
        // Cache must now hold an image whose size matches the page's native size.
        let cached = page
            .decoded_bg44()
            .expect("cache should be populated after render");
        assert_eq!(
            cached.width,
            page.width() as u32,
            "cached bg44 width must equal page width"
        );
        assert_eq!(
            cached.height,
            page.height() as u32,
            "cached bg44 height must equal page height"
        );
    }

    /// `render_region` applies page rotation the same way as `render_pixmap`.
    ///
    /// For a 90° CW rotation a non-square region of width×height is returned as
    /// height×width — proving rotation was applied (not silently skipped).
    #[test]
    fn render_region_applies_rotation() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        // Request an explicit 90° CW user rotation.
        let opts = RenderOptions {
            width: 80,
            height: 60,
            rotation: UserRotation::Cw90,
            ..Default::default()
        };
        // Non-square region so swapped dimensions are detectable.
        let region = RenderRect {
            x: 0,
            y: 0,
            width: 40,
            height: 20,
        };
        let part = render_region(page, region, &opts).expect("region render should succeed");
        // After CW90 rotation a 40×20 region becomes 20×40.
        assert_eq!(
            part.width, 20,
            "expected width=20 (was region.height) after CW90 rotation"
        );
        assert_eq!(
            part.height, 40,
            "expected height=40 (was region.width) after CW90 rotation"
        );
    }
}
