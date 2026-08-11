//! Tile-first progressive rendering API for viewer engines (#691, slices 1–2).
//!
//! This module formalizes a tile-oriented contract on top of the existing
//! region renderer: a page render at a chosen output size is partitioned into
//! a grid of fixed-size square tiles addressed in **display space** — the
//! pixel space of the final, post-rotation output that a viewer puts on
//! screen. See `docs/tile-rendering.md` for the full written contract.
//!
//! Guarantees (slice 1):
//!
//! - **Coordinate space.** Tile `(col, row)` covers display-space rectangle
//!   `[col·ts, min((col+1)·ts, W)) × [row·ts, min((row+1)·ts, H))` where
//!   `W × H` is the display canvas ([`TileLayout::output_width`] /
//!   [`TileLayout::output_height`]) and `ts` the tile size. Edge tiles are
//!   clipped, never padded.
//! - **Assembly parity.** Blitting every tile at its display rectangle
//!   reproduces [`render_pixmap`](crate::djvu_render::render_pixmap) output
//!   byte-for-byte (bilinear resampling; all rotations).
//! - **Order independence.** Tile pixels are a pure function of the tile
//!   coordinate and the render options; request order (and cache state, for
//!   [`render_tile_cached`]) never changes a single byte.
//!
//! Slice 2 adds cache control at tile granularity: [`tile_cache_usage`],
//! [`set_tile_cache_budget`], [`clear_tile_cache`],
//! [`invalidate_tile_region`], and (with the `parallel` feature) bounded
//! background [`prefetch_tiles`]. Cache state never changes rendered bytes —
//! only latency.
//!
//! Layer selection, explicit quality tiers, cancellation, and async/wasm
//! surfaces are later slices of #691.

use crate::djvu_document::DjVuPage;
use crate::djvu_render::{
    RenderError, RenderOptions, RenderRect, Resampling, combine_rotations, render_region,
};
use crate::info::Rotation;
use crate::pixmap::Pixmap;

/// Error type for the tile API.
#[derive(Debug, thiserror::Error)]
pub enum TileError {
    /// The underlying region render failed.
    #[error(transparent)]
    Render(#[from] RenderError),

    /// The tile size is zero.
    #[error("tile size must be non-zero")]
    InvalidTileSize,

    /// The tile coordinate lies outside the tile grid.
    #[error("tile ({col}, {row}) out of range for a {cols}x{rows} tile grid")]
    OutOfRange {
        /// Requested column.
        col: u32,
        /// Requested row.
        row: u32,
        /// Number of columns in the grid.
        cols: u32,
        /// Number of rows in the grid.
        rows: u32,
    },
}

/// A tile's rectangle in **display space** (post-rotation output pixels).
///
/// Distinct from [`RenderRect`], whose offsets live in the pre-rotation
/// render canvas: a `TileRect` is where the tile lands on the viewer's
/// screen, `x` growing right and `y` growing down from the top-left corner
/// of the rotated page image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileRect {
    /// X offset in display pixels.
    pub x: u32,
    /// Y offset in display pixels.
    pub y: u32,
    /// Width in display pixels.
    pub width: u32,
    /// Height in display pixels.
    pub height: u32,
}

/// The tile grid for one page render: display canvas size, tile size, and
/// the coordinate mapping induced by the combined (INFO + user) rotation.
///
/// A layout is a pure value derived from `(page, opts, tile_size)`; building
/// it decodes nothing. Rebuilding it with equal inputs yields an equal value,
/// so callers may construct it per request or hold on to it — the rendered
/// pixels depend only on the inputs, never on layout identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileLayout {
    /// Pre-rotation full render canvas (`opts.width/height`, min 1).
    full_width: u32,
    full_height: u32,
    /// Post-rotation display canvas.
    output_width: u32,
    output_height: u32,
    tile_size: u32,
    /// Combined INFO-chunk + user rotation.
    rotation: Rotation,
}

