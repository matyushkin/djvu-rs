//! The per-channel coefficient decoder and plane reconstruction.

use super::*;

// ---- Per-channel wavelet decoder --------------------------------------------

/// State for a single YCbCr plane wavelet decoder.
///
/// Holds 32×32 block coefficients and the ZP context tables that persist
/// across progressive slices.
#[derive(Clone, Debug)]
pub(super) struct PlaneDecoder {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) block_cols: usize,
    /// Row-major array of 32×32 blocks. A block addresses 1024 i16
    /// coefficients in zigzag-scan order but stores only the buckets it really
    /// uses — see [`CoefBlock`].
    pub(super) blocks: Vec<CoefBlock>,
    /// Running total of the `CoefBlock::hi` lengths, so [`PlaneDecoder::heap_bytes`]
    /// stays O(1) instead of walking every block on each cache-budget query.
    pub(super) hi_len: usize,
    pub(super) quant_lo: [u32; 16],
    pub(super) quant_hi: [u32; 10],
    /// Current band index (0..10, wraps around).
    pub(super) curband: usize,
    // ZP context bytes — persistent across slices and chunks.
    pub(super) ctx_decode_bucket: [u8; 1],
    pub(super) ctx_decode_coef: [u8; 80],
    pub(super) ctx_activate_coef: [u8; 16],
    pub(super) ctx_increase_coef: [u8; 1],
    // Per-block temporary decode state (re-used each block, not persisted).
    pub(super) coeffstate: [[u8; 16]; 16],
    pub(super) bucketstate: [u8; 16],
    pub(super) bbstate: u8,
}

