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

/// Inverse zigzag: `ZIGZAG_INV[row * 32 + col]` is the index `i` such that
/// `zigzag_row(i) == row as u8 && zigzag_col(i) == col as u8`.
///
/// Enables row-major scatter (sequential writes to the plane) at the cost of
/// gathering block coefficients in zigzag order (2 KB block fits in L1).
static ZIGZAG_INV: [u16; 1024] = {
    let mut table = [0u16; 1024];
    let mut i = 0usize;
    while i < 1024 {
        let r = zigzag_row(i) as usize;
        let c = zigzag_col(i) as usize;
        table[r * 32 + c] = i as u16;
        i += 1;
    }
    table
};

/// Compact inverse zigzag for sub=2 (16×16 sub-block, 256 entries).
/// `ZIGZAG_INV_SUB2[row * 16 + col]` = index `i` in 0..256 such that
/// `zigzag_row(i) >> 1 == row && zigzag_col(i) >> 1 == col`.
static ZIGZAG_INV_SUB2: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut i = 0usize;
    while i < 256 {
        let r = (zigzag_row(i) >> 1) as usize;
        let c = (zigzag_col(i) >> 1) as usize;
        table[r * 16 + c] = i as u8;
        i += 1;
    }
    table
};

/// Compact inverse zigzag for sub=4 (8×8 sub-block, 64 entries).
/// `ZIGZAG_INV_SUB4[row * 8 + col]` = index `i` in 0..64.
static ZIGZAG_INV_SUB4: [u8; 64] = {
    let mut table = [0u8; 64];
    let mut i = 0usize;
    while i < 64 {
        let r = (zigzag_row(i) >> 2) as usize;
        let c = (zigzag_col(i) >> 2) as usize;
        table[r * 8 + c] = i as u8;
        i += 1;
    }
    table
};

/// Compact inverse zigzag for sub=8 (4×4 sub-block, 16 entries).
/// `ZIGZAG_INV_SUB8[row * 4 + col]` = index `i` in 0..16.
static ZIGZAG_INV_SUB8: [u8; 16] = {
    let mut table = [0u8; 16];
    let mut i = 0usize;
    while i < 16 {
        let r = (zigzag_row(i) >> 3) as usize;
        let c = (zigzag_col(i) >> 3) as usize;
        table[r * 4 + c] = i as u8;
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
    debug_assert_eq!(y_row.len(), cb_row.len());
    debug_assert_eq!(y_row.len(), cr_row.len());
    debug_assert_eq!(out.len(), y_row.len() * 4);

    let w = y_row.len();

    #[cfg(target_arch = "aarch64")]
    {
        #[allow(unsafe_code)]
        unsafe {
            ycbcr_neon(
                y_row.as_ptr(),
                cb_row.as_ptr(),
                cr_row.as_ptr(),
                out.as_mut_ptr(),
                w,
            )
        };
        return;
    }

    // Portable path: chunks_exact eliminates per-element bounds checks.
    #[allow(unreachable_code)]
    ycbcr_portable(y_row, cb_row, cr_row, out, w);
}

/// Convert raw i16 plane row data to RGBA, fusing normalize + YCbCr in one pass.
///
/// Uses `ycbcr_neon_raw` on AArch64 (avoids three intermediate i32 buffers and
/// the separate normalize loops).  Falls back to two-pass on other targets.
///
/// `y`, `cb`, `cr` must all have the same length `w`; `out` must hold `w * 4` bytes.
#[inline]
fn ycbcr_row_from_i16(y: &[i16], cb: &[i16], cr: &[i16], out: &mut [u8]) {
    let w = y.len();
    debug_assert_eq!(cb.len(), w);
    debug_assert_eq!(cr.len(), w);
    debug_assert_eq!(out.len(), w * 4);
    #[cfg(target_arch = "aarch64")]
    {
        #[allow(unsafe_code)]
        unsafe {
            ycbcr_neon_raw(y.as_ptr(), cb.as_ptr(), cr.as_ptr(), out.as_mut_ptr(), w);
        }
        return;
    }
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            #[allow(unsafe_code)]
            unsafe {
                ycbcr_avx2_raw(y.as_ptr(), cb.as_ptr(), cr.as_ptr(), out.as_mut_ptr(), w);
            }
            return;
        }
    }
    #[allow(unreachable_code)]
    {
        let mut y_norm = vec![0i32; w];
        let mut cb_norm = vec![0i32; w];
        let mut cr_norm = vec![0i32; w];
        for (col, v) in y_norm.iter_mut().enumerate() {
            *v = normalize(y[col]);
        }
        for col in 0..w {
            cb_norm[col] = normalize(cb[col]);
            cr_norm[col] = normalize(cr[col]);
        }
        ycbcr_row_to_rgba(&y_norm, &cb_norm, &cr_norm, out);
    }
}

/// Convert raw i16 plane row data to RGBA with chroma at half horizontal resolution.
///
/// `y` has length ≥ `w`; `cb_half`/`cr_half` have length ≥ `(w+1)/2`.  Each
/// chroma sample is nearest-neighbour upsampled to two adjacent output pixels.
/// Uses `ycbcr_neon_raw_half` on AArch64; two-pass fallback elsewhere.
#[inline]
fn ycbcr_row_from_i16_half(y: &[i16], cb_half: &[i16], cr_half: &[i16], out: &mut [u8], w: usize) {
    debug_assert!(y.len() >= w);
    debug_assert_eq!(out.len(), w * 4);
    #[cfg(target_arch = "aarch64")]
    {
        #[allow(unsafe_code)]
        unsafe {
            ycbcr_neon_raw_half(
                y.as_ptr(),
                cb_half.as_ptr(),
                cr_half.as_ptr(),
                out.as_mut_ptr(),
                w,
            );
        }
        return;
    }
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            #[allow(unsafe_code)]
            unsafe {
                ycbcr_avx2_raw_half(
                    y.as_ptr(),
                    cb_half.as_ptr(),
                    cr_half.as_ptr(),
                    out.as_mut_ptr(),
                    w,
                );
            }
            return;
        }
    }
    #[allow(unreachable_code)]
    {
        let mut y_norm = vec![0i32; w];
        let mut cb_norm = vec![0i32; w];
        let mut cr_norm = vec![0i32; w];
        for (col, v) in y_norm.iter_mut().enumerate() {
            *v = normalize(y[col]);
        }
        for col in 0..w {
            cb_norm[col] = normalize(cb_half[col / 2]);
            cr_norm[col] = normalize(cr_half[col / 2]);
        }
        ycbcr_row_to_rgba(&y_norm, &cb_norm, &cr_norm, out);
    }
}