impl TileLayout {
    /// Build the tile grid for rendering `page` with `opts` at `tile_size`.
    ///
    /// # Errors
    ///
    /// - [`TileError::InvalidTileSize`] if `tile_size == 0`.
    /// - [`RenderError::UnsupportedOption`] if `opts.resampling` is
    ///   [`Resampling::Lanczos3`] or `opts.aa` is set: both are
    ///   whole-pixmap post-passes (windowed resampling; 2× downscale that
    ///   halves the output) and do not commute with per-tile assembly, so
    ///   tiles could not honor the assembly-parity guarantee (follow-up in
    ///   #691). For a smaller output, set `opts.width`/`opts.height` to the
    ///   target size instead of relying on the `aa` halving.
    pub fn new(page: &DjVuPage, opts: &RenderOptions, tile_size: u32) -> Result<Self, TileError> {
        if tile_size == 0 {
            return Err(TileError::InvalidTileSize);
        }
        if opts.resampling == Resampling::Lanczos3 {
            return Err(TileError::Render(RenderError::UnsupportedOption(
                "Resampling::Lanczos3 does not commute with per-tile assembly (#691)",
            )));
        }
        if opts.aa {
            return Err(TileError::Render(RenderError::UnsupportedOption(
                "the aa halving post-pass does not commute with per-tile assembly (#691); \
                 request the target size directly instead",
            )));
        }
        let full_width = opts.width.max(1);
        let full_height = opts.height.max(1);
        let rotation = combine_rotations(page.rotation(), opts.rotation);
        let (output_width, output_height) = match rotation {
            Rotation::None | Rotation::Rot180 => (full_width, full_height),
            Rotation::Cw90 | Rotation::Ccw90 => (full_height, full_width),
        };
        Ok(TileLayout {
            full_width,
            full_height,
            output_width,
            output_height,
            tile_size,
            rotation,
        })
    }

    /// Display canvas width (post-rotation), in pixels.
    pub fn output_width(&self) -> u32 {
        self.output_width
    }

    /// Display canvas height (post-rotation), in pixels.
    pub fn output_height(&self) -> u32 {
        self.output_height
    }

    /// Tile edge length in pixels (edge tiles may be smaller).
    pub fn tile_size(&self) -> u32 {
        self.tile_size
    }

    /// Number of tile columns (`ceil(output_width / tile_size)`).
    pub fn cols(&self) -> u32 {
        self.output_width.div_ceil(self.tile_size)
    }

    /// Number of tile rows (`ceil(output_height / tile_size)`).
    pub fn rows(&self) -> u32 {
        self.output_height.div_ceil(self.tile_size)
    }

    /// Total number of tiles in the grid.
    pub fn tile_count(&self) -> u64 {
        u64::from(self.cols()) * u64::from(self.rows())
    }

    /// The display-space rectangle covered by tile `(col, row)`.
    ///
    /// Edge tiles are clipped to the canvas; no tile is ever padded.
    ///
    /// # Errors
    ///
    /// [`TileError::OutOfRange`] if `col >= cols()` or `row >= rows()`.
    pub fn tile_rect(&self, col: u32, row: u32) -> Result<TileRect, TileError> {
        let (cols, rows) = (self.cols(), self.rows());
        if col >= cols || row >= rows {
            return Err(TileError::OutOfRange {
                col,
                row,
                cols,
                rows,
            });
        }
        let x = col * self.tile_size;
        let y = row * self.tile_size;
        Ok(TileRect {
            x,
            y,
            width: self.tile_size.min(self.output_width - x),
            height: self.tile_size.min(self.output_height - y),
        })
    }

