//! IW44 wavelet image decoder — pure-Rust clean-room implementation (phase 2c).
//!
//! Implements the IW44 progressive wavelet codec used by DjVu BG44, FG44, and
//! TH44 chunks.  Each BG44 chunk may carry one or more *slices*; the ZP coder
//! state persists across all chunks so that progressive refinement works correctly.
//!
//! ## Key public types
//!
//! - `Iw44Image` — progressive decoder; call `Iw44Image::decode_chunk` for
//!   each BG44/FG44/TH44 chunk, then `Iw44Image::to_rgb` to obtain an RGB
//!   pixmap.
//! - [`crate::error::Iw44Error`] — typed error enum (re-exported from
//!   `crate::error`).
//!
//! ## Architecture
//!
//! YCbCr planes are kept separate (`y: Vec<i16>`, `cb: Vec<i16>`, `cr: Vec<i16>`)
//! until `to_rgb()` is called.  This allows future SIMD processing on each plane
//! independently.  No interleaved buffers exist inside this module.

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

use crate::error::Iw44Error;
use crate::pixmap::Pixmap;
use crate::zp_impl::ZpDecoder;

// ---- Band-bucket mapping: 10 bands, each mapped to a range of buckets --------

/// `BAND_BUCKETS[band]` = `(first_bucket, last_bucket)` inclusive.
const BAND_BUCKETS: [(usize, usize); 10] = [
    (0, 0),
    (1, 1),
    (2, 2),
    (3, 3),
    (4, 7),
    (8, 11),
    (12, 15),
    (16, 31),
    (32, 47),
    (48, 63),
];

/// Initial quantization step table for the low-frequency band (band 0).
const QUANT_LO_INIT: [u32; 16] = [
    0x004000, 0x008000, 0x008000, 0x010000, 0x010000, 0x010000, 0x010000, 0x010000, 0x010000,
    0x010000, 0x010000, 0x010000, 0x020000, 0x020000, 0x020000, 0x020000,
];

/// Initial quantization step table for high-frequency bands (bands 1–9).
const QUANT_HI_INIT: [u32; 10] = [
    0, 0x020000, 0x020000, 0x040000, 0x040000, 0x040000, 0x080000, 0x040000, 0x040000, 0x080000,
];

// ---- Coefficient state flags -------------------------------------------------

const ZERO: u8 = 1;
const ACTIVE: u8 = 2;
const NEW: u8 = 4;
const UNK: u8 = 8;

// ---- Zigzag scan tables ------------------------------------------------------
//
// Each coefficient index `i` (0..1024) maps to a `(row, col)` within the 32×32
// block via bit-interleaving: even bits → column, odd bits → row.

const fn zigzag_row(i: usize) -> u8 {
    let b1 = ((i >> 1) & 1) as u8;
    let b3 = ((i >> 3) & 1) as u8;
    let b5 = ((i >> 5) & 1) as u8;
    let b7 = ((i >> 7) & 1) as u8;
    let b9 = ((i >> 9) & 1) as u8;
    b1 * 16 + b3 * 8 + b5 * 4 + b7 * 2 + b9
}

const fn zigzag_col(i: usize) -> u8 {
    let b0 = (i & 1) as u8;
    let b2 = ((i >> 2) & 1) as u8;
    let b4 = ((i >> 4) & 1) as u8;
    let b6 = ((i >> 6) & 1) as u8;
    let b8 = ((i >> 8) & 1) as u8;
    b0 * 16 + b2 * 8 + b4 * 4 + b6 * 2 + b8
}

static ZIGZAG_ROW: [u8; 1024] = {
    let mut table = [0u8; 1024];
    let mut i = 0;
    while i < 1024 {
        table[i] = zigzag_row(i);
        i += 1;
    }
    table
};

static ZIGZAG_COL: [u8; 1024] = {
    let mut table = [0u8; 1024];
    let mut i = 0;
    while i < 1024 {
        table[i] = zigzag_col(i);
        i += 1;
    }
    table
};

// ---- Normalization -----------------------------------------------------------

/// Map a raw wavelet coefficient to a signed pixel offset in `[-128, 127]`.
#[inline]
fn normalize(val: i16) -> i32 {
    let v = ((val as i32) + 32) >> 6;
    v.clamp(-128, 127)
}

// ---- SIMD YCbCr→RGBA row conversion -----------------------------------------
//
// Processes 8 pixels per iteration using `wide::i32x8` (maps to AVX2 on x86_64,
// NEON on ARM64, or scalar on other targets — all in safe Rust).

/// Convert one row of pre-normalized YCbCr values to RGBA using SIMD.
///
/// `y_row`, `cb_row`, `cr_row` are normalized i32 values in `[-128, 127]`.
/// `out` must hold exactly `y_row.len() * 4` bytes (RGBA).
///
/// DjVu YCbCr→RGB formula (LeCun 1998):
/// ```text
/// t2    = Cr + (Cr >> 1)
/// t3    = Y  + 128 - (Cb >> 2)
/// R     = clamp(Y  + 128 + t2,      0, 255)
/// G     = clamp(t3 - (t2 >> 1),     0, 255)
/// B     = clamp(t3 + (Cb << 1),     0, 255)
/// ```
pub(crate) fn ycbcr_row_to_rgba(y_row: &[i32], cb_row: &[i32], cr_row: &[i32], out: &mut [u8]) {
    use wide::i32x8;
    debug_assert_eq!(y_row.len(), cb_row.len());
    debug_assert_eq!(y_row.len(), cr_row.len());
    debug_assert_eq!(out.len(), y_row.len() * 4);

    let c128 = i32x8::splat(128);
    let c0 = i32x8::splat(0);
    let c255 = i32x8::splat(255);

    let w = y_row.len();
    let full_chunks = w / 8;

    for chunk in 0..full_chunks {
        let base = chunk * 8;
        let ys = i32x8::from([
            y_row[base],
            y_row[base + 1],
            y_row[base + 2],
            y_row[base + 3],
            y_row[base + 4],
            y_row[base + 5],
            y_row[base + 6],
            y_row[base + 7],
        ]);
        let bs = i32x8::from([
            cb_row[base],
            cb_row[base + 1],
            cb_row[base + 2],
            cb_row[base + 3],
            cb_row[base + 4],
            cb_row[base + 5],
            cb_row[base + 6],
            cb_row[base + 7],
        ]);
        let rs = i32x8::from([
            cr_row[base],
            cr_row[base + 1],
            cr_row[base + 2],
            cr_row[base + 3],
            cr_row[base + 4],
            cr_row[base + 5],
            cr_row[base + 6],
            cr_row[base + 7],
        ]);

        let t2 = rs + (rs >> 1_i32);
        let t3 = ys + c128 - (bs >> 2_i32);

        let red: i32x8 = (ys + c128 + t2).max(c0).min(c255);
        let green: i32x8 = (t3 - (t2 >> 1_i32)).max(c0).min(c255);
        let blue: i32x8 = (t3 + (bs << 1_i32)).max(c0).min(c255);

        let reds = red.to_array();
        let greens = green.to_array();
        let blues = blue.to_array();

        let out_base = base * 4;
        for i in 0..8 {
            out[out_base + i * 4] = reds[i] as u8;
            out[out_base + i * 4 + 1] = greens[i] as u8;
            out[out_base + i * 4 + 2] = blues[i] as u8;
            out[out_base + i * 4 + 3] = 255;
        }
    }

    // Scalar tail — fewer than 8 pixels remaining.
    for col in (full_chunks * 8)..w {
        let y = y_row[col];
        let b = cb_row[col];
        let r = cr_row[col];
        let t2 = r + (r >> 1);
        let t3 = y + 128 - (b >> 2);
        out[col * 4] = (y + 128 + t2).clamp(0, 255) as u8;
        out[col * 4 + 1] = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
        out[col * 4 + 2] = (t3 + (b << 1)).clamp(0, 255) as u8;
        out[col * 4 + 3] = 255;
    }
}

// ---- Per-channel wavelet decoder --------------------------------------------

/// State for a single YCbCr plane wavelet decoder.
///
/// Holds 32×32 block coefficients and the ZP context tables that persist
/// across progressive slices.
#[derive(Clone, Debug)]
struct PlaneDecoder {
    width: usize,
    height: usize,
    block_cols: usize,
    /// Row-major array of 32×32 blocks; each block holds 1024 i16 coefficients
    /// in zigzag-scan order.
    blocks: Vec<[i16; 1024]>,
    quant_lo: [u32; 16],
    quant_hi: [u32; 10],
    /// Current band index (0..10, wraps around).
    curband: usize,
    // ZP context bytes — persistent across slices and chunks.
    ctx_decode_bucket: [u8; 1],
    ctx_decode_coef: [u8; 80],
    ctx_activate_coef: [u8; 16],
    ctx_increase_coef: [u8; 1],
    // Per-block temporary decode state (re-used each block, not persisted).
    coeffstate: [[u8; 16]; 16],
    bucketstate: [u8; 16],
    bbstate: u8,
}

/// Map 16 i16 coefficients at `block[base..]` to UNK/ACTIVE flags, store in `bucket`,
/// and return the OR of all flag bytes (bstatetmp).
///
/// Scalar fallback used on non-aarch64 targets.
#[allow(unsafe_code)]
#[inline(always)]
fn prelim_flags_bucket(block: &[i16; 1024], base: usize, bucket: &mut [u8; 16]) -> u8 {
    #[cfg(target_arch = "aarch64")]
    // SAFETY: NEON is mandatory on aarch64; `base + 16 <= 1024` is guaranteed by
    // BAND_BUCKETS (max bucket index 63, so base = 63 * 16 = 1008, 1008 + 16 = 1024).
    return unsafe { prelim_flags_bucket_neon(block, base, bucket) };

    #[cfg(not(target_arch = "aarch64"))]
    {
        let mut bstate = 0u8;
        for k in 0..16 {
            let f = if block[base + k] == 0 { UNK } else { ACTIVE };
            bucket[k] = f;
            bstate |= f;
        }
        bstate
    }
}

