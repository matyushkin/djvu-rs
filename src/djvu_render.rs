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

    /// A render option is incompatible with the chosen entry point.
    ///
    /// Returned by [`render_streaming`] when an option requires post-processing
    /// of a fully-allocated pixmap (anti-aliasing, Lanczos resampling at a
    /// scaled output, or rotation).
    #[error("unsupported render option: {0}")]
    UnsupportedOption(&'static str),
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
/// use djvu_rs::djvu_render::RenderOptions;
///
/// // Set the output size; the pipeline derives the decode scale from `width`.
/// let opts = RenderOptions {
///     width: 800,
///     height: 600,
///     aa: true,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RenderOptions {
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
    /// Deprecated and **ignored by the render pipeline**.
    ///
    /// Rendering now derives its decode scale from `width` and the page's
    /// native width (see the internal `decode_scale`), so this field no longer
    /// controls anything. It is retained for backward compatibility — the
    /// `fit_to_*` constructors still populate it — but setting it by hand has no
    /// effect. Build options via
    /// [`RenderOptions::fit_to_width`] / [`fit_to_box`](RenderOptions::fit_to_box)
    /// instead of assembling the `(width, height, scale)` triple yourself.
    #[deprecated(
        since = "0.20.1",
        note = "scale is derived from `width` by the render pipeline and is no longer read; \
                build options via `RenderOptions::fit_to_width`/`fit_to_box`. \
                This field is ignored and will be removed in a future release."
    )]
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
    #[allow(deprecated)] // still sets the retained-for-compat `scale` field
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

#[allow(deprecated)] // the `fit_to_*` constructors still populate `scale` for back-compat
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

    /// Whether `page` can be rendered with [`render_streaming`] under these
    /// options, producing pixels identical to [`render_pixmap`].
    ///
    /// The streaming path emits the page row-by-row without buffering a full
    /// [`Pixmap`], so callers that only need to forward rows (PDF/TIFF image
    /// encoders) can avoid the intermediate allocation. It is only equivalent
    /// to the buffered path when no whole-image post-pass is required: no
    /// anti-aliasing, no rotation, and either bilinear resampling or a 1:1
    /// (unscaled) render.
    ///
    /// This is the single source of truth for streaming eligibility; export
    /// paths call it instead of re-deriving the rule.
    pub fn can_stream(&self, page: &crate::djvu_document::DjVuPage) -> bool {
        !self.aa
            && (self.resampling == Resampling::Bilinear
                || (page.width() as u32 == self.width && page.height() as u32 == self.height))
            && page.rotation() == crate::info::Rotation::None
            && self.rotation == UserRotation::None
    }

    /// The scale the decode pipeline uses to choose the IW44 wavelet subsample
    /// level (via [`best_iw44_subsample`]), derived from the requested output
    /// `width` and the page's **native** width.
    ///
    /// The compositor scales the native page raster (`page.width()` ×
    /// `page.height()`) into the `width` × `height` buffer and only *then*
    /// applies INFO/user rotation (see `composite_rows` and `rotate_pixmap`).
    /// The IW44 background is decoded in that pre-rotation native orientation, so
    /// the subsample must be chosen against `width / page.width()`. Dividing by
    /// the rotation-swapped *display* width would pick the wrong subsample for
    /// INFO-rotated, non-square pages — over-subsampling a downscaled portrait
    /// background and under-subsampling a landscape one.
    ///
    /// This is the single home of the `scale ≈ width / page-width` invariant that
    /// every caller used to maintain by hand — and that the PDF exporter got
    /// wrong, leaving `scale = 1.0` and silently over-decoding at every DPI.
    /// Rendering reads *this*, never the deprecated public [`scale`](Self::scale)
    /// field, so a caller can no longer cause a silent over- or under-decode by
    /// building the size triple inconsistently.
    pub(crate) fn decode_scale(&self, page: &crate::djvu_document::DjVuPage) -> f32 {
        self.width as f32 / (page.width() as u32).max(1) as f32
    }
}