    /// Map a display-space rectangle to the pre-rotation [`RenderRect`] whose
    /// rotated render equals that display rectangle.
    ///
    /// The region renderer selects its sub-rectangle before applying the
    /// combined rotation, then rotates the small result
    /// (`rotate_pixmap` runs last in `render_region`), so the display
    /// rectangle must be pulled back through the inverse rotation. With
    /// `(W, H)` the pre-rotation canvas and `(x, y, w, h)` the display rect:
    ///
    /// | combined rotation | pre-rotation rect |
    /// |---|---|
    /// | `None`  | `(x, y, w, h)` |
    /// | `Cw90`  | `(y, H − x − w, h, w)` |
    /// | `Rot180`| `(W − x − w, H − y − h, w, h)` |
    /// | `Ccw90` | `(W − y − h, x, h, w)` |
    fn to_render_rect(self, r: TileRect) -> RenderRect {
        let (fw, fh) = (self.full_width, self.full_height);
        match self.rotation {
            Rotation::None => RenderRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
            },
            Rotation::Cw90 => RenderRect {
                x: r.y,
                y: fh - r.x - r.width,
                width: r.height,
                height: r.width,
            },
            Rotation::Rot180 => RenderRect {
                x: fw - r.x - r.width,
                y: fh - r.y - r.height,
                width: r.width,
                height: r.height,
            },
            Rotation::Ccw90 => RenderRect {
                x: fw - r.y - r.height,
                y: r.x,
                width: r.height,
                height: r.width,
            },
        }
    }
}

/// Render one tile of `page` at the grid position `(col, row)`.
///
/// `opts.width`/`opts.height` define the full-page render size exactly as for
/// [`render_pixmap`](crate::djvu_render::render_pixmap); the returned pixmap
/// has the dimensions of [`TileLayout::tile_rect`] for `(col, row)` and its
/// pixels are byte-identical to that rectangle of the full-page render.
///
/// Every call recomposites the tile from the page's cached decoded layers.
/// For interactive viewers prefer [`render_tile_cached`], which memoizes
/// composited output.
///
/// # Errors
///
/// - [`TileError::InvalidTileSize`] / [`TileError::OutOfRange`] for grid
///   violations, [`RenderError::UnsupportedOption`] for Lanczos-3 resampling.
/// - Propagates decode and resource-limit errors from the region renderer.
pub fn render_tile(
    page: &DjVuPage,
    opts: &RenderOptions,
    tile_size: u32,
    col: u32,
    row: u32,
) -> Result<Pixmap, TileError> {
    let layout = TileLayout::new(page, opts, tile_size)?;
    let rect = layout.tile_rect(col, row)?;
    Ok(render_region(page, layout.to_render_rect(rect), opts)?)
}

/// Render one tile, assembling it from the page's composited-tile cache.
///
/// Byte-identical to [`render_tile`] for every input — this routes through
/// [`render_region_tiled`](crate::djvu_render::render_region_tiled), a cache
/// in front of the same compositor (falling back to a plain region render
/// whenever the cache is not eligible). Request order never affects output:
/// cache entries are keyed by absolute position in the full render, so hits
/// and misses reproduce the same bytes.
///
/// # Errors
///
/// Same as [`render_tile`].
#[cfg(feature = "std")]
pub fn render_tile_cached(
    page: &DjVuPage,
    opts: &RenderOptions,
    tile_size: u32,
    col: u32,
    row: u32,
) -> Result<Pixmap, TileError> {
    let layout = TileLayout::new(page, opts, tile_size)?;
    let rect = layout.tile_rect(col, row)?;
    Ok(crate::djvu_render::render_region_tiled(
        page,
        layout.to_render_rect(rect),
        opts,
    )?)
}

/// Snapshot of one page's composited-tile cache (#691 slice 2).
///
/// The cache stores *internal* 256-pixel composited tiles (the granularity of
/// [`render_region_tiled`](crate::djvu_render::render_region_tiled)), which
/// back any caller-chosen [`render_tile_cached`] grid. `tiles` therefore
/// counts internal tiles, not caller tiles.
#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileCacheUsage {
    /// Bytes currently held by cached composited tiles.
    pub bytes: usize,
    /// Byte budget the cache enforces. Defaults to 8 MiB per page; override
    /// with [`set_tile_cache_budget`].
    pub budget: usize,
    /// Number of cached internal tiles.
    pub tiles: usize,
}

/// Current usage of `page`'s composited-tile cache.
///
/// Reading usage never renders or decodes anything.
#[cfg(feature = "std")]
pub fn tile_cache_usage(page: &DjVuPage) -> TileCacheUsage {
    let layers = page.render_layers();
    TileCacheUsage {
        bytes: layers.tile_cache_bytes(),
        budget: layers.tile_cache_budget(),
        tiles: layers.tile_cache_len(),
    }
}