/// Portable YCbCr→RGBA using chunks_exact so LLVM sees exact 8-element slices.
#[inline(always)]
fn ycbcr_portable(y_row: &[i32], cb_row: &[i32], cr_row: &[i32], out: &mut [u8], w: usize) {
    use wide::i32x8;
    let c128 = i32x8::splat(128);
    let c0 = i32x8::splat(0);
    let c255 = i32x8::splat(255);

    let full8 = w / 8;
    for (((yc, cbc), crc), outc) in y_row[..full8 * 8]
        .chunks_exact(8)
        .zip(cb_row[..full8 * 8].chunks_exact(8))
        .zip(cr_row[..full8 * 8].chunks_exact(8))
        .zip(out[..full8 * 32].chunks_exact_mut(32))
    {
        let ys = i32x8::from([yc[0], yc[1], yc[2], yc[3], yc[4], yc[5], yc[6], yc[7]]);
        let bs = i32x8::from([
            cbc[0], cbc[1], cbc[2], cbc[3], cbc[4], cbc[5], cbc[6], cbc[7],
        ]);
        let rs = i32x8::from([
            crc[0], crc[1], crc[2], crc[3], crc[4], crc[5], crc[6], crc[7],
        ]);
        let t2 = rs + (rs >> 1_i32);
        let t3 = ys + c128 - (bs >> 2_i32);
        let red = (ys + c128 + t2).max(c0).min(c255).to_array();
        let grn = (t3 - (t2 >> 1_i32)).max(c0).min(c255).to_array();
        let blu = (t3 + (bs << 1_i32)).max(c0).min(c255).to_array();
        for i in 0..8 {
            outc[i * 4] = red[i] as u8;
            outc[i * 4 + 1] = grn[i] as u8;
            outc[i * 4 + 2] = blu[i] as u8;
            outc[i * 4 + 3] = 255;
        }
    }
    for col in (full8 * 8)..w {
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

/// AArch64 NEON fused normalize + YCbCr→RGBA from raw i16 plane data (non-chroma-half).
///
/// Loads 8 i16 per channel, applies `normalize()` inline using `vrshrq_n_s16`
/// (rounding-shift by 6, i.e. `(v+32)>>6`) and clamps to `[-128,127]`, then
/// runs the YCbCr→RGBA formula.  Eliminates the separate normalize pass and the
/// three intermediate i32 buffers.
///
/// `cbp` and `crp` must point to `w` values each (same stride as `yp`).
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "neon")]
unsafe fn ycbcr_neon_raw(
    yp: *const i16,
    cbp: *const i16,
    crp: *const i16,
    outp: *mut u8,
    w: usize,
) {
    use core::arch::aarch64::*;
    // After normalize+clamp all values ∈ [-128, 127].  The YCbCr arithmetic
    // intermediates all fit in i16 (proof: y128∈[0,255], t2∈[-192,190],
    // t3∈[-31,287], r16∈[-192,445], g16∈[-126,383], b16∈[-287,541]).
    // vqmovun_s16 saturates signed i16 → unsigned u8, clamping to [0,255]
    // in one instruction — no separate min/max clamp ops needed.
    let n_min = vdupq_n_s16(-128);
    let n_max = vdupq_n_s16(127);
    let c128 = vdupq_n_s16(128);
    let alpha = vdup_n_u8(255);

    let full8 = w / 8;
    for i in 0..full8 {
        let off = i * 8;
        // Load + normalize (rounded right-shift by 6) + clamp to [-128, 127] at i16
        let yc = vmaxq_s16(
            vminq_s16(vrshrq_n_s16::<6>(vld1q_s16(yp.add(off))), n_max),
            n_min,
        );
        let cbc = vmaxq_s16(
            vminq_s16(vrshrq_n_s16::<6>(vld1q_s16(cbp.add(off))), n_max),
            n_min,
        );
        let crc = vmaxq_s16(
            vminq_s16(vrshrq_n_s16::<6>(vld1q_s16(crp.add(off))), n_max),
            n_min,
        );
        // All arithmetic stays at i16 — no widening to i32 needed.
        // y128 = y + 128, range [0, 255]
        let y128 = vaddq_s16(yc, c128);
        // t2 = cr + (cr >> 1) = 1.5·cr, range [-192, 190]
        let t2 = vaddq_s16(crc, vshrq_n_s16::<1>(crc));
        // t3 = y128 - (cb >> 2), range [-31, 287]
        let t3 = vsubq_s16(y128, vshrq_n_s16::<2>(cbc));
        // R = y128 + t2, range [-192, 445]
        let r16 = vaddq_s16(y128, t2);
        // G = t3 - (t2 >> 1), range [-126, 383]
        let g16 = vsubq_s16(t3, vshrq_n_s16::<1>(t2));
        // B = t3 + 2·cb, range [-287, 541]
        let b16 = vaddq_s16(t3, vshlq_n_s16::<1>(cbc));
        // Saturating narrow signed i16 → unsigned u8 (clamps to [0, 255])
        let r8 = vqmovun_s16(r16);
        let g8 = vqmovun_s16(g16);
        let b8 = vqmovun_s16(b16);
        vst4_u8(outp.add(off * 4), uint8x8x4_t(r8, g8, b8, alpha));
    }
    // Scalar tail
    for col in (full8 * 8)..w {
        let y = normalize(*yp.add(col));
        let b = normalize(*cbp.add(col));
        let r = normalize(*crp.add(col));
        let t2 = r + (r >> 1);
        let t3 = y + 128 - (b >> 2);
        *outp.add(col * 4) = (y + 128 + t2).clamp(0, 255) as u8;
        *outp.add(col * 4 + 1) = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
        *outp.add(col * 4 + 2) = (t3 + (b << 1)).clamp(0, 255) as u8;
        *outp.add(col * 4 + 3) = 255;
    }
}

/// AArch64 NEON fused normalize + YCbCr→RGBA from raw i16 plane data (chroma-half).
///
/// `cbp` and `crp` point to chroma planes at half the horizontal resolution.
/// Each chroma sample is nearest-neighbour upsampled to two luma columns.
/// 8 output pixels are produced per iteration, consuming 8 Y samples and 4 Cb/Cr samples.
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "neon")]
unsafe fn ycbcr_neon_raw_half(
    yp: *const i16,
    cbp: *const i16,
    crp: *const i16,
    outp: *mut u8,
    w: usize,
) {
    use core::arch::aarch64::*;
    // Same i16 arithmetic as ycbcr_neon_raw — all intermediates fit in i16.
    let n_min = vdupq_n_s16(-128);
    let n_max = vdupq_n_s16(127);
    let c128 = vdupq_n_s16(128);
    let alpha = vdup_n_u8(255);

    let full8 = w / 8;
    for i in 0..full8 {
        let off = i * 8;
        let c_off = i * 4;
        // Load + normalize Y (8 consecutive)
        let yc = vmaxq_s16(
            vminq_s16(vrshrq_n_s16::<6>(vld1q_s16(yp.add(off))), n_max),
            n_min,
        );
        // Load 4 chroma values, normalize at i16 level, then upsample 4→8 by
        // duplicating each value: [a,b,c,d] → [a,a,b,b,c,c,d,d] via vzip1q
        let cb4 = vmaxq_s16(
            vminq_s16(
                vrshrq_n_s16::<6>(vcombine_s16(vld1_s16(cbp.add(c_off)), vdup_n_s16(0))),
                n_max,
            ),
            n_min,
        );
        let cr4 = vmaxq_s16(
            vminq_s16(
                vrshrq_n_s16::<6>(vcombine_s16(vld1_s16(crp.add(c_off)), vdup_n_s16(0))),
                n_max,
            ),
            n_min,
        );
        // Upsample: interleave low 4 lanes with themselves → [a,a,b,b,c,c,d,d]
        let cbc = vzip1q_s16(cb4, cb4);
        let crc = vzip1q_s16(cr4, cr4);
        // All arithmetic at i16 level (same ranges as non-half path after upsample)
        let y128 = vaddq_s16(yc, c128);
        let t2 = vaddq_s16(crc, vshrq_n_s16::<1>(crc));
        let t3 = vsubq_s16(y128, vshrq_n_s16::<2>(cbc));
        let r16 = vaddq_s16(y128, t2);
        let g16 = vsubq_s16(t3, vshrq_n_s16::<1>(t2));
        let b16 = vaddq_s16(t3, vshlq_n_s16::<1>(cbc));
        let r8 = vqmovun_s16(r16);
        let g8 = vqmovun_s16(g16);
        let b8 = vqmovun_s16(b16);
        vst4_u8(outp.add(off * 4), uint8x8x4_t(r8, g8, b8, alpha));
    }
    // Scalar tail
    for col in (full8 * 8)..w {
        let y = normalize(*yp.add(col));
        let b = normalize(*cbp.add(col / 2));
        let r = normalize(*crp.add(col / 2));
        let t2 = r + (r >> 1);
        let t3 = y + 128 - (b >> 2);
        *outp.add(col * 4) = (y + 128 + t2).clamp(0, 255) as u8;
        *outp.add(col * 4 + 1) = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
        *outp.add(col * 4 + 2) = (t3 + (b << 1)).clamp(0, 255) as u8;
        *outp.add(col * 4 + 3) = 255;
    }
}

