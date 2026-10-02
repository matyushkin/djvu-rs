//! Direct bitmap decoding with the 10-bit context, and the decoder budgets.

use super::*;

// ────────────────────────────────────────────────────────────────────────────
// Direct bitmap decode: 10-bit context
// ────────────────────────────────────────────────────────────────────────────

/// Decode a bitmap using the direct (10-pixel context) method.
///
/// Decodes top-to-bottom using an incremental rolling window that avoids
/// recomputing all 10 context bits from scratch each pixel.
pub(super) const MAX_SYMBOL_PIXELS: usize = 16 * 1024 * 1024; // 16 MP per symbol — allows large connected components while bounding DoS input
// Post-EOF spin guard. The ZP coder buffers up to 32 bits (4 bytes) of
// look-ahead, so its byte buffer drains a few bytes *before* the logical end of
// a valid stream: `zp.is_exhausted()` (pos ≥ len) flips while the final symbol
// still decodes legitimately from buffered bits, and no synthetic padding has
// been read yet. `synthetic_bytes()` counts only genuine post-EOF `0xFF` fill,
// so it stays 0 through a valid tail and climbs without bound once the stream
// is exhausted and spinning. Allowing this many synthetic bytes covers the
// look-ahead flush (≈4–8 bytes) with margin; beyond it every remaining bit is
// fill, so we stop. A single oversized symbol decoded within the slack window
// is still capped by `check_pixel_budget` (16 MP/symbol, 256 MP total).
//
// Using `zp.is_exhausted()` here instead wrongly rejects valid pages whose last
// symbol is larger than a few KB and finishes right at EOF (reported 2026-06).
pub(super) const ZP_EOF_SLACK_BYTES: usize = 16;
// 256 MP cumulative decoded-symbol work. Dense JB2 pages can contain many
// direct or refinement records whose individual symbols are valid and whose
// blit work is bounded separately below; 64 MP was too low for the
// `pathogenic_bacteria_1896.djvu` corpus (#258).
pub(crate) const MAX_TOTAL_SYMBOL_PIXELS: usize = 256 * 1024 * 1024;
// Per-PAGE cumulative decoded-symbol work (Sjbz). A page mask is at most a few MP
// of foreground; even dense pages with refinement records stay well under this.
// The 256 MP ceiling above is needed for the cross-page *shared dictionary* (Djbz,
// #258) but is far too loose for a single page: a crafted sub-1 KB Sjbz of matched-
// refinement records can otherwise decode ~48 MP (≈0.6 s native, a libFuzzer
// timeout under ASAN). Bounding per-page symbol work stops that amplification while
// leaving the dictionary path on the higher ceiling.
//
// 16 MP, not 32: the corpus's densest page needs >8 MP but <16 MP, and at 32 MP the
// fuzz_jb2 regression seed still decoded ~6 s under ASAN — close enough to the 10 s
// libFuzzer per-input timeout to flake intermittently on slow CI runners. 16 MP
// halves that worst case (~3-5 s) for a comfortable margin while still accepting
// every real page.
//
// The budget is counted in work units, not pixels. A refinement pixel (records
// 4–6) costs `REFINE_PIXEL_WORK` units and a direct pixel (records 1–3, 8) one
// unit: on low-entropy input, where ZP decodes from buffered bits without
// renormalizing, refinement decodes at ≈ 15 ns/px and direct at ≈ 4 ns/px
// (native). So 16 MP of refinement — the fuzz worst case above — and 64 MP of
// direct records cost about the same time. The direct allowance lets a page up
// to the 64 MP page limit be coded as direct tiles: a 62 MP scan otherwise failed
// with `ImageTooLarge`. Any stream that passed the old 16 MP pixel cap still passes.
pub(crate) const MAX_PAGE_SYMBOL_WORK: usize = 64 * 1024 * 1024;
/// Work units per decoded refinement pixel; see [`MAX_PAGE_SYMBOL_WORK`].
pub(crate) const REFINE_PIXEL_WORK: usize = 4;
pub(super) const MAX_TOTAL_BLIT_PIXELS: usize = 256 * 1024 * 1024; // 256 MP total blit work — prevents type-7 DoS
pub(super) const MAX_RECORDS: usize = 65_536; // 64 K records per stream — prevents DoS via record-loop spin on exhausted ZP input

