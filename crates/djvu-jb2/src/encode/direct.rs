//! The single-record direct encoder, `encode_jb2`.

use super::*;

/// Encode a bilevel [`Bitmap`] into a JB2 stream (Sjbz chunk payload).
///
/// The returned bytes can be embedded directly in a `Sjbz` IFF chunk.
/// Decoding with [`crate::decode`] will reconstruct the original bitmap.
///
/// ## Encoding
///
/// Images with `width * height ≤ 1 MP` are emitted as a single direct-bitmap
/// record (type 3). Larger images are split into ≤ 1024×1024 tiles, each
/// emitted as its own record-3. Every tile stays within the decoder's
/// 16 MP per-symbol guard, and the tiles together cost one unit per page
/// pixel in its 64 M per-page work budget, so the output round-trips through
/// [`crate::decode`] for any size up to the 64 MP page limit.
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
    // Tile size: 1024, so tile_w*tile_h ≤ 1 MP — well under the decoder's
    // 16 MP MAX_SYMBOL_PIXELS (the tile size predates that limit). Layout state mirrors the decoder (jb2.rs:LayoutState):
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
pub(super) fn crop_bitmap(src: &Bitmap, x0: u32, y0: u32, w: u32, h: u32) -> Bitmap {
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
