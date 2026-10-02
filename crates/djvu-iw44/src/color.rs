//! YCbCr → RGBA row conversion, portable and SIMD.

use super::*;

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

    // Portable path: as_chunks eliminates per-element bounds checks.
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
pub(super) fn ycbcr_row_from_i16(y: &[i16], cb: &[i16], cr: &[i16], out: &mut [u8]) {
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
    // Runtime AVX2 detection requires `std` (`is_x86_feature_detected!`).
    #[cfg(all(target_arch = "x86_64", feature = "std"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            #[allow(unsafe_code)]
            unsafe {
                ycbcr_avx2_raw(y.as_ptr(), cb.as_ptr(), cr.as_ptr(), out.as_mut_ptr(), w);
            }
            return;
        }
    }
    // WASM simd128 is compile-time only; no runtime detection.
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        #[allow(unsafe_code)]
        unsafe {
            ycbcr_simd128_raw(y.as_ptr(), cb.as_ptr(), cr.as_ptr(), out.as_mut_ptr(), w);
        }
        return;
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
///
/// This is DjVuLibre's `crcb_half` rendering (#830): nearest-neighbour, not
/// the bilinear upsampling #422 once used, because DjVuLibre repeats each
/// scale-2 chroma value over its 2x2 block.
#[inline]
pub(super) fn ycbcr_row_from_i16_half(
    y: &[i16],
    cb_half: &[i16],
    cr_half: &[i16],
    out: &mut [u8],
    w: usize,
) {
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
    #[cfg(all(target_arch = "x86_64", feature = "std"))]
    {
        if std::is_x86_feature_detected!("avx2") {
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
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        #[allow(unsafe_code)]
        unsafe {
            ycbcr_simd128_raw_half(
                y.as_ptr(),
                cb_half.as_ptr(),
                cr_half.as_ptr(),
                out.as_mut_ptr(),
                w,
            );
        }
        return;
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

/// Portable YCbCr→RGBA using as_chunks so LLVM sees exact 8-element slices.
#[inline(always)]
pub(super) fn ycbcr_portable(
    y_row: &[i32],
    cb_row: &[i32],
    cr_row: &[i32],
    out: &mut [u8],
    w: usize,
) {
    use wide::i32x8;
    let c128 = i32x8::splat(128);
    let c0 = i32x8::splat(0);
    let c255 = i32x8::splat(255);

    let full8 = w / 8;
    for (((yc, cbc), crc), outc) in y_row[..full8 * 8]
        .as_chunks::<8>()
        .0
        .iter()
        .zip(cb_row[..full8 * 8].as_chunks::<8>().0)
        .zip(cr_row[..full8 * 8].as_chunks::<8>().0)
        .zip(out[..full8 * 32].as_chunks_mut::<32>().0)
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
pub(super) unsafe fn ycbcr_neon_raw(
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
pub(super) unsafe fn ycbcr_neon_raw_half(
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
#[cfg(all(target_arch = "x86_64", feature = "std"))]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "avx2")]
pub(super) unsafe fn ycbcr_avx2_raw(
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
#[cfg(all(target_arch = "x86_64", feature = "std"))]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "avx2")]
pub(super) unsafe fn ycbcr_avx2_raw_half(
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

/// WASM simd128 fused normalize + YCbCr→RGBA from raw i16 plane data (non-chroma-half).
///
/// 8 pixels per iteration, mirroring the AArch64 NEON kernel byte-for-byte.
/// `v128` is 128 bits → 8×i16, same width as NEON's `int16x8_t`. Saturating
/// signed-i16 → unsigned-u8 narrow is one instruction (`u8x16_narrow_i16x8`),
/// equivalent to NEON `vqmovun_s16`.
///
/// RGBA byte-interleave is materialised via two `i8x16_shuffle` calls
/// (constant-mask shuffle, 16 lanes each, picking from {r/g pack, b/alpha pack}).
/// WASM has no `vst4`-equivalent; the shuffle pair is the simd128 idiom.
///
/// `cbp` and `crp` must point to `w` values each (same stride as `yp`).
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn, dead_code)]
#[target_feature(enable = "simd128")]
pub(super) unsafe fn ycbcr_simd128_raw(
    yp: *const i16,
    cbp: *const i16,
    crp: *const i16,
    outp: *mut u8,
    w: usize,
) {
    use core::arch::wasm32::*;
    let n_min = i16x8_splat(-128);
    let n_max = i16x8_splat(127);
    let c128 = i16x8_splat(128);
    let one = i16x8_splat(1);
    // Saturating-narrow input ≥ 255 → 255, so any sentinel ≥ 255 produces the
    // alpha byte without a separate splat-store path.
    let alpha_src = i16x8_splat(255);

    let full8 = w / 8;
    for i in 0..full8 {
        let off = i * 8;
        // Rounding right shift by 6 + clamp to [-128, 127].
        // Same overflow-safe form as the AVX2 path: `(v >> 6) + ((v as u16 >> 5) & 1)`.
        // Avoids the i16-overflow that would happen with `(v + 32) >> 6` for v near
        // `i16::MAX` and matches the wider intermediate that NEON `vrshrq_n_s16` uses.
        let load_norm_clamp = |p: *const i16| -> v128 {
            let v = v128_load(p as *const v128);
            let high = i16x8_shr(v, 6);
            let bit5 = v128_and(u16x8_shr(v, 5), one);
            let n = i16x8_add(high, bit5);
            i16x8_max(i16x8_min(n, n_max), n_min)
        };
        let yc = load_norm_clamp(yp.add(off));
        let cbc = load_norm_clamp(cbp.add(off));
        let crc = load_norm_clamp(crp.add(off));

        // Same i16 arithmetic as NEON / AVX2 — all intermediates fit in i16.
        let y128 = i16x8_add(yc, c128);
        let t2 = i16x8_add(crc, i16x8_shr(crc, 1));
        let t3 = i16x8_sub(y128, i16x8_shr(cbc, 2));
        let r16 = i16x8_add(y128, t2);
        let g16 = i16x8_sub(t3, i16x8_shr(t2, 1));
        let b16 = i16x8_add(t3, i16x8_shl(cbc, 1));

        // Saturating signed→unsigned narrow: i16x8 → u8x16 (clamps to [0, 255]).
        // Pack two i16x8 vectors into one u8x16 in a single op — exactly NEON's
        // `vqmovun_s16` semantics, just in the wider 16-lane form.
        let v_rg = u8x16_narrow_i16x8(r16, g16);
        let v_ba = u8x16_narrow_i16x8(b16, alpha_src);

        // Interleave to RGBA: pixel n = (r_n, g_n, b_n, a_n).
        // v_rg lanes: 0..7 = r, 8..15 = g. v_ba lanes: 0..7 = b, 8..15 = 255.
        // Constant byte-shuffle picks {r_n=v_rg[n], g_n=v_rg[n+8], b_n=v_ba[n+0], a_n=v_ba[n+8]}.
        let out0 =
            i8x16_shuffle::<0, 8, 16, 24, 1, 9, 17, 25, 2, 10, 18, 26, 3, 11, 19, 27>(v_rg, v_ba);
        let out1 =
            i8x16_shuffle::<4, 12, 20, 28, 5, 13, 21, 29, 6, 14, 22, 30, 7, 15, 23, 31>(v_rg, v_ba);

        v128_store(outp.add(off * 4) as *mut v128, out0);
        v128_store(outp.add(off * 4 + 16) as *mut v128, out1);
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

/// WASM simd128 fused normalize + YCbCr→RGBA, chroma-half variant.
///
/// 8 luma + 4 chroma per iteration. Chroma is loaded as 8 bytes via
/// `v128_load64_zero` (low half = 4 i16, high half = 0), normalized at
/// i16 width across all 8 lanes (high lanes normalize to 0, unused), and
/// nearest-neighbour upsampled to 8 lanes via a constant byte shuffle that
/// duplicates each of the low 4 i16 lanes (`[a,b,c,d,_,_,_,_]` → `[a,a,b,b,c,c,d,d]`).
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn, dead_code)]
#[target_feature(enable = "simd128")]
pub(super) unsafe fn ycbcr_simd128_raw_half(
    yp: *const i16,
    cbp: *const i16,
    crp: *const i16,
    outp: *mut u8,
    w: usize,
) {
    use core::arch::wasm32::*;
    let n_min = i16x8_splat(-128);
    let n_max = i16x8_splat(127);
    let c128 = i16x8_splat(128);
    let one = i16x8_splat(1);
    let alpha_src = i16x8_splat(255);

    let full8 = w / 8;
    for i in 0..full8 {
        let off = i * 8;
        let c_off = i * 4;
        // Y: full 8-lane load + normalize (same as non-half path).
        let load_norm_clamp = |p: *const i16| -> v128 {
            let v = v128_load(p as *const v128);
            let high = i16x8_shr(v, 6);
            let bit5 = v128_and(u16x8_shr(v, 5), one);
            let n = i16x8_add(high, bit5);
            i16x8_max(i16x8_min(n, n_max), n_min)
        };
        let yc = load_norm_clamp(yp.add(off));

        // Chroma: load 4 i16 = 8 bytes into low half of v128, zero upper half.
        // Normalize on the full vector (upper 4 lanes normalize to 0, harmless).
        let load_norm_chroma_4 = |p: *const i16| -> v128 {
            let v = v128_load64_zero(p as *const u64);
            let high = i16x8_shr(v, 6);
            let bit5 = v128_and(u16x8_shr(v, 5), one);
            let n = i16x8_add(high, bit5);
            i16x8_max(i16x8_min(n, n_max), n_min)
        };
        let cb4 = load_norm_chroma_4(cbp.add(c_off));
        let cr4 = load_norm_chroma_4(crp.add(c_off));

        // Upsample each i16 lane into a pair (`zip-low` of self+self).
        // Byte-level shuffle: bytes 0,1 → 0,1,2,3 ; 2,3 → 4,5,6,7 ; etc.
        let cbc = i8x16_shuffle::<0, 1, 0, 1, 2, 3, 2, 3, 4, 5, 4, 5, 6, 7, 6, 7>(cb4, cb4);
        let crc = i8x16_shuffle::<0, 1, 0, 1, 2, 3, 2, 3, 4, 5, 4, 5, 6, 7, 6, 7>(cr4, cr4);

        let y128 = i16x8_add(yc, c128);
        let t2 = i16x8_add(crc, i16x8_shr(crc, 1));
        let t3 = i16x8_sub(y128, i16x8_shr(cbc, 2));
        let r16 = i16x8_add(y128, t2);
        let g16 = i16x8_sub(t3, i16x8_shr(t2, 1));
        let b16 = i16x8_add(t3, i16x8_shl(cbc, 1));

        let v_rg = u8x16_narrow_i16x8(r16, g16);
        let v_ba = u8x16_narrow_i16x8(b16, alpha_src);
        let out0 =
            i8x16_shuffle::<0, 8, 16, 24, 1, 9, 17, 25, 2, 10, 18, 26, 3, 11, 19, 27>(v_rg, v_ba);
        let out1 =
            i8x16_shuffle::<4, 12, 20, 28, 5, 13, 21, 29, 6, 14, 22, 30, 7, 15, 23, 31>(v_rg, v_ba);
        v128_store(outp.add(off * 4) as *mut v128, out0);
        v128_store(outp.add(off * 4 + 16) as *mut v128, out1);
    }
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

/// AArch64 NEON: 6× vld1q_s32 + SIMD arithmetic + vst4_u8 per 8 pixels.
/// Replaces 80+ bounds-check branches per 8 pixels in the LLVM-generated portable code.
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "neon")]
pub(super) unsafe fn ycbcr_neon(
    yp: *const i32,
    cbp: *const i32,
    crp: *const i32,
    outp: *mut u8,
    w: usize,
) {
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