/// Check that decoding a `w × h` symbol won't exceed per-symbol or stream-total budgets.
///
/// The stream total grows by `pixels × work` (`work` = 1 for direct records,
/// [`REFINE_PIXEL_WORK`] for refinement records on page streams).
#[inline(always)]
pub(super) fn check_pixel_budget(
    w: i32,
    h: i32,
    work: usize,
    total: &mut usize,
    max_total: usize,
) -> Result<(), Jb2Error> {
    let pixels = (w.max(0) as usize).saturating_mul(h.max(0) as usize);
    if pixels > MAX_SYMBOL_PIXELS {
        return Err(Jb2Error::ImageTooLarge);
    }
    *total = total.saturating_add(pixels.saturating_mul(work));
    // Reject *before* the caller decodes the bitmap, so the running total is a
    // hard ceiling — not "the cap plus one more symbol". `max_total` is the
    // per-page cap on page streams, the higher dictionary ceiling on Djbz.
    if *total > max_total {
        return Err(Jb2Error::ImageTooLarge);
    }
    Ok(())
}

#[inline(always)]
pub(super) fn check_symbol_decode_budget(
    zp: &ZpDecoder<'_>,
    w: i32,
    h: i32,
    work: usize,
    total: &mut usize,
    max_total: usize,
) -> Result<(), Jb2Error> {
    check_pixel_budget(w, h, work, total, max_total)?;
    // Once the ZP coder has emitted more synthetic `0xFF` padding than the
    // look-ahead slack, every remaining bit is provably fill: real input is
    // exhausted and we are spinning. Bail immediately — the in-window symbol we
    // were about to decode is still size-capped by `check_pixel_budget` above.
    if zp.synthetic_bytes() > ZP_EOF_SLACK_BYTES {
        return Err(Jb2Error::Truncated);
    }
    Ok(())
}