/// Map one bucket's 16 i16 coefficients to UNK/ACTIVE flags, store in `bucket`,
/// and return the OR of all flag bytes (bstatetmp).
///
/// Dispatches to NEON on aarch64, AVX2 on x86_64 when available, else scalar.
#[allow(unsafe_code)]
#[inline(always)]
pub(super) fn prelim_flags_bucket(coefs: &[i16; 16], bucket: &mut [u8; 16]) -> u8 {
    #[cfg(target_arch = "aarch64")]
    // SAFETY: NEON is mandatory on aarch64; `coefs` is exactly 16 i16 wide.
    return unsafe { prelim_flags_bucket_neon(coefs, bucket) };

    #[cfg(all(target_arch = "x86_64", feature = "std"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            // SAFETY: AVX2 was just feature-detected; `coefs` is exactly 16 i16 wide.
            return unsafe { prelim_flags_bucket_avx2(coefs, bucket) };
        }
    }

    #[cfg_attr(target_arch = "aarch64", allow(unreachable_code))]
    {
        let mut bstate = 0u8;
        for k in 0..16 {
            let f = if coefs[k] == 0 { UNK } else { ACTIVE };
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
pub(super) unsafe fn prelim_flags_bucket_neon(coefs: &[i16; 16], bucket: &mut [u8; 16]) -> u8 {
    use core::arch::aarch64::*;
    let ptr = coefs.as_ptr();
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

/// AVX2-vectorized version of `prelim_flags_bucket` for x86_64.
///
/// Loads 16 i16 in one `__m256i`, compares to zero with `_mm256_cmpeq_epi16`,
/// builds UNK/ACTIVE flags via `uv ^ (xv & nz)` where UNK=8 and XV=10
/// (= UNK ^ ACTIVE), narrows to 16 u8 with `_mm_packus_epi16` (saturating but
/// values 2/8 fit), stores via `_mm_storeu_si128`, and horizontally OR-reduces
/// the 16 bytes to one byte via shift+OR.
///
/// Mirror of `prelim_flags_bucket_neon` — same operations, AVX2 lanes.
#[cfg(all(target_arch = "x86_64", feature = "std"))]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "avx2")]
pub(super) unsafe fn prelim_flags_bucket_avx2(coefs: &[i16; 16], bucket: &mut [u8; 16]) -> u8 {
    use core::arch::x86_64::*;
    // Load the bucket's 16 contiguous i16 (32 bytes).
    let coefs = _mm256_loadu_si256(coefs.as_ptr() as *const __m256i);
    // eq: 0xFFFF where coef == 0, 0x0000 where != 0.
    let zero = _mm256_setzero_si256();
    let eq = _mm256_cmpeq_epi16(coefs, zero);
    // nz = !eq.  cmpeq(x, x) == all-ones.
    let all_ones = _mm256_cmpeq_epi16(zero, zero);
    let nz = _mm256_xor_si256(eq, all_ones);
    // result = UNK ^ ((UNK ^ ACTIVE) & nz)  ⟹  UNK(8) if zero, ACTIVE(2) if nonzero.
    let xv = _mm256_set1_epi16(10);
    let uv = _mm256_set1_epi16(8);
    let r16 = _mm256_xor_si256(uv, _mm256_and_si256(xv, nz));
    // Narrow u16 → u8: pack the two 128-bit halves.  `_mm_packus_epi16` saturates
    // to [0, 255] but our values are 2 or 8 — equivalent to truncation here.
    let r_lo = _mm256_castsi256_si128(r16);
    let r_hi = _mm256_extracti128_si256::<1>(r16);
    let packed = _mm_packus_epi16(r_lo, r_hi);
    _mm_storeu_si128(bucket.as_mut_ptr() as *mut __m128i, packed);
    // Horizontal OR of 16 u8 lanes → 1 byte via successive shift+OR.
    let or64 = _mm_or_si128(packed, _mm_unpackhi_epi64(packed, packed));
    let or32 = _mm_or_si128(or64, _mm_srli_si128::<4>(or64));
    let or16_red = _mm_or_si128(or32, _mm_srli_si128::<2>(or32));
    let or8 = _mm_or_si128(or16_red, _mm_srli_si128::<1>(or16_red));
    _mm_extract_epi8::<0>(or8) as u8
}

/// NEON-vectorized band-0 path of `preliminary_flag_computation`.
///
/// Band 0 differs from bands 1-9: only update entries where `old_flags[k] != ZERO (1)`.
/// Uses `vbslq_u8` to blend new flags (UNK/ACTIVE from coef) with old flags (keep ZERO).
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "neon")]
pub(super) unsafe fn prelim_flags_band0_neon(block: &[i16; 16], old_flags: &mut [u8; 16]) -> u8 {
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

/// AVX2-vectorized band-0 path of `preliminary_flag_computation` for x86_64.
///
/// Mirror of `prelim_flags_band0_neon`: only updates entries where
/// `old_flags[k] != ZERO(1)`; uses an SSE2 blend (`(new & m) | (old & ~m)`)
/// for the conditional-write step that NEON does with `vbslq_u8`.
#[cfg(all(target_arch = "x86_64", feature = "std"))]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "avx2")]
pub(super) unsafe fn prelim_flags_band0_avx2(block: &[i16; 16], old_flags: &mut [u8; 16]) -> u8 {
    use core::arch::x86_64::*;
    // Load old coeffstate[0] (16 u8 flags: ZERO=1, UNK=8, ACTIVE=2).
    let old_u8 = _mm_loadu_si128(old_flags.as_ptr() as *const __m128i);
    // should_update mask: 0xFF where old_flags[k] != ZERO(1), 0x00 where == ZERO.
    let one_u8 = _mm_set1_epi8(1);
    let is_zero_state = _mm_cmpeq_epi8(old_u8, one_u8);
    let all_ones_128 = _mm_cmpeq_epi8(old_u8, old_u8);
    let should_update = _mm_xor_si128(is_zero_state, all_ones_128);

    // Compute new flags from first 16 coefs (same recipe as prelim_flags_bucket_avx2 with base=0).
    let coefs = _mm256_loadu_si256(block.as_ptr() as *const __m256i);
    let zero = _mm256_setzero_si256();
    let eq = _mm256_cmpeq_epi16(coefs, zero);
    let all_ones_256 = _mm256_cmpeq_epi16(zero, zero);
    let nz = _mm256_xor_si256(eq, all_ones_256);
    let xv = _mm256_set1_epi16(10);
    let uv = _mm256_set1_epi16(8);
    let r16 = _mm256_xor_si256(uv, _mm256_and_si256(xv, nz));
    let r_lo = _mm256_castsi256_si128(r16);
    let r_hi = _mm256_extracti128_si256::<1>(r16);
    let new_flags = _mm_packus_epi16(r_lo, r_hi);

    // Blend: (new & should_update) | (old & ~should_update).
    let blended = _mm_or_si128(
        _mm_and_si128(should_update, new_flags),
        _mm_andnot_si128(should_update, old_u8),
    );
    _mm_storeu_si128(old_flags.as_mut_ptr() as *mut __m128i, blended);

    // Horizontal OR of 16 u8 lanes → 1 byte (same reduction as the bucket path).
    let or64 = _mm_or_si128(blended, _mm_unpackhi_epi64(blended, blended));
    let or32 = _mm_or_si128(or64, _mm_srli_si128::<4>(or64));
    let or16_red = _mm_or_si128(or32, _mm_srli_si128::<2>(or32));
    let or8 = _mm_or_si128(or16_red, _mm_srli_si128::<1>(or16_red));
    _mm_extract_epi8::<0>(or8) as u8
}

/// Dispatcher for the band-0 path of `preliminary_flag_computation`.
///
/// Picks NEON on aarch64, AVX2 on x86_64 when available, scalar otherwise.
#[allow(unsafe_code)]
#[inline(always)]
pub(super) fn band0_dispatch(block: &[i16; 16], old_flags: &mut [u8; 16]) -> u8 {
    #[cfg(target_arch = "aarch64")]
    // SAFETY: NEON always available on aarch64; block[0..16] valid by construction.
    return unsafe { prelim_flags_band0_neon(block, old_flags) };

    #[cfg(all(target_arch = "x86_64", feature = "std"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            // SAFETY: AVX2 was just feature-detected; block[0..16] valid by construction.
            return unsafe { prelim_flags_band0_avx2(block, old_flags) };
        }
    }

    #[cfg_attr(target_arch = "aarch64", allow(unreachable_code))]
    {
        let mut b = 0u8;
        for k in 0..16 {
            if old_flags[k] != ZERO {
                old_flags[k] = if block[k] == 0 { UNK } else { ACTIVE };
            }
            b |= old_flags[k];
        }
        b
    }
}

