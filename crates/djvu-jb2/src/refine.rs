//! Refinement bitmap decoding with the 11-bit context.

use super::*;

// ────────────────────────────────────────────────────────────────────────────
// Refinement bitmap decode: 11-bit context
// ────────────────────────────────────────────────────────────────────────────

/// Decode a bitmap using the refinement (11-pixel context) method.
///
/// The new (child) bitmap `cbm` is decoded relative to a reference (matched)
/// bitmap `mbm`. Center alignment is used per the DjVu spec.
pub(super) fn decode_bitmap_ref(
    zp: &mut ZpDecoder<'_>,
    ctx: &mut [u8; 2048],
    ctx_p: &mut [u16; 2048],
    width: i32,
    height: i32,
    mbm: &Jbm,
    pool: &mut Vec<u8>,
) -> Result<Jbm, Jb2Error> {
    let pixels = (width.max(0) as usize).saturating_mul(height.max(0) as usize);
    if pixels > MAX_SYMBOL_PIXELS {
        return Err(Jb2Error::ImageTooLarge);
    }
    if width <= 0 || height <= 0 {
        return Ok(Jbm::new_from_pool(width, height, pool));
    }
    let mut cbm = Jbm::new_from_pool(width, height, pool);

    // Center alignment: anchor the reference bitmap at the center of the child.
    let crow = (height - 1) >> 1;
    let ccol = (width - 1) >> 1;
    let mrow = (mbm.height - 1) >> 1;
    let mcol = (mbm.width - 1) >> 1;
    let row_shift = mrow - crow;
    let col_shift = mcol - ccol;

    // Access a pre-sliced row at a possibly-negative column index; returns 0 for OOB.
    let pix_row = |row_slice: &[u8], col: i32| -> u32 {
        if col < 0 {
            return 0;
        }
        row_slice.get(col as usize).copied().unwrap_or(0) as u32
    };

    let cw = width as usize;
    let cstride = cbm.stride();
    let mw = mbm.width.max(0) as usize;
    let mstride = mbm.stride();

    // Rolling scratch (1 byte/pixel) for the three mbm reference rows.
    // Each slot holds the unpacked content of rows `mr+1`, `mr`, `mr-1` relative
    // to the current iteration's `mr = row + row_shift`. Empty slice when OOB.
    let mut s_mbm_r2 = vec![0u8; mw];
    let mut s_mbm_r1 = vec![0u8; mw];
    let mut s_mbm_r0 = vec![0u8; mw];
    let mut have_r2;
    let mut have_r1;
    let mut have_r0;

    // Scratch for cbm: current row being decoded, and previously-decoded row.
    let mut s_cbm_curr = vec![0u8; cw];
    let mut s_cbm_prev1 = vec![0u8; cw];

    let unpack_mbm_row = |r: i32, buf: &mut [u8]| -> bool {
        if r < 0 || r >= mbm.height || mw == 0 {
            return false;
        }
        let off = r as usize * mstride;
        unpack_row_into(&mbm.data[off..off + mstride], mw, buf);
        true
    };

    // Prime the rolling mbm scratch before the first iteration (row = height-1):
    // mbm_r2 = mbm[mr+1], mbm_r1 = mbm[mr], mbm_r0 = mbm[mr-1], with mr = (height-1) + row_shift.
    let first_mr = (height - 1) + row_shift;
    have_r2 = unpack_mbm_row(first_mr + 1, &mut s_mbm_r2);
    have_r1 = unpack_mbm_row(first_mr, &mut s_mbm_r1);
    have_r0 = unpack_mbm_row(first_mr - 1, &mut s_mbm_r0);

    for row in (0..height).rev() {
        let mr = row + row_shift;

        // Empty slice when the row is OOB (matches previous behaviour).
        let mbm_r2: &[u8] = if have_r2 { &s_mbm_r2 } else { &[] };
        let mbm_r1: &[u8] = if have_r1 { &s_mbm_r1 } else { &[] };
        let mbm_r0: &[u8] = if have_r0 { &s_mbm_r0 } else { &[] };

        let cbm_r1: &[u8] = if row + 1 < height { &s_cbm_prev1 } else { &[] };
        s_cbm_curr.iter_mut().for_each(|b| *b = 0);

        let init_c_r1 = pix_row(cbm_r1, 0) << 1 | pix_row(cbm_r1, 1);
        let init_m_r1 = pix_row(mbm_r1, col_shift - 1) << 2
            | pix_row(mbm_r1, col_shift) << 1
            | pix_row(mbm_r1, col_shift + 1);
        let init_m_r0 = pix_row(mbm_r0, col_shift - 1) << 2
            | pix_row(mbm_r0, col_shift) << 1
            | pix_row(mbm_r0, col_shift + 1);

        decode_ref_row(
            zp,
            ctx,
            ctx_p,
            &mut s_cbm_curr,
            cbm_r1,
            mbm_r2,
            mbm_r1,
            mbm_r0,
            col_shift,
            init_c_r1,
            init_m_r1,
            init_m_r0,
        );

        // Pack current cbm row into storage.
        pack_row_into(
            &s_cbm_curr,
            cw,
            &mut cbm.data[row as usize * cstride..(row as usize + 1) * cstride],
        );

        // Rotate cbm scratch: prev1 ← curr, curr ← (old prev1, reused next iteration).
        core::mem::swap(&mut s_cbm_prev1, &mut s_cbm_curr);

        // Rotate mbm scratch: r2 ← r1, r1 ← r0, r0 ← freshly unpacked mr-2.
        //   After this, new mr = mr-1, so:
        //     new r2 = mbm[new mr + 1]   = mbm[mr]       = old r1
        //     new r1 = mbm[new mr]       = mbm[mr-1]     = old r0
        //     new r0 = mbm[new mr - 1]   = mbm[mr-2]     = needs unpack
        core::mem::swap(&mut s_mbm_r2, &mut s_mbm_r1);
        have_r2 = have_r1;
        core::mem::swap(&mut s_mbm_r1, &mut s_mbm_r0);
        have_r1 = have_r0;
        have_r0 = unpack_mbm_row(mr - 2, &mut s_mbm_r0);
    }
    Ok(cbm)
}