/// Check that blitting a symbol won't exceed the total blit-work budget.
///
/// Prevents DoS via repeated blitting of a large dict symbol (type 7 / matched copy)
/// which has no decode cost but O(w×h) blit cost per record.
#[inline(always)]
pub(super) fn check_blit_budget(sym: &Jbm, total: &mut usize) -> Result<(), Jb2Error> {
    let pixels = (sym.width.max(0) as usize).saturating_mul(sym.height.max(0) as usize);
    *total = total.saturating_add(pixels);
    if *total > MAX_TOTAL_BLIT_PIXELS {
        return Err(Jb2Error::ImageTooLarge);
    }
    Ok(())
}
/// Decode one row of a direct-mode JB2 bitmap with inline ZP arithmetic.
///
/// Extracts the five hot ZP fields to true stack-locals so LLVM keeps them
/// in registers throughout the row without spilling through the struct pointer.
#[inline(never)]
pub(super) fn decode_direct_row(
    zp: &mut ZpDecoder<'_>,
    ctx: &mut [u8; 1024],
    row_slice: &mut [u8],
    rp1: &[u8],
    rp2: &[u8],
) {
    use djvu_zp::tables::{LPS_NEXT, MPS_NEXT, PROB, THRESHOLD};

    let mut a: u32 = zp.a;
    let mut c: u32 = zp.c;
    let mut fence: u32 = zp.fence;
    let mut bit_buf = zp.bit_buf;
    let mut bit_count = zp.bit_count;
    let data = zp.data;
    let mut pos = zp.pos;

    macro_rules! read_byte {
        () => {{
            let b = if pos < data.len() { data[pos] } else { 0xff };
            pos = pos.wrapping_add(1);
            b as u32
        }};
    }
    macro_rules! refill {
        () => {
            while bit_count <= 24 {
                bit_buf = (bit_buf << 8) | read_byte!();
                bit_count += 8;
            }
        };
    }
    macro_rules! renorm {
        () => {{
            let shift = (a as u16).leading_ones();
            bit_count -= shift as i32;
            a = (a << shift) & 0xffff;
            let mask = (1u32 << (shift & 31)).wrapping_sub(1);
            c = ((c << shift) | (bit_buf >> (bit_count as u32 & 31)) & mask) & 0xffff;
            if bit_count < 16 {
                refill!();
            }
            fence = c.min(0x7fff);
        }};
    }

    let pix = |row: &[u8], col: usize| -> u32 { row.get(col).copied().unwrap_or(0) as u32 };
    let w = row_slice.len();
    let mut r2 = pix(rp2, 0) << 1 | pix(rp2, 1);
    let mut r1 = pix(rp1, 0) << 2 | pix(rp1, 1) << 1 | pix(rp1, 2);
    let mut r0: u32 = 0;

    let (rp2_off, rp1_off) = if w >= 3 && rp2.len() >= w && rp1.len() >= w {
        (&rp2[2..w], &rp1[3..w])
    } else {
        (&rp2[..0], &rp1[..0])
    };
    let mid_end = rp2_off.len().min(rp1_off.len());

    macro_rules! decode_step {
        ($out:expr, $n2:expr, $n1:expr) => {{
            let idx = (((r2 << 7) | (r1 << 2) | r0) & 1023) as usize;
            let state = ctx[idx] as usize;
            let mps_bit = state & 1;
            let z = a + PROB[state] as u32;
            let bit = if z <= fence {
                a = z;
                mps_bit != 0
            } else {
                let boundary = 0x6000u32 + ((a + z) >> 2);
                let z_clamped = z.min(boundary);
                if z_clamped > c {
                    let complement = 0x10000u32 - z_clamped;
                    a = (a + complement) & 0xffff;
                    c = (c + complement) & 0xffff;
                    ctx[idx] = LPS_NEXT[state];
                    renorm!();
                    (1 - mps_bit) != 0
                } else {
                    if a >= THRESHOLD[state] as u32 {
                        ctx[idx] = MPS_NEXT[state];
                    }
                    bit_count -= 1;
                    a = (z_clamped << 1) & 0xffff;
                    c = ((c << 1) | (bit_buf >> (bit_count as u32 & 31)) & 1) & 0xffff;
                    if bit_count < 16 {
                        refill!();
                    }
                    fence = c.min(0x7fff);
                    mps_bit != 0
                }
            };
            *$out = bit as u8;
            r2 = ((r2 << 1) & 0b111) | ($n2 as u32);
            r1 = ((r1 << 1) & 0b11111) | ($n1 as u32);
            r0 = ((r0 << 1) & 0b11) | bit as u32;
        }};
    }

    let (fast_slice, slow_slice) = row_slice.split_at_mut(mid_end);
    for (out, (n2, n1)) in fast_slice.iter_mut().zip(rp2_off.iter().zip(rp1_off)) {
        decode_step!(out, *n2, *n1);
    }
    for (i, out) in slow_slice.iter_mut().enumerate() {
        let col = i + mid_end;
        decode_step!(out, pix(rp2, col + 2), pix(rp1, col + 3));
    }

    zp.a = a;
    zp.c = c;
    zp.fence = fence;
    zp.bit_buf = bit_buf;
    zp.bit_count = bit_count;
    zp.pos = pos;
}

