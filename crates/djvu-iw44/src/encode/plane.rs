//! The per-plane progressive coefficient encoder, `PlaneEncoder`.

use super::*;

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
pub(super) struct PlaneEncoder {
    /// True wavelet coefficients (read-only after `gather`).
    pub(super) blocks: Vec<[i16; 1024]>,
    /// Decoder's running reconstruction (all-zero initially).
    ///
    /// `i16`, like the decoder's own coefficients. The encoder has to mirror
    /// the decoder bit for bit, and the decoder truncates every store to `i16`;
    /// holding these in `i32` kept a precision the decoder never has.
    ///
    /// Sparse, for the same reason the decoder's grid is: only 0.6-12.7 % of
    /// its buckets are ever written (PERF_EXPERIMENTS.md ENCODE_SPARSE_RECON).
    pub(super) recon: Vec<CoefBlock>,
    pub(super) block_cols: usize,
    /// Per block, the largest `|v|` in each of bands 1..=9; empty until the
    /// first slice, when `blocks` is final. See [`Self::skip_quiet_block`].
    band_max: Vec<[u16; 9]>,
    /// Test switch: always run the full passes, for comparing bytes.
    #[cfg(test)]
    pub(super) full_passes: bool,

    pub(super) quant_lo: [u32; 16],
    pub(super) quant_hi: [u32; 10],
    pub(super) curband: usize,

    pub(super) ctx_decode_bucket: [u8; 1],
    pub(super) ctx_decode_coef: [u8; 80],
    pub(super) ctx_activate_coef: [u8; 16],
    pub(super) ctx_increase_coef: [u8; 1],

    /// Per-band-offset, per-coefficient state (mirrors decoder's `coeffstate`).
    pub(super) coeffstate: [[u8; 16]; 64],
    /// Per-band-offset bucket state (mirrors decoder's `bucketstate`).
    pub(super) bucketstate: [u8; 64],
    /// Combined state for the whole block-band (mirrors decoder's `bbstate`).
    pub(super) bbstate: u8,
}

/// Scatter one 32-row strip of a row-major plane into its blocks, in zigzag
/// order.
///
/// Reads the strip in row-major (sequential) order and scatters into the small
/// 2 KB, L1-resident block via `ZIGZAG_INV`, rather than reading the plane in
/// scattered zigzag order (ZIGZAG_ROW/COL) with a sequential block write. This
/// mirrors the decoder's `reconstruct()` scatter (see lib.rs) for the same
/// reason: keep the scattered access on the tiny L1-resident block, and stream
/// the multi-KB plane one cache line at a time.
///
/// Safety invariant: `stride` = `blocks.len() * 32` and `strip` holds at least
/// 32 rows of it, which the asserts below check once. For any block `c`,
/// row, col < 32:
///   src = row * stride + (c*32 + col) ≤ strip.len() - 1
///   i   = ZIGZAG_INV[row*32 + col] ∈ [0, 1024) = block.len()
/// Both indices are therefore always in bounds; `get_unchecked` drops the
/// dead branches from the inner loop.
#[cfg(feature = "std")]
#[allow(unsafe_code)]
fn scatter_strip(strip: &[i16], stride: usize, blocks: &mut [[i16; 1024]]) {
    assert_eq!(stride, blocks.len() * 32);
    assert!(strip.len() >= 32 * stride);
    for (c, block) in blocks.iter_mut().enumerate() {
        let col_base = c << 5;
        for row in 0..32usize {
            let src_base = row * stride + col_base;
            let inv_base = row << 5;
            for col in 0..32usize {
                // SAFETY: see invariant above.
                let i = unsafe { *crate::ZIGZAG_INV.get_unchecked(inv_base + col) } as usize;
                *unsafe { block.get_unchecked_mut(i) } =
                    unsafe { *strip.get_unchecked(src_base + col) };
            }
        }
    }
}

/// Whether any UNK coefficient of a bucket reaches its step: `|v| >= s`,
/// the encoder's significance test.
///
/// Branch-free over the 16 lanes so it vectorises; the per-coefficient
/// early exit it replaces was 40% of a Photo encode. Steps are clamped to
/// `u16`: a step of `0x8000` or more never belongs to an UNK coefficient
/// (`is_null_slice` marks it ZERO or skips the slice), so the clamp cannot
/// change the answer.
#[cfg(feature = "std")]
#[inline]
fn any_unk_reaches(states: &[u8; 16], coefs: &[i16], steps: &[u16; 16]) -> bool {
    let coefs: &[i16; 16] = coefs.try_into().expect("a bucket holds 16 coefficients");
    let mut any = false;
    for k in 0..16 {
        any |= (states[k] == UNK) & (coefs[k].unsigned_abs() >= steps[k]);
    }
    any
}

