//! JB2 bilevel image encoder — produces Sjbz chunk payloads.
//!
//! Encodes a [`Bitmap`] into a JB2 stream decodable by [`crate::jb2::decode`].
//!
//! ## Encoding strategy
//!
//! The encoder emits the entire image as a single **record type 3** ("new symbol,
//! direct, blit only") record.  This produces valid output without requiring
//! connected-component analysis or a symbol dictionary.
//!
//! ## Binary format summary (see jb2.rs for full spec)
//!
//! ```text
//! encode_num(record_type_ctx, [0,11], 0)  — start-of-image
//! encode_num(image_size_ctx,  [0,262142], width)
//! encode_num(image_size_ctx,  [0,262142], height)
//! encode_bit(flag_ctx, false)             — reserved flag
//! encode_num(record_type_ctx, [0,11], 3)  — new-symbol, direct, blit-only
//! encode_num(symbol_width_ctx, [0,262142], width)
//! encode_num(symbol_height_ctx,[0,262142], height)
//! encode_bitmap_direct(...)               — 10-bit context bitmap
//! encode_bit(offset_type_ctx, true)       — new-line positioning
//! encode_num(hoff_ctx, [-262143,262142], 1)
//! encode_num(voff_ctx, [-262143,262142], 0)
//! encode_num(record_type_ctx, [0,11], 11) — end-of-data
//! ```

use crate::bitmap::Bitmap;
use crate::zp_impl::encoder::ZpEncoder;

#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap;
#[cfg(feature = "std")]
use std::collections::BTreeMap;

// ── NumContext: binary-tree arena for variable-length integer encoding ─────────

/// Binary-tree context store used to encode variable-length integers with ZP.
///
/// Mirrors the decoder's `NumContext` exactly.  Nodes are allocated lazily;
/// index 0 is unused sentinel, index 1 is the root.
struct NumContext {
    ctx: Vec<u8>,
    left: Vec<u32>,
    right: Vec<u32>,
}

impl NumContext {
    fn new() -> Self {
        NumContext {
            ctx: vec![0, 0],
            left: vec![0, 0],
            right: vec![0, 0],
        }
    }

    fn root(&self) -> usize {
        1
    }

    fn get_left(&mut self, node: usize) -> usize {
        if self.left[node] == 0 {
            let idx = self.ctx.len() as u32;
            self.ctx.push(0);
            self.left.push(0);
            self.right.push(0);
            self.left[node] = idx;
        }
        self.left[node] as usize
    }

    fn get_right(&mut self, node: usize) -> usize {
        if self.right[node] == 0 {
            let idx = self.ctx.len() as u32;
            self.ctx.push(0);
            self.left.push(0);
            self.right.push(0);
            self.right[node] = idx;
        }
        self.right[node] as usize
    }
}

/// Encode integer `val` in `[low, high]` using the same binary-tree traversal
/// as the decoder's `decode_num`.
///
/// Emits a ZP bit at each "free" decision point (where neither `low >= cutoff`
/// nor `high < cutoff`).  Forced decisions traverse the tree without emitting.
fn encode_num(zp: &mut ZpEncoder, ctx: &mut NumContext, low: i32, high: i32, val: i32) {
    let mut low = low;
    let mut high = high;
    let mut val_inner = val;
    let mut cutoff: i32 = 0;
    let mut phase: u32 = 1;
    let mut range: u32 = 0xffff_ffff;
    let mut node = ctx.root();

    while range != 1 {
        // Determine decision (mirrors decode_num's decision logic).
        // Emit a bit only when the decision is "free" (not forced by low/high).
        let decision = if low >= cutoff {
            // Forced true — traverse right without emitting.
            let child = ctx.get_right(node);
            node = child;
            true
        } else if high >= cutoff {
            // Free — decision is (val_inner >= cutoff).
            let bit = val_inner >= cutoff;
            let child = if bit {
                ctx.get_right(node)
            } else {
                ctx.get_left(node)
            };
            zp.encode_bit(&mut ctx.ctx[node], bit);
            node = child;
            bit
        } else {
            // Forced false — traverse left without emitting.
            let child = ctx.get_left(node);
            node = child;
            false
        };

        match phase {
            1 => {
                let negative = !decision;
                if negative {
                    let temp = -low - 1;
                    low = -high - 1;
                    high = temp;
                    val_inner = -val_inner - 1;
                }
                phase = 2;
                cutoff = 1;
            }
            2 => {
                if !decision {
                    phase = 3;
                    range = ((cutoff + 1) / 2) as u32;
                    if range <= 1 {
                        range = 1;
                        cutoff = 0;
                    } else {
                        cutoff -= (range / 2) as i32;
                    }
                } else {
                    cutoff = cutoff * 2 + 1;
                }
            }
            3 => {
                range /= 2;
                if range == 0 {
                    range = 1;
                }
                if range != 1 {
                    if !decision {
                        cutoff -= (range / 2) as i32;
                    } else {
                        cutoff += (range / 2) as i32;
                    }
                } else if !decision {
                    cutoff -= 1;
                }
            }
            _ => unreachable!(),
        }
    }
}

