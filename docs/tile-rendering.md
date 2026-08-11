# Tile-first rendering contract

Issue [#691](https://github.com/matyushkin/djvu-rs/issues/691) tracks a
tile-oriented rendering API for viewer engines. This document is the formal
contract for the parts that have landed; the API lives in
[`src/djvu_tile.rs`](../src/djvu_tile.rs) (`djvu_rs::djvu_tile`).

## Slice 1: tile grid, coordinate space, deterministic tiles

### Coordinate space

Tiles live in **display space**: the pixel space of the final rendered image
*after* the combined rotation (INFO-chunk rotation + `RenderOptions::rotation`)
has been applied — exactly the pixels `render_pixmap` returns. The origin is
the top-left corner of the rotated page image, `x` grows right, `y` grows
down. Callers never deal with pre-rotation coordinates;
`TileLayout` performs the pull-back to the region renderer's pre-rotation
`RenderRect` internally (mapping table in the `to_render_rect` doc comment).

### Scale

Scale is expressed the same way as everywhere else in the render API: via
`RenderOptions::width`/`height` (build them with `fit_to_width`/`fit_to_box`).
`TileLayout::output_width()`/`output_height()` give the display canvas —
equal to `opts.width × opts.height` for identity/180° combined rotation,
swapped for 90°/270°.

### Grid and edge behavior

For tile size `ts`, the grid has `ceil(W / ts)` columns and `ceil(H / ts)`
rows over the `W × H` display canvas. Tile `(col, row)` covers

```
x ∈ [col·ts, min((col+1)·ts, W))    y ∈ [row·ts, min((row+1)·ts, H))
```

Edge tiles are **clipped** to the canvas — a tile pixmap never contains
padding pixels, so blitting tiles at their `tile_rect` offsets tiles the
canvas exactly once.

### Determinism guarantees (tested)

- **Assembly parity.** Blitting every tile at its display rectangle
  reproduces `render_pixmap` output **byte-for-byte** — including scaled
  renders, all four combined rotations, and the 1/4-resolution mask fast
  path taken at background subsample ≥ 4
  (`assembled_tiles_match_full_render_*` tests).
- **Order independence.** A tile's pixels are a pure function of
  `(page bytes, opts, tile_size, col, row)`. Request order never changes a
  byte — for the cached entry point this holds across cold misses, warm
  hits, and any interleaving (`tile_pixels_independent_of_request_order`).
- **Cache transparency.** `render_tile_cached` is byte-identical to
  `render_tile`; it routes through `render_region_tiled`, a memoization of
  the same compositor (see C4_TILE_CACHE notes there).

### Layer selection

Tiles composite the same layer stack as `render_pixmap` (mask + foreground +
background). Selecting individual layers (mask-only, background-only) is a
later slice of #691.

### Rejected options

`TileLayout::new` returns `RenderError::UnsupportedOption` for:

- `Resampling::Lanczos3` — a windowed whole-image resampling post-pass; its
  kernel windows straddle tile boundaries, so per-tile assembly cannot be
  byte-identical. Follow-up in #691.
- `aa: true` — a post-pass that *halves* the output; the tile grid would no
  longer match the produced pixels. Request the target size via
  `opts.width`/`height` instead.

`permissive: true` is allowed and inherits the region renderer's recovery
semantics (the cached path falls back to uncached rendering in that mode).

### Resource limits

Each tile render is bounded by the same `max_render_pixels` check as any
region render (limits inherited from the parent document at parse time).

## Renderer fix that fell out of slice 1

`render_region`/`render_region_tiled` previously always composited against
the full-resolution JB2 mask, while `render_into`/`render_rows` switch to a
pre-downsampled 1/4-resolution mask at background subsample ≥ 4 (no bold, no
FGbz). Region renders of heavily downscaled pages therefore diverged from
the matching crop of the full render. Both region entry points now take the
same `resolve_sub4_mask` decision (`render_region_matches_full_render_crop_at_sub4`).

## Planned follow-ups (#691)

- Explicit progressive quality tiers per tile (never regressing), on top of
  the BG44-chunk refinement primitives.
- Cancellation that stops remaining decode/composite work.
- Public cache budget / usage / eviction / invalidation API at tile
  granularity; bounded adjacent-tile prefetch.
- Layer selection (mask/foreground/background) per tile request.
- Lanczos-3 tiles (needs kernel-window-aware tile aprons).
- Async and WASM tile surfaces with equivalent observable semantics.