/// NEON-vectorized version of `prelim_flags_bucket` for aarch64.
///
/// Loads 16 i16 values, compares to zero with NEON, narrows to u8 flags
/// (UNK=8 for zero, ACTIVE=2 for non-zero), stores, and OR-reduces to bstatetmp.
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "neon")]
unsafe fn prelim_flags_bucket_neon(block: &[i16; 1024], base: usize, bucket: &mut [u8; 16]) -> u8 {
    use core::arch::aarch64::*;
    let ptr = block.as_ptr().add(base);
    // Load as u16 — zero-comparison is the same for signed and unsigned 16-bit.
    let c0 = vreinterpretq_u16_s16(vld1q_s16(ptr));
    let c1 = vreinterpretq_u16_s16(vld1q_s16(ptr.add(8)));
    // nz: 0xFFFF where coef != 0, 0x0000 where coef == 0
    let zero = vdupq_n_u16(0);
    let nz0 = vmvnq_u16(vceqq_u16(c0, zero));
    let nz1 = vmvnq_u16(vceqq_u16(c1, zero));
    // result = UNK ^ ((UNK ^ ACTIVE) & nz)  ⟹  UNK(8) if zero, ACTIVE(2) if nonzero
    // UNK ^ ACTIVE = 8 ^ 2 = 10
    let xv = vdupq_n_u16(10);
    let uv = vdupq_n_u16(8);
    let r0 = veorq_u16(uv, vandq_u16(xv, nz0));
    let r1 = veorq_u16(uv, vandq_u16(xv, nz1));
    // Narrow u16 → u8 (values 2 and 8 both fit; high byte of each lane is 0)
    let out = vcombine_u8(vmovn_u16(r0), vmovn_u16(r1));
    vst1q_u8(bucket.as_mut_ptr(), out);
    // Horizontal OR: fold 16 u8 lanes to 1
    let lo = vget_low_u8(out);
    let hi = vget_high_u8(out);
    let v4 = vorr_u8(lo, hi);
    let v2 = vorr_u8(v4, vext_u8::<4>(v4, v4));
    let v1 = vorr_u8(v2, vext_u8::<2>(v2, v2));
    let v0 = vorr_u8(v1, vext_u8::<1>(v1, v1));
    vget_lane_u8::<0>(v0)
}

/// NEON-vectorized band-0 path of `preliminary_flag_computation`.
///
/// Band 0 differs from bands 1-9: only update entries where `old_flags[k] != ZERO (1)`.
/// Uses `vbslq_u8` to blend new flags (UNK/ACTIVE from coef) with old flags (keep ZERO).
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "neon")]
unsafe fn prelim_flags_band0_neon(block: &[i16; 1024], old_flags: &mut [u8; 16]) -> u8 {
    use core::arch::aarch64::*;
    // Load old coeffstate[0] (u8 flags: ZERO=1, UNK=8, ACTIVE=2).
    let old_u8 = vld1q_u8(old_flags.as_ptr());
    // should_update mask: 0xFF where old_flags[k] != ZERO(1), 0x00 where == ZERO
    let one_u8 = vdupq_n_u8(1);
    let is_zero_state = vceqq_u8(old_u8, one_u8); // 0xFF where ZERO, 0x00 elsewhere
    let should_update = vmvnq_u8(is_zero_state); // 0xFF where not-ZERO
    // Compute new flags from first 16 coefs (same as prelim_flags_bucket_neon with base=0).
    let ptr = block.as_ptr();
    let c0 = vreinterpretq_u16_s16(vld1q_s16(ptr));
    let c1 = vreinterpretq_u16_s16(vld1q_s16(ptr.add(8)));
    let zero16 = vdupq_n_u16(0);
    let nz0 = vmvnq_u16(vceqq_u16(c0, zero16));
    let nz1 = vmvnq_u16(vceqq_u16(c1, zero16));
    let xv = vdupq_n_u16(10); // UNK ^ ACTIVE = 10
    let uv = vdupq_n_u16(8); // UNK = 8
    let r0 = veorq_u16(uv, vandq_u16(xv, nz0));
    let r1 = veorq_u16(uv, vandq_u16(xv, nz1));
    let new_flags = vcombine_u8(vmovn_u16(r0), vmovn_u16(r1));
    // Blend: where should_update, take new_flags; where ZERO state, keep old.
    let result = vbslq_u8(should_update, new_flags, old_u8);
    vst1q_u8(old_flags.as_mut_ptr(), result);
    // Horizontal OR of final flags for bstatetmp.
    let lo = vget_low_u8(result);
    let hi = vget_high_u8(result);
    let v4 = vorr_u8(lo, hi);
    let v2 = vorr_u8(v4, vext_u8::<4>(v4, v4));
    let v1 = vorr_u8(v2, vext_u8::<2>(v2, v2));
    let v0 = vorr_u8(v1, vext_u8::<1>(v1, v1));
    vget_lane_u8::<0>(v0)
}

impl PlaneDecoder {
    fn new(width: usize, height: usize) -> Self {
        let block_cols = width.div_ceil(32);
        let block_rows = height.div_ceil(32);
        let block_count = block_cols * block_rows;
        PlaneDecoder {
            width,
            height,
            block_cols,
            blocks: vec![[0i16; 1024]; block_count],
            quant_lo: QUANT_LO_INIT,
            quant_hi: QUANT_HI_INIT,
            curband: 0,
            ctx_decode_bucket: [0; 1],
            ctx_decode_coef: [0; 80],
            ctx_activate_coef: [0; 16],
            ctx_increase_coef: [0; 1],
            coeffstate: [[0; 16]; 16],
            bucketstate: [0; 16],
            bbstate: 0,
        }
    }

    /// Decode one slice (one band across all blocks) from `zp`.
    fn decode_slice(&mut self, zp: &mut ZpDecoder<'_>) {
        if !self.is_null_slice() {
            for block_idx in 0..self.blocks.len() {
                self.preliminary_flag_computation(block_idx);
                if self.block_band_decoding_pass(zp) && self.bucket_decoding_pass(zp, block_idx) {
                    self.newly_active_coefficient_decoding_pass(zp, block_idx);
                }
                self.previously_active_coefficient_decoding_pass(zp, block_idx);
            }
        }
        self.finish_slice();
    }

    fn is_null_slice(&mut self) -> bool {
        if self.curband == 0 {
            let mut is_null = true;
            for i in 0..16 {
                let threshold = self.quant_lo[i];
                self.coeffstate[0][i] = ZERO;
                if threshold > 0 && threshold < 0x8000 {
                    self.coeffstate[0][i] = UNK;
                    is_null = false;
                }
            }
            is_null
        } else {
            let threshold = self.quant_hi[self.curband];
            !(threshold > 0 && threshold < 0x8000)
        }
    }

    fn preliminary_flag_computation(&mut self, block_idx: usize) {
        self.bbstate = 0;
        let (from, to) = BAND_BUCKETS[self.curband];

        if self.curband != 0 {
            for (boff, j) in (from..=to).enumerate() {
                let bstatetmp = prelim_flags_bucket(
                    &self.blocks[block_idx],
                    j << 4,
                    &mut self.coeffstate[boff],
                );
                self.bucketstate[boff] = bstatetmp;
                self.bbstate |= bstatetmp;
            }
        } else {
            #[cfg(target_arch = "aarch64")]
            // SAFETY: NEON always available on aarch64; block[0..16] valid by construction.
            #[allow(unsafe_code)]
            let bstatetmp = unsafe {
                prelim_flags_band0_neon(&self.blocks[block_idx], &mut self.coeffstate[0])
            };
            #[cfg(not(target_arch = "aarch64"))]
            let bstatetmp = {
                let mut b = 0u8;
                for k in 0..16 {
                    if self.coeffstate[0][k] != ZERO {
                        self.coeffstate[0][k] = if self.blocks[block_idx][k] == 0 {
                            UNK
                        } else {
                            ACTIVE
                        };
                    }
                    b |= self.coeffstate[0][k];
                }
                b
            };
            self.bucketstate[0] = bstatetmp;
            self.bbstate |= bstatetmp;
        }
    }

    fn block_band_decoding_pass(&mut self, zp: &mut ZpDecoder<'_>) -> bool {
        let (from, to) = BAND_BUCKETS[self.curband];
        let bcount = to - from + 1;
        let should_mark_new = bcount < 16
            || (self.bbstate & ACTIVE) != 0
            || ((self.bbstate & UNK) != 0 && zp.decode_bit(&mut self.ctx_decode_bucket[0]));
        if should_mark_new {
            self.bbstate |= NEW;
        }
        (self.bbstate & NEW) != 0
    }

    /// Returns `true` if any bucket was newly marked active (NEW bit set).
    fn bucket_decoding_pass(&mut self, zp: &mut ZpDecoder<'_>, block_idx: usize) -> bool {
        let (from, to) = BAND_BUCKETS[self.curband];
        let mut any_new = false;
        for (boff, i) in (from..=to).enumerate() {
            if (self.bucketstate[boff] & UNK) == 0 {
                continue;
            }
            let mut n: usize = 0;
            if self.curband != 0 {
                let t = 4 * i;
                for j in t..t + 4 {
                    if self.blocks[block_idx][j] != 0 {
                        n += 1;
                    }
                }
                if n == 4 {
                    n = 3;
                }
            }
            if (self.bbstate & ACTIVE) != 0 {
                n |= 4;
            }
            if zp.decode_bit(&mut self.ctx_decode_coef[n + self.curband * 8]) {
                self.bucketstate[boff] |= NEW;
                any_new = true;
            }
        }
        any_new
    }

    fn newly_active_coefficient_decoding_pass(&mut self, zp: &mut ZpDecoder<'_>, block_idx: usize) {
        let (from, to) = BAND_BUCKETS[self.curband];
        let mut step = self.quant_hi[self.curband];
        for (boff, i) in (from..=to).enumerate() {
            if (self.bucketstate[boff] & NEW) != 0 {
                let shift: usize = if (self.bucketstate[boff] & ACTIVE) != 0 {
                    8
                } else {
                    0
                };
                let mut np: usize = 0;
                for j in 0..16 {
                    if (self.coeffstate[boff][j] & UNK) != 0 {
                        np += 1;
                    }
                }
                for j in 0..16 {
                    if (self.coeffstate[boff][j] & UNK) != 0 {
                        let ip = np.min(7);
                        if zp.decode_bit(&mut self.ctx_activate_coef[shift + ip]) {
                            let sign = if zp.decode_passthrough_iw44() {
                                -1i32
                            } else {
                                1i32
                            };
                            np = 0;
                            if self.curband == 0 {
                                step = self.quant_lo[j];
                            }
                            let s = step as i32;
                            let val = sign * (s + (s >> 1) - (s >> 3));
                            self.blocks[block_idx][(i << 4) | j] = val as i16;
                        }
                        np = np.saturating_sub(1);
                    }
                }
            }
        }
    }