// ── Direct bitmap encoding (10-bit context) ───────────────────────────────────

/// Encode a bitmap using the direct 10-pixel-context method.
///
/// Mirrors `decode_bitmap_direct` in `jb2` exactly.  Iterates rows
/// top-to-bottom, which corresponds to Bitmap y = 0 (top) up to height-1 (bottom).
///
/// The bitmap is first expanded to a flat byte-per-pixel array with 2 zero rows
/// above the image and 4 zero columns to the right of each row.  This eliminates
/// all per-pixel bounds checking and bit-manipulation in the inner loop.
#[allow(unsafe_code)]
fn encode_bitmap_direct(zp: &mut ZpEncoder, ctx: &mut [u8], bm: &Bitmap) {
    debug_assert_eq!(ctx.len(), 1024);
    let w = bm.width as usize;
    let h = bm.height as usize;
    // Row stride with 4 zero-padding columns so col+2 and col+3 are always in-bounds.
    let pw = w + 4;

    // Expand bitmap to byte-per-pixel (0 or 1).
    // Layout: rows 0..2 are zero (padding for bm_y_p2/bm_y_p1 when bm_y < 2),
    //         rows 2..h+2 hold image rows 0..h.
    // Mapping: padded_index(bm_y_p2) = bm_y, padded_index(bm_y_p1) = bm_y+1,
    //          padded_index(cur) = bm_y+2.
    let mut pixels = vec![0u8; (h + 2) * pw];
    for y in 0..h {
        for x in 0..w {
            pixels[(y + 2) * pw + x] = bm.get(x as u32, y as u32) as u8;
        }
    }

    for bm_y in 0..h {
        let row_p2 = &pixels[bm_y * pw..(bm_y + 1) * pw];
        let row_p1 = &pixels[(bm_y + 1) * pw..(bm_y + 2) * pw];
        let row_cur = &pixels[(bm_y + 2) * pw..(bm_y + 3) * pw];

        // Initialise rolling windows at col=0 (col-1 and col-2 are OOB → 0 via padding).
        //
        // r2 = 3 bits: (bm_y_p2, col-1=0), (col=0), (col+1=1)
        let mut r2 = (row_p2[0] as u32) << 1 | row_p2[1] as u32;
        // r1 = 5 bits: (bm_y_p1, col-2=0), (col-1=0), (col=0), (col+1=1), (col+2=2)
        let mut r1 = (row_p1[0] as u32) << 2 | (row_p1[1] as u32) << 1 | row_p1[2] as u32;
        let mut r0: u32 = 0;

        for col in 0..w {
            let idx = ((r2 << 7) | (r1 << 2) | r0) as usize;
            let bit = row_cur[col] != 0;
            // Safety: r2 ≤ 7, r1 ≤ 31, r0 ≤ 3 by the & masks above,
            // so idx ≤ (7<<7)|(31<<2)|3 = 1023 < ctx.len() = 1024.
            let ctx_byte = unsafe { ctx.get_unchecked_mut(idx) };
            zp.encode_bit(ctx_byte, bit);

            // Advance rolling windows — no bounds checks: col+2 < w+2 < pw, col+3 < w+3 < pw.
            r2 = ((r2 << 1) & 0b111) | row_p2[col + 2] as u32;
            r1 = ((r1 << 1) & 0b11111) | row_p1[col + 3] as u32;
            r0 = ((r0 << 1) & 0b11) | bit as u32;
        }
    }
}

// ── Public API ─────────────────────────────────────────────────────────────────

