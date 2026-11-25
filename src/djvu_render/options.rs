//! Render options, errors and the recovery report.

use super::*;

// ── Errors ───────────────────────────────────────────────────────────────────

/// Errors that can occur during DjVuPage rendering.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RenderError {
    /// IW44 wavelet decode error.
    #[error("IW44 decode error: {0}")]
    Iw44(#[from] crate::error::Iw44Error),

    /// JB2 bilevel decode error.
    #[error("JB2 decode error: {0}")]
    Jb2(#[from] crate::error::Jb2Error),

    /// The output buffer provided to [`RenderRequest::write_rgba`] is too small.
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
    Jpeg(#[source] JpegError),

    /// Document-level error (e.g. page index out of range).
    #[error("document error: {0}")]
    Doc(#[from] crate::djvu_document::DocError),

    /// A configured resource limit was exceeded during rendering.
    #[error("{0}")]
    ResourceLimit(#[from] crate::resource_limits::ResourceLimitExceeded),

    /// A render option is incompatible with the chosen entry point.
    ///
    /// Returned by [`RenderRequest::rows`] and [`RenderRequest::write_rgba`]
    /// when an option requires post-processing
    /// of a fully-allocated pixmap (anti-aliasing, Lanczos resampling at a
    /// scaled output, or rotation).
    #[error("unsupported render option: {0}")]
    UnsupportedOption(&'static str),

    /// A [`CancelToken`] stopped the render at a checkpoint.
    #[error("render cancelled")]
    Cancelled,

    /// A coarse render ([`Quality::Coarse`]) of a page without a background
    /// layer.
    #[error("page has no background layer for a coarse render")]
    NoBackground,
}

/// A BGjp/FGjp chunk could not be decoded as JPEG.
///
/// Carried by [`RenderError::Jpeg`]. The type is opaque so that the JPEG
/// decoder the crate uses stays out of the public API: its
/// [`Display`](core::fmt::Display) gives the decoder's message, and
/// [`source`](std::error::Error::source) returns the decoder's own error when
/// there is one.
#[cfg(feature = "std")]
#[derive(Debug)]
pub struct JpegError {
    message: String,
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

#[cfg(feature = "std")]
impl JpegError {
    /// A failure reported by the JPEG decoder.
    pub(crate) fn decoder<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            message: error.to_string(),
            source: Some(Box::new(error)),
        }
    }

    /// A failure detected by this crate, with no underlying error.
    pub(crate) fn msg(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }
}

#[cfg(feature = "std")]
impl core::fmt::Display for JpegError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.message)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for JpegError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

/// A refused output pixmap is a render-output limit.
///
/// [`Pixmap::try_new`] caps one pixmap at [`Pixmap::MAX_PIXELS`]; on the render
/// paths that ceiling belongs to the same axis as the configurable
/// `max_render_pixels`, so it surfaces as [`RenderError::ResourceLimit`]. A
/// `usize` overflow of `width * height` is an invalid size, not a limit.
impl From<crate::pixmap::PixmapError> for RenderError {
    fn from(e: crate::pixmap::PixmapError) -> Self {
        match e {
            crate::pixmap::PixmapError::Overflow { width, height } => {
                RenderError::InvalidDimensions { width, height }
            }
            crate::pixmap::PixmapError::TooLarge {
                width,
                height,
                pixels,
                max,
            } => RenderError::ResourceLimit(crate::resource_limits::ResourceLimitExceeded {
                operation: "render",
                axis: crate::resource_limits::ResourceLimitAxis::RenderOutputPixels,
                found: pixels as u64,
                limit: max as u64,
                page_number: None,
                width: Some(width),
                height: Some(height),
            }),
            // `PixmapError` is `#[non_exhaustive]` in a sibling crate; a
            // variant this version does not know is still a refused size.
            #[allow(unreachable_patterns)]
            _ => RenderError::UnsupportedOption("output pixmap size refused"),
        }
    }
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
#[non_exhaustive]
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

/// Rendering parameters passed to [`RenderRequest::new`] and [`render_pixmap`].
///
/// # Example
///
/// ```
/// use djvu_rs::djvu_render::RenderOptions;
///
/// // Set the output size; the pipeline derives the decode scale from `width`.
/// let opts = RenderOptions::new(800, 600).with_aa(true);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RenderOptions {
    /// Compositor width in pixels, in the page's native (pre-rotation)
    /// orientation. The returned pixmap is this size after the page's INFO
    /// rotation and [`rotation`](Self::rotation): a quarter turn swaps the
    /// sides. The `fit_to_*` constructors take display sizes and set this.
    pub width: u32,
    /// Compositor height in pixels, in the page's native orientation (see
    /// [`width`](Self::width)).
    pub height: u32,
    /// Bold level: number of dilation passes on the JB2 mask (0 = no dilation).
    pub bold: u8,
    /// Whether to apply anti-aliasing downscale pass: the page is composited
    /// at `width`×`height` and each 2×2 block is averaged, so the output is
    /// half the requested size.
    ///
    /// Ignored when [`Resampling::Lanczos3`] rescales the page (any size other
    /// than the native one): the Lanczos-3 filter already smooths the page,
    /// and the output keeps the requested size.
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
    /// Anti-alias the JB2 bilevel mask's edges when rendering at **upscale**
    /// (zoom > 1): instead of the hard nearest-bit lookup, bilinearly
    /// interpolate the mask's 0/255 coverage and blend foreground/background
    /// colour proportionally — smoother glyph edges under zoom.
    ///
    /// A no-op at scale ≤ 1 (native or downscaled renders are unaffected).
    ///
    /// **Opt-in, default `false`.** DjVuLibre hard-edges the mask under zoom,
    /// so enabling this is a deliberate, judged divergence from the reference
    /// renderer's pixel output — a "prettier than DjVuLibre" quality mode, not
    /// a faithfulness fix. Leaving it `false` keeps `render_pixmap` and
    /// friends byte-identical to prior releases.
    pub mask_aa: bool,
}

impl RenderOptions {
    /// Options for an output of `width` × `height` pixels, with every other
    /// field at its default.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            ..Self::default()
        }
    }

    /// Sets [`width`](Self::width).
    #[must_use]
    pub fn with_width(mut self, width: u32) -> Self {
        self.width = width;
        self
    }

    /// Sets [`height`](Self::height).
    #[must_use]
    pub fn with_height(mut self, height: u32) -> Self {
        self.height = height;
        self
    }

    /// Sets [`bold`](Self::bold).
    #[must_use]
    pub fn with_bold(mut self, bold: u8) -> Self {
        self.bold = bold;
        self
    }

    /// Sets [`aa`](Self::aa).
    #[must_use]
    pub fn with_aa(mut self, aa: bool) -> Self {
        self.aa = aa;
        self
    }

    /// Sets [`rotation`](Self::rotation).
    #[must_use]
    pub fn with_rotation(mut self, rotation: UserRotation) -> Self {
        self.rotation = rotation;
        self
    }

    /// Sets [`permissive`](Self::permissive).
    #[must_use]
    pub fn with_permissive(mut self, permissive: bool) -> Self {
        self.permissive = permissive;
        self
    }

    /// Sets [`resampling`](Self::resampling).
    #[must_use]
    pub fn with_resampling(mut self, resampling: Resampling) -> Self {
        self.resampling = resampling;
        self
    }

    /// Sets [`mask_aa`](Self::mask_aa).
    #[must_use]
    pub fn with_mask_aa(mut self, mask_aa: bool) -> Self {
        self.mask_aa = mask_aa;
        self
    }
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            width: 0,
            height: 0,
            bold: 0,
            aa: false,
            rotation: UserRotation::None,
            permissive: false,
            resampling: Resampling::Bilinear,
            mask_aa: false,
        }
    }
}