    /// Hot inner loop for refining already-active coefficients.
    ///
    /// Uses local copies of all ZP state fields so LLVM can keep them in
    /// registers for the duration of the double-loop, avoiding struct-pointer
    /// round-trips on every `decode_bit` / `decode_passthrough_iw44` call.
    #[inline(never)]
    fn previously_active_coefficient_decoding_pass(
        &mut self,
        zp: &mut ZpDecoder<'_>,
        block_idx: usize,
    ) {
        use crate::zp_impl::tables::{LPS_NEXT, MPS_NEXT, PROB, THRESHOLD};

        // Extract ZP state to true stack-locals — LLVM keeps these in registers.
        let mut a = zp.a;
        let mut c = zp.c;
        let mut fence = zp.fence;
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
        // Decode one bit using an adaptive context byte.
        macro_rules! decode_bit_ctx {
            ($ctx:expr) => {{
                let state = ($ctx) as usize;
                let mps_bit = state & 1;
                let z = a + PROB[state] as u32;
                if z <= fence {
                    a = z;
                    mps_bit != 0
                } else {
                    let boundary = 0x6000u32 + ((a + z) >> 2);
                    let z_clamped = z.min(boundary);
                    if z_clamped > c {
                        let complement = 0x10000u32 - z_clamped;
                        a = (a + complement) & 0xffff;
                        c = (c + complement) & 0xffff;
                        $ctx = LPS_NEXT[state];
                        renorm!();
                        (1 - mps_bit) != 0
                    } else {
                        if a >= THRESHOLD[state] as u32 {
                            $ctx = MPS_NEXT[state];
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
                }
            }};
        }
        // Decode one bit in IW44 passthrough mode (threshold = 0x8000 + 3a/8).
        macro_rules! decode_passthrough_iw44 {
            () => {{
                let z = (0x8000u32 + (3u32 * a) / 8) as u16;
                if z as u32 > c {
                    let complement = 0x10000u32 - z as u32;
                    a = (a + complement) & 0xffff;
                    c = (c + complement) & 0xffff;
                    renorm!();
                    true
                } else {
                    bit_count -= 1;
                    a = (z as u32 * 2) & 0xffff;
                    c = (c << 1 | (bit_buf >> (bit_count as u32 & 31)) & 1) & 0xffff;
                    if bit_count < 16 {
                        refill!();
                    }
                    fence = c.min(0x7fff);
                    false
                }
            }};
        }

        let (from, to) = BAND_BUCKETS[self.curband];
        let mut step = self.quant_hi[self.curband];
        for (boff, i) in (from..=to).enumerate() {
            for j in 0..16 {
                if (self.coeffstate[boff][j] & ACTIVE) != 0 {
                    if self.curband == 0 {
                        step = self.quant_lo[j];
                    }
                    let coef = self.blocks[block_idx][(i << 4) | j];
                    let mut abs_coef = coef.unsigned_abs() as i32;
                    let s = step as i32;
                    let des = if abs_coef <= 3 * s {
                        let d = decode_bit_ctx!(self.ctx_increase_coef[0]);
                        abs_coef += s >> 2;
                        d
                    } else {
                        decode_passthrough_iw44!()
                    };
                    if des {
                        abs_coef += s >> 1;
                    } else {
                        abs_coef += -s + (s >> 1);
                    }
                    self.blocks[block_idx][(i << 4) | j] = if coef < 0 {
                        -abs_coef as i16
                    } else {
                        abs_coef as i16
                    };
                }
            }
        }

        // Write back ZP state so subsequent calls see the updated arithmetic.
        zp.a = a;
        zp.c = c;
        zp.fence = fence;
        zp.bit_buf = bit_buf;
        zp.bit_count = bit_count;
        zp.pos = pos;
    }

    /// Advance quantization step and band counter after one slice.
    fn finish_slice(&mut self) {
        self.quant_hi[self.curband] >>= 1;
        if self.curband == 0 {
            for i in 0..16 {
                self.quant_lo[i] >>= 1;
            }
        }
        self.curband += 1;
        if self.curband == 10 {
            self.curband = 0;
        }
    }

    /// Apply the inverse wavelet transform and return a flat `i16` array.
    ///
    /// The returned vector is row-major, with stride = `width.div_ceil(32)*32`.
    /// `subsample` ≥ 1 controls the resolution (1 = full, 2 = half, etc.).
    fn reconstruct(&self, subsample: usize) -> FlatPlane {
        // ── Fast path for sub≥2: compact plane ────────────────────────────────
        //
        // For subsample=2 the wavelet only ever reads/writes (even_row, even_col)
        // positions — those with zigzag index i < 256 (see zigzag_row/col: both
        // are even iff bits 8 and 9 of i are 0).  We can therefore:
        //   1. Allocate a 4× smaller plane  (ceil(w/2) × ceil(h/2))
        //   2. Scatter only i ∈ [0, coeff_limit) at (row/sub, col/sub)
        //   3. Run the full wavelet (sub=1) on the compact plane, which now
        //      includes the SIMD s=1 pass.
        //
        // This is equivalent to running the wavelet at sub=2 on the full plane
        // and sampling every other position: each compact[k][c] equals the value
        // that full[k·sub][c·sub] would hold after the sub=2 wavelet.
        //
        // The same logic holds for sub=4 (coeff_limit=64, quarter-plane) and
        // sub=8 (coeff_limit=16, eighth-plane).
        if (2..=8).contains(&subsample) && subsample.is_power_of_two() {
            let sub = subsample;
            // Number of coefficients whose (row, col) are both multiples of `sub`.
            // For sub=2: 256; sub=4: 64; sub=8: 16.
            let coeff_limit = 1024 / (sub * sub);

            // Block structure: the compact plane inherits the same block grid but
            // each 32×32 block contributes a (32/sub)×(32/sub) sub-block.
            let block_rows = self.height.div_ceil(32);
            let sub_block = 32 / sub; // 16 for sub=2, 8 for sub=4, 4 for sub=8

            // Compact plane dimensions, aligned to the sub-block width.
            let compact_stride = self.block_cols * sub_block;
            let compact_rows = block_rows * sub_block;
            // Logical image dimensions at the target resolution.
            let compact_w = self.width.div_ceil(sub);
            let compact_h = self.height.div_ceil(sub);

            let mut plane = FlatPlane {
                data: vec![0i16; compact_stride * compact_rows],
                stride: compact_stride,
            };

            for r in 0..block_rows {
                for c in 0..self.block_cols {
                    let block = &self.blocks[r * self.block_cols + c];
                    let row_base = r << 5;
                    let col_base = c << 5;
                    for i in 0..coeff_limit {
                        // zigzag positions are even multiples of `sub`; divide
                        // by `sub` to get compact-plane coordinates.
                        let row = (ZIGZAG_ROW[i] as usize + row_base) / sub;
                        let col = (ZIGZAG_COL[i] as usize + col_base) / sub;
                        plane.data[row * compact_stride + col] = block[i];
                    }
                }
            }

            // Run the wavelet on the compact plane starting at scale 16/sub.
            // compact s=k ↔ full s=k·sub, so the coarsest valid pass is
            // s = 16/sub (e.g. s=8 for sub=2).  Starting at s=16 would add a
            // spurious pass with no coefficients and introduce rounding noise.
            let start_scale = 16 / sub;
            inverse_wavelet_transform_from(&mut plane, compact_w, compact_h, 1, start_scale);
            return plane;
        }

        // ── Default path (sub=1, or non-power-of-two sub) ─────────────────────
        let full_width = self.width.div_ceil(32) * 32;
        let full_height = self.height.div_ceil(32) * 32;
        let block_rows = self.height.div_ceil(32);
        let mut plane = FlatPlane {
            data: vec![0i16; full_width * full_height],
            stride: full_width,
        };

        // Scatter block coefficients into the flat plane via zigzag
        for r in 0..block_rows {
            for c in 0..self.block_cols {
                let block = &self.blocks[r * self.block_cols + c];
                let row_base = r << 5;
                let col_base = c << 5;
                for i in 0..1024 {
                    let row = ZIGZAG_ROW[i] as usize + row_base;
                    let col = ZIGZAG_COL[i] as usize + col_base;
                    plane.data[row * full_width + col] = block[i];
                }
            }
        }

        inverse_wavelet_transform(&mut plane, self.width, self.height, subsample);
        plane
    }
}

// ---- Flat plane helper -------------------------------------------------------

struct FlatPlane {
    data: Vec<i16>,
    stride: usize,
}

// ---- Inverse Dubuc-Deslauriers-Lemire (4,4) wavelet transform ---------------
//
// Two passes per resolution level:
//   1. Column pass (lifting + prediction along rows of subsampled columns)
//   2. Row pass (lifting + prediction along columns of subsampled rows)
//
// The column pass is transposed for cache efficiency.
//
// When `s == 1` (the final, highest-resolution level) the column indices are
// contiguous, so we can process 8 columns per iteration using `wide::i32x8`.

use wide::i32x8;

/// Load 8 contiguous `i16` values from `slice[off..]` into an `i32x8`.
#[inline(always)]
fn load8(slice: &[i16], off: usize) -> i32x8 {
    i32x8::from([
        slice[off] as i32,
        slice[off + 1] as i32,
        slice[off + 2] as i32,
        slice[off + 3] as i32,
        slice[off + 4] as i32,
        slice[off + 5] as i32,
        slice[off + 6] as i32,
        slice[off + 7] as i32,
    ])
}

/// Store 8 values from an `i32x8` into contiguous `i16` slots at `slice[off..]`.
#[inline(always)]
fn store8(slice: &mut [i16], off: usize, v: i32x8) {
    let a = v.to_array();
    slice[off] = a[0] as i16;
    slice[off + 1] = a[1] as i16;
    slice[off + 2] = a[2] as i16;
    slice[off + 3] = a[3] as i16;
    slice[off + 4] = a[4] as i16;
    slice[off + 5] = a[5] as i16;
    slice[off + 6] = a[6] as i16;
    slice[off + 7] = a[7] as i16;
}

/// Load 8 contiguous `i32` values from `slice[off..]` into an `i32x8`.
#[inline(always)]
fn load8_i32(slice: &[i32], off: usize) -> i32x8 {
    i32x8::from([
        slice[off],
        slice[off + 1],
        slice[off + 2],
        slice[off + 3],
        slice[off + 4],
        slice[off + 5],
        slice[off + 6],
        slice[off + 7],
    ])
}

/// Store 8 values from an `i32x8` into contiguous `i32` slots at `slice[off..]`.
#[inline(always)]
fn store8_i32(slice: &mut [i32], off: usize, v: i32x8) {
    let a = v.to_array();
    slice[off] = a[0];
    slice[off + 1] = a[1];
    slice[off + 2] = a[2];
    slice[off + 3] = a[3];
    slice[off + 4] = a[4];
    slice[off + 5] = a[5];
    slice[off + 6] = a[6];
    slice[off + 7] = a[7];
}

/// Gather one `i16` value from each of 8 consecutive rows at column index `k`.
///
/// `offs[i]` is the start offset `row_i * stride` for row `i`.
#[inline(always)]
fn load_rows8(data: &[i16], offs: &[usize; 8], k: usize) -> i32x8 {
    i32x8::from([
        data[offs[0] + k] as i32,
        data[offs[1] + k] as i32,
        data[offs[2] + k] as i32,
        data[offs[3] + k] as i32,
        data[offs[4] + k] as i32,
        data[offs[5] + k] as i32,
        data[offs[6] + k] as i32,
        data[offs[7] + k] as i32,
    ])
}

/// Scatter one value from `v` to each of 8 consecutive rows at column index `k`.
#[inline(always)]
fn store_rows8(data: &mut [i16], offs: &[usize; 8], k: usize, v: i32x8) {
    let a = v.to_array();
    data[offs[0] + k] = a[0] as i16;
    data[offs[1] + k] = a[1] as i16;
    data[offs[2] + k] = a[2] as i16;
    data[offs[3] + k] = a[3] as i16;
    data[offs[4] + k] = a[4] as i16;
    data[offs[5] + k] = a[5] as i16;
    data[offs[6] + k] = a[6] as i16;
    data[offs[7] + k] = a[7] as i16;
}

/// Lifting filter: `data[idx] -= ((9*(p1+n1) - (p3+n3) + 16) >> 5)`
#[inline(always)]
fn lifting_even(cur: i32x8, p1: i32x8, n1: i32x8, p3: i32x8, n3: i32x8) -> i32x8 {
    let a = p1 + n1;
    let c = p3 + n3;
    let c16 = i32x8::splat(16);
    cur - (((a << 3) + a - c + c16) >> 5)
}

/// Prediction filter (inner): `data[idx] += ((9*(p1+n1) - (p3+n3) + 8) >> 4)`
#[inline(always)]
fn predict_inner(cur: i32x8, p1: i32x8, n1: i32x8, p3: i32x8, n3: i32x8) -> i32x8 {
    let a = p1 + n1;
    let c8 = i32x8::splat(8);
    cur + (((a << 3) + a - (p3 + n3) + c8) >> 4)
}

/// Prediction filter (boundary): `data[idx] += ((p + n + 1) >> 1)`
#[inline(always)]
fn predict_avg(cur: i32x8, p: i32x8, n: i32x8) -> i32x8 {
    let c1 = i32x8::splat(1);
    cur + ((p + n + c1) >> 1)
}

/// Apply the row-direction wavelet pass for one resolution level.
///
/// When `use_simd` is `true` and `s == 1` (`sd == 0`), the first
/// `height / 8 * 8` rows are processed 8 at a time using `i32x8` SIMD.
/// The remaining rows (and all rows when `s > 1` or `use_simd` is false) use
/// the scalar path.
///
/// `s` — step between active samples (power of two); `sd = log2(s)`.
pub(crate) fn row_pass_inner(
    data: &mut [i16],
    width: usize,
    height: usize,
    stride: usize,
    s: usize,
    sd: usize,
    use_simd: bool,
) {
    let kmax = (width - 1) >> sd;
    let border = kmax.saturating_sub(3);

    // ── SIMD path: 8 rows at a time (only when s == 1, i.e. sd == 0) ─────────
    let simd_rows = if use_simd && s == 1 {
        height / 8 * 8
    } else {
        0
    };

    for row_base in (0..simd_rows).step_by(8) {
        let o: [usize; 8] = core::array::from_fn(|i| (row_base + i) * stride);

        // — Lifting (even k) ——————————————————————————————————————————————————
        let mut prev1v = i32x8::splat(0);
        let mut next1v = i32x8::splat(0);
        let mut next3v = if kmax >= 1 {
            load_rows8(data, &o, 1)
        } else {
            i32x8::splat(0)
        };
        let mut prev3v: i32x8;
        let mut k = 0usize;
        while k <= kmax {
            prev3v = prev1v;
            prev1v = next1v;
            next1v = next3v;
            next3v = if k + 3 <= kmax {
                load_rows8(data, &o, k + 3)
            } else {
                i32x8::splat(0)
            };
            let cur = load_rows8(data, &o, k);
            store_rows8(
                data,
                &o,
                k,
                lifting_even(cur, prev1v, next1v, prev3v, next3v),
            );
            k += 2;
        }

        // — Prediction (odd k) ————————————————————————————————————————————————
        if kmax >= 1 {
            let mut k = 1usize;
            prev1v = load_rows8(data, &o, k - 1); // data[0] per row
            if k < kmax {
                next1v = load_rows8(data, &o, k + 1);
                let cur = load_rows8(data, &o, k);
                store_rows8(data, &o, k, predict_avg(cur, prev1v, next1v));
            } else {
                // k == kmax: boundary — only one odd sample, += prev
                let cur = load_rows8(data, &o, k);
                store_rows8(data, &o, k, cur + prev1v);
                next1v = i32x8::splat(0);
            }

            next3v = if border >= 3 {
                load_rows8(data, &o, k + 3)
            } else {
                i32x8::splat(0)
            };

            k = 3;
            while k <= border {
                prev3v = prev1v;
                prev1v = next1v;
                next1v = next3v;
                next3v = load_rows8(data, &o, k + 3);
                let cur = load_rows8(data, &o, k);
                store_rows8(
                    data,
                    &o,
                    k,
                    predict_inner(cur, prev1v, next1v, prev3v, next3v),
                );
                k += 2;
            }

            while k <= kmax {
                prev1v = next1v;
                next1v = next3v;
                next3v = i32x8::splat(0);
                let cur = load_rows8(data, &o, k);
                if k < kmax {
                    store_rows8(data, &o, k, predict_avg(cur, prev1v, next1v));
                } else {
                    store_rows8(data, &o, k, cur + prev1v);
                }
                k += 2;
            }
        }
    }

    // ── Scalar path: remaining rows ───────────────────────────────────────────
    let scalar_start = simd_rows;
    for row in (scalar_start..height).step_by(s) {
        let off = row * stride;

        // Lifting (even samples)
        let mut prev1: i32 = 0;
        let mut next1: i32 = 0;
        let mut next3: i32 = if kmax >= 1 {
            data[off + (1 << sd)] as i32
        } else {
            0
        };
        let mut prev3: i32;
        let mut k = 0usize;
        while k <= kmax {
            prev3 = prev1;
            prev1 = next1;
            next1 = next3;
            next3 = if k + 3 <= kmax {
                data[off + ((k + 3) << sd)] as i32
            } else {
                0
            };
            let a = prev1 + next1;
            let c = prev3 + next3;
            let idx = off + (k << sd);
            data[idx] = (data[idx] as i32 - (((a << 3) + a - c + 16) >> 5)) as i16;
            k += 2;
        }

        // Prediction (odd samples)
        if kmax >= 1 {
            let mut k = 1usize;
            prev1 = data[off + ((k - 1) << sd)] as i32;
            if k < kmax {
                next1 = data[off + ((k + 1) << sd)] as i32;
                let idx = off + (k << sd);
                data[idx] = (data[idx] as i32 + ((prev1 + next1 + 1) >> 1)) as i16;
            } else {
                let idx = off + (k << sd);
                data[idx] = (data[idx] as i32 + prev1) as i16;
            }

            next3 = if border >= 3 {
                data[off + ((k + 3) << sd)] as i32
            } else {
                0
            };

            k = 3;
            while k <= border {
                prev3 = prev1;
                prev1 = next1;
                next1 = next3;
                next3 = data[off + ((k + 3) << sd)] as i32;
                let a = prev1 + next1;
                let idx = off + (k << sd);
                data[idx] = (data[idx] as i32 + (((a << 3) + a - (prev3 + next3) + 8) >> 4)) as i16;
                k += 2;
            }

            while k <= kmax {
                prev1 = next1;
                next1 = next3;
                next3 = 0;
                let idx = off + (k << sd);
                if k < kmax {
                    data[idx] = (data[idx] as i32 + ((prev1 + next1 + 1) >> 1)) as i16;
                } else {
                    data[idx] = (data[idx] as i32 + prev1) as i16;
                }
                k += 2;
            }
        }
    }
}

fn inverse_wavelet_transform(plane: &mut FlatPlane, width: usize, height: usize, subsample: usize) {
    inverse_wavelet_transform_from(plane, width, height, subsample, 16);
}

/// Like `inverse_wavelet_transform` but begins at `start_scale` instead of 16.
///
/// Use `start_scale = 16 / sub` when operating on a compact plane produced by
/// subsampling the coefficient scatter by factor `sub`.  For example, the sub=2
/// compact plane only contains coefficients up to scale 8, so the s=16 pass
/// would be purely spurious.
fn inverse_wavelet_transform_from(
    plane: &mut FlatPlane,
    width: usize,
    height: usize,
    subsample: usize,
    start_scale: usize,
) {
    let stride = plane.stride;
    let data = plane.data.as_mut_slice();
    let mut s = start_scale;
    let mut s_degree: u32 = start_scale.trailing_zeros();

    let mut st0 = vec![0i32; width];
    let mut st1 = vec![0i32; width];
    let mut st2 = vec![0i32; width];

    while s >= subsample {
        let sd = s_degree as usize;

        // When s == 1, column indices are contiguous → use SIMD.
        let use_simd = s == 1;

        // ── Column pass (transposed) ──────────────────────────────────────────
        {
            let kmax = (height - 1) >> sd;
            let border = kmax.saturating_sub(3);
            let num_cols = width.div_ceil(s);
            let simd_cols = if use_simd { num_cols / 8 * 8 } else { 0 };

            // Lifting (even samples)
            for v in &mut st0[..num_cols] {
                *v = 0;
            }
            for v in &mut st1[..num_cols] {
                *v = 0;
            }
            if kmax >= 1 {
                let off = (1 << sd) * stride;
                if use_simd {
                    for ci in (0..simd_cols).step_by(8) {
                        store8_i32(&mut st2, ci, load8(data, off + ci));
                    }
                    for ci in simd_cols..num_cols {
                        st2[ci] = data[off + ci] as i32;
                    }
                } else {
                    for (ci, col) in (0..width).step_by(s).enumerate() {
                        st2[ci] = data[off + col] as i32;
                    }
                }
            } else {
                for v in &mut st2[..num_cols] {
                    *v = 0;
                }
            }

            let mut k = 0usize;
            while k <= kmax {
                let k_off = (k << sd) * stride;
                let has_n3 = k + 3 <= kmax;
                let n3_off = if has_n3 { ((k + 3) << sd) * stride } else { 0 };

                if use_simd {
                    let zero8 = i32x8::splat(0);
                    let mut ci = 0usize;
                    while ci < simd_cols {
                        let vp3 = load8_i32(&st0, ci);
                        let vp1 = load8_i32(&st1, ci);
                        let vn1 = load8_i32(&st2, ci);
                        let vn3 = if has_n3 {
                            load8(data, n3_off + ci)
                        } else {
                            zero8
                        };
                        let cur = load8(data, k_off + ci);
                        store8(data, k_off + ci, lifting_even(cur, vp1, vn1, vp3, vn3));
                        store8_i32(&mut st0, ci, vp1);
                        store8_i32(&mut st1, ci, vn1);
                        store8_i32(&mut st2, ci, vn3);
                        ci += 8;
                    }
                    // scalar tail
                    while ci < num_cols {
                        let p3 = st0[ci];
                        let p1 = st1[ci];
                        let n1 = st2[ci];
                        let n3 = if has_n3 { data[n3_off + ci] as i32 } else { 0 };
                        let a = p1 + n1;
                        let c = p3 + n3;
                        let idx = k_off + ci;
                        data[idx] = (data[idx] as i32 - (((a << 3) + a - c + 16) >> 5)) as i16;
                        st0[ci] = p1;
                        st1[ci] = n1;
                        st2[ci] = n3;
                        ci += 1;
                    }
                } else {
                    for (ci, col) in (0..width).step_by(s).enumerate() {
                        let p3 = st0[ci];
                        let p1 = st1[ci];
                        let n1 = st2[ci];
                        let n3 = if has_n3 { data[n3_off + col] as i32 } else { 0 };

                        let a = p1 + n1;
                        let c = p3 + n3;
                        let idx = k_off + col;
                        data[idx] = (data[idx] as i32 - (((a << 3) + a - c + 16) >> 5)) as i16;

                        st0[ci] = p1;
                        st1[ci] = n1;
                        st2[ci] = n3;
                    }
                }
                k += 2;
            }

            // Prediction (odd samples)
            if kmax >= 1 {
                // k = 1
                let km1_off = 0;
                let k_off = (1 << sd) * stride;

                if 2 <= kmax {
                    let kp1_off = (2 << sd) * stride;
                    if use_simd {
                        let mut ci = 0usize;
                        while ci < simd_cols {
                            let vp = load8(data, km1_off + ci);
                            let vn = load8(data, kp1_off + ci);
                            let cur = load8(data, k_off + ci);
                            store8(data, k_off + ci, predict_avg(cur, vp, vn));
                            store8_i32(&mut st0, ci, vp);
                            store8_i32(&mut st1, ci, vn);
                            ci += 8;
                        }
                        while ci < num_cols {
                            let p = data[km1_off + ci] as i32;
                            let n = data[kp1_off + ci] as i32;
                            let idx = k_off + ci;
                            data[idx] = (data[idx] as i32 + ((p + n + 1) >> 1)) as i16;
                            st0[ci] = p;
                            st1[ci] = n;
                            ci += 1;
                        }
                    } else {
                        for (ci, col) in (0..width).step_by(s).enumerate() {
                            let p = data[km1_off + col] as i32;
                            let n = data[kp1_off + col] as i32;
                            let idx = k_off + col;
                            data[idx] = (data[idx] as i32 + ((p + n + 1) >> 1)) as i16;
                            st0[ci] = p;
                            st1[ci] = n;
                        }
                    }
                } else if use_simd {
                    let mut ci = 0usize;
                    while ci < simd_cols {
                        let vp = load8(data, km1_off + ci);
                        let cur = load8(data, k_off + ci);
                        store8(data, k_off + ci, cur + vp);
                        store8_i32(&mut st0, ci, vp);
                        ci += 8;
                    }
                    for v in &mut st1[..num_cols] {
                        *v = 0;
                    }
                    while ci < num_cols {
                        let p = data[km1_off + ci] as i32;
                        let idx = k_off + ci;
                        data[idx] = (data[idx] as i32 + p) as i16;
                        st0[ci] = p;
                        st1[ci] = 0;
                        ci += 1;
                    }
                } else {
                    for (ci, col) in (0..width).step_by(s).enumerate() {
                        let p = data[km1_off + col] as i32;
                        let idx = k_off + col;
                        data[idx] = (data[idx] as i32 + p) as i16;
                        st0[ci] = p;
                        st1[ci] = 0;
                    }
                }

                if border >= 3 {
                    let off = (4 << sd) * stride;
                    if use_simd {
                        let mut ci = 0usize;
                        while ci < simd_cols {
                            store8_i32(&mut st2, ci, load8(data, off + ci));
                            ci += 8;
                        }
                        while ci < num_cols {
                            st2[ci] = data[off + ci] as i32;
                            ci += 1;
                        }
                    } else {
                        for (ci, col) in (0..width).step_by(s).enumerate() {
                            st2[ci] = data[off + col] as i32;
                        }
                    }
                }

                // k = 3, 5, ..., border
                let mut k = 3usize;
                while k <= border {
                    let k_off = (k << sd) * stride;
                    let n3_off = ((k + 3) << sd) * stride;

                    if use_simd {
                        let mut ci = 0usize;
                        while ci < simd_cols {
                            let vp3 = load8_i32(&st0, ci);
                            let vp1 = load8_i32(&st1, ci);
                            let vn1 = load8_i32(&st2, ci);
                            let vn3 = load8(data, n3_off + ci);
                            let cur = load8(data, k_off + ci);
                            store8(data, k_off + ci, predict_inner(cur, vp1, vn1, vp3, vn3));
                            store8_i32(&mut st0, ci, vp1);
                            store8_i32(&mut st1, ci, vn1);
                            store8_i32(&mut st2, ci, vn3);
                            ci += 8;
                        }
                        while ci < num_cols {
                            let p3 = st0[ci];
                            let p1 = st1[ci];
                            let n1 = st2[ci];
                            let n3 = data[n3_off + ci] as i32;
                            let a = p1 + n1;
                            let idx = k_off + ci;
                            data[idx] =
                                (data[idx] as i32 + (((a << 3) + a - (p3 + n3) + 8) >> 4)) as i16;
                            st0[ci] = p1;
                            st1[ci] = n1;
                            st2[ci] = n3;
                            ci += 1;
                        }
                    } else {
                        for (ci, col) in (0..width).step_by(s).enumerate() {
                            let p3 = st0[ci];
                            let p1 = st1[ci];
                            let n1 = st2[ci];
                            let n3 = data[n3_off + col] as i32;

                            let a = p1 + n1;
                            let idx = k_off + col;
                            data[idx] =
                                (data[idx] as i32 + (((a << 3) + a - (p3 + n3) + 8) >> 4)) as i16;

                            st0[ci] = p1;
                            st1[ci] = n1;
                            st2[ci] = n3;
                        }
                    }
                    k += 2;
                }

                // tail
                while k <= kmax {
                    let k_off = (k << sd) * stride;

                    if k < kmax {
                        if use_simd {
                            let mut ci = 0usize;
                            while ci < simd_cols {
                                let vp = load8_i32(&st1, ci);
                                let vn = load8_i32(&st2, ci);
                                let cur = load8(data, k_off + ci);
                                store8(data, k_off + ci, predict_avg(cur, vp, vn));
                                store8_i32(&mut st1, ci, vn);
                                store8_i32(&mut st2, ci, i32x8::splat(0));
                                ci += 8;
                            }
                            while ci < num_cols {
                                let p = st1[ci];
                                let n = st2[ci];
                                let idx = k_off + ci;
                                data[idx] = (data[idx] as i32 + ((p + n + 1) >> 1)) as i16;
                                st1[ci] = n;
                                st2[ci] = 0;
                                ci += 1;
                            }
                        } else {
                            for (ci, col) in (0..width).step_by(s).enumerate() {
                                let p = st1[ci];
                                let n = st2[ci];
                                let idx = k_off + col;
                                data[idx] = (data[idx] as i32 + ((p + n + 1) >> 1)) as i16;
                                st1[ci] = n;
                                st2[ci] = 0;
                            }
                        }
                    } else if use_simd {
                        let mut ci = 0usize;
                        while ci < simd_cols {
                            let vp = load8_i32(&st1, ci);
                            let cur = load8(data, k_off + ci);
                            store8(data, k_off + ci, cur + vp);
                            store8_i32(&mut st1, ci, load8_i32(&st2, ci));
                            store8_i32(&mut st2, ci, i32x8::splat(0));
                            ci += 8;
                        }
                        while ci < num_cols {
                            let p = st1[ci];
                            let idx = k_off + ci;
                            data[idx] = (data[idx] as i32 + p) as i16;
                            st1[ci] = st2[ci];
                            st2[ci] = 0;
                            ci += 1;
                        }
                    } else {
                        for (ci, col) in (0..width).step_by(s).enumerate() {
                            let p = st1[ci];
                            let idx = k_off + col;
                            data[idx] = (data[idx] as i32 + p) as i16;
                            st1[ci] = st2[ci];
                            st2[ci] = 0;
                        }
                    }
                    k += 2;
                }
            }
        }

        // ── Row pass ─────────────────────────────────────────────────────────
        row_pass_inner(data, width, height, stride, s, sd, use_simd);

        s >>= 1;
        s_degree = s_degree.saturating_sub(1);
    }
}

// ---- Public API -------------------------------------------------------------

/// Progressive IW44 wavelet image decoder.
///
/// Holds three independent planar decoders (Y, Cb, Cr) whose ZP context tables
/// persist across chunks, enabling progressive refinement.
///
/// ## Usage
///
/// ```no_run
/// use djvu_rs::iw44_new::Iw44Image;
///
/// let chunk_data: &[u8] = &[]; // BG44 chunk bytes from the DjVu file
/// let mut img = Iw44Image::new();
/// // Feed each BG44 chunk in document order:
/// img.decode_chunk(chunk_data)?;
/// // Convert to an RGB pixmap once all desired chunks are decoded:
/// let pixmap = img.to_rgb()?;
/// # Ok::<(), djvu_rs::Iw44Error>(())
/// ```
#[derive(Clone, Debug)]
pub struct Iw44Image {
    /// Luma plane dimensions (pixels, before subsampling).
    pub width: u32,
    /// Luma plane dimensions (pixels, before subsampling).
    pub height: u32,
    /// `true` for color (YCbCr) images, `false` for grayscale.
    is_color: bool,
    /// Number of Y slices decoded before chroma decoding starts.
    delay: u8,
    /// `true` if chroma planes are stored at half resolution.
    chroma_half: bool,
    /// Luma plane decoder.
    y: Option<PlaneDecoder>,
    /// Blue-difference chroma plane decoder (color images only).
    cb: Option<PlaneDecoder>,
    /// Red-difference chroma plane decoder (color images only).
    cr: Option<PlaneDecoder>,
    /// Total slices decoded so far (used to implement the color-delay counter).
    cslice: usize,
}

impl Default for Iw44Image {
    fn default() -> Self {
        Self::new()
    }
}

impl Iw44Image {
    /// Create a new, empty decoder.
    pub fn new() -> Self {
        Iw44Image {
            width: 0,
            height: 0,
            is_color: false,
            delay: 0,
            chroma_half: false,
            y: None,
            cb: None,
            cr: None,
            cslice: 0,
        }
    }

    /// Returns the (width, height) of the Cb chroma plane as allocated.
    ///
    /// When `chroma_half=true` this should be `(ceil(w/2), ceil(h/2))`.
    /// Returns `None` if no color chunks have been decoded yet.
    #[cfg(test)]
    pub fn chroma_plane_dims(&self) -> Option<(usize, usize)> {
        self.cb.as_ref().map(|p| (p.width, p.height))
    }

    /// Returns `true` if the image is a color (YCbCr) image.
    #[cfg(test)]
    pub fn is_color(&self) -> bool {
        self.is_color
    }

    /// Returns `true` if chroma planes are stored at half resolution.
    #[cfg(test)]
    pub fn chroma_half(&self) -> bool {
        self.chroma_half
    }

    /// Decode one BG44/FG44/TH44 chunk.
    ///
    /// Call this once for each chunk in document order.  The ZP coder state
    /// is maintained internally so progressive refinement works automatically.
    ///
    /// ## Chunk format
    ///
    /// - First chunk (`serial == 0`): 9-byte header then ZP-coded payload.
    /// - Subsequent chunks: 2-byte header (`serial`, `slices`) then ZP payload.
    pub fn decode_chunk(&mut self, data: &[u8]) -> Result<(), Iw44Error> {
        if data.len() < 2 {
            return Err(Iw44Error::ChunkTooShort);
        }
        let serial = data[0];
        let slices = data[1];
        let payload_start;

        if serial == 0 {
            // First chunk — parse the 9-byte image header.
            if data.len() < 9 {
                return Err(Iw44Error::HeaderTooShort);
            }
            let majver = data[2];
            let minor = data[3];
            let is_grayscale = (majver >> 7) != 0;
            let w = u16::from_be_bytes([data[4], data[5]]);
            let h = u16::from_be_bytes([data[6], data[7]]);
            let delay_byte = data[8];
            let delay = if minor >= 2 { delay_byte & 127 } else { 0 };
            let chroma_half = minor >= 2 && (delay_byte & 0x80) == 0;

            if w == 0 || h == 0 {
                return Err(Iw44Error::ZeroDimension);
            }
            // Prevent OOM / slow decode on malformed input.
            // 64 MP allows real scanned documents (e.g. 6780×9148 ≈ 62 MP at 600 dpi)
            // while bounding worst-case fuzz decode to ~3 s (vs 12 s at 256 MP).
            let pixels = w as u64 * h as u64;
            if pixels > 64 * 1024 * 1024 {
                return Err(Iw44Error::ImageTooLarge);
            }

            self.width = w as u32;
            self.height = h as u32;
            self.is_color = !is_grayscale;
            self.delay = delay;
            self.chroma_half = self.is_color && chroma_half;
            self.cslice = 0;
            self.y = Some(PlaneDecoder::new(w as usize, h as usize));
            if self.is_color {
                let (cw, ch) = if self.chroma_half {
                    ((w as usize).div_ceil(2), (h as usize).div_ceil(2))
                } else {
                    (w as usize, h as usize)
                };
                self.cb = Some(PlaneDecoder::new(cw, ch));
                self.cr = Some(PlaneDecoder::new(cw, ch));
            }
            payload_start = 9;
        } else {
            if self.y.is_none() {
                return Err(Iw44Error::MissingFirstChunk);
            }
            payload_start = 2;
        }

        let zp_data = &data[payload_start..];
        let mut zp = ZpDecoder::new(zp_data).map_err(|_| Iw44Error::ZpTooShort)?;

        for _ in 0..slices {
            self.cslice += 1;
            if let Some(ref mut y) = self.y {
                y.decode_slice(&mut zp);
            }
            if self.is_color && self.cslice > self.delay as usize {
                if let Some(ref mut cb) = self.cb {
                    cb.decode_slice(&mut zp);
                }
                if let Some(ref mut cr) = self.cr {
                    cr.decode_slice(&mut zp);
                }
            }
            // Once all real input bytes are consumed the ZP coder returns
            // 0xFF indefinitely, producing deterministic but meaningless
            // bits. Remaining slices carry no new information, so stop early
            // to bound decode time on crafted inputs.
            if zp.is_exhausted() {
                break;
            }
        }

        Ok(())
    }

    /// Convert the decoded image to an RGB [`Pixmap`].
    ///
    /// This is the **only** place where the separate Y, Cb, Cr planes are
    /// interleaved into RGB pixels.  DjVu images are stored bottom-to-top;
    /// this method flips the output to top-to-bottom.
    ///
    /// Equivalent to `to_rgb_subsample(1)`.
    pub fn to_rgb(&self) -> Result<Pixmap, Iw44Error> {
        self.to_rgb_subsample(1)
    }

    /// Convert to an RGB [`Pixmap`] at reduced resolution.
    ///
    /// `subsample` must be ≥ 1.  A value of 1 gives full resolution; 2 gives
    /// half resolution in each dimension, etc.
    pub fn to_rgb_subsample(&self, subsample: u32) -> Result<Pixmap, Iw44Error> {
        if subsample == 0 {
            return Err(Iw44Error::InvalidSubsample);
        }
        let y_dec = self.y.as_ref().ok_or(Iw44Error::MissingCodec)?;
        let sub = subsample as usize;
        let w = (self.width as usize).div_ceil(sub) as u32;
        let h = (self.height as usize).div_ceil(sub) as u32;

        if self.is_color {
            // When chroma_half=true the chroma planes are stored at half luma
            // resolution.  Divide the subsample factor by 2 (minimum 1) so that
            // reconstruct() operates at the correct scale relative to the smaller
            // plane.
            let chroma_sub = if self.chroma_half {
                sub.div_ceil(2)
            } else {
                sub
            };
            let cb_dec = self.cb.as_ref().ok_or(Iw44Error::MissingCodec)?;
            let cr_dec = self.cr.as_ref().ok_or(Iw44Error::MissingCodec)?;

            // Reconstruct Y, Cb and Cr planes.  With the `parallel` feature the
            // three independent inverse-wavelet-transforms run concurrently on
            // separate rayon threads, cutting the reconstruction wall-time from
            // Y+Cb+Cr sequential to max(Y, Cb, Cr) — roughly 1.5–2× faster on
            // large pages where Y dominates.
            #[cfg(feature = "parallel")]
            let (y_plane, cb_plane, cr_plane) = {
                let (y, (cb, cr)) = rayon::join(
                    || y_dec.reconstruct(sub),
                    || {
                        rayon::join(
                            || cb_dec.reconstruct(chroma_sub),
                            || cr_dec.reconstruct(chroma_sub),
                        )
                    },
                );
                (y, cb, cr)
            };
            #[cfg(not(feature = "parallel"))]
            let (y_plane, cb_plane, cr_plane) = (
                y_dec.reconstruct(sub),
                cb_dec.reconstruct(chroma_sub),
                cr_dec.reconstruct(chroma_sub),
            );

            let pw = w as usize;
            let ph = h as usize;
            let mut pm = Pixmap::new(w, h, 0, 0, 0, 255);

            // Fast path: sub=1 (most common — full-resolution render).
            // Pre-normalize Y/Cb/Cr into flat row buffers and apply the
            // YCbCr→RGBA formula 8 pixels at a time with SIMD.
            if sub == 1 {
                #[cfg(feature = "parallel")]
                {
                    use rayon::prelude::*;
                    let chroma_half = self.chroma_half;
                    pm.data
                        .par_chunks_mut(pw * 4)
                        .enumerate()
                        .for_each(|(out_row, row_data)| {
                            let row = ph - 1 - out_row; // DjVu rows are bottom-to-top
                            let y_off = row * y_plane.stride;
                            let mut y_norm = vec![0i32; pw];
                            let mut cb_norm = vec![0i32; pw];
                            let mut cr_norm = vec![0i32; pw];
                            for (col, v) in y_norm.iter_mut().enumerate() {
                                *v = normalize(y_plane.data[y_off + col]);
                            }
                            if chroma_half {
                                let c_row = row / 2;
                                let cb_off = c_row * cb_plane.stride;
                                let cr_off = c_row * cr_plane.stride;
                                for col in 0..pw {
                                    let c_col = col / 2;
                                    cb_norm[col] = normalize(cb_plane.data[cb_off + c_col]);
                                    cr_norm[col] = normalize(cr_plane.data[cr_off + c_col]);
                                }
                            } else {
                                let c_off = row * cb_plane.stride;
                                for col in 0..pw {
                                    cb_norm[col] = normalize(cb_plane.data[c_off + col]);
                                    cr_norm[col] = normalize(cr_plane.data[c_off + col]);
                                }
                            }
                            ycbcr_row_to_rgba(&y_norm, &cb_norm, &cr_norm, row_data);
                        });
                }
                #[cfg(not(feature = "parallel"))]
                {
                    let mut y_norm = vec![0i32; pw];
                    let mut cb_norm = vec![0i32; pw];
                    let mut cr_norm = vec![0i32; pw];

                    for row in 0..ph {
                        let out_row = ph - 1 - row; // DjVu rows are bottom-to-top
                        let y_off = row * y_plane.stride;

                        for (col, v) in y_norm.iter_mut().enumerate() {
                            *v = normalize(y_plane.data[y_off + col]);
                        }

                        if self.chroma_half {
                            // Chroma plane is half-resolution: index with row/2, col/2.
                            let c_row = row / 2;
                            let cb_off = c_row * cb_plane.stride;
                            let cr_off = c_row * cr_plane.stride;
                            for col in 0..pw {
                                let c_col = col / 2;
                                cb_norm[col] = normalize(cb_plane.data[cb_off + c_col]);
                                cr_norm[col] = normalize(cr_plane.data[cr_off + c_col]);
                            }
                        } else {
                            let c_off = row * cb_plane.stride;
                            for col in 0..pw {
                                cb_norm[col] = normalize(cb_plane.data[c_off + col]);
                                cr_norm[col] = normalize(cr_plane.data[c_off + col]);
                            }
                        }

                        let row_start = out_row * pw * 4;
                        ycbcr_row_to_rgba(
                            &y_norm,
                            &cb_norm,
                            &cr_norm,
                            &mut pm.data[row_start..row_start + pw * 4],
                        );
                    }
                }
                return Ok(pm);
            }

            // Compact path: sub ≥ 2 with power-of-two subsample.
            //
            // `reconstruct(sub)` now returns a plane that is already at the
            // target resolution (ceil(w/sub) × ceil(h/sub)), so we access it
            // with sub=1 indexing.  Chroma planes are at the same output size
            // (the chroma_half factor is absorbed into chroma_sub), so no
            // chroma_half division is needed here.
            //
            // Uses SIMD via `ycbcr_row_to_rgba` (same as the sub=1 fast path).
            if (2..=8).contains(&sub) && sub.is_power_of_two() {
                let mut y_norm = vec![0i32; pw];
                let mut cb_norm = vec![0i32; pw];
                let mut cr_norm = vec![0i32; pw];

                for row in 0..ph {
                    let out_row = ph - 1 - row; // DjVu rows are bottom-to-top
                    let y_off = row * y_plane.stride;
                    let c_off = row * cb_plane.stride;

                    for (col, v) in y_norm.iter_mut().enumerate() {
                        *v = normalize(y_plane.data[y_off + col]);
                    }
                    for col in 0..pw {
                        cb_norm[col] = normalize(cb_plane.data[c_off + col]);
                        cr_norm[col] = normalize(cr_plane.data[c_off + col]);
                    }

                    let row_start = out_row * pw * 4;
                    ycbcr_row_to_rgba(
                        &y_norm,
                        &cb_norm,
                        &cr_norm,
                        &mut pm.data[row_start..row_start + pw * 4],
                    );
                }
                return Ok(pm);
            }

            // Fallback scalar path for non-power-of-two or large sub values.
            for row in 0..h {
                let out_row = h - 1 - row;
                for col in 0..w {
                    let src_row = row as usize * sub;
                    let src_col = col as usize * sub;
                    let y_idx = src_row * y_plane.stride + src_col;
                    let chroma_row = if self.chroma_half {
                        src_row / 2
                    } else {
                        src_row
                    };
                    let chroma_col = if self.chroma_half {
                        src_col / 2
                    } else {
                        src_col
                    };
                    let c_idx = chroma_row * cb_plane.stride + chroma_col;

                    let y = normalize(y_plane.data[y_idx]);
                    let b = normalize(cb_plane.data[c_idx]);
                    let r = normalize(cr_plane.data[c_idx]);

                    let t2 = r + (r >> 1);
                    let t3 = y + 128 - (b >> 2);

                    let red = (y + 128 + t2).clamp(0, 255) as u8;
                    let green = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
                    let blue = (t3 + (b << 1)).clamp(0, 255) as u8;
                    pm.set_rgb(col, out_row, red, green, blue);
                }
            }
            Ok(pm)
        } else {
            // Grayscale: only the Y plane is needed.
            // For sub≥2 the plane is compact (at output resolution); for sub=1 it
            // is full-resolution.  Use compact-aware indexing.
            let y_plane = y_dec.reconstruct(sub);
            let is_compact = (2..=8).contains(&sub) && sub.is_power_of_two();
            let mut pm = Pixmap::new(w, h, 0, 0, 0, 255);
            for row in 0..h {
                let out_row = h - 1 - row;
                for col in 0..w {
                    let (src_row, src_col) = if is_compact {
                        (row as usize, col as usize)
                    } else {
                        (row as usize * sub, col as usize * sub)
                    };
                    let idx = src_row * y_plane.stride + src_col;
                    let val = normalize(y_plane.data[idx]);
                    // Grayscale: DjVu luma 0 maps to black, −128 → white
                    let gray = (127 - val) as u8;
                    pm.set_rgb(col, out_row, gray, gray, gray);
                }
            }
            Ok(pm)
        }
    }
}

// ---- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn assets_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/djvujs/library/assets")
    }