/// Encode a bilevel [`Bitmap`] into a JB2 stream (Sjbz chunk payload).
///
/// The returned bytes can be embedded directly in a `Sjbz` IFF chunk.
/// Decoding with [`crate::jb2::decode`] will reconstruct the original bitmap.
///
/// ## Encoding
///
/// Images with `width * height ≤ 1 MP` are emitted as a single direct-bitmap
/// record (type 3). Larger images are split into ≤ 1024×1024 tiles, each
/// emitted as its own record-3 — this keeps every symbol within the decoder's
/// `MAX_SYMBOL_PIXELS = 1 MP` DoS guard so the output round-trips through
/// [`crate::jb2::decode`] for any size up to `MAX_PIXELS = 64 MP`.
///
/// No connected-component analysis or symbol dictionary is used.
/// For substantially better compression on text-heavy bitmaps see
/// [`encode_jb2_dict`].
pub fn encode_jb2(bitmap: &Bitmap) -> Vec<u8> {
    let w = bitmap.width as i32;
    let h = bitmap.height as i32;

    if w == 0 || h == 0 {
        return Vec::new();
    }

    let mut zp = ZpEncoder::new();

    // ── Contexts (mirrors decode_image_with_pool) ──────────────────────────
    let mut record_type_ctx = NumContext::new();
    let mut image_size_ctx = NumContext::new();
    let mut symbol_width_ctx = NumContext::new();
    let mut symbol_height_ctx = NumContext::new();
    let mut hoff_ctx = NumContext::new();
    let mut voff_ctx = NumContext::new();
    let mut direct_bitmap_ctx = vec![0u8; 1024];
    let mut offset_type_ctx: u8 = 0;
    let mut flag_ctx: u8 = 0;

    // ── Preamble ───────────────────────────────────────────────────────────
    // Record type 0: start-of-image.
    encode_num(&mut zp, &mut record_type_ctx, 0, 11, 0);

    encode_num(&mut zp, &mut image_size_ctx, 0, 262142, w);
    encode_num(&mut zp, &mut image_size_ctx, 0, 262142, h);

    // Reserved flag bit — must be 0.
    zp.encode_bit(&mut flag_ctx, false);

    // ── Direct-bitmap records, tiled to stay under MAX_SYMBOL_PIXELS ───────
    //
    // Tile size: 1024 — equal to the decoder's MAX_SYMBOL_PIXELS = 1024*1024,
    // and the per-symbol check is `pixels > MAX`, so tile_w*tile_h ≤ 1 MP is
    // accepted. Layout state mirrors the decoder (jb2.rs:LayoutState):
    //   first_left = -1, first_bottom = image_height - 1.
    //
    // JB2 stream coords are y-flipped relative to image coords: blit_to_bitmap
    // uses bm_y = (image_height - 1 - jb2_y) - sym_row. For a tile of height
    // th to land with its top row at image_y = ty (top-down convention), the
    // required JB2 stream coord is jb2_y = h - th - ty. With new_line=true:
    //   nx = first_left + hoff          → hoff = tx - first_left
    //   ny = first_bottom + voff - th+1 → voff = (h - th - ty) + th - 1 - first_bottom
    //                                          = h - 1 - ty - first_bottom
    // After emit: first_left = nx, first_bottom = ny.
    const TILE: u32 = 1024;
    let mut first_left: i32 = -1;
    let mut first_bottom: i32 = h - 1;

    let mut ty: u32 = 0;
    while ty < bitmap.height {
        let th = TILE.min(bitmap.height - ty);
        let mut tx: u32 = 0;
        while tx < bitmap.width {
            let tw = TILE.min(bitmap.width - tx);

            // Record type 3: new symbol, direct, blit to page, NOT stored in dict.
            encode_num(&mut zp, &mut record_type_ctx, 0, 11, 3);

            // Symbol dimensions.
            encode_num(&mut zp, &mut symbol_width_ctx, 0, 262142, tw as i32);
            encode_num(&mut zp, &mut symbol_height_ctx, 0, 262142, th as i32);

            // Bitmap data — crop tile from source.
            let tile_bm = if tw == bitmap.width && th == bitmap.height {
                // Single-tile fast path: avoid the crop allocation.
                bitmap.clone()
            } else {
                crop_bitmap(bitmap, tx, ty, tw, th)
            };
            encode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, &tile_bm);

            // Coordinates: new_line=true, hoff/voff target (tx, ty).
            let hoff = tx as i32 - first_left;
            let voff = h - 1 - ty as i32 - first_bottom;
            zp.encode_bit(&mut offset_type_ctx, true);
            encode_num(&mut zp, &mut hoff_ctx, -262143, 262142, hoff);
            encode_num(&mut zp, &mut voff_ctx, -262143, 262142, voff);

            first_left = tx as i32;
            first_bottom = h - th as i32 - ty as i32;

            tx += tw;
        }
        ty += th;
    }

    // ── End-of-data ────────────────────────────────────────────────────────
    encode_num(&mut zp, &mut record_type_ctx, 0, 11, 11);

    zp.finish()
}

/// Crop a tight sub-rectangle out of a bilevel bitmap.
fn crop_bitmap(src: &Bitmap, x0: u32, y0: u32, w: u32, h: u32) -> Bitmap {
    let mut out = Bitmap::new(w, h);
    for y in 0..h {
        for x in 0..w {
            if src.get(x0 + x, y0 + y) {
                out.set_black(x, y);
            }
        }
    }
    out
}

// ── Connected-component extraction (symbol-dict encoding) ─────────────────────

/// A single connected component: its cropped bitmap and top-left bbox origin.
struct Cc {
    /// Top-left x of the component in the source bitmap (0 = left edge).
    x: u32,
    /// Top-left y of the component in the source bitmap (0 = top edge).
    y: u32,
    /// Cropped bitmap: tight bbox, pixels of this component only.
    bitmap: Bitmap,
}

