//! Configurable resource limits shared by validation, parse, and render entry points.

/// Documented default ceiling for render output pixel area (`width * height`).
pub const DEFAULT_MAX_RENDER_PIXELS: u64 = 512 * 1024 * 1024;

/// Which configured resource axis was exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLimitAxis {
    /// Input file size in bytes.
    FileBytes,
    /// Page count from INFO-bearing components.
    PageCount,
    /// Embedded component count in a bundled DJVM.
    ComponentCount,
    /// Single-page pixel area from an INFO chunk.
    PagePixels,
    /// Sum of every page's pixel area.
    TotalPixels,
    /// Estimated peak decoded-page memory in bytes.
    DecodedBytes,
    /// Render output pixel area (`width * height`).
    RenderOutputPixels,
}

/// A configured resource limit was exceeded by a public decode/render operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceLimitExceeded {
    /// Public entry point that rejected the input (for example `"document.parse"`
    /// or `"render_pixmap"`).
    pub operation: &'static str,
    /// Which limit axis was exceeded.
    pub axis: ResourceLimitAxis,
    /// Observed value that exceeded the limit.
    pub found: u64,
    /// Configured limit for the axis.
    pub limit: u64,
    /// 1-based page number when [`Self::axis`] is [`ResourceLimitAxis::PagePixels`].
    pub page_number: Option<usize>,
    /// Page or render width when relevant.
    pub width: Option<u32>,
    /// Page or render height when relevant.
    pub height: Option<u32>,
}

impl core::fmt::Display for ResourceLimitExceeded {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.axis {
            ResourceLimitAxis::PagePixels => write!(
                f,
                "{}: page {} is {}x{} = {} pixels, exceeding limit {}",
                self.operation,
                self.page_number.unwrap_or(0),
                self.width.unwrap_or(0),
                self.height.unwrap_or(0),
                self.found,
                self.limit
            ),
            ResourceLimitAxis::RenderOutputPixels => write!(
                f,
                "{}: render output {}x{} = {} pixels exceeds limit {}",
                self.operation,
                self.width.unwrap_or(0),
                self.height.unwrap_or(0),
                self.found,
                self.limit
            ),
            ResourceLimitAxis::FileBytes => write!(
                f,
                "{}: file is {} bytes, exceeding limit {}",
                self.operation, self.found, self.limit
            ),
            ResourceLimitAxis::PageCount => write!(
                f,
                "{}: document has {} pages, exceeding limit {}",
                self.operation, self.found, self.limit
            ),
            ResourceLimitAxis::ComponentCount => write!(
                f,
                "{}: document has {} components, exceeding limit {}",
                self.operation, self.found, self.limit
            ),
            ResourceLimitAxis::TotalPixels => write!(
                f,
                "{}: document totals {} pixels, exceeding limit {}",
                self.operation, self.found, self.limit
            ),
            ResourceLimitAxis::DecodedBytes => write!(
                f,
                "{}: peak decoded page memory is an estimated {} bytes, exceeding limit {}",
                self.operation, self.found, self.limit
            ),
        }
    }
}

impl core::error::Error for ResourceLimitExceeded {}

/// Configured processing limits checked by the resource layer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResourceLimits {
    /// Maximum accepted input size in bytes.
    pub max_file_bytes: Option<u64>,
    /// Maximum accepted page count.
    pub max_pages: Option<u64>,
    /// Maximum accepted embedded component count (bundled documents).
    pub max_components: Option<u64>,
    /// Maximum accepted pixel area (`width * height`) of any single page.
    pub max_page_pixels: Option<u64>,
    /// Maximum accepted sum of every page's pixel area.
    pub max_total_pixels: Option<u64>,
    /// Maximum accepted peak decoded-page memory, in bytes.
    pub max_decoded_bytes: Option<u64>,
    /// Maximum accepted render output pixel area (`width * height`).
    pub max_render_pixels: Option<u64>,
}

impl ResourceLimits {
    /// Whether every limit field is unset.
    pub const fn is_empty(&self) -> bool {
        self.max_file_bytes.is_none()
            && self.max_pages.is_none()
            && self.max_components.is_none()
            && self.max_page_pixels.is_none()
            && self.max_total_pixels.is_none()
            && self.max_decoded_bytes.is_none()
            && self.max_render_pixels.is_none()
    }

    /// Document-level inherited render ceiling from the public API contract.
    pub const fn inherited() -> Self {
        Self {
            max_render_pixels: Some(DEFAULT_MAX_RENDER_PIXELS),
            max_file_bytes: None,
            max_pages: None,
            max_components: None,
            max_page_pixels: None,
            max_total_pixels: None,
            max_decoded_bytes: None,
        }
    }
}

/// Options controlling document parse behaviour and resource limits.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParseOptions {
    /// Configurable resource limits checked before the document is fully parsed.
    pub limits: Option<ResourceLimits>,
}
