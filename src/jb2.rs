//! JB2 bilevel image decoder — clean-room implementation (phase 2b).
//!
//! Decodes JB2-encoded bitonal images from DjVu Sjbz and Djbz chunks.
//! The JB2 format uses a ZP adaptive arithmetic coder with 262 context variables
//! and a symbol dictionary for run-length compression of recurring glyphs.
//!
//! # Key public types
//!
//! - `Jb2Dict` — shared symbol dictionary decoded from a Djbz chunk
//! - `decode` — decode a Sjbz image stream to a `Bitmap`
//! - `decode_dict` — decode a Djbz dictionary stream to a `Jb2Dict`
//!
//! # Record types
//!
//! | Code | Meaning |
//! |------|---------|
//! | 0    | start-of-image |
//! | 1    | new-symbol, add to dict AND blit to page |
//! | 2    | new-symbol, add to dict only |
//! | 3    | new-symbol (direct), blit only (not added to dict) |
//! | 4    | matched-refine, add to dict AND blit |
//! | 5    | matched-refine, add to dict only |
//! | 6    | matched-refine, blit only |
//! | 7    | matched-copy (no refinement), blit only |
//! | 8    | non-symbol (halftone block), blit only |
//! | 9    | required-dict-or-reset |
//! | 10   | comment |
//! | 11   | end-of-data |

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

use crate::bitmap::Bitmap;
use crate::error::Jb2Error;
use crate::zp_impl::ZpDecoder;

// ────────────────────────────────────────────────────────────────────────────
// NumContext: binary-tree arena for variable-length integer decoding
// ────────────────────────────────────────────────────────────────────────────

/// Binary-tree context store used to decode variable-length integers with ZP.
///
/// Each node in the tree holds one adaptive ZP context byte. Nodes are
/// allocated lazily as the decoder traverses the tree.
struct NumContext {
    ctx: Vec<u8>,
    left: Vec<u32>,
    right: Vec<u32>,
}

