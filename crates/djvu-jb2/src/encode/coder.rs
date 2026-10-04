//! ZP coding primitives: integers, direct and refinement bitmaps.

use super::*;

/// Encode integer `val` in `[low, high]` using the same binary-tree traversal
/// as the decoder's `decode_num`.
///
/// Emits a ZP bit at each "free" decision point (where neither `low >= cutoff`
/// nor `high < cutoff`).  Forced decisions traverse the tree without emitting.
pub(super) fn encode_num(zp: &mut ZpEncoder, ctx: &mut NumContext, low: i32, high: i32, val: i32) {
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

/// Encode a bitmap using the direct 10-pixel-context method.
///
/// Mirrors `decode_bitmap_direct` in `jb2` exactly.  Iterates rows
/// top-to-bottom, which corresponds to Bitmap y = 0 (top) up to height-1 (bottom).
///
/// The bitmap is first expanded to a flat byte-per-pixel array with 2 zero rows
/// above the image and 4 zero columns to the right of each row.  This eliminates
/// all per-pixel bounds checking and bit-manipulation in the inner loop.
#[allow(unsafe_code)]
pub(super) fn encode_bitmap_direct(zp: &mut ZpEncoder, ctx: &mut [u8], bm: &Bitmap) {
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
        let (chunks, tail) = dst.as_chunks_mut::<8>();
        for (&byte, chunk) in src.iter().zip(chunks) {
            chunk[0] = (byte >> 7) & 1;
            chunk[1] = (byte >> 6) & 1;
            chunk[2] = (byte >> 5) & 1;
            chunk[3] = (byte >> 4) & 1;
            chunk[4] = (byte >> 3) & 1;
            chunk[5] = (byte >> 2) & 1;
            chunk[6] = (byte >> 1) & 1;
            chunk[7] = byte & 1;
        }
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

        let mut col = 0;
        while col < w {
            let idx = ((r2 << 7) | (r1 << 2) | r0) as usize;
            let bit = row_cur[col] != 0;
            if idx == 0 && !bit {
                // White on white: every pixel of the run codes `false` in
                // context 0, so the ZP coder takes it as one run.
                let n = white_run(row_cur, row_p1, row_p2, col, w);
                // Context 0 at `col` means the first pixel already qualifies.
                debug_assert!(n >= 1);
                zp.encode_run(&mut ctx[0], false, n);
                col += n;
                // The run leaves zeros in every window bit except the
                // newest one of the two rows above.
                r2 = row_p2[col + 1] as u32;
                r1 = row_p1[col + 2] as u32;
                r0 = 0;
                continue;
            }
            // Safety: r2 ≤ 7, r1 ≤ 31, r0 ≤ 3 by the & masks above,
            // so idx ≤ (7<<7)|(31<<2)|3 = 1023 < ctx.len() = 1024.
            let ctx_byte = unsafe { ctx.get_unchecked_mut(idx) };
            zp.encode_bit(ctx_byte, bit);

            // Advance rolling windows — no bounds checks: col+2 < w+2 < pw, col+3 < w+3 < pw.
            r2 = ((r2 << 1) & 0b111) | row_p2[col + 2] as u32;
            r1 = ((r1 << 1) & 0b11111) | row_p1[col + 3] as u32;
            r0 = ((r0 << 1) & 0b11) | bit as u32;
            col += 1;
        }
    }
}

/// Length of the white run that starts at `col`, where the context is 0 and
/// the pixel is white: pixel `col + j` keeps both while it, the pixel at
/// `col + j + 1` in the row above-above, and the one at `col + j + 2` in the
/// row above are white. Rows carry 4 zero padding columns past `w`.
fn white_run(cur: &[u8], p1: &[u8], p2: &[u8], col: usize, w: usize) -> usize {
    let mut j = col;
    // Eight byte-per-pixel columns per step; the last read is p1[j + 9] < w + 2.
    while j + 8 <= w {
        let word = |row: &[u8], at: usize| {
            u64::from_le_bytes(row[at..at + 8].try_into().expect("8 bytes"))
        };
        let any = word(cur, j) | word(p2, j + 1) | word(p1, j + 2);
        if any != 0 {
            return j + (any.trailing_zeros() / 8) as usize - col;
        }
        j += 8;
    }
    while j < w && (cur[j] | p2[j + 1] | p1[j + 2]) == 0 {
        j += 1;
    }
    j - col
}

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
pub(super) fn encode_bitmap_ref(zp: &mut ZpEncoder, ctx: &mut [u8], cbm: &Bitmap, mbm: &Bitmap) {
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