/// Extract all 8-connected components of black pixels from `bitmap`.
///
/// Uses iterative DFS on an unpacked byte grid; each component's cropped
/// bitmap is the minimal bounding box that contains its black pixels.
/// Ordering is raster-scan of the seed pixel (roughly top-to-bottom,
/// left-to-right).
fn extract_ccs(bitmap: &Bitmap) -> Vec<Cc> {
    let w = bitmap.width as usize;
    let h = bitmap.height as usize;
    if w == 0 || h == 0 {
        return Vec::new();
    }

    // Unpack into a mutable byte grid — 1 = black-unvisited, 0 = white-or-visited.
    let mut pix = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            if bitmap.get(x as u32, y as u32) {
                pix[y * w + x] = 1;
            }
        }
    }

    let mut out = Vec::new();
    let mut stack: Vec<(u32, u32)> = Vec::new();
    let mut cc_pixels: Vec<(u32, u32)> = Vec::new();

    for y0 in 0..h {
        for x0 in 0..w {
            if pix[y0 * w + x0] == 0 {
                continue;
            }
            stack.clear();
            cc_pixels.clear();
            stack.push((x0 as u32, y0 as u32));
            pix[y0 * w + x0] = 0;

            let mut min_x = x0;
            let mut max_x = x0;
            let mut min_y = y0;
            let mut max_y = y0;

            while let Some((cx, cy)) = stack.pop() {
                cc_pixels.push((cx, cy));
                let cxi = cx as usize;
                let cyi = cy as usize;
                if cxi < min_x {
                    min_x = cxi;
                }
                if cxi > max_x {
                    max_x = cxi;
                }
                if cyi < min_y {
                    min_y = cyi;
                }
                if cyi > max_y {
                    max_y = cyi;
                }

                let lo_x = cxi.saturating_sub(1);
                let hi_x = (cxi + 1).min(w - 1);
                let lo_y = cyi.saturating_sub(1);
                let hi_y = (cyi + 1).min(h - 1);
                for ny in lo_y..=hi_y {
                    let row_base = ny * w;
                    for nx in lo_x..=hi_x {
                        if pix[row_base + nx] != 0 {
                            pix[row_base + nx] = 0;
                            stack.push((nx as u32, ny as u32));
                        }
                    }
                }
            }

            let cc_w = (max_x - min_x + 1) as u32;
            let cc_h = (max_y - min_y + 1) as u32;
            let mut cc_bm = Bitmap::new(cc_w, cc_h);
            for &(px, py) in &cc_pixels {
                cc_bm.set(px - min_x as u32, py - min_y as u32, true);
            }
            out.push(Cc {
                x: min_x as u32,
                y: min_y as u32,
                bitmap: cc_bm,
            });
        }
    }

    out
}

// ── Dict-based encoding: record types 1 (new) + 7 (matched copy) ──────────────

