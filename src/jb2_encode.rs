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
/// The entire image is encoded as a single direct-bitmap record (type 3).
/// No connected-component analysis or symbol dictionary is used.
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

    // ── Single direct-bitmap record ────────────────────────────────────────
    // Record type 3: new symbol, direct, blit to page, NOT stored in dict.
    encode_num(&mut zp, &mut record_type_ctx, 0, 11, 3);

    // Symbol dimensions.
    encode_num(&mut zp, &mut symbol_width_ctx, 0, 262142, w);
    encode_num(&mut zp, &mut symbol_height_ctx, 0, 262142, h);

    // Bitmap data.
    encode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, bitmap);

    // Coordinates: new_line=true, hoff=1, voff=0.
    //
    // Decoder initial state: first_left=-1, first_bottom=image_height-1.
    // new_line=true:
    //   x = first_left + hoff = -1 + 1 = 0
    //   y = first_bottom + voff - h + 1 = (image_height-1) + 0 - h + 1 = 0
    // So the symbol lands at (0, 0) — bottom-left of the JB2 page. ✓
    zp.encode_bit(&mut offset_type_ctx, true); // new_line = true
    encode_num(&mut zp, &mut hoff_ctx, -262143, 262142, 1);
    encode_num(&mut zp, &mut voff_ctx, -262143, 262142, 0);

    // ── End-of-data ────────────────────────────────────────────────────────
    encode_num(&mut zp, &mut record_type_ctx, 0, 11, 11);

    zp.finish()
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
}
