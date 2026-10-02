//! The inverse Dubuc-Deslauriers-Lemire (4,4) wavelet transform.

use super::*;

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

/// Load 8 `i16` values at stride `s` starting at `slice[phys_off]`.
///
/// Reads `slice[phys_off + j*s]` for j = 0..7. For s=1 this is identical to
/// [`load8`]. For s=2 and s=4 the AArch64 path uses `ld2`/`ld4` to deinterleave
/// in a single instruction; other targets use scalar loads that LLVM may
/// auto-vectorize.
#[inline(always)]
pub(super) fn load8s(slice: &[i16], phys_off: usize, s: usize) -> i32x8 {
    // s=1 fast path: single contiguous load + sign-extend.  Checked FIRST so that
    // the s=1 branch is a single cmp+b (not taken on s≠1) rather than a 5-branch
    // dispatch chain inside load8s_neon.
    if s == 1 {
        // x86_64 + AVX2 enabled at compile time: `vpmovsxwd ymm, [mem]` is one
        // instruction (movdqu + vpmovsxwd, fused on most µarchs). Compile-time
        // gating keeps the hot loop branch-free; runtime detection in this loop
        // would dominate the kernel.
        #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
        {
            #[allow(unsafe_code)]
            return unsafe { load8s_s1_avx2(slice, phys_off) };
        }
        // WASM simd128 compile-time path: `i32x4.extend_low/high_i16x8_s` sign-extends
        // 8×i16 → 8×i32 in two 128-bit ops, avoiding 8 scalar cast+store pairs.
        // On WASM, `wide::i32x8` is `{a: i32x4, b: i32x4}` where each `i32x4` is
        // `repr(transparent)` over `v128`, so [lo, hi]: [v128; 2] transmutes cleanly.
        #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
        {
            #[allow(unsafe_code)]
            return unsafe { load8s_s1_simd128(slice, phys_off) };
        }
        #[allow(unsafe_code, unreachable_code)]
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
pub(super) fn store8s(slice: &mut [i16], phys_off: usize, s: usize, v: i32x8) {
    // s=1 fast path: narrow and store contiguously.  Same reasoning as load8s.
    if s == 1 {
        #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
        {
            #[allow(unsafe_code)]
            return unsafe { store8s_s1_avx2(slice, phys_off, v) };
        }
        // WASM simd128: byte-shuffle to pack the low halfword of each i32 lane into
        // a contiguous i16x8.  Indices 0,1,4,5,8,9,12,13 pick bytes 0-1 of each 4-byte
        // i32 from the low half (lo), and indices 16,17,20,21,24,25,28,29 do the same
        // for the high half (hi).  This matches the truncating `as i16` semantics
        // (not saturating narrow) and mirrors the AVX2 byte-shuffle approach.
        #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
        {
            #[allow(unsafe_code)]
            return unsafe { store8s_s1_simd128(slice, phys_off, v) };
        }
        #[allow(unsafe_code, unreachable_code)]
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
pub(super) unsafe fn load8s_neon(slice: &[i16], phys_off: usize, s: usize) -> i32x8 {
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

// ---- x86_64 AVX2 stride-1 load/store ---------------------------------------
//
// `vpmovsxwd ymm, [mem]` sign-extends 8×i16 → 8×i32 in one fused load+convert.
// Truncating narrow i32x8 → i16x8 has no native AVX2 instruction (the only
// pack ops saturate); we emulate it with a per-lane byte shuffle that gathers
// the low halfword of each i32 lane, then a 64-bit lane permute to combine
// the two 128-bit halves.
//
// `i32x8` ↔ `__m256i` are layout-compatible on x86_64 with AVX2 enabled
// (`wide` uses `__m256i` internally), and the existing `C16: i32x8 = transmute([16i32; 8])`
// pattern at line ~1639 already relies on this. Both are 32 bytes.

#[cfg(all(target_arch = "x86_64", feature = "std"))]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn, dead_code)]
#[target_feature(enable = "avx2")]
#[inline]
pub(super) unsafe fn load8s_s1_avx2(slice: &[i16], phys_off: usize) -> i32x8 {
    use core::arch::x86_64::*;
    let ptr = slice.as_ptr().add(phys_off) as *const __m128i;
    let v16 = _mm_loadu_si128(ptr);
    let v32 = _mm256_cvtepi16_epi32(v16);
    let arr: [i32; 8] = core::mem::transmute(v32);
    i32x8::from(arr)
}

#[cfg(all(target_arch = "x86_64", feature = "std"))]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn, dead_code)]
#[target_feature(enable = "avx2")]
#[inline]
pub(super) unsafe fn store8s_s1_avx2(slice: &mut [i16], phys_off: usize, v: i32x8) {
    use core::arch::x86_64::*;
    let arr: [i32; 8] = v.to_array();
    let v32: __m256i = core::mem::transmute(arr);
    // Per-lane byte shuffle: pack low halfwords of each i32 into the low 64 bits
    // of each 128-bit lane. _mm256_shuffle_epi8 is per-128-bit-lane, so the same
    // 16-byte mask applies to both halves.
    let shuf = _mm256_setr_epi8(
        0, 1, 4, 5, 8, 9, 12, 13, -1, -1, -1, -1, -1, -1, -1, -1, 0, 1, 4, 5, 8, 9, 12, 13, -1, -1,
        -1, -1, -1, -1, -1, -1,
    );
    let shuffled = _mm256_shuffle_epi8(v32, shuf);
    // 64-bit lanes after shuffle: [lo_packed | zeros | hi_packed | zeros].
    // Permute to bring [lo_packed | hi_packed] into the low 128 bits.
    // Imm 0b00_00_10_00 = lane 0 → 0 (lo_packed), lane 1 → 2 (hi_packed).
    let permuted = _mm256_permute4x64_epi64::<0b00_00_10_00>(shuffled);
    let result = _mm256_castsi256_si128(permuted);
    let ptr = slice.as_mut_ptr().add(phys_off) as *mut __m128i;
    _mm_storeu_si128(ptr, result);
}

#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "neon")]
pub(super) unsafe fn store8s_neon(slice: &mut [i16], phys_off: usize, s: usize, v: i32x8) {
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

// ---- WASM simd128 stride-1 load/store ----------------------------------------
//
// On WASM simd128, `wide::i32x8` compiles to `{a: i32x4, b: i32x4}` where each
// `i32x4` is `repr(transparent)` over `v128`.  The struct is `repr(C, align(32))`
// so it is memory-compatible with `[v128; 2]` (two consecutive 128-bit values).
//
// Load: `i32x4.extend_low_i16x8_s` / `i32x4.extend_high_i16x8_s` each produce one
// `v128` of 4×i32 from the low/high 4 lanes of an i16x8, sign-extending in a single
// WASM instruction (equivalent to `_mm256_cvtepi16_epi32` on AVX2 but in two 128-bit
// ops).
//
// Store: `i8x16_shuffle` with constant mask picks bytes 0,1,4,5,8,9,12,13 from the
// low half and 0,1,4,5,8,9,12,13 from the high half (as indices 16..31 into the
// second operand), packing the low 2 bytes of each 4-byte i32 lane into a contiguous
// 16-byte i16x8.  This is the truncating `as i16` cast (not saturating), matching
// the scalar fallback and the AVX2 byte-shuffle approach.

#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn, dead_code)]
#[target_feature(enable = "simd128")]
#[inline]
pub(super) unsafe fn load8s_s1_simd128(slice: &[i16], phys_off: usize) -> i32x8 {
    use core::arch::wasm32::*;
    // Load 8 consecutive i16 (16 bytes) as a v128.
    let v16 = v128_load(slice.as_ptr().add(phys_off) as *const v128);
    // Sign-extend lower 4 i16 → i32x4 and upper 4 i16 → i32x4.
    let lo = i32x4_extend_low_i16x8(v16);
    let hi = i32x4_extend_high_i16x8(v16);
    // Transmute [v128; 2] → i32x8.  On WASM simd128, i32x8 is {a: i32x4(v128), b: i32x4(v128)}
    // (repr(C, align(32))), layout-compatible with two consecutive v128 values.
    core::mem::transmute::<[v128; 2], i32x8>([lo, hi])
}

#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn, dead_code)]
#[target_feature(enable = "simd128")]
#[inline]
pub(super) unsafe fn store8s_s1_simd128(slice: &mut [i16], phys_off: usize, v: i32x8) {
    use core::arch::wasm32::*;
    // Transmute i32x8 → [v128; 2] (lo = lower 4 lanes, hi = upper 4 lanes).
    let [lo, hi]: [v128; 2] = core::mem::transmute(v);
    // Pack low halfwords of each i32 lane via constant byte-shuffle.
    // Indices 0,1,4,5,8,9,12,13 select bytes 0-1 of lanes 0-3 from `lo` (first operand).
    // Indices 16,17,20,21,24,25,28,29 select bytes 0-1 of lanes 0-3 from `hi` (second operand).
    // Result is 8 consecutive i16 values, truncating i32→i16 (low 16 bits only).
    let out = i8x16_shuffle::<0, 1, 4, 5, 8, 9, 12, 13, 16, 17, 20, 21, 24, 25, 28, 29>(lo, hi);
    v128_store(slice.as_mut_ptr().add(phys_off) as *mut v128, out);
}

/// Load 8 contiguous `i32` values from `slice[off..]` into an `i32x8`.
///
/// # Safety
/// Caller must ensure `off + 7 < slice.len()`.
#[inline(always)]
#[allow(unsafe_code)]
pub(super) fn load8_i32(slice: &[i32], off: usize) -> i32x8 {
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
pub(super) fn store8_i32(slice: &mut [i32], off: usize, v: i32x8) {
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
pub(super) fn load_rows8(data: &[i16], offs: &[usize; 8], k: usize) -> i32x8 {
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
pub(super) fn store_rows8(data: &mut [i16], offs: &[usize; 8], k: usize, v: i32x8) {
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
pub(super) const C16: i32x8 = unsafe { core::mem::transmute([16i32; 8]) };
#[allow(unsafe_code)]
pub(super) const C8: i32x8 = unsafe { core::mem::transmute([8i32; 8]) };
#[allow(unsafe_code)]
pub(super) const C1: i32x8 = unsafe { core::mem::transmute([1i32; 8]) };

/// Lifting filter: `data[idx] -= ((9*(p1+n1) - (p3+n3) + 16) >> 5)`
#[inline(always)]
pub(super) fn lifting_even(cur: i32x8, p1: i32x8, n1: i32x8, p3: i32x8, n3: i32x8) -> i32x8 {
    let a = p1 + n1;
    let c = p3 + n3;
    cur - (((a << 3) + a - c + C16) >> 5)
}

/// Prediction filter (inner): `data[idx] += ((9*(p1+n1) - (p3+n3) + 8) >> 4)`
#[inline(always)]
pub(super) fn predict_inner(cur: i32x8, p1: i32x8, n1: i32x8, p3: i32x8, n3: i32x8) -> i32x8 {
    let a = p1 + n1;
    cur + (((a << 3) + a - (p3 + n3) + C8) >> 4)
}

/// Prediction filter (boundary): `data[idx] += ((p + n + 1) >> 1)`
#[inline(always)]
pub(super) fn predict_avg(cur: i32x8, p: i32x8, n: i32x8) -> i32x8 {
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
pub(super) unsafe fn row_pass_neon_s1_row(data: &mut [i16], row_off: usize, width: usize) {
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
            } else if k == 2 || k == 4 {
                // DjVuLibre `filter_bh` keeps the previous a3 here.
                next1
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
            } else if k == 2 || k == 4 {
                // DjVuLibre `filter_bh` keeps the previous a3 here.
                next1v
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

            next3v = if kmax >= 4 {
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
            } else if k == 2 || k == 4 {
                // DjVuLibre `filter_bh` keeps the previous a3 here.
                next1
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

            next3 = if kmax >= 4 {
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

pub(super) fn inverse_wavelet_transform(
    plane: &mut FlatPlane,
    width: usize,
    height: usize,
    subsample: usize,
) {
    inverse_wavelet_transform_from(plane, width, height, subsample, 16);
}

/// Like `inverse_wavelet_transform` but begins at `start_scale` instead of 16.
///
/// Use `start_scale = 16 / sub` when operating on a compact plane produced by
/// subsampling the coefficient scatter by factor `sub`.  For example, the sub=2
/// compact plane only contains coefficients up to scale 8, so the s=16 pass
/// would be purely spurious.
pub(super) fn inverse_wavelet_transform_from(
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

                if kmax >= 4 {
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