pub(super) fn effective_max_render_pixels(
    page: &crate::djvu_document::DjVuPage,
    limits: Option<crate::resource_limits::ResourceLimits>,
) -> u64 {
    limits
        .and_then(|limits| limits.max_render_pixels)
        .or(page
            .resource_limits()
            .and_then(|limits| limits.max_render_pixels))
        .unwrap_or(crate::resource_limits::DEFAULT_MAX_RENDER_PIXELS)
}

pub(super) fn check_output_pixels(
    operation: &'static str,
    page: &crate::djvu_document::DjVuPage,
    limits: Option<crate::resource_limits::ResourceLimits>,
    width: u32,
    height: u32,
) -> Result<(), RenderError> {
    if width == 0 || height == 0 {
        return Err(RenderError::InvalidDimensions { width, height });
    }
    let pixels = u64::from(width) * u64::from(height);
    let limit = effective_max_render_pixels(page, limits);
    if pixels > limit {
        return Err(RenderError::ResourceLimit(
            crate::resource_limits::ResourceLimitExceeded {
                operation,
                axis: crate::resource_limits::ResourceLimitAxis::RenderOutputPixels,
                found: pixels,
                limit,
                page_number: Some(page.index() + 1),
                width: Some(width),
                height: Some(height),
            },
        ));
    }
    Ok(())
}

/// A document layer a permissive render skipped or fell back on (#696).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveredLayer {
    /// The IW44 background (`BG44`), possibly truncated at a corrupt chunk.
    Background,
    /// The IW44 foreground detail plane (`FG44`).
    Foreground,
    /// The JB2 stencil mask (`Sjbz`).
    Mask,
    /// The `FGbz` foreground colour palette.
    ForegroundPalette,
}

/// One recovery action a permissive render took to keep going (#696).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RenderRecovery {
    /// Which layer was affected.
    pub layer: RecoveredLayer,
    /// Human-readable explanation of what was skipped or substituted.
    pub detail: String,
}

/// Structured record of what a permissive render skipped or recovered (#696).
///
/// Returned alongside the pixmap by [`RenderRequest::pixmap_with_report`]. An empty
/// report (`is_clean`) means the page decoded fully with no fallbacks.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct RenderReport {
    /// Recovery actions in the order the renderer took them.
    pub recoveries: Vec<RenderRecovery>,
}

impl RenderReport {
    /// Whether the render needed no recovery (every layer decoded cleanly).
    pub fn is_clean(&self) -> bool {
        self.recoveries.is_empty()
    }
}

