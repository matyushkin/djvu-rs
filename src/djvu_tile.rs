//! Tile-first progressive rendering API for viewer engines (#691, slice 1).
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
//! Layer selection, explicit quality tiers, cancellation, cache budgeting,
//! and async/wasm surfaces are later slices of #691.

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
}