/// A bucket that was never written. Reading an absent bucket yields this —
/// exactly what a zero-filled one held before (see [`CoefBlock`]).
pub(super) const ZERO_BUCKET: [i16; 16] = [0; 16];

/// One 32x32 IW44 coefficient block, stored as the prefix of buckets that
/// actually holds data.
///
/// A block is 1024 coefficients in zigzag order, grouped into 64 buckets of 16.
/// Storing all of them costs a flat 2 KB per block whatever the image really
/// contains: 124 MB for one plane of a 6780x9148 page, and three such planes
/// made a single-page thumbnail peak at 383 MB. Almost all of it is zeros —
/// measured on that page, a complete four-chunk decode leaves **9.3 %** of the
/// luma buckets non-zero and **1.6 %** of the chroma ones; a first-chunk
/// preview leaves 3.5 % (PERF_EXPERIMENTS.md IW44_SPARSE_BLOCKS).
///
/// A block only ever needs a *prefix* of its buckets, so `lo` holds bucket 0
/// inline — every block has one — and `hi` covers buckets `1..=n`, growing on
/// the first write above bucket 0. An absent bucket reads as zero, which is
/// what a never-written bucket already held, so the decoder's behaviour is
/// unchanged: the UNK/ACTIVE flags it derives depend only on whether a
/// coefficient is zero.
#[derive(Clone, Debug, Default)]
pub(crate) struct CoefBlock {
    /// Bucket 0 — coefficients 0..16. Always present.
    pub(super) lo: [i16; 16],
    /// Buckets 1..=n — coefficients 16..16*(n+1). Empty until a coefficient
    /// above bucket 0 is written; `len()` is always a multiple of 16.
    pub(super) hi: Vec<i16>,
}

impl CoefBlock {
    /// Coefficient `i` in zigzag order; zero when its bucket is absent.
    #[inline(always)]
    pub(crate) fn coef(&self, i: usize) -> i16 {
        if i < 16 {
            self.lo[i]
        } else {
            self.hi.get(i - 16).copied().unwrap_or(0)
        }
    }

    /// Bucket `b`'s 16 coefficients, or [`ZERO_BUCKET`] when absent.
    #[inline(always)]
    pub(crate) fn bucket(&self, b: usize) -> &[i16; 16] {
        if b == 0 {
            return &self.lo;
        }
        let off = (b - 1) * 16;
        match self.hi.get(off..off + 16) {
            // `expect` cannot fire: the slice is 16 long by construction.
            Some(s) => s.try_into().expect("16-wide bucket slice"),
            None => &ZERO_BUCKET,
        }
    }

    /// Make buckets `0..=top` exist, and report how many coefficients that added.
    ///
    /// Growth is per band, not per bucket: a block is walked bucket by bucket in
    /// band order, so growing to the bucket asked for meant up to 64
    /// reallocations per block and cost 10 % on `iw44_decode_first_chunk`.
    #[inline]
    pub(crate) fn grow_through(&mut self, top: usize) -> usize {
        let need = top * 16;
        if self.hi.len() >= need {
            return 0;
        }
        let grow = need - self.hi.len();
        self.hi.reserve_exact(grow);
        self.hi.resize(need, 0);
        grow
    }

    /// Whether bucket `b` exists. Buckets grow in order, so an absent bucket
    /// means every bucket above it is absent too.
    #[cfg(feature = "std")]
    #[inline]
    pub(crate) fn has_bucket(&self, b: usize) -> bool {
        b == 0 || self.hi.len() >= b * 16
    }

    /// Bucket `b`, which must already exist (see [`grow_through`](Self::grow_through)).
    #[inline]
    pub(crate) fn bucket_mut(&mut self, b: usize) -> &mut [i16; 16] {
        if b == 0 {
            return &mut self.lo;
        }
        let off = (b - 1) * 16;
        // `expect` cannot fire: the slice is 16 long by construction.
        <&mut [i16; 16]>::try_from(&mut self.hi[off..off + 16]).expect("16-wide bucket slice")
    }