#[cfg(feature = "std")]
thread_local! {
    /// Active recovery sink. `Some` only while [`RenderRequest::pixmap_with_report`] is
    /// on the stack; otherwise `record_recovery` is a no-op, so the ordinary
    /// [`render_pixmap`] hot path is untouched.
    pub(super) static RECOVERY_SINK: core::cell::RefCell<Option<Vec<RenderRecovery>>> =
        const { core::cell::RefCell::new(None) };
}

/// Record a permissive recovery when a report is being collected.
#[cfg(feature = "std")]
pub(super) fn record_recovery(layer: RecoveredLayer, detail: impl Into<String>) {
    RECOVERY_SINK.with(|sink| {
        if let Some(list) = sink.borrow_mut().as_mut() {
            list.push(RenderRecovery {
                layer,
                detail: detail.into(),
            });
        }
    });
}

/// No-op in `no_std`: the report API requires `std` thread-locals.
#[cfg(not(feature = "std"))]
pub(super) fn record_recovery(_layer: RecoveredLayer, _detail: &str) {}

/// Unwrap a permissive layer decode, recording a recovery on failure.
pub(super) fn permissive_layer<T>(
    result: Result<Option<T>, RenderError>,
    layer: RecoveredLayer,
) -> Option<T> {
    match result {
        Ok(value) => value,
        Err(error) => {
            record_recovery(layer, {
                #[cfg(feature = "std")]
                {
                    error.to_string()
                }
                #[cfg(not(feature = "std"))]
                {
                    let _ = error;
                    ""
                }
            });
            None
        }
    }
}

impl RenderOptions {
    /// Create render options that scale the page to fit the given width,
    /// preserving aspect ratio. Respects page rotation from the INFO chunk:
    /// `width` is the width of the rendered (rotated) pixmap.
    pub fn fit_to_width(page: &crate::djvu_document::DjVuPage, width: u32) -> Self {
        RenderSize::fit_width(page, width).options()
    }

    /// Create render options that scale the page to fit the given height,
    /// preserving aspect ratio. Respects page rotation from the INFO chunk:
    /// `height` is the height of the rendered (rotated) pixmap.
    pub fn fit_to_height(page: &crate::djvu_document::DjVuPage, height: u32) -> Self {
        RenderSize::fit_height(page, height).options()
    }

    /// Create render options that scale the page to fit within a bounding box,
    /// preserving aspect ratio. Respects page rotation from the INFO chunk:
    /// the rendered (rotated) pixmap fits in `max_width × max_height`.
    pub fn fit_to_box(
        page: &crate::djvu_document::DjVuPage,
        max_width: u32,
        max_height: u32,
    ) -> Self {
        RenderSize::fit_box(page, max_width, max_height).options()
    }

    /// Whether `page` can be rendered with [`RenderRequest::rows`] under these
    /// options, producing pixels identical to [`render_pixmap`].
    ///
    /// The streaming path emits the page row-by-row without buffering a full
    /// [`Pixmap`], so callers that only need to forward rows (PDF/TIFF image
    /// encoders) can avoid the intermediate allocation. It is only equivalent
    /// to the buffered path when no whole-image post-pass is required: no
    /// anti-aliasing, no rotation, and either bilinear resampling or a 1:1
    /// (unscaled) render.
    ///
    /// The rotation is the combined INFO + user rotation: a user rotation that
    /// cancels the page's INFO rotation streams too.
    ///
    /// This is the single source of truth for streaming eligibility; export
    /// paths call it instead of re-deriving the rule, and [`RenderRequest::rows`]
    /// refuses exactly the options it rejects.
    pub fn can_stream(&self, page: &crate::djvu_document::DjVuPage) -> bool {
        self.whole_pixmap_reason(page).is_none()
    }

    /// Why a render of `page` needs a whole [`Pixmap`] before its output is
    /// final, or `None` when rows come out of the compositor final.
    ///
    /// The three whole-image post-passes are the anti-aliasing halving,
    /// Lanczos-3 rescaling, and the combined rotation. The reason doubles as
    /// the streaming error message.
    pub(crate) fn whole_pixmap_reason(
        &self,
        page: &crate::djvu_document::DjVuPage,
    ) -> Option<&'static str> {
        if self.aa {
            Some("anti-aliasing requires a full pixmap; use render_pixmap")
        } else if lanczos_rescales(page, self.resampling, (self.width, self.height)) {
            Some("Lanczos-3 resampling at scaled output requires a full pixmap; use render_pixmap")
        } else if self.output_rotation(page) != crate::info::Rotation::None {
            Some("rotation requires a full pixmap; use render_pixmap")
        } else {
            None
        }
    }

    /// The combined INFO + user rotation applied after compositing.
    pub(crate) fn output_rotation(
        &self,
        page: &crate::djvu_document::DjVuPage,
    ) -> crate::info::Rotation {
        combine_rotations(page.rotation(), self.rotation)
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
    /// Rendering reads *this*. Until 1.0 the options also had a public `scale`
    /// field that a caller could set inconsistently with the size.
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
