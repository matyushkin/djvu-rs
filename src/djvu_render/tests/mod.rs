use super::*;
use crate::djvu_document::DjVuDocument;

/// #815: a refused output pixmap surfaces as the render-output limit, not
/// as a blank page.
#[test]
fn pixmap_too_large_maps_to_render_output_limit() {
    let e = RenderError::from(crate::pixmap::PixmapError::TooLarge {
        width: 10000,
        height: 10000,
        pixels: 100_000_000,
        max: Pixmap::MAX_PIXELS,
    });
    match e {
        RenderError::ResourceLimit(x) => {
            assert_eq!(
                x.axis,
                crate::resource_limits::ResourceLimitAxis::RenderOutputPixels
            );
            assert_eq!((x.found, x.limit), (100_000_000, Pixmap::MAX_PIXELS as u64));
            assert_eq!((x.width, x.height), (Some(10000), Some(10000)));
        }
        other => panic!("expected ResourceLimit, got {other:?}"),
    }
}

#[test]
fn pixmap_overflow_maps_to_invalid_dimensions() {
    let e = RenderError::from(crate::pixmap::PixmapError::Overflow {
        width: u32::MAX,
        height: u32::MAX,
    });
    assert!(matches!(
        e,
        RenderError::InvalidDimensions {
            width: u32::MAX,
            height: u32::MAX
        }
    ));
}

pub(super) fn assets_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/djvujs/library/assets")
}

/// Helper that returns an owned document so tests can borrow pages from it.
pub(super) fn load_doc(filename: &str) -> DjVuDocument {
    let data = std::fs::read(assets_path().join(filename))
        .unwrap_or_else(|_| panic!("{filename} must exist"));
    DjVuDocument::parse(&data).unwrap_or_else(|e| panic!("parse failed: {e}"))
}

// ── Shorthands ───────────────────────────────────────────────────────────────
//
// The tests below predate `RenderRequest` and were written against free
// functions that 1.0 removed. These shorthands keep them readable. The first
// group is the public request API; the second reaches the internal region
// renderers, which take a rectangle in native (pre-rotation) coordinates.

pub(super) fn render_into(
    page: &DjVuPage,
    opts: &RenderOptions,
    buf: &mut [u8],
) -> Result<(), RenderError> {
    RenderRequest::new(opts.clone()).write_rgba(page, buf)
}

pub(super) fn render_pixmap_with_limits(
    page: &DjVuPage,
    opts: &RenderOptions,
    limits: Option<crate::resource_limits::ResourceLimits>,
) -> Result<Pixmap, RenderError> {
    RenderRequest::new(opts.clone()).limits(limits).pixmap(page)
}

pub(super) fn render_streaming<F>(
    page: &DjVuPage,
    opts: &RenderOptions,
    sink: F,
) -> Result<(), RenderError>
where
    F: FnMut(usize, &[u8]),
{
    RenderRequest::new(opts.clone()).rows(page, sink)
}

/// A coarse render; `None` for a page without a background.
pub(super) fn render_coarse(
    page: &DjVuPage,
    opts: &RenderOptions,
) -> Result<Option<Pixmap>, RenderError> {
    match RenderRequest::new(opts.clone())
        .quality(Quality::Coarse)
        .pixmap(page)
    {
        Err(RenderError::NoBackground) => Ok(None),
        result => result.map(Some),
    }
}

pub(super) fn render_progressive(
    page: &DjVuPage,
    opts: &RenderOptions,
    step: usize,
) -> Result<Pixmap, RenderError> {
    RenderRequest::new(opts.clone())
        .quality(Quality::Step(step))
        .pixmap(page)
}

/// The uncached region renderer on a native (pre-rotation) rectangle.
pub(super) fn render_region(
    page: &DjVuPage,
    region: RenderRect,
    opts: &RenderOptions,
) -> Result<Pixmap, RenderError> {
    check_output_pixels("render_region", page, None, region.width, region.height)?;
    let pm = Composite::decode(page, opts, Detail::Full)?.region(region)?;
    Ok(rotate_pixmap(pm, opts.output_rotation(page)))
}

/// The tile-cached region renderer on a native (pre-rotation) rectangle.
pub(super) fn render_region_tiled(
    page: &DjVuPage,
    region: RenderRect,
    opts: &RenderOptions,
) -> Result<Pixmap, RenderError> {
    let pm = render_region_tiled_cancellable(page, region, opts, None)?;
    Ok(pm.expect("uncancellable render completed"))
}

mod aa_zoom;
mod banded;
mod compositor;
mod jpeg;
mod lanczos;
mod options;
mod permissive;
mod pipeline;
mod region;
mod streaming;
mod tile_cache;

use compositor::*;
use permissive::*;
use pipeline::*;
