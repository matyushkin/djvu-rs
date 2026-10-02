//! Blitting symbols onto the page and converting the page buffer to a `Bitmap`.

use super::*;

// ────────────────────────────────────────────────────────────────────────────
// Blit a symbol onto the page (OR compositing, bottom-left origin)
// ────────────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub(super) fn blit_indexed(
    page: &mut [u8],
    blit_map: &mut [i32],
    page_w: i32,
    page_h: i32,
    symbol: &Jbm,
    x: i32,
    y: i32,
    blit_idx: i32,
) {
    // Guard: negative/zero dimensions would wrap `width as usize` to a huge value
    // in the fast-path loop, causing an effectively infinite iteration count.
    if symbol.width <= 0 || symbol.height <= 0 {
        return;
    }
    if x >= 0 && y >= 0 && x + symbol.width <= page_w && y + symbol.height <= page_h {
        let pw = page_w as usize;
        let sw = symbol.width as usize;
        let sym_stride = symbol.stride();
        let full_bytes = sw / 8;
        let rem = sw & 7;
        for row in 0..symbol.height as usize {
            let src_row_off = row * sym_stride;
            let dst_off = (y as usize + row) * pw + x as usize;
            for byte_i in 0..full_bytes {
                let b = symbol.data[src_row_off + byte_i];
                if b == 0 {
                    continue;
                }
                let base_col = byte_i * 8;
                for j in 0..8 {
                    if (b >> (7 - j)) & 1 != 0 {
                        page[dst_off + base_col + j] = 1;
                        blit_map[dst_off + base_col + j] = blit_idx;
                    }
                }
            }
            if rem > 0 {
                let b = symbol.data[src_row_off + full_bytes];
                if b != 0 {
                    let base_col = full_bytes * 8;
                    for j in 0..rem {
                        if (b >> (7 - j)) & 1 != 0 {
                            page[dst_off + base_col + j] = 1;
                            blit_map[dst_off + base_col + j] = blit_idx;
                        }
                    }
                }
            }
        }
    } else {
        for row in 0..symbol.height {
            let py = y + row;
            if py < 0 || py >= page_h {
                continue;
            }
            for col in 0..symbol.width {
                if symbol.get(row, col) != 0 {
                    let px = x + col;
                    if px >= 0 && px < page_w {
                        let idx = (py * page_w + px) as usize;
                        page[idx] = 1;
                        blit_map[idx] = blit_idx;
                    }
                }
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Blit symbol directly into a packed Bitmap (no intermediate byte-per-pixel buffer)
// ────────────────────────────────────────────────────────────────────────────

/// Blit a symbol into a packed Bitmap with JB2→bitmap coordinate flip.
///
/// JB2 uses y=0 at the bottom; `Bitmap` uses y=0 at the top.
/// Both source (`Jbm`) and destination (`Bitmap`) are 1-bit-per-pixel,
/// MSB-first within byte, byte-aligned rows. Fast path is a shift-align
/// byte OR; no bit packing needed.
pub(super) fn blit_to_bitmap(bm: &mut Bitmap, sym: &Jbm, x: i32, y: i32) {
    if sym.width <= 0 || sym.height <= 0 {
        return;
    }
    let bw = bm.width as i32;
    let bh = bm.height as i32;
    let bm_stride = bm.row_stride();
    let sw = sym.width;
    let sh = sym.height;
    let sym_stride = sym.stride();

    // Fast path: symbol completely within bitmap bounds.
    if x >= 0
        && y >= 0
        && x.checked_add(sw).is_some_and(|v| v <= bw)
        && y.checked_add(sh).is_some_and(|v| v <= bh)
    {
        let x_off = x as usize;
        let byte_off = x_off / 8;
        let bit_off = x_off & 7;
        let sw_u = sw as usize;
        let full = sw_u / 8;
        let rem = sw_u & 7;
        let bm_y_base = (bm.height as usize) - 1 - y as usize;

        if bit_off == 0 {
            for sym_row in 0..sh as usize {
                let bm_y = bm_y_base - sym_row;
                let src = &sym.data[sym_row * sym_stride..sym_row * sym_stride + sym_stride];
                let dst = &mut bm.data[bm_y * bm_stride..];
                for i in 0..full {
                    dst[byte_off + i] |= src[i];
                }
                if rem > 0 {
                    // Last byte of packed source: its high `rem` bits are valid
                    // pixels; low `8 - rem` bits are padding (guaranteed 0 by
                    // construction), so OR-ing the whole byte is correct.
                    dst[byte_off + full] |= src[full];
                }
            }
        } else {
            let rshift = bit_off as u32;
            let lshift = 8 - bit_off as u32;
            for sym_row in 0..sh as usize {
                let bm_y = bm_y_base - sym_row;
                let src = &sym.data[sym_row * sym_stride..sym_row * sym_stride + sym_stride];
                let row_off = bm_y * bm_stride;
                for (i, &s) in src.iter().enumerate().take(full) {
                    bm.data[row_off + byte_off + i] |= s >> rshift;
                    bm.data[row_off + byte_off + i + 1] |= s << lshift;
                }
                if rem > 0 {
                    let s = src[full];
                    bm.data[row_off + byte_off + full] |= s >> rshift;
                    let overflow = row_off + byte_off + full + 1;
                    if overflow < bm.data.len() {
                        bm.data[overflow] |= s << lshift;
                    }
                }
            }
        }
    } else {
        // Slow path: clipped blit, per-pixel bounds checks.
        for sym_row in 0..sh {
            let bm_y = bh - 1 - y - sym_row;
            if bm_y < 0 || bm_y >= bh {
                continue;
            }
            let bm_y = bm_y as usize;
            let row_off = bm_y * bm_stride;
            let src_row_off = sym_row as usize * sym_stride;
            for col in 0..sw {
                let b = sym.data[src_row_off + (col as usize / 8)];
                if (b >> (7 - (col as usize & 7))) & 1 != 0 {
                    let px = x + col;
                    if px >= 0 && px < bw {
                        let px = px as usize;
                        bm.data[row_off + px / 8] |= 0x80u8 >> (px & 7);
                    }
                }
            }
        }
    }
}

/// Blit a symbol into a `1/2^shift`-resolution packed Bitmap, OR-reducing
/// (max-pooling) each set source pixel into its downsampled destination cell.
///
/// Mirrors [`blit_to_bitmap`]'s coordinate flip (JB2 y=0 at the bottom,
/// `Bitmap` y=0 at the top) but works in the full-resolution coordinate space
/// per source pixel — `full_w`/`full_h` are the *undownsampled* page
/// dimensions — then right-shifts by `shift` to land in the smaller `bm`.
/// Unlike [`blit_to_bitmap`] there is no byte-aligned fast path: downsampled
/// destination columns/rows generally don't stay byte-aligned across a
/// symbol's width, so this always walks bit-by-bit. That is still cheap in
/// practice — the loop is bounded by the symbol's own (typically small) area,
/// not by the page canvas, matching the existing clipped/slow path of
/// [`blit_to_bitmap`].
pub(super) fn blit_to_bitmap_downsampled(
    bm: &mut Bitmap,
    sym: &Jbm,
    x: i32,
    y: i32,
    full_w: i32,
    full_h: i32,
    shift: u32,
) {
    if sym.width <= 0 || sym.height <= 0 {
        return;
    }
    let dst_w = bm.width as i32;
    let dst_h = bm.height as i32;
    let bm_stride = bm.row_stride();
    let sw = sym.width;
    let sh = sym.height;
    let sym_stride = sym.stride();

    for sym_row in 0..sh {
        // JB2 row 0 = bottom of the page; `Bitmap` row 0 = top, matching
        // `blit_to_bitmap`'s `bm.height - 1 - y` flip before downsampling.
        let full_y_top = full_h - 1 - (y + sym_row);
        if full_y_top < 0 || full_y_top >= full_h {
            continue;
        }
        let dy = full_y_top >> shift;
        if dy >= dst_h {
            continue;
        }
        let row_off = dy as usize * bm_stride;
        let src_row_off = sym_row as usize * sym_stride;
        for col in 0..sw {
            let b = sym.data[src_row_off + (col as usize / 8)];
            if (b >> (7 - (col as usize & 7))) & 1 == 0 {
                continue;
            }
            let full_x = x + col;
            if full_x < 0 || full_x >= full_w {
                continue;
            }
            let dx = full_x >> shift;
            if dx >= dst_w {
                continue;
            }
            bm.data[row_off + dx as usize / 8] |= 0x80u8 >> (dx as usize & 7);
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Convert internal page buffer (row 0 = bottom) to Bitmap (row 0 = top)
// ────────────────────────────────────────────────────────────────────────────

/// Pack a single byte: each of the 8 input bytes (0 or 1) into one output byte.
/// Bit 7 = src[0], bit 6 = src[1], …, bit 0 = src[7].
#[inline(always)]
pub(super) fn pack_byte(s: &[u8; 8]) -> u8 {
    ((s[0] != 0) as u8) << 7
        | ((s[1] != 0) as u8) << 6
        | ((s[2] != 0) as u8) << 5
        | ((s[3] != 0) as u8) << 4
        | ((s[4] != 0) as u8) << 3
        | ((s[5] != 0) as u8) << 2
        | ((s[6] != 0) as u8) << 1
        | ((s[7] != 0) as u8)
}

pub(super) fn page_to_bitmap(page: &[u8], width: i32, height: i32) -> Bitmap {
    let w = width as usize;
    let h = height as usize;
    let mut bm = Bitmap::new(width as u32, height as u32);
    let stride = bm.row_stride();
    let full_bytes = w / 8;
    let remaining = w % 8;

    for row in 0..h {
        let src_row = &page[row * w..(row + 1) * w];
        let dst_y = h - 1 - row; // flip: JB2 row 0=bottom → PBM row 0=top
        let dst_off = dst_y * stride;

        // Process 8 source bytes → 1 packed byte.
        // The fixed-size array slice tells LLVM the chunk is exactly 8 bytes,
        // allowing it to vectorize the comparison+shift tree.
        for byte_idx in 0..full_bytes {
            let s: &[u8; 8] = src_row[byte_idx * 8..(byte_idx + 1) * 8]
                .try_into()
                .unwrap();
            bm.data[dst_off + byte_idx] = pack_byte(s);
        }

        // Partial last byte (< 8 pixels).
        if remaining > 0 {
            let base = full_bytes * 8;
            let mut byte_val = 0u8;
            for bit_pos in 0..remaining {
                if src_row[base + bit_pos] != 0 {
                    byte_val |= 0x80u8 >> bit_pos;
                }
            }
            bm.data[dst_off + full_bytes] = byte_val;
        }
    }
    bm
}

/// Flip blit_map vertically to match bitmap coordinate system (bottom→top).
pub(super) fn flip_blit_map(blit_map: &mut [i32], width: usize, height: usize) {
    for row in 0..height / 2 {
        let mirror = height - 1 - row;
        let a = row * width;
        let b = mirror * width;
        for col in 0..width {
            blit_map.swap(a + col, b + col);
        }
    }
}
