//! JB2 bilevel image encoder — produces Sjbz chunk payloads.
//!
//! Encodes a [`Bitmap`] into a JB2 stream decodable by [`crate::decode`].
//!
//! ## Encoding strategy
//!
//! The encoder emits the entire image as a single **record type 3** ("new symbol,
//! direct, blit only") record.  This produces valid output without requiring
//! connected-component analysis or a symbol dictionary.
//!
//! ## Binary format summary (see the crate-root decoder for full spec)
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

use crate::NumContext;
use djvu_bitmap::Bitmap;
use djvu_zp::encoder::ZpEncoder;

use std::collections::BTreeMap;

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
    // Unpack the MSB-first packed rows one byte → 8 pixels, instead of a
    // per-pixel `bm.get()` (which recomputes `y*stride + x/8` and `7-(x%8)`
    // for every pixel). Byte-identical: same bit layout, padding columns
    // `[w..pw]` stay zero.
    let stride = bm.row_stride();
    let full_bytes = w / 8;
    for y in 0..h {
        let src = &bm.data[y * stride..y * stride + stride];
        let dst = &mut pixels[(y + 2) * pw..(y + 2) * pw + w];
        let mut chunks = dst.chunks_exact_mut(8);
        for (&byte, chunk) in src.iter().zip(&mut chunks) {
            chunk[0] = (byte >> 7) & 1;
            chunk[1] = (byte >> 6) & 1;
            chunk[2] = (byte >> 5) & 1;
            chunk[3] = (byte >> 4) & 1;
            chunk[4] = (byte >> 3) & 1;
            chunk[5] = (byte >> 2) & 1;
            chunk[6] = (byte >> 1) & 1;
            chunk[7] = byte & 1;
        }
        let tail = chunks.into_remainder();
        if !tail.is_empty() {
            let byte = src[full_bytes];
            for (bit, slot) in tail.iter_mut().enumerate() {
                *slot = (byte >> (7 - bit)) & 1;
            }
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
/// Mirrors `decode_bitmap_ref` in the `djvu-jb2` crate **exactly**, including
/// its row traversal order and centre alignment. The decoder works in packed
/// Jbm storage, which is bottom-up: Jbm row `r` is image row `H - 1 - r`. It
/// decodes rows from `r = H-1` (image top) down to `r = 0` (image bottom),
/// and its centre-alignment `row_shift = mrow - crow` is applied in that
/// Jbm-row space. Because the `>> 1` centre floor is not symmetric under a
/// top-down/bottom-up flip, the encoder must operate in the *same* Jbm-row
/// space rather than image space — otherwise the reference rows the two sides
/// sample disagree on odd/even size deltas and the ZP streams desynchronise.
///
/// Both `cbm` and `mbm` are [`Bitmap`]s in top-down storage; the closures
/// below translate Jbm row indices back into top-down `get` calls.
///
/// Only the experiment-only cross-size rec-6 refinement path emits these, so
/// the encoder is gated behind `experimental`.
#[cfg(feature = "experimental")]
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
    let row_shift = mrow - crow;
    let col_shift = mcol - ccol;

    // Jbm-space pixel reads: Jbm row `r` ↔ image row `height - 1 - r`.
    let mbm_pix = |r: i32, x: i32| -> u32 {
        if r < 0 || r >= mh || x < 0 || x >= mw {
            0
        } else {
            mbm.get(x as u32, (mh - 1 - r) as u32) as u32
        }
    };
    let cbm_pix = |r: i32, x: i32| -> u32 {
        if r < 0 || r >= ch || x < 0 || x >= cw {
            0
        } else {
            cbm.get(x as u32, (ch - 1 - r) as u32) as u32
        }
    };

    for row in (0..ch).rev() {
        let mr = row + row_shift;

        // Rolling windows at col=0 (col-1 / col-2 OOB → 0). `c_r1` is the row
        // decoded just before this one — Jbm row `row + 1`.
        let mut c_r1 = (cbm_pix(row + 1, 0) << 1) | cbm_pix(row + 1, 1);
        let mut c_r0: u32 = 0;
        let mut m_r1 = (mbm_pix(mr, col_shift - 1) << 2)
            | (mbm_pix(mr, col_shift) << 1)
            | mbm_pix(mr, col_shift + 1);
        let mut m_r0 = (mbm_pix(mr - 1, col_shift - 1) << 2)
            | (mbm_pix(mr - 1, col_shift) << 1)
            | mbm_pix(mr - 1, col_shift + 1);

        for col in 0..cw {
            let m_r2 = mbm_pix(mr + 1, col + col_shift);
            let idx = ((c_r1 << 8) | (c_r0 << 7) | (m_r2 << 6) | (m_r1 << 3) | m_r0) & 2047;
            let bit = cbm_pix(row, col) != 0;
            zp.encode_bit(&mut ctx[idx as usize], bit);

            c_r1 = ((c_r1 << 1) & 0b111) | cbm_pix(row + 1, col + 2);
            c_r0 = bit as u32;
            m_r1 = ((m_r1 << 1) & 0b111) | mbm_pix(mr, col + col_shift + 2);
            m_r0 = ((m_r0 << 1) & 0b111) | mbm_pix(mr - 1, col + col_shift + 2);
        }
    }
}

// ── Public API ─────────────────────────────────────────────────────────────────

/// Encode a bilevel [`Bitmap`] into a JB2 stream (Sjbz chunk payload).
///
/// The returned bytes can be embedded directly in a `Sjbz` IFF chunk.
/// Decoding with [`crate::decode`] will reconstruct the original bitmap.
///
/// ## Encoding
///
/// Images with `width * height ≤ 1 MP` are emitted as a single direct-bitmap
/// record (type 3). Larger images are split into ≤ 1024×1024 tiles, each
/// emitted as its own record-3 — this keeps every symbol within the decoder's
/// `MAX_SYMBOL_PIXELS = 1 MP` DoS guard so the output round-trips through
/// [`crate::decode`] for any size up to `MAX_PIXELS = 64 MP`.
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
    // Byte-aligned fast path: when x0 is on a byte boundary (the only case the
    // tile loop produces — tiles start at multiples of TILE=1024), each output
    // row is a contiguous byte-copy from the source row instead of w per-pixel
    // get()+set_black() calls. The trailing bits of the last output byte that lie
    // beyond `w` must stay 0 (the per-pixel path never sets them), so mask them.
    if x0.is_multiple_of(8) {
        let src_stride = src.row_stride();
        let out_stride = out.row_stride();
        let src_byte0 = (x0 / 8) as usize;
        let last_mask: u8 = if w.is_multiple_of(8) {
            0xFF
        } else {
            0xFFu8 << (8 - (w % 8))
        };
        for y in 0..h as usize {
            let s = (y0 as usize + y) * src_stride + src_byte0;
            let d = y * out_stride;
            out.data[d..d + out_stride].copy_from_slice(&src.data[s..s + out_stride]);
            out.data[d + out_stride - 1] &= last_mask;
        }
        return out;
    }
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
    /// Count of black (foreground) pixels making up this component — the
    /// true "ink" area, unlike `bitmap.width * bitmap.height` (the bbox
    /// area, which overcounts for thin diagonal strokes). Used by
    /// [`Jb2EncodeOptions::despeckle`] to size-filter noise blobs before
    /// they ever reach clustering/dedup.
    pixel_count: u32,
}