/// x86_64 AVX2 fused normalize + YCbCr→RGBA from raw i16 plane data (non-chroma-half).
///
/// 16 pixels per iteration (vs NEON's 8): __m256i holds 16 i16. Pack-down to u8
/// is done via SSE `_mm_packus_epi16` on the two 128-bit halves followed by an
/// SSE byte-interleave to materialise R/G/B/A → RGBA bytes.
///
/// `cbp` and `crp` must point to `w` values each (same stride as `yp`).
#[cfg(target_arch = "x86_64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "avx2")]
unsafe fn ycbcr_avx2_raw(
    yp: *const i16,
    cbp: *const i16,
    crp: *const i16,
    outp: *mut u8,
    w: usize,
) {
    use core::arch::x86_64::*;
    let n_min = _mm256_set1_epi16(-128);
    let n_max = _mm256_set1_epi16(127);
    let c128 = _mm256_set1_epi16(128);
    let one = _mm256_set1_epi16(1);

    let full16 = w / 16;
    for i in 0..full16 {
        let off = i * 16;
        // Rounding right shift by 6 + clamp to [-128, 127].
        // Equivalent to scalar `((v as i32 + 32) >> 6).clamp(-128, 127)` and to NEON
        // `vrshrq_n_s16::<6>` followed by clamp.  We compute it at i16 width without
        // overflow as `(v >> 6) + ((v as u16 >> 5) & 1)` — the bit-5 logical-shifted
        // term is the round-half-away-from-zero correction and matches the wider
        // intermediate that NEON / scalar use.
        let load_norm_clamp = |p: *const i16| -> __m256i {
            let v = _mm256_loadu_si256(p as *const __m256i);
            let high = _mm256_srai_epi16::<6>(v);
            let bit5 = _mm256_and_si256(_mm256_srli_epi16::<5>(v), one);
            let n = _mm256_add_epi16(high, bit5);
            _mm256_max_epi16(_mm256_min_epi16(n, n_max), n_min)
        };
        let yc = load_norm_clamp(yp.add(off));
        let cbc = load_norm_clamp(cbp.add(off));
        let crc = load_norm_clamp(crp.add(off));

        // Same i16 arithmetic as NEON path; ranges fit in i16 → no widening.
        let y128 = _mm256_add_epi16(yc, c128);
        let t2 = _mm256_add_epi16(crc, _mm256_srai_epi16::<1>(crc));
        let t3 = _mm256_sub_epi16(y128, _mm256_srai_epi16::<2>(cbc));
        let r16 = _mm256_add_epi16(y128, t2);
        let g16 = _mm256_sub_epi16(t3, _mm256_srai_epi16::<1>(t2));
        let b16 = _mm256_add_epi16(t3, _mm256_slli_epi16::<1>(cbc));

        // Saturating narrow signed i16 → unsigned u8 in halves (clamps to [0, 255])
        let r_pack = _mm_packus_epi16(
            _mm256_castsi256_si128(r16),
            _mm256_extracti128_si256::<1>(r16),
        );
        let g_pack = _mm_packus_epi16(
            _mm256_castsi256_si128(g16),
            _mm256_extracti128_si256::<1>(g16),
        );
        let b_pack = _mm_packus_epi16(
            _mm256_castsi256_si128(b16),
            _mm256_extracti128_si256::<1>(b16),
        );
        let a_pack = _mm_set1_epi8(-1i8);

        // Interleave R/G and B/A into pairs, then unpack i16 to materialise RGBA.
        let rg_lo = _mm_unpacklo_epi8(r_pack, g_pack);
        let rg_hi = _mm_unpackhi_epi8(r_pack, g_pack);
        let ba_lo = _mm_unpacklo_epi8(b_pack, a_pack);
        let ba_hi = _mm_unpackhi_epi8(b_pack, a_pack);

        let rgba0 = _mm_unpacklo_epi16(rg_lo, ba_lo);
        let rgba1 = _mm_unpackhi_epi16(rg_lo, ba_lo);
        let rgba2 = _mm_unpacklo_epi16(rg_hi, ba_hi);
        let rgba3 = _mm_unpackhi_epi16(rg_hi, ba_hi);

        let dst = outp.add(off * 4) as *mut __m128i;
        _mm_storeu_si128(dst, rgba0);
        _mm_storeu_si128(dst.add(1), rgba1);
        _mm_storeu_si128(dst.add(2), rgba2);
        _mm_storeu_si128(dst.add(3), rgba3);
    }
    // Scalar tail
    for col in (full16 * 16)..w {
        let y = normalize(*yp.add(col));
        let b = normalize(*cbp.add(col));
        let r = normalize(*crp.add(col));
        let t2 = r + (r >> 1);
        let t3 = y + 128 - (b >> 2);
        *outp.add(col * 4) = (y + 128 + t2).clamp(0, 255) as u8;
        *outp.add(col * 4 + 1) = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
        *outp.add(col * 4 + 2) = (t3 + (b << 1)).clamp(0, 255) as u8;
        *outp.add(col * 4 + 3) = 255;
    }
}

/// x86_64 AVX2 fused normalize + YCbCr→RGBA from raw i16 plane data (chroma-half).
///
/// 16 Y / 8 chroma per iteration. Chroma upsample uses `_mm256_permute4x64_epi64`
/// to place chromas 0-3 in the low 128-bit lane low half and chromas 4-7 in the
/// high 128-bit lane low half, then `_mm256_unpacklo_epi16(v, v)` duplicates each
/// chroma into two adjacent i16 lanes per 128-bit half.
#[cfg(target_arch = "x86_64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "avx2")]
unsafe fn ycbcr_avx2_raw_half(
    yp: *const i16,
    cbp: *const i16,
    crp: *const i16,
    outp: *mut u8,
    w: usize,
) {
    use core::arch::x86_64::*;
    let n_min = _mm256_set1_epi16(-128);
    let n_max = _mm256_set1_epi16(127);
    let c128 = _mm256_set1_epi16(128);
    let one = _mm256_set1_epi16(1);

    // Overflow-safe rounding right shift by 6 + clamp to [-128, 127];
    // see `ycbcr_avx2_raw` for the equivalence proof.
    let norm_clamp = |v: __m256i| -> __m256i {
        let high = _mm256_srai_epi16::<6>(v);
        let bit5 = _mm256_and_si256(_mm256_srli_epi16::<5>(v), one);
        let n = _mm256_add_epi16(high, bit5);
        _mm256_max_epi16(_mm256_min_epi16(n, n_max), n_min)
    };

    let full16 = w / 16;
    for i in 0..full16 {
        let off = i * 16;
        let c_off = i * 8;

        // Load + normalize 16 Y samples
        let yv = _mm256_loadu_si256(yp.add(off) as *const __m256i);
        let yc = norm_clamp(yv);

        // Load 8 chroma i16 (one __m128i), upsample to 16 by duplicating each.
        let upsample = |p: *const i16| -> __m256i {
            let v8 = _mm_loadu_si128(p as *const __m128i);
            // Place i16s 0-3 into i64-lane 0 (already there), i16s 4-7 into i64-lane 2.
            // permute4x64 mask 0b00_01_00_00: out0←src0, out1←src0, out2←src1, out3←src0.
            let spread = _mm256_permute4x64_epi64::<0b00_01_00_00>(_mm256_castsi128_si256(v8));
            // Per-128-bit-lane interleave with itself: duplicates each i16 lane.
            _mm256_unpacklo_epi16(spread, spread)
        };
        let cbc = norm_clamp(upsample(cbp.add(c_off)));
        let crc = norm_clamp(upsample(crp.add(c_off)));

        let y128 = _mm256_add_epi16(yc, c128);
        let t2 = _mm256_add_epi16(crc, _mm256_srai_epi16::<1>(crc));
        let t3 = _mm256_sub_epi16(y128, _mm256_srai_epi16::<2>(cbc));
        let r16 = _mm256_add_epi16(y128, t2);
        let g16 = _mm256_sub_epi16(t3, _mm256_srai_epi16::<1>(t2));
        let b16 = _mm256_add_epi16(t3, _mm256_slli_epi16::<1>(cbc));

        let r_pack = _mm_packus_epi16(
            _mm256_castsi256_si128(r16),
            _mm256_extracti128_si256::<1>(r16),
        );
        let g_pack = _mm_packus_epi16(
            _mm256_castsi256_si128(g16),
            _mm256_extracti128_si256::<1>(g16),
        );
        let b_pack = _mm_packus_epi16(
            _mm256_castsi256_si128(b16),
            _mm256_extracti128_si256::<1>(b16),
        );
        let a_pack = _mm_set1_epi8(-1i8);

        let rg_lo = _mm_unpacklo_epi8(r_pack, g_pack);
        let rg_hi = _mm_unpackhi_epi8(r_pack, g_pack);
        let ba_lo = _mm_unpacklo_epi8(b_pack, a_pack);
        let ba_hi = _mm_unpackhi_epi8(b_pack, a_pack);

        let rgba0 = _mm_unpacklo_epi16(rg_lo, ba_lo);
        let rgba1 = _mm_unpackhi_epi16(rg_lo, ba_lo);
        let rgba2 = _mm_unpacklo_epi16(rg_hi, ba_hi);
        let rgba3 = _mm_unpackhi_epi16(rg_hi, ba_hi);

        let dst = outp.add(off * 4) as *mut __m128i;
        _mm_storeu_si128(dst, rgba0);
        _mm_storeu_si128(dst.add(1), rgba1);
        _mm_storeu_si128(dst.add(2), rgba2);
        _mm_storeu_si128(dst.add(3), rgba3);
    }
    // Scalar tail
    for col in (full16 * 16)..w {
        let y = normalize(*yp.add(col));
        let b = normalize(*cbp.add(col / 2));
        let r = normalize(*crp.add(col / 2));
        let t2 = r + (r >> 1);
        let t3 = y + 128 - (b >> 2);
        *outp.add(col * 4) = (y + 128 + t2).clamp(0, 255) as u8;
        *outp.add(col * 4 + 1) = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
        *outp.add(col * 4 + 2) = (t3 + (b << 1)).clamp(0, 255) as u8;
        *outp.add(col * 4 + 3) = 255;
    }
}