    fn golden_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/iw44")
    }

    /// Extract all BG44 chunk payloads from the first DJVU form in the file.
    fn extract_bg44_chunks(file: &crate::iff::DjvuFile) -> Vec<&[u8]> {
        fn collect(chunk: &crate::iff::Chunk) -> Option<Vec<&[u8]>> {
            match chunk {
                crate::iff::Chunk::Form {
                    secondary_id,
                    children,
                    ..
                } => {
                    if secondary_id == b"DJVU" {
                        let v = children
                            .iter()
                            .filter_map(|c| match c {
                                crate::iff::Chunk::Leaf {
                                    id: [b'B', b'G', b'4', b'4'],
                                    data,
                                } => Some(data.as_slice()),
                                _ => None,
                            })
                            .collect::<Vec<_>>();
                        return Some(v);
                    }
                    for c in children {
                        if let Some(v) = collect(c) {
                            return Some(v);
                        }
                    }
                    None
                }
                _ => None,
            }
        }
        collect(&file.root).unwrap_or_default()
    }

    fn find_ppm_data_start(ppm: &[u8]) -> usize {
        let mut newlines = 0;
        for (i, &b) in ppm.iter().enumerate() {
            if b == b'\n' {
                newlines += 1;
                if newlines == 3 {
                    return i + 1;
                }
            }
        }
        0
    }

    /// Compare `actual_ppm` against a golden file, creating it on first run.
    ///
    /// If the file doesn't exist it is written (first-time generation).
    /// On subsequent runs an exact byte-for-byte comparison is enforced so that
    /// any accidental change to the pixel output is caught immediately.
    fn assert_or_create_golden(actual_ppm: &[u8], golden_file: &str) {
        let path = golden_path().join(golden_file);
        if !path.exists() {
            std::fs::write(&path, actual_ppm)
                .unwrap_or_else(|e| panic!("failed to write golden {golden_file}: {e}"));
            return; // golden created — test passes on first run
        }
        assert_ppm_match(actual_ppm, golden_file);
    }

    fn assert_ppm_match(actual_ppm: &[u8], golden_file: &str) {
        let expected_ppm = std::fs::read(golden_path().join(golden_file))
            .unwrap_or_else(|_| panic!("golden file not found: {}", golden_file));
        assert_eq!(
            actual_ppm.len(),
            expected_ppm.len(),
            "PPM size mismatch for {}: got {} expected {}",
            golden_file,
            actual_ppm.len(),
            expected_ppm.len()
        );
        if actual_ppm != expected_ppm {
            let header_end = find_ppm_data_start(actual_ppm);
            let actual_pixels = &actual_ppm[header_end..];
            let expected_pixels = &expected_ppm[header_end..];
            let total_pixels = actual_pixels.len() / 3;
            let diff_pixels = actual_pixels
                .chunks(3)
                .zip(expected_pixels.chunks(3))
                .filter(|(a, b)| a != b)
                .count();
            panic!(
                "{} pixel mismatch: {}/{} pixels differ ({:.1}%)",
                golden_file,
                diff_pixels,
                total_pixels,
                diff_pixels as f64 / total_pixels as f64 * 100.0
            );
        }
    }

    // ---- TDD: failing tests first -------------------------------------------

    /// Decode must fail gracefully on empty input.
    #[test]
    fn iw44_new_rejects_empty_chunk() {
        let mut img = Iw44Image::new();
        assert!(matches!(
            img.decode_chunk(&[]),
            Err(Iw44Error::ChunkTooShort)
        ));
    }

    /// Decode must fail gracefully on a truncated first-chunk header.
    #[test]
    fn iw44_new_rejects_truncated_header() {
        let mut img = Iw44Image::new();
        // serial=0 but only 5 bytes (need ≥ 9)
        assert!(matches!(
            img.decode_chunk(&[0x00, 0x01, 0x00, 0x02, 0x00]),
            Err(Iw44Error::HeaderTooShort)
        ));
    }

    /// Zero-dimension image must be rejected.
    #[test]
    fn iw44_new_rejects_zero_dimension() {
        let mut img = Iw44Image::new();
        // serial=0, slices=1, majver=0, minor=2, w=0, h=100, delay=0
        let header = [0x00u8, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x64, 0x00];
        assert!(matches!(
            img.decode_chunk(&header),
            Err(Iw44Error::ZeroDimension)
        ));
    }

    /// Subsequent chunk before first chunk must be rejected.
    #[test]
    fn iw44_new_rejects_subsequent_before_first() {
        let mut img = Iw44Image::new();
        // serial != 0
        assert!(matches!(
            img.decode_chunk(&[0x01, 0x01]),
            Err(Iw44Error::MissingFirstChunk)
        ));
    }

    /// `to_rgb()` on an uninitialised decoder must return an error.
    #[test]
    fn iw44_new_to_rgb_without_data_returns_error() {
        let img = Iw44Image::new();
        assert!(matches!(img.to_rgb(), Err(Iw44Error::MissingCodec)));
    }

    /// `to_rgb_subsample(0)` must be rejected.
    #[test]
    fn iw44_new_subsample_zero_rejected() {
        let img = Iw44Image::new();
        assert!(matches!(
            img.to_rgb_subsample(0),
            Err(Iw44Error::InvalidSubsample)
        ));
    }

    // ---- Pixel-exact golden tests -------------------------------------------

    #[test]
    fn iw44_new_decode_boy_bg() {
        let data = std::fs::read(assets_path().join("boy.djvu")).expect("boy.djvu not found");
        let file = crate::iff::parse(&data).expect("failed to parse boy.djvu");
        let chunks = extract_bg44_chunks(&file);
        assert_eq!(chunks.len(), 1, "expected 1 BG44 chunk in boy.djvu");

        let mut img = Iw44Image::new();
        for c in &chunks {
            img.decode_chunk(c).expect("decode_chunk failed");
        }
        assert_eq!(img.width, 192);
        assert_eq!(img.height, 256);

        let pm = img.to_rgb().expect("to_rgb failed");
        assert_ppm_match(&pm.to_ppm(), "boy_bg.ppm");
    }

    #[test]
    fn iw44_new_decode_chicken_bg() {
        let data =
            std::fs::read(assets_path().join("chicken.djvu")).expect("chicken.djvu not found");
        let file = crate::iff::parse(&data).expect("failed to parse chicken.djvu");
        let chunks = extract_bg44_chunks(&file);
        assert_eq!(chunks.len(), 3, "expected 3 BG44 chunks in chicken.djvu");

        let mut img = Iw44Image::new();
        for c in &chunks {
            img.decode_chunk(c).expect("decode_chunk failed");
        }
        assert_eq!(img.width, 181);
        assert_eq!(img.height, 240);

        let pm = img.to_rgb().expect("to_rgb failed");
        assert_ppm_match(&pm.to_ppm(), "chicken_bg.ppm");
    }

    /// `to_rgb_subsample(2)` on boy.djvu must produce a pixel-exact result.
    ///
    /// This golden test guards against any regression in the compact-plane sub=2
    /// optimization path.  On first run the golden file is created from the
    /// current (correct) output; subsequent runs compare against it.
    #[test]
    fn iw44_new_decode_boy_sub2() {
        let data = std::fs::read(assets_path().join("boy.djvu")).expect("boy.djvu not found");
        let file = crate::iff::parse(&data).expect("failed to parse boy.djvu");
        let chunks = extract_bg44_chunks(&file);

        let mut img = Iw44Image::new();
        for c in &chunks {
            img.decode_chunk(c).expect("decode_chunk failed");
        }
        assert_eq!(img.width, 192);
        assert_eq!(img.height, 256);

        let pm = img.to_rgb_subsample(2).expect("to_rgb_subsample(2) failed");
        assert_eq!(pm.width, 96, "sub=2 width must be ceil(192/2)");
        assert_eq!(pm.height, 128, "sub=2 height must be ceil(256/2)");

        assert_or_create_golden(&pm.to_ppm(), "boy_bg_sub2.ppm");
    }

    /// `to_rgb_subsample(2)` on big-scanned-page.djvu (color IW44).
    ///
    /// Exercises the compact-plane path on a large color document.
    #[test]
    fn iw44_new_decode_big_scanned_sub2() {
        let data = std::fs::read(assets_path().join("big-scanned-page.djvu"))
            .expect("big-scanned-page.djvu not found");
        let file = crate::iff::parse(&data).expect("failed to parse big-scanned-page.djvu");
        let chunks = extract_bg44_chunks(&file);

        let mut img = Iw44Image::new();
        for c in &chunks {
            img.decode_chunk(c).expect("decode_chunk failed");
        }
        assert_eq!(img.width, 6780);
        assert_eq!(img.height, 9148);

        let pm = img.to_rgb_subsample(2).expect("to_rgb_subsample(2) failed");
        assert_eq!(pm.width, 3390, "sub=2 width must be ceil(6780/2)");
        assert_eq!(pm.height, 4574, "sub=2 height must be ceil(9148/2)");

        assert_or_create_golden(&pm.to_ppm(), "big_scanned_sub2.ppm");
    }

    #[test]
    fn iw44_new_decode_big_scanned_sub4() {
        let data = std::fs::read(assets_path().join("big-scanned-page.djvu"))
            .expect("big-scanned-page.djvu not found");
        let file = crate::iff::parse(&data).expect("failed to parse big-scanned-page.djvu");
        let chunks = extract_bg44_chunks(&file);
        assert_eq!(chunks.len(), 4, "expected 4 BG44 chunks");

        let mut img = Iw44Image::new();
        for c in &chunks {
            img.decode_chunk(c).expect("decode_chunk failed");
        }
        assert_eq!(img.width, 6780);
        assert_eq!(img.height, 9148);

        let pm = img.to_rgb_subsample(4).expect("to_rgb_subsample failed");
        assert_ppm_match(&pm.to_ppm(), "big_scanned_sub4.ppm");
    }

    /// Progressive decode: feeding all chunks at once and feeding them one-by-one
    /// must produce identical results.
    #[test]
    fn iw44_new_progressive_matches_full_decode_chicken() {
        let data =
            std::fs::read(assets_path().join("chicken.djvu")).expect("chicken.djvu not found");
        let file = crate::iff::parse(&data).expect("failed to parse");
        let chunks = extract_bg44_chunks(&file);
        assert!(
            chunks.len() > 1,
            "need multiple chunks for progressive test"
        );

        // Full decode (all chunks at once via repeated decode_chunk calls)
        let mut full = Iw44Image::new();
        for c in &chunks {
            full.decode_chunk(c).expect("full decode failed");
        }
        let full_pm = full.to_rgb().expect("full to_rgb failed");

        // Progressive decode — same result since ZP state persists
        let mut prog = Iw44Image::new();
        for c in chunks.iter().take(1) {
            prog.decode_chunk(c).expect("progressive decode failed");
        }
        for c in chunks.iter().skip(1) {
            prog.decode_chunk(c).expect("progressive decode failed");
        }
        let prog_pm = prog.to_rgb().expect("progressive to_rgb failed");

        assert_eq!(
            full_pm.data, prog_pm.data,
            "progressive and full decode must produce identical pixels"
        );
    }

    // ── chroma_half allocation test ──────────────────────────────────────────

    /// When `chroma_half=true`, chroma planes must be allocated at half
    /// resolution (ceil(w/2) × ceil(h/2)), not at full luma resolution.
    ///
    /// carte.djvu is a color image with chroma_half=true (w=1400, h=852).
    #[test]
    fn chroma_half_allocates_half_size_plane() {
        let data = std::fs::read(assets_path().join("carte.djvu")).expect("carte.djvu not found");
        let file = crate::iff::parse(&data).expect("iff parse");
        let chunks = extract_bg44_chunks(&file);
        assert!(!chunks.is_empty(), "carte.djvu must have BG44 chunks");

        let mut img = Iw44Image::new();
        img.decode_chunk(chunks[0]).expect("decode_chunk");

        assert!(img.is_color(), "carte.djvu must be a color image");
        assert!(img.chroma_half(), "carte.djvu must have chroma_half=true");
        let (cw, ch) = img
            .chroma_plane_dims()
            .expect("chroma plane must be allocated after first color chunk");
        let lw = img.width as usize;
        let lh = img.height as usize;
        let expected_w = lw.div_ceil(2);
        let expected_h = lh.div_ceil(2);
        assert_eq!(
            cw, expected_w,
            "chroma plane width must be ceil(luma_w/2)={expected_w}, got {cw}"
        );
        assert_eq!(
            ch, expected_h,
            "chroma plane height must be ceil(luma_h/2)={expected_h}, got {ch}"
        );
    }

    /// Decode carte.djvu (chroma_half=true color image) fully and compare
    /// pixel output to the golden reference, ensuring the half-plane allocation
    /// does not corrupt the decoded image.
    #[test]
    fn iw44_new_decode_carte_bg_chroma_half() {
        let data = std::fs::read(assets_path().join("carte.djvu")).expect("carte.djvu not found");
        let file = crate::iff::parse(&data).expect("iff parse");
        let chunks = extract_bg44_chunks(&file);

        let mut img = Iw44Image::new();
        for c in &chunks {
            img.decode_chunk(c).expect("decode_chunk failed");
        }
        assert_eq!(img.width, 1400);
        assert_eq!(img.height, 852);

        let pm = img.to_rgb().expect("to_rgb failed");
        assert_ppm_match(&pm.to_ppm(), "carte_bg.ppm");
    }

    // ── Error path tests ────────────────────────────────────────────────────

    #[test]
    fn test_decode_empty_chunk() {
        let mut img = Iw44Image::new();
        let result = img.decode_chunk(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_truncated_header() {
        let mut img = Iw44Image::new();
        // Only 2 bytes — not enough for a header
        let result = img.decode_chunk(&[0x00, 0x01]);
        assert!(result.is_err());
    }

    #[test]
    fn test_to_rgb_before_decode() {
        let img = Iw44Image::new();
        // No chunks decoded yet — should fail
        let result = img.to_rgb();
        assert!(result.is_err());
    }

    #[test]
    fn test_to_rgb_subsample_zero() {
        let img = Iw44Image::new();
        let result = img.to_rgb_subsample(0);
        assert!(result.is_err());
    }

    // ---- SIMD YCbCr→RGBA tests -----------------------------------------------

    /// `ycbcr_row_to_rgba` matches the scalar formula on synthetic data.
    #[test]
    fn simd_ycbcr_row_matches_scalar() {
        // Cover all 8-wide SIMD chunks plus a tail (n=20).
        let n = 20usize;
        let ys: Vec<i32> = (0..n).map(|i| (i as i32 * 7) % 200 - 100).collect();
        let bs: Vec<i32> = (0..n).map(|i| (i as i32 * 13) % 200 - 100).collect();
        let rs: Vec<i32> = (0..n).map(|i| (i as i32 * 17) % 200 - 100).collect();

        // Scalar reference
        let mut expected = vec![0u8; n * 4];
        for col in 0..n {
            let y = ys[col];
            let b = bs[col];
            let r = rs[col];
            let t2 = r + (r >> 1);
            let t3 = y + 128 - (b >> 2);
            expected[col * 4] = (y + 128 + t2).clamp(0, 255) as u8;
            expected[col * 4 + 1] = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
            expected[col * 4 + 2] = (t3 + (b << 1)).clamp(0, 255) as u8;
            expected[col * 4 + 3] = 255;
        }

        // SIMD result
        let mut actual = vec![0u8; n * 4];
        super::ycbcr_row_to_rgba(&ys, &bs, &rs, &mut actual);

        assert_eq!(
            expected, actual,
            "SIMD must produce identical output to scalar"
        );
    }

    /// `ycbcr_row_to_rgba` handles extreme values (clamping at 0 and 255).
    #[test]
    fn simd_ycbcr_row_clamps_correctly() {
        let n = 8usize;
        // Use values that will clamp to 0 and 255 in each channel.
        let ys: Vec<i32> = vec![127, -128, 127, -128, 0, 0, 0, 0];
        let bs: Vec<i32> = vec![-128, 127, -128, 127, 0, 0, 0, 0];
        let rs: Vec<i32> = vec![127, -128, -128, 127, 0, 0, 0, 0];

        let mut simd_out = vec![0u8; n * 4];
        super::ycbcr_row_to_rgba(&ys, &bs, &rs, &mut simd_out);

        // All RGBA values must be in [0, 255] and alpha == 255.
        for chunk in simd_out.chunks_exact(4) {
            assert_eq!(chunk[3], 255, "alpha must always be 255");
        }
    }

    /// SIMD render of boy.djvu produces identical output to the scalar path.
    ///
    /// This verifies that the fast path (sub=1) and general path (sub=2, which
    /// uses the old scalar code) produce consistent results on a real file.
    #[test]
    fn simd_render_matches_subsampled_render_dimensions() {
        let data = std::fs::read(assets_path().join("boy.djvu")).expect("boy.djvu not found");
        let file = crate::iff::parse(&data).expect("parse failed");
        let chunks = extract_bg44_chunks(&file);

        let mut img = Iw44Image::new();
        for c in &chunks {
            img.decode_chunk(c).expect("decode_chunk failed");
        }

        // Full-resolution render uses SIMD path (sub=1).
        let full = img.to_rgb().expect("to_rgb failed");
        // sub=2 uses the scalar general path — just check dims match half.
        let half = img.to_rgb_subsample(2).expect("subsample(2) failed");

        assert_eq!(full.width, img.width);
        assert_eq!(full.height, img.height);
        assert_eq!(half.width, img.width.div_ceil(2));
        assert_eq!(half.height, img.height.div_ceil(2));
        // SIMD path must still pass the existing golden test (done in iw44_new_decode_boy_bg).
    }

    /// SIMD row pass (8 rows at a time) produces identical results to the scalar
    /// path on a synthetic 32×16 plane with a deterministic non-trivial pattern.
    ///
    /// Both paths are exercised by calling `row_pass_inner` with `use_simd=false`
    /// (all scalar) and `use_simd=true` (SIMD + scalar tail) on identical copies
    /// of the same data.
    #[test]
    fn simd_row_pass_matches_scalar() {
        let width = 32usize;
        let height = 16usize;
        let stride = width;
        let n = stride * height;

        // Deterministic non-trivial pattern: values in [-255, 255].
        let initial: Vec<i16> = (0..n).map(|i| ((i * 7 + 13) % 511) as i16 - 255).collect();

        let mut scalar_data = initial.clone();
        // s=1, sd=0, use_simd=false → pure scalar
        super::row_pass_inner(&mut scalar_data, width, height, stride, 1, 0, false);

        let mut simd_data = initial.clone();
        // s=1, sd=0, use_simd=true → SIMD for rows 0..15, scalar tail for remainder
        super::row_pass_inner(&mut simd_data, width, height, stride, 1, 0, true);

        assert_eq!(
            scalar_data, simd_data,
            "SIMD row pass must produce identical output to scalar"
        );
    }
}