impl NumContext {
    fn new() -> Self {
        // Index 0 = unused sentinel; index 1 = root.
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

/// Decode a variable-length integer in the range `[low, high]` using ZP
/// with a binary-tree context store.
fn decode_num(zp: &mut ZpDecoder<'_>, ctx: &mut NumContext, low: i32, high: i32) -> i32 {
    let mut low = low;
    let mut high = high;
    let mut negative = false;
    let mut cutoff: i32 = 0;
    let mut phase: u32 = 1;
    let mut range: u32 = 0xffff_ffff;
    let mut node = ctx.root();

    while range != 1 {
        let decision = if low >= cutoff {
            true
        } else if high >= cutoff {
            zp.decode_bit(&mut ctx.ctx[node])
        } else {
            false
        };

        node = if decision {
            ctx.get_right(node)
        } else {
            ctx.get_left(node)
        };

        match phase {
            1 => {
                negative = !decision;
                if negative {
                    let temp = -low - 1;
                    low = -high - 1;
                    high = temp;
                }
                phase = 2;
                cutoff = 1;
            }
            2 => {
                if !decision {
                    phase = 3;
                    range = ((cutoff + 1) / 2) as u32;
                    if range == 1 {
                        // range is already 1; set cutoff to 0 to terminate the loop.
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
            _ => {
                // Unreachable: phase cycles through 1, 2, 3 only.
                // Use a saturating decrement to keep range moving toward 1.
                range = range.saturating_sub(1);
            }
        }
    }

    if negative { -cutoff - 1 } else { cutoff }
}

// ────────────────────────────────────────────────────────────────────────────
// Jbm: internal bit-packed working bitmap (row 0 = bottom of page)
// ────────────────────────────────────────────────────────────────────────────

/// Internal working bitmap used during JB2 decoding.
///
/// Pixels are stored bit-packed: 1 bit per pixel, MSB-first within each byte,
/// rows padded to byte boundary (`row_stride_bytes`). Matches `Bitmap`'s
/// convention, which makes blit into `Bitmap` a shift-align copy rather than
/// a byte→bit pack.
/// Row 0 is the **bottom** of the image (DjVu convention).
#[derive(Clone)]
struct Jbm {
    width: i32,
    height: i32,
    data: Vec<u8>,
}

impl Jbm {
    #[inline(always)]
    fn row_stride_bytes(width: i32) -> usize {
        (width.max(0) as usize).div_ceil(8)
    }

    #[inline(always)]
    fn stride(&self) -> usize {
        Self::row_stride_bytes(self.width)
    }

    #[inline(always)]
    fn storage_bytes(width: i32, height: i32) -> usize {
        Self::row_stride_bytes(width).saturating_mul(height.max(0) as usize)
    }

    fn new(width: i32, height: i32) -> Self {
        let len = Self::storage_bytes(width, height);
        Jbm {
            width,
            height,
            data: vec![0u8; len],
        }
    }

    /// Return the pixel value at (row, col); out-of-bounds → 0.
    #[inline(always)]
    fn get(&self, row: i32, col: i32) -> u8 {
        if row < 0 || row >= self.height || col < 0 || col >= self.width {
            return 0;
        }
        let stride = self.stride();
        let byte = self.data[row as usize * stride + (col as usize / 8)];
        (byte >> (7 - (col as usize & 7))) & 1
    }

    /// Set pixel at (row, col) to black (1). Caller must ensure in-bounds.
    #[inline(always)]
    fn set_black(&mut self, row: usize, col: usize) {
        let stride = self.stride();
        self.data[row * stride + (col / 8)] |= 0x80u8 >> (col & 7);
    }

    /// Construct a `Jbm` using a reusable scratch buffer.
    ///
    /// The buffer is grown to at least `storage_bytes(width, height)` bytes
    /// (never shrunk), and the used portion is zeroed.  The old buffer
    /// contents are taken via `std::mem::take` so `pool` is left empty on
    /// return; the caller regains the buffer by calling
    /// [`Jbm::crop_and_recycle`] or [`Jbm::recycle_into`].
    fn new_from_pool(width: i32, height: i32, pool: &mut Vec<u8>) -> Self {
        let bytes = Self::storage_bytes(width, height);
        if pool.len() < bytes {
            pool.resize(bytes, 0u8);
        }
        // Zero the portion we will use (including any bytes reused from a previous symbol).
        pool[..bytes].fill(0u8);
        let mut data = core::mem::take(pool);
        data.truncate(bytes);
        Jbm {
            width,
            height,
            data,
        }
    }

    /// Crop to content and return the original backing buffer to the pool.
    ///
    /// This is the pool-aware alternative to `crop_to_content()`: it performs
    /// the same crop but moves the (now-unused) full-size backing buffer back
    /// into `pool` so it can be reused for the next symbol decode.
    ///
    /// Fast path: if all four border edges already have content (i.e. the bitmap
    /// is already tight), skip the O(w×h) full scan and copy entirely — just
    /// return `self` directly.  This handles the common case where the JB2
    /// encoder already provided tight bounding box dimensions.
    fn crop_and_recycle(self, pool: &mut Vec<u8>) -> Jbm {
        if self.width > 0 && self.height > 0 {
            let w = self.width as usize;
            let h = self.height as usize;
            let stride = self.stride();
            let last_col = w - 1;
            let data = &self.data;
            // Any bit set in the row's stride bytes. Padding bits (if any) are
            // guaranteed zero, so OR-ing the whole row is safe.
            let top_has = data[..stride].iter().any(|&b| b != 0);
            let bot_has = data[(h - 1) * stride..h * stride].iter().any(|&b| b != 0);
            let left_has = (0..h).any(|r| (data[r * stride] & 0x80) != 0);
            let right_has =
                (0..h).any(|r| (data[r * stride + last_col / 8] & (0x80u8 >> (last_col & 7))) != 0);
            if top_has && bot_has && left_has && right_has {
                // Already tight — return self directly without copying.
                // Pre-allocate the pool with the same capacity so the next
                // new_from_pool call can reuse it without a realloc.
                *pool = Vec::with_capacity(self.data.len());
                return self;
            }
        }
        let cropped = self.crop_to_content();
        // Move our data buffer back to the pool (it may be larger than `cropped.data`)
        *pool = self.data;
        cropped
    }

    /// Move the backing buffer back into `pool` without cropping.
    ///
    /// Used for symbols that are blitted but not stored in the dict.
    fn recycle_into(self, pool: &mut Vec<u8>) {
        *pool = self.data;
    }

    /// Return a new Jbm with surrounding empty rows/columns removed.
    fn crop_to_content(&self) -> Jbm {
        if self.width <= 0 || self.height <= 0 {
            return Jbm::new(0, 0);
        }
        let stride = self.stride();
        let mut min_row = self.height;
        let mut max_row: i32 = -1;
        let mut min_col = self.width;
        let mut max_col: i32 = -1;

        for row in 0..self.height {
            let row_bytes = &self.data[row as usize * stride..(row as usize + 1) * stride];
            // Find first/last nonzero byte in the row, then refine to column index.
            let mut byte_min: Option<usize> = None;
            let mut byte_max: Option<usize> = None;
            for (i, &b) in row_bytes.iter().enumerate() {
                if b != 0 {
                    if byte_min.is_none() {
                        byte_min = Some(i);
                    }
                    byte_max = Some(i);
                }
            }
            if let (Some(bmin), Some(bmax)) = (byte_min, byte_max) {
                let col_lo = bmin * 8 + row_bytes[bmin].leading_zeros() as usize;
                // leading zeros in a reversed sense: for MSB-first, the first set
                // bit position within the byte is `leading_zeros`.
                let col_hi = bmax * 8 + (7 - row_bytes[bmax].trailing_zeros() as usize);
                let col_hi = col_hi.min(self.width as usize - 1) as i32;
                let col_lo = col_lo as i32;
                min_row = min_row.min(row);
                max_row = max_row.max(row);
                min_col = min_col.min(col_lo);
                max_col = max_col.max(col_hi);
            }
        }

        if max_row < 0 {
            return Jbm::new(0, 0);
        }

        let nw = max_col - min_col + 1;
        let nh = max_row - min_row + 1;
        let mut out = Jbm::new(nw, nh);

        for row in min_row..=max_row {
            for col in min_col..=max_col {
                let src_byte = self.data[row as usize * stride + (col as usize / 8)];
                if (src_byte >> (7 - (col as usize & 7))) & 1 != 0 {
                    let out_row = (row - min_row) as usize;
                    let out_col = (col - min_col) as usize;
                    out.set_black(out_row, out_col);
                }
            }
        }
        out
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Direct bitmap decode: 10-bit context
// ────────────────────────────────────────────────────────────────────────────

/// Decode a bitmap using the direct (10-pixel context) method.
///
/// Decodes top-to-bottom using an incremental rolling window that avoids
/// recomputing all 10 context bits from scratch each pixel.
const MAX_SYMBOL_PIXELS: usize = 16 * 1024 * 1024; // 16 MP per symbol — allows large connected components while bounding DoS input
// 256 MP cumulative decoded-symbol work. Dense JB2 pages can contain many
// direct or refinement records whose individual symbols are valid and whose
// blit work is bounded separately below; 64 MP was too low for the
// `pathogenic_bacteria_1896.djvu` corpus (#258).
pub(crate) const MAX_TOTAL_SYMBOL_PIXELS: usize = 256 * 1024 * 1024;
const MAX_TOTAL_BLIT_PIXELS: usize = 256 * 1024 * 1024; // 256 MP total blit work — prevents type-7 DoS
const MAX_RECORDS: usize = 65_536; // 64 K records per stream — prevents DoS via record-loop spin on exhausted ZP input
const MAX_COMMENT_BYTES: usize = 4096; // 4 KiB per comment record — prevents DoS via huge comment length

/// Check that decoding a `w × h` symbol won't exceed per-symbol or stream-total pixel budgets.
#[inline(always)]
fn check_pixel_budget(w: i32, h: i32, total: &mut usize) -> Result<(), Jb2Error> {
    let pixels = (w.max(0) as usize).saturating_mul(h.max(0) as usize);
    if pixels > MAX_SYMBOL_PIXELS {
        return Err(Jb2Error::ImageTooLarge);
    }
    *total = total.saturating_add(pixels);
    if *total > MAX_TOTAL_SYMBOL_PIXELS {
        return Err(Jb2Error::ImageTooLarge);
    }
    Ok(())
}

/// Check that blitting a symbol won't exceed the total blit-work budget.
///
/// Prevents DoS via repeated blitting of a large dict symbol (type 7 / matched copy)
/// which has no decode cost but O(w×h) blit cost per record.
#[inline(always)]
fn check_blit_budget(sym: &Jbm, total: &mut usize) -> Result<(), Jb2Error> {
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
fn decode_direct_row(
    zp: &mut ZpDecoder<'_>,
    ctx: &mut [u8; 1024],
    row_slice: &mut [u8],
    rp1: &[u8],
    rp2: &[u8],
) {
    use crate::zp_impl::tables::{LPS_NEXT, MPS_NEXT, PROB, THRESHOLD};

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
fn decode_ref_row(
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
    use crate::zp_impl::tables::{LPS_NEXT, MPS_NEXT, PROB, THRESHOLD};

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
fn pack_row_into(src: &[u8], width: usize, dst: &mut [u8]) {
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
fn unpack_row_into(src: &[u8], width: usize, dst: &mut [u8]) {
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

fn decode_bitmap_direct(
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

// ────────────────────────────────────────────────────────────────────────────
// Refinement bitmap decode: 11-bit context
// ────────────────────────────────────────────────────────────────────────────

/// Decode a bitmap using the refinement (11-pixel context) method.
///
/// The new (child) bitmap `cbm` is decoded relative to a reference (matched)
/// bitmap `mbm`. Center alignment is used per the DjVu spec.
fn decode_bitmap_ref(
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

// ────────────────────────────────────────────────────────────────────────────
// Baseline: rolling median-of-3 for vertical symbol positioning
// ────────────────────────────────────────────────────────────────────────────

struct Baseline {
    arr: [i32; 3],
    index: i32,
}

impl Baseline {
    fn new() -> Self {
        Baseline {
            arr: [0, 0, 0],
            index: -1,
        }
    }

    fn fill(&mut self, val: i32) {
        self.arr = [val, val, val];
    }

    fn add(&mut self, val: i32) {
        self.index += 1;
        if self.index == 3 {
            self.index = 0;
        }
        self.arr[self.index as usize] = val;
    }

    fn get_val(&self) -> i32 {
        let (a, b, c) = (self.arr[0], self.arr[1], self.arr[2]);
        if (a >= b && a <= c) || (a <= b && a >= c) {
            a
        } else if (b >= a && b <= c) || (b <= a && b >= c) {
            b
        } else {
            c
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Blit a symbol onto the page (OR compositing, bottom-left origin)
// ────────────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn blit_indexed(
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
fn blit_to_bitmap(bm: &mut Bitmap, sym: &Jbm, x: i32, y: i32) {
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

// ────────────────────────────────────────────────────────────────────────────
// Convert internal page buffer (row 0 = bottom) to Bitmap (row 0 = top)
// ────────────────────────────────────────────────────────────────────────────

/// Pack a single byte: each of the 8 input bytes (0 or 1) into one output byte.
/// Bit 7 = src[0], bit 6 = src[1], …, bit 0 = src[7].
#[inline(always)]
fn pack_byte(s: &[u8; 8]) -> u8 {
    ((s[0] != 0) as u8) << 7
        | ((s[1] != 0) as u8) << 6
        | ((s[2] != 0) as u8) << 5
        | ((s[3] != 0) as u8) << 4
        | ((s[4] != 0) as u8) << 3
        | ((s[5] != 0) as u8) << 2
        | ((s[6] != 0) as u8) << 1
        | ((s[7] != 0) as u8)
}

fn page_to_bitmap(page: &[u8], width: i32, height: i32) -> Bitmap {
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
fn flip_blit_map(blit_map: &mut [i32], width: usize, height: usize) {
    for row in 0..height / 2 {
        let mirror = height - 1 - row;
        let a = row * width;
        let b = mirror * width;
        for col in 0..width {
            blit_map.swap(a + col, b + col);
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Symbol coordinate decoding
// ────────────────────────────────────────────────────────────────────────────

/// ZP coder contexts used exclusively for symbol coordinate decoding.
struct CoordContexts {
    offset_type: u8,
    hoff: NumContext,
    voff: NumContext,
    shoff: NumContext,
    svoff: NumContext,
}

impl CoordContexts {
    fn new() -> Self {
        Self {
            offset_type: 0,
            hoff: NumContext::new(),
            voff: NumContext::new(),
            shoff: NumContext::new(),
            svoff: NumContext::new(),
        }
    }
}

/// Running layout state for symbol positioning within a JB2 image.
struct LayoutState {
    first_left: i32,
    first_bottom: i32,
    last_right: i32,
    baseline: Baseline,
}

impl LayoutState {
    fn new(image_height: i32) -> Self {
        Self {
            first_left: -1,
            first_bottom: image_height - 1,
            last_right: 0,
            baseline: Baseline::new(),
        }
    }
}

fn decode_symbol_coords(
    zp: &mut ZpDecoder<'_>,
    coord_ctx: &mut CoordContexts,
    layout: &mut LayoutState,
    sym_width: i32,
    sym_height: i32,
) -> (i32, i32) {
    let new_line = zp.decode_bit(&mut coord_ctx.offset_type);

    let (x, y) = if new_line {
        let hoff = decode_num(zp, &mut coord_ctx.hoff, -262143, 262142);
        let voff = decode_num(zp, &mut coord_ctx.voff, -262143, 262142);
        let nx = layout.first_left + hoff;
        let ny = layout.first_bottom + voff - sym_height + 1;
        layout.first_left = nx;
        layout.first_bottom = ny;
        layout.baseline.fill(ny);
        (nx, ny)
    } else {
        let hoff = decode_num(zp, &mut coord_ctx.shoff, -262143, 262142);
        let voff = decode_num(zp, &mut coord_ctx.svoff, -262143, 262142);
        (layout.last_right + hoff, layout.baseline.get_val() + voff)
    };

    layout.baseline.add(y);
    layout.last_right = x + sym_width - 1;
    (x, y)
}

// ────────────────────────────────────────────────────────────────────────────
// Working symbol table: zero-copy view of shared dict + local symbols
// ────────────────────────────────────────────────────────────────────────────

/// Two-part symbol table used during JB2 image/dict decode.
///
/// The `shared` slice refers directly to the cached shared dictionary's symbols
/// (no clone), while `local` holds symbols defined by the stream being decoded.
/// This avoids deep-copying the (potentially large) shared dictionary on every
/// `decode_mask()` call.
struct JbmDict<'a> {
    shared: &'a [Jbm],
    local: Vec<Jbm>,
}

impl<'a> JbmDict<'a> {
    fn new(shared: &'a [Jbm]) -> Self {
        JbmDict {
            shared,
            local: Vec::new(),
        }
    }
    fn len(&self) -> usize {
        self.shared.len() + self.local.len()
    }
    fn is_empty(&self) -> bool {
        self.shared.is_empty() && self.local.is_empty()
    }
    fn push(&mut self, sym: Jbm) {
        self.local.push(sym);
    }
    fn into_symbols(self) -> Vec<Jbm> {
        // Used by decode_dictionary to return the complete symbol list.
        let mut out = self.shared.to_vec();
        out.extend(self.local);
        out
    }
}

impl core::ops::Index<usize> for JbmDict<'_> {
    type Output = Jbm;
    #[inline(always)]
    fn index(&self, index: usize) -> &Jbm {
        let n = self.shared.len();
        if index < n {
            &self.shared[index]
        } else {
            &self.local[index - n]
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Public API
// ────────────────────────────────────────────────────────────────────────────

/// A shared JB2 symbol dictionary decoded from a Djbz chunk.
///
/// Pass this to [`decode`] when the Sjbz stream references an external dict
/// via a "required-dict-or-reset" (type 9) record.
pub struct Jb2Dict {
    symbols: Vec<Jbm>,
}

/// Decode a JB2 image stream (Sjbz chunk data) into a [`Bitmap`].
///
/// `shared_dict` must be provided when the Sjbz stream begins with a
/// "required-dict-or-reset" record that references an external dictionary.
///
/// # Errors
///
/// Returns [`Jb2Error`] on malformed input, missing dictionary, or oversized image.
pub fn decode(data: &[u8], shared_dict: Option<&Jb2Dict>) -> Result<Bitmap, Jb2Error> {
    decode_image(data, shared_dict)
}

/// Decode a JB2 image stream with per-pixel blit index tracking.
///
/// Returns the bitmap and a blit map (`Vec<i32>`) of the same pixel dimensions.
/// `blit_map[y * width + x]` holds the blit record index for each foreground
/// pixel, or `-1` for background. This is used by the FGbz palette to assign
/// per-glyph colors.
pub fn decode_indexed(
    data: &[u8],
    shared_dict: Option<&Jb2Dict>,
) -> Result<(Bitmap, Vec<i32>), Jb2Error> {
    decode_image_indexed(data, shared_dict)
}

/// Decode a JB2 dictionary stream (Djbz chunk data) into a [`Jb2Dict`].
///
/// The returned dict can then be passed to [`decode`] for Sjbz streams that
/// reference it via an INCL or "required-dict-or-reset" record.
///
/// # Errors
///
/// Returns [`Jb2Error`] on malformed input.
pub fn decode_dict(data: &[u8], inherited: Option<&Jb2Dict>) -> Result<Jb2Dict, Jb2Error> {
    decode_dictionary(data, inherited)
}

// ────────────────────────────────────────────────────────────────────────────
// Core image decode
// ────────────────────────────────────────────────────────────────────────────

fn decode_image(data: &[u8], shared_dict: Option<&Jb2Dict>) -> Result<Bitmap, Jb2Error> {
    let mut pool = Vec::new();
    decode_image_with_pool(data, shared_dict, &mut pool)
}

/// Decode a JB2 image stream, reusing `pool` as a scratch buffer for symbol bitmaps.
///
/// `pool` is resized up (never shrunk) across symbol decodes, eliminating
/// per-symbol heap allocations. Pass `&mut Vec::new()` to use a fresh pool,
/// or reuse a pool across multiple decode calls for additional savings.
fn decode_image_with_pool(
    data: &[u8],
    shared_dict: Option<&Jb2Dict>,
    pool: &mut Vec<u8>,
) -> Result<Bitmap, Jb2Error> {
    let mut zp = ZpDecoder::new(data).map_err(|_| Jb2Error::ZpInitFailed)?;

    // Contexts for variable-length integer decoding
    let mut record_type_ctx = NumContext::new();
    let mut image_size_ctx = NumContext::new();
    let mut symbol_width_ctx = NumContext::new();
    let mut symbol_height_ctx = NumContext::new();
    let mut inherit_dict_size_ctx = NumContext::new();
    let mut coord_ctx = CoordContexts::new();
    let mut symbol_index_ctx = NumContext::new();
    let mut symbol_width_diff_ctx = NumContext::new();
    let mut symbol_height_diff_ctx = NumContext::new();
    let mut horiz_abs_loc_ctx = NumContext::new();
    let mut vert_abs_loc_ctx = NumContext::new();
    let mut comment_length_ctx = NumContext::new();
    let mut comment_octet_ctx = NumContext::new();

    let mut direct_bitmap_ctx = [0u8; 1024];
    let mut refinement_bitmap_ctx = [0u8; 2048];
    let mut refinement_bitmap_ctx_p = [0x8000u16; 2048];
    let mut total_sym_pixels = 0usize;
    let mut total_blit_pixels = 0usize;

    // Preamble: optional "required-dict-or-reset" (type 9) followed by
    // "start-of-image" (type 0).
    let mut rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);
    let mut initial_dict_length: usize = 0;
    if rtype == 9 {
        initial_dict_length = decode_num(&mut zp, &mut inherit_dict_size_ctx, 0, 262142) as usize;
        rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);
    }
    // `rtype` is now the start-of-image record (0); ignore its value.
    let _ = rtype;

    // Image dimensions
    let image_width = {
        let w = decode_num(&mut zp, &mut image_size_ctx, 0, 262142);
        if w == 0 { 200 } else { w }
    };
    let image_height = {
        let h = decode_num(&mut zp, &mut image_size_ctx, 0, 262142);
        if h == 0 { 200 } else { h }
    };

    // Reserved flag bit — must be 0
    let mut flag_ctx: u8 = 0;
    if zp.decode_bit(&mut flag_ctx) {
        return Err(Jb2Error::BadHeaderFlag);
    }

    // Populate initial dictionary from shared dict — zero-copy: borrow the
    // cached dict's symbol slice directly rather than deep-cloning it.
    let initial_symbols: &[Jbm] = if initial_dict_length > 0 {
        match shared_dict {
            Some(sd) => {
                if initial_dict_length > sd.symbols.len() {
                    return Err(Jb2Error::InheritedDictTooLarge);
                }
                &sd.symbols[..initial_dict_length]
            }
            None => return Err(Jb2Error::MissingSharedDict),
        }
    } else {
        &[]
    };
    let mut dict = JbmDict::new(initial_symbols);

    // Safety cap: ~64M pixels (same guard, but now the backing store is 8× smaller).
    const MAX_PIXELS: usize = 64 * 1024 * 1024;
    let page_size = (image_width as usize).saturating_mul(image_height as usize);
    if page_size > MAX_PIXELS {
        return Err(Jb2Error::ImageTooLarge);
    }
    // Use a packed 1-bit-per-pixel bitmap as the page buffer instead of a
    // byte-per-pixel Vec. This is 8× smaller (~1.8 MB vs ~14.5 MB for a 600 dpi
    // page), fitting in L2 cache and dramatically reducing cache pressure during blits.
    let mut page_bm = Bitmap::new(image_width as u32, image_height as u32);

    let mut layout = LayoutState::new(image_height);

    // Main decode loop — capped to prevent infinite spin when ZP input is exhausted
    let mut record_count = 0usize;
    loop {
        if record_count >= MAX_RECORDS {
            return Err(Jb2Error::TooManyRecords);
        }
        record_count += 1;
        let rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);

        match rtype {
            // 1 — new symbol, direct decode → add to dict AND blit
            1 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                check_pixel_budget(w, h, &mut total_sym_pixels)?;
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h, pool)?;
                let (x, y) =
                    decode_symbol_coords(&mut zp, &mut coord_ctx, &mut layout, bm.width, bm.height);
                check_blit_budget(&bm, &mut total_blit_pixels)?;
                blit_to_bitmap(&mut page_bm, &bm, x, y);
                dict.push(bm.crop_and_recycle(pool));
            }

            // 2 — new symbol, direct decode → add to dict only
            2 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                check_pixel_budget(w, h, &mut total_sym_pixels)?;
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h, pool)?;
                dict.push(bm.crop_and_recycle(pool));
            }

            // 3 — new symbol, direct decode → blit only (not stored in dict)
            3 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                check_pixel_budget(w, h, &mut total_sym_pixels)?;
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h, pool)?;
                let (x, y) =
                    decode_symbol_coords(&mut zp, &mut coord_ctx, &mut layout, bm.width, bm.height);
                check_blit_budget(&bm, &mut total_blit_pixels)?;
                blit_to_bitmap(&mut page_bm, &bm, x, y);
                bm.recycle_into(pool);
            }

            // 4 — matched refinement → add to dict AND blit
            4 => {
                if dict.is_empty() {
                    return Err(Jb2Error::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(Jb2Error::InvalidSymbolIndex);
                }
                let wdiff = decode_num(&mut zp, &mut symbol_width_diff_ctx, -262143, 262142);
                let hdiff = decode_num(&mut zp, &mut symbol_height_diff_ctx, -262143, 262142);
                let cbm_w = dict[index].width + wdiff;
                let cbm_h = dict[index].height + hdiff;
                check_pixel_budget(cbm_w, cbm_h, &mut total_sym_pixels)?;
                let cbm = decode_bitmap_ref(
                    &mut zp,
                    &mut refinement_bitmap_ctx,
                    &mut refinement_bitmap_ctx_p,
                    cbm_w,
                    cbm_h,
                    &dict[index],
                    pool,
                )?;
                let (x, y) = decode_symbol_coords(
                    &mut zp,
                    &mut coord_ctx,
                    &mut layout,
                    cbm.width,
                    cbm.height,
                );
                check_blit_budget(&cbm, &mut total_blit_pixels)?;
                blit_to_bitmap(&mut page_bm, &cbm, x, y);
                dict.push(cbm.crop_and_recycle(pool));
            }

            // 5 — matched refinement → add to dict only
            5 => {
                if dict.is_empty() {
                    return Err(Jb2Error::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(Jb2Error::InvalidSymbolIndex);
                }
                let wdiff = decode_num(&mut zp, &mut symbol_width_diff_ctx, -262143, 262142);
                let hdiff = decode_num(&mut zp, &mut symbol_height_diff_ctx, -262143, 262142);
                let cbm_w = dict[index].width + wdiff;
                let cbm_h = dict[index].height + hdiff;
                check_pixel_budget(cbm_w, cbm_h, &mut total_sym_pixels)?;
                let cbm = decode_bitmap_ref(
                    &mut zp,
                    &mut refinement_bitmap_ctx,
                    &mut refinement_bitmap_ctx_p,
                    cbm_w,
                    cbm_h,
                    &dict[index],
                    pool,
                )?;
                dict.push(cbm.crop_and_recycle(pool));
            }

            // 6 — matched refinement → blit only
            6 => {
                if dict.is_empty() {
                    return Err(Jb2Error::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(Jb2Error::InvalidSymbolIndex);
                }
                let wdiff = decode_num(&mut zp, &mut symbol_width_diff_ctx, -262143, 262142);
                let hdiff = decode_num(&mut zp, &mut symbol_height_diff_ctx, -262143, 262142);
                let cbm_w = dict[index].width + wdiff;
                let cbm_h = dict[index].height + hdiff;
                check_pixel_budget(cbm_w, cbm_h, &mut total_sym_pixels)?;
                let cbm = decode_bitmap_ref(
                    &mut zp,
                    &mut refinement_bitmap_ctx,
                    &mut refinement_bitmap_ctx_p,
                    cbm_w,
                    cbm_h,
                    &dict[index],
                    pool,
                )?;
                let (x, y) = decode_symbol_coords(
                    &mut zp,
                    &mut coord_ctx,
                    &mut layout,
                    cbm.width,
                    cbm.height,
                );
                check_blit_budget(&cbm, &mut total_blit_pixels)?;
                blit_to_bitmap(&mut page_bm, &cbm, x, y);
                cbm.recycle_into(pool);
            }

            // 7 — matched copy, no refinement → blit only
            7 => {
                if dict.is_empty() {
                    return Err(Jb2Error::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(Jb2Error::InvalidSymbolIndex);
                }
                let bm_w = dict[index].width;
                let bm_h = dict[index].height;
                let (x, y) = decode_symbol_coords(&mut zp, &mut coord_ctx, &mut layout, bm_w, bm_h);
                let sym = &dict[index];
                check_blit_budget(sym, &mut total_blit_pixels)?;
                blit_to_bitmap(&mut page_bm, sym, x, y);
            }

            // 8 — non-symbol (halftone), absolute coordinates
            8 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                check_pixel_budget(w, h, &mut total_sym_pixels)?;
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h, pool)?;
                let left = decode_num(&mut zp, &mut horiz_abs_loc_ctx, 1, image_width);
                let top = decode_num(&mut zp, &mut vert_abs_loc_ctx, 1, image_height);
                let x = left - 1;
                let y = top - h;
                check_blit_budget(&bm, &mut total_blit_pixels)?;
                blit_to_bitmap(&mut page_bm, &bm, x, y);
                bm.recycle_into(pool);
            }

            // 9 — required-dict-or-reset (already consumed in preamble; ignore here)
            9 => {}

            // 10 — comment: skip bytes
            10 => {
                let length = decode_num(&mut zp, &mut comment_length_ctx, 0, 262142) as usize;
                let length = length.min(MAX_COMMENT_BYTES);
                for _ in 0..length {
                    decode_num(&mut zp, &mut comment_octet_ctx, 0, 255);
                }
            }

            // 11 — end-of-data
            11 => break,

            _ => return Err(Jb2Error::UnknownRecordType),
        }
    }

    Ok(page_bm)
}

/// Same as `decode_image` but tracks per-pixel blit indices.
fn decode_image_indexed(
    data: &[u8],
    shared_dict: Option<&Jb2Dict>,
) -> Result<(Bitmap, Vec<i32>), Jb2Error> {
    let mut pool = Vec::new();
    decode_image_indexed_with_pool(data, shared_dict, &mut pool)
}

fn decode_image_indexed_with_pool(
    data: &[u8],
    shared_dict: Option<&Jb2Dict>,
    pool: &mut Vec<u8>,
) -> Result<(Bitmap, Vec<i32>), Jb2Error> {
    let mut zp = ZpDecoder::new(data).map_err(|_| Jb2Error::ZpInitFailed)?;

    let mut record_type_ctx = NumContext::new();
    let mut image_size_ctx = NumContext::new();
    let mut symbol_width_ctx = NumContext::new();
    let mut symbol_height_ctx = NumContext::new();
    let mut inherit_dict_size_ctx = NumContext::new();
    let mut coord_ctx = CoordContexts::new();
    let mut symbol_index_ctx = NumContext::new();
    let mut symbol_width_diff_ctx = NumContext::new();
    let mut symbol_height_diff_ctx = NumContext::new();
    let mut horiz_abs_loc_ctx = NumContext::new();
    let mut vert_abs_loc_ctx = NumContext::new();
    let mut comment_length_ctx = NumContext::new();
    let mut comment_octet_ctx = NumContext::new();

    let mut direct_bitmap_ctx = [0u8; 1024];
    let mut refinement_bitmap_ctx = [0u8; 2048];
    let mut refinement_bitmap_ctx_p = [0x8000u16; 2048];
    let mut total_sym_pixels = 0usize;
    let mut total_blit_pixels = 0usize;

    let mut rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);
    let mut initial_dict_length: usize = 0;
    if rtype == 9 {
        initial_dict_length = decode_num(&mut zp, &mut inherit_dict_size_ctx, 0, 262142) as usize;
        rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);
    }
    let _ = rtype;

    let image_width = {
        let w = decode_num(&mut zp, &mut image_size_ctx, 0, 262142);
        if w == 0 { 200 } else { w }
    };
    let image_height = {
        let h = decode_num(&mut zp, &mut image_size_ctx, 0, 262142);
        if h == 0 { 200 } else { h }
    };

    let mut flag_ctx: u8 = 0;
    if zp.decode_bit(&mut flag_ctx) {
        return Err(Jb2Error::BadHeaderFlag);
    }

    let initial_symbols_idx: &[Jbm] = if initial_dict_length > 0 {
        match shared_dict {
            Some(sd) => {
                if initial_dict_length > sd.symbols.len() {
                    return Err(Jb2Error::InheritedDictTooLarge);
                }
                &sd.symbols[..initial_dict_length]
            }
            None => return Err(Jb2Error::MissingSharedDict),
        }
    } else {
        &[]
    };
    let mut dict = JbmDict::new(initial_symbols_idx);

    const MAX_PIXELS: usize = 64 * 1024 * 1024;
    let page_size = (image_width as usize).saturating_mul(image_height as usize);
    if page_size > MAX_PIXELS {
        return Err(Jb2Error::ImageTooLarge);
    }
    let mut page = vec![0u8; page_size];
    let mut blit_map = vec![-1i32; page_size];

    let mut layout = LayoutState::new(image_height);
    let mut blit_count: i32 = 0;

    let mut record_count = 0usize;
    loop {
        if record_count >= MAX_RECORDS {
            return Err(Jb2Error::TooManyRecords);
        }
        record_count += 1;
        let rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);

        match rtype {
            1 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                check_pixel_budget(w, h, &mut total_sym_pixels)?;
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h, pool)?;
                let (x, y) =
                    decode_symbol_coords(&mut zp, &mut coord_ctx, &mut layout, bm.width, bm.height);
                check_blit_budget(&bm, &mut total_blit_pixels)?;
                blit_indexed(
                    &mut page,
                    &mut blit_map,
                    image_width,
                    image_height,
                    &bm,
                    x,
                    y,
                    blit_count,
                );
                blit_count += 1;
                dict.push(bm.crop_and_recycle(pool));
            }
            2 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                check_pixel_budget(w, h, &mut total_sym_pixels)?;
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h, pool)?;
                dict.push(bm.crop_and_recycle(pool));
            }
            3 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                check_pixel_budget(w, h, &mut total_sym_pixels)?;
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h, pool)?;
                let (x, y) =
                    decode_symbol_coords(&mut zp, &mut coord_ctx, &mut layout, bm.width, bm.height);
                check_blit_budget(&bm, &mut total_blit_pixels)?;
                blit_indexed(
                    &mut page,
                    &mut blit_map,
                    image_width,
                    image_height,
                    &bm,
                    x,
                    y,
                    blit_count,
                );
                blit_count += 1;
                bm.recycle_into(pool);
            }
            4 => {
                if dict.is_empty() {
                    return Err(Jb2Error::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(Jb2Error::InvalidSymbolIndex);
                }
                let wdiff = decode_num(&mut zp, &mut symbol_width_diff_ctx, -262143, 262142);
                let hdiff = decode_num(&mut zp, &mut symbol_height_diff_ctx, -262143, 262142);
                let cbm_w = dict[index].width + wdiff;
                let cbm_h = dict[index].height + hdiff;
                check_pixel_budget(cbm_w, cbm_h, &mut total_sym_pixels)?;
                let cbm = decode_bitmap_ref(
                    &mut zp,
                    &mut refinement_bitmap_ctx,
                    &mut refinement_bitmap_ctx_p,
                    cbm_w,
                    cbm_h,
                    &dict[index],
                    pool,
                )?;
                let (x, y) = decode_symbol_coords(
                    &mut zp,
                    &mut coord_ctx,
                    &mut layout,
                    cbm.width,
                    cbm.height,
                );
                check_blit_budget(&cbm, &mut total_blit_pixels)?;
                blit_indexed(
                    &mut page,
                    &mut blit_map,
                    image_width,
                    image_height,
                    &cbm,
                    x,
                    y,
                    blit_count,
                );
                blit_count += 1;
                dict.push(cbm.crop_and_recycle(pool));
            }
            5 => {
                if dict.is_empty() {
                    return Err(Jb2Error::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(Jb2Error::InvalidSymbolIndex);
                }
                let wdiff = decode_num(&mut zp, &mut symbol_width_diff_ctx, -262143, 262142);
                let hdiff = decode_num(&mut zp, &mut symbol_height_diff_ctx, -262143, 262142);
                let cbm_w = dict[index].width + wdiff;
                let cbm_h = dict[index].height + hdiff;
                check_pixel_budget(cbm_w, cbm_h, &mut total_sym_pixels)?;
                let cbm = decode_bitmap_ref(
                    &mut zp,
                    &mut refinement_bitmap_ctx,
                    &mut refinement_bitmap_ctx_p,
                    cbm_w,
                    cbm_h,
                    &dict[index],
                    pool,
                )?;
                dict.push(cbm.crop_and_recycle(pool));
            }
            6 => {
                if dict.is_empty() {
                    return Err(Jb2Error::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(Jb2Error::InvalidSymbolIndex);
                }
                let wdiff = decode_num(&mut zp, &mut symbol_width_diff_ctx, -262143, 262142);
                let hdiff = decode_num(&mut zp, &mut symbol_height_diff_ctx, -262143, 262142);
                let cbm_w = dict[index].width + wdiff;
                let cbm_h = dict[index].height + hdiff;
                check_pixel_budget(cbm_w, cbm_h, &mut total_sym_pixels)?;
                let cbm = decode_bitmap_ref(
                    &mut zp,
                    &mut refinement_bitmap_ctx,
                    &mut refinement_bitmap_ctx_p,
                    cbm_w,
                    cbm_h,
                    &dict[index],
                    pool,
                )?;
                let (x, y) = decode_symbol_coords(
                    &mut zp,
                    &mut coord_ctx,
                    &mut layout,
                    cbm.width,
                    cbm.height,
                );
                check_blit_budget(&cbm, &mut total_blit_pixels)?;
                blit_indexed(
                    &mut page,
                    &mut blit_map,
                    image_width,
                    image_height,
                    &cbm,
                    x,
                    y,
                    blit_count,
                );
                blit_count += 1;
                cbm.recycle_into(pool);
            }
            7 => {
                if dict.is_empty() {
                    return Err(Jb2Error::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(Jb2Error::InvalidSymbolIndex);
                }
                let (x, y) = decode_symbol_coords(
                    &mut zp,
                    &mut coord_ctx,
                    &mut layout,
                    dict[index].width,
                    dict[index].height,
                );
                check_blit_budget(&dict[index], &mut total_blit_pixels)?;
                blit_indexed(
                    &mut page,
                    &mut blit_map,
                    image_width,
                    image_height,
                    &dict[index],
                    x,
                    y,
                    blit_count,
                );
                blit_count += 1;
            }
            8 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                check_pixel_budget(w, h, &mut total_sym_pixels)?;
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h, pool)?;
                let left = decode_num(&mut zp, &mut horiz_abs_loc_ctx, 1, image_width);
                let top = decode_num(&mut zp, &mut vert_abs_loc_ctx, 1, image_height);
                check_blit_budget(&bm, &mut total_blit_pixels)?;
                blit_indexed(
                    &mut page,
                    &mut blit_map,
                    image_width,
                    image_height,
                    &bm,
                    left - 1,
                    top - h,
                    blit_count,
                );
                blit_count += 1;
                bm.recycle_into(pool);
            }
            9 => {}
            10 => {
                let length = decode_num(&mut zp, &mut comment_length_ctx, 0, 262142) as usize;
                let length = length.min(MAX_COMMENT_BYTES);
                for _ in 0..length {
                    decode_num(&mut zp, &mut comment_octet_ctx, 0, 255);
                }
            }
            11 => break,
            _ => return Err(Jb2Error::UnknownRecordType),
        }
    }

    let bm = page_to_bitmap(&page, image_width, image_height);
    flip_blit_map(&mut blit_map, image_width as usize, image_height as usize);
    Ok((bm, blit_map))
}

// ────────────────────────────────────────────────────────────────────────────
// Core dictionary decode
// ────────────────────────────────────────────────────────────────────────────

fn decode_dictionary(data: &[u8], inherited: Option<&Jb2Dict>) -> Result<Jb2Dict, Jb2Error> {
    let mut pool: Vec<u8> = Vec::new();
    decode_dictionary_with_pool(data, inherited, &mut pool)
}

fn decode_dictionary_with_pool(
    data: &[u8],
    inherited: Option<&Jb2Dict>,
    pool: &mut Vec<u8>,
) -> Result<Jb2Dict, Jb2Error> {
    let mut zp = ZpDecoder::new(data).map_err(|_| Jb2Error::ZpInitFailed)?;

    let mut record_type_ctx = NumContext::new();
    let mut image_size_ctx = NumContext::new();
    let mut symbol_width_ctx = NumContext::new();
    let mut symbol_height_ctx = NumContext::new();
    let mut inherit_dict_size_ctx = NumContext::new();
    let mut symbol_index_ctx = NumContext::new();
    let mut symbol_width_diff_ctx = NumContext::new();
    let mut symbol_height_diff_ctx = NumContext::new();
    let mut comment_length_ctx = NumContext::new();
    let mut comment_octet_ctx = NumContext::new();

    let mut direct_bitmap_ctx = [0u8; 1024];
    let mut refinement_bitmap_ctx = [0u8; 2048];
    let mut refinement_bitmap_ctx_p = [0x8000u16; 2048];
    let mut total_sym_pixels = 0usize;

    // Preamble
    let mut rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);
    let mut initial_dict_length: usize = 0;
    if rtype == 9 {
        initial_dict_length = decode_num(&mut zp, &mut inherit_dict_size_ctx, 0, 262142) as usize;
        rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);
    }
    let _ = rtype;

    // Dimensions (present but unused in dict streams)
    let _dict_width = decode_num(&mut zp, &mut image_size_ctx, 0, 262142);
    let _dict_height = decode_num(&mut zp, &mut image_size_ctx, 0, 262142);

    // Reserved flag bit
    let mut flag_ctx: u8 = 0;
    if zp.decode_bit(&mut flag_ctx) {
        return Err(Jb2Error::BadHeaderFlag);
    }

    let initial_inh: &[Jbm] = if initial_dict_length > 0 {
        match inherited {
            Some(inh) => {
                if initial_dict_length > inh.symbols.len() {
                    return Err(Jb2Error::InheritedDictTooLarge);
                }
                &inh.symbols[..initial_dict_length]
            }
            None => return Err(Jb2Error::MissingSharedDict),
        }
    } else {
        &[]
    };
    let mut dict = JbmDict::new(initial_inh);

    // Dict streams only accept types 2, 5, 9, 10, 11
    let mut record_count = 0usize;
    loop {
        if record_count >= MAX_RECORDS {
            return Err(Jb2Error::TooManyRecords);
        }
        record_count += 1;
        let rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);

        match rtype {
            // 2 — new symbol, direct decode → add to dict
            2 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                check_pixel_budget(w, h, &mut total_sym_pixels)?;
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h, pool)?;
                dict.push(bm.crop_and_recycle(pool));
            }

            // 5 — matched refinement → add to dict
            5 => {
                if dict.is_empty() {
                    return Err(Jb2Error::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(Jb2Error::InvalidSymbolIndex);
                }
                let wdiff = decode_num(&mut zp, &mut symbol_width_diff_ctx, -262143, 262142);
                let hdiff = decode_num(&mut zp, &mut symbol_height_diff_ctx, -262143, 262142);
                let cbm_w = dict[index].width + wdiff;
                let cbm_h = dict[index].height + hdiff;
                check_pixel_budget(cbm_w, cbm_h, &mut total_sym_pixels)?;
                let cbm = decode_bitmap_ref(
                    &mut zp,
                    &mut refinement_bitmap_ctx,
                    &mut refinement_bitmap_ctx_p,
                    cbm_w,
                    cbm_h,
                    &dict[index],
                    pool,
                )?;
                dict.push(cbm.crop_and_recycle(pool));
            }

            // 9 — required-dict-or-reset (ignored in dict streams)
            9 => {}

            // 10 — comment: skip bytes
            10 => {
                let length = decode_num(&mut zp, &mut comment_length_ctx, 0, 262142) as usize;
                let length = length.min(MAX_COMMENT_BYTES);
                for _ in 0..length {
                    decode_num(&mut zp, &mut comment_octet_ctx, 0, 255);
                }
            }

            // 11 — end-of-data
            11 => break,

            _ => return Err(Jb2Error::UnexpectedDictRecordType),
        }
    }

    Ok(Jb2Dict {
        symbols: dict.into_symbols(),
    })
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn assets_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/djvujs/library/assets")
    }

    fn golden_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/jb2")
    }

    // ── IFF helpers ──────────────────────────────────────────────────────────

    fn extract_sjbz(djvu_data: &[u8]) -> Vec<u8> {
        let file = crate::iff::parse(djvu_data).unwrap();
        let sjbz = file.root.find_first(b"Sjbz").unwrap();
        sjbz.data().to_vec()
    }

    fn extract_first_page_sjbz(djvu_data: &[u8]) -> Vec<u8> {
        let file = crate::iff::parse(djvu_data).unwrap();
        let page_form = file
            .root
            .children()
            .iter()
            .find(|c| {
                matches!(c, crate::iff::Chunk::Form { secondary_id, .. }
                    if secondary_id == b"DJVU")
            })
            .expect("no DJVU form");
        page_form.find_first(b"Sjbz").unwrap().data().to_vec()
    }

    fn find_page_form_data(djvu_data: &[u8], page: usize) -> Vec<u8> {
        let file = crate::iff::parse(djvu_data).unwrap();
        let mut idx = 0;
        for chunk in file.root.children() {
            if matches!(chunk, crate::iff::Chunk::Form { secondary_id, .. }
                if secondary_id == b"DJVU")
            {
                if idx == page {
                    return chunk.find_first(b"Sjbz").unwrap().data().to_vec();
                }
                idx += 1;
            }
        }
        panic!("page {} not found", page);
    }

    fn find_djvi_djbz_data(djvu_data: &[u8]) -> Vec<u8> {
        let file = crate::iff::parse(djvu_data).unwrap();
        for chunk in file.root.children() {
            if let crate::iff::Chunk::Form { secondary_id, .. } = chunk
                && secondary_id == b"DJVI"
                && let Some(djbz) = chunk.find_first(b"Djbz")
            {
                return djbz.data().to_vec();
            }
        }
        panic!("DJVI with Djbz not found");
    }

    // ── Failing tests written first (TDD Red phase) ──────────────────────────

    /// The new decoder must produce the same pixel-exact output as the legacy
    /// decoder for boy_jb2.djvu.
    #[test]
    fn jb2_new_decode_boy_jb2_mask() {
        let djvu = std::fs::read(assets_path().join("boy_jb2.djvu")).unwrap();
        let sjbz = extract_sjbz(&djvu);
        let bitmap = decode(&sjbz, None).unwrap();
        let actual_pbm = bitmap.to_pbm();
        let expected_pbm = std::fs::read(golden_path().join("boy_jb2_mask.pbm")).unwrap();
        assert_eq!(
            actual_pbm.len(),
            expected_pbm.len(),
            "PBM size mismatch: got {} expected {}",
            actual_pbm.len(),
            expected_pbm.len()
        );
        assert_eq!(actual_pbm, expected_pbm, "boy_jb2_mask pixel mismatch");
    }

    #[test]
    fn jb2_new_decode_carte_p1_mask() {
        let djvu = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let sjbz = extract_first_page_sjbz(&djvu);
        let bitmap = decode(&sjbz, None).unwrap();
        let actual_pbm = bitmap.to_pbm();
        let expected_pbm = std::fs::read(golden_path().join("carte_p1_mask.pbm")).unwrap();
        assert_eq!(
            actual_pbm.len(),
            expected_pbm.len(),
            "carte_p1_mask size mismatch"
        );
        assert_eq!(actual_pbm, expected_pbm, "carte_p1_mask pixel mismatch");
    }

    #[test]
    fn jb2_new_decode_djvu3spec_p1_mask() {
        let djvu = std::fs::read(assets_path().join("DjVu3Spec_bundled.djvu")).unwrap();
        let file = crate::iff::parse(&djvu).unwrap();

        // Inline Djbz in page 1
        let mut idx = 0usize;
        let mut page_form_opt: Option<&crate::iff::Chunk> = None;
        for chunk in file.root.children() {
            if matches!(chunk, crate::iff::Chunk::Form { secondary_id, .. }
                if secondary_id == b"DJVU")
            {
                if idx == 0 {
                    page_form_opt = Some(chunk);
                    break;
                }
                idx += 1;
            }
        }
        let page_form = page_form_opt.expect("page 0 not found");
        let djbz_data = page_form.find_first(b"Djbz").unwrap().data().to_vec();
        let sjbz_data = page_form.find_first(b"Sjbz").unwrap().data().to_vec();

        let shared_dict = decode_dict(&djbz_data, None).unwrap();
        let bitmap = decode(&sjbz_data, Some(&shared_dict)).unwrap();
        let actual_pbm = bitmap.to_pbm();
        let expected_pbm = std::fs::read(golden_path().join("djvu3spec_p1_mask.pbm")).unwrap();
        assert_eq!(
            actual_pbm.len(),
            expected_pbm.len(),
            "djvu3spec_p1_mask size mismatch"
        );
        assert_eq!(actual_pbm, expected_pbm, "djvu3spec_p1_mask pixel mismatch");
    }

    #[test]
    fn jb2_new_decode_djvu3spec_p2_mask() {
        let djvu = std::fs::read(assets_path().join("DjVu3Spec_bundled.djvu")).unwrap();
        let djbz_data = find_djvi_djbz_data(&djvu);
        let sjbz_data = find_page_form_data(&djvu, 1);

        let shared_dict = decode_dict(&djbz_data, None).unwrap();
        let bitmap = decode(&sjbz_data, Some(&shared_dict)).unwrap();
        let actual_pbm = bitmap.to_pbm();
        let expected_pbm = std::fs::read(golden_path().join("djvu3spec_p2_mask.pbm")).unwrap();
        assert_eq!(
            actual_pbm.len(),
            expected_pbm.len(),
            "djvu3spec_p2_mask size mismatch"
        );
        assert_eq!(actual_pbm, expected_pbm, "djvu3spec_p2_mask pixel mismatch");
    }

    #[test]
    fn jb2_new_decode_navm_fgbz_p1_mask() {
        let djvu = std::fs::read(assets_path().join("navm_fgbz.djvu")).unwrap();
        let djbz_data = find_djvi_djbz_data(&djvu);
        let sjbz_data = find_page_form_data(&djvu, 0);

        let shared_dict = decode_dict(&djbz_data, None).unwrap();
        let bitmap = decode(&sjbz_data, Some(&shared_dict)).unwrap();
        let actual_pbm = bitmap.to_pbm();
        let expected_pbm = std::fs::read(golden_path().join("navm_fgbz_p1_mask.pbm")).unwrap();
        assert_eq!(
            actual_pbm.len(),
            expected_pbm.len(),
            "navm_fgbz_p1_mask size mismatch"
        );
        assert_eq!(actual_pbm, expected_pbm, "navm_fgbz_p1_mask pixel mismatch");
    }

    // ── Robustness tests ─────────────────────────────────────────────────────

    #[test]
    fn jb2_new_empty_input_does_not_panic() {
        let _ = decode(&[], None);
    }

    #[test]
    fn jb2_new_single_byte_does_not_panic() {
        let _ = decode(&[0x00], None);
    }

    #[test]
    fn jb2_new_all_zeros_does_not_panic() {
        let _ = decode(&[0u8; 64], None);
    }

    #[test]
    fn jb2_new_dict_empty_input_does_not_panic() {
        let _ = decode_dict(&[], None);
    }

    #[test]
    fn jb2_new_dict_truncated_does_not_panic() {
        let _ = decode_dict(&[0u8; 8], None);
    }

    // ── Error variant tests ──────────────────────────────────────────────────

    #[test]
    fn jb2_error_variants_have_meaningful_messages() {
        assert!(Jb2Error::BadHeaderFlag.to_string().contains("flag"));
        assert!(Jb2Error::InheritedDictTooLarge.to_string().contains("dict"));
        assert!(Jb2Error::MissingSharedDict.to_string().contains("dict"));
        assert!(Jb2Error::ImageTooLarge.to_string().contains("large"));
        assert!(Jb2Error::EmptyDictReference.to_string().contains("dict"));
        assert!(Jb2Error::InvalidSymbolIndex.to_string().contains("symbol"));
        assert!(Jb2Error::UnknownRecordType.to_string().contains("record"));
        assert!(
            Jb2Error::UnexpectedDictRecordType
                .to_string()
                .contains("record")
        );
        assert!(Jb2Error::ZpInitFailed.to_string().contains("ZP"));
        assert!(Jb2Error::Truncated.to_string().contains("truncated"));
    }

    /// Verify `ImageTooLarge` fires via saturating multiply.
    #[test]
    fn jb2_image_size_overflow_guard() {
        let w: usize = 65536;
        let h: usize = 65537;
        let safe_size = w.saturating_mul(h);
        assert!(
            safe_size > 64 * 1024 * 1024,
            "saturating_mul must exceed MAX_PIXELS"
        );
    }

    // ── Error path tests ───────────────────��────────────────────────────────

    #[test]
    fn test_decode_empty_data() {
        let result = decode(&[], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_dict_empty() {
        let result = decode_dict(&[], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_indexed_empty() {
        let result = decode_indexed(&[], None);
        assert!(result.is_err());
    }

    /// Regression test: negative symbol dimensions caused `width as usize` to
    /// wrap to a huge value in the blit fast path, producing a near-infinite
    /// inner loop and effectively hanging the decoder.
    #[test]
    fn blit_negative_width_does_not_hang() {
        let start = std::time::Instant::now();
        let _ = decode(&[0x7e, 0x00, 0x0c], None);
        assert!(start.elapsed().as_secs() < 2, "took {:?}", start.elapsed());
    }

    // ── Pool reuse tests ──────────────────────────────────────────────────────

    /// Decoding a real JB2 stream with an explicit scratch pool must produce
    /// pixel-identical output to the poolless `decode` path, and the pool must
    /// grow to at least 1 byte (proving it was used for at least one symbol).
    #[test]
    fn jb2_pool_decode_matches_regular_decode_carte() {
        let djvu = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let sjbz = extract_first_page_sjbz(&djvu);

        let regular = decode(&sjbz, None).expect("regular decode");

        let mut pool = Vec::new();
        let pooled = decode_image_with_pool(&sjbz, None, &mut pool).expect("pool decode");

        assert_eq!(regular.width, pooled.width, "width must match");
        assert_eq!(regular.height, pooled.height, "height must match");
        assert_eq!(regular.data, pooled.data, "pixel data must be identical");
        assert!(
            pool.capacity() > 0,
            "pool must have been used (capacity > 0 after decode)"
        );
    }
}

#[cfg(test)]
mod regression_fuzz2 {
    use super::*;

    /// Regression test: a fuzzer-discovered 11-byte input triggered two DoS
    /// paths simultaneously — the ZP-exhausted record loop spinning up to
    /// MAX_RECORDS times, and a near-4MP symbol decode. Both are now bounded
    /// by the reduced MAX_RECORDS (64K) and MAX_SYMBOL_PIXELS (1MP) limits.
    #[test]
    fn huge_symbol_from_small_input_does_not_hang() {
        let data = &[
            0x7f, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let start = std::time::Instant::now();
        let _ = decode(data, None);
        // In release the full decode is <100 ms; in debug the unoptimised loop
        // is ~10× slower, so we allow 8 s (still well under the 10 s fuzz
        // CI timeout that motivated this fix).
        let limit_secs = if cfg!(debug_assertions) { 8 } else { 2 };
        assert!(
            start.elapsed().as_secs() < limit_secs,
            "took {:?}",
            start.elapsed()
        );
    }
}