/// Override `page`'s composited-tile cache byte budget (#691 slice 2).
///
/// Takes effect immediately: if the cache currently holds more than
/// `max_bytes`, the oldest tiles are evicted until it fits. A budget of `0`
/// effectively disables composited-tile caching for this page —
/// [`render_tile_cached`] stays correct, it just stops being warm.
///
/// The override is kept when the document's budget sweep *downgrades* the
/// page (`DjVuDocument::downgrade_render_caches`), but is reset to the
/// default when the page's whole render cache is dropped
/// (`DjVuPage::evict_render_cache`, `DjVuDocument::evict_render_caches`, or
/// an `enforce_cache_budget` eviction): the budget lives with the cache it
/// bounds.
#[cfg(feature = "std")]
pub fn set_tile_cache_budget(page: &DjVuPage, max_bytes: usize) {
    page.render_layers().set_tile_cache_budget(max_bytes);
}

/// Drop every cached composited tile of `page`, returning the bytes freed.
///
/// Decoded layers (mask, background, foreground) stay cached; only the
/// compositor's memoized output is invalidated. A budget override set via
/// [`set_tile_cache_budget`] survives.
#[cfg(feature = "std")]
pub fn clear_tile_cache(page: &DjVuPage) -> usize {
    page.render_layers().clear_tile_cache()
}

/// Invalidate every cached composited tile that intersects `region`,
/// returning the bytes freed (#691 slice 2).
///
/// `region` is a **display-space** rectangle under `opts` — the same
/// coordinate space as [`TileLayout::tile_rect`]; it is clipped to the
/// display canvas. Cached tiles are dropped across **all** cached render
/// sizes of the page, not just `opts.width × opts.height`: the region is
/// mapped proportionally into each cached size, rounding outward, so a tile
/// that touches the region at any scale is dropped rather than kept. Tiles
/// wholly outside the region stay warm.
///
/// # Errors
///
/// [`RenderError::UnsupportedOption`] for Lanczos-3 resampling or `aa`
/// (same eligibility as [`TileLayout::new`]).
#[cfg(feature = "std")]
pub fn invalidate_tile_region(
    page: &DjVuPage,
    opts: &RenderOptions,
    region: TileRect,
) -> Result<usize, TileError> {
    // Tile size is irrelevant here — the layout is only used for its canvas
    // dimensions and rotation pull-back.
    let layout = TileLayout::new(page, opts, 1)?;
    let x = region.x.min(layout.output_width);
    let y = region.y.min(layout.output_height);
    let width = region.width.min(layout.output_width - x);
    let height = region.height.min(layout.output_height - y);
    if width == 0 || height == 0 {
        return Ok(0);
    }
    let rect = layout.to_render_rect(TileRect {
        x,
        y,
        width,
        height,
    });
    Ok(page
        .render_layers()
        .remove_tiles_intersecting(rect, layout.full_width, layout.full_height))
}

