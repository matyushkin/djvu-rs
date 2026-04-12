//! IW44 wavelet encoder — produces BG44/FG44/TH44 chunk payloads.
//!
//! ## Algorithm overview
//!
//! 1. Convert RGB → IW44 YCbCr (or accept grayscale directly).
//! 2. Apply the forward IW44 wavelet transform to each plane.
//! 3. Gather transformed coefficients into 32×32 blocks (zigzag scan).
//! 4. Progressively encode bands 0–9 using ZP arithmetic coding.
//! 5. Assemble BG44 chunk payloads with the required headers.
//!
//! The forward transform is the exact inverse of the analysis filter in
//! `iw44_new::inverse_wavelet_transform`. Passes run from s=1 (finest) to
//! s=16 (coarsest); within each pass the predict step is undone before the
//! lifting step (reversed vs the synthesis filter).

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

use crate::pixmap::{GrayPixmap, Pixmap};
#[cfg(feature = "std")]
use crate::zp_impl::encoder::ZpEncoder;

// ---- Zigzag tables (replicated from iw44_new) --------------------------------

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

// ---- Band and quantization constants -----------------------------------------

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

const QUANT_LO_INIT: [u32; 16] = [
    0x004000, 0x008000, 0x008000, 0x010000, 0x010000, 0x010000, 0x010000, 0x010000, 0x010000,
    0x010000, 0x010000, 0x010000, 0x020000, 0x020000, 0x020000, 0x020000,
];

const QUANT_HI_INIT: [u32; 10] = [
    0, 0x020000, 0x020000, 0x040000, 0x040000, 0x040000, 0x080000, 0x040000, 0x040000, 0x080000,
];

// ---- Coefficient state flags -------------------------------------------------

const ZERO: u8 = 1;
const ACTIVE: u8 = 2;
const NEW: u8 = 4;
const UNK: u8 = 8;

// ---- Forward wavelet transform -----------------------------------------------
//
// IW44 inverse transform (decoder, iw44_new) at each scale s:
//   column pass:
//     1. even rows: data[k] -= ((9*(p1+n1)-(p3+n3)+16)>>5)   [lifting_even]
//     2. odd  rows: data[k] += predict(even neighbors)
//   row pass (same structure, across columns)
//
// Forward analysis transform (this file) at each scale s, run s=1→16:
//   row pass (forward):
//     1. odd  columns: data[k] -= predict(even neighbors)
//     2. even columns: data[k] += lifting(odd neighbors)
//   column pass (forward, same structure)

/// Lifting update: even sample += ((9*(p1+n1)-(p3+n3)+16)>>5)
#[inline(always)]
fn lift(cur: i32, p1: i32, n1: i32, p3: i32, n3: i32) -> i32 {
    let a = p1 + n1;
    let c = p3 + n3;
    cur + (((a << 3) + a - c + 16) >> 5)
}

/// Predict (inner): odd sample -= ((9*(p1+n1)-(p3+n3)+8)>>4)
#[inline(always)]
fn pred_inner_fwd(cur: i32, p1: i32, n1: i32, p3: i32, n3: i32) -> i32 {
    let a = p1 + n1;
    cur - (((a << 3) + a - (p3 + n3) + 8) >> 4)
}

/// Predict (boundary avg): odd sample -= (p+n+1)>>1
#[inline(always)]
fn pred_avg_fwd(cur: i32, p: i32, n: i32) -> i32 {
    cur - ((p + n + 1) >> 1)
}

/// Forward row pass (analysis) at scale `s`.
///
/// Operates on every `s`-th row, within each row on every sample.
fn forward_row_pass(data: &mut [i16], width: usize, height: usize, stride: usize, s: usize) {
    let sd = s.trailing_zeros() as usize;
    let kmax = (width - 1) >> sd;
    let border = kmax.saturating_sub(3);

    for row in (0..height).step_by(s) {
        let off = row * stride;

        // Step 1: undo prediction — odd columns (k=1,3,5,...)
        if kmax >= 1 {
            // k=1
            let p = data[off] as i32;
            let idx1 = off + (1 << sd);
            if kmax >= 2 {
                let n = data[off + (2 << sd)] as i32;
                data[idx1] = pred_avg_fwd(data[idx1] as i32, p, n) as i16;
            } else {
                data[idx1] = (data[idx1] as i32 - p) as i16;
            }

            // k=3..border (inner predict)
            let mut k = 3usize;
            while k <= border {
                let km3 = off + ((k - 3) << sd);
                let km1 = off + ((k - 1) << sd);
                let k0 = off + (k << sd);
                let kp1 = off + ((k + 1) << sd);
                let kp3 = if k + 3 <= kmax {
                    off + ((k + 3) << sd)
                } else {
                    0
                };
                let p1 = data[km1] as i32;
                let n1 = data[kp1] as i32;
                let p3 = data[km3] as i32;
                let n3 = if k + 3 <= kmax { data[kp3] as i32 } else { 0 };
                data[k0] = pred_inner_fwd(data[k0] as i32, p1, n1, p3, n3) as i16;
                k += 2;
            }

            // boundary tail: k continues from where inner loop left off
            while k <= kmax {
                let km1 = off + ((k - 1) << sd);
                let k0 = off + (k << sd);
                let p = data[km1] as i32;
                if k < kmax {
                    let kp1 = off + ((k + 1) << sd);
                    let n = data[kp1] as i32;
                    data[k0] = pred_avg_fwd(data[k0] as i32, p, n) as i16;
                } else {
                    data[k0] = (data[k0] as i32 - p) as i16;
                }
                k += 2;
            }
        }

        // Step 2: undo lifting — even columns (k=0,2,4,...)
        {
            let mut prev3: i32 = 0;
            let mut prev1: i32 = 0;
            let mut next1: i32 = if kmax >= 1 {
                data[off + (1 << sd)] as i32
            } else {
                0
            };
            let mut k = 0usize;
            while k <= kmax {
                let n3 = if k + 3 <= kmax {
                    data[off + ((k + 3) << sd)] as i32
                } else {
                    0
                };
                let idx = off + (k << sd);
                data[idx] = lift(data[idx] as i32, prev1, next1, prev3, n3) as i16;
                prev3 = prev1;
                prev1 = next1;
                next1 = n3;
                k += 2;
            }
        }
    }
}