/// AArch64 NEON: 6× vld1q_s32 + SIMD arithmetic + vst4_u8 per 8 pixels.
/// Replaces 80+ bounds-check branches per 8 pixels in the LLVM-generated portable code.
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "neon")]
unsafe fn ycbcr_neon(yp: *const i32, cbp: *const i32, crp: *const i32, outp: *mut u8, w: usize) {
    use core::arch::aarch64::*;
    let c128 = vdupq_n_s32(128);
    let c0 = vdupq_n_s32(0);
    let c255 = vdupq_n_s32(255);
    let alpha = vdup_n_u8(255);

    let full8 = w / 8;
    for i in 0..full8 {
        let off = i * 8;
        // Load 8 × i32 from each channel (2 × vld1q_s32 = one cache line per channel)
        let y_lo = vld1q_s32(yp.add(off));
        let y_hi = vld1q_s32(yp.add(off + 4));
        let cb_lo = vld1q_s32(cbp.add(off));
        let cb_hi = vld1q_s32(cbp.add(off + 4));
        let cr_lo = vld1q_s32(crp.add(off));
        let cr_hi = vld1q_s32(crp.add(off + 4));

        // t2 = cr + (cr >> 1)
        let t2_lo = vaddq_s32(cr_lo, vshrq_n_s32::<1>(cr_lo));
        let t2_hi = vaddq_s32(cr_hi, vshrq_n_s32::<1>(cr_hi));
        // t3 = y + 128 - (cb >> 2)
        let t3_lo = vsubq_s32(vaddq_s32(y_lo, c128), vshrq_n_s32::<2>(cb_lo));
        let t3_hi = vsubq_s32(vaddq_s32(y_hi, c128), vshrq_n_s32::<2>(cb_hi));

        // red = clamp(y + 128 + t2)
        let r_lo = vminq_s32(vmaxq_s32(vaddq_s32(vaddq_s32(y_lo, c128), t2_lo), c0), c255);
        let r_hi = vminq_s32(vmaxq_s32(vaddq_s32(vaddq_s32(y_hi, c128), t2_hi), c0), c255);
        // green = clamp(t3 - (t2 >> 1))
        let g_lo = vminq_s32(
            vmaxq_s32(vsubq_s32(t3_lo, vshrq_n_s32::<1>(t2_lo)), c0),
            c255,
        );
        let g_hi = vminq_s32(
            vmaxq_s32(vsubq_s32(t3_hi, vshrq_n_s32::<1>(t2_hi)), c0),
            c255,
        );
        // blue = clamp(t3 + (cb << 1))
        let b_lo = vminq_s32(
            vmaxq_s32(vaddq_s32(t3_lo, vshlq_n_s32::<1>(cb_lo)), c0),
            c255,
        );
        let b_hi = vminq_s32(
            vmaxq_s32(vaddq_s32(t3_hi, vshlq_n_s32::<1>(cb_hi)), c0),
            c255,
        );

        // Narrow i32×4 → i16×4 → u8×8 for each channel
        let r8 = vqmovun_s16(vcombine_s16(vmovn_s32(r_lo), vmovn_s32(r_hi)));
        let g8 = vqmovun_s16(vcombine_s16(vmovn_s32(g_lo), vmovn_s32(g_hi)));
        let b8 = vqmovun_s16(vcombine_s16(vmovn_s32(b_lo), vmovn_s32(b_hi)));

        // Store 8 RGBA pixels (32 bytes) interleaved via vst4_u8
        vst4_u8(outp.add(off * 4), uint8x8x4_t(r8, g8, b8, alpha));
    }

    // Scalar tail
    for col in (full8 * 8)..w {
        let y = *yp.add(col);
        let b = *cbp.add(col);
        let r = *crp.add(col);
        let t2 = r + (r >> 1);
        let t3 = y + 128 - (b >> 2);
        *outp.add(col * 4) = (y + 128 + t2).clamp(0, 255) as u8;
        *outp.add(col * 4 + 1) = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
        *outp.add(col * 4 + 2) = (t3 + (b << 1)).clamp(0, 255) as u8;
        *outp.add(col * 4 + 3) = 255;
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
                // Skip the inner loop entirely when no ACTIVE coefficients exist
                // (avoids function call + zp register flush for fresh/sparse blocks).
                if (self.bbstate & ACTIVE) != 0 {
                    self.previously_active_coefficient_decoding_pass(zp, block_idx);
                }
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
            #[allow(unsafe_code)]
            for r in 0..block_rows {
                for c in 0..self.block_cols {
                    let block = &self.blocks[r * self.block_cols + c];
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
                                    *block.get_unchecked(i);
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
        for r in 0..block_rows {
            for c in 0..self.block_cols {
                let block = &self.blocks[r * self.block_cols + c];
                let row_base = r << 5;
                let col_base = c << 5;
                for row in 0..32usize {
                    let dst_base = (row_base + row) * full_width + col_base;
                    let inv_base = row * 32;
                    for col in 0..32usize {
                        let i = ZIGZAG_INV[inv_base + col] as usize;
                        plane.data[dst_base + col] = block[i];
                    }
                }
            }
        }

        inverse_wavelet_transform(&mut plane, self.width, self.height, subsample);
        plane
    }
}

// ---- Flat plane helper -------------------------------------------------------

/// Allocate `n` uninitialized `i16` elements.
///
/// Uses `Vec<MaybeUninit<i16>>` (the clippy-blessed pattern) and reinterprets
/// as `Vec<i16>`.
///
/// # Safety
/// Caller must write every element before reading it.
#[allow(unsafe_code)]
unsafe fn uninit_i16_vec(n: usize) -> Vec<i16> {
    use core::mem::MaybeUninit;
    let mut v: Vec<MaybeUninit<i16>> = Vec::with_capacity(n);
    // Safety: MaybeUninit<i16> requires no initialization; len will equal capacity.
    unsafe { v.set_len(n) };
    let mut md = core::mem::ManuallyDrop::new(v);
    // Safety: MaybeUninit<i16> and i16 have identical layout; capacity unchanged.
    unsafe { Vec::from_raw_parts(md.as_mut_ptr().cast::<i16>(), md.len(), md.capacity()) }
}

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

/// Load 8 `i16` values at stride `s` starting at `slice[phys_off]`.
///
/// Reads `slice[phys_off + j*s]` for j = 0..7. For s=1 this is identical to
/// [`load8`]. For s=2 and s=4 the AArch64 path uses `ld2`/`ld4` to deinterleave
/// in a single instruction; other targets use scalar loads that LLVM may
/// auto-vectorize.
#[inline(always)]
fn load8s(slice: &[i16], phys_off: usize, s: usize) -> i32x8 {
    // s=1 fast path: single contiguous load + sign-extend.  Checked FIRST so that
    // the s=1 branch is a single cmp+b (not taken on s≠1) rather than a 5-branch
    // dispatch chain inside load8s_neon.
    if s == 1 {
        #[allow(unsafe_code)]
        return unsafe {
            // SAFETY: caller ensures phys_off+7 < slice.len().
            let arr: [i16; 8] = core::ptr::read(slice.as_ptr().add(phys_off) as *const [i16; 8]);
            i32x8::from([
                arr[0] as i32,
                arr[1] as i32,
                arr[2] as i32,
                arr[3] as i32,
                arr[4] as i32,
                arr[5] as i32,
                arr[6] as i32,
                arr[7] as i32,
            ])
        };
    }
    #[cfg(target_arch = "aarch64")]
    if s == 2 || s == 4 {
        #[allow(unsafe_code)]
        return unsafe { load8s_neon(slice, phys_off, s) };
    }
    i32x8::from([
        slice[phys_off] as i32,
        slice[phys_off + s] as i32,
        slice[phys_off + 2 * s] as i32,
        slice[phys_off + 3 * s] as i32,
        slice[phys_off + 4 * s] as i32,
        slice[phys_off + 5 * s] as i32,
        slice[phys_off + 6 * s] as i32,
        slice[phys_off + 7 * s] as i32,
    ])
}

/// Store 8 `i32x8` values (truncated to `i16`) at stride `s` starting at `slice[phys_off]`.
///
/// Writes `slice[phys_off + j*s] = v[j] as i16` for j = 0..7. Interleaved positions
/// (those not at multiples of `s`) are left unchanged.
#[inline(always)]
fn store8s(slice: &mut [i16], phys_off: usize, s: usize, v: i32x8) {
    // s=1 fast path: narrow and store contiguously.  Same reasoning as load8s.
    if s == 1 {
        #[allow(unsafe_code)]
        return unsafe {
            // SAFETY: caller ensures phys_off+7 < slice.len().
            let a = v.to_array();
            let narrow: [i16; 8] = [
                a[0] as i16,
                a[1] as i16,
                a[2] as i16,
                a[3] as i16,
                a[4] as i16,
                a[5] as i16,
                a[6] as i16,
                a[7] as i16,
            ];
            core::ptr::write(slice.as_mut_ptr().add(phys_off) as *mut [i16; 8], narrow);
        };
    }
    #[cfg(target_arch = "aarch64")]
    if s == 2 || s == 4 {
        #[allow(unsafe_code)]
        return unsafe { store8s_neon(slice, phys_off, s, v) };
    }
    let a = v.to_array();
    for j in 0..8 {
        slice[phys_off + j * s] = a[j] as i16;
    }
}

// ---- AArch64 NEON stride load/store -----------------------------------------
//
// ld2 deinterleaves 16 consecutive i16s into two vectors (even, odd).
// ld4 deinterleaves 32 consecutive i16s into four vectors.
// After widening the target lane to i32, `lifting_even` / `predict_inner`
// run on i32x8 exactly as for s=1.
// On store, we re-interleave the updated even lane with the unchanged odd lanes.

#[cfg(target_arch = "aarch64")]
// s=1 is now handled directly in load8s/store8s (single ldr/str q without dispatch).
// This function only needs to handle s=2 and s=4.
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "neon")]
unsafe fn load8s_neon(slice: &[i16], phys_off: usize, s: usize) -> i32x8 {
    use core::arch::aarch64::*;
    let ptr = slice.as_ptr().add(phys_off);
    let target: int16x8_t = if s == 2 {
        vld2q_s16(ptr).0
    } else {
        // s == 4
        vld4q_s16(ptr).0
    };
    // Widen i16x8 → two i32x4, then reinterpret as [i32;8] → i32x8
    let lo = vmovl_s16(vget_low_s16(target));
    let hi = vmovl_high_s16(target);
    let arr = core::mem::transmute::<[int32x4_t; 2], [i32; 8]>([lo, hi]);
    i32x8::from(arr)
}