    /// Bucket `b` when it exists, else `None` — for a caller that knows an
    /// absent bucket has nothing to do.
    #[inline]
    pub(crate) fn bucket_mut_if_present(&mut self, b: usize) -> Option<&mut [i16; 16]> {
        if b == 0 {
            return Some(&mut self.lo);
        }
        let off = (b - 1) * 16;
        let s = self.hi.get_mut(off..off + 16)?;
        Some(<&mut [i16; 16]>::try_from(s).expect("16-wide bucket slice"))
    }

    /// Copy this block's coefficients `0..n` into `out`, zero-filling the rest.
    ///
    /// `reconstruct` scatters coefficients by zigzag index in a tight loop, so
    /// it materialises the prefix it needs once per block rather than paying
    /// [`coef`](Self::coef)'s bounds test per coefficient.
    #[inline]
    pub(super) fn materialize(&self, out: &mut [i16]) {
        let n = out.len();
        let lo = n.min(16);
        out[..lo].copy_from_slice(&self.lo[..lo]);
        if n > 16 {
            let hi = self.hi.len().min(n - 16);
            out[16..16 + hi].copy_from_slice(&self.hi[..hi]);
            out[16 + hi..].fill(0);
        }
    }
}

impl PlaneDecoder {
    /// Heap bytes held by this plane's coefficient array: the block index plus
    /// the buckets that were actually written (see [`CoefBlock`]).
    pub(super) fn heap_bytes(&self) -> usize {
        self.blocks.capacity() * core::mem::size_of::<CoefBlock>()
            + self.hi_len * core::mem::size_of::<i16>()
    }

    /// Bucket `b` of block `block_idx`, growing the block to reach it.
    ///
    /// Every write above bucket 0 goes through here so `hi_len` stays exact.
    #[inline]
    pub(super) fn bucket_mut(&mut self, block_idx: usize, b: usize) -> &mut [i16; 16] {
        // Grow to the end of the band `b` belongs to, not to `b` itself. A
        // block is walked bucket by bucket in band order, so growing per bucket
        // meant up to 64 reallocations per block and cost 10 % on
        // `iw44_decode_first_chunk`; per band there are at most 10. The band is
        // also the natural unit: the passes that follow read every bucket in it.
        let band_top = BAND_BUCKETS[self.curband].1;
        debug_assert!(
            b >= BAND_BUCKETS[self.curband].0 && b <= band_top,
            "bucket {b} is outside band {}",
            self.curband
        );
        let block = &mut self.blocks[block_idx];
        self.hi_len += block.grow_through(band_top);
        block.bucket_mut(b)
    }