/// Forward column pass (analysis) at scale `s`.
fn forward_col_pass(data: &mut [i16], width: usize, height: usize, stride: usize, s: usize) {
    let sd = s.trailing_zeros() as usize;
    let kmax = (height - 1) >> sd;
    let border = kmax.saturating_sub(3);
    let col_step = s; // we process columns at stride `s`

    // Step 1: undo prediction — odd rows (k=1,3,5,...)
    if kmax >= 1 {
        // k=1
        let k1_off = (1 << sd) * stride;
        if kmax >= 2 {
            let kp1_off = (2 << sd) * stride;
            for col in (0..width).step_by(col_step) {
                let p = data[col] as i32;
                let n = data[kp1_off + col] as i32;
                data[k1_off + col] = pred_avg_fwd(data[k1_off + col] as i32, p, n) as i16;
            }
        } else {
            for col in (0..width).step_by(col_step) {
                let p = data[col] as i32;
                data[k1_off + col] = (data[k1_off + col] as i32 - p) as i16;
            }
        }

        // k=3..border (inner predict)
        let mut k = 3usize;
        while k <= border {
            let km3_off = ((k - 3) << sd) * stride;
            let km1_off = ((k - 1) << sd) * stride;
            let k0_off = (k << sd) * stride;
            let kp1_off = ((k + 1) << sd) * stride;
            let kp3_off = ((k + 3) << sd) * stride;
            for col in (0..width).step_by(col_step) {
                let p1 = data[km1_off + col] as i32;
                let n1 = data[kp1_off + col] as i32;
                let p3 = data[km3_off + col] as i32;
                let n3 = data[kp3_off + col] as i32;
                data[k0_off + col] =
                    pred_inner_fwd(data[k0_off + col] as i32, p1, n1, p3, n3) as i16;
            }
            k += 2;
        }

        // boundary tail: k continues from where inner loop left off
        while k <= kmax {
            let km1_off = ((k - 1) << sd) * stride;
            let k0_off = (k << sd) * stride;
            if k < kmax {
                let kp1_off = ((k + 1) << sd) * stride;
                for col in (0..width).step_by(col_step) {
                    let p = data[km1_off + col] as i32;
                    let n = data[kp1_off + col] as i32;
                    data[k0_off + col] = pred_avg_fwd(data[k0_off + col] as i32, p, n) as i16;
                }
            } else {
                for col in (0..width).step_by(col_step) {
                    let p = data[km1_off + col] as i32;
                    data[k0_off + col] = (data[k0_off + col] as i32 - p) as i16;
                }
            }
            k += 2;
        }
    }

    // Step 2: undo lifting — even rows (k=0,2,4,...)
    {
        let num_cols = width.div_ceil(col_step);
        let mut prev3: Vec<i32> = vec![0i32; num_cols];
        let mut prev1: Vec<i32> = vec![0i32; num_cols];
        let mut next1: Vec<i32> = if kmax >= 1 {
            let off = (1 << sd) * stride;
            (0..width)
                .step_by(col_step)
                .map(|c| data[off + c] as i32)
                .collect()
        } else {
            vec![0i32; num_cols]
        };

        let mut k = 0usize;
        while k <= kmax {
            let k0_off = (k << sd) * stride;
            let has_n3 = k + 3 <= kmax;
            let n3_off = if has_n3 { ((k + 3) << sd) * stride } else { 0 };

            for (ci, col) in (0..width).step_by(col_step).enumerate() {
                let p3 = prev3[ci];
                let p1 = prev1[ci];
                let n1 = next1[ci];
                let n3 = if has_n3 { data[n3_off + col] as i32 } else { 0 };
                let idx = k0_off + col;
                data[idx] = lift(data[idx] as i32, p1, n1, p3, n3) as i16;
                prev3[ci] = p1;
                prev1[ci] = n1;
                next1[ci] = n3;
            }
            k += 2;
        }
    }
}