#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "neon")]
unsafe fn store8s_neon(slice: &mut [i16], phys_off: usize, s: usize, v: i32x8) {
    use core::arch::aarch64::*;
    let ptr = slice.as_mut_ptr().add(phys_off);
    // Narrow v (i32x8) back to i16x8 via vmovn (truncate low 16 bits)
    let v_arr = core::mem::transmute::<[i32; 8], [int32x4_t; 2]>(v.to_array());
    let new_vals = vcombine_s16(vmovn_s32(v_arr[0]), vmovn_s32(v_arr[1]));
    // For s=2,4: scatter-store 8 i16s to stride-s positions.
    // Using 8 individual str h avoids the extra vld2/vld4 that would be needed
    // to preserve interleaved odd lanes before a vst2/vst4.
    // Each str h targets the same ~16-byte cache region (already hot from load8s).
    let a: [i16; 8] = core::mem::transmute(new_vals);
    for (j, &val) in a.iter().enumerate() {
        *ptr.add(j * s) = val;
    }
}

/// Load 8 contiguous `i32` values from `slice[off..]` into an `i32x8`.
///
/// # Safety
/// Caller must ensure `off + 7 < slice.len()`.
#[inline(always)]
#[allow(unsafe_code)]
fn load8_i32(slice: &[i32], off: usize) -> i32x8 {
    // SAFETY: caller guarantees off+7 is in bounds.
    unsafe {
        i32x8::from([
            *slice.get_unchecked(off),
            *slice.get_unchecked(off + 1),
            *slice.get_unchecked(off + 2),
            *slice.get_unchecked(off + 3),
            *slice.get_unchecked(off + 4),
            *slice.get_unchecked(off + 5),
            *slice.get_unchecked(off + 6),
            *slice.get_unchecked(off + 7),
        ])
    }
}

/// Store 8 values from an `i32x8` into contiguous `i32` slots at `slice[off..]`.
///
/// # Safety
/// Caller must ensure `off + 7 < slice.len()`.
#[inline(always)]
#[allow(unsafe_code)]
fn store8_i32(slice: &mut [i32], off: usize, v: i32x8) {
    let a = v.to_array();
    // SAFETY: caller guarantees off+7 is in bounds.
    unsafe {
        *slice.get_unchecked_mut(off) = a[0];
        *slice.get_unchecked_mut(off + 1) = a[1];
        *slice.get_unchecked_mut(off + 2) = a[2];
        *slice.get_unchecked_mut(off + 3) = a[3];
        *slice.get_unchecked_mut(off + 4) = a[4];
        *slice.get_unchecked_mut(off + 5) = a[5];
        *slice.get_unchecked_mut(off + 6) = a[6];
        *slice.get_unchecked_mut(off + 7) = a[7];
    }
}

/// Gather one `i16` value from each of 8 consecutive rows at column index `k`.
///
/// `offs[i]` is the start offset `row_i * stride` for row `i`.
///
/// # Safety
/// Caller must ensure `offs[i] + k < data.len()` for all `i in 0..8`.
#[inline(always)]
#[allow(unsafe_code)]
fn load_rows8(data: &[i16], offs: &[usize; 8], k: usize) -> i32x8 {
    // SAFETY: caller guarantees offs[i]+k is in bounds for all i.
    unsafe {
        i32x8::from([
            *data.get_unchecked(offs[0] + k) as i32,
            *data.get_unchecked(offs[1] + k) as i32,
            *data.get_unchecked(offs[2] + k) as i32,
            *data.get_unchecked(offs[3] + k) as i32,
            *data.get_unchecked(offs[4] + k) as i32,
            *data.get_unchecked(offs[5] + k) as i32,
            *data.get_unchecked(offs[6] + k) as i32,
            *data.get_unchecked(offs[7] + k) as i32,
        ])
    }
}

/// Scatter one value from `v` to each of 8 consecutive rows at column index `k`.
///
/// # Safety
/// Caller must ensure `offs[i] + k < data.len()` for all `i in 0..8`.
#[inline(always)]
#[allow(unsafe_code)]
fn store_rows8(data: &mut [i16], offs: &[usize; 8], k: usize, v: i32x8) {
    let a = v.to_array();
    // SAFETY: caller guarantees offs[i]+k is in bounds for all i.
    unsafe {
        *data.get_unchecked_mut(offs[0] + k) = a[0] as i16;
        *data.get_unchecked_mut(offs[1] + k) = a[1] as i16;
        *data.get_unchecked_mut(offs[2] + k) = a[2] as i16;
        *data.get_unchecked_mut(offs[3] + k) = a[3] as i16;
        *data.get_unchecked_mut(offs[4] + k) = a[4] as i16;
        *data.get_unchecked_mut(offs[5] + k) = a[5] as i16;
        *data.get_unchecked_mut(offs[6] + k) = a[6] as i16;
        *data.get_unchecked_mut(offs[7] + k) = a[7] as i16;
    }
}

// Compile-time rounding constants — avoids the `memcpy` call that
// `i32x8::splat(N)` generates on AArch64 (LLVM doesn't hoist splat to movi.4s).
// SAFETY: [i32; 8] and i32x8 have identical representations (8 × 4-byte i32,
// 32-byte size); the transmute is value-preserving.
#[allow(unsafe_code)]
const C16: i32x8 = unsafe { core::mem::transmute([16i32; 8]) };
#[allow(unsafe_code)]
const C8: i32x8 = unsafe { core::mem::transmute([8i32; 8]) };
#[allow(unsafe_code)]
const C1: i32x8 = unsafe { core::mem::transmute([1i32; 8]) };

/// Lifting filter: `data[idx] -= ((9*(p1+n1) - (p3+n3) + 16) >> 5)`
#[inline(always)]
fn lifting_even(cur: i32x8, p1: i32x8, n1: i32x8, p3: i32x8, n3: i32x8) -> i32x8 {
    let a = p1 + n1;
    let c = p3 + n3;
    cur - (((a << 3) + a - c + C16) >> 5)
}

/// Prediction filter (inner): `data[idx] += ((9*(p1+n1) - (p3+n3) + 8) >> 4)`
#[inline(always)]
fn predict_inner(cur: i32x8, p1: i32x8, n1: i32x8, p3: i32x8, n3: i32x8) -> i32x8 {
    let a = p1 + n1;
    cur + (((a << 3) + a - (p3 + n3) + C8) >> 4)
}

/// Prediction filter (boundary): `data[idx] += ((p + n + 1) >> 1)`
#[inline(always)]
fn predict_avg(cur: i32x8, p: i32x8, n: i32x8) -> i32x8 {
    cur + ((p + n + C1) >> 1)
}

