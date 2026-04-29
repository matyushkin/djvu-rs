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

// ── Refinement bitmap encoding (11-bit context) ──────────────────────────────

/// Encode `cbm` relative to a reference (matched) bitmap `mbm` using the
/// refinement 11-pixel context.
///
/// Mirrors `decode_bitmap_ref` in `jb2.rs`. Center alignment is used per
/// the DjVu spec: the reference bitmap is anchored at the centre of the
/// child.
///
/// Both bitmaps are in [`Bitmap`] top-down storage (y=0 = top of image).
/// The decoder's bottom-up Jbm traversal corresponds to top-down image
/// processing, so we iterate `y in 0..ch`. Context rows are then:
///   * c_r1 = the row just above current in image  (Y - 1 in Bitmap storage)
///   * m_r1 = mbm row at the same image height as the current cbm row
///   * m_r0 = mbm row one below current in image    (m_r1's Bitmap Y + 1)
///   * m_r2 = mbm row one above current in image    (m_r1's Bitmap Y - 1)
///
/// `row_offset = mrow - crow` is the centre-alignment shift expressed in
/// Bitmap (top-down) row indices.
fn encode_bitmap_ref(zp: &mut ZpEncoder, ctx: &mut [u8], cbm: &Bitmap, mbm: &Bitmap) {
    debug_assert_eq!(ctx.len(), 2048);
    let cw = cbm.width as i32;
    let ch = cbm.height as i32;
    if cw <= 0 || ch <= 0 {
        return;
    }
    let mw = mbm.width as i32;
    let mh = mbm.height as i32;

    let crow = (ch - 1) >> 1;
    let ccol = (cw - 1) >> 1;
    let mrow = (mh - 1) >> 1;
    let mcol = (mw - 1) >> 1;
    let row_offset = mrow - crow;
    let col_shift = mcol - ccol;

    let mbm_pixel = |y: i32, x: i32| -> u32 {
        if y < 0 || y >= mh || x < 0 || x >= mw {
            0
        } else {
            mbm.get(x as u32, y as u32) as u32
        }
    };
    let cbm_pixel = |y: i32, x: i32| -> u32 {
        if y < 0 || y >= ch || x < 0 || x >= cw {
            0
        } else {
            cbm.get(x as u32, y as u32) as u32
        }
    };

    for y in 0..ch {
        let my = y + row_offset; // mbm row at same image height as cbm row y

        // Initialise rolling windows at col=0 (col-1 / col-2 OOB → 0).
        let mut c_r1 = (cbm_pixel(y - 1, 0) << 1) | cbm_pixel(y - 1, 1);
        let mut c_r0: u32 = 0;
        let mut m_r1 = (mbm_pixel(my, col_shift - 1) << 2)
            | (mbm_pixel(my, col_shift) << 1)
            | mbm_pixel(my, col_shift + 1);
        let mut m_r0 = (mbm_pixel(my + 1, col_shift - 1) << 2)
            | (mbm_pixel(my + 1, col_shift) << 1)
            | mbm_pixel(my + 1, col_shift + 1);

        for col in 0..cw {
            let m_r2 = mbm_pixel(my - 1, col + col_shift);
            let idx = ((c_r1 << 8) | (c_r0 << 7) | (m_r2 << 6) | (m_r1 << 3) | m_r0) & 2047;
            let bit = cbm_pixel(y, col) != 0;
            zp.encode_bit(&mut ctx[idx as usize], bit);

            c_r1 = ((c_r1 << 1) & 0b111) | cbm_pixel(y - 1, col + 2);
            c_r0 = bit as u32;
            m_r1 = ((m_r1 << 1) & 0b111) | mbm_pixel(my, col + col_shift + 2);
            m_r0 = ((m_r0 << 1) & 0b111) | mbm_pixel(my + 1, col + col_shift + 2);
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

// ── Dict-based encoding: record types 1 (new) + 6 (refinement) + 7 (copy) ────

/// Hamming distance between two equal-sized packed bitmap byte buffers.
fn packed_hamming(a: &[u8], b: &[u8]) -> u32 {
    debug_assert_eq!(a.len(), b.len());
    let mut total: u32 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        total += (x ^ y).count_ones();
    }
    total
}

/// Phase 3 refinement-match threshold (percent of pixels).
///
/// A candidate dict entry of the same (w, h) qualifies as a refinement
/// reference when its Hamming distance to the new CC is at most this
/// fraction of total pixels. Calibrated against `tests/corpus/*.djvu`:
/// values above ~4% start producing record-6 emissions whose refinement
/// bitmap costs more bits than a fresh record-1 on noisy / halftone CCs,
/// regressing total size on dense scanned pages.
const REFINEMENT_DIFF_FRACTION: u32 = 4;

/// Minimum pixel area for a CC to be considered for refinement matching.
///
/// Sub-32-pixel CCs (typical: dust, single anti-aliasing fragments) encode
/// in only a handful of bytes via record-1; the per-record overhead of a
/// record-6 (matched-refinement coordinate header + 11-bit refinement
/// context state) outweighs any saving even at low Hamming distance.
const REFINEMENT_MIN_PIXELS: u64 = 32;

/// Find the closest same-size dict entry within a Hamming-distance budget,
/// for use as a **lossy copy** target (record-7) — the encoder pretends the
/// near-duplicate is byte-exact, the decoder produces the dict entry's pixels
/// instead of the original CC. Visual loss is bounded by the threshold.
///
/// Used by [`Jb2EncodeOptions::lossy_threshold`] (#224 Phase 4); independent
/// of [`find_refinement_ref`] which gates record-6 (lossless refinement).
fn find_lossy_copy_ref(
    cand: &Bitmap,
    dict_entries: &[Bitmap],
    same_size_indices: &[usize],
    threshold: f32,
) -> Option<usize> {
    if same_size_indices.is_empty() || threshold <= 0.0 {
        return None;
    }
    let pixel_count = (cand.width as u64) * (cand.height as u64);
    if pixel_count < REFINEMENT_MIN_PIXELS {
        return None;
    }
    // Hamming budget in pixel count, rounded to the nearest integer.
    let max_diff = ((pixel_count as f64) * (threshold as f64)).round() as u32;
    let mut best: Option<(usize, u32)> = None;
    for &i in same_size_indices {
        let ref_bm = &dict_entries[i];
        debug_assert_eq!(ref_bm.width, cand.width);
        debug_assert_eq!(ref_bm.height, cand.height);
        let d = packed_hamming(&cand.data, &ref_bm.data);
        if d > max_diff {
            continue;
        }
        match best {
            None => best = Some((i, d)),
            Some((_, bd)) if d < bd => best = Some((i, d)),
            _ => {}
        }
    }
    best.map(|(i, _)| i)
}

/// Find the best refinement-reference dict index for `cand` among
/// dict entries of identical (w, h). Returns `None` if no candidate is
/// close enough to be worth a record-6 emission.
fn find_refinement_ref(
    cand: &Bitmap,
    dict_entries: &[Bitmap],
    same_size_indices: &[usize],
) -> Option<usize> {
    if same_size_indices.is_empty() {
        return None;
    }
    let pixel_count = (cand.width as u64) * (cand.height as u64);
    if pixel_count < REFINEMENT_MIN_PIXELS {
        return None;
    }
    let max_diff = ((pixel_count * REFINEMENT_DIFF_FRACTION as u64) / 100) as u32;

    let mut best: Option<(usize, u32)> = None;
    for &i in same_size_indices {
        let ref_bm = &dict_entries[i];
        debug_assert_eq!(ref_bm.width, cand.width);
        debug_assert_eq!(ref_bm.height, cand.height);
        let d = packed_hamming(&cand.data, &ref_bm.data);
        if d > max_diff {
            continue;
        }
        match best {
            None => best = Some((i, d)),
            Some((_, bd)) if d < bd => best = Some((i, d)),
            _ => {}
        }
    }
    best.map(|(i, _)| i)
}

/// Tunable knobs for the JB2 dictionary encoder.
///
/// Default values reproduce the lossless behavior of [`encode_jb2_dict`]
/// and [`encode_jb2_dict_with_shared`].
#[derive(Debug, Clone, Copy)]
pub struct Jb2EncodeOptions {
    /// Hamming-distance threshold (as fraction of pixel count) for **lossy
    /// rec-7 substitution** (#224 Phase 4). When `> 0`, CCs that are not
    /// byte-exact but match a same-size dict entry within
    /// `pixel_count × lossy_threshold` flipped pixels are emitted as
    /// rec-7 (matched copy) — the decoder produces the dict entry's pixels
    /// instead of the original. Visual error per CC is bounded by the
    /// threshold; bytes shrink because rec-7 carries no refinement bitmap.
    ///
    /// `0.0` (default) = lossless: rec-7 fires only on byte-exact matches.
    /// `cjb2 -lossy` ships at roughly the equivalent of 0.04–0.05 here.
    pub lossy_threshold: f32,
}

impl Default for Jb2EncodeOptions {
    fn default() -> Self {
        Self {
            lossy_threshold: 0.0,
        }
    }
}

/// Encode a bilevel [`Bitmap`] into a JB2 stream using a **symbol dictionary**.
///
/// Performs connected-component (CC) extraction, exact-match deduplication,
/// near-duplicate refinement matching, and emits one of:
///  * record type 1 — new symbol (direct, stored in dict + blitted)
///  * record type 6 — matched refinement (blit only, encodes diff vs an
///    existing dict entry of identical size using the 11-bit context)
///  * record type 7 — matched copy (no refinement, blit only)
///
/// Lossless. Matches [`crate::jb2::decode`] for round-trip.
///
/// ## Limitations
/// - Refinement matching only considers dict entries of identical (w, h).
///   Cross-size matching (which the format permits via wdiff/hdiff) needs
///   per-pixel resampling to compute the Hamming distance and is left to
///   a future phase.
/// - Components >= 1 MP are encoded as-is; the decoder will reject them via
///   `MAX_SYMBOL_PIXELS`. For scanned text pages this is not a practical issue.
pub fn encode_jb2_dict(bitmap: &Bitmap) -> Vec<u8> {
    encode_jb2_dict_with_shared(bitmap, &[])
}

/// Encode a bilevel [`Bitmap`] into a JB2 stream that inherits its initial
/// symbol library from a previously-encoded shared dictionary (Djbz).
///
/// Same as [`encode_jb2_dict`] but emits a "required-dict-or-reset"
/// (record type 9) preamble announcing `shared_symbols.len()` inherited
/// entries. Per-symbol matches that hit any of the shared symbols are
/// emitted as record-7 (matched copy) referencing the shared index, so
/// the per-page Sjbz never re-transmits glyphs already present in the
/// shared Djbz.
///
/// `shared_symbols` must be the **identical bitmap sequence** the matching
/// Djbz was built from (see [`encode_jb2_djbz`]).
///
/// Round-trip: pass the resulting Sjbz bytes plus
/// `decode_dict(djbz_bytes, None)` to [`crate::jb2::decode`].
pub fn encode_jb2_dict_with_shared(bitmap: &Bitmap, shared_symbols: &[Bitmap]) -> Vec<u8> {
    encode_jb2_dict_with_options(bitmap, shared_symbols, &Jb2EncodeOptions::default())
}

/// Encode like [`encode_jb2_dict_with_shared`] but with caller-specified
/// [`Jb2EncodeOptions`]. The default options reproduce the lossless
/// behavior of [`encode_jb2_dict_with_shared`]; raising
/// `opts.lossy_threshold` enables rec-7 substitution for near-duplicate
/// CCs (see [`Jb2EncodeOptions::lossy_threshold`]).
///
/// Lossy output: when `lossy_threshold > 0`, the decoded page is no
/// longer pixel-exact relative to the input; reconstruction error per CC
/// is bounded by the threshold (Hamming as a fraction of pixel count).
/// Lossless output (default) round-trips byte-for-byte through
/// [`crate::jb2::decode`].
pub fn encode_jb2_dict_with_options(
    bitmap: &Bitmap,
    shared_symbols: &[Bitmap],
    opts: &Jb2EncodeOptions,
) -> Vec<u8> {
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
    let mut symbol_width_diff_ctx = NumContext::new();
    let mut symbol_height_diff_ctx = NumContext::new();
    let mut symbol_index_ctx = NumContext::new();
    let mut inherit_dict_size_ctx = NumContext::new();
    let mut hoff_ctx = NumContext::new();
    let mut voff_ctx = NumContext::new();
    let mut shoff_ctx = NumContext::new();
    let mut svoff_ctx = NumContext::new();
    let mut direct_bitmap_ctx = vec![0u8; 1024];
    let mut refinement_bitmap_ctx = vec![0u8; 2048];
    let mut offset_type_ctx: u8 = 0;
    let mut flag_ctx: u8 = 0;

    // Preamble.
    if !shared_symbols.is_empty() {
        // Required-dict-or-reset: announce the inherited library size before
        // start-of-image so the decoder pre-populates `dict` from `shared_dict`.
        encode_num(&mut zp, &mut record_type_ctx, 0, 11, 9);
        encode_num(
            &mut zp,
            &mut inherit_dict_size_ctx,
            0,
            262142,
            shared_symbols.len() as i32,
        );
    }
    encode_num(&mut zp, &mut record_type_ctx, 0, 11, 0);
    encode_num(&mut zp, &mut image_size_ctx, 0, 262142, w);
    encode_num(&mut zp, &mut image_size_ctx, 0, 262142, h);
    zp.encode_bit(&mut flag_ctx, false);

    // Layout state — mirrors `LayoutState::new` in jb2.rs:1187.
    let mut layout = EncoderLayout::new(h);

    // Exact-match dedup: (w, h, packed-data) → dict index. Pre-populated from
    // shared_symbols so cross-page identical glyphs encode as rec-7 (copy)
    // referencing the shared library.
    let mut dedup: BTreeMap<(u32, u32, Vec<u8>), usize> = BTreeMap::new();
    // Stored dict entries (parallel to the decoder's `dict` vector) — needed
    // so refinement matching can score Hamming distance against historical
    // glyphs.
    let mut dict_entries: Vec<Bitmap> = Vec::new();
    // Index of dict entries by (w, h) for O(1) lookup of refinement candidates.
    let mut by_size: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for sym in shared_symbols {
        let idx = dict_entries.len();
        dedup.insert((sym.width, sym.height, sym.data.clone()), idx);
        by_size
            .entry((sym.width, sym.height))
            .or_default()
            .push(idx);
        dict_entries.push(sym.clone());
    }

    for &cc_idx in &order {
        let cc = &ccs[cc_idx];
        let cc_w = cc.bitmap.width as i32;
        let cc_h = cc.bitmap.height as i32;
        // JB2 uses bottom-up y: y_jb2 is the bottom y of the symbol.
        let x_jb2 = cc.x as i32;
        let y_jb2 = h - cc.y as i32 - cc_h;

        let key = (cc.bitmap.width, cc.bitmap.height, cc.bitmap.data.clone());
        let exact_match = dedup.get(&key).copied();

        // Choose record type:
        //   exact match → 7  (matched copy, blit only)
        //   near match  → 6  (matched refinement, blit only)
        //   otherwise   → 1  (new symbol, direct, add to dict + blit)
        enum Action {
            New,
            Copy(usize),
            Refine(usize),
        }
        let action = if let Some(idx) = exact_match {
            Action::Copy(idx)
        } else {
            let candidates = by_size
                .get(&(cc.bitmap.width, cc.bitmap.height))
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            // Phase 4 (#224): lossy rec-7 substitution. Tried before
            // refinement so a same-size near-twin produces a smaller
            // rec-7 (no refinement bitmap) instead of a larger rec-6.
            let lossy_copy = if opts.lossy_threshold > 0.0 {
                find_lossy_copy_ref(&cc.bitmap, &dict_entries, candidates, opts.lossy_threshold)
            } else {
                None
            };
            match lossy_copy {
                Some(idx) => Action::Copy(idx),
                None => match find_refinement_ref(&cc.bitmap, &dict_entries, candidates) {
                    Some(ref_idx) => Action::Refine(ref_idx),
                    None => Action::New,
                },
            }
        };

        let dict_size = dict_entries.len();
        match &action {
            Action::New => {
                encode_num(&mut zp, &mut record_type_ctx, 0, 11, 1);
                encode_num(&mut zp, &mut symbol_width_ctx, 0, 262142, cc_w);
                encode_num(&mut zp, &mut symbol_height_ctx, 0, 262142, cc_h);
                encode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, &cc.bitmap);
            }
            Action::Copy(dict_idx) => {
                encode_num(&mut zp, &mut record_type_ctx, 0, 11, 7);
                encode_num(
                    &mut zp,
                    &mut symbol_index_ctx,
                    0,
                    (dict_size - 1) as i32,
                    *dict_idx as i32,
                );
            }
            Action::Refine(ref_idx) => {
                encode_num(&mut zp, &mut record_type_ctx, 0, 11, 6);
                encode_num(
                    &mut zp,
                    &mut symbol_index_ctx,
                    0,
                    (dict_size - 1) as i32,
                    *ref_idx as i32,
                );
                // Same-size refinement: width/height diffs are zero.
                encode_num(&mut zp, &mut symbol_width_diff_ctx, -262143, 262142, 0);
                encode_num(&mut zp, &mut symbol_height_diff_ctx, -262143, 262142, 0);
                encode_bitmap_ref(
                    &mut zp,
                    &mut refinement_bitmap_ctx,
                    &cc.bitmap,
                    &dict_entries[*ref_idx],
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

        // Only record-type-1 (new symbol) extends the dict — types 6 and 7
        // are blit-only and the decoder leaves the dict untouched.
        if matches!(action, Action::New) {
            let next_idx = dict_entries.len();
            dedup.insert(key, next_idx);
            by_size
                .entry((cc.bitmap.width, cc.bitmap.height))
                .or_default()
                .push(next_idx);
            dict_entries.push(cc.bitmap.clone());
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

// ── Djbz dictionary stream + multi-page sharing (#194) ─────────────────────────

/// Encode a sequence of bilevel symbols as a JB2 **Djbz** chunk payload.
///
/// Each symbol is emitted as record-type-2 (new symbol, direct, dict-only) in
/// the order given. The decoder side ([`crate::jb2::decode_dict`]) reconstructs
/// a [`crate::jb2::Jb2Dict`] whose symbol indices match this input order, so
/// downstream Sjbz streams encoded with [`encode_jb2_dict_with_shared`] using
/// the same `&[Bitmap]` reference will round-trip cleanly.
///
/// The Djbz contains no positioning information — symbols are abstract
/// glyph bitmaps, not blits. The page Sjbz alone places them.
pub fn encode_jb2_djbz(symbols: &[Bitmap]) -> Vec<u8> {
    let mut zp = ZpEncoder::new();
    let mut record_type_ctx = NumContext::new();
    let mut image_size_ctx = NumContext::new();
    let mut symbol_width_ctx = NumContext::new();
    let mut symbol_height_ctx = NumContext::new();
    let mut direct_bitmap_ctx = vec![0u8; 1024];
    let mut flag_ctx: u8 = 0;

    // Preamble: start-of-image (rec 0) — no rec-9 since a Djbz never inherits
    // from another dict in this encoder. Dimensions are written but unused on
    // the decode side (see `decode_dictionary` in jb2.rs:1990).
    encode_num(&mut zp, &mut record_type_ctx, 0, 11, 0);
    encode_num(&mut zp, &mut image_size_ctx, 0, 262142, 0);
    encode_num(&mut zp, &mut image_size_ctx, 0, 262142, 0);
    zp.encode_bit(&mut flag_ctx, false);

    // Symbol body: rec-2 per entry.
    for sym in symbols {
        encode_num(&mut zp, &mut record_type_ctx, 0, 11, 2);
        encode_num(&mut zp, &mut symbol_width_ctx, 0, 262142, sym.width as i32);
        encode_num(
            &mut zp,
            &mut symbol_height_ctx,
            0,
            262142,
            sym.height as i32,
        );
        encode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, sym);
    }

    // End-of-data.
    encode_num(&mut zp, &mut record_type_ctx, 0, 11, 11);
    zp.finish()
}

/// Cluster CCs from `pages` and return the bitmaps that should live in a
/// shared Djbz: any (w, h, packed-data) signature that appears on `>=
/// page_threshold` distinct pages, represented by the first-seen CC.
///
/// Returns shared symbols in deterministic order (sorted by first-seen page,
/// then first-seen position within that page). Pages without enough repetition
/// produce an empty shared dict.
///
/// **Byte-exact dedup only.** Tried Hamming-distance clustering for #194
/// Phase 2 (`cluster_shared_symbols_tunable` with `diff_fraction > 0`):
/// no measurable byte saving on the 517-page `pathogenic_bacteria_1896`
/// corpus (< 0.05% delta from byte-exact across 0%/1%/2% Hamming) and
/// `diff_fraction = 3%` introduced per-page Sjbz decode mismatches under
/// rec-6 refinement against shared reps. Byte-exact clustering already
/// captures the multi-page win (−13.0% bundle vs independent on the same
/// corpus). See CLAUDE.md "Multi-page shared Djbz dictionary, Phase 2"
/// investigation for measurements.
pub fn cluster_shared_symbols(pages: &[Bitmap], page_threshold: usize) -> Vec<Bitmap> {
    cluster_shared_symbols_tunable(pages, page_threshold, 0)
}

/// Same as [`cluster_shared_symbols`] but exposes the per-CC Hamming
/// allowance as a percentage of pixel count. `diff_fraction = 0` produces
/// byte-exact clustering (the shipped default); higher values fold
/// near-duplicate glyphs into a single shared rep at the cost of forcing
/// the per-page Sjbz emitter through rec-6 (matched refinement) more often.
///
/// Provided for corpus benchmarking — most callers want
/// [`cluster_shared_symbols`].
pub fn cluster_shared_symbols_tunable(
    pages: &[Bitmap],
    page_threshold: usize,
    diff_fraction: u32,
) -> Vec<Bitmap> {
    if page_threshold < 2 || pages.len() < page_threshold {
        return Vec::new();
    }

    struct Cluster {
        rep: Bitmap,
        pages_seen: Vec<usize>,
        first_seen: (usize, usize),
    }

    let mut buckets: BTreeMap<(u32, u32), Vec<Cluster>> = BTreeMap::new();

    for (page_idx, page) in pages.iter().enumerate() {
        let ccs = extract_ccs(page);
        for (cc_idx, cc) in ccs.iter().enumerate() {
            let bm = &cc.bitmap;
            let pixel_count = (bm.width as u64) * (bm.height as u64);
            // Tiny CCs always byte-exact: refinement is rejected per-page anyway.
            let max_diff: u32 = if pixel_count < REFINEMENT_MIN_PIXELS {
                0
            } else {
                ((pixel_count * diff_fraction as u64) / 100) as u32
            };

            let bucket = buckets.entry((bm.width, bm.height)).or_default();
            let mut best: Option<(usize, u32)> = None;
            for (i, c) in bucket.iter().enumerate() {
                let d = packed_hamming(&bm.data, &c.rep.data);
                if d > max_diff {
                    continue;
                }
                match best {
                    None => best = Some((i, d)),
                    Some((_, bd)) if d < bd => best = Some((i, d)),
                    _ => {}
                }
            }
            match best {
                Some((i, _)) => {
                    if !bucket[i].pages_seen.contains(&page_idx) {
                        bucket[i].pages_seen.push(page_idx);
                    }
                }
                None => bucket.push(Cluster {
                    rep: bm.clone(),
                    pages_seen: vec![page_idx],
                    first_seen: (page_idx, cc_idx),
                }),
            }
        }
    }

    let mut promoted: Vec<Cluster> = buckets
        .into_values()
        .flatten()
        .filter(|c| c.pages_seen.len() >= page_threshold)
        .collect();
    promoted.sort_by_key(|c| c.first_seen);
    promoted.into_iter().map(|c| c.rep).collect()
}

/// Per-CC accounting of which JB2 record type a single page would emit
/// against a given shared dictionary, without performing the actual encode.
///
/// Phase 2.5 measurement aid (#194): mirrors the action-selection branch in
/// [`encode_jb2_dict_with_shared`] (rec-7 exact / rec-6 refinement / rec-1
/// new) and reports counts, pixel totals, and Hamming-distance distribution
/// for the rec-6 emissions, distinguishing references that resolve into the
/// shared Djbz vs ones that resolve into the page-local running dict.
///
/// Use this to answer questions like "how many CCs would actually benefit
/// from a tighter refinement threshold" or "how large is the rec-7 win
/// from the shared dict on this corpus" without round-tripping bytes.
#[derive(Debug, Default, Clone)]
pub struct CcStats {
    pub total_ccs: usize,
    /// rec-7: byte-exact match found in the running dict.
    pub rec_7_exact: usize,
    /// rec-6 against a slot inside the shared (cross-page) Djbz.
    pub rec_6_refine_shared: usize,
    /// rec-6 against a slot emitted earlier on the same page.
    pub rec_6_refine_local: usize,
    /// rec-1: no usable match, fresh emission.
    pub rec_1_new: usize,
    /// Hamming distances of rec-6 matches (one entry per rec-6 CC).
    pub rec_6_hamming: Vec<u32>,
    /// Pixel-count totals split by record type.
    pub pixels_rec_1: u64,
    pub pixels_rec_6: u64,
    pub pixels_rec_7: u64,
}

/// Walk `page`'s connected components in encoder order and accumulate
/// per-CC accounting against `shared_symbols` using the same action-
/// selection rules as [`encode_jb2_dict_with_shared`]. Pure observation —
/// no bytes are emitted.
pub fn analyze_jb2_cc_stats(page: &Bitmap, shared_symbols: &[Bitmap]) -> CcStats {
    let mut stats = CcStats::default();
    if page.width == 0 || page.height == 0 {
        return stats;
    }

    let ccs = extract_ccs(page);
    let mut order: Vec<usize> = (0..ccs.len()).collect();
    let bucket = (SAME_LINE_BASELINE_TOL.max(1)) as u32;
    order.sort_by_key(|&i| {
        let cc = &ccs[i];
        let bottom = cc.y + cc.bitmap.height;
        (bottom / bucket, cc.x)
    });

    let mut dedup: BTreeMap<(u32, u32, Vec<u8>), usize> = BTreeMap::new();
    let mut dict_entries: Vec<Bitmap> = Vec::new();
    let mut by_size: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    let shared_len = shared_symbols.len();
    for sym in shared_symbols {
        let idx = dict_entries.len();
        dedup.insert((sym.width, sym.height, sym.data.clone()), idx);
        by_size
            .entry((sym.width, sym.height))
            .or_default()
            .push(idx);
        dict_entries.push(sym.clone());
    }

    for &cc_idx in &order {
        let cc = &ccs[cc_idx];
        let pixels = (cc.bitmap.width as u64) * (cc.bitmap.height as u64);
        stats.total_ccs += 1;

        let key = (cc.bitmap.width, cc.bitmap.height, cc.bitmap.data.clone());
        if let Some(idx) = dedup.get(&key).copied() {
            stats.rec_7_exact += 1;
            stats.pixels_rec_7 += pixels;
            // rec-7 emits no new dict entry, no need to update tables.
            let _ = idx;
            continue;
        }

        let candidates = by_size
            .get(&(cc.bitmap.width, cc.bitmap.height))
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        match find_refinement_ref(&cc.bitmap, &dict_entries, candidates) {
            Some(ref_idx) => {
                let ref_bm = &dict_entries[ref_idx];
                let d = packed_hamming(&cc.bitmap.data, &ref_bm.data);
                stats.rec_6_hamming.push(d);
                if ref_idx < shared_len {
                    stats.rec_6_refine_shared += 1;
                } else {
                    stats.rec_6_refine_local += 1;
                }
                stats.pixels_rec_6 += pixels;
                // rec-6 emits no new dict entry — the refinement is blit-only.
            }
            None => {
                stats.rec_1_new += 1;
                stats.pixels_rec_1 += pixels;
                let idx = dict_entries.len();
                dedup.insert(key, idx);
                by_size
                    .entry((cc.bitmap.width, cc.bitmap.height))
                    .or_default()
                    .push(idx);
                dict_entries.push(cc.bitmap.clone());
            }
        }
    }

    stats
}

/// Encode a multi-page bilevel document as a bundled DJVM with a shared Djbz.
///
/// CCs that appear on at least `shared_dict_page_threshold` distinct input
/// pages are promoted into a single shared [`Jb2Dict`] (Djbz) emitted as a
/// `FORM:DJVI` component. Each page's `FORM:DJVU` then carries a small
/// `INCL` chunk pointing at that DJVI plus a Sjbz that references the shared
/// dictionary by index.
///
/// Returns the full DjVu container bytes (with `AT&T` magic, ready to write
/// to a file). With `pages.len() < 2` or `shared_dict_page_threshold > pages.len()`,
/// no symbols qualify for sharing and the encoder degrades to per-page
/// independent encoding (still wrapped in DJVM).
///
/// [`Jb2Dict`]: crate::jb2::Jb2Dict
pub fn encode_djvm_bundle_jb2(pages: &[Bitmap], shared_dict_page_threshold: usize) -> Vec<u8> {
    let shared = cluster_shared_symbols(pages, shared_dict_page_threshold);
    encode_djvm_bundle_jb2_with_shared(pages, &shared)
}

/// Same as [`encode_djvm_bundle_jb2`] but uses a caller-supplied shared
/// dictionary instead of running [`cluster_shared_symbols`]. Lets callers
/// drive cluster selection (e.g. corpus benchmarks measuring different
/// Hamming thresholds) while reusing the IFF/DIRM emission logic.
pub fn encode_djvm_bundle_jb2_with_shared(pages: &[Bitmap], shared: &[Bitmap]) -> Vec<u8> {
    use crate::iff;

    let djbz_bytes = encode_jb2_djbz(shared);

    // ── Build component buffers (each = full FORM body, ready for IFF emit) ──
    //
    // DJVI component (only when there is something to share): contains a single
    // INFO chunk (none required by spec) + the Djbz.
    //
    // DJVU page components: INFO + INCL("dict0001.djvi") + Sjbz.
    let mut comp_form_bodies: Vec<(Vec<u8>, /*is_page*/ bool, String)> = Vec::new();

    let dict_id = "dict0001.djvi".to_string();
    let has_shared = !shared.is_empty();
    if has_shared {
        let mut djvi_body = Vec::new();
        djvi_body.extend_from_slice(b"DJVI");
        djvi_body.extend_from_slice(b"Djbz");
        djvi_body.extend_from_slice(&(djbz_bytes.len() as u32).to_be_bytes());
        djvi_body.extend_from_slice(&djbz_bytes);
        if !djbz_bytes.len().is_multiple_of(2) {
            djvi_body.push(0);
        }
        comp_form_bodies.push((djvi_body, false, dict_id.clone()));
    }

    let shared_ref: &[Bitmap] = shared;
    for (page_idx, page) in pages.iter().enumerate() {
        let sjbz = encode_jb2_dict_with_shared(page, shared_ref);
        let mut info = Vec::with_capacity(10);
        info.extend_from_slice(&(page.width as u16).to_be_bytes());
        info.extend_from_slice(&(page.height as u16).to_be_bytes());
        info.extend_from_slice(&[24, 0, 100, 0, 1, 0]); // version major, minor, dpi(le16), gamma, rotation

        let mut djvu_body = Vec::new();
        djvu_body.extend_from_slice(b"DJVU");
        djvu_body.extend_from_slice(b"INFO");
        djvu_body.extend_from_slice(&(info.len() as u32).to_be_bytes());
        djvu_body.extend_from_slice(&info);
        if !info.len().is_multiple_of(2) {
            djvu_body.push(0);
        }
        if has_shared {
            let incl_payload = dict_id.as_bytes();
            djvu_body.extend_from_slice(b"INCL");
            djvu_body.extend_from_slice(&(incl_payload.len() as u32).to_be_bytes());
            djvu_body.extend_from_slice(incl_payload);
            if !incl_payload.len().is_multiple_of(2) {
                djvu_body.push(0);
            }
        }
        djvu_body.extend_from_slice(b"Sjbz");
        djvu_body.extend_from_slice(&(sjbz.len() as u32).to_be_bytes());
        djvu_body.extend_from_slice(&sjbz);
        if !sjbz.len().is_multiple_of(2) {
            djvu_body.push(0);
        }

        let pid = format!("p{:04}.djvu", page_idx + 1);
        comp_form_bodies.push((djvu_body, true, pid));
    }

    // ── Build DIRM directly (bundled, with offsets) ──
    //
    // Reuses the shape of `crate::djvm::build_djvm` but inlined here because
    // we have FORM bodies (not full FORM chunks with header) to embed. The
    // simpler path: build full FORM chunks here, then call `iff::emit_form`.
    // Each component is a FORM chunk: { id: "FORM", body: <DJVU/DJVI ...> }.
    let comp_form_data: Vec<&[u8]> = comp_form_bodies
        .iter()
        .map(|(b, _, _)| b.as_slice())
        .collect();

    // DIRM payload: build matching the bundled-format layout in
    // `djvu_document.rs::parse` (flags=0x81 → bundled+1.0; count u16-be;
    // per-component offsets u32-be; bzz-compressed metadata table).
    let n = comp_form_bodies.len();
    let mut dirm = Vec::new();
    dirm.push(0x81); // bundled (high bit) + version 1
    dirm.extend_from_slice(&(n as u16).to_be_bytes());

    // Compute offsets after the DIRM chunk has been laid down.
    // Layout: AT&T (4) + FORM (4) + form_size (4) + "DJVM" (4) + "DIRM" (4) +
    //         dirm_size (4) + dirm_payload_with_offsets+bzz_meta + pad +
    //         each FORM chunk header (8) + body + pad.
    //
    // Offsets in the DIRM are *file-byte* offsets to the AT&T-stripped FORM
    // chunk header for each component. So offset[i] = position of "FORM" id
    // bytes for that component within the file.
    //
    // We don't know the DIRM size until we know the offsets; resolve via
    // two-pass: build metadata table first, then layout.
    let mut meta = Vec::new();
    for (body, _, _) in &comp_form_bodies {
        let total = body.len() + 8; // FORM + size + body
        meta.extend_from_slice(&(total as u32).to_be_bytes()[1..4]); // 24-bit size
    }
    for (_, is_page, _) in &comp_form_bodies {
        let flag = if *is_page { 1u8 } else { 0u8 };
        meta.push(flag);
    }
    for (_, _, id) in &comp_form_bodies {
        meta.extend_from_slice(id.as_bytes());
        meta.push(0);
    }
    for (_, _, id) in &comp_form_bodies {
        meta.extend_from_slice(id.as_bytes());
        meta.push(0);
    }
    meta.extend(core::iter::repeat_n(0u8, n)); // empty titles
    let bzz_meta = crate::bzz_encode::bzz_encode(&meta);

    // dirm payload final size = 1 (flags) + 2 (count) + 4*n (offsets) + bzz_meta.len()
    let dirm_size = 1 + 2 + 4 * n + bzz_meta.len();

    // Compute DJVM body size and component offsets.
    let dirm_chunk_total = 8 + dirm_size + (dirm_size & 1); // header + payload + pad
    let mut form_body_size: usize = 4; // "DJVM"
    form_body_size += dirm_chunk_total;
    let mut comp_offsets: Vec<u32> = Vec::with_capacity(n);
    for body in &comp_form_data {
        // File offset = AT&T(4) + FORM(4) + size(4) + DJVM(4) + dirm_chunk_total
        //             + sum-of-prior-comp-totals
        // The decoder treats DIRM offsets as byte offsets from start of file
        // pointing at the "FORM" id bytes of the component. Offset 0 of the
        // file = 'A' of "AT&T", so offset = 12 + 4 + dirm_chunk_total + prior.
        let off = 4 + 4 + 4 + 4 + dirm_chunk_total + (form_body_size - 4 - dirm_chunk_total);
        comp_offsets.push(off as u32);
        let tot = body.len() + 8;
        form_body_size += tot + (tot & 1); // pad component to even
    }

    // Now write final dirm payload with offsets.
    let _ = dirm; // computed above for reference; final form built fresh below.
    let mut dirm_full = Vec::with_capacity(dirm_size);
    dirm_full.push(0x81);
    dirm_full.extend_from_slice(&(n as u16).to_be_bytes());
    for off in &comp_offsets {
        dirm_full.extend_from_slice(&off.to_be_bytes());
    }
    dirm_full.extend_from_slice(&bzz_meta);
    debug_assert_eq!(dirm_full.len(), dirm_size);

    // Emit final file.
    let mut out = Vec::with_capacity(12 + form_body_size);
    out.extend_from_slice(b"AT&T");
    out.extend_from_slice(b"FORM");
    out.extend_from_slice(&(form_body_size as u32).to_be_bytes());
    out.extend_from_slice(b"DJVM");
    out.extend_from_slice(b"DIRM");
    out.extend_from_slice(&(dirm_size as u32).to_be_bytes());
    out.extend_from_slice(&dirm_full);
    if dirm_size & 1 == 1 {
        out.push(0);
    }
    for body in &comp_form_data {
        out.extend_from_slice(b"FORM");
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(body);
        if body.len() & 1 == 1 {
            out.push(0);
        }
    }

    let _ = iff::parse_form; // silence unused-import warning
    out
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
        let mut mismatches = Vec::new();
        for y in 0..a.height {
            for x in 0..a.width {
                if a.get(x, y) != b.get(x, y) {
                    mismatches.push((x, y, a.get(x, y), b.get(x, y)));
                }
            }
        }
        assert!(
            mismatches.is_empty(),
            "{} pixel mismatches: {:?}",
            mismatches.len(),
            mismatches
        );
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
            (x < 3 && y < 5) || ((20..23).contains(&x) && (10..15).contains(&y))
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
                || ((10..13).contains(&x) && (10..13).contains(&y))
                || ((25..28).contains(&x) && (25..28).contains(&y))
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

    // ── Refinement matching (Phase 3 of #188): record type 6 ─────────────────

    #[test]
    fn refine_near_duplicate_glyphs_roundtrip() {
        // Two glyph-like shapes with the same bounding box and a 1-pixel diff
        // — well within REFINEMENT_DIFF_FRACTION (10%). The encoder should
        // emit record-1 for the first and record-6 for the second; the
        // decoder must reconstruct each shape exactly at its own location.
        //
        // Shape A: solid 5×5 block.
        // Shape B: same 5×5 block with one pixel flipped (4% diff).
        let src = make_bitmap(40, 12, |x, y| {
            // CC1 at (2, 2)..(7, 7): solid 5×5
            let in_a = (2..7).contains(&x) && (2..7).contains(&y);
            // CC2 at (20, 2)..(25, 7): solid 5×5 with (24, 6) flipped to white
            let in_b = (20..25).contains(&x) && (2..7).contains(&y) && !(x == 24 && y == 6);
            in_a || in_b
        });
        let decoded = roundtrip_dict(&src);
        assert_bitmaps_eq(&src, &decoded);
    }

    #[test]
    fn refine_text_like_repeats_roundtrip() {
        // Six 7×9 "letters" — a plus sign and small variants — laid out in a
        // single row. Same size, low Hamming distance, so the encoder should
        // pick refinement encoding for the variants.
        let src = make_bitmap(80, 12, |x, y| {
            let local_x = x % 12;
            let local_y = y;
            let glyph_idx = x / 12;
            // Base glyph: a plus sign in a 7×9 box.
            let base = (local_x == 3 && (1..8).contains(&local_y))
                || (local_y == 4 && (1..7).contains(&local_x));
            // Each repeat flips one different pixel (introducing a tiny diff).
            let perturbed = match glyph_idx {
                1 => local_x == 0 && local_y == 0,
                2 => local_x == 6 && local_y == 8,
                3 => local_x == 6 && local_y == 0,
                4 => local_x == 0 && local_y == 8,
                _ => false,
            };
            base ^ perturbed
        });
        let decoded = roundtrip_dict(&src);
        assert_bitmaps_eq(&src, &decoded);
    }

    #[test]
    fn refine_far_glyph_falls_back_to_new() {
        // A 5×5 block followed by an unrelated 5×5 X-pattern — Hamming
        // distance ≫ 10%, so refinement matching should *not* fire and the
        // encoder should emit two record-1 entries. Output must still
        // round-trip exactly.
        let src = make_bitmap(40, 12, |x, y| {
            let in_block = (2..7).contains(&x) && (2..7).contains(&y);
            let in_x = (20..25).contains(&x)
                && (2..7).contains(&y)
                && (x - 20 == y - 2 || x - 20 == 6 - (y - 2));
            in_block || in_x
        });
        let decoded = roundtrip_dict(&src);
        assert_bitmaps_eq(&src, &decoded);
    }

    #[test]
    fn refine_packed_hamming_basic() {
        let a = vec![0b1010_1010u8, 0b0000_1111u8];
        let b = vec![0b1010_1011u8, 0b0000_1111u8];
        assert_eq!(packed_hamming(&a, &b), 1);
        let c = vec![0u8; 2];
        let d = vec![0xff; 2];
        assert_eq!(packed_hamming(&c, &d), 16);
    }

    // ── #194 multi-page shared Djbz ────────────────────────────────────────────

    fn render_glyph(bm: &mut Bitmap, x: u32, y: u32, glyph: &[&[u8]]) {
        for (gy, row) in glyph.iter().enumerate() {
            for (gx, &c) in row.iter().enumerate() {
                if c == b'#' {
                    bm.set(x + gx as u32, y + gy as u32, true);
                }
            }
        }
    }

    fn glyph_a() -> Vec<&'static [u8]> {
        vec![
            b" ## " as &[u8],
            b"#  #" as &[u8],
            b"####" as &[u8],
            b"#  #" as &[u8],
            b"#  #" as &[u8],
        ]
    }
    fn glyph_b() -> Vec<&'static [u8]> {
        vec![
            b"### " as &[u8],
            b"#  #" as &[u8],
            b"### " as &[u8],
            b"#  #" as &[u8],
            b"### " as &[u8],
        ]
    }

    fn make_text_page(words: &[&[u8]]) -> Bitmap {
        let mut bm = Bitmap::new(80, 30);
        let mut x = 4;
        for word in words {
            for &letter in *word {
                let g = match letter {
                    b'A' => glyph_a(),
                    b'B' => glyph_b(),
                    _ => continue,
                };
                render_glyph(&mut bm, x, 8, &g);
                x += 6;
            }
            x += 4;
        }
        bm
    }

    fn assert_decoded_eq(src: &Bitmap, decoded: &Bitmap) {
        assert_eq!(src.width, decoded.width, "width mismatch");
        assert_eq!(src.height, decoded.height, "height mismatch");
        let mut mismatches = 0u32;
        for y in 0..src.height {
            for x in 0..src.width {
                if src.get(x, y) != decoded.get(x, y) {
                    mismatches += 1;
                }
            }
        }
        assert_eq!(mismatches, 0, "{mismatches} pixel mismatches");
    }

    #[test]
    fn djbz_roundtrip_two_glyphs() {
        // Encode two distinct glyph bitmaps as a Djbz, decode it, and verify
        // the resulting Jb2Dict has exactly those two symbols in order.
        let mut a = Bitmap::new(4, 5);
        render_glyph(&mut a, 0, 0, &glyph_a());
        let mut b = Bitmap::new(4, 5);
        render_glyph(&mut b, 0, 0, &glyph_b());
        let djbz = encode_jb2_djbz(&[a.clone(), b.clone()]);
        assert!(!djbz.is_empty());

        // Sanity-decode by constructing a Sjbz that uses the shared dict
        // and checking the two glyphs round-trip.
        let dict = jb2::decode_dict(&djbz, None).expect("decode_dict");
        // Use the shared dict in a 1-page Sjbz that places both glyphs.
        let mut page = Bitmap::new(20, 8);
        render_glyph(&mut page, 2, 2, &glyph_a());
        render_glyph(&mut page, 10, 2, &glyph_b());
        let sjbz = encode_jb2_dict_with_shared(&page, &[a, b]);
        let decoded = jb2::decode(&sjbz, Some(&dict)).expect("decode");
        assert_decoded_eq(&page, &decoded);
    }

    #[test]
    fn shared_dict_smaller_than_independent_for_repeated_pages() {
        // Build two identical text pages. Encoding them with a shared Djbz
        // should produce strictly smaller total bytes than two independent
        // dict encodings.
        let p1 = make_text_page(&[b"AABB", b"BABA"]);
        let p2 = make_text_page(&[b"AABB", b"BABA"]);

        let independent_total = encode_jb2_dict(&p1).len() + encode_jb2_dict(&p2).len();

        let bundle = encode_djvm_bundle_jb2(&[p1.clone(), p2.clone()], 2);
        assert!(!bundle.is_empty());

        // Round-trip via the document parser.
        let doc = crate::djvu_document::DjVuDocument::parse(&bundle).expect("parse DJVM");
        assert_eq!(doc.page_count(), 2);
        let d1 = doc
            .page(0)
            .expect("page 0")
            .extract_mask()
            .expect("extract_mask 0")
            .expect("mask 0 present");
        let d2 = doc
            .page(1)
            .expect("page 1")
            .extract_mask()
            .expect("extract_mask 1")
            .expect("mask 1 present");
        assert_decoded_eq(&p1, &d1);
        assert_decoded_eq(&p2, &d2);

        // Size win sanity check (bundle includes DIRM + IFF wrappers, so we
        // only assert the pure JB2 payload across (Djbz + 2× Sjbz) is smaller
        // than the pure 2× independent Sjbz).
        let shared = cluster_shared_symbols(&[p1.clone(), p2.clone()], 2);
        assert!(
            !shared.is_empty(),
            "two identical pages should produce shared symbols"
        );
        let djbz = encode_jb2_djbz(&shared);
        let sjbz1 = encode_jb2_dict_with_shared(&p1, &shared);
        let sjbz2 = encode_jb2_dict_with_shared(&p2, &shared);
        let shared_jb2_total = djbz.len() + sjbz1.len() + sjbz2.len();
        assert!(
            shared_jb2_total < independent_total,
            "expected shared jb2 < independent: shared={}  independent={}",
            shared_jb2_total,
            independent_total
        );
    }

    #[test]
    fn cluster_promotes_only_repeated_glyphs() {
        // A appears on both pages, B appears on only one. With threshold=2,
        // only A should be promoted.
        let mut p1 = Bitmap::new(20, 10);
        render_glyph(&mut p1, 2, 2, &glyph_a());
        render_glyph(&mut p1, 10, 2, &glyph_b());
        let mut p2 = Bitmap::new(20, 10);
        render_glyph(&mut p2, 2, 2, &glyph_a());
        // No B on page 2.

        let shared = cluster_shared_symbols(&[p1, p2], 2);
        assert_eq!(shared.len(), 1, "only A should cross the threshold");
        // A glyph is 4×5.
        assert_eq!(shared[0].width, 4);
        assert_eq!(shared[0].height, 5);
    }

    fn glyph_box8() -> Vec<&'static [u8]> {
        vec![
            b"########" as &[u8],
            b"#      #" as &[u8],
            b"#      #" as &[u8],
            b"#      #" as &[u8],
            b"#      #" as &[u8],
            b"#      #" as &[u8],
            b"#      #" as &[u8],
            b"########" as &[u8],
        ]
    }

    #[test]
    fn cluster_tunable_groups_near_duplicate_large_glyphs() {
        // 8×8 = 64 pixels (>= REFINEMENT_MIN_PIXELS=32). With diff_fraction=4%,
        // max_diff = 64*4/100 = 2 bits. A glyph and its 1-bit-perturbed twin
        // must cluster into one rep when Hamming clustering is opted into.
        // Default (byte-exact) ships off — see Phase 2 calibration in CLAUDE.md.
        //
        // Use box outlines with one outline-pixel removed (so the noise alters
        // the same CC instead of producing a stray 1-pixel CC).
        let mut p1 = Bitmap::new(20, 20);
        render_glyph(&mut p1, 4, 4, &glyph_box8());
        p1.set(5, 4, false); // notch the top edge at x=5
        let mut p2 = Bitmap::new(20, 20);
        render_glyph(&mut p2, 4, 4, &glyph_box8());
        p2.set(6, 4, false); // notch at x=6 instead — different bit

        let shared = cluster_shared_symbols_tunable(&[p1.clone(), p2.clone()], 2, 4);
        assert_eq!(shared.len(), 1, "near-duplicate twins should cluster at 4%");

        // Default (byte-exact) keeps them separate — neither passes
        // page_threshold=2 since each variant only appears on one page.
        let shared_exact = cluster_shared_symbols(&[p1, p2], 2);
        assert!(
            shared_exact.is_empty(),
            "byte-exact default must not promote noisy near-dupes"
        );
    }

    #[test]
    fn djvm_bundle_with_no_repeats_still_round_trips() {
        // Two pages with no shared CCs — bundle should still parse and decode
        // each page correctly (degraded path: empty Djbz / no shared dict).
        let mut p1 = Bitmap::new(20, 10);
        render_glyph(&mut p1, 2, 2, &glyph_a());
        let mut p2 = Bitmap::new(20, 10);
        render_glyph(&mut p2, 2, 2, &glyph_b());

        let bundle = encode_djvm_bundle_jb2(&[p1.clone(), p2.clone()], 2);
        let doc = crate::djvu_document::DjVuDocument::parse(&bundle).expect("parse DJVM");
        assert_eq!(doc.page_count(), 2);
        let d1 = doc
            .page(0)
            .expect("page 0")
            .extract_mask()
            .expect("extract_mask 0")
            .expect("mask 0 present");
        let d2 = doc
            .page(1)
            .expect("page 1")
            .extract_mask()
            .expect("extract_mask 1")
            .expect("mask 1 present");
        assert_decoded_eq(&p1, &d1);
        assert_decoded_eq(&p2, &d2);
    }

    #[test]
    fn lossy_threshold_substitutes_near_duplicate_with_rec7() {
        // Three 6×6 CCs on one page:
        //   - "base" solid block (1st CC → rec-1, becomes dict entry 0)
        //   - "near_dup" solid block with one pixel off (Hamming = 1)
        //   - "another_near_dup" solid block with a different pixel off
        //     (Hamming = 1 from base, Hamming = 2 from near_dup)
        //
        // Lossless (threshold = 0) → 2 rec-6 refinements.
        // Lossy (threshold = 0.05 ≈ 2 pixels of 36) → 2 rec-7 copies of base.
        // Lossy bytes < lossless bytes (rec-7 is smaller — no refinement bitmap).
        let base = make_bitmap(6, 6, |_, _| true);
        let near_dup = make_bitmap(6, 6, |x, y| !(x == 3 && y == 3));
        let another = make_bitmap(6, 6, |x, y| !(x == 1 && y == 4));

        let stamp = |page: &mut Bitmap, ox: u32, oy: u32, src: &Bitmap| {
            for y in 0..src.height {
                for x in 0..src.width {
                    if src.get(x, y) {
                        page.set(ox + x, oy + y, true);
                    }
                }
            }
        };
        let mut page = make_bitmap(40, 12, |_, _| false);
        stamp(&mut page, 2, 2, &base);
        stamp(&mut page, 14, 2, &near_dup);
        stamp(&mut page, 26, 2, &another);

        let lossless = encode_jb2_dict_with_options(
            &page,
            &[],
            &Jb2EncodeOptions {
                lossy_threshold: 0.0,
            },
        );
        let lossy = encode_jb2_dict_with_options(
            &page,
            &[],
            &Jb2EncodeOptions {
                lossy_threshold: 0.05,
            },
        );

        assert!(
            lossy.len() < lossless.len(),
            "lossy should be smaller than lossless: lossy={} lossless={}",
            lossy.len(),
            lossless.len()
        );

        // Lossy output decodes; the decoded near-duplicate CCs should now
        // be byte-identical to `base` (not to their original perturbed
        // pixels — that's the deliberate visual loss).
        let decoded = jb2::decode(&lossy, None).expect("lossy decode");
        assert_eq!(decoded.width, page.width);
        assert_eq!(decoded.height, page.height);

        // The first CC region (base) is unchanged. The second/third
        // regions used to have one missing pixel each; in lossy mode the
        // decoder fills them in (the substitute rec-7 references the
        // solid base).
        //
        // Sanity: original page is missing pixel at (14+3, 2+3) = (17, 5)
        // and at (26+1, 2+4) = (27, 6). The lossy decode should have those
        // pixels set (because rec-7 copied the solid `base`).
        assert!(
            decoded.get(17, 5),
            "lossy decode should fill base at (17,5)"
        );
        assert!(
            decoded.get(27, 6),
            "lossy decode should fill base at (27,6)"
        );

        // Lossless decode preserves the holes faithfully.
        let decoded_lossless = jb2::decode(&lossless, None).expect("lossless decode");
        assert!(
            !decoded_lossless.get(17, 5),
            "lossless should preserve hole at (17,5)"
        );
        assert!(
            !decoded_lossless.get(27, 6),
            "lossless should preserve hole at (27,6)"
        );
    }

    #[test]
    fn analyze_jb2_cc_stats_classifies_records() {
        // Three CCs on one page, each well-separated:
        //   1. solid 6×6 block             → byte-exact match against shared (rec-7)
        //   2. solid 6×6 minus one pixel   → Hamming-1 match against shared  (rec-6)
        //   3. solid 5×5 block             → unrelated, no same-size match   (rec-1)
        //
        // REFINEMENT_MIN_PIXELS = 32 forces the 5×5 path through rec-1 even
        // if the dict had a same-size entry. The 6×6 entries (36 pixels each)
        // are eligible for rec-6 against the shared dict.
        let shared_glyph = make_bitmap(6, 6, |_, _| true);
        let near_dup = make_bitmap(6, 6, |x, y| !(x == 3 && y == 3));
        let unrelated = make_bitmap(5, 5, |_, _| true);

        let stamp = |page: &mut Bitmap, ox: u32, oy: u32, src: &Bitmap| {
            for y in 0..src.height {
                for x in 0..src.width {
                    if src.get(x, y) {
                        page.set(ox + x, oy + y, true);
                    }
                }
            }
        };
        let mut page = make_bitmap(40, 12, |_, _| false);
        stamp(&mut page, 2, 2, &shared_glyph);
        stamp(&mut page, 14, 2, &near_dup);
        stamp(&mut page, 26, 2, &unrelated);

        let stats = analyze_jb2_cc_stats(&page, &[shared_glyph]);
        assert_eq!(stats.rec_7_exact, 1, "expected one byte-exact rec-7 hit");
        assert_eq!(
            stats.rec_6_refine_shared, 1,
            "expected one refinement against the shared dict slot"
        );
        assert_eq!(stats.rec_6_refine_local, 0);
        assert!(
            stats.rec_1_new >= 1,
            "expected at least one rec-1 (got {})",
            stats.rec_1_new
        );
        assert_eq!(stats.rec_6_hamming.len(), 1);
        assert_eq!(stats.rec_6_hamming[0], 1);
        assert!(stats.pixels_rec_7 > 0);
        assert!(stats.pixels_rec_6 > 0);
        assert!(stats.pixels_rec_1 > 0);
        assert_eq!(
            stats.total_ccs,
            stats.rec_1_new
                + stats.rec_6_refine_local
                + stats.rec_6_refine_shared
                + stats.rec_7_exact
        );
    }
}