/// The largest `|v|` of a block's coefficients in each of bands 1..=9.
#[cfg(feature = "std")]
fn band_maxima(block: &[i16; 1024]) -> [u16; 9] {
    core::array::from_fn(|i| {
        let (from, to) = BAND_BUCKETS[i + 1];
        block[from << 4..(to + 1) << 4]
            .iter()
            .fold(0, |max, v| max.max(v.unsigned_abs()))
    })
}

#[cfg(feature = "std")]
impl PlaneEncoder {
    pub(super) fn new(width: usize, height: usize) -> Self {
        Self::with_blocks(Self::new_plane(width, height), width.div_ceil(32))
    }

    fn with_blocks(blocks: Vec<[i16; 1024]>, block_cols: usize) -> Self {
        let n_blocks = blocks.len();
        PlaneEncoder {
            blocks,
            recon: vec![CoefBlock::default(); n_blocks],
            block_cols,
            band_max: Vec::new(),
            #[cfg(test)]
            full_passes: false,
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

    /// A zeroed plane for `width`×`height`, laid out as the encoder's own
    /// block storage.
    ///
    /// Flattened, it is the row-major plane the transform works on (`stride`
    /// = `block_cols * 32`). Block row `r` of the grid and plane rows
    /// `32r..32r+32` cover the same memory, which is what lets
    /// [`Self::from_plane`] gather in place.
    pub(super) fn new_plane(width: usize, height: usize) -> Vec<[i16; 1024]> {
        vec![[0i16; 1024]; width.div_ceil(32) * height.div_ceil(32)]
    }

    /// Build the encoder from a plane made by [`Self::new_plane`] that already
    /// holds the transformed coefficients in row-major order.
    ///
    /// The plane's memory becomes the coefficient grid: each 32-row strip is
    /// copied out to a one-strip buffer and scattered back into its own blocks.
    /// The page never holds a second plane-sized grid beside the plane
    /// (PERF_EXPERIMENTS.md IW44_ENCODE_INPLACE_GATHER).
    pub(super) fn from_plane(width: usize, height: usize, mut plane: Vec<[i16; 1024]>) -> Self {
        let block_cols = width.div_ceil(32);
        let block_rows = height.div_ceil(32);
        assert_eq!(plane.len(), block_cols * block_rows);
        let stride = block_cols * 32;
        let mut strip = vec![0i16; 32 * stride];
        for blocks in plane.chunks_exact_mut(block_cols) {
            strip.copy_from_slice(blocks.as_flattened());
            scatter_strip(&strip, stride, blocks);
        }
        Self::with_blocks(plane, block_cols)
    }

    /// Gather block rows `first_block..last_block` from `plane`, whose row 0
    /// is the plane's absolute row `buf_first_block * 32`.
    ///
    /// This is what lets [`forward_gather_banded`] feed the grid one band at a
    /// time: the band's buffer starts at its halo, not at the page's top.
    pub(super) fn gather_rows(
        &mut self,
        plane: &[i16],
        stride: usize,
        buf_first_block: usize,
        first_block: usize,
        last_block: usize,
    ) {
        let block_rows = self.blocks.len() / self.block_cols;
        assert!(buf_first_block <= first_block && first_block <= last_block);
        assert!(last_block <= block_rows);
        assert_eq!(stride, self.block_cols * 32);
        assert!(plane.len() >= (last_block - buf_first_block) * 32 * stride);
        let strip_len = 32 * stride;
        for r in first_block..last_block {
            let from = (r - buf_first_block) * strip_len;
            let blocks = &mut self.blocks[r * self.block_cols..(r + 1) * self.block_cols];
            scatter_strip(&plane[from..from + strip_len], stride, blocks);
        }
    }

    /// Returns true if this slice produces no bits (all quantization steps exhausted).
    pub(super) fn is_null_slice(&mut self) -> bool {
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
    pub(super) fn preliminary_flag_computation(&mut self, block_idx: usize) {
        self.bbstate = 0;
        let (from, to) = BAND_BUCKETS[self.curband];
        if self.curband != 0 {
            for (boff, j) in (from..=to).enumerate() {
                // `recon` is now `i16`, so this is the decoder's own helper —
                // same flags, and the encoder picks up its AVX2 path too.
                let coefs = self.recon[block_idx].bucket(j);
                let bstatetmp = prelim_flags_bucket(coefs, &mut self.coeffstate[boff]);
                self.bucketstate[boff] = bstatetmp;
                self.bbstate |= bstatetmp;
            }
        } else {
            // Band 0: coeffstate[0] is pre-initialized by is_null_slice
            let coefs = self.recon[block_idx].bucket(0);
            let bstatetmp = band0_dispatch(coefs, &mut self.coeffstate[0]);
            self.bucketstate[0] = bstatetmp;
            self.bbstate |= bstatetmp;
        }
    }

    pub(super) fn encode_slice(&mut self, zp: &mut ZpEncoder) {
        #[cfg(feature = "iw44-probe")]
        let (probe_band, probe_before) = (self.curband, zp.bytes_written());
        if !self.is_null_slice() {
            if self.band_max.is_empty() {
                self.band_max = self.blocks.iter().map(band_maxima).collect();
            }
            for block_idx in 0..self.blocks.len() {
                if self.skip_quiet_block(zp, block_idx) {
                    continue;
                }
                self.preliminary_flag_computation(block_idx);
                let emit = self.block_band_encoding_pass(zp, block_idx);
                if emit {
                    self.bucket_encoding_pass(zp, block_idx);
                    self.newly_active_encoding_pass(zp, block_idx);
                }
                if (self.bbstate & ACTIVE) != 0 {
                    self.previously_active_encoding_pass(zp, block_idx);
                }
            }
        }
        self.finish_slice();
        #[cfg(feature = "iw44-probe")]
        probe::add_bytes(probe_band, (zp.bytes_written() - probe_before) as u64);
    }

    /// Codes a block whose band is still all zero in `recon` and stays so this
    /// slice, without the passes; returns false, coding nothing, otherwise.
    ///
    /// Such a block is most of a page in the early slices of every band but
    /// 0. With `recon` zero every coefficient is UNK, so the passes would code
    /// exactly this: one "no new bucket" bit for a 16-bucket band, else one
    /// "not new" bit per bucket, in the context `bucket_encoding_pass` takes
    /// from the bucket's parent coefficients `4i..4i+4` in a lower band
    /// (never active in this band). Nothing else changes, so the bytes and
    /// context states match the full passes.
    fn skip_quiet_block(&mut self, zp: &mut ZpEncoder, block_idx: usize) -> bool {
        #[cfg(test)]
        if self.full_passes {
            return false;
        }
        let band = self.curband;
        let (from, to) = BAND_BUCKETS[band];
        if band == 0
            || self.recon[block_idx].has_bucket(from)
            || u32::from(self.band_max[block_idx][band - 1]) >= self.quant_hi[band]
        {
            return false;
        }
        let bcount = to - from + 1;
        if bcount >= 16 {
            zp.encode_bit_inline(&mut self.ctx_decode_bucket[0], false);
            #[cfg(feature = "iw44-probe")]
            probe::record_block_band(band, false);
        } else {
            let recon = &self.recon[block_idx];
            for i in from..=to {
                let n = (4 * i..4 * i + 4)
                    .filter(|&j| recon.coef(j) != 0)
                    .count()
                    .min(3);
                zp.encode_bit_inline(&mut self.ctx_decode_coef[n + band * 8], false);
                #[cfg(feature = "iw44-probe")]
                probe::record_bucket_new(band, false);
            }
        }
        true
    }

    /// Mirrors decoder's `block_band_decoding_pass`.
    ///
    /// Encodes one bit (when needed) to tell the decoder whether any bucket
    /// in this band has a newly-active coefficient.
    pub(super) fn block_band_encoding_pass(
        &mut self,
        zp: &mut ZpEncoder,
        block_idx: usize,
    ) -> bool {
        let (from, to) = BAND_BUCKETS[self.curband];
        let bcount = to - from + 1;

        let should_encode_bit =
            bcount >= 16 && (self.bbstate & ACTIVE) == 0 && (self.bbstate & UNK) != 0;

        if should_encode_bit {
            // Determine if any UNK coefficient in this block-band will become active.
            let any_will_activate = self.any_unk_activates(block_idx, from, to);
            zp.encode_bit_inline(&mut self.ctx_decode_bucket[0], any_will_activate);
            #[cfg(feature = "iw44-probe")]
            probe::record_block_band(self.curband, any_will_activate);
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
    ///
    /// Significance must use the encoder's `|V| >= s` threshold. Activating at
    /// the decoder's lower 11s/16 decision boundary starts a coefficient one
    /// bitplane too early; its reconstruction is then too large for later
    /// refinement to bring back down on dense pages.
    pub(super) fn any_unk_activates(&self, block_idx: usize, from: usize, to: usize) -> bool {
        let steps = self.steps();
        let block = &self.blocks[block_idx];
        (from..=to).enumerate().any(|(boff, j)| {
            any_unk_reaches(&self.coeffstate[boff], &block[j << 4..(j + 1) << 4], &steps)
        })
    }

    /// The step of each of a bucket's 16 coefficients in the current band,
    /// clamped to `u16` for [`any_unk_reaches`].
    fn steps(&self) -> [u16; 16] {
        let clamp = |step: u32| step.min(u32::from(u16::MAX)) as u16;
        if self.curband == 0 {
            self.quant_lo.map(clamp)
        } else {
            [clamp(self.quant_hi[self.curband]); 16]
        }
    }

    /// Mirrors decoder's `bucket_decoding_pass` — encodes per-bucket NEW bits.
    pub(super) fn bucket_encoding_pass(&mut self, zp: &mut ZpEncoder, block_idx: usize) {
        let (from, to) = BAND_BUCKETS[self.curband];
        let steps = self.steps();
        for (boff, i) in (from..=to).enumerate() {
            if (self.bucketstate[boff] & UNK) == 0 {
                continue;
            }
            // Context index: count of active coefficients among the first 4 of the bucket.
            let mut n: usize = 0;
            if self.curband != 0 {
                let t = 4 * i;
                for j in t..t + 4 {
                    if self.recon[block_idx].coef(j) != 0 {
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
            // Will any UNK coefficient in this bucket become active? Same
            // test as `any_unk_activates` and the coefficient gate below.
            let is_new = any_unk_reaches(
                &self.coeffstate[boff],
                &self.blocks[block_idx][i << 4..(i + 1) << 4],
                &steps,
            );
            if is_new {
                self.bucketstate[boff] |= NEW;
            }
            zp.encode_bit_inline(&mut self.ctx_decode_coef[n + self.curband * 8], is_new);
            #[cfg(feature = "iw44-probe")]
            probe::record_bucket_new(self.curband, is_new);
        }
    }

    /// Mirrors decoder's `newly_active_coefficient_decoding_pass`.
    ///
    /// For each UNK coefficient in a NEW bucket: encodes whether it becomes active,
    /// and if so, its sign. Updates `recon` to match the decoder's new value.
    pub(super) fn newly_active_encoding_pass(&mut self, zp: &mut ZpEncoder, block_idx: usize) {
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
                    // The IW44 encoder makes a coefficient significant once its
                    // magnitude reaches this bitplane's quantization step.
                    let is_active = true_val.unsigned_abs() as i32 >= s;
                    zp.encode_bit_inline(&mut self.ctx_activate_coef[shift + ip], is_active);
                    #[cfg(feature = "iw44-probe")]
                    probe::record_activate(self.curband, is_active);
                    if is_active {
                        let negative = true_val < 0;
                        zp.encode_passthrough_iw44(negative);
                        // Mirror decoder: recon = sign * (s + s>>1 - s>>3)
                        let decoded_val = s + (s >> 1) - (s >> 3);
                        // `as i16` exactly as the decoder writes it.
                        let val = if negative { -decoded_val } else { decoded_val } as i16;
                        let block = &mut self.recon[block_idx];
                        block.grow_through(BAND_BUCKETS[self.curband].1);
                        block.bucket_mut(i)[k] = val;
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
    pub(super) fn previously_active_encoding_pass(&mut self, zp: &mut ZpEncoder, block_idx: usize) {
        let (from, to) = BAND_BUCKETS[self.curband];
        let mut step = self.quant_hi[self.curband];
        for (boff, i) in (from..=to).enumerate() {
            // An ACTIVE coefficient is non-zero by definition, so its bucket was
            // written. An absent bucket has nothing to refine.
            if self.recon[block_idx].bucket_mut_if_present(i).is_none() {
                continue;
            }
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
                let d = self.recon[block_idx].coef(coef_idx) as i32; // decoder's current value
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
                    zp.encode_bit_inline(&mut self.ctx_increase_coef[0], des);
                } else {
                    des = abs_v > abs_d;
                    zp.encode_passthrough_iw44(des);
                }
                #[cfg(feature = "iw44-probe")]
                probe::record_refine(self.curband, des);
                if des {
                    new_abs_d += s >> 1;
                } else {
                    new_abs_d += -s + (s >> 1);
                }
                // Update recon with the decoder's new value
                let sign = if d < 0 { -1i32 } else { 1i32 };
                // `as i16` exactly as the decoder writes it.
                self.recon[block_idx].bucket_mut(i)[k] = (sign * new_abs_d.max(0)) as i16;
            }
        }
    }

    pub(super) fn finish_slice(&mut self) {
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