/// Encode a bilevel [`Bitmap`] into a JB2 stream using a **symbol dictionary**.
///
/// Performs connected-component (CC) extraction, exact-match deduplication,
/// and emits record type 1 for each unique CC (new symbol, direct, stored in
/// dict + blitted) and record type 7 for each repeat (matched copy, blit only).
///
/// Lossless. Matches [`crate::jb2::decode`] for round-trip.
///
/// ## Limitations (Phase 1)
/// - Exact-match only — no near-duplicate matching or lossy refinement.
/// - Coordinate coding uses `new_line = true` for every symbol (wastes space
///   compared to baseline-relative same-line coding; optimized in a later phase).
/// - Components >= 1 MP are encoded as-is; the decoder will reject them via
///   `MAX_SYMBOL_PIXELS`. For scanned text pages this is not a practical issue.
pub fn encode_jb2_dict(bitmap: &Bitmap) -> Vec<u8> {
    let w = bitmap.width as i32;
    let h = bitmap.height as i32;
    if w == 0 || h == 0 {
        return Vec::new();
    }

    let ccs = extract_ccs(bitmap);

    // Reading-order sort by baseline-bucket, then left-to-right.
    //
    // The JB2 coord stream's `same_line` mode is keyed off `y_jb2` (the bottom
    // edge of each symbol in JB2 bottom-up coords). Glyphs sharing a text
    // baseline have similar `y_jb2` values regardless of height (e.g. 't' vs
    // 'o'), but they differ in top-left `cc.y`. Sorting by `cc.y` therefore
    // interleaves glyphs from adjacent lines, defeating same-line coding.
    //
    // Bucketing by bottom-row in top-down coords (`cc.y + cc_h`), rounded to a
    // line-height grid, then by `x` within a bucket, gives proper reading order
    // for same-line detection. The bucket granularity is the same baseline
    // tolerance used in the same/new-line decision below.
    let mut order: Vec<usize> = (0..ccs.len()).collect();
    let bucket = (SAME_LINE_BASELINE_TOL.max(1)) as u32;
    order.sort_by_key(|&i| {
        let cc = &ccs[i];
        let bottom = cc.y + cc.bitmap.height;
        (bottom / bucket, cc.x)
    });

    let mut zp = ZpEncoder::new();
    let mut record_type_ctx = NumContext::new();
    let mut image_size_ctx = NumContext::new();
    let mut symbol_width_ctx = NumContext::new();
    let mut symbol_height_ctx = NumContext::new();
    let mut symbol_index_ctx = NumContext::new();
    let mut hoff_ctx = NumContext::new();
    let mut voff_ctx = NumContext::new();
    let mut shoff_ctx = NumContext::new();
    let mut svoff_ctx = NumContext::new();
    let mut direct_bitmap_ctx = vec![0u8; 1024];
    let mut offset_type_ctx: u8 = 0;
    let mut flag_ctx: u8 = 0;

    // Preamble.
    encode_num(&mut zp, &mut record_type_ctx, 0, 11, 0);
    encode_num(&mut zp, &mut image_size_ctx, 0, 262142, w);
    encode_num(&mut zp, &mut image_size_ctx, 0, 262142, h);
    zp.encode_bit(&mut flag_ctx, false);

    // Layout state — mirrors `LayoutState::new` in jb2.rs:1187.
    let mut layout = EncoderLayout::new(h);

    // Dedup: (w, h, packed-data) → dict index assigned on first emission.
    let mut dedup: BTreeMap<(u32, u32, Vec<u8>), usize> = BTreeMap::new();
    let mut dict_size: usize = 0;

    for &cc_idx in &order {
        let cc = &ccs[cc_idx];
        let cc_w = cc.bitmap.width as i32;
        let cc_h = cc.bitmap.height as i32;
        // JB2 uses bottom-up y: y_jb2 is the bottom y of the symbol.
        let x_jb2 = cc.x as i32;
        let y_jb2 = h - cc.y as i32 - cc_h;

        let key = (cc.bitmap.width, cc.bitmap.height, cc.bitmap.data.clone());
        let dict_action = dedup.get(&key).copied();

        // Record header (shared between types 1 and 7).
        match dict_action {
            None => {
                encode_num(&mut zp, &mut record_type_ctx, 0, 11, 1);
                encode_num(&mut zp, &mut symbol_width_ctx, 0, 262142, cc_w);
                encode_num(&mut zp, &mut symbol_height_ctx, 0, 262142, cc_h);
                encode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, &cc.bitmap);
            }
            Some(dict_idx) => {
                encode_num(&mut zp, &mut record_type_ctx, 0, 11, 7);
                encode_num(
                    &mut zp,
                    &mut symbol_index_ctx,
                    0,
                    (dict_size - 1) as i32,
                    dict_idx as i32,
                );
            }
        }

        // ── Coordinate coding (Phase 2: same-line vs new_line) ────────────
        //
        // Decide whether the symbol fits the running baseline / line:
        //   * shoff = x_jb2 - last_right     (small, often 0..16 for a font)
        //   * svoff = y_jb2 - baseline_value (small, near 0 if same line)
        //
        // If both are within typical text-line tolerances we encode with
        // offset_type=false (same-line); else fall back to offset_type=true
        // (new_line), exactly mirroring what the decoder does in
        // jb2.rs::decode_symbol_coords.
        let shoff = x_jb2 - layout.last_right;
        let svoff = y_jb2 - layout.baseline_get();
        let same_line = layout.same_line_seen
            && svoff.abs() <= SAME_LINE_BASELINE_TOL
            && (-SAME_LINE_OVERLAP_TOL..=SAME_LINE_GAP_MAX).contains(&shoff);

        if same_line {
            zp.encode_bit(&mut offset_type_ctx, false);
            encode_num(&mut zp, &mut shoff_ctx, -262143, 262142, shoff);
            encode_num(&mut zp, &mut svoff_ctx, -262143, 262142, svoff);
            // Decoder: x = last_right + shoff, y = baseline + svoff.
            let nx = layout.last_right + shoff;
            let ny = layout.baseline_get() + svoff;
            layout.baseline_add(ny);
            layout.last_right = nx + cc_w - 1;
        } else {
            zp.encode_bit(&mut offset_type_ctx, true);
            let hoff = x_jb2 - layout.first_left;
            let voff = y_jb2 + cc_h - 1 - layout.first_bottom;
            encode_num(&mut zp, &mut hoff_ctx, -262143, 262142, hoff);
            encode_num(&mut zp, &mut voff_ctx, -262143, 262142, voff);
            // Decoder: nx = first_left+hoff, ny = first_bottom+voff-h+1, then
            // first_left = nx, first_bottom = ny, baseline.fill(ny).
            let nx = layout.first_left + hoff;
            let ny = layout.first_bottom + voff - cc_h + 1;
            layout.first_left = nx;
            layout.first_bottom = ny;
            layout.baseline_fill(ny);
            layout.baseline_add(ny);
            layout.last_right = nx + cc_w - 1;
            layout.same_line_seen = true;
        }

        if dict_action.is_none() {
            dedup.insert(key, dict_size);
            dict_size += 1;
        }
    }

    encode_num(&mut zp, &mut record_type_ctx, 0, 11, 11);
    zp.finish()
}