/// Apply the full forward wavelet transform in-place on a flat plane.
///
/// `data` is row-major, `stride` samples per row.
/// Passes run from s=1 (finest) to s=16 (coarsest).
fn forward_wavelet_transform(data: &mut [i16], width: usize, height: usize, stride: usize) {
    let mut s = 1usize;
    while s <= 16 {
        forward_row_pass(data, width, height, stride, s);
        forward_col_pass(data, width, height, stride, s);
        s <<= 1;
    }
}

// ---- RGB → YCbCr conversion --------------------------------------------------

/// Convert one RGB pixel to IW44 YCbCr.
///
/// Uses the same integer approximation as DjVuLibre:
/// ```text
/// Y  = (r + 2·g + b) / 4 - 128
/// Cb = b - g
/// Cr = r - g
/// ```
#[inline(always)]
fn rgb_to_ycbcr(r: u8, g: u8, b: u8) -> (i16, i16, i16) {
    let r = r as i32;
    let g = g as i32;
    let b = b as i32;
    let y = (r + (g << 1) + b) / 4 - 128;
    let cb = b - g;
    let cr = r - g;
    (
        y.clamp(-128, 127) as i16,
        cb.clamp(-256, 255) as i16,
        cr.clamp(-256, 255) as i16,
    )
}

// ---- Plane encoder (requires std for ZpEncoder) ------------------------------

// ---- Plane encoder (requires std for ZpEncoder) ------------------------------
//
// The encoder mirrors the decoder pass-by-pass. The key difference is that it
// must track the decoder's running reconstruction (`recon`) independently from
// the true wavelet coefficients (`blocks`), because:
//
//   - `preliminary_flag_computation` in the decoder uses the decoder's own
//     `blocks` array (which is its running reconstruction), NOT the true values.
//   - So the encoder must mirror this by using `recon` for ACTIVE/UNK state.
//
// Reconstruction tracking:
//   - When encoding a newly-active coefficient: set recon to ±(s+s>>1-s>>3).
//   - When encoding a previously-active coefficient: apply the same delta that
//     the decoder will apply, choosing the bit that minimises |true - decoded|.

#[cfg(feature = "std")]
struct PlaneEncoder {
    /// True wavelet coefficients (read-only after `gather`).
    blocks: Vec<[i16; 1024]>,
    /// Decoder's running reconstruction (all-zero initially).
    recon: Vec<[i32; 1024]>,
    block_cols: usize,

    quant_lo: [u32; 16],
    quant_hi: [u32; 10],
    curband: usize,

    ctx_decode_bucket: [u8; 1],
    ctx_decode_coef: [u8; 80],
    ctx_activate_coef: [u8; 16],
    ctx_increase_coef: [u8; 1],

    /// Per-band-offset, per-coefficient state (mirrors decoder's `coeffstate`).
    coeffstate: [[u8; 16]; 64],
    /// Per-band-offset bucket state (mirrors decoder's `bucketstate`).
    bucketstate: [u8; 64],
    /// Combined state for the whole block-band (mirrors decoder's `bbstate`).
    bbstate: u8,
}

#[cfg(feature = "std")]
impl PlaneEncoder {
    fn new(width: usize, height: usize) -> Self {
        let block_cols = width.div_ceil(32);
        let block_rows = height.div_ceil(32);
        let n_blocks = block_cols * block_rows;
        PlaneEncoder {
            blocks: vec![[0i16; 1024]; n_blocks],
            recon: vec![[0i32; 1024]; n_blocks],
            block_cols,
            quant_lo: QUANT_LO_INIT,
            quant_hi: QUANT_HI_INIT,
            curband: 0,
            ctx_decode_bucket: [0; 1],
            ctx_decode_coef: [0; 80],
            ctx_activate_coef: [0; 16],
            ctx_increase_coef: [0; 1],
            coeffstate: [[0; 16]; 64],
            bucketstate: [0; 64],
            bbstate: 0,
        }
    }

    /// Gather wavelet coefficients from a flat plane into zigzag blocks.
    fn gather(&mut self, plane: &[i16], stride: usize) {
        let block_rows = self.blocks.len() / self.block_cols;
        for r in 0..block_rows {
            for c in 0..self.block_cols {
                let block = &mut self.blocks[r * self.block_cols + c];
                let row_base = r << 5;
                let col_base = c << 5;
                for i in 0..1024 {
                    let row = ZIGZAG_ROW[i] as usize + row_base;
                    let col = ZIGZAG_COL[i] as usize + col_base;
                    let safe_idx = (row * stride + col).min(plane.len() - 1);
                    block[i] = plane[safe_idx];
                }
            }
        }
    }

