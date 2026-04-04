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

// ---- Per-channel wavelet decoder --------------------------------------------

/// State for a single YCbCr plane wavelet decoder.
///
/// Holds 32×32 block coefficients and the ZP context tables that persist
/// across progressive slices.
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
                if self.block_band_decoding_pass(zp) {
                    self.bucket_decoding_pass(zp, block_idx);
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
                let mut bstatetmp: u8 = 0;
                for k in 0..16 {
                    if self.blocks[block_idx][(j << 4) | k] == 0 {
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
            let mut bstatetmp: u8 = 0;
            for k in 0..16 {
                if self.coeffstate[0][k] != ZERO {
                    if self.blocks[block_idx][k] == 0 {
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

    fn bucket_decoding_pass(&mut self, zp: &mut ZpDecoder<'_>, block_idx: usize) {
        let (from, to) = BAND_BUCKETS[self.curband];
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
            }
        }
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

    fn previously_active_coefficient_decoding_pass(
        &mut self,
        zp: &mut ZpDecoder<'_>,
        block_idx: usize,
    ) {
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
                        let d = zp.decode_bit(&mut self.ctx_increase_coef[0]);
                        abs_coef += s >> 2;
                        d
                    } else {
                        zp.decode_passthrough_iw44()
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

fn inverse_wavelet_transform(plane: &mut FlatPlane, width: usize, height: usize, subsample: usize) {
    let stride = plane.stride;
    let data = plane.data.as_mut_slice();
    let mut s_degree: u32 = 4;
    let mut s = 16usize;

    let mut st0 = vec![0i32; width];
    let mut st1 = vec![0i32; width];
    let mut st2 = vec![0i32; width];

    while s >= subsample {
        let sd = s_degree as usize;

        // ── Column pass (transposed) ──────────────────────────────────────────
        {
            let kmax = (height - 1) >> sd;
            let border = kmax.saturating_sub(3);
            let num_cols = width.div_ceil(s);

            // Lifting (even samples)
            for v in &mut st0[..num_cols] {
                *v = 0;
            }
            for v in &mut st1[..num_cols] {
                *v = 0;
            }
            if kmax >= 1 {
                let off = (1 << sd) * stride;
                for (ci, col) in (0..width).step_by(s).enumerate() {
                    st2[ci] = data[off + col] as i32;
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
                k += 2;
            }

            // Prediction (odd samples)
            if kmax >= 1 {
                // k = 1
                let km1_off = 0;
                let k_off = (1 << sd) * stride;

                if 2 <= kmax {
                    let kp1_off = (2 << sd) * stride;
                    for (ci, col) in (0..width).step_by(s).enumerate() {
                        let p = data[km1_off + col] as i32;
                        let n = data[kp1_off + col] as i32;
                        let idx = k_off + col;
                        data[idx] = (data[idx] as i32 + ((p + n + 1) >> 1)) as i16;
                        st0[ci] = p;
                        st1[ci] = n;
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
                    for (ci, col) in (0..width).step_by(s).enumerate() {
                        st2[ci] = data[off + col] as i32;
                    }
                }

                // k = 3, 5, ..., border
                let mut k = 3usize;
                while k <= border {
                    let k_off = (k << sd) * stride;
                    let n3_off = ((k + 3) << sd) * stride;

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
                    k += 2;
                }

                // tail
                while k <= kmax {
                    let k_off = (k << sd) * stride;

                    if k < kmax {
                        for (ci, col) in (0..width).step_by(s).enumerate() {
                            let p = st1[ci];
                            let n = st2[ci];
                            let idx = k_off + col;
                            data[idx] = (data[idx] as i32 + ((p + n + 1) >> 1)) as i16;
                            st1[ci] = n;
                            st2[ci] = 0;
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
        {
            let kmax = (width - 1) >> sd;
            let border = kmax.saturating_sub(3);

            for row in (0..height).step_by(s) {
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
                        data[idx] =
                            (data[idx] as i32 + (((a << 3) + a - (prev3 + next3) + 8) >> 4)) as i16;
                        k += 2;
                    }

                    while k <= kmax {
                        prev1 = next1;
                        next1 = next3;
                        next3 = 0;
                        if k < kmax {
                            let idx = off + (k << sd);
                            data[idx] = (data[idx] as i32 + ((prev1 + next1 + 1) >> 1)) as i16;
                        } else {
                            let idx = off + (k << sd);
                            data[idx] = (data[idx] as i32 + prev1) as i16;
                        }
                        k += 2;
                    }
                }
            }
        }

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
            // Prevent OOM on malformed input (~256 M pixels).
            let pixels = w as u64 * h as u64;
            if pixels > 256 * 1024 * 1024 {
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
                self.cb = Some(PlaneDecoder::new(w as usize, h as usize));
                self.cr = Some(PlaneDecoder::new(w as usize, h as usize));
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

        let y_plane = y_dec.reconstruct(sub);

        if self.is_color {
            let chroma_sub = if self.chroma_half { sub.max(2) } else { sub };
            let cb_plane = self
                .cb
                .as_ref()
                .ok_or(Iw44Error::MissingCodec)?
                .reconstruct(chroma_sub);
            let cr_plane = self
                .cr
                .as_ref()
                .ok_or(Iw44Error::MissingCodec)?
                .reconstruct(chroma_sub);

            let mut pm = Pixmap::new(w, h, 0, 0, 0, 255);
            for row in 0..h {
                // DjVu stores rows bottom-to-top; flip on output.
                let out_row = h - 1 - row;
                for col in 0..w {
                    let src_row = row as usize * sub;
                    let src_col = col as usize * sub;
                    let y_idx = src_row * y_plane.stride + src_col;
                    let chroma_row = if self.chroma_half {
                        src_row & !1
                    } else {
                        src_row
                    };
                    let chroma_col = if self.chroma_half {
                        src_col & !1
                    } else {
                        src_col
                    };
                    let c_idx = chroma_row * cb_plane.stride + chroma_col;

                    let y = normalize(y_plane.data[y_idx]);
                    let b = normalize(cb_plane.data[c_idx]);
                    let r = normalize(cr_plane.data[c_idx]);

                    // DjVu YCbCr → RGB (LeCun 1998 formula)
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
            let mut pm = Pixmap::new(w, h, 0, 0, 0, 255);
            for row in 0..h {
                let out_row = h - 1 - row;
                for col in 0..w {
                    let src_row = row as usize * sub;
                    let src_col = col as usize * sub;
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
}