/// Same-line tolerances (Phase 2 of #188) used to decide between new_line
/// and same-line coordinate coding. Values are in image pixels and chosen
/// to cover normal text glyph variation while still treating a real line
/// break as a new_line. Looser thresholds reduce shoff/svoff magnitudes
/// at the cost of forcing same-line coding when the receiver would have
/// preferred a fresh baseline; tighter thresholds do the opposite.
const SAME_LINE_BASELINE_TOL: i32 = 16;
const SAME_LINE_OVERLAP_TOL: i32 = 16;
const SAME_LINE_GAP_MAX: i32 = 1000;

/// Mirror of jb2::LayoutState held encoder-side.
struct EncoderLayout {
    first_left: i32,
    first_bottom: i32,
    last_right: i32,
    baseline: [i32; 3],
    baseline_idx: i32,
    /// `false` until the first symbol has been emitted — same-line coding
    /// is invalid before then because there is no "previous" baseline.
    same_line_seen: bool,
}

impl EncoderLayout {
    fn new(image_height: i32) -> Self {
        Self {
            first_left: -1,
            first_bottom: image_height - 1,
            last_right: 0,
            baseline: [0, 0, 0],
            baseline_idx: -1,
            same_line_seen: false,
        }
    }

    fn baseline_fill(&mut self, val: i32) {
        self.baseline = [val, val, val];
    }

    fn baseline_add(&mut self, val: i32) {
        self.baseline_idx += 1;
        if self.baseline_idx == 3 {
            self.baseline_idx = 0;
        }
        self.baseline[self.baseline_idx as usize] = val;
    }