/// Decode one row of a refinement-mode JB2 bitmap with inline ZP arithmetic.
///
/// Same local-variable register-allocation trick as `decode_direct_row`,
/// but uses the 11-bit refinement context (ctx: [u8; 2048]).
/// Rolling-window initial values (`init_c_r1`, `init_m_r1`, `init_m_r0`) are
/// pre-computed by the caller at the start of each outer (row) iteration.
#[allow(clippy::too_many_arguments)]
#[inline(never)]
pub(super) fn decode_ref_row(
    zp: &mut ZpDecoder<'_>,
    ctx: &mut [u8; 2048],
    ctx_p: &mut [u16; 2048],
    cbm_row_mut: &mut [u8],
    cbm_r1: &[u8],
    mbm_r2: &[u8],
    mbm_r1: &[u8],
    mbm_r0: &[u8],
    col_shift: i32,
    init_c_r1: u32,
    init_m_r1: u32,
    init_m_r0: u32,
) {
    use djvu_zp::tables::{LPS_NEXT, MPS_NEXT, PROB, THRESHOLD};

    let mut a: u32 = zp.a;
    let mut c: u32 = zp.c;
    let mut fence: u32 = zp.fence;
    let mut bit_buf = zp.bit_buf;
    let mut bit_count = zp.bit_count;
    let data = zp.data;
    let mut pos = zp.pos;

    macro_rules! read_byte {
        () => {{
            let b = if pos < data.len() { data[pos] } else { 0xff };
            pos = pos.wrapping_add(1);
            b as u32
        }};
    }
    macro_rules! refill {
        () => {
            while bit_count <= 24 {
                bit_buf = (bit_buf << 8) | read_byte!();
                bit_count += 8;
            }
        };
    }
    macro_rules! renorm {
        () => {{
            let shift = (a as u16).leading_ones();
            bit_count -= shift as i32;
            a = (a << shift) & 0xffff;
            let mask = (1u32 << (shift & 31)).wrapping_sub(1);
            c = ((c << shift) | (bit_buf >> (bit_count as u32 & 31)) & mask) & 0xffff;
            if bit_count < 16 {
                refill!();
            }
            fence = c.min(0x7fff);
        }};
    }

    let pix_row = |row_slice: &[u8], col: i32| -> u32 {
        if col < 0 {
            return 0;
        }
        row_slice.get(col as usize).copied().unwrap_or(0) as u32
    };

    // c_r0 = previous decoded pixel in this row (starts 0; advances with `bit`).
    let mut c_r0: u32 = 0;
    let mut c_r1 = init_c_r1;
    let mut m_r1 = init_m_r1;
    let mut m_r0 = init_m_r0;

    for col in 0..cbm_row_mut.len() as i32 {
        let m_r2 = pix_row(mbm_r2, col + col_shift);
        // idx ≤ 2047: c_r1<8, c_r0<2, m_r2<2, m_r1<8, m_r0<8
        let idx = ((c_r1 << 8) | (c_r0 << 7) | (m_r2 << 6) | (m_r1 << 3) | m_r0) & 2047;

        let state = ctx[idx as usize] as usize;
        let prob = ctx_p[idx as usize] as u32; // parallel load: precomputed PROB[state]
        let mps_bit = state & 1;
        let z = a + prob;

        let bit = if z <= fence {
            a = z;
            mps_bit != 0
        } else {
            let boundary = 0x6000u32 + ((a + z) >> 2);
            let z_clamped = z.min(boundary);
            if z_clamped > c {
                let complement = 0x10000u32 - z_clamped;
                a = (a + complement) & 0xffff;
                c = (c + complement) & 0xffff;
                let next = LPS_NEXT[state];
                ctx[idx as usize] = next;
                ctx_p[idx as usize] = PROB[next as usize];
                renorm!();
                (1 - mps_bit) != 0
            } else {
                if a >= THRESHOLD[state] as u32 {
                    let next = MPS_NEXT[state];
                    ctx[idx as usize] = next;
                    ctx_p[idx as usize] = PROB[next as usize];
                }
                bit_count -= 1;
                a = (z_clamped << 1) & 0xffff;
                c = ((c << 1) | (bit_buf >> (bit_count as u32 & 31)) & 1) & 0xffff;
                if bit_count < 16 {
                    refill!();
                }
                fence = c.min(0x7fff);
                mps_bit != 0
            }
        };

        if bit {
            cbm_row_mut[col as usize] = 1;
        }

        c_r1 = ((c_r1 << 1) & 0b111) | pix_row(cbm_r1, col + 2);
        c_r0 = bit as u32;
        m_r1 = ((m_r1 << 1) & 0b111) | pix_row(mbm_r1, col + col_shift + 2);
        m_r0 = ((m_r0 << 1) & 0b111) | pix_row(mbm_r0, col + col_shift + 2);
    }

    zp.a = a;
    zp.c = c;
    zp.fence = fence;
    zp.bit_buf = bit_buf;
    zp.bit_count = bit_count;
    zp.pos = pos;
}