/// AArch64 NEON horizontal row pass for s=1.
///
/// Processes each row independently using `vld2q_s16` to deinterleave even/odd
/// positions and `vextq_s16` for the 5-tap sliding-window neighbors, eliminating
/// the scatter loads (`8×ldrh`) used by the vertical 8-rows-at-a-time path.
///
/// # Even pass (lifting)
/// For each chunk of 8 even positions (`chunk*16 .. chunk*16+15`):
/// ```text
///   vld2q_s16(chunk*16)     → curr_even[0..8], curr_odd[0..8]
///   vld2q_s16((chunk+1)*16) → next_even (for n3)
///   p1 = vextq_s16(prev_odd, curr_odd, 7)
///   n1 = curr_odd
///   p3 = vextq_s16(prev_odd, curr_odd, 6)
///   n3 = vextq_s16(curr_odd, next_odd, 1)
/// ```
///
/// # Odd pass (prediction)
/// For each chunk of 8 inner odd positions at `3+chunk*16, 5+..., 17+chunk*16`:
/// ```text
///   pair1 = vld2q_s16(chunk*16)     → p3=.0, odds_lo=.1
///   pair2 = vld2q_s16((chunk+1)*16) → next_even=.0, odds_hi=.1
///   curr_odds = vextq_s16(odds_lo, odds_hi, 1)
///   p1 = vextq_s16(p3, next_even, 1)
///   n1 = vextq_s16(p3, next_even, 2)
///   n3 = vextq_s16(p3, next_even, 3)
/// ```
///
/// # Safety
/// `data[row_off .. row_off+width]` must be valid. `width >= 1`.
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "neon")]
unsafe fn row_pass_neon_s1_row(data: &mut [i16], row_off: usize, width: usize) {
    use core::arch::aarch64::*;

    let kmax = width - 1;
    let border = kmax.saturating_sub(3);
    let ptr = data.as_mut_ptr().add(row_off);

    // Number of NEON even chunks: need next chunk fully in bounds for n3.
    // Condition: (chunk+1)*16+15 < width  →  chunk < (width-31)/16.
    let even_chunks = if width >= 32 { (width - 31) / 16 } else { 0 };

    // ── Even pass (lifting) ────────────────────────────────────────────────────

    let mut prev_odd = vdupq_n_s16(0i16);

    for chunk in 0..even_chunks {
        let curr_pair = vld2q_s16(ptr.add(chunk * 16) as *const i16);
        let next_pair = vld2q_s16(ptr.add((chunk + 1) * 16) as *const i16);
        let curr_even = curr_pair.0;
        let curr_odd = curr_pair.1;
        let next_odd = next_pair.1;

        let p1 = vextq_s16::<7>(prev_odd, curr_odd);
        let n1 = curr_odd;
        let p3 = vextq_s16::<6>(prev_odd, curr_odd);
        let n3 = vextq_s16::<1>(curr_odd, next_odd);

        // cur -= ((9*(p1+n1) - (p3+n3) + 16) >> 5)
        macro_rules! lift {
            ($ce:expr, $p1:expr, $n1:expr, $p3:expr, $n3:expr) => {{
                let a = vaddq_s32($p1, $n1);
                let c = vaddq_s32($p3, $n3);
                let nine_a = vaddq_s32(vshlq_n_s32::<3>(a), a);
                let delta = vshrq_n_s32::<5>(vsubq_s32(vaddq_s32(nine_a, vdupq_n_s32(16i32)), c));
                vsubq_s32($ce, delta)
            }};
        }

        let new_lo = lift!(
            vmovl_s16(vget_low_s16(curr_even)),
            vmovl_s16(vget_low_s16(p1)),
            vmovl_s16(vget_low_s16(n1)),
            vmovl_s16(vget_low_s16(p3)),
            vmovl_s16(vget_low_s16(n3))
        );
        let new_hi = lift!(
            vmovl_high_s16(curr_even),
            vmovl_high_s16(p1),
            vmovl_high_s16(n1),
            vmovl_high_s16(p3),
            vmovl_high_s16(n3)
        );
        let new_evens = vcombine_s16(vmovn_s32(new_lo), vmovn_s32(new_hi));

        vst2q_s16(ptr.add(chunk * 16), int16x8x2_t(new_evens, curr_odd));

        prev_odd = curr_odd;
    }

    // Scalar even tail: k = even_chunks*16, +2, ... <= kmax.
    // State just before the first advance: prev1=prev_odd[6], next1=prev_odd[7], next3=data[k+1].
    {
        let k_start = even_chunks * 16;
        let mut prev1 = if even_chunks > 0 {
            vgetq_lane_s16::<6>(prev_odd) as i32
        } else {
            0
        };
        let mut next1 = if even_chunks > 0 {
            vgetq_lane_s16::<7>(prev_odd) as i32
        } else {
            0
        };
        let mut next3 = if k_start < kmax {
            *data.get_unchecked(row_off + k_start + 1) as i32
        } else {
            0
        };
        let mut k = k_start;
        while k <= kmax {
            let prev3 = prev1;
            prev1 = next1;
            next1 = next3;
            next3 = if k + 3 <= kmax {
                *data.get_unchecked(row_off + k + 3) as i32
            } else {
                0
            };
            let a = prev1 + next1;
            let c = prev3 + next3;
            let idx = row_off + k;
            *data.get_unchecked_mut(idx) =
                (*data.get_unchecked(idx) as i32 - (((a << 3) + a - c + 16) >> 5)) as i16;
            k += 2;
        }
    }

    // ── Odd pass (prediction) ──────────────────────────────────────────────────

    if kmax < 1 {
        return;
    }

    // k=1: always predict_avg (or +=prev if k==kmax)
    {
        let p1 = *data.get_unchecked(row_off) as i32;
        let idx1 = row_off + 1;
        if 1 < kmax {
            let n1 = *data.get_unchecked(row_off + 2) as i32;
            *data.get_unchecked_mut(idx1) =
                (*data.get_unchecked(idx1) as i32 + ((p1 + n1 + 1) >> 1)) as i16;
        } else {
            *data.get_unchecked_mut(idx1) = (*data.get_unchecked(idx1) as i32 + p1) as i16;
        }
    }

    // NEON inner odd chunks: predict_inner for k=3,5,...,17+chunk*16.
    // Safety: need (chunk+1)*16+15 < width AND 17+chunk*16 <= border (= kmax-3).
    // Combined: chunk < (width-31)/16 (same as even_chunks).
    // Inner check: 17+chunk*16 <= kmax-3  →  chunk <= (kmax-20)/16.
    let odd_chunks = if kmax >= 20 {
        even_chunks.min((kmax - 20) / 16 + 1)
    } else {
        0
    };

    for chunk in 0..odd_chunks {
        // pair1: evens[chunk*8..+7] in .0, odds[chunk*8..+7] in .1
        let pair1 = vld2q_s16(ptr.add(chunk * 16) as *const i16);
        // pair2: evens[(chunk+1)*8..+7] in .0, odds[(chunk+1)*8..+7] in .1
        let pair2 = vld2q_s16(ptr.add((chunk + 1) * 16) as *const i16);

        // 8 inner odds at physical positions 3+chunk*16, 5+..., 17+chunk*16
        let curr_odds = vextq_s16::<1>(pair1.1, pair2.1);

        // Even neighbors for predict_inner:
        // p3[i] = even at k_odd-3 = chunk*16+2i → pair1.0[i]
        // p1[i] = even at k_odd-1 = chunk*16+2i+2 → vextq(pair1.0, pair2.0, 1)[i]
        // n1[i] = even at k_odd+1 = chunk*16+2i+4 → vextq(pair1.0, pair2.0, 2)[i]
        // n3[i] = even at k_odd+3 = chunk*16+2i+6 → vextq(pair1.0, pair2.0, 3)[i]
        let p3_e = pair1.0;
        let p1_e = vextq_s16::<1>(pair1.0, pair2.0); // also = unchanged evens for store
        let n1_e = vextq_s16::<2>(pair1.0, pair2.0);
        let n3_e = vextq_s16::<3>(pair1.0, pair2.0);

        // cur += ((9*(p1+n1) - (p3+n3) + 8) >> 4)
        macro_rules! predict {
            ($co:expr, $p1:expr, $n1:expr, $p3:expr, $n3:expr) => {{
                let a = vaddq_s32($p1, $n1);
                let c = vaddq_s32($p3, $n3);
                let nine_a = vaddq_s32(vshlq_n_s32::<3>(a), a);
                let delta = vshrq_n_s32::<4>(vsubq_s32(vaddq_s32(nine_a, vdupq_n_s32(8i32)), c));
                vaddq_s32($co, delta)
            }};
        }

        let new_lo = predict!(
            vmovl_s16(vget_low_s16(curr_odds)),
            vmovl_s16(vget_low_s16(p1_e)),
            vmovl_s16(vget_low_s16(n1_e)),
            vmovl_s16(vget_low_s16(p3_e)),
            vmovl_s16(vget_low_s16(n3_e))
        );
        let new_hi = predict!(
            vmovl_high_s16(curr_odds),
            vmovl_high_s16(p1_e),
            vmovl_high_s16(n1_e),
            vmovl_high_s16(p3_e),
            vmovl_high_s16(n3_e)
        );
        let new_odds = vcombine_s16(vmovn_s32(new_lo), vmovn_s32(new_hi));

        // Store: evens at chunk*16+2,+4,...,+16 unchanged (= p1_e), odds updated.
        vst2q_s16(ptr.add(chunk * 16 + 2), int16x8x2_t(p1_e, new_odds));
    }

    // Scalar odd tail: k = 3+odd_chunks*16, ..., kmax (inner then boundary).
    // State before the advance at k_scalar: prev1=data[k-3], next1=data[k-1], next3=data[k+1].
    if kmax >= 3 {
        let k_scalar = 3 + odd_chunks * 16;
        let mut prev1 = *data.get_unchecked(row_off + k_scalar - 3) as i32;
        let mut next1 = *data.get_unchecked(row_off + k_scalar - 1) as i32;
        let mut next3 = if k_scalar < kmax {
            *data.get_unchecked(row_off + k_scalar + 1) as i32
        } else {
            0
        };
        let mut k = k_scalar;
        while k <= kmax {
            let prev3 = prev1;
            prev1 = next1;
            next1 = next3;
            next3 = if k + 3 <= kmax {
                *data.get_unchecked(row_off + k + 3) as i32
            } else {
                0
            };
            let idx = row_off + k;
            if k <= border {
                let a = prev1 + next1;
                let c = prev3 + next3;
                *data.get_unchecked_mut(idx) =
                    (*data.get_unchecked(idx) as i32 + (((a << 3) + a - c + 8) >> 4)) as i16;
            } else if k < kmax {
                *data.get_unchecked_mut(idx) =
                    (*data.get_unchecked(idx) as i32 + ((prev1 + next1 + 1) >> 1)) as i16;
            } else {
                *data.get_unchecked_mut(idx) = (*data.get_unchecked(idx) as i32 + prev1) as i16;
            }
            k += 2;
        }
    }
}