/// Summary of an experiment-only cross-size refinement search.
///
/// This does not affect encoding. It estimates how many components currently
/// emitted as fresh record-1 symbols have a nearly matching dictionary symbol
/// with a slightly different bounding box.
#[cfg(feature = "experimental")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CrossSizeRefinementStats {
    /// Total connected components seen in reading order.
    pub total_ccs: usize,
    /// Components that do not have an exact same-size dictionary hit.
    pub fresh_ccs: usize,
    /// Fresh components large enough to consider for refinement.
    pub eligible_fresh_ccs: usize,
    /// Eligible fresh components with at least one near-size candidate.
    pub candidate_ccs: usize,
    /// Candidate components whose best normalized Hamming score is within
    /// the caller-provided pixel fraction.
    pub near_matches: usize,
    /// Sum of source component pixels for near matches.
    pub near_match_pixels: u64,
    /// Best normalized Hamming score observed for every near-size candidate.
    pub best_hamming: Vec<u32>,
    /// Approximate bytes current record-1 emissions spend on near-match symbols.
    ///
    /// This includes direct bitmap payload bytes plus a small fixed record
    /// overhead approximation. It intentionally excludes coordinate coding
    /// because both record-1 and hypothetical record-6 still place a symbol.
    pub estimated_rec1_bytes: u64,
    /// Approximate bytes a hypothetical cross-size record-6 path would spend
    /// for the same near-match symbols.
    ///
    /// Includes an estimated symbol-index/context overhead and a packed
    /// refinement-difference payload estimate. No encoder behavior depends on
    /// this value; it is measurement-only.
    pub estimated_cross_size_rec6_bytes: u64,
    /// Estimated byte delta: `cross_size_rec6 - current_rec1`.
    /// Negative means the hypothetical path may save bytes.
    pub estimated_byte_delta: i64,
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
    // Byte-unpack the MSB-first packed rows (8 pixels per byte, constant shifts)
    // instead of a per-pixel `bitmap.get()` that recomputes `y*stride + x/8` and
    // `7-(x%8)` (a hidden divide) for every pixel — the same win as the
    // `encode_bitmap_direct` byte-unpack (PS2). Byte-identical `pix` output.
    let mut pix = vec![0u8; w * h];
    let stride = bitmap.row_stride();
    let full_bytes = w / 8;
    for y in 0..h {
        let src = &bitmap.data[y * stride..y * stride + stride];
        let dst = &mut pix[y * w..y * w + w];
        let mut chunks = dst.chunks_exact_mut(8);
        for (&byte, chunk) in src.iter().zip(&mut chunks) {
            chunk[0] = (byte >> 7) & 1;
            chunk[1] = (byte >> 6) & 1;
            chunk[2] = (byte >> 5) & 1;
            chunk[3] = (byte >> 4) & 1;
            chunk[4] = (byte >> 3) & 1;
            chunk[5] = (byte >> 2) & 1;
            chunk[6] = (byte >> 1) & 1;
            chunk[7] = byte & 1;
        }
        let tail = chunks.into_remainder();
        if !tail.is_empty() {
            let byte = src[full_bytes];
            for (bit, slot) in tail.iter_mut().enumerate() {
                *slot = (byte >> (7 - bit)) & 1;
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
                pixel_count: cc_pixels.len() as u32,
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

/// Minimum pixel area for a CC to be considered for refinement matching.
///
/// Sub-32-pixel CCs (typical: dust, single anti-aliasing fragments) encode
/// in only a handful of bytes via record-1; the per-record overhead of a
/// record-6 (matched-refinement coordinate header + 11-bit refinement
/// context state) outweighs any saving even at low Hamming distance.
const REFINEMENT_MIN_PIXELS: u64 = 32;

#[cfg(feature = "experimental")]
fn scaled_hamming(cand: &Bitmap, reference: &Bitmap) -> u32 {
    let mut diff = 0u32;
    for y in 0..cand.height {
        let ry = (u64::from(y) * u64::from(reference.height) / u64::from(cand.height)) as u32;
        for x in 0..cand.width {
            let rx = (u64::from(x) * u64::from(reference.width) / u64::from(cand.width)) as u32;
            if cand.get(x, y) != reference.get(rx, ry) {
                diff += 1;
            }
        }
    }
    diff
}

#[cfg(feature = "experimental")]
fn packed_bytes_for_pixels(pixels: u64) -> u64 {
    pixels.div_ceil(8)
}

/// FNV-1a hash of a symbol's `(w, h, packed-data)`, used as the bucket key for
/// exact-match dedup. Replaces a `BTreeMap` keyed by `(u32, u32, Vec<u8>)`, which
/// cloned the bitmap data on every connected-component lookup; the hash buckets
/// (`BTreeMap<u64, Vec<usize>>`) compare the actual data only on a hash hit, so
/// dedup stays byte-identical while avoiding the per-CC allocation.
#[inline]
fn symbol_hash(w: u32, h: u32, data: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut mix = |bytes: &[u8]| {
        for &b in bytes {
            hash = (hash ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    mix(&w.to_le_bytes());
    mix(&h.to_le_bytes());
    mix(data);
    hash
}

#[cfg(feature = "experimental")]
fn index_overhead_bytes(dict_len: usize) -> u64 {
    let bits = usize::BITS - dict_len.max(1).leading_zeros();
    u64::from(bits).div_ceil(8)
}

#[cfg(feature = "experimental")]
fn estimate_record1_symbol_bytes(symbol: &Bitmap) -> u64 {
    const RECORD1_OVERHEAD_BYTES: u64 = 3; // record type + width + height, approximate.
    symbol.data.len() as u64 + RECORD1_OVERHEAD_BYTES
}

#[cfg(feature = "experimental")]
fn estimate_cross_size_rec6_symbol_bytes(hamming: u32, dict_len: usize) -> u64 {
    const RECORD6_OVERHEAD_BYTES: u64 = 5; // record type + wdiff/hdiff + refinement flags, approximate.
    RECORD6_OVERHEAD_BYTES
        + index_overhead_bytes(dict_len)
        + packed_bytes_for_pixels(u64::from(hamming))
}

/// Estimate cross-size refinement headroom without changing encoder output.
///
/// The JB2 format can encode record-6 refinements where the reference symbol
/// has a different `(w, h)`, but the shipped encoder intentionally only uses
/// exact record-7 copies. This helper mirrors the dictionary growth of
/// [`encode_jb2_dict_with_shared`] and, for fresh symbols, scores nearby
/// dictionary entries after nearest-neighbor normalization into the candidate
/// component's dimensions.
///
/// `max_dim_delta` limits candidates to entries with width/height differing
/// by at most that many pixels. `max_hamming_fraction` is the accepted
/// normalized Hamming budget relative to the candidate's pixel count.
#[cfg(feature = "experimental")]
pub fn analyze_jb2_cross_size_refinement(
    bitmap: &Bitmap,
    shared_symbols: &[Bitmap],
    max_dim_delta: u32,
    max_hamming_fraction: f32,
) -> CrossSizeRefinementStats {
    let mut stats = CrossSizeRefinementStats::default();
    if bitmap.width == 0 || bitmap.height == 0 {
        return stats;
    }

    let ccs = extract_ccs(bitmap);
    let mut order: Vec<usize> = (0..ccs.len()).collect();
    let bucket = (SAME_LINE_BASELINE_TOL.max(1)) as u32;
    order.sort_by_key(|&i| {
        let cc = &ccs[i];
        let bottom = cc.y + cc.bitmap.height;
        (bottom / bucket, cc.x)
    });

    let mut dedup: BTreeMap<(u32, u32, Vec<u8>), usize> = BTreeMap::new();
    let mut dict_entries: Vec<Bitmap> = Vec::new();
    for sym in shared_symbols {
        let idx = dict_entries.len();
        dedup.insert((sym.width, sym.height, sym.data.clone()), idx);
        dict_entries.push(sym.clone());
    }
    let mut by_size: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for (idx, sym) in dict_entries.iter().enumerate() {
        by_size
            .entry((sym.width, sym.height))
            .or_default()
            .push(idx);
    }

    for &cc_idx in &order {
        let cc = &ccs[cc_idx];
        stats.total_ccs += 1;

        let key = (cc.bitmap.width, cc.bitmap.height, cc.bitmap.data.clone());
        if dedup.contains_key(&key) {
            continue;
        }

        stats.fresh_ccs += 1;
        let pixels = u64::from(cc.bitmap.width) * u64::from(cc.bitmap.height);
        if pixels >= REFINEMENT_MIN_PIXELS {
            stats.eligible_fresh_ccs += 1;
            let mut best: Option<u32> = None;
            let min_w = cc.bitmap.width.saturating_sub(max_dim_delta);
            let max_w = cc.bitmap.width.saturating_add(max_dim_delta);
            let min_h = cc.bitmap.height.saturating_sub(max_dim_delta);
            let max_h = cc.bitmap.height.saturating_add(max_dim_delta);
            for w in min_w..=max_w {
                for h in min_h..=max_h {
                    if w == cc.bitmap.width && h == cc.bitmap.height {
                        continue;
                    }
                    let Some(indices) = by_size.get(&(w, h)) else {
                        continue;
                    };
                    for &idx in indices {
                        let d = scaled_hamming(&cc.bitmap, &dict_entries[idx]);
                        best = Some(best.map_or(d, |b| b.min(d)));
                    }
                }
            }
            if let Some(best) = best {
                stats.candidate_ccs += 1;
                stats.best_hamming.push(best);
                let max_diff = ((pixels as f64) * (max_hamming_fraction as f64)).round() as u32;
                if best <= max_diff {
                    stats.near_matches += 1;
                    stats.near_match_pixels += pixels;
                    let rec1 = estimate_record1_symbol_bytes(&cc.bitmap);
                    let rec6 = estimate_cross_size_rec6_symbol_bytes(best, dict_entries.len());
                    stats.estimated_rec1_bytes += rec1;
                    stats.estimated_cross_size_rec6_bytes += rec6;
                    stats.estimated_byte_delta += rec6 as i64 - rec1 as i64;
                }
            }
        }

        let next_idx = dict_entries.len();
        dedup.insert(key, next_idx);
        by_size
            .entry((cc.bitmap.width, cc.bitmap.height))
            .or_default()
            .push(next_idx);
        dict_entries.push(cc.bitmap.clone());
    }

    stats
}

/// Summary of an experiment-only **same-size** refinement search (Phase A0 of
/// `docs/jb2-size-gap-plan.md`).
///
/// Measurement only — does not change encoding. It counts, for the components
/// the default encoder emits as fresh record-1 symbols, how many have a
/// **same-bounding-box** dictionary twin within a small Hamming distance. Those
/// are the candidates for a lossless same-size record-6 refinement, the one
/// untried lever that avoids the resampling misalignment that made cross-size
/// rec-6 (#322) lose bytes. It proves a *population*, not a byte outcome — the
/// #301 lesson is that only a real emitter (Phase A1/A2) proves bytes.
#[cfg(feature = "experimental")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SameSizeRefinementStats {
    /// Total connected components seen in reading order.
    pub total_ccs: usize,
    /// Components with no exact same-`(w, h, data)` dictionary hit (i.e. the
    /// ones the default encoder emits as a fresh record-1 symbol).
    pub fresh_ccs: usize,
    /// Fresh components large enough to consider for refinement
    /// (`pixels >= REFINEMENT_MIN_PIXELS`).
    pub eligible_fresh_ccs: usize,
    /// Eligible fresh components with at least one same-size dictionary entry to
    /// score against.
    pub candidate_ccs: usize,
    /// Candidate components whose best (minimum) same-size Hamming distance is
    /// within 2 % / 5 % / 10 % of the component's pixel count.
    pub near_le_2pct: usize,
    pub near_le_5pct: usize,
    pub near_le_10pct: usize,
    /// Sum of component pixels for the ≤5 % near-twins (a proxy for how much
    /// direct-bitmap payload a refinement path could shrink).
    pub near_le_5pct_pixels: u64,
    /// Sum of the raw best Hamming *bytes* (`ceil(hamming/8)`) for ≤5 % near-twins
    /// — a crude floor on the refinement-bitmap payload if it coded one bit per
    /// differing pixel (the real ZP cost differs; this is only a scale hint).
    pub near_le_5pct_hamming_bytes: u64,
    /// Best Hamming fraction in per-mille (‰) for every candidate component, for
    /// histogramming the distance distribution.
    pub best_hamming_permille: Vec<u32>,
}

/// Measure the same-size refinement candidate population without changing output
/// (Phase A0). Mirrors the default encoder's exact-dedup dictionary growth
/// (`encode_jb2_dict_with_shared`: exact record-7 copy or fresh record-1, no
/// refinement), then for every fresh, eligible component scores its minimum
/// Hamming distance against same-`(w, h)` dictionary entries.
#[cfg(feature = "experimental")]
pub fn analyze_jb2_same_size_refinement(
    bitmap: &Bitmap,
    shared_symbols: &[Bitmap],
) -> SameSizeRefinementStats {
    same_size_refinement_scan(bitmap, shared_symbols, None)
}

/// Shared scan core behind [`analyze_jb2_same_size_refinement`] (full
/// document, `fresh_cc_limit: None`) and [`probe_same_size_rec6_density`]
/// (bounded, `Some(max_ccs)` — the auto-policy's cheap density probe, JB2_AUTO_REC6).
/// Mirrors the default encoder's exact-dedup dictionary growth, then for
/// every fresh, eligible component scores its minimum Hamming distance
/// against same-`(w, h)` dictionary entries. When `fresh_cc_limit` is set,
/// scanning stops as soon as that many *fresh* CCs have been examined —
/// bounding the Hamming-scoring cost independent of page size, since the one
/// unavoidable fixed cost (`extract_ccs`) is paid by the real encoder anyway.
#[cfg(feature = "experimental")]
fn same_size_refinement_scan(
    bitmap: &Bitmap,
    shared_symbols: &[Bitmap],
    fresh_cc_limit: Option<usize>,
) -> SameSizeRefinementStats {
    let mut stats = SameSizeRefinementStats::default();
    if bitmap.width == 0 || bitmap.height == 0 {
        return stats;
    }

    let ccs = extract_ccs(bitmap);
    // Same reading-order sort the real encoder uses, so `first_seen` reference
    // selection matches the shipped path.
    let mut order: Vec<usize> = (0..ccs.len()).collect();
    let bucket = (SAME_LINE_BASELINE_TOL.max(1)) as u32;
    order.sort_by_key(|&i| {
        let cc = &ccs[i];
        let bottom = cc.y + cc.bitmap.height;
        (bottom / bucket, cc.x)
    });

    // Exact-dedup dictionary, seeded from the shared symbols, exactly as the
    // encoder builds it (symbol_hash key + by_size index).
    let mut dedup: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    let mut dict_entries: Vec<&Bitmap> = Vec::new();
    let mut by_size: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for sym in shared_symbols {
        let idx = dict_entries.len();
        dedup
            .entry(symbol_hash(sym.width, sym.height, &sym.data))
            .or_default()
            .push(idx);
        by_size
            .entry((sym.width, sym.height))
            .or_default()
            .push(idx);
        dict_entries.push(sym);
    }

    for &cc_idx in &order {
        let cc = &ccs[cc_idx];
        stats.total_ccs += 1;
        let bm = &cc.bitmap;
        let dkey = symbol_hash(bm.width, bm.height, &bm.data);
        let exact = dedup.get(&dkey).is_some_and(|cands| {
            cands.iter().copied().any(|i| {
                let d = dict_entries[i];
                d.width == bm.width && d.height == bm.height && d.data == bm.data
            })
        });
        if exact {
            // Default encoder: record-7 copy, dict unchanged.
            continue;
        }

        stats.fresh_ccs += 1;
        let pixels = u64::from(bm.width) * u64::from(bm.height);
        if pixels >= REFINEMENT_MIN_PIXELS {
            stats.eligible_fresh_ccs += 1;
            if let Some(indices) = by_size.get(&(bm.width, bm.height)) {
                let mut best: Option<u32> = None;
                for &idx in indices {
                    let d = packed_hamming(&bm.data, &dict_entries[idx].data);
                    best = Some(best.map_or(d, |b| b.min(d)));
                }
                if let Some(best) = best {
                    stats.candidate_ccs += 1;
                    let frac_permille = ((u64::from(best) * 1000) / pixels.max(1)) as u32;
                    stats.best_hamming_permille.push(frac_permille);
                    if frac_permille <= 20 {
                        stats.near_le_2pct += 1;
                    }
                    if frac_permille <= 50 {
                        stats.near_le_5pct += 1;
                        stats.near_le_5pct_pixels += pixels;
                        stats.near_le_5pct_hamming_bytes += u64::from(best).div_ceil(8);
                    }
                    if frac_permille <= 100 {
                        stats.near_le_10pct += 1;
                    }
                }
            }
        }

        // Default encoder: fresh record-1, added to the dict.
        let next_idx = dict_entries.len();
        dedup.entry(dkey).or_default().push(next_idx);
        by_size
            .entry((bm.width, bm.height))
            .or_default()
            .push(next_idx);
        dict_entries.push(bm);

        if let Some(limit) = fresh_cc_limit
            && stats.fresh_ccs >= limit
        {
            break;
        }
    }

    stats
}

/// Default bounded sample size for [`probe_same_size_rec6_density`] and
/// [`Jb2EncodeOptions::same_size_rec6_auto`]: the probe stops scanning once
/// this many *fresh* CCs have been examined. Large enough to be a stable
/// estimate — watchmaker's whole-page fresh population is 3 475 components,
/// and 1 000 samples converges well before that — yet small enough that the
/// probe's Hamming-scoring cost is capped regardless of page size: the
/// 821 330-fresh-CC `pathogenic_bacteria_1896` page pays the same bounded
/// scan a small page would.
#[cfg(feature = "experimental")]
pub const SAME_SIZE_REC6_AUTO_SAMPLE_CCS: usize = 1000;

/// Density threshold (fraction of sampled fresh CCs with a same-size ≤5 %
/// Hamming twin) at/above which [`Jb2EncodeOptions::same_size_rec6_auto`]
/// enables same-size rec-6.
///
/// Calibrated against **real emitted-byte deltas** on four corpora (not just
/// population counts — see the #301 lesson), JB2_AUTO_REC6 in
/// `PERF_EXPERIMENTS.md`:
///
/// | Corpus | density (≤5 % near-twins / fresh) | Sjbz delta at frac 2 % |
/// |--------|------------------------------------|------------------------|
/// | watchmaker | 39.6 % | **−11.67 %** |
/// | cable_1973_100133 | 12.4 % | −0.43 % |
/// | conquete_paix | 1.7 % | **+0.49 %** (loss) |
/// | pathogenic_bacteria_1896 | 0.9 % | +0.00 % (flat) |
///
/// The real-byte outcome flips from a loss to a win between 1.7 % and
/// 12.4 % density. `0.05` sits with roughly 2.9× margin above the measured
/// loss and 2.5× margin below the measured win — enabling on `cable` (a
/// small extra win) and `watchmaker` (the large win) while staying off for
/// `conquete_paix` and `pathogenic` (avoiding their losses/flat result).
#[cfg(feature = "experimental")]
pub const SAME_SIZE_REC6_AUTO_DENSITY_THRESHOLD: f32 = 0.05;

/// Refinement fraction [`Jb2EncodeOptions::same_size_rec6_auto`] applies once
/// density clears [`SAME_SIZE_REC6_AUTO_DENSITY_THRESHOLD`] — the validated
/// sweet spot from round 18 (a tighter threshold wins more: 2 % > 5 % > 8 %
/// on watchmaker).
#[cfg(feature = "experimental")]
pub const SAME_SIZE_REC6_AUTO_FRAC: f32 = 0.02;

/// Cheap, bounded density probe backing [`Jb2EncodeOptions::same_size_rec6_auto`]
/// (Phase A3 follow-up of `docs/jb2-size-gap-plan.md`, JB2_AUTO_REC6).
///
/// Scans at most `max_ccs` *fresh* connected components (same reading order
/// the encoder uses) and returns the fraction of them with a same-size
/// Hamming twin within 5 % — the metric validated in round 17/18 as
/// predictive of a real byte win (see
/// [`SAME_SIZE_REC6_AUTO_DENSITY_THRESHOLD`]'s table). Capping the scan
/// keeps the probe's incremental cost independent of page size; the fixed
/// `extract_ccs` pass is one the real encoder pays regardless of this
/// option, so the probe's marginal cost over a plain encode is just that
/// bounded Hamming scan.
#[cfg(feature = "experimental")]
pub fn probe_same_size_rec6_density(
    bitmap: &Bitmap,
    shared_symbols: &[Bitmap],
    max_ccs: usize,
) -> f32 {
    let stats = same_size_refinement_scan(bitmap, shared_symbols, Some(max_ccs.max(1)));
    if stats.fresh_ccs == 0 {
        return 0.0;
    }
    stats.near_le_5pct as f32 / stats.fresh_ccs as f32
}

/// Find the closest same-size dict entry within a Hamming-distance budget,
/// for use as a **lossy copy** target (record-7) — the encoder pretends the
/// near-duplicate is byte-exact, the decoder produces the dict entry's pixels
/// instead of the original CC. Visual loss is bounded by the threshold.
///
/// Used by [`Jb2EncodeOptions::lossy_threshold`] (#224 Phase 4); independent
/// of `find_refinement_ref`, which gated record-6 (lossless refinement).
fn find_lossy_copy_ref(
    cand: &Bitmap,
    dict_entries: &[&Bitmap],
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
        let ref_bm = dict_entries[i];
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

/// Find the closest **cross-size** dict entry suitable for a lossless record-6
/// matched refinement (#322 experiment).
///
/// Candidates are dict entries whose width and height each differ from `cand`
/// by at most `max_dim_delta` (and are not exactly `cand`'s size). Distance is
/// scored with [`scaled_hamming`] — nearest-neighbor resampling of the
/// candidate into `cand`'s grid — and accepted when within
/// `pixel_count × max_hamming_fraction` flipped pixels. Returns the dict index
/// of the best (lowest-distance) accepted candidate.
///
/// The match only selects the *reference* glyph; the emitted refinement bitmap
/// reproduces `cand` exactly, so this is lossless regardless of the score.
#[cfg(feature = "experimental")]
fn find_cross_size_refine_ref(
    cand: &Bitmap,
    dict_entries: &[&Bitmap],
    by_size: &BTreeMap<(u32, u32), Vec<usize>>,
    max_dim_delta: u32,
    max_hamming_fraction: f32,
) -> Option<usize> {
    let pixel_count = (cand.width as u64) * (cand.height as u64);
    if pixel_count < REFINEMENT_MIN_PIXELS {
        return None;
    }
    let max_diff = ((pixel_count as f64) * (max_hamming_fraction as f64)).round() as u32;
    let min_w = cand.width.saturating_sub(max_dim_delta);
    let max_w = cand.width.saturating_add(max_dim_delta);
    let min_h = cand.height.saturating_sub(max_dim_delta);
    let max_h = cand.height.saturating_add(max_dim_delta);
    let mut best: Option<(usize, u32)> = None;
    for w in min_w..=max_w {
        for h in min_h..=max_h {
            if w == cand.width && h == cand.height {
                continue;
            }
            let Some(indices) = by_size.get(&(w, h)) else {
                continue;
            };
            for &idx in indices {
                let d = scaled_hamming(cand, dict_entries[idx]);
                if d > max_diff {
                    continue;
                }
                match best {
                    None => best = Some((idx, d)),
                    Some((_, bd)) if d < bd => best = Some((idx, d)),
                    _ => {}
                }
            }
        }
    }
    best.map(|(i, _)| i)
}

/// Find the closest **same-size** dict entry for a lossless record-6 matched
/// refinement (Phase A1 of `docs/jb2-size-gap-plan.md`).
///
/// Candidates are dict entries of exactly `cand`'s `(w, h)`. Distance is the
/// direct packed Hamming (no resampling — the reference aligns pixel-for-pixel),
/// accepted within `pixel_count × max_hamming_fraction` flipped pixels. Returns
/// the dict index of the nearest accepted candidate. The emitted refinement
/// bitmap reproduces `cand` exactly, so the result is lossless.
#[cfg(feature = "experimental")]
fn find_same_size_refine_ref(
    cand: &Bitmap,
    dict_entries: &[&Bitmap],
    same_size_indices: &[usize],
    max_hamming_fraction: f32,
) -> Option<usize> {
    let pixel_count = (cand.width as u64) * (cand.height as u64);
    if pixel_count < REFINEMENT_MIN_PIXELS {
        return None;
    }
    let max_diff = ((pixel_count as f64) * (max_hamming_fraction as f64)).round() as u32;
    let mut best: Option<(usize, u32)> = None;
    for &idx in same_size_indices {
        let ref_bm = dict_entries[idx];
        debug_assert_eq!(ref_bm.width, cand.width);
        debug_assert_eq!(ref_bm.height, cand.height);
        let d = packed_hamming(&cand.data, &ref_bm.data);
        if d > max_diff {
            continue;
        }
        match best {
            None => best = Some((idx, d)),
            Some((_, bd)) if d < bd => best = Some((idx, d)),
            _ => {}
        }
    }
    best.map(|(i, _)| i)
}

/// Experiment-only knobs for the cross-size record-6 refinement emitter (#322).
///
/// When present in [`Jb2EncodeOptions::cross_size_rec6_probe`], fresh
/// connected components that have no exact dictionary hit are matched against
/// dictionary entries whose bounding box differs by at most `max_dim_delta`
/// pixels in each axis. If a near-twin is found within the normalized Hamming
/// budget, the component is emitted as a **lossless** record-6 matched
/// refinement (`wdiff`/`hdiff` + 11-bit refinement bitmap) referencing that
/// entry instead of a fresh record-1.
///
/// This is a measurement vehicle: the refinement bitmap reproduces the
/// component exactly (round-trip is pixel-lossless), but it is *not* wired into
/// any shipped encoder path. [`encode_jb2_dict`] /
/// [`encode_jb2_dict_with_shared`] leave it disabled, so default output is
/// byte-identical to before.
#[cfg(feature = "experimental")]
#[derive(Debug, Clone, Copy)]
pub struct CrossSizeRec6Probe {
    /// Maximum per-axis bounding-box difference (in pixels) between a fresh
    /// component and a candidate dictionary entry.
    pub max_dim_delta: u32,
    /// Accepted normalized Hamming budget as a fraction of the component's
    /// pixel count, scored after nearest-neighbor resampling of the candidate
    /// into the component's dimensions.
    pub max_hamming_fraction: f32,
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
    ///
    /// **Measured operating points** (Branch B / round 19; `watchmaker`, a text
    /// scan where the JB2 mask is 67 % of the file; mask quality via the D1
    /// PSNR/SSIM harness):
    ///
    /// | `lossy_threshold` | Sjbz size | SSIM |
    /// |-------------------|-----------|------|
    /// | `0.02` | **−22 %** | 0.9993 |
    /// | `0.05` | −23 % | 0.9989 |
    /// | `0.08` | −24 % | 0.9986 |
    ///
    /// `0.02` is the sweet spot: a large size reduction at near-imperceptible
    /// loss (≈ 0.02 % of mask pixels flipped). Returns diminish sharply above it.
    /// See [`Jb2EncodeOptions::lossy_text`] for that preset. **Note:** the win is
    /// a *text*-document lever — on noisy high-dpi photo scans the same-size
    /// near-twin population is thin, so low thresholds barely shrink anything.
    pub lossy_threshold: f32,
    /// **Despeckle** pre-pass (cjb2's classic noise-removal move). `None`
    /// (default) keeps every connected component, exactly as extracted.
    /// `Some(max_px)` drops any component whose **foreground pixel count**
    /// (not bbox area — a diagonal speck's bbox overcounts) is `<= max_px`
    /// *before* clustering/dedup: the speck is never emitted as a symbol at
    /// all, so its dict-entry + coordinate-record cost disappears and it
    /// also stops diluting the near-twin population that
    /// [`lossy_threshold`](Self::lossy_threshold) matches against.
    ///
    /// This is lossy: the removed pixels are gone from the decoded page —
    /// there is no dict entry left to reconstruct them from. Intended for
    /// noisy high-dpi scans (binarization "salt" dust), where isolated
    /// 1–8 px blobs are overwhelmingly noise rather than content.
    ///
    /// **Measured operating points** (JB2_DESPECKLE; `pathogenic_bacteria_1896`,
    /// a 600 dpi noisy scan where prior levers — same-size and cross-size
    /// lossy substitution — found almost nothing to substitute; mask quality
    /// via the D1 SSIM harness):
    ///
    /// | `despeckle` | Sjbz size | SSIM |
    /// |-------------|-----------|------|
    /// | `2` | −0.94 % | 0.99950 |
    /// | `4` | −1.59 % | 0.99904 |
    /// | `8` | **−2.43 %** | 0.99845 |
    ///
    /// On clean text (`watchmaker`) despeckle at every tested level is a
    /// **byte-identical no-op** — real glyphs are all well above 8 px, so
    /// nothing is removed. Despeckle is a scan-specific lever: it is the
    /// first lossy lever found to move the noisy-scan corpus at all
    /// (`lossy_threshold` alone gives ≈ 0 % there — see its docs above)
    /// and does so at near-invisible cost (≤ 0.02 % of mask pixels flipped
    /// even at `8`). See [`Jb2EncodeOptions::lossy_scan`] for the combined
    /// preset and `PERF_EXPERIMENTS.md` (JB2_DESPECKLE) for the full sweep
    /// and a punctuation/diacritic-survival test.
    ///
    /// `None` (default) = lossless: no component is ever dropped,
    /// byte-identical to the shipped encoder.
    pub despeckle: Option<u32>,
    /// Experiment-only cross-size record-6 refinement (#322). `None` (default)
    /// keeps the shipped behavior — only record-1 (new) and record-7 (copy)
    /// are emitted. `Some(_)` enables the lossless cross-size refinement path
    /// described on [`CrossSizeRec6Probe`].
    #[cfg(feature = "experimental")]
    pub cross_size_rec6_probe: Option<CrossSizeRec6Probe>,
    /// Experiment-only **same-size** record-6 refinement (Phase A1 of
    /// `docs/jb2-size-gap-plan.md`). `None` (default) keeps the shipped
    /// behavior. `Some(frac)` diverts a fresh CC that has a **same-bounding-box**
    /// dictionary twin within `pixel_count × frac` flipped pixels to a lossless
    /// record-6 refinement (`wdiff = hdiff = 0`) against that twin, instead of a
    /// fresh record-1. Unlike the cross-size probe, no resampling is involved, so
    /// the refinement context stays pixel-aligned. Lossless (round-trip exact).
    #[cfg(feature = "experimental")]
    pub same_size_rec6: Option<f32>,
}

impl Default for Jb2EncodeOptions {
    fn default() -> Self {
        Self {
            lossy_threshold: 0.0,
            despeckle: None,
            #[cfg(feature = "experimental")]
            cross_size_rec6_probe: None,
            #[cfg(feature = "experimental")]
            same_size_rec6: None,
        }
    }
}

impl Jb2EncodeOptions {
    /// Recommended **lossy** preset for text documents: `lossy_threshold = 0.02`.
    ///
    /// Measured on `watchmaker` (round 19): **≈ −22 % Sjbz** at SSIM 0.9993 —
    /// a large size reduction at near-imperceptible loss, the sweet spot of the
    /// [`lossy_threshold`](Self::lossy_threshold) curve and roughly DjVuLibre
    /// `cjb2`'s default lossy operating point. Opt-in: the encoder stays lossless
    /// unless you choose this (or set `lossy_threshold` yourself). Best for text
    /// scans; on noisy photo scans the near-twin population is thin so it saves
    /// little.
    pub fn lossy_text() -> Self {
        Self::with_lossy_threshold(0.02)
    }

    /// Set the [`lossy_threshold`](Self::lossy_threshold) (builder style),
    /// leaving every other knob at its default. `0.0` keeps lossless behavior.
    #[allow(clippy::needless_update)] // spread sets the experimental fields when compiled in
    pub fn with_lossy_threshold(threshold: f32) -> Self {
        Self {
            lossy_threshold: threshold,
            ..Self::default()
        }
    }

    /// Set [`despeckle`](Self::despeckle) (builder style), leaving every
    /// other knob at its default. `max_px` is the largest foreground-pixel
    /// count a component may have and still be dropped as a speck.
    #[allow(clippy::needless_update)]
    pub fn with_despeckle(max_px: u32) -> Self {
        Self {
            despeckle: Some(max_px),
            ..Self::default()
        }
    }

    /// Recommended **lossy** preset for noisy high-dpi scans (JB2_DESPECKLE):
    /// `despeckle = 8` combined with `lossy_threshold = 0.02`.
    ///
    /// Measured on `pathogenic_bacteria_1896` (600 dpi scan): despeckle at
    /// `8` gives **−2.43 % Sjbz at SSIM 0.99845** (≤ 0.02 % of mask pixels
    /// flipped) — the first lossy lever measured to move this corpus at all
    /// (same-size and cross-size near-twin substitution both found ≈ 0 %
    /// headroom there; see [`lossy_threshold`](Self::lossy_threshold)'s
    /// docs). The stacked `lossy_threshold = 0.02` adds negligible extra
    /// size on this scan corpus (its near-twin population stays thin
    /// regardless of despeckling) but costs nothing and helps on any
    /// mixed/text-like content on the same page. On clean text
    /// (`watchmaker`) despeckle at every tested level (2/4/8) is a
    /// byte-identical no-op — real glyphs are all well above 8 px — so this
    /// preset is safe to try on both, but its win is scan-specific. See
    /// `PERF_EXPERIMENTS.md` (JB2_DESPECKLE) for the full despeckle x
    /// lossy_threshold sweep and a punctuation/diacritic-survival test.
    ///
    /// Opt-in: the encoder stays lossless unless you choose this (or set
    /// `despeckle`/`lossy_threshold` yourself).
    #[allow(clippy::needless_update)]
    pub fn lossy_scan() -> Self {
        Self {
            despeckle: Some(8),
            lossy_threshold: 0.02,
            ..Self::default()
        }
    }

    /// Auto-policy preset for [`same_size_rec6`](Self::same_size_rec6)
    /// (JB2_AUTO_REC6, the Phase A3 follow-up of
    /// `docs/jb2-size-gap-plan.md`): runs the cheap, bounded
    /// [`probe_same_size_rec6_density`] against `bitmap` and enables
    /// same-size rec-6 at [`SAME_SIZE_REC6_AUTO_FRAC`] when the near-twin
    /// density is at/above [`SAME_SIZE_REC6_AUTO_DENSITY_THRESHOLD`], leaving
    /// it `None` (shipped lossless behavior) otherwise. Every other knob
    /// stays at its default.
    ///
    /// Call this **once per document** — e.g. on its first page, or any
    /// single representative page — and reuse the returned [`Jb2EncodeOptions`]
    /// for every page's [`encode_jb2_dict_with_options`] call. That way the
    /// probe's one fixed `extract_ccs` pass is paid once per document rather
    /// than once per page, and every page gets a consistent policy (matching
    /// the validated per-corpus numbers, which were measured with one
    /// decision applied across all of a document's pages).
    ///
    /// **Validated:** on `watchmaker` (text, density 39.6 %) fires and
    /// reproduces the ≈ −11.67 % Sjbz win, lossless round-trip on every page.
    /// On `pathogenic_bacteria_1896` (600 dpi scan, density 0.9 %) and
    /// `conquete_paix` (density 1.7 %) it stays off, so output is
    /// byte-identical to the default encoder. Opt-in and behind
    /// `experimental`; not a stable API and not enabled by default — the
    /// maintainer's A3 decision keeps `same_size_rec6` an experimental,
    /// explicit-opt-in lever (see `PERF_EXPERIMENTS.md`).
    #[cfg(feature = "experimental")]
    pub fn same_size_rec6_auto(bitmap: &Bitmap, shared_symbols: &[Bitmap]) -> Self {
        let density =
            probe_same_size_rec6_density(bitmap, shared_symbols, SAME_SIZE_REC6_AUTO_SAMPLE_CCS);
        Self {
            same_size_rec6: if density >= SAME_SIZE_REC6_AUTO_DENSITY_THRESHOLD {
                Some(SAME_SIZE_REC6_AUTO_FRAC)
            } else {
                None
            },
            ..Self::default()
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
/// Lossless. Matches [`crate::decode`] for round-trip.
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
/// `decode_dict(djbz_bytes, None)` to [`crate::decode`].
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
/// [`crate::decode`].
pub fn encode_jb2_dict_with_options(
    bitmap: &Bitmap,
    shared_symbols: &[Bitmap],
    opts: &Jb2EncodeOptions,
) -> Vec<u8> {
    encode_jb2_dict_with_blits(bitmap, shared_symbols, opts).0
}

/// One emitted blit: its cropped shape and top-left position (top-down page
/// coordinates), in emission order — blit *i* here is blit index *i* on the
/// decoder side.
///
/// For the lossless paths (default options, despeckle, exact and rec-6
/// matches) the shape is pixel-identical to what the decoder reconstructs;
/// only lossy rec-7 substitution (`lossy_threshold > 0`) blits a near-twin
/// whose pixels can differ from this original component.
pub struct EncodedBlit {
    /// Top-left x of the blit in the page (0 = left edge).
    pub x: u32,
    /// Top-left y of the blit in the page (0 = top edge, top-down).
    pub y: u32,
    /// Cropped component bitmap (tight bbox, this component's pixels only).
    pub bitmap: Bitmap,
}

/// Encode like [`encode_jb2_dict_with_options`] and also return the emitted
/// blits (shape + placement, in emission order).
///
/// The byte stream is identical to [`encode_jb2_dict_with_options`] — the
/// blit list is metadata the encoder already owns, handed back so callers
/// (e.g. the FGbz palette builder, #612) don't have to decode the stream
/// they just produced to recover the per-blit layout.
pub fn encode_jb2_dict_with_blits(
    bitmap: &Bitmap,
    shared_symbols: &[Bitmap],
    opts: &Jb2EncodeOptions,
) -> (Vec<u8>, Vec<EncodedBlit>) {
    let w = bitmap.width as i32;
    let h = bitmap.height as i32;
    if w == 0 || h == 0 {
        return (Vec::new(), Vec::new());
    }

    let mut ccs = extract_ccs(bitmap);

    // Despeckle pre-pass (JB2_DESPECKLE): drop isolated small components
    // *before* clustering/dedup so a speck never becomes a dict entry or a
    // coordinate record, and never dilutes the near-twin population that
    // `lossy_threshold` matches against. Filtering on `pixel_count` (true
    // ink area) rather than bbox area avoids over-crediting thin diagonal
    // strokes as "small". Lossy: the decoder has nothing left to
    // reconstruct these pixels from.
    if let Some(max_speck_px) = opts.despeckle {
        ccs.retain(|cc| cc.pixel_count > max_speck_px);
    }

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
    let mut inherit_dict_size_ctx = NumContext::new();
    // Cross-size record-6 refinement contexts (#322 experiment). Only the
    // refinement path (behind `experimental`) ever touches these. Constructing
    // a `NumContext` never touches the ZP coder, so omitting them when the
    // feature is off leaves the default (probe-off) byte stream identical.
    #[cfg(feature = "experimental")]
    let mut symbol_width_diff_ctx = NumContext::new();
    #[cfg(feature = "experimental")]
    let mut symbol_height_diff_ctx = NumContext::new();
    #[cfg(feature = "experimental")]
    let mut refinement_bitmap_ctx = vec![0u8; 2048];
    let mut hoff_ctx = NumContext::new();
    let mut voff_ctx = NumContext::new();
    let mut shoff_ctx = NumContext::new();
    let mut svoff_ctx = NumContext::new();
    let mut direct_bitmap_ctx = vec![0u8; 1024];
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

    // Exact-match dedup: symbol_hash(w, h, packed-data) → dict indices. Buckets
    // (compared against `dict_entries` on a hit) keep this byte-identical while
    // avoiding a bitmap-data clone per connected component. Pre-populated from
    // shared_symbols so cross-page identical glyphs encode as rec-7 (copy).
    let mut dedup: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    // Stored dict entries (parallel to the decoder's `dict` vector) — needed
    // so refinement matching can score Hamming distance against historical
    // glyphs. Held by reference: `shared_symbols` and this page's own `ccs`
    // both outlive the encode, so the dict never needs to own (clone) a bitmap.
    // Drops the per-page `shared_symbols` deep-copy a bundled multi-page encode
    // paid on every page for an identical shared dictionary
    // (SHARED_DICT_CLONE_PER_PAGE / swarm P2).
    let mut dict_entries: Vec<&Bitmap> = Vec::new();
    // Index of dict entries by (w, h) for O(1) lookup of refinement candidates.
    let mut by_size: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for sym in shared_symbols {
        let idx = dict_entries.len();
        dedup
            .entry(symbol_hash(sym.width, sym.height, &sym.data))
            .or_default()
            .push(idx);
        by_size
            .entry((sym.width, sym.height))
            .or_default()
            .push(idx);
        dict_entries.push(sym);
    }

    for &cc_idx in &order {
        let cc = &ccs[cc_idx];
        let cc_w = cc.bitmap.width as i32;
        let cc_h = cc.bitmap.height as i32;
        // JB2 uses bottom-up y: y_jb2 is the bottom y of the symbol.
        let x_jb2 = cc.x as i32;
        let y_jb2 = h - cc.y as i32 - cc_h;

        let dkey = symbol_hash(cc.bitmap.width, cc.bitmap.height, &cc.bitmap.data);
        let exact_match = dedup.get(&dkey).and_then(|cands| {
            cands.iter().copied().find(|&i| {
                let d = dict_entries[i];
                d.width == cc.bitmap.width
                    && d.height == cc.bitmap.height
                    && d.data == cc.bitmap.data
            })
        });

        // Choose record type:
        //   exact match → 7  (matched copy, blit only)
        //   near match  → 6  (matched refinement, blit only)
        //   otherwise   → 1  (new symbol, direct, add to dict + blit)
        enum Action {
            New,
            Copy(usize),
            /// Record-6 matched refinement — same-size (Phase A1) or cross-size
            /// (#322) experiment.
            #[cfg(feature = "experimental")]
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
            if let Some(idx) = lossy_copy {
                Action::Copy(idx)
            } else {
                // Experiment: divert fresh components with a dictionary twin to a
                // lossless rec-6 refinement. Same-size (Phase A1) is tried first —
                // no resampling, so the refinement context stays pixel-aligned —
                // then the cross-size #322 path. Behind `experimental`; the default
                // build always emits `New`.
                #[cfg(feature = "experimental")]
                {
                    let same_size = opts.same_size_rec6.and_then(|frac| {
                        find_same_size_refine_ref(&cc.bitmap, &dict_entries, candidates, frac)
                    });
                    if let Some(idx) = same_size {
                        Action::Refine(idx)
                    } else if let Some(probe) = opts.cross_size_rec6_probe {
                        match find_cross_size_refine_ref(
                            &cc.bitmap,
                            &dict_entries,
                            &by_size,
                            probe.max_dim_delta,
                            probe.max_hamming_fraction,
                        ) {
                            Some(idx) => Action::Refine(idx),
                            None => Action::New,
                        }
                    } else {
                        Action::New
                    }
                }
                #[cfg(not(feature = "experimental"))]
                {
                    Action::New
                }
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
            #[cfg(feature = "experimental")]
            Action::Refine(dict_idx) => {
                // Record type 6: matched refinement, blit only. The decoder
                // computes the child size as `dict[idx].dim + diff`, decodes the
                // refinement bitmap against that reference, then blits — it does
                // not extend the dict (handled by the `Action::New` guard below).
                let reference = dict_entries[*dict_idx];
                let wdiff = cc_w - reference.width as i32;
                let hdiff = cc_h - reference.height as i32;
                encode_num(&mut zp, &mut record_type_ctx, 0, 11, 6);
                encode_num(
                    &mut zp,
                    &mut symbol_index_ctx,
                    0,
                    (dict_size - 1) as i32,
                    *dict_idx as i32,
                );
                encode_num(&mut zp, &mut symbol_width_diff_ctx, -262143, 262142, wdiff);
                encode_num(&mut zp, &mut symbol_height_diff_ctx, -262143, 262142, hdiff);
                encode_bitmap_ref(&mut zp, &mut refinement_bitmap_ctx, &cc.bitmap, reference);
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
            dedup.entry(dkey).or_default().push(next_idx);
            by_size
                .entry((cc.bitmap.width, cc.bitmap.height))
                .or_default()
                .push(next_idx);
            dict_entries.push(&cc.bitmap);
        }
    }

    encode_num(&mut zp, &mut record_type_ctx, 0, 11, 11);
    let bytes = zp.finish();

    // Hand back the emitted blits in emission order. The bitmaps are moved
    // out of `ccs` (no clones); `dict_entries`' borrows of them end here.
    drop(dict_entries);
    let blits = order
        .iter()
        .map(|&i| {
            let cc = &mut ccs[i];
            EncodedBlit {
                x: cc.x,
                y: cc.y,
                bitmap: core::mem::replace(&mut cc.bitmap, Bitmap::new(0, 0)),
            }
        })
        .collect();
    (bytes, blits)
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

const SHARED_DICT_PIXEL_BUDGET: usize = 4 * 1024 * 1024;

/// Encode a sequence of bilevel symbols as a JB2 **Djbz** chunk payload.
///
/// Each symbol is emitted as record-type-2 (new symbol, direct, dict-only) in
/// the order given. The decoder side ([`crate::decode_dict`]) reconstructs
/// a [`crate::Jb2Dict`] whose symbol indices match this input order, so
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

/// Reference-slice variant of [`cluster_shared_symbols`] (#565): identical
/// clustering over borrowed masks, so a caller holding masks inside larger
/// per-page structs doesn't have to clone every bitmap into a contiguous
/// `Vec<Bitmap>` first.
pub fn cluster_shared_symbols_from_refs(pages: &[&Bitmap], page_threshold: usize) -> Vec<Bitmap> {
    cluster_impl(pages, page_threshold)
}

/// Same as [`cluster_shared_symbols`], preserving the old benchmarking
/// signature that accepted a per-CC Hamming allowance. The allowance is now
/// ignored: #258 showed that Hamming shared clustering can produce invalid
/// page streams on long corpora, and prior measurements found no material
/// size win over byte-exact clustering.
///
/// Provided for corpus benchmarking — most callers want
/// [`cluster_shared_symbols`].
pub fn cluster_shared_symbols_tunable(
    pages: &[Bitmap],
    page_threshold: usize,
    _diff_fraction: u32,
) -> Vec<Bitmap> {
    let refs: Vec<&Bitmap> = pages.iter().collect();
    cluster_impl(&refs, page_threshold)
}

fn cluster_impl(pages: &[&Bitmap], page_threshold: usize) -> Vec<Bitmap> {
    if page_threshold < 2 || pages.len() < page_threshold {
        return Vec::new();
    }

    struct Cluster {
        rep: Bitmap,
        pages_seen: Vec<usize>,
        first_seen: (usize, usize),
    }

    // One size class: its clusters (in creation order) plus a `symbol_hash`
    // index over their reps for O(1) exact-match lookup. Since clustering is
    // byte-exact (see below), every rep in a bucket is distinct, so at most one
    // cluster can match a candidate — the hash index replaces the O(K) linear
    // `packed_hamming` scan the old code ran per CC (CLUSTER_BUCKET_HASH_DEDUP,
    // the clustering analog of CLUSTER_DEDUP #446 for the running-dict encoder).
    #[derive(Default)]
    struct SizeBucket {
        clusters: Vec<Cluster>,
        by_hash: BTreeMap<u64, Vec<usize>>,
    }

    // Byte-exact bucketing of one page's connected components, in CC order.
    // Kept as a local item so the parallel and sequential extract paths share
    // it; visits CCs in page order to keep `first_seen`/`pages_seen` identical.
    fn bucket_page_ccs(
        buckets: &mut BTreeMap<(u32, u32), SizeBucket>,
        ccs: &[Cc],
        page_idx: usize,
    ) {
        for (cc_idx, cc) in ccs.iter().enumerate() {
            let bm = &cc.bitmap;
            // Hamming shared clustering was rejected for #258: it produced
            // invalid page streams on the 517-page corpus while providing no
            // measured size win. Clustering is byte-exact for all callers, so a
            // candidate merges only into a rep with identical bytes.
            let bucket = buckets.entry((bm.width, bm.height)).or_default();
            let hash = symbol_hash(bm.width, bm.height, &bm.data);
            // Exact match: the unique rep (if any) whose bytes equal `bm`. The
            // per-hash verify guards against `symbol_hash` collisions so the
            // result stays byte-identical to the old full-scan `best` pick.
            let hit = bucket.by_hash.get(&hash).and_then(|cands| {
                cands
                    .iter()
                    .copied()
                    .find(|&i| bucket.clusters[i].rep.data == bm.data)
            });
            match hit {
                Some(i) => {
                    // #446: pages are visited in strictly non-decreasing `page_idx`
                    // order, so `pages_seen` is sorted and the current page, if
                    // already counted, is the last element. An O(1) `last()` check
                    // replaces the O(K) `contains` scan (O(P²)→O(P) total on a corpus
                    // where a cluster recurs on many pages).
                    if bucket.clusters[i].pages_seen.last() != Some(&page_idx) {
                        bucket.clusters[i].pages_seen.push(page_idx);
                    }
                }
                None => {
                    let idx = bucket.clusters.len();
                    bucket.clusters.push(Cluster {
                        rep: bm.clone(),
                        pages_seen: vec![page_idx],
                        first_seen: (page_idx, cc_idx),
                    });
                    bucket.by_hash.entry(hash).or_default().push(idx);
                }
            }
        }
    }

    let mut buckets: BTreeMap<(u32, u32), SizeBucket> = BTreeMap::new();

    // Connected-component extraction is independent per page and is the bulk of
    // the clustering cost; the bucketing that follows is order-dependent (it
    // must visit CCs in page order to keep `first_seen`/`pages_seen` and the
    // trim-priority tie-breaks byte-identical). So extract CCs for a bounded
    // batch of pages in parallel, then bucket that batch sequentially in order.
    // Batching (rather than one big `par_iter().collect()`) caps the transient
    // CC memory to `BATCH` pages — important for long bilevel corpora (e.g. the
    // 517-page `pathogenic_bacteria_1896`) where holding every page's CCs at
    // once would be a memory regression. Output is byte-identical to the old
    // strictly-sequential extract-then-bucket loop.
    const BATCH: usize = 32;
    let mut page_idx = 0usize;
    for chunk in pages.chunks(BATCH) {
        #[cfg(feature = "parallel")]
        let ccs_batch: Vec<Vec<Cc>> = {
            use rayon::prelude::*;
            chunk.par_iter().map(|bm| extract_ccs(bm)).collect()
        };
        #[cfg(not(feature = "parallel"))]
        let ccs_batch: Vec<Vec<Cc>> = chunk.iter().map(|bm| extract_ccs(bm)).collect();

        for ccs in &ccs_batch {
            bucket_page_ccs(&mut buckets, ccs, page_idx);
            page_idx += 1;
        }
    }

    let mut promoted: Vec<Cluster> = buckets
        .into_values()
        .flat_map(|b| b.clusters)
        .filter(|c| c.pages_seen.len() >= page_threshold)
        .collect();

    // Cap cumulative pixels at the decoder's per-stream symbol budget
    // (`MAX_TOTAL_SYMBOL_PIXELS` in src/jb2.rs). Without this guard, a long
    // bilevel corpus (e.g. 517-page `pathogenic_bacteria_1896.djvu` produces
    // ~78 MP of shared symbols at threshold 2) yields a `Djbz` that
    // `decode_dictionary` then rejects with `Jb2Error::ImageTooLarge`,
    // rendering the whole bundle undecodable. See #270.
    //
    // When trimming, prefer to keep the highest-value reps: those seen on
    // more pages save more bytes per byte of shared-dict footprint. Ties on
    // page count → smaller pixel cost wins (cheaper, less likely to push us
    // back over budget on the next item).
    let mut total_pixels: u64 = 0;
    let cap = SHARED_DICT_PIXEL_BUDGET as u64;
    let any_over_budget = promoted.iter().fold(0u64, |acc, c| {
        acc + (c.rep.width as u64) * (c.rep.height as u64)
    }) > cap;
    if any_over_budget {
        let mut by_value: Vec<usize> = (0..promoted.len()).collect();
        by_value.sort_by(|&a, &b| {
            promoted[b]
                .pages_seen
                .len()
                .cmp(&promoted[a].pages_seen.len())
                .then_with(|| {
                    let pa = (promoted[a].rep.width as u64) * (promoted[a].rep.height as u64);
                    let pb = (promoted[b].rep.width as u64) * (promoted[b].rep.height as u64);
                    pa.cmp(&pb)
                })
        });
        let mut keep = vec![false; promoted.len()];
        for &i in &by_value {
            let pix = (promoted[i].rep.width as u64) * (promoted[i].rep.height as u64);
            if total_pixels + pix > cap {
                continue;
            }
            keep[i] = true;
            total_pixels += pix;
        }
        let mut idx = 0;
        promoted.retain(|_| {
            let k = keep[idx];
            idx += 1;
            k
        });
    }

    promoted.sort_by_key(|c| c.first_seen);
    promoted.into_iter().map(|c| c.rep).collect()
}

/// JB2_DICT_ORDER probe (diagnostic-only): three orderings of the same
/// byte-exact shared-symbol set [`cluster_shared_symbols`] produces, for
/// measuring whether dictionary index assignment affects `Sjbz`/`Djbz` size.
///
/// The JB2 format numbers dictionary entries by emission order — decoders
/// resolve rec-6/rec-7 references by that index, and a rec-6 refinement can
/// only reference an already-emitted entry. The whole shared block is always
/// emitted before any page-local symbol, so reordering *within* the shared
/// block alone can never violate that "reference precedes refiner" rule —
/// any permutation here is a legal encoder choice. This type exists purely to
/// feed [`encode_jb2_djbz`] / [`encode_jb2_dict_with_shared`] with different
/// shared-dict orderings for A/B size measurement; it ships no behavior
/// change (gated behind `experimental`, called from no default code path).
#[cfg(feature = "experimental")]
pub struct DictOrderVariants {
    /// Current shipped order: first-seen (page, then CC index within page).
    /// Identical to [`cluster_shared_symbols`]'s output.
    pub baseline: Vec<Bitmap>,
    /// Descending usage-frequency (number of distinct pages a symbol was
    /// promoted from), ties broken by first-seen order.
    pub by_frequency: Vec<Bitmap>,
    /// Grouped by `(width, height)` size bucket (ascending), first-seen order
    /// within each bucket — same-shaped symbols end up adjacent. This is the
    /// clustering pass's natural bucket-iteration order, before the final
    /// first-seen sort the shipped path applies.
    pub by_bucket: Vec<Bitmap>,
}

/// Build the three [`DictOrderVariants`] orderings from the same byte-exact
/// clustering pass [`cluster_shared_symbols`] runs. Mirrors that function's
/// dedup + pixel-budget trim exactly (so the *set* of promoted symbols is
/// identical to what the shipped encoder would use) and only changes what
/// happens after: instead of one fixed sort, it captures per-cluster
/// usage-count and bucket-position metadata to emit all three orderings.
#[cfg(feature = "experimental")]
pub fn cluster_shared_symbols_order_variants(
    pages: &[Bitmap],
    page_threshold: usize,
) -> DictOrderVariants {
    if page_threshold < 2 || pages.len() < page_threshold {
        return DictOrderVariants {
            baseline: Vec::new(),
            by_frequency: Vec::new(),
            by_bucket: Vec::new(),
        };
    }

    struct Cluster {
        rep: Bitmap,
        pages_seen: Vec<usize>,
        first_seen: (usize, usize),
    }

    #[derive(Default)]
    struct SizeBucket {
        clusters: Vec<Cluster>,
        by_hash: BTreeMap<u64, Vec<usize>>,
    }

    fn bucket_page_ccs(
        buckets: &mut BTreeMap<(u32, u32), SizeBucket>,
        ccs: &[Cc],
        page_idx: usize,
    ) {
        for (cc_idx, cc) in ccs.iter().enumerate() {
            let bm = &cc.bitmap;
            let bucket = buckets.entry((bm.width, bm.height)).or_default();
            let hash = symbol_hash(bm.width, bm.height, &bm.data);
            let hit = bucket.by_hash.get(&hash).and_then(|cands| {
                cands
                    .iter()
                    .copied()
                    .find(|&i| bucket.clusters[i].rep.data == bm.data)
            });
            match hit {
                Some(i) => {
                    if bucket.clusters[i].pages_seen.last() != Some(&page_idx) {
                        bucket.clusters[i].pages_seen.push(page_idx);
                    }
                }
                None => {
                    let idx = bucket.clusters.len();
                    bucket.clusters.push(Cluster {
                        rep: bm.clone(),
                        pages_seen: vec![page_idx],
                        first_seen: (page_idx, cc_idx),
                    });
                    bucket.by_hash.entry(hash).or_default().push(idx);
                }
            }
        }
    }

    let mut buckets: BTreeMap<(u32, u32), SizeBucket> = BTreeMap::new();
    const BATCH: usize = 32;
    let mut page_idx = 0usize;
    for chunk in pages.chunks(BATCH) {
        #[cfg(feature = "parallel")]
        let ccs_batch: Vec<Vec<Cc>> = {
            use rayon::prelude::*;
            chunk.par_iter().map(extract_ccs).collect()
        };
        #[cfg(not(feature = "parallel"))]
        let ccs_batch: Vec<Vec<Cc>> = chunk.iter().map(extract_ccs).collect();

        for ccs in &ccs_batch {
            bucket_page_ccs(&mut buckets, ccs, page_idx);
            page_idx += 1;
        }
    }

    let mut promoted: Vec<Cluster> = buckets
        .into_values()
        .flat_map(|b| b.clusters)
        .filter(|c| c.pages_seen.len() >= page_threshold)
        .collect();

    // Same pixel-budget trim as `cluster_shared_symbols_tunable`, so the
    // promoted *set* matches the shipped path exactly (only its order
    // differs below).
    let mut total_pixels: u64 = 0;
    let cap = SHARED_DICT_PIXEL_BUDGET as u64;
    let any_over_budget = promoted.iter().fold(0u64, |acc, c| {
        acc + (c.rep.width as u64) * (c.rep.height as u64)
    }) > cap;
    if any_over_budget {
        let mut by_value: Vec<usize> = (0..promoted.len()).collect();
        by_value.sort_by(|&a, &b| {
            promoted[b]
                .pages_seen
                .len()
                .cmp(&promoted[a].pages_seen.len())
                .then_with(|| {
                    let pa = (promoted[a].rep.width as u64) * (promoted[a].rep.height as u64);
                    let pb = (promoted[b].rep.width as u64) * (promoted[b].rep.height as u64);
                    pa.cmp(&pb)
                })
        });
        let mut keep = vec![false; promoted.len()];
        for &i in &by_value {
            let pix = (promoted[i].rep.width as u64) * (promoted[i].rep.height as u64);
            if total_pixels + pix > cap {
                continue;
            }
            keep[i] = true;
            total_pixels += pix;
        }
        let mut idx = 0;
        promoted.retain(|_| {
            let k = keep[idx];
            idx += 1;
            k
        });
    }

    // `promoted` right now is in bucket-iteration order: `(width, height)`
    // ascending (BTreeMap key order), then first-seen within each bucket
    // (creation order) — same-shaped symbols are already adjacent.
    let by_bucket: Vec<Bitmap> = promoted.iter().map(|c| c.rep.clone()).collect();

    let mut baseline_idx: Vec<usize> = (0..promoted.len()).collect();
    baseline_idx.sort_by_key(|&i| promoted[i].first_seen);
    let baseline: Vec<Bitmap> = baseline_idx
        .iter()
        .map(|&i| promoted[i].rep.clone())
        .collect();

    let mut freq_idx: Vec<usize> = (0..promoted.len()).collect();
    freq_idx.sort_by(|&a, &b| {
        promoted[b]
            .pages_seen
            .len()
            .cmp(&promoted[a].pages_seen.len())
            .then_with(|| promoted[a].first_seen.cmp(&promoted[b].first_seen))
    });
    let by_frequency: Vec<Bitmap> = freq_idx.iter().map(|&i| promoted[i].rep.clone()).collect();

    DictOrderVariants {
        baseline,
        by_frequency,
        by_bucket,
    }
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

    stats
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate as jb2;
    use djvu_bitmap::Bitmap;

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

    /// Regression for the JB2 post-EOF guard wrongly rejecting valid pages
    /// (email report, 2026-06; companion to the IW44 early-exit bug).
    ///
    /// `encode_jb2` tiles at 1024 rows, row-major. These images pack a
    /// high-entropy first tile (rows 0..1024) that drains the ZP byte buffer,
    /// followed by **multiple** solid trailing tiles (rows ≥1024). After the
    /// first solid tile consumes the last real bytes, a later solid tile's
    /// header (a large, >4096px symbol) is read while `zp.is_exhausted()` is
    /// already true — but it decodes correctly from the ~4 bytes of ZP
    /// look-ahead. The previous `is_exhausted() && pixels > 4096` guard
    /// returned `Truncated` here; the `synthetic_bytes()` guard does not,
    /// because no synthetic `0xFF` padding has actually been consumed yet.
    ///
    /// Verified to fail (decode returns `Err(Truncated)` for 2100/3100) if the
    /// guard is reverted to `is_exhausted()`.
    #[test]
    fn large_symbol_at_eof_not_wrongly_truncated() {
        for &h in &[2100u32, 3100] {
            let w = 200u32;
            let src = make_bitmap(w, h, |x, y| {
                if y < 1024 {
                    // First tile: well-mixed hash with no spatial correlation,
                    // so JB2's 10-bit context model can't compress it — this is
                    // what actually drains the byte buffer.
                    let mut s = x
                        .wrapping_mul(374761393)
                        .wrapping_add(y.wrapping_mul(668265263));
                    s = (s ^ (s >> 13)).wrapping_mul(1274126177);
                    (s ^ (s >> 16)) & 1 == 0
                } else {
                    // Solid trailing tiles: tiny compressed, large decoded.
                    true
                }
            });
            let encoded = encode_jb2(&src);
            let decoded = jb2::decode(&encoded, None)
                .unwrap_or_else(|e| panic!("{w}x{h} valid page wrongly rejected: {e:?}"));
            assert_eq!((decoded.width, decoded.height), (w, h));
            // Full pixel-exact verification: the page must decode in its
            // entirety (every tile), not be truncated mid-stream.
            for y in 0..h {
                for x in 0..w {
                    assert_eq!(
                        decoded.get(x, y),
                        src.get(x, y),
                        "{w}x{h} pixel mismatch at ({x},{y})"
                    );
                }
            }
        }
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

    // ── #322 cross-size record-6 refinement probe ────────────────────────────

    #[cfg(feature = "experimental")]
    const REC6_PROBE: CrossSizeRec6Probe = CrossSizeRec6Probe {
        max_dim_delta: 2,
        max_hamming_fraction: 0.05,
    };

    #[cfg(feature = "experimental")]
    fn probe_opts() -> Jb2EncodeOptions {
        Jb2EncodeOptions {
            cross_size_rec6_probe: Some(REC6_PROBE),
            ..Jb2EncodeOptions::default()
        }
    }

    #[test]
    fn cross_size_rec6_probe_off_is_byte_identical() {
        // With the probe disabled (default options), the option-based encoder
        // must reproduce the shipped `encode_jb2_dict` byte stream exactly.
        let src = make_bitmap(80, 40, |x, y| {
            let a = (4..16).contains(&x) && (4..28).contains(&y);
            let b = (40..53).contains(&x) && (4..28).contains(&y);
            a || b
        });
        let shipped = encode_jb2_dict(&src);
        let opt = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
        assert_eq!(shipped, opt, "default options must match shipped output");
    }

    #[cfg(feature = "experimental")]
    #[test]
    fn cross_size_rec6_probe_roundtrips_solid_near_twins() {
        // Two solid rectangles differing only in width by one pixel. The first
        // is a fresh rec-1; the second is a cross-size near twin (resampled
        // Hamming 0) so the probe diverts it to a lossless rec-6 refinement.
        let src = make_bitmap(80, 40, |x, y| {
            let a = (4..16).contains(&x) && (4..28).contains(&y); // 12×24 solid
            let b = (40..53).contains(&x) && (4..28).contains(&y); // 13×24 solid
            a || b
        });

        let default_bytes = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
        let probe_bytes = encode_jb2_dict_with_options(&src, &[], &probe_opts());

        // The probe must actually take the refinement path (different bytes)…
        assert_ne!(
            default_bytes, probe_bytes,
            "probe should emit a rec-6 refinement, changing the byte stream"
        );
        // …and stay lossless.
        let decoded = jb2::decode(&probe_bytes, None).expect("probe decode failed");
        assert_bitmaps_eq(&src, &decoded);
    }

    #[cfg(feature = "experimental")]
    #[test]
    fn cross_size_rec6_probe_roundtrips_perturbed_glyphs() {
        // A column of near-duplicate near-solid "glyphs": a base block plus a
        // few variants that differ by one bounding-box pixel and a small corner
        // notch — keeping the resampled Hamming distance under the 5% budget so
        // the probe fires, while still exercising non-trivial refinement
        // bitmaps (a handful of differing pixels, not just solid blocks).
        let mut src = Bitmap::new(64, 130);
        let draw_block = |bm: &mut Bitmap, ox: u32, oy: u32, w: u32, h: u32, notch: bool| {
            for y in 0..h {
                for x in 0..w {
                    // Solid fill minus a small 2×2 corner notch when requested.
                    if notch && x >= w - 2 && y >= h - 2 {
                        continue;
                    }
                    bm.set(ox + x, oy + y, true);
                }
            }
        };
        draw_block(&mut src, 4, 2, 14, 18, false); // reference, 14×18 solid
        draw_block(&mut src, 4, 24, 15, 18, false); // +1 width, solid
        draw_block(&mut src, 4, 46, 14, 19, true); // +1 height, corner notch
        draw_block(&mut src, 4, 70, 15, 19, true); // +1/+1, corner notch
        draw_block(&mut src, 4, 94, 13, 18, false); // −1 width, solid

        let default_bytes = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
        let probe_bytes = encode_jb2_dict_with_options(&src, &[], &probe_opts());
        assert_ne!(
            default_bytes, probe_bytes,
            "probe should fire on near twins"
        );

        let decoded = jb2::decode(&probe_bytes, None).expect("probe decode failed");
        assert_bitmaps_eq(&src, &decoded);
    }

    // ── Same-size rec-6 refinement (docs/jb2-size-gap-plan.md Phase A1) ─────────

    #[cfg(feature = "experimental")]
    fn same_size_opts() -> Jb2EncodeOptions {
        Jb2EncodeOptions {
            same_size_rec6: Some(0.05),
            ..Jb2EncodeOptions::default()
        }
    }

    #[test]
    fn lossy_text_preset_is_lossy_and_smaller() {
        // The lossy_text() preset sets the 0.02 operating point and, on a page
        // with same-size near-twin glyphs, must produce a strictly smaller stream
        // than the lossless default (it substitutes near-twins as rec-7 copies).
        assert_eq!(Jb2EncodeOptions::lossy_text().lossy_threshold, 0.02);
        assert_eq!(
            Jb2EncodeOptions::with_lossy_threshold(0.07).lossy_threshold,
            0.07
        );

        // A 14×24 solid block plus a same-size near-twin (2×2 corner notch,
        // < 2% of pixels) — the twin is within the 0.02 budget.
        let mut src = Bitmap::new(64, 60);
        for (oy, notch) in [(2u32, false), (30u32, true)] {
            for y in 0..24 {
                for x in 0..14 {
                    if notch && x >= 12 && y >= 22 {
                        continue;
                    }
                    src.set(4 + x, oy + y, true);
                }
            }
        }
        let lossless = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
        let lossy = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::lossy_text());
        assert!(
            lossy.len() < lossless.len(),
            "lossy_text should shrink a near-twin page: {} vs {}",
            lossy.len(),
            lossless.len()
        );
        // Still a valid, decodable stream.
        assert!(
            jb2::decode(&lossy, None).is_ok(),
            "lossy output must decode"
        );
    }

    // ── Despeckle pre-pass (JB2_DESPECKLE) ───────────────────────────────────

    #[test]
    fn despeckle_off_is_byte_identical() {
        // With `despeckle` unset (default: None), the encoder must reproduce
        // the shipped `encode_jb2_dict` byte stream exactly, even on a page
        // that contains isolated single-pixel specks (nothing gets filtered).
        let mut src = make_bitmap(80, 40, |x, y| {
            let a = (4..16).contains(&x) && (4..28).contains(&y);
            let b = (40..53).contains(&x) && (4..28).contains(&y);
            a || b
        });
        src.set(70, 5, true); // isolated 1px "speck"
        src.set(75, 35, true); // another isolated 1px "speck"

        let shipped = encode_jb2_dict(&src);
        let opt = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
        assert_eq!(shipped, opt, "default options must match shipped output");

        let opt_explicit_none = encode_jb2_dict_with_options(
            &src,
            &[],
            &Jb2EncodeOptions {
                despeckle: None,
                ..Jb2EncodeOptions::default()
            },
        );
        assert_eq!(shipped, opt_explicit_none);
    }

    #[test]
    fn despeckle_removes_isolated_1px_specks_and_shrinks_output() {
        // A page with one real glyph (14x24 solid block) plus five isolated
        // 1-pixel "dust" specks scattered around it. `despeckle = 2` must
        // drop every speck (pixel_count = 1 <= 2) before they ever become
        // dict entries / coordinate records, shrinking the stream, while the
        // decoded page keeps the real glyph fully intact.
        let mut src = Bitmap::new(64, 40);
        for y in 4..28 {
            for x in 4..18 {
                src.set(x, y, true);
            }
        }
        let specks = [(30u32, 2u32), (35, 10), (40, 20), (50, 5), (55, 30)];
        for &(x, y) in &specks {
            src.set(x, y, true);
        }

        let lossless = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
        let despeckled =
            encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::with_despeckle(2));
        assert!(
            despeckled.len() < lossless.len(),
            "despeckling 1px dust should shrink the stream: despeckled={} lossless={}",
            despeckled.len(),
            lossless.len()
        );

        let decoded = jb2::decode(&despeckled, None).expect("despeckled decode failed");
        assert_eq!(decoded.width, src.width);
        assert_eq!(decoded.height, src.height);
        // Every speck pixel must be gone.
        for &(x, y) in &specks {
            assert!(!decoded.get(x, y), "speck at ({x},{y}) should be removed");
        }
        // The real glyph must be pixel-exact.
        for y in 4..28 {
            for x in 4..18 {
                assert!(
                    decoded.get(x, y),
                    "glyph pixel ({x},{y}) must survive despeckle"
                );
            }
        }
    }

    #[test]
    fn despeckle_preserves_punctuation_and_diacritic_dots() {
        // Regression guard for the failure mode called out in the task: a
        // despeckle pass that is too aggressive would eat periods, commas,
        // and dots of i/j along with real dust. Build a page with:
        //   - an 'i' stem (3x20) plus its dot (4x4, separated by a gap) —
        //     dots of i/j are small but not dust-sized.
        //   - an isolated period (4x4 solid block, standing alone).
        //   - a single 1x1 dust speck far away — the thing that *should* go.
        // At despeckle=8 (the most aggressive level in the measured sweep,
        // PERF_EXPERIMENTS.md JB2_DESPECKLE), the dot and period (16 px each)
        // must survive (16 > 8) while the 1px speck (1 <= 8) is removed.
        let mut src = Bitmap::new(80, 50);
        // 'i' stem.
        for y in 10..30 {
            for x in 10..13 {
                src.set(x, y, true);
            }
        }
        // 'i' dot: 4x4 block a few pixels above the stem.
        let dot_px: Vec<(u32, u32)> = (4..8).flat_map(|y| (9..13).map(move |x| (x, y))).collect();
        for &(x, y) in &dot_px {
            src.set(x, y, true);
        }
        // Isolated period: 4x4 block, standing alone.
        let period_px: Vec<(u32, u32)> = (40..44)
            .flat_map(|y| (40..44).map(move |x| (x, y)))
            .collect();
        for &(x, y) in &period_px {
            src.set(x, y, true);
        }
        // True dust: a single isolated pixel.
        let speck = (70u32, 45u32);
        src.set(speck.0, speck.1, true);

        for max_px in [2u32, 4, 8] {
            let opts = Jb2EncodeOptions::with_despeckle(max_px);
            let enc = encode_jb2_dict_with_options(&src, &[], &opts);
            let decoded = jb2::decode(&enc, None)
                .unwrap_or_else(|e| panic!("despeckle={max_px} decode failed: {e:?}"));

            for &(x, y) in &dot_px {
                assert!(
                    decoded.get(x, y),
                    "despeckle={max_px}: i-dot pixel ({x},{y}) must survive"
                );
            }
            for &(x, y) in &period_px {
                assert!(
                    decoded.get(x, y),
                    "despeckle={max_px}: period pixel ({x},{y}) must survive"
                );
            }
            assert!(
                !decoded.get(speck.0, speck.1),
                "despeckle={max_px}: 1px dust speck must be removed"
            );
        }
    }

    #[test]
    fn lossy_scan_preset_values() {
        let preset = Jb2EncodeOptions::lossy_scan();
        assert_eq!(preset.despeckle, Some(8));
        assert_eq!(preset.lossy_threshold, 0.02);
    }

    #[test]
    fn same_size_rec6_off_is_byte_identical() {
        // With `same_size_rec6` unset (default), the encoder must reproduce the
        // shipped `encode_jb2_dict` byte stream exactly.
        let src = make_bitmap(80, 40, |x, y| {
            let a = (4..16).contains(&x) && (4..28).contains(&y);
            let b = (40..53).contains(&x) && (4..28).contains(&y);
            a || b
        });
        let shipped = encode_jb2_dict(&src);
        let opt = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
        assert_eq!(shipped, opt, "default options must match shipped output");
    }

    #[cfg(feature = "experimental")]
    #[test]
    fn same_size_rec6_roundtrips_near_twins() {
        // Two same-bounding-box near-twin glyphs: a 14×24 solid block and a copy
        // with a small 2×2 corner notch (same bbox, a few flipped pixels, well
        // under 5%). The first is a fresh rec-1; the second has a same-size twin,
        // so it diverts to a lossless same-size rec-6 refinement (wdiff=hdiff=0).
        let mut src = Bitmap::new(64, 60);
        let draw = |bm: &mut Bitmap, ox: u32, oy: u32, notch: bool| {
            for y in 0..24 {
                for x in 0..14 {
                    if notch && x >= 12 && y >= 22 {
                        continue;
                    }
                    bm.set(ox + x, oy + y, true);
                }
            }
        };
        draw(&mut src, 4, 2, false); // reference
        draw(&mut src, 4, 30, true); // same-size near twin (2×2 notch)

        let default_bytes = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
        let same_bytes = encode_jb2_dict_with_options(&src, &[], &same_size_opts());

        // The same-size path must actually fire (different byte stream)…
        assert_ne!(
            default_bytes, same_bytes,
            "same_size_rec6 should emit a rec-6 refinement, changing the stream"
        );
        // …and stay lossless (round-trip pixel-exact).
        let decoded = jb2::decode(&same_bytes, None).expect("same-size decode failed");
        assert_bitmaps_eq(&src, &decoded);
    }

    // ── JB2_AUTO_REC6: adaptive same-size rec-6 auto-policy ─────────────────────

    #[cfg(feature = "experimental")]
    fn dense_near_twin_bitmap() -> Bitmap {
        // Ten glyph pairs, each a solid (14+row)x24 block plus a same-size
        // near-twin with a small 2x2 corner notch (well under the 5% Hamming
        // budget). The width varies per row so each row's reference is a
        // distinct fresh symbol (no cross-row exact-match collisions collapsing
        // repeats into rec-7 copies) while each row's twin only near-matches
        // its own row's same-size reference. Every pair contributes one fresh
        // reference (no dict twin yet) and one fresh near-twin, so the
        // population density is 50% — far above
        // SAME_SIZE_REC6_AUTO_DENSITY_THRESHOLD.
        let mut src = Bitmap::new(600, 400);
        let draw = |bm: &mut Bitmap, ox: u32, oy: u32, w: u32, notch: bool| {
            for y in 0..24 {
                for x in 0..w {
                    if notch && x + 2 >= w && y >= 22 {
                        continue;
                    }
                    bm.set(ox + x, oy + y, true);
                }
            }
        };
        for row in 0..10u32 {
            let w = 14 + row;
            draw(&mut src, 4, 2 + row * 28, w, false); // fresh reference
            draw(&mut src, 4 + w + 6, 2 + row * 28, w, true); // fresh near-twin (same w,h)
        }
        src
    }

    #[cfg(feature = "experimental")]
    fn sparse_no_twin_bitmap() -> Bitmap {
        // Ten distinct-size solid blocks, each a different (w, h) so none has
        // a same-size dictionary twin to score against: density = 0%.
        let mut src = Bitmap::new(400, 400);
        for row in 0..10u32 {
            let w = 8 + row;
            let h = 10 + row;
            for y in 0..h {
                for x in 0..w {
                    src.set(4 + x, 2 + row * 20 + y, true);
                }
            }
        }
        src
    }

    #[cfg(feature = "experimental")]
    #[test]
    fn same_size_rec6_auto_fires_on_dense_near_twins() {
        let src = dense_near_twin_bitmap();
        let density = probe_same_size_rec6_density(&src, &[], SAME_SIZE_REC6_AUTO_SAMPLE_CCS);
        assert!(
            density >= SAME_SIZE_REC6_AUTO_DENSITY_THRESHOLD,
            "dense synthetic input should clear the auto-policy threshold: density={density}"
        );

        let opts = Jb2EncodeOptions::same_size_rec6_auto(&src, &[]);
        assert_eq!(
            opts.same_size_rec6,
            Some(SAME_SIZE_REC6_AUTO_FRAC),
            "auto-policy must enable same_size_rec6 on dense near-twin input"
        );

        // Firing must actually change (shrink or resize) the byte stream
        // relative to the lossless default and stay round-trip exact.
        let default_bytes = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
        let auto_bytes = encode_jb2_dict_with_options(&src, &[], &opts);
        assert_ne!(
            default_bytes, auto_bytes,
            "auto policy should divert near-twins to rec-6, changing the stream"
        );
        let decoded = jb2::decode(&auto_bytes, None).expect("auto-policy decode failed");
        assert_bitmaps_eq(&src, &decoded);
    }

    #[cfg(feature = "experimental")]
    #[test]
    fn same_size_rec6_auto_stays_off_on_sparse_input() {
        let src = sparse_no_twin_bitmap();
        let density = probe_same_size_rec6_density(&src, &[], SAME_SIZE_REC6_AUTO_SAMPLE_CCS);
        assert!(
            density < SAME_SIZE_REC6_AUTO_DENSITY_THRESHOLD,
            "sparse synthetic input should stay below the auto-policy threshold: density={density}"
        );

        let opts = Jb2EncodeOptions::same_size_rec6_auto(&src, &[]);
        assert_eq!(
            opts.same_size_rec6, None,
            "auto-policy must leave same_size_rec6 off on sparse input"
        );

        // Output must be byte-identical to the default encoder.
        let default_bytes = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
        let auto_bytes = encode_jb2_dict_with_options(&src, &[], &opts);
        assert_eq!(
            default_bytes, auto_bytes,
            "auto policy must stay off (byte-identical) on sparse input"
        );
    }

    #[cfg(feature = "experimental")]
    #[test]
    fn probe_same_size_rec6_density_bounds_the_scan() {
        // Bounding the probe to the first `fresh_cc_limit` fresh CCs must not
        // change the *decision* on data where the near-twin ratio is uniform
        // throughout (each pair independently contributes 1 fresh + 1 near
        // twin): scanning only the first pair already yields the same ~50%
        // density as scanning all ten.
        let src = dense_near_twin_bitmap();
        let full = same_size_refinement_scan(&src, &[], None);
        let bounded = same_size_refinement_scan(&src, &[], Some(2));
        assert!(
            bounded.fresh_ccs <= 2,
            "bounded scan must stop at the fresh-CC cap: got {}",
            bounded.fresh_ccs
        );
        assert!(
            full.fresh_ccs > bounded.fresh_ccs,
            "unbounded scan should see strictly more fresh CCs than the capped one"
        );
        // Both scans see a 50% density on this uniform-pair synthetic input.
        let full_density = full.near_le_5pct as f32 / full.fresh_ccs as f32;
        let bounded_density = bounded.near_le_5pct as f32 / bounded.fresh_ccs as f32;
        assert!((full_density - bounded_density).abs() < 1e-6);
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
    fn cluster_tunable_keeps_near_duplicate_large_glyphs_separate() {
        // Hamming clustering was rejected for #258. The tunable API remains
        // available for benchmark compatibility, but all thresholds now use
        // byte-exact clustering.
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
        assert!(
            shared.is_empty(),
            "tunable clustering must not promote noisy near-dupes"
        );

        // Default (byte-exact) keeps them separate — neither passes
        // page_threshold=2 since each variant only appears on one page.
        let shared_exact = cluster_shared_symbols(&[p1, p2], 2);
        assert!(
            shared_exact.is_empty(),
            "byte-exact default must not promote noisy near-dupes"
        );
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
                ..Jb2EncodeOptions::default()
            },
        );
        let lossy = encode_jb2_dict_with_options(
            &page,
            &[],
            &Jb2EncodeOptions {
                lossy_threshold: 0.05,
                ..Jb2EncodeOptions::default()
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
        //   2. solid 6×6 minus one pixel   → rec-1; shared rec-6 is disabled
        //   3. solid 5×5 block             → unrelated, no same-size match   (rec-1)
        //
        // REFINEMENT_MIN_PIXELS = 32 forces the 5×5 path through rec-1 even
        // if the dict had a same-size entry. The 6×6 entries (36 pixels each)
        // would otherwise be eligible for rec-6 against the shared dict.
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
            stats.rec_6_refine_shared, 0,
            "shared-dict near matches must not use rec-6"
        );
        assert_eq!(stats.rec_6_refine_local, 0);
        assert!(
            stats.rec_1_new >= 2,
            "expected near shared hit and unrelated CC to use rec-1 (got {})",
            stats.rec_1_new
        );
        assert!(stats.rec_6_hamming.is_empty());
        assert!(stats.pixels_rec_7 > 0);
        assert_eq!(stats.pixels_rec_6, 0);
        assert!(stats.pixels_rec_1 > 0);
        assert_eq!(
            stats.total_ccs,
            stats.rec_1_new
                + stats.rec_6_refine_local
                + stats.rec_6_refine_shared
                + stats.rec_7_exact
        );
    }

    #[cfg(feature = "experimental")]
    #[test]
    fn analyze_cross_size_refinement_counts_near_size_candidates() {
        let shared_glyph = make_bitmap(6, 6, |_, _| true);
        let taller_near = make_bitmap(6, 7, |_, _| true);
        let unrelated = make_bitmap(12, 12, |x, y| x == y);

        let stamp = |page: &mut Bitmap, ox: u32, oy: u32, src: &Bitmap| {
            for y in 0..src.height {
                for x in 0..src.width {
                    if src.get(x, y) {
                        page.set(ox + x, oy + y, true);
                    }
                }
            }
        };
        let mut page = make_bitmap(40, 16, |_, _| false);
        stamp(&mut page, 2, 2, &shared_glyph);
        stamp(&mut page, 14, 2, &taller_near);
        stamp(&mut page, 26, 2, &unrelated);

        let stats = analyze_jb2_cross_size_refinement(&page, &[shared_glyph], 1, 0.05);
        assert_eq!(stats.near_matches, 1);
        assert_eq!(stats.near_match_pixels, 42);
        assert!(stats.estimated_rec1_bytes > 0);
        assert!(stats.estimated_cross_size_rec6_bytes > 0);
        assert!(
            stats.candidate_ccs >= stats.near_matches,
            "near matches must be a subset of cross-size candidates"
        );
    }

    /// Regression for #270: a clustered shared dict whose total symbol pixels
    /// would exceed `MAX_TOTAL_SYMBOL_PIXELS` must be trimmed at clustering
    /// time so the produced `Djbz` round-trips through `decode_dict`.
    #[test]
    fn cluster_shared_symbols_caps_total_pixel_budget() {
        let cap = SHARED_DICT_PIXEL_BUDGET;

        // Build many distinct same-size CCs, then stamp each twice (across two
        // pages) so they all promote at threshold 2. Sum > cap.
        let glyph_w: u32 = 96;
        let glyph_h: u32 = 96;
        let pixels_per_glyph = (glyph_w as usize) * (glyph_h as usize);
        let n_glyphs = (cap / pixels_per_glyph) + 64; // overshoot the cap

        let glyphs: Vec<Bitmap> = (0..n_glyphs)
            .map(|i| {
                make_bitmap(glyph_w, glyph_h, |x, y| {
                    // Per-glyph pseudo-random pattern; ensures buckets are
                    // populated by distinct (w, h, data) reps.
                    let v =
                        (x.wrapping_mul(2654435761) ^ y.wrapping_mul(40503)).wrapping_add(i as u32);
                    (v & 0xff) < 128
                })
            })
            .collect();

        let page_w: u32 = 1024;
        let make_page = |start: usize, count: usize| -> Bitmap {
            // Pack glyphs in rows; canvas grows just enough to fit.
            let cols = (page_w / (glyph_w + 2)).max(1) as usize;
            let rows = count.div_ceil(cols);
            let canvas_h = (rows as u32) * (glyph_h + 2) + 2;
            let mut canvas = Bitmap::new(page_w, canvas_h);
            for (i, g) in glyphs[start..start + count].iter().enumerate() {
                let col = (i % cols) as u32;
                let row = (i / cols) as u32;
                let ox = col * (glyph_w + 2) + 1;
                let oy = row * (glyph_h + 2) + 1;
                for y in 0..glyph_h {
                    for x in 0..glyph_w {
                        if g.get(x, y) {
                            canvas.set(ox + x, oy + y, true);
                        }
                    }
                }
            }
            canvas
        };
        let p1 = make_page(0, n_glyphs);
        let p2 = make_page(0, n_glyphs);

        let shared = cluster_shared_symbols_tunable(&[p1, p2], 2, 0);
        let total: usize = shared
            .iter()
            .map(|s| (s.width as usize) * (s.height as usize))
            .sum();
        assert!(
            total <= cap,
            "cluster output {total} px must respect MAX_TOTAL_SYMBOL_PIXELS={cap}"
        );

        let djbz = encode_jb2_djbz(&shared);
        crate::decode_dict(&djbz, None)
            .expect("encoded shared Djbz must round-trip through decode_dict");
    }
}