/// Pack one decoded row (1 byte per pixel, 0 or 1) into packed Jbm storage
/// (1 bit per pixel, MSB-first within byte).
#[inline]
pub(super) fn pack_row_into(src: &[u8], width: usize, dst: &mut [u8]) {
    let full_bytes = width / 8;
    let rem = width % 8;
    for i in 0..full_bytes {
        let s: &[u8; 8] = src[i * 8..(i + 1) * 8].try_into().unwrap();
        dst[i] = pack_byte(s);
    }
    if rem > 0 {
        let base = full_bytes * 8;
        let mut byte_val = 0u8;
        for j in 0..rem {
            if src[base + j] != 0 {
                byte_val |= 0x80u8 >> j;
            }
        }
        dst[full_bytes] = byte_val;
    }
}

/// Unpack one Jbm row (packed, MSB-first) into a 1-byte-per-pixel scratch
/// buffer. Caller ensures `dst.len() >= width`.
#[inline]
pub(super) fn unpack_row_into(src: &[u8], width: usize, dst: &mut [u8]) {
    let full_bytes = width / 8;
    let rem = width % 8;
    for i in 0..full_bytes {
        let b = src[i];
        let out = &mut dst[i * 8..(i + 1) * 8];
        out[0] = (b >> 7) & 1;
        out[1] = (b >> 6) & 1;
        out[2] = (b >> 5) & 1;
        out[3] = (b >> 4) & 1;
        out[4] = (b >> 3) & 1;
        out[5] = (b >> 2) & 1;
        out[6] = (b >> 1) & 1;
        out[7] = b & 1;
    }
    if rem > 0 {
        let b = src[full_bytes];
        let base = full_bytes * 8;
        for j in 0..rem {
            dst[base + j] = (b >> (7 - j)) & 1;
        }
    }
}

pub(super) fn decode_bitmap_direct(
    zp: &mut ZpDecoder<'_>,
    ctx: &mut [u8; 1024],
    width: i32,
    height: i32,
    pool: &mut Vec<u8>,
) -> Result<Jbm, Jb2Error> {
    let pixels = (width.max(0) as usize).saturating_mul(height.max(0) as usize);
    if pixels > MAX_SYMBOL_PIXELS {
        return Err(Jb2Error::ImageTooLarge);
    }
    if width <= 0 || height <= 0 {
        return Ok(Jbm::new_from_pool(width, height, pool));
    }
    let mut bm = Jbm::new_from_pool(width, height, pool);
    let w = width as usize;
    let h = height as usize;
    let stride = bm.stride();
    debug_assert_eq!(bm.data.len(), stride * h);

    // Scratch rows: 1 byte per pixel. Rotated each iteration so the decoder
    // can read the two previously-decoded rows without unpacking from storage.
    let mut s_curr = vec![0u8; w];
    let mut s_prev1 = vec![0u8; w];
    let mut s_prev2 = vec![0u8; w];

    for row in (0..h).rev() {
        s_curr.iter_mut().for_each(|b| *b = 0);
        decode_direct_row(zp, ctx, &mut s_curr, &s_prev1, &s_prev2);
        pack_row_into(&s_curr, w, &mut bm.data[row * stride..(row + 1) * stride]);
        // Rotate: prev2 ← prev1, prev1 ← curr, curr ← (old prev2, re-used).
        core::mem::swap(&mut s_prev2, &mut s_prev1);
        core::mem::swap(&mut s_prev1, &mut s_curr);
    }
    Ok(bm)
}