/// Apply the row-direction wavelet pass for one resolution level.
///
/// When `use_simd` is `true` and `s == 1` (`sd == 0`), on AArch64 the
/// horizontal NEON path (`row_pass_neon_s1_row`) is used for each row,
/// processing 8 even/odd positions at a time with `vld2q_s16` instead of
/// scatter loads. For `s > 1` and non-AArch64, the vertical 8-rows-at-a-time
/// `i32x8` path is used. The remaining rows (and all rows when `use_simd` is
/// false) use the scalar path.
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
    // AArch64 horizontal NEON path: at s=1, process each row using vld2q_s16
    // (sequential deinterleave) instead of scatter loads across 8 rows.
    #[cfg(target_arch = "aarch64")]
    if use_simd && s == 1 {
        for row in (0..height).step_by(s) {
            #[allow(unsafe_code)]
            unsafe {
                row_pass_neon_s1_row(data, row * stride, width);
            }
        }
        return;
    }

    let kmax = (width - 1) >> sd;
    let border = kmax.saturating_sub(3);

    // ── SIMD path: 8 active rows at a time ───────────────────────────────────
    //
    // At s=1 the 8 rows are consecutive (o[i] = (row_base + i) * stride).
    // At s=2 they are spaced by 2  (o[i] = (row_base + i*2) * stride), etc.
    // Column accesses use `k << sd` so the logical k loop is unchanged.
    let simd_active = if use_simd { height / s / 8 * 8 } else { 0 };
    let simd_rows = simd_active * s;

    for group in 0..simd_active / 8 {
        let row_base = group * 8 * s;
        let o: [usize; 8] = core::array::from_fn(|i| (row_base + i * s) * stride);

        // — Lifting (even k) ——————————————————————————————————————————————————
        let mut prev1v = i32x8::splat(0);
        let mut next1v = i32x8::splat(0);
        let mut next3v = if kmax >= 1 {
            load_rows8(data, &o, 1 << sd)
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
                load_rows8(data, &o, (k + 3) << sd)
            } else {
                i32x8::splat(0)
            };
            let cur = load_rows8(data, &o, k << sd);
            store_rows8(
                data,
                &o,
                k << sd,
                lifting_even(cur, prev1v, next1v, prev3v, next3v),
            );
            k += 2;
        }

        // — Prediction (odd k) ————————————————————————————————————————————————
        if kmax >= 1 {
            let mut k = 1usize;
            prev1v = load_rows8(data, &o, (k - 1) << sd);
            if k < kmax {
                next1v = load_rows8(data, &o, (k + 1) << sd);
                let cur = load_rows8(data, &o, k << sd);
                store_rows8(data, &o, k << sd, predict_avg(cur, prev1v, next1v));
            } else {
                // k == kmax: boundary — only one odd sample, += prev
                let cur = load_rows8(data, &o, k << sd);
                store_rows8(data, &o, k << sd, cur + prev1v);
                next1v = i32x8::splat(0);
            }

            next3v = if border >= 3 {
                load_rows8(data, &o, (k + 3) << sd)
            } else {
                i32x8::splat(0)
            };

            k = 3;
            while k <= border {
                prev3v = prev1v;
                prev1v = next1v;
                next1v = next3v;
                next3v = load_rows8(data, &o, (k + 3) << sd);
                let cur = load_rows8(data, &o, k << sd);
                store_rows8(
                    data,
                    &o,
                    k << sd,
                    predict_inner(cur, prev1v, next1v, prev3v, next3v),
                );
                k += 2;
            }

            while k <= kmax {
                prev1v = next1v;
                next1v = next3v;
                next3v = i32x8::splat(0);
                let cur = load_rows8(data, &o, k << sd);
                if k < kmax {
                    store_rows8(data, &o, k << sd, predict_avg(cur, prev1v, next1v));
                } else {
                    store_rows8(data, &o, k << sd, cur + prev1v);
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

        // Column pass SIMD: enabled for s=1,2,4 using stride-aware load8s/store8s.
        // For s=2 the load uses vld2q_s16 (deinterleave even/odd), for s=4 vld4q_s16.
        // The scalar else-branches below are now only reached for s>4 (s=8, s=16).
        let use_simd = s <= 4;

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
                        store8_i32(&mut st2, ci, load8s(data, off + ci * s, s));
                    }
                    for ci in simd_cols..num_cols {
                        st2[ci] = data[off + ci * s] as i32;
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

            // Split even pass into: main (k+3 <= kmax → n3 always in-bounds) and
            // tail (k+3 > kmax → n3 = 0), mirroring the odd pass structure.
            // This hoists the `has_n3` branch out of the ci inner loop so that
            // the hot path (≥97% of k-iterations) has no runtime conditional.
            let mut k = 0usize;
            // Main: n3 always available
            while k + 3 <= kmax {
                let k_off = (k << sd) * stride;
                let n3_off = ((k + 3) << sd) * stride;
                if use_simd {
                    let mut ci = 0usize;
                    while ci < simd_cols {
                        let vp3 = load8_i32(&st0, ci);
                        let vp1 = load8_i32(&st1, ci);
                        let vn1 = load8_i32(&st2, ci);
                        let vn3 = load8s(data, n3_off + ci * s, s);
                        let cur = load8s(data, k_off + ci * s, s);
                        store8s(
                            data,
                            k_off + ci * s,
                            s,
                            lifting_even(cur, vp1, vn1, vp3, vn3),
                        );
                        store8_i32(&mut st0, ci, vp1);
                        store8_i32(&mut st1, ci, vn1);
                        store8_i32(&mut st2, ci, vn3);
                        ci += 8;
                    }
                    while ci < num_cols {
                        let p3 = st0[ci];
                        let p1 = st1[ci];
                        let n1 = st2[ci];
                        let n3 = data[n3_off + ci * s] as i32;
                        let a = p1 + n1;
                        let idx = k_off + ci * s;
                        data[idx] =
                            (data[idx] as i32 - (((a << 3) + a - (p3 + n3) + 16) >> 5)) as i16;
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
            // Tail: k+3 > kmax → n3 = 0
            while k <= kmax {
                let k_off = (k << sd) * stride;
                if use_simd {
                    let zero8 = i32x8::splat(0);
                    let mut ci = 0usize;
                    while ci < simd_cols {
                        let vp3 = load8_i32(&st0, ci);
                        let vp1 = load8_i32(&st1, ci);
                        let vn1 = load8_i32(&st2, ci);
                        let cur = load8s(data, k_off + ci * s, s);
                        store8s(
                            data,
                            k_off + ci * s,
                            s,
                            lifting_even(cur, vp1, vn1, vp3, zero8),
                        );
                        store8_i32(&mut st0, ci, vp1);
                        store8_i32(&mut st1, ci, vn1);
                        store8_i32(&mut st2, ci, zero8);
                        ci += 8;
                    }
                    while ci < num_cols {
                        let p3 = st0[ci];
                        let p1 = st1[ci];
                        let n1 = st2[ci];
                        let a = p1 + n1;
                        let idx = k_off + ci * s;
                        data[idx] = (data[idx] as i32 - (((a << 3) + a - p3 + 16) >> 5)) as i16;
                        st0[ci] = p1;
                        st1[ci] = n1;
                        st2[ci] = 0;
                        ci += 1;
                    }
                } else {
                    for (ci, col) in (0..width).step_by(s).enumerate() {
                        let p3 = st0[ci];
                        let p1 = st1[ci];
                        let n1 = st2[ci];
                        let a = p1 + n1;
                        let idx = k_off + col;
                        data[idx] = (data[idx] as i32 - (((a << 3) + a - p3 + 16) >> 5)) as i16;
                        st0[ci] = p1;
                        st1[ci] = n1;
                        st2[ci] = 0;
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
                            let vp = load8s(data, km1_off + ci * s, s);
                            let vn = load8s(data, kp1_off + ci * s, s);
                            let cur = load8s(data, k_off + ci * s, s);
                            store8s(data, k_off + ci * s, s, predict_avg(cur, vp, vn));
                            store8_i32(&mut st0, ci, vp);
                            store8_i32(&mut st1, ci, vn);
                            ci += 8;
                        }
                        while ci < num_cols {
                            let p = data[km1_off + ci * s] as i32;
                            let n = data[kp1_off + ci * s] as i32;
                            let idx = k_off + ci * s;
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
                        let vp = load8s(data, km1_off + ci * s, s);
                        let cur = load8s(data, k_off + ci * s, s);
                        store8s(data, k_off + ci * s, s, cur + vp);
                        store8_i32(&mut st0, ci, vp);
                        ci += 8;
                    }
                    for v in &mut st1[..num_cols] {
                        *v = 0;
                    }
                    while ci < num_cols {
                        let p = data[km1_off + ci * s] as i32;
                        let idx = k_off + ci * s;
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
                            store8_i32(&mut st2, ci, load8s(data, off + ci * s, s));
                            ci += 8;
                        }
                        while ci < num_cols {
                            st2[ci] = data[off + ci * s] as i32;
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
                            let vn3 = load8s(data, n3_off + ci * s, s);
                            let cur = load8s(data, k_off + ci * s, s);
                            store8s(
                                data,
                                k_off + ci * s,
                                s,
                                predict_inner(cur, vp1, vn1, vp3, vn3),
                            );
                            store8_i32(&mut st0, ci, vp1);
                            store8_i32(&mut st1, ci, vn1);
                            store8_i32(&mut st2, ci, vn3);
                            ci += 8;
                        }
                        while ci < num_cols {
                            let p3 = st0[ci];
                            let p1 = st1[ci];
                            let n1 = st2[ci];
                            let n3 = data[n3_off + ci * s] as i32;
                            let a = p1 + n1;
                            let idx = k_off + ci * s;
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
                                let cur = load8s(data, k_off + ci * s, s);
                                store8s(data, k_off + ci * s, s, predict_avg(cur, vp, vn));
                                store8_i32(&mut st1, ci, vn);
                                store8_i32(&mut st2, ci, i32x8::splat(0));
                                ci += 8;
                            }
                            while ci < num_cols {
                                let p = st1[ci];
                                let n = st2[ci];
                                let idx = k_off + ci * s;
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
                            let cur = load8s(data, k_off + ci * s, s);
                            store8s(data, k_off + ci * s, s, cur + vp);
                            store8_i32(&mut st1, ci, load8_i32(&st2, ci));
                            store8_i32(&mut st2, ci, i32x8::splat(0));
                            ci += 8;
                        }
                        while ci < num_cols {
                            let p = st1[ci];
                            let idx = k_off + ci * s;
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
        // Row pass SIMD works for any s — always enable it.
        row_pass_inner(data, width, height, stride, s, sd, true);

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
                            if chroma_half {
                                let c_row = row / 2;
                                let cb_off = c_row * cb_plane.stride;
                                let cr_off = c_row * cr_plane.stride;
                                ycbcr_row_from_i16_half(
                                    &y_plane.data[y_off..y_off + pw],
                                    &cb_plane.data[cb_off..],
                                    &cr_plane.data[cr_off..],
                                    row_data,
                                    pw,
                                );
                            } else {
                                let c_off = row * cb_plane.stride;
                                ycbcr_row_from_i16(
                                    &y_plane.data[y_off..y_off + pw],
                                    &cb_plane.data[c_off..c_off + pw],
                                    &cr_plane.data[c_off..c_off + pw],
                                    row_data,
                                );
                            }
                        });
                }
                #[cfg(not(feature = "parallel"))]
                {
                    for row in 0..ph {
                        let out_row = ph - 1 - row; // DjVu rows are bottom-to-top
                        let y_off = row * y_plane.stride;
                        let row_start = out_row * pw * 4;

                        if self.chroma_half {
                            let c_row = row / 2;
                            let cb_off = c_row * cb_plane.stride;
                            let cr_off = c_row * cr_plane.stride;
                            ycbcr_row_from_i16_half(
                                &y_plane.data[y_off..y_off + pw],
                                &cb_plane.data[cb_off..],
                                &cr_plane.data[cr_off..],
                                &mut pm.data[row_start..row_start + pw * 4],
                                pw,
                            );
                        } else {
                            let c_off = row * cb_plane.stride;
                            ycbcr_row_from_i16(
                                &y_plane.data[y_off..y_off + pw],
                                &cb_plane.data[c_off..c_off + pw],
                                &cr_plane.data[c_off..c_off + pw],
                                &mut pm.data[row_start..row_start + pw * 4],
                            );
                        }
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
                for row in 0..ph {
                    let out_row = ph - 1 - row; // DjVu rows are bottom-to-top
                    let y_off = row * y_plane.stride;
                    let c_off = row * cb_plane.stride;
                    let row_start = out_row * pw * 4;
                    ycbcr_row_from_i16(
                        &y_plane.data[y_off..y_off + pw],
                        &cb_plane.data[c_off..c_off + pw],
                        &cr_plane.data[c_off..c_off + pw],
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

    /// Same as `simd_row_pass_matches_scalar` but for s=2 (sd=1).
    ///
    /// Active rows are every other row; active columns are every other column.
    /// The generalised SIMD path (8 active rows at a time with stride s) must
    /// produce the same result as the pure scalar path.
    #[test]
    fn simd_row_pass_s2_matches_scalar() {
        let width = 64usize;
        let height = 32usize;
        let stride = width;
        let n = stride * height;
        let s = 2usize;
        let sd = 1usize;

        let initial: Vec<i16> = (0..n).map(|i| ((i * 7 + 13) % 511) as i16 - 255).collect();

        let mut scalar_data = initial.clone();
        super::row_pass_inner(&mut scalar_data, width, height, stride, s, sd, false);

        let mut simd_data = initial.clone();
        super::row_pass_inner(&mut simd_data, width, height, stride, s, sd, true);

        assert_eq!(
            scalar_data, simd_data,
            "SIMD row pass (s=2) must produce identical output to scalar"
        );
    }

    /// Reference scalar implementation of the fused-normalize YCbCr→RGBA path.
    /// Mirrors `ycbcr_neon_raw` byte-for-byte (same formula, same clamps).
    #[cfg(target_arch = "x86_64")]
    fn ycbcr_raw_scalar(y: &[i16], cb: &[i16], cr: &[i16], out: &mut [u8]) {
        let w = y.len();
        for col in 0..w {
            let yn = super::normalize(y[col]);
            let bn = super::normalize(cb[col]);
            let rn = super::normalize(cr[col]);
            let t2 = rn + (rn >> 1);
            let t3 = yn + 128 - (bn >> 2);
            out[col * 4] = (yn + 128 + t2).clamp(0, 255) as u8;
            out[col * 4 + 1] = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
            out[col * 4 + 2] = (t3 + (bn << 1)).clamp(0, 255) as u8;
            out[col * 4 + 3] = 255;
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn ycbcr_raw_half_scalar(y: &[i16], cb: &[i16], cr: &[i16], out: &mut [u8]) {
        let w = y.len();
        for col in 0..w {
            let yn = super::normalize(y[col]);
            let bn = super::normalize(cb[col / 2]);
            let rn = super::normalize(cr[col / 2]);
            let t2 = rn + (rn >> 1);
            let t3 = yn + 128 - (bn >> 2);
            out[col * 4] = (yn + 128 + t2).clamp(0, 255) as u8;
            out[col * 4 + 1] = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
            out[col * 4 + 2] = (t3 + (bn << 1)).clamp(0, 255) as u8;
            out[col * 4 + 3] = 255;
        }
    }

    /// AVX2 fused-normalize YCbCr→RGBA must agree byte-for-byte with the scalar
    /// reference across the full i16 input range and all width residues mod 16
    /// (covers main loop + scalar tail).
    #[cfg(target_arch = "x86_64")]
    #[test]
    fn ycbcr_avx2_raw_matches_scalar() {
        if !std::is_x86_feature_detected!("avx2") {
            eprintln!("skipping: AVX2 not available on this host");
            return;
        }
        // Range chosen to exercise normalize + clamp + every arithmetic branch.
        let raw_vals: [i16; 8] = [-32768, -8192, -64, -1, 0, 63, 8191, 32767];
        for &width in &[1usize, 7, 16, 17, 31, 32, 33, 47, 48, 64, 100] {
            let n = width;
            let make_seq = |seed: usize| -> Vec<i16> {
                (0..n)
                    .map(|i| raw_vals[(i + seed) % raw_vals.len()])
                    .collect()
            };
            let y = make_seq(0);
            let cb = make_seq(3);
            let cr = make_seq(5);

            let mut got = vec![0u8; n * 4];
            #[allow(unsafe_code)]
            unsafe {
                super::ycbcr_avx2_raw(y.as_ptr(), cb.as_ptr(), cr.as_ptr(), got.as_mut_ptr(), n);
            }

            let mut want = vec![0u8; n * 4];
            ycbcr_raw_scalar(&y, &cb, &cr, &mut want);

            assert_eq!(got, want, "AVX2 raw mismatch at width {}", width);
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn ycbcr_avx2_raw_half_matches_scalar() {
        if !std::is_x86_feature_detected!("avx2") {
            eprintln!("skipping: AVX2 not available on this host");
            return;
        }
        let raw_vals: [i16; 8] = [-32768, -8192, -64, -1, 0, 63, 8191, 32767];
        for &width in &[2usize, 8, 16, 18, 30, 32, 34, 48, 64, 96] {
            let n = width;
            let half = n.div_ceil(2);
            let make_seq = |seed: usize, len: usize| -> Vec<i16> {
                (0..len)
                    .map(|i| raw_vals[(i + seed) % raw_vals.len()])
                    .collect()
            };
            let y = make_seq(0, n);
            let cb_half = make_seq(3, half);
            let cr_half = make_seq(5, half);

            let mut got = vec![0u8; n * 4];
            #[allow(unsafe_code)]
            unsafe {
                super::ycbcr_avx2_raw_half(
                    y.as_ptr(),
                    cb_half.as_ptr(),
                    cr_half.as_ptr(),
                    got.as_mut_ptr(),
                    n,
                );
            }

            let mut want = vec![0u8; n * 4];
            ycbcr_raw_half_scalar(&y, &cb_half, &cr_half, &mut want);

            assert_eq!(got, want, "AVX2 raw_half mismatch at width {}", width);
        }
    }
}