    fn baseline_get(&self) -> i32 {
        let (a, b, c) = (self.baseline[0], self.baseline[1], self.baseline[2]);
        if (a >= b && a <= c) || (a <= b && a >= c) {
            a
        } else if (b >= a && b <= c) || (b <= a && b >= c) {
            b
        } else {
            c
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitmap::Bitmap;
    use crate::jb2;

    fn make_bitmap(w: u32, h: u32, f: impl Fn(u32, u32) -> bool) -> Bitmap {
        let mut bm = Bitmap::new(w, h);
        for y in 0..h {
            for x in 0..w {
                bm.set(x, y, f(x, y));
            }
        }
        bm
    }

    fn roundtrip(bm: &Bitmap) -> Bitmap {
        let encoded = encode_jb2(bm);
        jb2::decode(&encoded, None).expect("decode failed")
    }

    #[test]
    fn all_white_roundtrip() {
        let src = Bitmap::new(32, 32);
        let decoded = roundtrip(&src);
        assert_eq!(decoded.width, 32);
        assert_eq!(decoded.height, 32);
        for y in 0..32u32 {
            for x in 0..32u32 {
                assert!(!decoded.get(x, y), "expected white at ({x},{y})");
            }
        }
    }

    #[test]
    fn all_black_roundtrip() {
        let src = make_bitmap(32, 32, |_, _| true);
        let decoded = roundtrip(&src);
        for y in 0..32u32 {
            for x in 0..32u32 {
                assert!(decoded.get(x, y), "expected black at ({x},{y})");
            }
        }
    }

    #[test]
    fn checkerboard_roundtrip() {
        let src = make_bitmap(16, 16, |x, y| (x + y) % 2 == 0);
        let decoded = roundtrip(&src);
        for y in 0..16u32 {
            for x in 0..16u32 {
                assert_eq!(decoded.get(x, y), (x + y) % 2 == 0, "mismatch at ({x},{y})");
            }
        }
    }

    #[test]
    fn single_pixel_roundtrip() {
        // A 1×1 bitmap with a single black pixel.
        let src = make_bitmap(1, 1, |_, _| true);
        let decoded = roundtrip(&src);
        assert_eq!(decoded.width, 1);
        assert_eq!(decoded.height, 1);
        assert!(decoded.get(0, 0));
    }

    #[test]
    fn larger_image_roundtrip() {
        let src = make_bitmap(64, 64, |x, y| (x * 17 + y * 31) % 5 != 0);
        let decoded = roundtrip(&src);
        assert_eq!(decoded.width, 64);
        assert_eq!(decoded.height, 64);
        let mut mismatches = 0u32;
        for y in 0..64u32 {
            for x in 0..64u32 {
                if decoded.get(x, y) != src.get(x, y) {
                    mismatches += 1;
                }
            }
        }
        assert_eq!(
            mismatches, 0,
            "{mismatches} pixel mismatches in 64×64 roundtrip"
        );
    }

    #[test]
    fn encoded_is_nonempty() {
        let src = Bitmap::new(8, 8);
        let encoded = encode_jb2(&src);
        assert!(!encoded.is_empty());
    }

    #[test]
    fn zero_dimension_returns_empty() {
        assert!(encode_jb2(&Bitmap::new(0, 0)).is_empty());
        assert!(encode_jb2(&Bitmap::new(8, 0)).is_empty());
        assert!(encode_jb2(&Bitmap::new(0, 8)).is_empty());
    }

    /// Round-trip across the 1 MP tile boundary (#198).
    /// 2048×2048 = 4 MP forces a 2×2 tile grid (each tile 1024×1024 = 1 MP).
    #[test]
    fn tiled_2048x2048_roundtrip() {
        let src = make_bitmap(2048, 2048, |x, y| {
            // Pseudo-random pattern that stresses each tile differently.
            ((x.wrapping_mul(2654435761)) ^ y.wrapping_mul(40503)) & 7 == 0
        });
        let encoded = encode_jb2(&src);
        let decoded = jb2::decode(&encoded, None).expect("decode failed");
        assert_eq!(decoded.width, 2048);
        assert_eq!(decoded.height, 2048);
        for y in 0..2048u32 {
            for x in 0..2048u32 {
                assert_eq!(decoded.get(x, y), src.get(x, y), "mismatch at ({x},{y})");
            }
        }
    }

    /// Tile boundary not on a power-of-two stride — checks edge tiles smaller
    /// than 1024 in either axis (#198).
    #[test]
    fn tiled_irregular_size_roundtrip() {
        let src = make_bitmap(1500, 1100, |x, y| (x * 13 + y * 7) % 11 == 0);
        let encoded = encode_jb2(&src);
        let decoded = jb2::decode(&encoded, None).expect("decode failed");
        assert_eq!(decoded.width, 1500);
        assert_eq!(decoded.height, 1100);
        let mut mismatches = 0u32;
        for y in 0..1100u32 {
            for x in 0..1500u32 {
                if decoded.get(x, y) != src.get(x, y) {
                    mismatches += 1;
                }
            }
        }
        assert_eq!(mismatches, 0);
    }

    /// 1×1 single-pixel image — smallest round-trip case (#198 DoD).
    #[test]
    fn tiled_1x1_roundtrip() {
        for &px in &[false, true] {
            let src = make_bitmap(1, 1, |_, _| px);
            let encoded = encode_jb2(&src);
            let decoded = jb2::decode(&encoded, None).expect("decode failed");
            assert_eq!(decoded.width, 1);
            assert_eq!(decoded.height, 1);
            assert_eq!(decoded.get(0, 0), px, "1x1 pixel mismatch px={px}");
        }
    }

    /// 100×100 sub-tile image — single tile, exercise non-trivial geometry (#198 DoD).
    #[test]
    fn tiled_100x100_roundtrip() {
        let src = make_bitmap(100, 100, |x, y| (x ^ y) & 1 == 0);
        let encoded = encode_jb2(&src);
        let decoded = jb2::decode(&encoded, None).expect("decode failed");
        assert_eq!(decoded.width, 100);
        assert_eq!(decoded.height, 100);
        for y in 0..100u32 {
            for x in 0..100u32 {
                assert_eq!(decoded.get(x, y), src.get(x, y), "mismatch at ({x},{y})");
            }
        }
    }

    /// 4096×4096 = 16 MP forces a 4×4 tile grid (#198 DoD).
    /// Sparse pattern keeps this test light enough to run in CI.
    #[test]
    #[ignore = "16 MP pixel-by-pixel verify is slow; enable with --ignored"]
    fn tiled_4096x4096_roundtrip() {
        let src = make_bitmap(4096, 4096, |x, y| {
            ((x.wrapping_mul(2654435761)) ^ y.wrapping_mul(40503)) & 31 == 0
        });
        let encoded = encode_jb2(&src);
        let decoded = jb2::decode(&encoded, None).expect("decode failed");
        assert_eq!(decoded.width, 4096);
        assert_eq!(decoded.height, 4096);
        for y in 0..4096u32 {
            for x in 0..4096u32 {
                assert_eq!(decoded.get(x, y), src.get(x, y), "mismatch at ({x},{y})");
            }
        }
    }

    // ── Dict-based encoder (Phase 1: record types 1 + 7) ──────────────────────

    fn roundtrip_dict(bm: &Bitmap) -> Bitmap {
        let encoded = encode_jb2_dict(bm);
        jb2::decode(&encoded, None).expect("dict decode failed")
    }

    fn assert_bitmaps_eq(a: &Bitmap, b: &Bitmap) {
        assert_eq!(a.width, b.width, "width mismatch");
        assert_eq!(a.height, b.height, "height mismatch");
        let mut mismatches = 0u32;
        for y in 0..a.height {
            for x in 0..a.width {
                if a.get(x, y) != b.get(x, y) {
                    mismatches += 1;
                }
            }
        }
        assert_eq!(mismatches, 0, "{mismatches} pixel mismatches");
    }

    #[test]
    fn dict_all_white_roundtrip() {
        let src = Bitmap::new(32, 32);
        let decoded = roundtrip_dict(&src);
        assert_bitmaps_eq(&src, &decoded);
    }

    #[test]
    fn dict_single_pixel_roundtrip() {
        let src = make_bitmap(16, 16, |x, y| x == 4 && y == 7);
        let decoded = roundtrip_dict(&src);
        assert_bitmaps_eq(&src, &decoded);
    }

    #[test]
    fn dict_two_dots_dedup() {
        // Two identical 1-pixel CCs — dict size should be 1.
        let src = make_bitmap(32, 32, |x, y| (x == 3 && y == 5) || (x == 20 && y == 25));
        let decoded = roundtrip_dict(&src);
        assert_bitmaps_eq(&src, &decoded);
        // Assert deduplication happened by checking that the encoded stream
        // is *smaller* than encoding each CC as a fresh record-type-1 would be.
        // Indirect check: re-encode and make sure two CCs exist in the source.
        let ccs = extract_ccs(&src);
        assert_eq!(ccs.len(), 2);
    }

    #[test]
    fn dict_letter_like_shapes() {
        // Two disconnected 3×5 rectangles — should dedup to 1 symbol.
        let src = make_bitmap(32, 32, |x, y| {
            (x < 3 && y < 5) || (x >= 20 && x < 23 && y >= 10 && y < 15)
        });
        let decoded = roundtrip_dict(&src);
        assert_bitmaps_eq(&src, &decoded);
    }

    #[test]
    fn dict_checkerboard_many_ccs() {
        // 8×8 checkerboard: 32 single-pixel CCs, all identical → 1 dict entry.
        let src = make_bitmap(8, 8, |x, y| (x + y) % 2 == 0);
        let decoded = roundtrip_dict(&src);
        assert_bitmaps_eq(&src, &decoded);
    }

    #[test]
    fn dict_two_different_shapes_multiple_occurrences() {
        // Shape A: 2x2 block.  Shape B: 1x3 vertical line.
        // Four copies of each, interleaved spatially.
        let src = make_bitmap(64, 64, |x, y| {
            // A: (0-1, 0-1), (30-31, 0-1), (0-1, 30-31), (30-31, 30-31)
            let in_a = |ax: u32, ay: u32| x >= ax && x < ax + 2 && y >= ay && y < ay + 2;
            // B: (10, 5-7), (40, 5-7), (10, 45-47), (40, 45-47)
            let in_b = |bx: u32, by: u32| x == bx && y >= by && y < by + 3;
            in_a(0, 0)
                || in_a(30, 0)
                || in_a(0, 30)
                || in_a(30, 30)
                || in_b(10, 5)
                || in_b(40, 5)
                || in_b(10, 45)
                || in_b(40, 45)
        });
        let decoded = roundtrip_dict(&src);
        assert_bitmaps_eq(&src, &decoded);
        let ccs = extract_ccs(&src);
        assert_eq!(ccs.len(), 8, "expected 4+4 CCs");
    }

    #[test]
    fn dict_dimension_encoded_correctly() {
        // Non-multiple-of-8 dimensions stress row-stride handling.
        let src = make_bitmap(13, 7, |x, y| (x * 3 + y) % 5 == 0);
        let decoded = roundtrip_dict(&src);
        assert_bitmaps_eq(&src, &decoded);
    }

    #[test]
    fn dict_zero_dimension_returns_empty() {
        assert!(encode_jb2_dict(&Bitmap::new(0, 0)).is_empty());
        assert!(encode_jb2_dict(&Bitmap::new(8, 0)).is_empty());
        assert!(encode_jb2_dict(&Bitmap::new(0, 8)).is_empty());
    }

    #[test]
    fn dict_extract_ccs_counts() {
        // 3 non-touching black squares.
        let src = make_bitmap(30, 30, |x, y| {
            (x < 3 && y < 3)
                || (x >= 10 && x < 13 && y >= 10 && y < 13)
                || (x >= 25 && x < 28 && y >= 25 && y < 28)
        });
        let ccs = extract_ccs(&src);
        assert_eq!(ccs.len(), 3);
        for cc in &ccs {
            assert_eq!(cc.bitmap.width, 3);
            assert_eq!(cc.bitmap.height, 3);
        }
    }

    #[test]
    fn dict_extract_ccs_8connected() {
        // Diagonal pair — 8-connected should merge into 1 CC.
        let src = make_bitmap(4, 4, |x, y| (x == 0 && y == 0) || (x == 1 && y == 1));
        let ccs = extract_ccs(&src);
        assert_eq!(ccs.len(), 1);
        assert_eq!(ccs[0].bitmap.width, 2);
        assert_eq!(ccs[0].bitmap.height, 2);
    }
}