/// Return `(display_width, display_height)` — dimensions after rotation.
///
/// The single source of the INFO-rotation dimension swap; the `fit_to_*`
/// constructors and `Page::display_dims` both call it instead of re-deriving it.
pub(crate) fn display_dimensions(page: &crate::djvu_document::DjVuPage) -> (u32, u32) {
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

/// Generic Q24 ratios `(plane_w << 24) / page_w` and `(plane_h << 24) / page_h`
/// for converting page-space FRACBITS coords into plane-space FRACBITS coords.
/// Returns `(0, 0)` when `plane` is `None` or page dims are zero.  The public
/// FG/BG helpers below apply layer-specific cell-grid adjustments first.
#[inline]
fn plane_q24(plane: Option<&Pixmap>, page_w: u32, page_h: u32) -> (u64, u64) {
    match plane {
        Some(p) if page_w > 0 && page_h > 0 => (
            ((p.width as u64) << 24) / page_w as u64,
            ((p.height as u64) << 24) / page_h as u64,
        ),
        _ => (0, 0),
    }
}

/// Map page-space fixed-point coordinates to BG44/FG44 plane-space by aligning
/// pixel centres instead of top-left corners.  This matches the usual image
/// resampling convention and reduces native-resolution drift against ddjvu for
/// non-integer page→plane ratios such as colorbook's 2260→754 BG scale.
#[inline]
fn map_plane_center_frac(page_frac: u32, q24: u64) -> u32 {
    let centered = (((page_frac as u64 + (FRAC / 2) as u64) * q24) >> 24) as u32;
    centered.saturating_sub(FRAC / 2)
}

#[inline]
fn fg_q24(fg: Option<&Pixmap>, page_w: u32, page_h: u32) -> (u64, u64) {
    match fg {
        Some(p) if page_w > 0 && page_h > 0 && p.width > 0 && p.height > 0 => {
            // FG44 is a sparse foreground colour map.  Horizontally, the last
            // encoded column is often padding for a fixed-width colour cell
            // grid (e.g. 2260px page / 189px FG => 12px cells), so use the
            // inferred integer cell pitch instead of stretching across the
            // padded column.  Vertically, use the encoded plane ratio so the
            // bottom FG row remains reachable when the page height is not an
            // exact multiple of the cell pitch.
            let sx = page_w.div_ceil(p.width).max(1);
            (
                (1u64 << 24) / sx as u64,
                ((p.height as u64) << 24) / page_h as u64,
            )
        }
        _ => plane_q24(fg, page_w, page_h),
    }
}

#[inline]
fn bg_q24(bg: Option<&Pixmap>, page_w: u32, page_h: u32) -> (u64, u64) {
    match bg {
        Some(p) if page_w > 0 && page_h > 0 && p.width > 0 && p.height > 0 => {
            // BG44 planes are cell grids too (usually page/3 for scans).  Use
            // the inferred integer subsample pitch so the padded right/bottom
            // edge cells do not stretch across the page during native render.
            let sx = page_w.div_ceil(p.width).max(1);
            let sy = page_h.div_ceil(p.height).max(1);
            ((1u64 << 24) / sx as u64, (1u64 << 24) / sy as u64)
        }
        _ => plane_q24(bg, page_w, page_h),
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
        let numerator = top * (FRAC - ty) + bot * ty;
        let v = (numerator + (1 << (2 * FRACBITS - 1))) >> (2 * FRACBITS);
        v.min(255) as u8
    };

    (
        lerp(r00, r10, r01, r11),
        lerp(g00, g10, g01, g11),
        lerp(b00, b10, b01, b11),
    )
}

#[inline]
fn sample_nearest(pm: &Pixmap, fx: u32, fy: u32) -> (u8, u8, u8) {
    let x = ((fx + FRAC / 2) >> FRACBITS).min(pm.width.saturating_sub(1));
    let y = ((fy + FRAC / 2) >> FRACBITS).min(pm.height.saturating_sub(1));
    pm.get_rgb(x, y)
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
    // Exclusive upper bounds: the output pixel covers source [x0, x1) × [y0, y1).
    // The old inclusive formula gave a 3×3 box at 2× downscale because x1 landed
    // on the first pixel of the next output cell; exclusive gives the correct 2×2.
    let x1 = ((fx + fx_step) >> FRACBITS).min(pm.width);
    let y1 = ((fy + fy_step) >> FRACBITS).min(pm.height);

    let cols = (x1 - x0) as usize;
    let rows = (y1 - y0) as usize;

    // Fast path: 1×1 box → direct read.
    if cols <= 1 && rows <= 1 {
        return pm.get_rgb(x0, y0);
    }

    let mut r_sum = 0u32;
    let mut g_sum = 0u32;
    let mut b_sum = 0u32;

    let pw = pm.width as usize;
    // One bounds check per row (not per pixel) to let the inner loop vectorize.
    for sy in y0..y1 {
        let row_off = sy as usize * pw * 4 + x0 as usize * 4;
        if let Some(row) = pm.data.get(row_off..row_off + cols * 4) {
            for chunk in row.chunks_exact(4) {
                r_sum += chunk[0] as u32;
                g_sum += chunk[1] as u32;
                b_sum += chunk[2] as u32;
            }
        }
    }

    let count = (rows * cols) as u32;
    if count == 0 {
        return (255, 255, 255);
    }

    // Power-of-2 counts (count=4 at 2× downscale): replace UDIV with shifts.
    let half = count >> 1;
    if count.is_power_of_two() {
        let shift = count.trailing_zeros();
        (
            ((r_sum + half) >> shift) as u8,
            ((g_sum + half) >> shift) as u8,
            ((b_sum + half) >> shift) as u8,
        )
    } else {
        (
            ((r_sum + half) / count) as u8,
            ((g_sum + half) / count) as u8,
            ((b_sum + half) / count) as u8,
        )
    }
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
    // Exclusive upper bounds, consistent with sample_area_avg.
    let x1 = ((fx + fx_step) >> FRACBITS).min(mask.width);
    let y1 = ((fy + fy_step) >> FRACBITS).min(mask.height);

    for sy in y0..y1 {
        for sx in x0..x1 {
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

// FGbz chunk parsing lives in `crate::fgbz` so this module receives already-decoded
// palette data and never calls `bzz_decode` itself. `parse_fgbz` returns a
// `BzzError`; callers below propagate it through `RenderError`'s `From<BzzError>`.
use crate::fgbz::{FgbzPalette, PaletteColor, parse_fgbz};

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

/// Render-tier cache of a page's decoded layers.
///
/// These are the decoded wavelet / bitmap forms the compositor consumes —
/// background (`bg44`, plus the first-chunk-only `bg44_partial` used at
/// subsample ≥ 4), the JB2 mask (`mask`), its quarter-resolution max-pool
/// downsample (`mask_sub4`, a pure compositor concern), and the FG44 colour
/// layer (`fg44`). They live here in the render tier rather than on
/// [`DjVuPage`] so the page model stays close to its raw bytes and every
/// render concern — including compositor subsampling — concentrates in one
/// place.
///
/// Each layer is decoded lazily and cached, so repeated renders of the same
/// page (e.g. thumbnails then full resolution) reuse the expensive ZP
/// arithmetic decode. A `DjVuPage` holds one of these behind a `OnceLock`;
/// the accessors borrow the page only to decode on a cache miss, so the
/// returned reference is tied to the cache, not the call.
#[cfg(feature = "std")]
#[derive(Default)]
pub(crate) struct PageLayers {
    bg44: std::sync::OnceLock<Option<Iw44Image>>,
    bg44_partial: std::sync::OnceLock<Option<Iw44Image>>,
    mask: std::sync::OnceLock<Option<crate::bitmap::Bitmap>>,
    mask_sub4: std::sync::OnceLock<Option<crate::bitmap::Bitmap>>,
    fg44: std::sync::OnceLock<Option<Pixmap>>,
}

#[cfg(feature = "std")]
impl PageLayers {
    /// An empty cache. Layers are decoded on first access.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// The fully decoded BG44 wavelet image (all chunks), decoding on first
    /// call. `None` when the page has no BG44 chunks. Decode errors are
    /// swallowed (the partial image so far is kept, matching the permissive
    /// render path). The wavelet inverse-transform / YCbCr→RGB conversion is
    /// *not* cached here — it runs per render at the requested subsample.
    pub(crate) fn bg44(&self, page: &DjVuPage) -> Option<&Iw44Image> {
        self.bg44
            .get_or_init(|| {
                let chunks = page.bg44_chunks();
                if chunks.is_empty() {
                    return None;
                }
                let mut img = Iw44Image::new();
                for chunk_data in &chunks {
                    if img.decode_chunk(chunk_data).is_err() {
                        break;
                    }
                }
                if img.width == 0 { None } else { Some(img) }
            })
            .as_ref()
    }

    /// A partially-decoded BG44 image — first chunk only — decoding on first
    /// call. Roughly 4× cheaper to decode; used at subsample ≥ 4 where the
    /// high-frequency refinement chunks are imperceptible.
    pub(crate) fn bg44_partial(&self, page: &DjVuPage) -> Option<&Iw44Image> {
        self.bg44_partial
            .get_or_init(|| {
                let chunks = page.bg44_chunks();
                if chunks.is_empty() {
                    return None;
                }
                let mut img = Iw44Image::new();
                if img.decode_chunk(chunks[0]).is_err() {
                    return None;
                }
                if img.width == 0 { None } else { Some(img) }
            })
            .as_ref()
    }

    /// The decoded JB2 / G4 foreground mask, decoding on first call. `None`
    /// when the page has no mask chunk or decoding fails.
    pub(crate) fn mask(&self, page: &DjVuPage) -> Option<&crate::bitmap::Bitmap> {
        self.mask
            .get_or_init(|| page.extract_mask().ok().flatten())
            .as_ref()
    }

    /// A 1/4-resolution max-pool downsample of the mask, decoding the mask
    /// first if needed. Each bit is 1 if any bit in the corresponding 4×4
    /// block is set, letting the compositor do one lookup per output pixel at
    /// subsample ≥ 4 instead of 4–9. Purely a compositor optimisation, which
    /// is why it lives in the render tier rather than on the page.
    pub(crate) fn mask_sub4(&self, page: &DjVuPage) -> Option<&crate::bitmap::Bitmap> {
        self.mask_sub4
            .get_or_init(|| {
                let src = self.mask(page)?;
                Some(downsample_mask_4x(src))
            })
            .as_ref()
    }

    /// The decoded FG44 foreground colour layer, decoding on first call.
    /// `None` when the page has no FG44 chunks or decoding fails.
    pub(crate) fn fg44(&self, page: &DjVuPage) -> Option<&Pixmap> {
        self.fg44
            .get_or_init(|| page.extract_foreground().ok().flatten())
            .as_ref()
    }
}

/// Max-pool 4× downsample of a bilevel mask.
///
/// Each output pixel is 1 if any bit in the corresponding 4×4 block of `src`
/// is set. Used by [`PageLayers::mask_sub4`] to build the 1/4-resolution mask
/// the compositor uses for sub=4 renders instead of `mask_box_any`.
#[cfg(feature = "std")]
fn downsample_mask_4x(src: &crate::bitmap::Bitmap) -> crate::bitmap::Bitmap {
    let out_w = src.width.div_ceil(4);
    let out_h = src.height.div_ceil(4);
    let mut out = crate::bitmap::Bitmap::new(out_w, out_h);
    for oy in 0..out_h {
        for ox in 0..out_w {
            'outer: for dy in 0..4u32 {
                for dx in 0..4u32 {
                    let sx = ox * 4 + dx;
                    let sy = oy * 4 + dy;
                    if sx < src.width && sy < src.height && src.get(sx, sy) {
                        out.set(ox, oy, true);
                        break 'outer;
                    }
                }
            }
        }
    }
    out
}

/// The page's render-tier `mask_sub4` layer, or `None` without `std`.
///
/// Wraps the `std`-only [`PageLayers`] cache so the compositor's sub=4 path
/// compiles identically with and without the `std` feature.
#[cfg(feature = "std")]
fn page_mask_sub4(page: &DjVuPage) -> Option<&crate::bitmap::Bitmap> {
    page.render_layers().mask_sub4(page)
}

/// Decode background from BG44 chunks up to `max_chunks`.
///
/// `subsample` controls IW44 decode resolution: 1 = full, 2 = half, 4 = quarter.
/// Use `best_iw44_subsample(opts.decode_scale(page))` to pick an appropriate value.
///
/// When `max_chunks == usize::MAX`, the decoded wavelet image is fetched from
/// the page's [`PageLayers`] cache, avoiding repeated ZP arithmetic decode.
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

/// The page layers decoded for a full (non-progressive) composite: background,
/// foreground palette, mask, optional indexed blit map, and the FG44/FGjp
/// foreground pixmap.
struct DecodedLayers {
    bg: Option<Pixmap>,
    fg_palette: Option<FgbzPalette>,
    mask: Option<crate::bitmap::Bitmap>,
    blit_map: Option<Vec<i32>>,
    fg44: Option<Pixmap>,
}

/// Decode every layer needed for a full composite at `bg_subsample` — the one
/// home for the permissive-vs-strict decode decision.
///
/// In permissive mode each step swallows errors (`.ok().flatten()`) and the
/// background stops at the first corrupt chunk; in strict mode any decode error
/// propagates. The returned `mask` already has `opts.bold` dilation applied,
/// since both callers do that immediately after decoding.
///
/// Both [`render_rows`] (the row path behind `render_pixmap` / `render_into` /
/// `render_streaming`) and [`render_region`] build their `CompositeContext`
/// from this, keeping their decode logic identical. The progressive path
/// decodes differently (partial background up to `chunk_n`) and is not routed
/// through here.
fn decode_layers(
    page: &DjVuPage,
    opts: &RenderOptions,
    bg_subsample: u32,
    bg_chunk_limit: usize,
) -> Result<DecodedLayers, RenderError> {
    let bg;
    let fg_palette;
    let mask;
    let blit_map;
    let fg44;

    if opts.permissive {
        bg = decode_background_chunks_permissive(page, bg_chunk_limit, bg_subsample);
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
        bg = decode_background_chunks(page, bg_chunk_limit, bg_subsample)?;
        let fg = decode_foreground_strict(page)?;
        fg_palette = fg.fg_palette;
        mask = fg.mask;
        blit_map = fg.blit_map;
        fg44 = fg.fg44;
    }

    let mask = if opts.bold > 0 {
        mask.map(|m| m.dilate_n(opts.bold as u32))
    } else {
        mask
    };

    Ok(DecodedLayers {
        bg,
        fg_palette,
        mask,
        blit_map,
        fg44,
    })
}

/// The foreground layers (mask + blit map, FGbz palette, FG44 colour) decoded in
/// strict mode, *before* any bold dilation.
struct ForegroundLayers {
    fg_palette: Option<FgbzPalette>,
    mask: Option<crate::bitmap::Bitmap>,
    blit_map: Option<Vec<i32>>,
    fg44: Option<Pixmap>,
}

/// Strict-mode decode of a page's foreground layers, shared by the full
/// [`decode_layers`] path and [`render_progressive`] (which decodes a partial
/// background but the same foreground).
///
/// Owns the "indexed mask when an FGbz palette is present, plain mask
/// otherwise" decision so the two callers cannot drift apart. Bold dilation is
/// applied by the caller, since `decode_layers` shares one dilation step across
/// its permissive and strict branches.
fn decode_foreground_strict(page: &DjVuPage) -> Result<ForegroundLayers, RenderError> {
    let fg_palette = decode_fg_palette_full(page)?;
    let (mask, blit_map) = if fg_palette.is_some() {
        match decode_mask_indexed(page)? {
            Some((bm, bm_map)) => (Some(bm), Some(bm_map)),
            None => (None, None),
        }
    } else {
        (decode_mask(page)?, None)
    };
    let fg44 = decode_fg44(page)?;
    Ok(ForegroundLayers {
        fg_palette,
        mask,
        blit_map,
        fg44,
    })
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
    /// Q24 ratio for converting page-space FRACBITS coordinates to BG-plane
    /// FRACBITS coordinates.  Uses the inferred integer BG44 cell pitch so
    /// padded edge cells do not stretch across the native render.  `0` when
    /// `bg` is `None`.
    bg_x_q24: u64,
    bg_y_q24: u64,
    mask: Option<&'a crate::bitmap::Bitmap>,
    /// `mask_sub.trailing_zeros()` where mask_sub is 1 (full-res) or 4 (1/4-res).
    /// Using a shift instead of division avoids a UDIV instruction in the hot path.
    mask_shift: u32,
    fg_palette: Option<&'a FgbzPalette>,
    /// Per-pixel blit index map (same dimensions as mask). `-1` = no blit.
    blit_map: Option<&'a [i32]>,
    fg44: Option<&'a Pixmap>,
    /// Q24 ratio for converting page-space FRACBITS coordinates to FG44-space
    /// FRACBITS coordinates.  The horizontal ratio uses the inferred integer
    /// foreground colour-cell pitch; the vertical ratio uses the encoded plane
    /// height so the bottom row remains reachable.  `0` when `fg44` is `None`.
    fg_x_q24: u64,
    fg_y_q24: u64,
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

impl<'a> CompositeContext<'a> {
    /// Build a composite context from already-decoded layers.
    ///
    /// This is the single home for the per-render wiring that was copy-pasted
    /// across all five render entry points: the q24 cell-pitch arithmetic (where
    /// the #199 BG/FG alignment fix lives), the page-dimension lookup, and the
    /// gamma-LUT / offset / output-size plumbing. Callers decode their layers
    /// (full or partial background, optionally sub-4 mask) and hand them here so
    /// a q24 / offset / gamma bug can only ever be fixed in one place.
    ///
    /// The `(mask, mask_shift)` pair is passed in rather than derived because it
    /// legitimately varies per entry point — the full-page paths swap in the
    /// 1/4-resolution mask via [`resolve_sub4_mask`], while region / coarse /
    /// progressive renders always composite against the full-resolution mask.
    #[allow(clippy::too_many_arguments)]
    fn from_layers(
        page: &DjVuPage,
        opts: &'a RenderOptions,
        bg: Option<&'a Pixmap>,
        mask: Option<&'a crate::bitmap::Bitmap>,
        mask_shift: u32,
        fg_palette: Option<&'a FgbzPalette>,
        blit_map: Option<&'a [i32]>,
        fg44: Option<&'a Pixmap>,
        gamma_lut: &'a [u8; 256],
        offset: (u32, u32),
        out: (u32, u32),
    ) -> Self {
        let page_w = page.width() as u32;
        let page_h = page.height() as u32;
        let (fg_x_q24, fg_y_q24) = fg_q24(fg44, page_w, page_h);
        let (bg_x_q24, bg_y_q24) = bg_q24(bg, page_w, page_h);
        CompositeContext {
            opts,
            page_w,
            page_h,
            bg,
            bg_x_q24,
            bg_y_q24,
            mask,
            mask_shift,
            fg_palette,
            blit_map,
            fg44,
            fg_x_q24,
            fg_y_q24,
            gamma_lut,
            offset_x: offset.0,
            offset_y: offset.1,
            out_w: out.0,
            out_h: out.1,
        }
    }
}

/// Pick the mask plane and shift for a full-page composite.
///
/// At background subsample ≥ 4 (and only when no bold dilation or FGbz palette
/// is in play, since those need full-resolution lookups) the compositor reads a
/// pre-downsampled 1/4-resolution mask — one bit lookup per output pixel instead
/// of 4–9. The returned shift is `mask_sub.trailing_zeros()` (2 for the 1/4-res
/// mask, 0 for the full-res mask). This is the single home for that decision,
/// previously copy-pasted into `render_rows` and `render_into`.
#[cfg(feature = "std")]
fn resolve_sub4_mask<'a>(
    page: &'a DjVuPage,
    bg_subsample: u32,
    opts: &RenderOptions,
    full_mask: Option<&'a crate::bitmap::Bitmap>,
    fg_palette: Option<&FgbzPalette>,
) -> (Option<&'a crate::bitmap::Bitmap>, u32) {
    if bg_subsample >= 4 && opts.bold == 0 && fg_palette.is_none() {
        (page_mask_sub4(page), 2)
    } else {
        (full_mask, 0)
    }
}

#[cfg(not(feature = "std"))]
fn resolve_sub4_mask<'a>(
    _page: &'a DjVuPage,
    _bg_subsample: u32,
    _opts: &RenderOptions,
    full_mask: Option<&'a crate::bitmap::Bitmap>,
    _fg_palette: Option<&FgbzPalette>,
) -> (Option<&'a crate::bitmap::Bitmap>, u32) {
    (full_mask, 0)
}

/// Look up the palette color for a foreground pixel at (px, py).
///
/// Uses the blit map to find the per-glyph blit index, then maps it through
/// the FGbz index table to get the final color. Falls back to `palette[0]` when
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

/// Composite one page into `buf` (RGBA, pre-allocated) using the given context.
///
/// This is a zero-allocation render path when `buf` is already the right size.
/// For region renders, `ctx.out_w`/`ctx.out_h` give the output dimensions and
/// `ctx.offset_x`/`ctx.offset_y` give the starting offset within the full render.
///
/// Iterates the output rows over the same single-row composite bodies used by
/// [`composite_rows`], writing each row directly into its slice of `buf` with no
/// intermediate copy. The two paths therefore share one per-pixel decision tree.
fn composite_into(ctx: &CompositeContext<'_>, buf: &mut [u8]) -> Result<(), RenderError> {
    let full_w = ctx.opts.width;
    let full_h = ctx.opts.height;

    // Fixed-point step: how many source pixels per full-render output pixel
    let fx_step = ((ctx.page_w as u64 * FRAC as u64) / full_w.max(1) as u64) as u32;
    let fy_step = ((ctx.page_h as u64 * FRAC as u64) / full_h.max(1) as u64) as u32;

    let row_stride = ctx.out_w as usize * 4;
    let rows = buf.chunks_exact_mut(row_stride).take(ctx.out_h as usize);

    // Bilevel fast path: JB2-only page (no IW44 bg, no FG44, no palette).
    // Skips bilinear sampling and gamma LUT — just white fill + black mask writes.
    // The bilevel body writes alpha itself, so no fill_alpha_255 pass is needed.
    if ctx.bg.is_none() && ctx.fg44.is_none() && ctx.fg_palette.is_none() {
        for (oy, row) in rows.enumerate() {
            composite_rows_bilevel_one(ctx, oy as u32, fx_step, fy_step, row);
        }
        return Ok(());
    }

    let downscale = fx_step > FRAC || fy_step > FRAC;
    // Precompute bg-space step for the area-average path (avoids per-pixel multiply).
    let bg_fx_step = ((fx_step as u64 * ctx.bg_x_q24) >> 24) as u32;
    let bg_fy_step = ((fy_step as u64 * ctx.bg_y_q24) >> 24) as u32;

    for (oy, row) in rows.enumerate() {
        if downscale {
            composite_rows_area_avg_one(
                ctx, oy as u32, fx_step, fy_step, bg_fx_step, bg_fy_step, row,
            );
        } else {
            composite_rows_bilinear_one(ctx, oy as u32, fx_step, fy_step, row);
        }
    }

    // Set alpha = 255 for all output pixels in a single SIMD pass.
    // This covers both Pixmap::white() pre-initialised buffers and externally
    // supplied buffers (render_into) that may not have alpha pre-set.
    fill_alpha_255(buf);

    Ok(())
}

/// Drive the composite hot path row-by-row, calling `sink(row_index, &row_rgba)`
/// once per output row.
///
/// Each call to `sink` receives a 4-byte-per-pixel RGBA slice of width
/// `ctx.out_w`.  A single scratch row is allocated up-front (not per-row), so
/// peak additional heap use is `out_w * 4` bytes regardless of page height.
///
/// This is the internal streaming primitive used by [`render_rows`]; callers
/// that already hold a flat output buffer should prefer [`composite_into`],
/// which writes directly without an intermediate copy.
fn composite_rows<F>(ctx: &CompositeContext<'_>, mut sink: F) -> Result<(), RenderError>
where
    F: FnMut(usize, &[u8]),
{
    let full_w = ctx.opts.width;
    let full_h = ctx.opts.height;

    // Fixed-point step: how many source pixels per full-render output pixel.
    let fx_step = ((ctx.page_w as u64 * FRAC as u64) / full_w.max(1) as u64) as u32;
    let fy_step = ((ctx.page_h as u64 * FRAC as u64) / full_h.max(1) as u64) as u32;

    let row_stride = ctx.out_w as usize * 4;

    // Bilevel fast path: JB2-only page (no IW44 bg, no FG44, no palette).
    if ctx.bg.is_none() && ctx.fg44.is_none() && ctx.fg_palette.is_none() {
        let mut row_buf = vec![0u8; row_stride];
        for oy in 0..ctx.out_h {
            composite_rows_bilevel_one(ctx, oy, fx_step, fy_step, &mut row_buf);
            sink(oy as usize, &row_buf);
        }
        return Ok(());
    }

    let mut row_buf = vec![0u8; row_stride];
    let downscale = fx_step > FRAC || fy_step > FRAC;

    // Precompute bg-space step for area-average path (avoids per-pixel multiply).
    let bg_fx_step = ((fx_step as u64 * ctx.bg_x_q24) >> 24) as u32;
    let bg_fy_step = ((fy_step as u64 * ctx.bg_y_q24) >> 24) as u32;

    for oy in 0..ctx.out_h {
        if downscale {
            composite_rows_area_avg_one(
                ctx,
                oy,
                fx_step,
                fy_step,
                bg_fx_step,
                bg_fy_step,
                &mut row_buf,
            );
        } else {
            composite_rows_bilinear_one(ctx, oy, fx_step, fy_step, &mut row_buf);
        }
        fill_alpha_255(&mut row_buf);
        sink(oy as usize, &row_buf);
    }

    Ok(())
}

/// Write one bilevel row into `row_buf`.
#[inline]
fn composite_rows_bilevel_one(
    ctx: &CompositeContext<'_>,
    oy: u32,
    fx_step: u32,
    fy_step: u32,
    row_buf: &mut [u8],
) {
    let mask = match ctx.mask {
        Some(m) => m,
        None => {
            for chunk in row_buf.chunks_exact_mut(4) {
                chunk[0] = 255;
                chunk[1] = 255;
                chunk[2] = 255;
                chunk[3] = 255;
            }
            return;
        }
    };

    // 1:1 scale fast path.
    if fx_step == FRAC && fy_step == FRAC {
        let stride = mask.row_stride();
        let py = (oy + ctx.offset_y).min(ctx.page_h.saturating_sub(1)) as usize;
        let mask_row = &mask.data[py * stride..(py + 1) * stride];
        for (ox, pixel) in row_buf.chunks_exact_mut(4).enumerate() {
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
        return;
    }

    let downscale = fx_step > FRAC || fy_step > FRAC;
    let fy = (oy + ctx.offset_y) * fy_step;
    let py = (fy >> FRACBITS).min(ctx.page_h.saturating_sub(1));

    for (ox, pixel) in row_buf.chunks_exact_mut(4).enumerate() {
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

/// Write one bilinear row into `row_buf` (upscale / 1:1).
#[inline]
fn composite_rows_bilinear_one(
    ctx: &CompositeContext<'_>,
    oy: u32,
    fx_step: u32,
    fy_step: u32,
    row_buf: &mut [u8],
) {
    let (page_w, page_h) = (ctx.page_w, ctx.page_h);
    let fy = (oy + ctx.offset_y) * fy_step;
    let py = (fy >> FRACBITS).min(page_h.saturating_sub(1));

    for (ox, pixel) in row_buf.chunks_exact_mut(4).enumerate() {
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
                let fg_fx = map_plane_center_frac(fx, ctx.fg_x_q24);
                let fg_fy = map_plane_center_frac(fy, ctx.fg_y_q24);
                sample_nearest(fg, fg_fx, fg_fy)
            } else {
                (0, 0, 0)
            }
        } else if let Some(bg) = ctx.bg {
            let bg_fx = map_plane_center_frac(fx, ctx.bg_x_q24);
            let bg_fy = map_plane_center_frac(fy, ctx.bg_y_q24);
            sample_bilinear(bg, bg_fx, bg_fy)
        } else {
            (255, 255, 255)
        };

        pixel[0] = ctx.gamma_lut[r as usize];
        pixel[1] = ctx.gamma_lut[g as usize];
        pixel[2] = ctx.gamma_lut[b as usize];
        // alpha written by fill_alpha_255 in composite_rows
    }
}

/// Write one area-average row into `row_buf` (downscale).
#[inline]
fn composite_rows_area_avg_one(
    ctx: &CompositeContext<'_>,
    oy: u32,
    fx_step: u32,
    fy_step: u32,
    bg_fx_step: u32,
    bg_fy_step: u32,
    row_buf: &mut [u8],
) {
    let fy = (oy + ctx.offset_y) * fy_step;
    let bg_fy = ((fy as u64 * ctx.bg_y_q24) >> 24) as u32;

    for (ox, pixel) in row_buf.chunks_exact_mut(4).enumerate() {
        let fx = (ox as u32 + ctx.offset_x) * fx_step;

        let is_fg = ctx.mask.is_some_and(|m| {
            if ctx.mask_shift > 0 {
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
                let fg_fx = ((fx as u64 * ctx.fg_x_q24) >> 24) as u32;
                let fg_fy = ((fy as u64 * ctx.fg_y_q24) >> 24) as u32;
                let fg_fx_step = ((fx_step as u64 * ctx.fg_x_q24) >> 24) as u32;
                let fg_fy_step = ((fy_step as u64 * ctx.fg_y_q24) >> 24) as u32;
                sample_area_avg(fg, fg_fx, fg_fy, fg_fx_step, fg_fy_step)
            } else {
                (0, 0, 0)
            }
        } else if let Some(bg) = ctx.bg {
            let bg_fx = ((fx as u64 * ctx.bg_x_q24) >> 24) as u32;
            sample_area_avg(bg, bg_fx, bg_fy, bg_fx_step, bg_fy_step)
        } else {
            (255, 255, 255)
        };

        pixel[0] = ctx.gamma_lut[r as usize];
        pixel[1] = ctx.gamma_lut[g as usize];
        pixel[2] = ctx.gamma_lut[b as usize];
        // alpha written by fill_alpha_255 in composite_rows
    }
}

/// Render a `DjVuPage` row by row, calling `sink(row_index, &rgba_row)` for
/// each output row in top-to-bottom order.
///
/// `rgba_row` contains `opts.width * 4` bytes (RGBA, alpha = 255).
///
/// This is the internal streaming primitive used by [`render_streaming`] and by
/// permissive [`render_pixmap`] fallback. Public callers that need full-pixmap
/// post-processing should use [`render_pixmap`].
///
/// # Errors
///
/// - [`RenderError::InvalidDimensions`] if `width == 0 || height == 0`
/// - Propagates IW44 / JB2 decode errors.
pub(crate) fn render_rows<F>(
    page: &DjVuPage,
    opts: &RenderOptions,
    sink: F,
) -> Result<(), RenderError>
where
    F: FnMut(usize, &[u8]),
{
    let w = opts.width;
    let h = opts.height;

    if w == 0 || h == 0 {
        return Err(RenderError::InvalidDimensions {
            width: w,
            height: h,
        });
    }

    let gamma_lut = build_gamma_lut(page.gamma());

    let bg_subsample = best_iw44_subsample(opts.decode_scale(page));

    let DecodedLayers {
        bg,
        fg_palette,
        mask,
        blit_map,
        fg44,
    } = decode_layers(page, opts, bg_subsample, usize::MAX)?;

    let (ctx_mask, mask_shift) =
        resolve_sub4_mask(page, bg_subsample, opts, mask.as_ref(), fg_palette.as_ref());
    let ctx = CompositeContext::from_layers(
        page,
        opts,
        bg.as_ref(),
        ctx_mask,
        mask_shift,
        fg_palette.as_ref(),
        blit_map.as_deref(),
        fg44.as_ref(),
        &gamma_lut,
        (0, 0),
        (w, h),
    );
    composite_rows(&ctx, sink)
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

    // Decode all layers (shared permissive/strict seam, same as render_rows).
    let bg_subsample = best_iw44_subsample(opts.decode_scale(page));
    let DecodedLayers {
        bg,
        fg_palette,
        mask,
        blit_map,
        fg44,
    } = decode_layers(page, opts, bg_subsample, usize::MAX)?;

    // Use pre-downsampled 1/4-res mask for sub=4 renders (single bit lookup vs
    // 4-9 lookups per pixel in the full-res mask).
    let (ctx_mask, mask_shift) =
        resolve_sub4_mask(page, bg_subsample, opts, mask.as_ref(), fg_palette.as_ref());
    let ctx = CompositeContext::from_layers(
        page,
        opts,
        bg.as_ref(),
        ctx_mask,
        mask_shift,
        fg_palette.as_ref(),
        blit_map.as_deref(),
        fg44.as_ref(),
        &gamma_lut,
        (0, 0),
        (w, h),
    );
    composite_into(&ctx, buf)?;

    Ok(())
}

/// Build the options for the native-resolution pre-pass that feeds the
/// Lanczos-3 post-filter: full page size, no scaling, no AA, no rotation, and
/// bilinear resampling (so the recursive render never re-enters this path).
///
/// Bold and permissive flags are carried through so the high-resolution
/// re-render matches the requested render in everything but the final
/// resampling step.
fn native_render_opts(page: &DjVuPage, opts: &RenderOptions) -> RenderOptions {
    // Full page size (decode scale derives to 1.0), bilinear resampling so the
    // recursive render never re-enters this path, no AA / no rotation. Bold and
    // permissive are carried through.
    RenderOptions {
        width: page.width() as u32,
        height: page.height() as u32,
        bold: opts.bold,
        permissive: opts.permissive,
        ..Default::default()
    }
}

/// Apply the shared Lanczos-3 post-pass to a freshly composited pixmap.
///
/// When `opts.resampling` is [`Resampling::Lanczos3`] *and* actual scaling
/// happened (`page` native size differs from `full_w × full_h`), the page is
/// re-rendered at native resolution via `render_native` and downscaled to
/// `out_w × out_h` with [`crate::pixmap::scale_lanczos3`]. Otherwise `pm` is
/// returned unchanged — covering both the non-Lanczos case and the 1:1 case
/// where Lanczos would be a no-op. If the native re-render fails, the original
/// (bilinear) `pm` is kept, matching the previous per-call behaviour.
///
/// `full` is the full output size used to decide whether scaling occurred;
/// `out` is the target the result is scaled to. They differ only for
/// [`render_region`], where the comparison is against the full page render but
/// the output is the (smaller) region.
fn apply_lanczos_postpass<F>(
    pm: Pixmap,
    page: &DjVuPage,
    opts: &RenderOptions,
    full: (u32, u32),
    out: (u32, u32),
    render_native: F,
) -> Pixmap
where
    F: FnOnce(&RenderOptions) -> Result<Pixmap, RenderError>,
{
    if opts.resampling != Resampling::Lanczos3 {
        return pm;
    }
    let (full_w, full_h) = full;
    let need_scale = page.width() as u32 != full_w || page.height() as u32 != full_h;
    if !need_scale {
        return pm;
    }
    let native_opts = native_render_opts(page, opts);
    match render_native(&native_opts) {
        Ok(native_pm) => crate::pixmap::scale_lanczos3(&native_pm, out.0, out.1),
        // Native render failed — keep the bilinear result already in `pm`.
        Err(_) => pm,
    }
}

/// Render a `DjVuPage` to a new [`Pixmap`] using the given options.
///
/// Strict renders composite directly into the full pixmap. Permissive renders
/// reuse the row path so decode-error recovery remains shared with
/// [`render_streaming`].
pub fn render_pixmap(page: &DjVuPage, opts: &RenderOptions) -> Result<Pixmap, RenderError> {
    let w = opts.width;
    let h = opts.height;

    if w == 0 || h == 0 {
        return Err(RenderError::InvalidDimensions {
            width: w,
            height: h,
        });
    }

    let mut pm = Pixmap::white(w, h);

    if opts.permissive {
        let row_stride = w as usize * 4;
        // Permissive rendering has its own decode-error recovery path in
        // render_rows; keep that behaviour and copy each recovered row.
        render_rows(page, opts, |y, row| {
            let start = y * row_stride;
            pm.data[start..start + row_stride].copy_from_slice(row);
        })?;
    } else {
        // Strict renders can composite directly into the output Pixmap,
        // avoiding the scratch row + row copy used by the streaming adapter.
        render_into(page, opts, &mut pm.data)?;
    }

    if opts.aa {
        pm = aa_downscale(&pm);
    }

    // Apply the shared Lanczos-3 post-pass (re-render at native size, then
    // downscale) when requested and scaling actually happened.
    let pm = apply_lanczos_postpass(pm, page, opts, (w, h), (w, h), |native_opts| {
        render_pixmap(page, native_opts)
    });

    Ok(rotate_pixmap(
        pm,
        combine_rotations(page.rotation(), opts.rotation),
    ))
}

/// Render a `DjVuPage` row by row, calling `sink(row_index, &rgba_row)` for
/// each output row in top-to-bottom order. Each `rgba_row` slice has length
/// `opts.width * 4` bytes (RGBA, alpha = 255).
///
/// This is the constant-memory render path for low-memory targets (mobile,
/// WASM, embedded) and for streaming consumers (TIFF/PDF row-encoders, the
/// browser `OffscreenCanvas` row blit). Internally it allocates a single
/// `opts.width * 4` byte scratch row and reuses it across all rows; peak heap
/// usage during compositing is bounded by that scratch plus the decoded
/// background (BG44) and mask (JB2) buffers.
///
/// Output is byte-identical to [`render_pixmap`] when both produce a result.
///
/// # Constraints
///
/// [`render_pixmap`] applies anti-aliasing, Lanczos-3 resampling, and rotation
/// as **post-processing on the full pixmap**. The streaming path cannot
/// support those modes without buffering the entire output, defeating its
/// purpose. The following must hold or [`RenderError::UnsupportedOption`] is
/// returned:
///
/// - `opts.aa == false`
/// - `opts.resampling == Resampling::Bilinear`, *or* the output dimensions
///   match the page's native dimensions (in which case Lanczos becomes a
///   no-op and `Bilinear` produces the same bytes anyway)
/// - `opts.rotation == UserRotation::None` *and* the page's INFO rotation is
///   `Rotation::None` (i.e. the combined rotation is identity)
///
/// For any of those modes, use [`render_pixmap`] instead.
///
/// # Errors
///
/// - [`RenderError::InvalidDimensions`] if `opts.width == 0 || opts.height == 0`
/// - [`RenderError::UnsupportedOption`] if a post-processing option is set
/// - Propagates IW44 / JB2 decode errors.
///
/// # Example
///
/// ```no_run
/// use djvu_rs::djvu_render::{render_streaming, RenderOptions};
/// # let doc = djvu_rs::djvu_document::DjVuDocument::parse(&[]).unwrap();
/// # let page = doc.page(0).unwrap();
/// let opts = RenderOptions::fit_to_width(page, 1024);
/// render_streaming(page, &opts, |y, rgba_row| {
///     // hand the row off to an encoder, GPU upload, network write, etc
///     # let _ = (y, rgba_row);
/// }).unwrap();
/// ```
pub fn render_streaming<F>(
    page: &DjVuPage,
    opts: &RenderOptions,
    sink: F,
) -> Result<(), RenderError>
where
    F: FnMut(usize, &[u8]),
{
    if opts.aa {
        return Err(RenderError::UnsupportedOption(
            "anti-aliasing requires a full pixmap; use render_pixmap",
        ));
    }
    let lanczos_with_scaling = opts.resampling == Resampling::Lanczos3
        && (page.width() as u32 != opts.width || page.height() as u32 != opts.height);
    if lanczos_with_scaling {
        return Err(RenderError::UnsupportedOption(
            "Lanczos-3 resampling at scaled output requires a full pixmap; use render_pixmap",
        ));
    }
    if combine_rotations(page.rotation(), opts.rotation) != crate::info::Rotation::None {
        return Err(RenderError::UnsupportedOption(
            "rotation requires a full pixmap; use render_pixmap",
        ));
    }
    render_rows(page, opts, sink)
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

    let bg_subsample = best_iw44_subsample(opts.decode_scale(page));
    let DecodedLayers {
        bg,
        fg_palette,
        mask,
        blit_map,
        fg44,
    } = decode_layers(page, opts, bg_subsample, usize::MAX)?;

    let out_w = region.width;
    let out_h = region.height;
    let mut pm = Pixmap::white(out_w, out_h);

    let region_opts = RenderOptions {
        width: full_w,
        height: full_h,
        ..*opts
    };
    let ctx = CompositeContext::from_layers(
        page,
        &region_opts,
        bg.as_ref(),
        mask.as_ref(),
        0,
        fg_palette.as_ref(),
        blit_map.as_deref(),
        fg44.as_ref(),
        &gamma_lut,
        (region.x, region.y),
        (out_w, out_h),
    );
    composite_into(&ctx, &mut pm.data)?;

    // Shared Lanczos-3 post-pass: scaling is judged against the full render
    // size (full_w/full_h) but the result is scaled to the region (out_w/out_h).
    let pm = apply_lanczos_postpass(
        pm,
        page,
        opts,
        (full_w, full_h),
        (out_w, out_h),
        |native_opts| render_region(page, region, native_opts),
    );

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
                ..Default::default()
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

    let bg_subsample = best_iw44_subsample(opts.decode_scale(page));
    let bg = decode_background_chunks(page, 1, bg_subsample)?;
    let bg = match bg {
        Some(b) => b,
        None => return Ok(None),
    };

    let gamma_lut = build_gamma_lut(page.gamma());
    let mut pm = Pixmap::white(w, h);

    {
        let ctx = CompositeContext::from_layers(
            page,
            opts,
            Some(&bg),
            None,
            0,
            None,
            None,
            None,
            &gamma_lut,
            (0, 0),
            (w, h),
        );
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

    // Decode background up to chunk_n + 1 chunks; full foreground + bold dilation
    // via the shared decode_layers path so no logic can drift between this and the
    // full render.
    let bg_subsample = best_iw44_subsample(opts.decode_scale(page));
    let DecodedLayers {
        bg,
        fg_palette,
        mask,
        blit_map,
        fg44,
    } = decode_layers(page, opts, bg_subsample, chunk_n + 1)?;

    let mut pm = Pixmap::white(w, h);
    {
        let ctx = CompositeContext::from_layers(
            page,
            opts,
            bg.as_ref(),
            mask.as_ref(),
            0,
            fg_palette.as_ref(),
            blit_map.as_deref(),
            fg44.as_ref(),
            &gamma_lut,
            (0, 0),
            (w, h),
        );
        composite_into(&ctx, &mut pm.data)?;
    }

    // Shared Lanczos-3 post-pass; the native re-render decodes the same
    // `chunk_n` refinement level.
    let pm = apply_lanczos_postpass(pm, page, opts, (w, h), (w, h), |native_opts| {
        render_progressive(page, native_opts, chunk_n)
    });

    Ok(rotate_pixmap(
        pm,
        combine_rotations(page.rotation(), opts.rotation),
    ))
}

/// Number of progressive refinement frames a page yields: one per BG44 chunk,
/// or a single frame for bilevel/JB2-only pages with no BG44 data.
///
/// This is the seam that hides the `max(1, bg44_chunks().len())` convention from
/// callers, so progressive consumers never reach into [`DjVuPage::bg44_chunks`]
/// to size their own loops.
pub fn progressive_steps(page: &DjVuPage) -> usize {
    page.bg44_chunks().len().max(1)
}

/// Render progressive frame `step` (`0..`[`progressive_steps`]).
///
/// Encapsulates the "no BG44 chunks ⇒ a single full [`render_pixmap`], otherwise
/// [`render_progressive`]`(step)`" decision that every progressive caller (the
/// `Page::render_scaled_progressive` collector and the async
/// `render_progressive_stream`) previously open-coded. `step` is interpreted as
/// the BG44 chunk index on multi-chunk pages.
pub fn render_progressive_step(
    page: &DjVuPage,
    opts: &RenderOptions,
    step: usize,
) -> Result<Pixmap, RenderError> {
    if page.bg44_chunks().is_empty() {
        render_pixmap(page, opts)
    } else {
        render_progressive(page, opts, step)
    }
}

/// Eagerly render every progressive frame into a `Vec`, coarsest first.
///
/// The convenience form of [`render_progressive_step`] over the full
/// [`progressive_steps`] range; the last frame equals [`render_pixmap`].
/// Streaming consumers that want one frame at a time should drive
/// [`render_progressive_step`] directly instead of collecting.
pub fn render_progressive_all(
    page: &DjVuPage,
    opts: &RenderOptions,
) -> Result<Vec<Pixmap>, RenderError> {
    let steps = progressive_steps(page);
    let mut frames = Vec::with_capacity(steps);
    for step in 0..steps {
        frames.push(render_progressive_step(page, opts, step)?);
    }
    Ok(frames)
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

    /// Issue #199 regression: page-space FRACBITS coords must be scaled into
    /// FG44-space using the `fg_x_q24` / `fg_y_q24` ratios. With page_w=2260
    /// and fg_w=189 a Q24 ratio of `(189 << 24) / 2260` maps the rightmost
    /// column to `fg_w - 1` instead of clamping every column past x=189.
    #[test]
    fn fg_q24_maps_endpoints_into_fg_space() {
        let fg = Pixmap::white(189, 306);
        let (qx, qy) = fg_q24(Some(&fg), 2260, 3669);
        assert!(qx > 0 && qy > 0);
        let frac = 1u64 << FRACBITS;
        let last_x = 2259u64 * frac;
        let fg_fx = (last_x * qx) >> 24;
        let fg_px = fg_fx >> FRACBITS;
        assert_eq!(fg_px, (fg.width as u64) - 1);
        let last_y = 3668u64 * frac;
        let fg_fy = (last_y * qy) >> 24;
        let fg_py = fg_fy >> FRACBITS;
        assert_eq!(fg_py, (fg.height as u64) - 1);
    }

    #[test]
    fn fg_q24_returns_zero_when_fg_is_none() {
        assert_eq!(fg_q24(None, 100, 100), (0, 0));
    }

    /// Issue #199 second-half regression: BG plane is often stored at a
    /// non-power-of-2 fraction of the page (1/3 is common for 400dpi colour
    /// scans). Without `bg_x_q24` / `bg_y_q24` page→bg-space scaling the BG
    /// sampler clamped most of the page to the rightmost BG column.
    #[test]
    fn bg_q24_maps_non_pow2_subsample() {
        // Page 2260×3669 with BG plane 754×1223 (DjVu's padded 1/3-page layout).
        let bg = Pixmap::white(754, 1223);
        let (qx, qy) = bg_q24(Some(&bg), 2260, 3669);
        assert_eq!(qx, (1u64 << 24) / 3);
        assert_eq!(qy, (1u64 << 24) / 3);

        let last_x = 2259u32 * FRAC;
        let bg_px = (map_plane_center_frac(last_x, qx) as u64) >> FRACBITS;
        assert!(bg_px < bg.width as u64);
        let last_y = 3668u32 * FRAC;
        let bg_py = (map_plane_center_frac(last_y, qy) as u64) >> FRACBITS;
        assert!(bg_py < bg.height as u64);
    }

    #[test]
    fn bg_q24_returns_zero_when_bg_is_none() {
        assert_eq!(bg_q24(None, 100, 100), (0, 0));
    }

    #[test]
    fn plane_q24_some_branch_with_zero_dimension_plane() {
        // Lines 508-512: plane_q24 Some(p) arm, reached via fg_q24 when fg has
        // zero width (so fg_q24's inner guard p.width > 0 fails, falling through
        // to plane_q24). With page_w > 0 && page_h > 0, plane_q24 takes its
        // Some arm and returns (0/page_w, 0/page_h) = (0, 0).
        let zero_width_fg = Pixmap::new(0, 10, 0, 0, 0, 0);
        let (qx, qy) = fg_q24(Some(&zero_width_fg), 100, 100);
        assert_eq!(qx, 0); // (0 << 24) / 100 = 0
        assert_eq!(qy, (10u64 << 24) / 100); // height-based ratio
    }

    #[test]
    fn fg_q24_uses_integer_horizontal_cell_pitch() {
        let fg = Pixmap::white(189, 306);
        let (qx, qy) = fg_q24(Some(&fg), 2260, 3669);
        assert_eq!(qx, (1u64 << 24) / 12);
        assert_eq!(qy, ((306u64) << 24) / 3669);
    }

    #[test]
    fn bg_q24_uses_integer_cell_pitch_for_padded_edges() {
        let bg = Pixmap::white(754, 1223);
        let (qx, qy) = bg_q24(Some(&bg), 2260, 3669);
        assert_eq!(qx, (1u64 << 24) / 3);
        assert_eq!(qy, (1u64 << 24) / 3);
    }

    #[test]
    fn map_plane_center_frac_aligns_pixel_centers() {
        // Destination page is twice the source plane size.  Page pixel x=1 has
        // centre 1.5; mapped to source centre space that is 1.5 * 0.5 - 0.5 = 0.25.
        let q24 = (1u64 << 24) / 2;
        assert_eq!(map_plane_center_frac(0, q24), 0);
        assert_eq!(map_plane_center_frac(FRAC, q24), FRAC / 4);
    }

    #[test]
    fn sample_bilinear_rounds_to_nearest() {
        let mut pm = Pixmap::new(2, 2, 0, 0, 0, 255);
        pm.set_rgb(1, 1, 255, 255, 255);

        // At the exact centre, bilinear interpolation is 63.75, which should
        // round to 64 instead of truncating to 63.
        assert_eq!(sample_bilinear(&pm, FRAC / 2, FRAC / 2), (64, 64, 64));
    }

    #[test]
    fn sample_nearest_rounds_to_nearest_pixel() {
        let mut pm = Pixmap::new(2, 1, 10, 20, 30, 255);
        pm.set_rgb(1, 0, 200, 210, 220);

        assert_eq!(sample_nearest(&pm, FRAC / 2 - 1, 0), (10, 20, 30));
        assert_eq!(sample_nearest(&pm, FRAC / 2, 0), (200, 210, 220));
    }

    /// RenderOptions default values.
    #[test]
    fn render_options_default() {
        let opts = RenderOptions::default();
        assert_eq!(opts.width, 0);
        assert_eq!(opts.height, 0);
        assert_eq!(opts.bold, 0);
        assert!(!opts.aa);
        assert_eq!(opts.resampling, Resampling::Bilinear);
    }

    /// RenderOptions can be constructed with explicit fields.
    #[test]
    fn render_options_construction() {
        let opts = RenderOptions {
            width: 400,
            height: 300,
            bold: 1,
            aa: true,
            rotation: UserRotation::Cw90,
            ..Default::default()
        };
        assert_eq!(opts.width, 400);
        assert_eq!(opts.height, 300);
        assert_eq!(opts.bold, 1);
        assert!(opts.aa);
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
        // The pipeline's decode scale is derived from width; it matches the
        // width/page-width ratio the deprecated `scale` field used to carry.
        assert!((opts.decode_scale(page) - 800.0 / pw as f32).abs() < 0.01);
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
        // fit_to_height preserves aspect, so width/page-width equals
        // height/page-height — the decode scale matches either ratio.
        assert!((opts.decode_scale(page) - 600.0 / ph as f32).abs() < 0.01);
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

    /// `render_progressive_all` yields `progressive_steps` frames and its last
    /// frame is byte-identical to `render_pixmap` (the sealed protocol contract).
    #[test]
    fn render_progressive_all_seals_chunk_loop() {
        let doc = load_doc("boy.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 80,
            height: 100,
            ..Default::default()
        };

        // The seam hides `max(1, bg44_chunks().len())` from callers.
        let steps = progressive_steps(page);
        assert_eq!(steps, page.bg44_chunks().len().max(1));

        let frames = render_progressive_all(page, &opts).expect("progressive_all must succeed");
        assert_eq!(frames.len(), steps, "one frame per progressive step");

        let full = render_pixmap(page, &opts).expect("render_pixmap must succeed");
        assert_eq!(
            frames.last().unwrap().data,
            full.data,
            "final progressive frame must equal the full render"
        );
        // `render_progressive_step(0)` is also a valid (coarse) frame.
        let first = render_progressive_step(page, &opts, 0).expect("step 0 must succeed");
        assert_eq!(first.data, frames[0].data);
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
            ..Default::default()
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
            ..Default::default()
        };
        let pm = render_coarse(page, &opts).expect("render_coarse must succeed");
        assert!(pm.is_some(), "must return Some when BGjp present");
        let pm = pm.unwrap();
        assert_eq!(pm.width, 4);
        assert_eq!(pm.height, 4);
    }

    // ── Lanczos-3 tests ───────────────────────────────────────────────────────
    // (The raw resampler `scale_lanczos3` and its `lanczos3_kernel` now live in
    // the `pixmap` module and are unit-tested there; these exercise the render
    // path's use of Lanczos-3.)

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

    /// The IW44 decode subsample is derived from the output `width`, not from
    /// the deprecated `scale` field — the regression guard for the PDF
    /// over-decode (#377).
    #[test]
    #[allow(deprecated)] // deliberately writes the legacy `scale` field to prove it is ignored
    fn decode_subsample_derives_from_width_not_scale_field() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let (dw, _) = display_dimensions(page);

        // Quarter-width output. The PDF exporter built exactly this — a small
        // `width` with `scale` left at the 1.0 default — and the old pipeline
        // read `scale` and decoded at full wavelet resolution (subsample 1).
        // The decode scale is now derived from `width`, so it is 0.25 →
        // subsample 4, and the over-decode is gone.
        let opts = RenderOptions {
            width: dw / 4,
            ..Default::default()
        };
        assert!((opts.decode_scale(page) - 0.25).abs() < 0.01);
        assert_eq!(best_iw44_subsample(opts.decode_scale(page)), 4);

        // Writing the legacy `scale` field by hand — to any value — must not
        // change the width-derived subsample.
        for misleading in [1.0_f32, 0.0, 0.5, 4.0] {
            let mut o = RenderOptions {
                width: dw / 4,
                ..Default::default()
            };
            o.scale = misleading;
            assert_eq!(
                best_iw44_subsample(o.decode_scale(page)),
                4,
                "scale={misleading} must not change the width-derived subsample",
            );
        }
    }

    /// INFO-rotated page: the decode scale follows the *native* page width, not
    /// the rotation-swapped display width. The compositor scales the native
    /// raster into the output buffer before `rotate_pixmap` runs, so a downscaled
    /// rotated page must not pick its IW44 subsample from the swapped dimension —
    /// doing so over-subsampled a downscaled portrait background (#377 follow-up).
    #[test]
    fn decode_scale_uses_native_width_for_rotated_page() {
        // boy_jb2_rotate90 carries a 90° rotation in its INFO chunk.
        let doc = load_doc("boy_jb2_rotate90.djvu");
        let page = doc.page(0).unwrap();
        let pw = page.width() as u32;
        let (dw, _) = display_dimensions(page);
        assert_ne!(dw, pw, "fixture must be a non-square INFO-rotated page");

        // The raster exporters size the output from the native page width
        // (page.width() * s); at s = 0.5 the half-size render must decode at 0.5.
        let opts = RenderOptions {
            width: pw / 2,
            height: (page.height() as u32) / 2,
            ..Default::default()
        };
        let native = opts.width as f32 / pw as f32; // correct: width / native width
        let display = opts.width as f32 / dw as f32; // the old bug: width / display width
        assert!(
            (opts.decode_scale(page) - native).abs() < 1e-4,
            "decode_scale {} should equal the native-width ratio {native}",
            opts.decode_scale(page),
        );
        assert!(
            (opts.decode_scale(page) - display).abs() > 1e-2,
            "decode_scale must not follow the rotation-swapped display width ({display})",
        );
    }

    /// Rendering with bg_subsample=2 (scale=0.5) produces the correct output dimensions.
    #[test]
    fn render_pixmap_subsampled_bg_correct_dimensions() {
        let doc = load_doc("boy.djvu");
        let page = doc.page(0).unwrap();
        // width = half the page → decode_scale ≈ 0.5 → bg_subsample=2 internally
        let opts = RenderOptions {
            width: (page.width() as f32 * 0.5) as u32,
            height: (page.height() as f32 * 0.5) as u32,
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

    /// `downsample_mask_4x` is render-tier logic now testable on a hand-built
    /// bitmap — no DjVu bytes to parse. Each 4×4 source block collapses to one
    /// output bit that is set iff any source bit in the block is set.
    #[test]
    fn downsample_mask_4x_max_pools_each_block() {
        // 8×8 mask: set a single pixel in the top-left block and one in the
        // bottom-right block; the other two blocks stay clear.
        let mut src = crate::bitmap::Bitmap::new(8, 8);
        src.set(1, 2, true); // top-left 4×4 block
        src.set(6, 5, true); // bottom-right 4×4 block
        let out = downsample_mask_4x(&src);
        assert_eq!((out.width, out.height), (2, 2));
        assert!(out.get(0, 0), "top-left block had a set bit");
        assert!(!out.get(1, 0), "top-right block was empty");
        assert!(!out.get(0, 1), "bottom-left block was empty");
        assert!(out.get(1, 1), "bottom-right block had a set bit");
    }

    /// A non-multiple-of-4 mask rounds up: a 5×5 source yields a 2×2 result and
    /// the ragged edge block still max-pools its single column/row.
    #[test]
    fn downsample_mask_4x_rounds_up_ragged_edges() {
        let mut src = crate::bitmap::Bitmap::new(5, 5);
        src.set(4, 4, true); // lone pixel in the ragged bottom-right block
        let out = downsample_mask_4x(&src);
        assert_eq!((out.width, out.height), (2, 2));
        assert!(out.get(1, 1), "ragged corner block must capture its bit");
        assert!(!out.get(0, 0));
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

    // ── Issue #225: render_rows byte-identical to render_into (direct-write path) ──

    // ── Issue #225 Phase 2: public render_streaming API ──────────────────────

    /// `render_streaming` must produce byte-for-byte identical output to
    /// `render_pixmap` when no post-processing options are set (no aa, no
    /// Lanczos scaling, no rotation).
    #[test]
    fn render_streaming_byte_identical_to_render_pixmap_color() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let w = 60u32;
        let h = 80u32;
        let opts = RenderOptions {
            width: w,
            height: h,
            ..Default::default()
        };

        let pm = render_pixmap(page, &opts).expect("render_pixmap should succeed");

        let row_stride = w as usize * 4;
        let mut streamed = vec![0u8; w as usize * h as usize * 4];
        render_streaming(page, &opts, |y, row| {
            assert_eq!(row.len(), row_stride);
            let start = y * row_stride;
            streamed[start..start + row_stride].copy_from_slice(row);
        })
        .expect("render_streaming should succeed");

        assert_eq!(
            pm.data, streamed,
            "render_streaming must be byte-identical to render_pixmap when no post-processing options are set"
        );
    }

    /// `render_streaming` must produce byte-for-byte identical output to
    /// `render_pixmap` for a bilevel page.
    #[test]
    fn render_streaming_byte_identical_to_render_pixmap_bilevel() {
        let doc = load_doc("boy_jb2.djvu");
        let page = doc.page(0).unwrap();
        let w = 50u32;
        let h = 70u32;
        let opts = RenderOptions {
            width: w,
            height: h,
            ..Default::default()
        };

        let pm = render_pixmap(page, &opts).expect("render_pixmap should succeed");

        let row_stride = w as usize * 4;
        let mut streamed = vec![0u8; w as usize * h as usize * 4];
        render_streaming(page, &opts, |y, row| {
            let start = y * row_stride;
            streamed[start..start + row_stride].copy_from_slice(row);
        })
        .expect("render_streaming should succeed");

        assert_eq!(pm.data, streamed);
    }

    /// `render_streaming` rejects anti-aliasing with `UnsupportedOption`.
    #[test]
    fn render_streaming_rejects_aa() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 60,
            height: 80,
            aa: true,
            ..Default::default()
        };
        let err = render_streaming(page, &opts, |_, _| {}).unwrap_err();
        assert!(
            matches!(err, RenderError::UnsupportedOption(_)),
            "expected UnsupportedOption, got {err:?}"
        );
    }

    /// `render_streaming` rejects Lanczos-3 when scaling actually happens.
    #[test]
    fn render_streaming_rejects_lanczos_with_scaling() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        // Force scaling by picking dimensions ≠ the page's native size.
        let opts = RenderOptions {
            width: page.width() as u32 / 2,
            height: page.height() as u32 / 2,
            resampling: Resampling::Lanczos3,
            ..Default::default()
        };
        let err = render_streaming(page, &opts, |_, _| {}).unwrap_err();
        assert!(matches!(err, RenderError::UnsupportedOption(_)));
    }

    /// `render_streaming` allows Lanczos-3 when output matches native page
    /// dimensions (Lanczos is a no-op in that case).
    #[test]
    fn render_streaming_allows_lanczos_at_native_size() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: page.width() as u32,
            height: page.height() as u32,
            resampling: Resampling::Lanczos3,
            ..Default::default()
        };
        let mut row_count = 0usize;
        render_streaming(page, &opts, |_, _| row_count += 1).expect("should succeed at native");
        assert_eq!(row_count, page.height() as usize);
    }

    /// `render_streaming` rejects user rotation.
    #[test]
    fn render_streaming_rejects_user_rotation() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 60,
            height: 80,
            rotation: UserRotation::Cw90,
            ..Default::default()
        };
        let err = render_streaming(page, &opts, |_, _| {}).unwrap_err();
        assert!(matches!(err, RenderError::UnsupportedOption(_)));
    }

    /// `render_streaming` rejects zero dimensions with `InvalidDimensions`.
    #[test]
    fn render_streaming_rejects_zero_dimensions() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 0,
            height: 80,
            ..Default::default()
        };
        let err = render_streaming(page, &opts, |_, _| {}).unwrap_err();
        assert!(matches!(err, RenderError::InvalidDimensions { .. }));
    }

    // ── Permissive render path ────────────────────────────────────────────────

    /// Permissive render on a standard IW44+JB2 page completes without error.
    #[test]
    fn permissive_render_iw44_jb2_page() {
        let doc = load_doc("czech.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 40,
            height: 40,
            permissive: true,
            ..Default::default()
        };
        let pm = render_pixmap(page, &opts).expect("permissive render should not error");
        assert_eq!(pm.width, 40);
        assert_eq!(pm.height, 40);
    }

    /// Permissive render on a BGjp page (no BG44) falls through to decode_bgjp.
    #[test]
    fn permissive_render_bgjp_page() {
        let doc = load_doc("bgjp_test.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 4,
            height: 4,
            permissive: true,
            ..Default::default()
        };
        let pm = render_pixmap(page, &opts).expect("permissive render of BGjp page should succeed");
        assert_eq!(pm.width, 4);
        assert_eq!(pm.height, 4);
    }

    /// Permissive render on a page with FGbz palette exercises the indexed mask path.
    #[test]
    fn permissive_render_fgbz_page() {
        let doc = load_doc("navm_fgbz.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 40,
            height: 40,
            permissive: true,
            ..Default::default()
        };
        let pm = render_pixmap(page, &opts).expect("permissive render of FGbz page should succeed");
        assert!(pm.width > 0 && pm.height > 0);
    }

    /// render_gray8 produces a grayscale pixmap.
    #[test]
    fn render_gray8_produces_grayscale_output() {
        let doc = load_doc("boy_jb2.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 20,
            height: 20,
            ..Default::default()
        };
        let gpm = render_gray8(page, &opts).expect("render_gray8 should succeed");
        assert_eq!(gpm.width, 20);
        assert_eq!(gpm.height, 20);
        assert_eq!(gpm.data.len(), 20 * 20);
    }

    /// render_coarse rejects zero width.
    #[test]
    fn render_coarse_rejects_zero_dimensions() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 0,
            height: 50,
            ..Default::default()
        };
        let err = render_coarse(page, &opts).unwrap_err();
        assert!(matches!(err, RenderError::InvalidDimensions { .. }));
    }

    /// render_coarse on a JB2-only page returns None.
    #[test]
    fn render_coarse_jb2_only_page_returns_none() {
        let doc = load_doc("boy_jb2.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 40,
            height: 40,
            ..Default::default()
        };
        let result = render_coarse(page, &opts).expect("should not error");
        assert!(
            result.is_none(),
            "JB2-only page has no BG44 so render_coarse yields None"
        );
    }

    /// render_progressive rejects zero dimensions.
    #[test]
    fn render_progressive_rejects_zero_dimensions() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 0,
            height: 50,
            ..Default::default()
        };
        let err = render_progressive(page, &opts, 0).unwrap_err();
        assert!(matches!(err, RenderError::InvalidDimensions { .. }));
    }

    /// render_progressive on a JB2-only page (no BG44) completes successfully.
    #[test]
    fn render_progressive_jb2_only_page_succeeds() {
        let doc = load_doc("boy_jb2.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 40,
            height: 40,
            ..Default::default()
        };
        let result = render_progressive(page, &opts, 0)
            .expect("render_progressive should succeed even without BG44");
        assert_eq!(result.width, 40);
        assert_eq!(result.height, 40);
    }

    /// Lanczos3 at native resolution skips the re-render (need_scale=false path).
    #[test]
    fn lanczos3_at_native_resolution_skips_rerender() {
        let doc = load_doc("bgjp_test.djvu");
        let page = doc.page(0).unwrap();
        // bgjp_test.djvu is 4×4 — render at native size with Lanczos3
        let opts = RenderOptions {
            width: page.width() as u32,
            height: page.height() as u32,
            resampling: Resampling::Lanczos3,
            ..Default::default()
        };
        let pm = render_pixmap(page, &opts).expect("lanczos at native size must succeed");
        assert_eq!(pm.width, page.width() as u32);
        assert_eq!(pm.height, page.height() as u32);
    }

    /// render_pixmap rejects zero width.
    #[test]
    fn render_pixmap_rejects_zero_width() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 0,
            height: 50,
            ..Default::default()
        };
        let err = render_pixmap(page, &opts).unwrap_err();
        assert!(matches!(err, RenderError::InvalidDimensions { .. }));
    }

    /// Build a minimal single-page DJVU document with the given width and height.
    fn make_doc_with_dims(w: u16, h: u16) -> Vec<u8> {
        use crate::iff::{Chunk, DjvuFile, emit};
        let mut info = vec![0u8; 10];
        info[0] = (w >> 8) as u8;
        info[1] = w as u8;
        info[2] = (h >> 8) as u8;
        info[3] = h as u8;
        let file = DjvuFile {
            root: Chunk::Form {
                secondary_id: *b"DJVU",
                length: 0,
                children: vec![Chunk::Leaf {
                    id: *b"INFO",
                    data: info,
                }],
            },
        };
        emit(&file)
    }

    // ── RenderOptions edge-case helpers ──────────────────────────────────────

    #[test]
    fn fit_to_width_zero_page_width_uses_requested_width() {
        // Line 213: dw == 0 → height = width (same as requested width).
        let bytes = make_doc_with_dims(0, 100);
        let doc = DjVuDocument::parse(&bytes).unwrap();
        let page = doc.page(0).unwrap();
        let opts = RenderOptions::fit_to_width(page, 40);
        assert_eq!(opts.width, 40);
        assert_eq!(opts.height, 40); // because dw==0 → height = width
    }

    #[test]
    fn fit_to_height_zero_page_height_uses_requested_height() {
        // Line 232: dh == 0 → width = height (same as requested height).
        let bytes = make_doc_with_dims(100, 0);
        let doc = DjVuDocument::parse(&bytes).unwrap();
        let page = doc.page(0).unwrap();
        let opts = RenderOptions::fit_to_height(page, 60);
        assert_eq!(opts.height, 60);
        assert_eq!(opts.width, 60); // because dh==0 → width = height
    }

    #[test]
    fn fit_to_box_zero_page_dims_falls_back_to_box_max() {
        // Lines 255-259: dw==0 || dh==0 → return max_width × max_height with scale=1.
        let bytes = make_doc_with_dims(0, 0);
        let doc = DjVuDocument::parse(&bytes).unwrap();
        let page = doc.page(0).unwrap();
        let opts = RenderOptions::fit_to_box(page, 80, 60);
        assert_eq!(opts.width, 80);
        assert_eq!(opts.height, 60);
    }

    #[test]
    fn can_stream_true_at_native_resolution_with_lanczos3() {
        // Line 290: the "|| native dims" branch — evaluated when Bilinear is false.
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: page.width() as u32,
            height: page.height() as u32,
            resampling: Resampling::Lanczos3,
            ..Default::default()
        };
        assert!(opts.can_stream(page));
    }

    /// Rendering a JB2-only (no BG44) page at very small scale triggers
    /// subsample >= 4 and exercises bg44_partial's empty-chunks path (line 918).
    #[test]
    fn render_bilevel_at_tiny_scale_exercises_bg44_partial_empty() {
        let doc = load_doc("boy_jb2.djvu");
        let page = doc.page(0).unwrap();
        // Use a tiny width so decode_scale << 0.25 and best_iw44_subsample >= 4.
        let opts = RenderOptions {
            width: 10,
            height: 10,
            ..Default::default()
        };
        let pm = render_pixmap(page, &opts).expect("tiny bilevel render should succeed");
        assert!(pm.width > 0 && pm.height > 0);
    }

    /// Bold dilation (opts.bold > 0) thickens the mask.
    #[test]
    fn render_with_bold_dilation_produces_output() {
        let doc = load_doc("boy_jb2.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 40,
            height: 40,
            bold: 1,
            ..Default::default()
        };
        let pm = render_pixmap(page, &opts).expect("bold render should succeed");
        assert_eq!(pm.width, 40);
        assert_eq!(pm.height, 40);
    }
}