    /// Returns true if this slice produces no bits (all quantization steps exhausted).
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

    /// Mirrors decoder's `preliminary_flag_computation` but uses `recon` (not `blocks`)
    /// to classify coefficients as ACTIVE (recon != 0) or UNK (recon == 0).
    fn preliminary_flag_computation(&mut self, block_idx: usize) {
        self.bbstate = 0;
        let (from, to) = BAND_BUCKETS[self.curband];
        if self.curband != 0 {
            for (boff, j) in (from..=to).enumerate() {
                let mut bstatetmp: u8 = 0;
                for k in 0..16 {
                    // Use recon (decoder reconstruction) not blocks (true value)
                    if self.recon[block_idx][(j << 4) | k] == 0 {
                        self.coeffstate[boff][k] = UNK;
                    } else {
                        self.coeffstate[boff][k] = ACTIVE;
                    }
                    bstatetmp |= self.coeffstate[boff][k];
                }
                self.bucketstate[boff] = bstatetmp;
                self.bbstate |= bstatetmp;
            }
        } else {
            // Band 0: coeffstate[0] is pre-initialized by is_null_slice
            let mut bstatetmp: u8 = 0;
            for k in 0..16 {
                if self.coeffstate[0][k] != ZERO {
                    if self.recon[block_idx][k] == 0 {
                        self.coeffstate[0][k] = UNK;
                    } else {
                        self.coeffstate[0][k] = ACTIVE;
                    }
                }
                bstatetmp |= self.coeffstate[0][k];
            }
            self.bucketstate[0] = bstatetmp;
            self.bbstate |= bstatetmp;
        }
    }

    fn encode_slice(&mut self, zp: &mut ZpEncoder) {
        if !self.is_null_slice() {
            for block_idx in 0..self.blocks.len() {
                self.preliminary_flag_computation(block_idx);
                let emit = self.block_band_encoding_pass(zp, block_idx);
                if emit {
                    self.bucket_encoding_pass(zp, block_idx);
                    self.newly_active_encoding_pass(zp, block_idx);
                }
                self.previously_active_encoding_pass(zp, block_idx);
            }
        }
        self.finish_slice();
    }

    /// Mirrors decoder's `block_band_decoding_pass`.
    ///
    /// Encodes one bit (when needed) to tell the decoder whether any bucket
    /// in this band has a newly-active coefficient.
    fn block_band_encoding_pass(&mut self, zp: &mut ZpEncoder, block_idx: usize) -> bool {
        let (from, to) = BAND_BUCKETS[self.curband];
        let bcount = to - from + 1;

        let should_encode_bit =
            bcount >= 16 && (self.bbstate & ACTIVE) == 0 && (self.bbstate & UNK) != 0;

        if should_encode_bit {
            // Determine if any UNK coefficient in this block-band will become active.
            let any_will_activate = self.any_unk_activates(block_idx, from, to);
            zp.encode_bit(&mut self.ctx_decode_bucket[0], any_will_activate);
            if any_will_activate {
                self.bbstate |= NEW;
            }
        } else if bcount < 16 || (self.bbstate & ACTIVE) != 0 {
            self.bbstate |= NEW;
        }
        (self.bbstate & NEW) != 0
    }

    /// Returns true if any UNK coefficient in `[from..=to]` buckets will activate
    /// at the current quantization step.
    fn any_unk_activates(&self, block_idx: usize, from: usize, to: usize) -> bool {
        let step_hi = self.quant_hi[self.curband] as i32;
        for (boff, j) in (from..=to).enumerate() {
            for k in 0..16 {
                if self.coeffstate[boff][k] != UNK {
                    continue;
                }
                let coef_idx = if self.curband == 0 { k } else { (j << 4) | k };
                let s = if self.curband == 0 {
                    self.quant_lo[k] as i32
                } else {
                    step_hi
                };
                let v = self.blocks[block_idx][coef_idx].unsigned_abs() as i32;
                if v > s / 2 {
                    return true;
                }
            }
        }
        false
    }