/// Schedule a bounded background prefetch of the tiles around `(col, row)`
/// (#691 slice 2), returning how many tiles were scheduled.
///
/// Warms the same composited-tile cache [`render_tile_cached`] reads, for
/// every grid tile within Chebyshev distance `radius` of the center tile
/// (at most `(2·radius + 1)²`, clipped to the grid; `radius = 0` prefetches
/// just the center tile). The work runs on the shared rayon pool; whichever
/// side finishes a tile first populates the cache, the other observes it —
/// there is no separate prefetch buffer to race against.
///
/// This is a hint, not a guarantee: an out-of-range `page_index` is a no-op
/// returning `Ok(0)`, and decode errors inside the background task are
/// swallowed — a later foreground [`render_tile_cached`] call will surface
/// them. Retained bytes stay bounded by the page's tile-cache budget.
///
/// # Errors
///
/// Same as [`render_tile`] for grid violations and rejected options; the
/// center tile must lie inside the grid.
#[cfg(all(feature = "std", feature = "parallel"))]
pub fn prefetch_tiles(
    doc: &std::sync::Arc<crate::djvu_document::DjVuDocument>,
    page_index: usize,
    opts: &RenderOptions,
    tile_size: u32,
    col: u32,
    row: u32,
    radius: u32,
) -> Result<u64, TileError> {
    let Ok(page) = doc.page(page_index) else {
        return Ok(0);
    };
    let layout = TileLayout::new(page, opts, tile_size)?;
    layout.tile_rect(col, row)?;
    let c0 = col.saturating_sub(radius);
    let c1 = col.saturating_add(radius).min(layout.cols() - 1);
    let r0 = row.saturating_sub(radius);
    let r1 = row.saturating_add(radius).min(layout.rows() - 1);
    let scheduled = u64::from(c1 - c0 + 1) * u64::from(r1 - r0 + 1);
    let doc = std::sync::Arc::clone(doc);
    let opts = opts.clone();
    rayon::spawn(move || {
        let Ok(page) = doc.page(page_index) else {
            return;
        };
        for r in r0..=r1 {
            for c in c0..=c1 {
                let _ = render_tile_cached(page, &opts, tile_size, c, r);
            }
        }
    });
    Ok(scheduled)
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::djvu_document::DjVuDocument;
    use crate::djvu_render::{UserRotation, render_pixmap};

    fn assets_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/djvujs/library/assets")
    }

    fn load_doc(filename: &str) -> DjVuDocument {
        let data = std::fs::read(assets_path().join(filename))
            .unwrap_or_else(|_| panic!("{filename} must exist"));
        DjVuDocument::parse(&data).unwrap_or_else(|e| panic!("parse failed: {e}"))
    }

    /// Blit every tile of `layout` into one display-canvas pixmap.
    fn assemble<F>(layout: &TileLayout, mut tile: F) -> Pixmap
    where
        F: FnMut(u32, u32) -> Pixmap,
    {
        let mut out = Pixmap::white(layout.output_width(), layout.output_height());
        let stride = layout.output_width() as usize * 4;
        for row in 0..layout.rows() {
            for col in 0..layout.cols() {
                let rect = layout.tile_rect(col, row).unwrap();
                let pm = tile(col, row);
                assert_eq!((pm.width, pm.height), (rect.width, rect.height));
                for y in 0..rect.height as usize {
                    let src = y * rect.width as usize * 4;
                    let dst = (rect.y as usize + y) * stride + rect.x as usize * 4;
                    out.data[dst..dst + rect.width as usize * 4]
                        .copy_from_slice(&pm.data[src..src + rect.width as usize * 4]);
                }
            }
        }
        out
    }

    /// Assembled tiles are byte-identical to `render_pixmap` for every user
    /// rotation, on a layered (IW44 + JB2) page at a scaled size.
    #[test]
    fn assembled_tiles_match_full_render_all_rotations() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        for rotation in [
            UserRotation::None,
            UserRotation::Cw90,
            UserRotation::Rot180,
            UserRotation::Ccw90,
        ] {
            let opts = RenderOptions {
                width: 61, // deliberately not tile-size aligned
                height: 83,
                rotation,
                ..Default::default()
            };
            let full = render_pixmap(page, &opts).unwrap();
            let layout = TileLayout::new(page, &opts, 32).unwrap();
            assert_eq!(
                (full.width, full.height),
                (layout.output_width(), layout.output_height())
            );
            let stitched = assemble(&layout, |c, r| render_tile(page, &opts, 32, c, r).unwrap());
            assert_eq!(
                full.data, stitched.data,
                "stitched tiles must be byte-identical to the full render (rotation {rotation:?})"
            );
        }
    }

    /// Same parity on a bilevel page whose INFO chunk itself carries a
    /// rotation, combined with a user rotation.
    #[test]
    fn assembled_tiles_match_full_render_info_rotation() {
        let doc = load_doc("boy_jb2_rotate90.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 45,
            height: 57,
            rotation: UserRotation::Cw90,
            ..Default::default()
        };
        let full = render_pixmap(page, &opts).unwrap();
        let layout = TileLayout::new(page, &opts, 16).unwrap();
        let stitched = assemble(&layout, |c, r| render_tile(page, &opts, 16, c, r).unwrap());
        assert_eq!(full.data, stitched.data);
    }

    /// Parity holds on a pure-bilevel page at a downscale that activates the
    /// 1/4-resolution mask fast path in the full-page renderer.
    #[test]
    fn assembled_tiles_match_full_render_bilevel_downscale() {
        let doc = load_doc("boy_jb2.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 40,
            height: 52,
            ..Default::default()
        };
        let full = render_pixmap(page, &opts).unwrap();
        let layout = TileLayout::new(page, &opts, 16).unwrap();
        let stitched = assemble(&layout, |c, r| render_tile(page, &opts, 16, c, r).unwrap());
        assert_eq!(full.data, stitched.data);
    }

    /// Request order never changes pixels: forward, reverse, and cached
    /// renders of every tile agree byte-for-byte.
    #[test]
    fn tile_pixels_independent_of_request_order() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 61,
            height: 83,
            ..Default::default()
        };
        let layout = TileLayout::new(page, &opts, 32).unwrap();
        let coords: Vec<(u32, u32)> = (0..layout.rows())
            .flat_map(|r| (0..layout.cols()).map(move |c| (c, r)))
            .collect();

        // Uncached forward pass is the reference.
        let reference: Vec<Pixmap> = coords
            .iter()
            .map(|&(c, r)| render_tile(page, &opts, 32, c, r).unwrap())
            .collect();

        // Cached, in reverse order (cold cache → misses in reverse).
        for (i, &(c, r)) in coords.iter().enumerate().rev() {
            let pm = render_tile_cached(page, &opts, 32, c, r).unwrap();
            assert_eq!(pm.data, reference[i].data, "reverse pass, tile ({c}, {r})");
        }
        // Cached again, forward (warm cache → hits) — still identical.
        for (i, &(c, r)) in coords.iter().enumerate() {
            let pm = render_tile_cached(page, &opts, 32, c, r).unwrap();
            assert_eq!(pm.data, reference[i].data, "warm pass, tile ({c}, {r})");
        }
    }

    /// A tile size covering the whole canvas yields exactly the full render.
    #[test]
    fn single_tile_equals_full_render() {
        let doc = load_doc("boy.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 50,
            height: 70,
            ..Default::default()
        };
        let layout = TileLayout::new(page, &opts, 1024).unwrap();
        assert_eq!((layout.cols(), layout.rows()), (1, 1));
        let tile = render_tile(page, &opts, 1024, 0, 0).unwrap();
        let full = render_pixmap(page, &opts).unwrap();
        assert_eq!(tile.data, full.data);
    }

    /// Grid geometry: counts, clipped edge tiles, out-of-range coordinates.
    #[test]
    fn layout_geometry_and_errors() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 100,
            height: 65,
            ..Default::default()
        };
        let layout = TileLayout::new(page, &opts, 32).unwrap();
        assert_eq!((layout.cols(), layout.rows()), (4, 3));
        assert_eq!(layout.tile_count(), 12);
        // Interior tile is full-size; edge tiles are clipped, not padded.
        assert_eq!(
            layout.tile_rect(0, 0).unwrap(),
            TileRect {
                x: 0,
                y: 0,
                width: 32,
                height: 32
            }
        );
        assert_eq!(
            layout.tile_rect(3, 2).unwrap(),
            TileRect {
                x: 96,
                y: 64,
                width: 4,
                height: 1
            }
        );
        assert!(matches!(
            layout.tile_rect(4, 0),
            Err(TileError::OutOfRange {
                col: 4,
                row: 0,
                cols: 4,
                rows: 3
            })
        ));
        assert!(matches!(
            render_tile(page, &opts, 32, 0, 3),
            Err(TileError::OutOfRange { .. })
        ));

        assert!(matches!(
            TileLayout::new(page, &opts, 0),
            Err(TileError::InvalidTileSize)
        ));

        let lanczos = RenderOptions {
            resampling: Resampling::Lanczos3,
            ..opts
        };
        assert!(matches!(
            TileLayout::new(page, &lanczos, 32),
            Err(TileError::Render(RenderError::UnsupportedOption(_)))
        ));
        let aa = RenderOptions { aa: true, ..opts };
        assert!(matches!(
            TileLayout::new(page, &aa, 32),
            Err(TileError::Render(RenderError::UnsupportedOption(_)))
        ));
    }

    /// 90° rotations swap the display canvas relative to `opts.width/height`.
    #[test]
    fn rotated_layout_swaps_display_dimensions() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 80,
            height: 60,
            rotation: UserRotation::Cw90,
            ..Default::default()
        };
        let layout = TileLayout::new(page, &opts, 32).unwrap();
        assert_eq!(
            (layout.output_width(), layout.output_height()),
            (60, 80),
            "Cw90 display canvas must be opts.height × opts.width"
        );
    }

    /// Warm every cached tile of the `opts`-sized render of `page` via the
    /// caller grid that matches the internal 256-px cache granularity.
    fn warm_grid(page: &DjVuPage, opts: &RenderOptions) {
        let layout = TileLayout::new(page, opts, 256).unwrap();
        for row in 0..layout.rows() {
            for col in 0..layout.cols() {
                render_tile_cached(page, opts, 256, col, row).unwrap();
            }
        }
    }

    /// Usage reporting, budget enforcement (including shrink-on-set and
    /// budget 0 = caching off), and clear-with-budget-preserved semantics.
    #[test]
    fn cache_usage_budget_and_clear() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 600,
            height: 800,
            ..Default::default()
        };

        let fresh = tile_cache_usage(page);
        assert_eq!((fresh.bytes, fresh.tiles), (0, 0));
        assert_eq!(fresh.budget, 8 * 1024 * 1024, "default budget is 8 MiB");

        warm_grid(page, &opts);
        let warm = tile_cache_usage(page);
        assert!(warm.bytes > 0 && warm.tiles > 0);

        // Shrinking the budget below current usage evicts immediately.
        set_tile_cache_budget(page, 300_000);
        let shrunk = tile_cache_usage(page);
        assert!(shrunk.bytes <= 300_000, "usage {} > budget", shrunk.bytes);
        assert!(shrunk.tiles < warm.tiles);
        assert_eq!(shrunk.budget, 300_000);

        // Under a tiny budget rendering stays byte-correct, just cold.
        let cached = render_tile_cached(page, &opts, 256, 0, 0).unwrap();
        let direct = render_tile(page, &opts, 256, 0, 0).unwrap();
        assert_eq!(cached.data, direct.data);
        assert!(tile_cache_usage(page).bytes <= 300_000);

        // Budget 0 disables caching entirely; correctness is unaffected.
        set_tile_cache_budget(page, 0);
        let cached = render_tile_cached(page, &opts, 256, 1, 1).unwrap();
        let direct = render_tile(page, &opts, 256, 1, 1).unwrap();
        assert_eq!(cached.data, direct.data);
        assert_eq!(tile_cache_usage(page).bytes, 0);

        // clear_tile_cache reports the bytes it freed and keeps the budget.
        set_tile_cache_budget(page, 8 * 1024 * 1024);
        warm_grid(page, &opts);
        let before = tile_cache_usage(page);
        let freed = clear_tile_cache(page);
        assert_eq!(freed, before.bytes);
        let after = tile_cache_usage(page);
        assert_eq!((after.bytes, after.tiles), (0, 0));
        assert_eq!(after.budget, 8 * 1024 * 1024);
    }

    /// Region invalidation drops exactly the overlapping tiles — across all
    /// cached render sizes — and re-rendering restores identical bytes.
    #[test]
    fn invalidate_region_drops_overlapping_tiles_across_scales() {
        let doc = load_doc("chicken.djvu");
        let page = doc.page(0).unwrap();
        let big = RenderOptions {
            width: 600,
            height: 800,
            ..Default::default()
        };
        let small = RenderOptions {
            width: 300,
            height: 400,
            ..Default::default()
        };
        warm_grid(page, &big); // internal tiles: 3 cols × 4 rows = 12
        warm_grid(page, &small); // internal tiles: 2 cols × 2 rows = 4
        let reference = render_tile_cached(page, &big, 256, 0, 0).unwrap();
        let before = tile_cache_usage(page);
        assert_eq!(before.tiles, 16);

        // Left half of the 600-wide display canvas. At 600: x < 300 drops
        // columns 0 and 256, keeps 512 (4 tiles survive). At 300 the rect
        // scales to x < 150: drops column 0, keeps 256 (2 tiles survive).
        let freed = invalidate_tile_region(
            page,
            &big,
            TileRect {
                x: 0,
                y: 0,
                width: 300,
                height: 800,
            },
        )
        .unwrap();
        assert!(freed > 0);
        let after = tile_cache_usage(page);
        assert_eq!(after.tiles, 6, "only right-column tiles survive");
        assert_eq!(after.bytes, before.bytes - freed);

        // Re-rendering a dropped tile reproduces the original bytes.
        let rerendered = render_tile_cached(page, &big, 256, 0, 0).unwrap();
        assert_eq!(rerendered.data, reference.data);

        // A region wholly outside every cached tile frees nothing.
        assert_eq!(
            invalidate_tile_region(
                page,
                &big,
                TileRect {
                    x: 599,
                    y: 799,
                    width: 0,
                    height: 0,
                },
            )
            .unwrap(),
            0
        );
    }

    /// A display-space region under a rotated view invalidates the same
    /// pre-rotation tiles as the equivalent unrotated region.
    #[test]
    fn invalidate_maps_display_rect_through_rotation() {
        let identity = RenderOptions {
            width: 600,
            height: 800,
            ..Default::default()
        };
        let rotated = RenderOptions {
            rotation: UserRotation::Cw90,
            ..identity.clone()
        };

        // Two identically warmed caches (separate documents = separate caches).
        let doc_a = load_doc("chicken.djvu");
        let page_a = doc_a.page(0).unwrap();
        warm_grid(page_a, &identity);
        let doc_b = load_doc("chicken.djvu");
        let page_b = doc_b.page(0).unwrap();
        warm_grid(page_b, &identity);

        // Under Cw90 the display canvas is 800×600 and its top strip
        // `{0, 0, 800, 300}` pulls back to the pre-rotation left strip
        // `{0, 0, 300, 800}` — the same region as the identity-space rect.
        let freed_identity = invalidate_tile_region(
            page_a,
            &identity,
            TileRect {
                x: 0,
                y: 0,
                width: 300,
                height: 800,
            },
        )
        .unwrap();
        let freed_rotated = invalidate_tile_region(
            page_b,
            &rotated,
            TileRect {
                x: 0,
                y: 0,
                width: 800,
                height: 300,
            },
        )
        .unwrap();
        assert!(freed_identity > 0);
        assert_eq!(freed_identity, freed_rotated);
        assert_eq!(
            tile_cache_usage(page_a).tiles,
            tile_cache_usage(page_b).tiles
        );
    }

    /// Prefetch warms the same cache `render_tile_cached` reads, without
    /// changing a byte of output; out-of-range pages are a no-op.
    #[cfg(feature = "parallel")]
    #[test]
    fn prefetch_tiles_warms_cache() {
        let doc = std::sync::Arc::new(load_doc("chicken.djvu"));
        let page = doc.page(0).unwrap();
        let opts = RenderOptions {
            width: 600,
            height: 800,
            ..Default::default()
        };

        assert_eq!(prefetch_tiles(&doc, 99, &opts, 256, 0, 0, 1).unwrap(), 0);
        assert!(matches!(
            prefetch_tiles(&doc, 0, &opts, 256, 99, 0, 1),
            Err(TileError::OutOfRange { .. })
        ));

        // radius 1 around (0,0) on a 3×4 grid clips to a 2×2 neighborhood.
        let scheduled = prefetch_tiles(&doc, 0, &opts, 256, 0, 0, 1).unwrap();
        assert_eq!(scheduled, 4);

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while tile_cache_usage(page).tiles < 4 && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            tile_cache_usage(page).tiles >= 4,
            "prefetch never landed: {:?}",
            tile_cache_usage(page)
        );
        let warm = render_tile_cached(page, &opts, 256, 0, 0).unwrap();
        let direct = render_tile(page, &opts, 256, 0, 0).unwrap();
        assert_eq!(warm.data, direct.data);
    }
}