    pub(super) fn new(width: usize, height: usize) -> Self {
        let block_cols = width.div_ceil(32);
        let block_rows = height.div_ceil(32);
        let block_count = block_cols * block_rows;
        PlaneDecoder {
            width,
            height,
            block_cols,
            blocks: vec![CoefBlock::default(); block_count],
            hi_len: 0,
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
    pub(super) fn decode_slice(&mut self, zp: &mut ZpDecoder<'_>) {
        if !self.is_null_slice() {
            for block_idx in 0..self.blocks.len() {
                self.preliminary_flag_computation(block_idx);
                if self.block_band_decoding_pass(zp) && self.bucket_decoding_pass(zp, block_idx) {
                    self.newly_active_coefficient_decoding_pass(zp, block_idx);
                }
                // Skip the inner loop entirely when no ACTIVE coefficients exist
                // (avoids function call + zp register flush for fresh/sparse blocks).
                if (self.bbstate & ACTIVE) != 0 {
                    self.previously_active_coefficient_decoding_pass(zp, block_idx);
                }
            }
        }
        self.finish_slice();
    }

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

    pub(super) fn preliminary_flag_computation(&mut self, block_idx: usize) {
        self.bbstate = 0;
        let (from, to) = BAND_BUCKETS[self.curband];

        if self.curband != 0 {
            // The band's buckets are consecutive in the block's tail, so resolve
            // the block and its tail length once instead of per bucket: this
            // loop runs for every band of every block and the indexing showed up
            // as a few percent on `iw44_decode_first_chunk`.
            let hi = &self.blocks[block_idx].hi[..];
            for (boff, j) in (from..=to).enumerate() {
                let off = (j - 1) * 16;
                let coefs = match hi.get(off..off + 16) {
                    Some(s) => <&[i16; 16]>::try_from(s).expect("16-wide bucket slice"),
                    None => &ZERO_BUCKET,
                };
                let bstatetmp = prelim_flags_bucket(coefs, &mut self.coeffstate[boff]);
                self.bucketstate[boff] = bstatetmp;
                self.bbstate |= bstatetmp;
            }
        } else {
            let bstatetmp =
                band0_dispatch(self.blocks[block_idx].bucket(0), &mut self.coeffstate[0]);
            self.bucketstate[0] = bstatetmp;
            self.bbstate |= bstatetmp;
        }
    }

    pub(super) fn block_band_decoding_pass(&mut self, zp: &mut ZpDecoder<'_>) -> bool {
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
    pub(super) fn bucket_decoding_pass(
        &mut self,
        zp: &mut ZpDecoder<'_>,
        block_idx: usize,
    ) -> bool {
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
                    if self.blocks[block_idx].coef(j) != 0 {
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

    pub(super) fn newly_active_coefficient_decoding_pass(
        &mut self,
        zp: &mut ZpDecoder<'_>,
        block_idx: usize,
    ) {
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
                            self.bucket_mut(block_idx, i)[j] = val as i16;
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
    pub(super) fn previously_active_coefficient_decoding_pass(
        &mut self,
        zp: &mut ZpDecoder<'_>,
        block_idx: usize,
    ) {
        use djvu_zp::tables::{LPS_NEXT, MPS_NEXT, PROB, THRESHOLD};

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
            // An ACTIVE coefficient is by definition non-zero, so its bucket
            // was written and `bucket_mut` never grows the block here. Skipping
            // buckets with none also skips that call entirely.
            // An ACTIVE coefficient is by definition non-zero, so its bucket
            // was written. An absent bucket therefore has nothing to refine.
            let bucket = match self.blocks[block_idx].bucket_mut_if_present(i) {
                Some(b) => b,
                None => continue,
            };
            let flags = &self.coeffstate[boff];
            for (j, slot) in bucket.iter_mut().enumerate() {
                if (flags[j] & ACTIVE) != 0 {
                    if self.curband == 0 {
                        step = self.quant_lo[j];
                    }
                    let coef = *slot;
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
                    *slot = if coef < 0 {
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

    /// Apply the inverse wavelet transform and return a flat `i16` array.
    ///
    /// The returned vector is row-major, with stride = `width.div_ceil(32)*32`.
    /// `subsample` ≥ 1 controls the resolution (1 = full, 2 = half, etc.).
    pub(super) fn reconstruct(&self, subsample: usize) -> FlatPlane {
        // ── Fast path for sub≥2: compact plane ────────────────────────────────
        //
        // For subsample=2 the wavelet only ever reads/writes (even_row, even_col)
        // positions — those with zigzag index i < 256 (see zigzag_row/col: both
        // are even iff bits 8 and 9 of i are 0).  We can therefore:
        //   1. Allocate a 4× smaller plane  (ceil(w/2) × ceil(h/2))
        //   2. Scatter only the sub_block² low-frequency coefficients per block
        //      (zigzag indices 0..sub_block² map to even multiples of sub)
        //   3. Run the full wavelet (sub=1) on the compact plane, which now
        //      includes the SIMD s=1 pass.
        //
        // This is equivalent to running the wavelet at sub=2 on the full plane
        // and sampling every other position: each compact[k][c] equals the value
        // that full[k·sub][c·sub] would hold after the sub=2 wavelet.
        //
        // The same logic holds for sub=4 (8×8 sub-block) and sub=8 (4×4 sub-block).
        if (2..=8).contains(&subsample) && subsample.is_power_of_two() {
            let sub = subsample;

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

            // Safety: zigzag_row(i)/sub × zigzag_col(i)/sub for i in 0..sub_block²
            // is a bijection over [0..sub_block) × [0..sub_block) (bits 8/9 of i are
            // 0 → both zigzag values are even; dividing by sub tiles all sub_block²
            // positions per block → every element is written before the wavelet reads).
            #[allow(unsafe_code)]
            let mut plane = FlatPlane {
                data: unsafe { uninit_i16_vec(compact_stride * compact_rows) },
                stride: compact_stride,
            };

            // Row-major scatter via compact inverse zigzag tables: write
            // sub_block consecutive i16 per row before advancing, maximising
            // write-combine efficiency (one cache line per row for sub=2).
            // Safety invariants for get_unchecked below:
            //   inv: inv_base+col = row*sub_block+col, row,col ∈ 0..sub_block → < sub_block²
            //        = compact_inv.len(); block[i]: compact_inv values < sub_block² ≤ 256
            //        < 1024 = block.len(); plane[dst_base+col]: sequential within
            //        (base_row+row)*compact_stride+base_col+[0,sub_block) — all in bounds.
            let compact_inv: &[u8] = match sub {
                2 => &ZIGZAG_INV_SUB2,
                4 => &ZIGZAG_INV_SUB4,
                _ => &ZIGZAG_INV_SUB8, // sub=8
            };
            // The compact tables only ever name zigzag indices < sub_block², so
            // each block is materialised into that prefix once (see
            // `CoefBlock::materialize`) and the scatter below stays a tight
            // read of a contiguous array.
            let mut prefix = [0i16; 256];
            #[allow(unsafe_code)]
            for r in 0..block_rows {
                for c in 0..self.block_cols {
                    let block = &self.blocks[r * self.block_cols + c];
                    block.materialize(&mut prefix[..sub_block * sub_block]);
                    let base_row = r * sub_block;
                    let base_col = c * sub_block;
                    for row in 0..sub_block {
                        let dst_base = (base_row + row) * compact_stride + base_col;
                        let inv_base = row * sub_block;
                        for col in 0..sub_block {
                            // Safety: see invariants above.
                            let i = unsafe { *compact_inv.get_unchecked(inv_base + col) } as usize;
                            unsafe {
                                *plane.data.get_unchecked_mut(dst_base + col) =
                                    *prefix.get_unchecked(i);
                            }
                        }
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
        // Safety: ZIGZAG_ROW/COL for i in 0..1024 is a bijection over [0..32)×[0..32)
        // (odd-indexed bits → row, even-indexed bits → col, non-overlapping). The
        // scatter below writes every element before the wavelet reads any of them.
        #[allow(unsafe_code)]
        let mut plane = FlatPlane {
            data: unsafe { uninit_i16_vec(full_width * full_height) },
            stride: full_width,
        };

        // Row-major scatter via ZIGZAG_INV: write 32 consecutive i16 per row
        // (= 1 cache line) before advancing, maximising write-combine efficiency.
        // block[ZIGZAG_INV[row*32+col]] is a gathered read from a 2 KB array
        // that fits in L1, so the scatter cost is minimal.
        let mut full = [0i16; 1024];
        for r in 0..block_rows {
            for c in 0..self.block_cols {
                self.blocks[r * self.block_cols + c].materialize(&mut full);
                let row_base = r << 5;
                let col_base = c << 5;
                for row in 0..32usize {
                    let dst_base = (row_base + row) * full_width + col_base;
                    let inv_base = row * 32;
                    for col in 0..32usize {
                        let i = ZIGZAG_INV[inv_base + col] as usize;
                        plane.data[dst_base + col] = full[i];
                    }
                }
            }
        }

        inverse_wavelet_transform(&mut plane, self.width, self.height, subsample);
        plane
    }

    /// Reconstruct a window of the full-resolution plane: block rows
    /// `[first_block, last_block)` and block columns `[first_col, last_col)`.
    ///
    /// The returned plane's row 0 and column 0 are the image's absolute row
    /// `first_block * 32` and column `first_col * 32` (`* 16` in the compact
    /// scale-2 plane). All block columns give a horizontal band with the
    /// stride `reconstruct(1)` would give.
    ///
    /// The inverse wavelet couples rows: a pass at scale `s` reads three
    /// samples either side, so an output row depends on rows up to
    /// `3 * (16 + 8 + 4 + 2 + 1) = 93` away, doubled to `186` by the two
    /// lifting stages of each pass. A caller therefore asks for more block rows
    /// than it keeps — see [`BAND_HALO_BLOCKS`] — and uses only the interior.
    /// The edges of the band carry the transform's own boundary handling, which
    /// is correct only where the band edge is the image edge. The transform
    /// reaches as far across columns as across rows, so a window needs the
    /// same halo on its left and right as above and below.
    ///
    /// Block coordinates are what make the window's row and column
    /// coordinates agree with the full plane's on every scale: 32 is a
    /// multiple of the coarsest pass's 16.
    pub(super) fn reconstruct_window(
        &self,
        (first_block, last_block): (usize, usize),
        (first_col, last_col): (usize, usize),
        sub: usize,
    ) -> FlatPlane {
        debug_assert!(first_block < last_block);
        debug_assert!(first_col < last_col);
        debug_assert!(sub == 1 || sub == 2);
        // A block contributes `side` rows and columns: all 32 at full
        // resolution, or the 16x16 even samples of the compact scale-2 plane
        // (see `reconstruct`).
        let side = 32 / sub;
        let inv = |k: usize| -> usize {
            if sub == 1 {
                ZIGZAG_INV[k] as usize
            } else {
                ZIGZAG_INV_SUB2[k] as usize
            }
        };
        let block_rows = self.height.div_ceil(32);
        let last_block = last_block.min(block_rows);
        let last_col = last_col.min(self.block_cols);
        let stride = (last_col - first_col) * side;
        let band_rows = (last_block - first_block) * side;

        // Safety: as in `reconstruct` — the scatter below writes every element
        // before the wavelet reads any of them.
        #[allow(unsafe_code)]
        let mut plane = FlatPlane {
            data: unsafe { uninit_i16_vec(stride * band_rows) },
            stride,
        };

        let mut full = [0i16; 1024];
        for r in first_block..last_block {
            for c in first_col..last_col {
                self.blocks[r * self.block_cols + c].materialize(&mut full[..side * side]);
                let row_base = (r - first_block) * side;
                let col_base = (c - first_col) * side;
                for row in 0..side {
                    let dst_base = (row_base + row) * stride + col_base;
                    let inv_base = row * side;
                    for col in 0..side {
                        plane.data[dst_base + col] = full[inv(inv_base + col)];
                    }
                }
            }
        }

        // The logical height decides where the transform applies its boundary
        // handling. A band that reaches the bottom of the image must report the
        // image's own remaining height, so that boundary is the real one.
        let logical = if last_block == block_rows {
            self.height.div_ceil(sub) - first_block * side
        } else {
            band_rows
        };
        // The same for the right edge.
        let logical_w = if last_col == self.block_cols {
            self.width.div_ceil(sub) - first_col * side
        } else {
            stride
        };
        inverse_wavelet_transform_from(&mut plane, logical_w, logical, 1, 16 / sub);
        plane
    }
}

/// Reconstruct the three colour planes.
///
/// With the `parallel` feature the three independent inverse wavelet
/// transforms run concurrently on separate rayon threads, cutting the
/// reconstruction wall time from Y+Cb+Cr sequential to max(Y, Cb, Cr) —
/// roughly 1.5-2x faster on large pages, where Y dominates.
pub(super) fn reconstruct_planes(
    y_dec: &PlaneDecoder,
    cb_dec: &PlaneDecoder,
    cr_dec: &PlaneDecoder,
    sub: usize,
    chroma_sub: usize,
) -> (FlatPlane, FlatPlane, FlatPlane) {
    #[cfg(feature = "parallel")]
    {
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
    }
    #[cfg(not(feature = "parallel"))]
    {
        (
            y_dec.reconstruct(sub),
            cb_dec.reconstruct(chroma_sub),
            cr_dec.reconstruct(chroma_sub),
        )
    }
}

/// Write image rows `rows` of a full-resolution colour page into `out`.
///
/// `out` holds exactly those rows as RGBA, top to bottom, each `pw` pixels
/// wide; only columns `cols` of each row are written. The planes need not
/// cover the whole image: `y_row0` and `c_row0` say which plane row each
/// plane's row 0 holds, and `y_col0` and `c_col0` which column its column 0
/// holds, which is what lets a banded or windowed caller pass a slice of the
/// page. With `chroma_half`, `cols.start` must be even, so that each chroma
/// value still covers a pair of columns. With `chroma_half` the chroma planes are the scale-2 reconstruction,
/// one row per two image rows, and `c_row0` counts those half rows. DjVu stores rows bottom-to-top, so image row `r` is the output row
/// `ph - 1 - r` of the whole picture, and the first row of `out`.
#[allow(clippy::too_many_arguments)]
pub(super) fn convert_rgb_rows(
    chroma_half: bool,
    y: &FlatPlane,
    y_row0: usize,
    cb: &FlatPlane,
    cr: &FlatPlane,
    c_row0: usize,
    (y_col0, c_col0): (usize, usize),
    rows: core::ops::Range<usize>,
    cols: core::ops::Range<usize>,
    pw: usize,
    ph: usize,
    out: &mut [u8],
) {
    let out_lo = ph - rows.end;
    debug_assert_eq!(out.len(), rows.len() * pw * 4);
    debug_assert!(cols.start <= cols.end && cols.end <= pw);
    debug_assert!(!chroma_half || cols.start.is_multiple_of(2));
    let (x0, n) = (cols.start, cols.len());

    // One output row. With `chroma_half` the chroma planes hold one value per
    // 2x2 block, so image row `row` reads chroma row `row / 2` and each value
    // covers two columns: DjVuLibre's `Map::image` replication, rows paired
    // from the bottom as the planes store them.
    let convert_row = |row: usize, row_data: &mut [u8]| {
        let y_off = (row - y_row0) * y.stride + (x0 - y_col0);
        let y_row = &y.data[y_off..y_off + n];
        let row_data = &mut row_data[x0 * 4..(x0 + n) * 4];
        if chroma_half {
            let c_off = (row / 2 - c_row0) * cb.stride + (x0 / 2 - c_col0);
            let cw = n.div_ceil(2);
            ycbcr_row_from_i16_half(
                y_row,
                &cb.data[c_off..c_off + cw],
                &cr.data[c_off..c_off + cw],
                row_data,
                n,
            );
        } else {
            let c_off = (row - c_row0) * cb.stride + (x0 - c_col0);
            ycbcr_row_from_i16(
                y_row,
                &cb.data[c_off..c_off + n],
                &cr.data[c_off..c_off + n],
                row_data,
            );
        }
    };

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        out.par_chunks_mut(pw * 4)
            .enumerate()
            .for_each(|(i, row_data)| convert_row(ph - 1 - (out_lo + i), row_data));
    }
    #[cfg(not(feature = "parallel"))]
    {
        for (i, row_data) in out.chunks_mut(pw * 4).enumerate() {
            convert_row(ph - 1 - (out_lo + i), row_data); // DjVu rows are bottom-to-top
        }
    }
}

/// Planes smaller than this reconstruct whole: banding would cost work and
/// save memory nobody is short of. 128 MiB is about a 4600x4600 colour page.
pub(super) const BAND_MIN_PLANE_BYTES: usize = 128 * 1024 * 1024;

/// What one band of planes may cost. A band holds its kept rows plus a halo on
/// each side, so this is the real working set, not the kept part.
pub(super) const BAND_BUDGET_BYTES: usize = 128 * 1024 * 1024;

/// The smallest band worth keeping: four halos, so the doubled halo work is at
/// most half of the band's own.
pub(super) const BAND_MIN_KEEP_BLOCKS: usize = 4 * BAND_HALO_BLOCKS;

/// How many block rows one band keeps, or `None` to reconstruct whole planes.
///
/// Banding trades work for memory. The halo rows are transformed twice, so a
/// band keeping `k` block rows does `(k + 2 * BAND_HALO_BLOCKS) / k` of the
/// whole-plane work. Only a page whose planes are genuinely large is worth
/// that; under [`BAND_MIN_PLANE_BYTES`] the whole-plane path runs exactly as
/// it did before.
///
/// `out_bytes_per_px` is what the caller keeps per kept pixel beside the
/// planes: 0 when the RGB goes into a picture that exists anyway, 4 when the
/// caller holds one band of RGB rows and nothing else (#811). Those bytes come
/// out of the same budget, so such a band keeps fewer block rows.
pub(super) fn band_keep_blocks(
    y_dec: &PlaneDecoder,
    chroma_half: bool,
    out_bytes_per_px: usize,
) -> Option<usize> {
    let stride = y_dec.width.div_ceil(32) * 32;
    // Bytes the three planes hold per luma row. Luma is two bytes a pixel; the
    // two chroma planes add two more each, or one more together when chroma is
    // stored at half resolution in both directions.
    let per_row = if chroma_half { stride * 3 } else { stride * 6 };
    let block_rows = y_dec.height.div_ceil(32);
    if per_row.saturating_mul(block_rows * 32) <= BAND_MIN_PLANE_BYTES {
        return None;
    }
    let per_block_row = per_row * 32;
    // The halos are pure plane rows; every kept block row also carries the
    // caller's output bytes.
    let halo_bytes = 2 * BAND_HALO_BLOCKS * per_block_row;
    let per_kept_block_row = per_block_row + stride * 32 * out_bytes_per_px;
    let affordable = BAND_BUDGET_BYTES.saturating_sub(halo_bytes) / per_kept_block_row;
    let keep = affordable.max(BAND_MIN_KEEP_BLOCKS);
    // Band only when a band really is a part of the page. A `keep` just under
    // `block_rows` would split the page into two bands that each carry almost
    // all of it: the halo work doubles and the memory saving is nearly zero.
    // Half the page is the point where the saving pays for the extra pass.
    (keep * 2 <= block_rows).then_some(keep)
}

/// Block rows of overlap a band needs on each side before its interior is
/// exact. The transform's vertical reach is 186 rows (see
/// [`PlaneDecoder::reconstruct_window`]); 8 block rows is 256, the next block
/// multiple above it with margin to spare.
pub(super) const BAND_HALO_BLOCKS: usize = 8;

// ---- Flat plane helper -------------------------------------------------------

/// Allocate `n` uninitialized `i16` elements.
///
/// Uses `Vec<MaybeUninit<i16>>` (the clippy-blessed pattern) and reinterprets
/// as `Vec<i16>`.
///
/// # Safety
/// Caller must write every element before reading it.
#[allow(unsafe_code)]
pub(super) unsafe fn uninit_i16_vec(n: usize) -> Vec<i16> {
    use core::mem::MaybeUninit;
    let mut v: Vec<MaybeUninit<i16>> = Vec::with_capacity(n);
    // Safety: MaybeUninit<i16> requires no initialization; len will equal capacity.
    unsafe { v.set_len(n) };
    let mut md = core::mem::ManuallyDrop::new(v);
    // Safety: MaybeUninit<i16> and i16 have identical layout; capacity unchanged.
    unsafe { Vec::from_raw_parts(md.as_mut_ptr().cast::<i16>(), md.len(), md.capacity()) }
}

pub(super) struct FlatPlane {
    pub(super) data: Vec<i16>,
    pub(super) stride: usize,
}