    /// Mirrors decoder's `bucket_decoding_pass` — encodes per-bucket NEW bits.
    fn bucket_encoding_pass(&mut self, zp: &mut ZpEncoder, block_idx: usize) {
        let (from, to) = BAND_BUCKETS[self.curband];
        let step_hi = self.quant_hi[self.curband] as i32;
        for (boff, i) in (from..=to).enumerate() {
            if (self.bucketstate[boff] & UNK) == 0 {
                continue;
            }
            // Context index: count of active coefficients among the first 4 of the bucket.
            let mut n: usize = 0;
            if self.curband != 0 {
                let t = 4 * i;
                for j in t..t + 4 {
                    if self.recon[block_idx][j] != 0 {
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
            // Will any UNK coefficient in this bucket become active?
            let is_new = (0..16usize).any(|k| {
                if self.coeffstate[boff][k] != UNK {
                    return false;
                }
                let coef_idx = if self.curband == 0 { k } else { (i << 4) | k };
                let s = if self.curband == 0 {
                    self.quant_lo[k] as i32
                } else {
                    step_hi
                };
                let v = self.blocks[block_idx][coef_idx].unsigned_abs() as i32;
                v > s / 2
            });
            if is_new {
                self.bucketstate[boff] |= NEW;
            }
            zp.encode_bit(&mut self.ctx_decode_coef[n + self.curband * 8], is_new);
        }
    }

    /// Mirrors decoder's `newly_active_coefficient_decoding_pass`.
    ///
    /// For each UNK coefficient in a NEW bucket: encodes whether it becomes active,
    /// and if so, its sign. Updates `recon` to match the decoder's new value.
    fn newly_active_encoding_pass(&mut self, zp: &mut ZpEncoder, block_idx: usize) {
        let (from, to) = BAND_BUCKETS[self.curband];
        let mut step = self.quant_hi[self.curband];
        for (boff, i) in (from..=to).enumerate() {
            if (self.bucketstate[boff] & NEW) == 0 {
                continue;
            }
            let shift: usize = if (self.bucketstate[boff] & ACTIVE) != 0 {
                8
            } else {
                0
            };
            let mut np: usize = 0;
            for k in 0..16 {
                if self.coeffstate[boff][k] == UNK {
                    np += 1;
                }
            }
            for k in 0..16 {
                if self.coeffstate[boff][k] == UNK {
                    let ip = np.min(7);
                    if self.curband == 0 {
                        step = self.quant_lo[k];
                    }
                    let coef_idx = if self.curband == 0 { k } else { (i << 4) | k };
                    let true_val = self.blocks[block_idx][coef_idx] as i32;
                    let s = step as i32;
                    // Activate if true value exceeds half the decoded activation value.
                    // Decoded value = sign*(s + s/2 - s/8) = sign*11s/8.
                    // Threshold for activation: |V| > 11s/16.
                    let is_active = true_val.unsigned_abs() as i32 > (s * 11 / 16).max(1);
                    zp.encode_bit(&mut self.ctx_activate_coef[shift + ip], is_active);
                    if is_active {
                        let negative = true_val < 0;
                        zp.encode_passthrough_iw44(negative);
                        // Mirror decoder: recon = sign * (s + s>>1 - s>>3)
                        let decoded_val = s + (s >> 1) - (s >> 3);
                        self.recon[block_idx][coef_idx] =
                            if negative { -decoded_val } else { decoded_val };
                        np = 0;
                    }
                    np = np.saturating_sub(1);
                }
            }
        }
    }

    /// Mirrors decoder's `previously_active_coefficient_decoding_pass`.
    ///
    /// For each ACTIVE coefficient: encodes the refinement bit and updates `recon`.
    fn previously_active_encoding_pass(&mut self, zp: &mut ZpEncoder, block_idx: usize) {
        let (from, to) = BAND_BUCKETS[self.curband];
        let mut step = self.quant_hi[self.curband];
        for (boff, i) in (from..=to).enumerate() {
            for k in 0..16 {
                if (self.coeffstate[boff][k] & ACTIVE) == 0 {
                    continue;
                }
                if self.curband == 0 {
                    step = self.quant_lo[k];
                }
                let coef_idx = if self.curband == 0 { k } else { (i << 4) | k };
                let s = step as i32;
                let true_v = self.blocks[block_idx][coef_idx] as i32;
                let d = self.recon[block_idx][coef_idx]; // decoder's current value
                let abs_d = d.unsigned_abs() as i32;
                let abs_v = true_v.unsigned_abs() as i32;

                // Decoder logic (from iw44_new.rs):
                //   if abs_d <= 3*s:
                //     d += s>>2;          // base adjustment
                //     bit -> if 1: d += s>>1; if 0: d += -s + s>>1 = -s>>1 (net: -s>>2)
                //   else (passthrough):
                //     bit -> if 1: d += s>>1; if 0: d += -s>>1
                //
                // Midpoint for abs_d <= 3*s: abs_d + s>>2 (after base) + (3/4*s/2) midpoint
                //   Between (abs_d + s/4 + s/2) and (abs_d + s/4 - s/2):
                //   midpoint = abs_d + s/4
                //   → encode 1 if abs_v > abs_d + s/4
                //
                // Midpoint for abs_d > 3*s: abs_d
                //   Between (abs_d + s/2) and (abs_d - s/2):
                //   midpoint = abs_d
                //   → encode 1 if abs_v > abs_d

                let des: bool;
                let mut new_abs_d = abs_d;
                if abs_d <= 3 * s {
                    des = abs_v > abs_d + (s >> 2);
                    new_abs_d += s >> 2;
                    zp.encode_bit(&mut self.ctx_increase_coef[0], des);
                } else {
                    des = abs_v > abs_d;
                    zp.encode_passthrough_iw44(des);
                }
                if des {
                    new_abs_d += s >> 1;
                } else {
                    new_abs_d += -s + (s >> 1);
                }
                // Update recon with the decoder's new value
                let sign = if d < 0 { -1i32 } else { 1i32 };
                self.recon[block_idx][coef_idx] = sign * new_abs_d.max(0);
            }
        }
    }

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
}

// ---- Public encoder API (requires std for ZpEncoder) -------------------------

#[cfg(feature = "std")]
/// Options for IW44 encoding.
#[derive(Clone, Debug)]
pub struct Iw44EncodeOptions {
    /// Number of slices per BG44 chunk (1..=99, default 10).
    pub slices_per_chunk: u8,
    /// Total number of slices to encode (default 100).
    pub total_slices: u8,
    /// Chroma delay — Y slices before Cb/Cr encoding begins (default 0).
    pub chroma_delay: u8,
    /// Encode chroma at half resolution (default true).
    pub chroma_half: bool,
}

#[cfg(feature = "std")]
impl Default for Iw44EncodeOptions {
    fn default() -> Self {
        Iw44EncodeOptions {
            slices_per_chunk: 10,
            total_slices: 100,
            chroma_delay: 0,
            chroma_half: true,
        }
    }
}

#[cfg(feature = "std")]
/// Encode a color [`Pixmap`] into BG44 chunk payloads (one `Vec<u8>` per chunk).
///
/// Wrap each chunk in a `BG44` IFF chunk tag before embedding in a DjVu file.
pub fn encode_iw44_color(pixmap: &Pixmap, opts: &Iw44EncodeOptions) -> Vec<Vec<u8>> {
    let w = pixmap.width as usize;
    let h = pixmap.height as usize;
    let stride = w.div_ceil(32) * 32;
    let plane_h = h.div_ceil(32) * 32;

    let mut y_plane = vec![0i16; stride * plane_h];

    let (cw, ch) = if opts.chroma_half {
        (w.div_ceil(2), h.div_ceil(2))
    } else {
        (w, h)
    };
    let c_stride = cw.div_ceil(32) * 32;
    let c_plane_h = ch.div_ceil(32) * 32;
    let mut cb_plane = vec![0i16; c_stride * c_plane_h];
    let mut cr_plane = vec![0i16; c_stride * c_plane_h];

    // DjVu stores images bottom-to-top: wavelet row 0 = image bottom row.
    // The decoder's to_rgb flips via out_row = h-1-row, so mirror that here.
    // Scale by 64 because normalize() divides by 64 on decode.
    for row in 0..h {
        let wavelet_row = h - 1 - row;
        for col in 0..w {
            let (r, g, b) = pixmap.get_rgb(col as u32, row as u32);
            let (y, _cb, _cr) = rgb_to_ycbcr(r, g, b);
            y_plane[wavelet_row * stride + col] = (y as i32 * 64) as i16;
        }
    }
    if opts.chroma_half {
        for row in (0..h).step_by(2) {
            let wavelet_row = (h - 1 - row) / 2;
            for col in (0..w).step_by(2) {
                let (r, g, b) = pixmap.get_rgb(col as u32, row as u32);
                let (_y, cb, cr) = rgb_to_ycbcr(r, g, b);
                let cc2 = col / 2;
                cb_plane[wavelet_row * c_stride + cc2] = (cb as i32 * 64) as i16;
                cr_plane[wavelet_row * c_stride + cc2] = (cr as i32 * 64) as i16;
            }
        }
    } else {
        for row in 0..h {
            let wavelet_row = h - 1 - row;
            for col in 0..w {
                let (r, g, b) = pixmap.get_rgb(col as u32, row as u32);
                let (_y, cb, cr) = rgb_to_ycbcr(r, g, b);
                cb_plane[wavelet_row * c_stride + col] = (cb as i32 * 64) as i16;
                cr_plane[wavelet_row * c_stride + col] = (cr as i32 * 64) as i16;
            }
        }
    }

    forward_wavelet_transform(&mut y_plane, w, h, stride);
    forward_wavelet_transform(&mut cb_plane, cw, ch, c_stride);
    forward_wavelet_transform(&mut cr_plane, cw, ch, c_stride);

    let mut y_enc = PlaneEncoder::new(w, h);
    let mut cb_enc = PlaneEncoder::new(cw, ch);
    let mut cr_enc = PlaneEncoder::new(cw, ch);
    y_enc.gather(&y_plane, stride);
    cb_enc.gather(&cb_plane, c_stride);
    cr_enc.gather(&cr_plane, c_stride);

    encode_chunks(
        &mut y_enc,
        Some(&mut cb_enc),
        Some(&mut cr_enc),
        w as u16,
        h as u16,
        true,
        opts,
    )
}

#[cfg(feature = "std")]
/// Encode a grayscale [`GrayPixmap`] into BG44/FG44 chunk payloads.
pub fn encode_iw44_gray(pixmap: &GrayPixmap, opts: &Iw44EncodeOptions) -> Vec<Vec<u8>> {
    let w = pixmap.width as usize;
    let h = pixmap.height as usize;
    let stride = w.div_ceil(32) * 32;
    let plane_h = h.div_ceil(32) * 32;
    let mut y_plane = vec![0i16; stride * plane_h];

    // DjVu stores images bottom-to-top: wavelet row 0 = image bottom row.
    // The decoder's to_rgb flips via out_row = h-1-row, so we must mirror that.
    // The grayscale formula: coeff = (127 - p) * 64 (decoder gives gray = 127 - normalize(coeff)).
    for row in 0..h {
        let wavelet_row = h - 1 - row;
        for col in 0..w {
            let p = pixmap.get(col as u32, row as u32) as i32;
            y_plane[wavelet_row * stride + col] = ((127 - p) * 64) as i16;
        }
    }

    forward_wavelet_transform(&mut y_plane, w, h, stride);

    let mut y_enc = PlaneEncoder::new(w, h);
    y_enc.gather(&y_plane, stride);

    encode_chunks(&mut y_enc, None, None, w as u16, h as u16, false, opts)
}

#[cfg(feature = "std")]
fn encode_chunks(
    y_enc: &mut PlaneEncoder,
    mut cb_enc: Option<&mut PlaneEncoder>,
    mut cr_enc: Option<&mut PlaneEncoder>,
    width: u16,
    height: u16,
    is_color: bool,
    opts: &Iw44EncodeOptions,
) -> Vec<Vec<u8>> {
    let slices_per_chunk = opts.slices_per_chunk.max(1) as usize;
    let total = opts.total_slices as usize;
    let delay = opts.chroma_delay as usize;

    let mut chunks: Vec<Vec<u8>> = Vec::new();
    let mut slice_idx = 0usize;
    let mut serial: u8 = 0;
    let mut cslice = 0usize;

    while slice_idx < total {
        let n = slices_per_chunk.min(total - slice_idx);
        let mut zp = ZpEncoder::new();

        for _ in 0..n {
            cslice += 1;
            y_enc.encode_slice(&mut zp);
            if is_color && cslice > delay {
                if let Some(cb) = cb_enc.as_deref_mut() {
                    cb.encode_slice(&mut zp);
                }
                if let Some(cr) = cr_enc.as_deref_mut() {
                    cr.encode_slice(&mut zp);
                }
            }
            slice_idx += 1;
            if slice_idx >= total {
                break;
            }
        }

        let mut zp_bytes = zp.finish();
        // Pad with 0xFF bytes to prevent the decoder's is_exhausted() guard from
        // firing before all `n` slices in this chunk are processed.  The ZP
        // decoder reads 2 bytes during construction plus 4 more in refill_buffer
        // (6 total), so `pos = 2` (= min(6, data.len())) after init.  If
        // data.len() == 2, pos == data.len() → is_exhausted() is immediately
        // true, the loop breaks after the first slice, and curband gets out of
        // sync.  Appending 0xFF bytes is safe: read_byte() already returns 0xFF
        // beyond the real data, so the decoded bit-stream is unchanged.
        let min_zp_len = n + 4; // enough for init + one refill per slice
        while zp_bytes.len() < min_zp_len {
            zp_bytes.push(0xFF);
        }
        let mut chunk = Vec::new();

        if serial == 0 {
            chunk.push(0u8); // serial
            chunk.push(n as u8); // slices
            let majver: u8 = if !is_color { 0x80 } else { 0x00 };
            chunk.push(majver);
            chunk.push(0x02); // minor = 2
            chunk.push((width >> 8) as u8);
            chunk.push(width as u8);
            chunk.push((height >> 8) as u8);
            chunk.push(height as u8);
            // delay_byte: bits 0-6 = chroma_delay, bit 7 = !chroma_half
            let delay_byte = (opts.chroma_delay & 0x7F)
                | if is_color && !opts.chroma_half {
                    0x80
                } else {
                    0x00
                };
            chunk.push(delay_byte);
        } else {
            chunk.push(serial);
            chunk.push(n as u8);
        }
        chunk.extend_from_slice(&zp_bytes);
        chunks.push(chunk);
        serial = serial.wrapping_add(1);
    }
    chunks
}

// ---- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iw44_new::Iw44Image;
    use crate::pixmap::{GrayPixmap, Pixmap};

    fn make_pixmap(w: u32, h: u32, f: impl Fn(u32, u32) -> (u8, u8, u8)) -> Pixmap {
        let mut px = Pixmap::white(w, h);
        for y in 0..h {
            for x in 0..w {
                let (r, g, b) = f(x, y);
                px.set_rgb(x, y, r, g, b);
            }
        }
        px
    }

    fn make_gray(w: u32, h: u32, f: impl Fn(u32, u32) -> u8) -> GrayPixmap {
        let mut data = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            for x in 0..w {
                data.push(f(x, y));
            }
        }
        GrayPixmap {
            width: w,
            height: h,
            data,
        }
    }

    fn decode_color(chunks: &[Vec<u8>]) -> Pixmap {
        let mut img = Iw44Image::new();
        for c in chunks {
            img.decode_chunk(c).unwrap();
        }
        img.to_rgb().unwrap()
    }

    fn decode_gray(chunks: &[Vec<u8>]) -> GrayPixmap {
        let mut img = Iw44Image::new();
        for c in chunks {
            img.decode_chunk(c).unwrap();
        }
        img.to_rgb().unwrap().to_gray8()
    }

    #[test]
    fn encode_color_produces_decodable_chunks() {
        let src = make_pixmap(64, 64, |x, y| {
            ((x * 4) as u8, (y * 4) as u8, ((x + y) * 2) as u8)
        });
        let opts = Iw44EncodeOptions {
            slices_per_chunk: 10,
            total_slices: 10,
            ..Default::default()
        };
        let chunks = encode_iw44_color(&src, &opts);
        assert!(!chunks.is_empty());
        let decoded = decode_color(&chunks);
        assert_eq!(decoded.width, 64);
        assert_eq!(decoded.height, 64);
    }

    #[test]
    fn encode_gray_produces_decodable_chunks() {
        let src = make_gray(32, 32, |x, y| ((x + y) * 4) as u8);
        let opts = Iw44EncodeOptions {
            slices_per_chunk: 10,
            total_slices: 10,
            ..Default::default()
        };
        let chunks = encode_iw44_gray(&src, &opts);
        assert!(!chunks.is_empty());
        let decoded = decode_gray(&chunks);
        assert_eq!(decoded.width, 32);
        assert_eq!(decoded.height, 32);
    }

    #[test]
    fn chunk_header_serial_0() {
        let src = make_pixmap(16, 16, |_, _| (200, 100, 50));
        let opts = Iw44EncodeOptions {
            slices_per_chunk: 5,
            total_slices: 5,
            ..Default::default()
        };
        let chunks = encode_iw44_color(&src, &opts);
        let first = &chunks[0];
        assert_eq!(first[0], 0, "serial must be 0");
        assert_eq!(first[1], 5, "slices count");
        assert_eq!(first[2] & 0x80, 0, "color image: majver bit 7 = 0");
        assert_eq!(first[3], 2, "minor = 2");
        assert_eq!(u16::from_be_bytes([first[4], first[5]]), 16u16);
        assert_eq!(u16::from_be_bytes([first[6], first[7]]), 16u16);
    }

    #[test]
    fn multi_chunk_serials_increment() {
        let src = make_pixmap(32, 32, |x, y| ((x * 8) as u8, (y * 8) as u8, 0));
        let opts = Iw44EncodeOptions {
            slices_per_chunk: 10,
            total_slices: 30,
            ..Default::default()
        };
        let chunks = encode_iw44_color(&src, &opts);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0][0], 0);
        assert_eq!(chunks[1][0], 1);
        assert_eq!(chunks[2][0], 2);
    }

    #[test]
    fn gray_flat_roundtrip() {
        // Flat image: all pixels = 100. After encode/decode should be ~100.
        let src = make_gray(32, 32, |_x, _y| 100u8);
        let opts = Iw44EncodeOptions {
            slices_per_chunk: 10,
            total_slices: 100,
            ..Default::default()
        };
        let chunks = encode_iw44_gray(&src, &opts);
        let decoded = decode_gray(&chunks);
        let mut total = 0u64;
        for y in 0..32 {
            for x in 0..32 {
                total += (src.get(x, y) as i32 - decoded.get(x, y) as i32).unsigned_abs() as u64;
            }
        }
        let avg = total as f64 / (32.0 * 32.0);
        // Diagnose: print a few decoded values
        for y in 0..4 {
            for x in 0..4 {
                print!("({},{})={} ", x, y, decoded.get(x, y));
            }
            println!();
        }
        assert!(avg < 10.0, "flat avg error = {avg:.2} (expected < 10)");
    }

    #[test]
    fn gray_low_error_many_slices() {
        // Test grayscale roundtrip quality — avoids the YCbCr color-space mismatch.
        // With 100 slices the average absolute error per pixel should be well below 20.
        let src = make_gray(64, 64, |x, y| ((x * 2 + y * 2).min(255)) as u8);
        let opts = Iw44EncodeOptions {
            slices_per_chunk: 10,
            total_slices: 100,
            ..Default::default()
        };
        let chunks = encode_iw44_gray(&src, &opts);
        let decoded = decode_gray(&chunks);
        assert_eq!((decoded.width, decoded.height), (64, 64));
        let mut total = 0u64;
        for y in 0..src.height {
            for x in 0..src.width {
                total += (src.get(x, y) as i32 - decoded.get(x, y) as i32).unsigned_abs() as u64;
            }
        }
        let avg = total as f64 / (64.0 * 64.0);
        assert!(avg < 30.0, "avg gray abs error = {avg:.2} (expected < 30)");
    }
}
