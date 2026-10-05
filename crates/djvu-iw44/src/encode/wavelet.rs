//! The forward IW44 wavelet transform (portable and NEON).

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
pub(super) fn lift(cur: i32, p1: i32, n1: i32, p3: i32, n3: i32) -> i32 {
    let a = p1 + n1;
    let c = p3 + n3;
    cur + (((a << 3) + a - c + 16) >> 5)
}

/// Predict (inner): odd sample -= ((9*(p1+n1)-(p3+n3)+8)>>4)
#[inline(always)]
pub(super) fn pred_inner_fwd(cur: i32, p1: i32, n1: i32, p3: i32, n3: i32) -> i32 {
    let a = p1 + n1;
    cur - (((a << 3) + a - (p3 + n3) + 8) >> 4)
}

/// Predict (boundary avg): odd sample -= (p+n+1)>>1
#[inline(always)]
pub(super) fn pred_avg_fwd(cur: i32, p: i32, n: i32) -> i32 {
    cur - ((p + n + 1) >> 1)
}

/// NEON row pass for s=1 (forward analysis: odd pass first, then even pass).
///
/// Forward predict subtracts; forward lift adds.  This is the exact sign-dual
/// of `row_pass_neon_s1_row` in `iw44_new`.
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "neon")]
pub(super) unsafe fn forward_row_neon_s1_row(data: &mut [i16], row_off: usize, width: usize) {
    use core::arch::aarch64::*;

    let kmax = width - 1;
    let border = kmax.saturating_sub(3);
    let ptr = data.as_mut_ptr().add(row_off);

    let even_chunks = if width >= 32 { (width - 31) / 16 } else { 0 };

    // ── Step 1: odd pass (predict, forward: subtract) ─────────────────────────

    // k=1 boundary scalar
    if kmax >= 1 {
        let p = *data.get_unchecked(row_off) as i32;
        let idx1 = row_off + 1;
        if kmax >= 2 {
            let n = *data.get_unchecked(row_off + 2) as i32;
            *data.get_unchecked_mut(idx1) =
                (*data.get_unchecked(idx1) as i32 - ((p + n + 1) >> 1)) as i16;
        } else {
            *data.get_unchecked_mut(idx1) = (*data.get_unchecked(idx1) as i32 - p) as i16;
        }
    }

    // NEON inner odd chunks
    let odd_chunks = if kmax >= 20 {
        even_chunks.min((kmax - 20) / 16 + 1)
    } else {
        0
    };

    // `pair1` carries over from the previous chunk's `pair2`. Reloading it
    // would read across the 16 samples just stored at `chunk * 16 + 2`; the
    // partial overlap defeats store-to-load forwarding and stalls every
    // chunk. Only its lane-0 odd changed since the load, and that lane is
    // never used (`curr_odds` starts at lane 1).
    let mut pair1 = if odd_chunks > 0 {
        vld2q_s16(ptr as *const i16)
    } else {
        int16x8x2_t(vdupq_n_s16(0), vdupq_n_s16(0))
    };
    for chunk in 0..odd_chunks {
        let pair2 = vld2q_s16(ptr.add((chunk + 1) * 16) as *const i16);

        // 8 inner odds at physical positions 3+chunk*16, 5+..., 17+chunk*16
        let curr_odds = vextq_s16::<1>(pair1.1, pair2.1);

        let p3_e = pair1.0;
        let p1_e = vextq_s16::<1>(pair1.0, pair2.0);
        let n1_e = vextq_s16::<2>(pair1.0, pair2.0);
        let n3_e = vextq_s16::<3>(pair1.0, pair2.0);

        macro_rules! predict_fwd {
            ($co:expr, $p1:expr, $n1:expr, $p3:expr, $n3:expr) => {{
                let a = vaddq_s32($p1, $n1);
                let c = vaddq_s32($p3, $n3);
                let nine_a = vaddq_s32(vshlq_n_s32::<3>(a), a);
                let delta = vshrq_n_s32::<4>(vsubq_s32(vaddq_s32(nine_a, vdupq_n_s32(8i32)), c));
                vsubq_s32($co, delta) // forward: subtract
            }};
        }

        let new_lo = predict_fwd!(
            vmovl_s16(vget_low_s16(curr_odds)),
            vmovl_s16(vget_low_s16(p1_e)),
            vmovl_s16(vget_low_s16(n1_e)),
            vmovl_s16(vget_low_s16(p3_e)),
            vmovl_s16(vget_low_s16(n3_e))
        );
        let new_hi = predict_fwd!(
            vmovl_high_s16(curr_odds),
            vmovl_high_s16(p1_e),
            vmovl_high_s16(n1_e),
            vmovl_high_s16(p3_e),
            vmovl_high_s16(n3_e)
        );
        let new_odds = vcombine_s16(vmovn_s32(new_lo), vmovn_s32(new_hi));

        // store: evens at chunk*16+2..+16 unchanged (= p1_e), odds updated
        vst2q_s16(ptr.add(chunk * 16 + 2), int16x8x2_t(p1_e, new_odds));
        pair1 = pair2;
    }

    // scalar odd tail: k = 3+odd_chunks*16, ..., kmax
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
                    (*data.get_unchecked(idx) as i32 - (((a << 3) + a - c + 8) >> 4)) as i16;
            } else if k < kmax {
                *data.get_unchecked_mut(idx) =
                    (*data.get_unchecked(idx) as i32 - ((prev1 + next1 + 1) >> 1)) as i16;
            } else {
                *data.get_unchecked_mut(idx) = (*data.get_unchecked(idx) as i32 - prev1) as i16;
            }
            k += 2;
        }
    }

    // ── Step 2: even pass (lift, forward: add) ────────────────────────────────

    let mut prev_odd = vdupq_n_s16(0i16);

    for chunk in 0..even_chunks {
        let curr_pair = vld2q_s16(ptr.add(chunk * 16) as *const i16);
        let next_pair = vld2q_s16(ptr.add((chunk + 1) * 16) as *const i16);
        let curr_even = curr_pair.0;
        let curr_odd = curr_pair.1; // already updated by Step 1
        let next_odd = next_pair.1;

        let p1 = vextq_s16::<7>(prev_odd, curr_odd);
        let n1 = curr_odd;
        let p3 = vextq_s16::<6>(prev_odd, curr_odd);
        let n3 = vextq_s16::<1>(curr_odd, next_odd);

        macro_rules! lift_fwd {
            ($ce:expr, $p1:expr, $n1:expr, $p3:expr, $n3:expr) => {{
                let a = vaddq_s32($p1, $n1);
                let c = vaddq_s32($p3, $n3);
                let nine_a = vaddq_s32(vshlq_n_s32::<3>(a), a);
                let delta = vshrq_n_s32::<5>(vsubq_s32(vaddq_s32(nine_a, vdupq_n_s32(16i32)), c));
                vaddq_s32($ce, delta) // forward: add
            }};
        }

        let new_lo = lift_fwd!(
            vmovl_s16(vget_low_s16(curr_even)),
            vmovl_s16(vget_low_s16(p1)),
            vmovl_s16(vget_low_s16(n1)),
            vmovl_s16(vget_low_s16(p3)),
            vmovl_s16(vget_low_s16(n3))
        );
        let new_hi = lift_fwd!(
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

    // scalar even tail
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
                (*data.get_unchecked(idx) as i32 + (((a << 3) + a - c + 16) >> 5)) as i16;
            k += 2;
        }
    }
}

/// Forward row pass (analysis) at scale `s`.
///
/// Operates on every `s`-th row, within each row on every sample.
pub(super) fn forward_row_pass(
    data: &mut [i16],
    width: usize,
    height: usize,
    stride: usize,
    s: usize,
) {
    // AArch64 NEON path at s=1
    #[cfg(target_arch = "aarch64")]
    if s == 1 {
        for row in (0..height).step_by(s) {
            #[allow(unsafe_code)]
            unsafe {
                forward_row_neon_s1_row(data, row * stride, width);
            }
        }
        return;
    }

    // At s≥2 the row's active samples (every `s`-th) form the same sequence the
    // s=1 pass sees, so gather them into a dense buffer, run the NEON s=1 row
    // there, and scatter back. Bit-identical to the strided scalar loop below.
    #[cfg(target_arch = "aarch64")]
    {
        let n = ((width - 1) >> s.trailing_zeros()) + 1;
        let mut buf = vec![0i16; n];
        for row in (0..height).step_by(s) {
            let off = row * stride;
            let line = &mut data[off..off + width];
            gather_strided(line, &mut buf, s);
            #[allow(unsafe_code)]
            unsafe {
                forward_row_neon_s1_row(&mut buf, 0, n);
            }
            scatter_strided(&buf, line, s);
        }
    }
    #[cfg(not(target_arch = "aarch64"))]
    forward_row_pass_scalar(data, width, height, stride, s);
}

/// Copy every `s`-th sample of `line` (from index 0) into `buf`.
#[cfg(target_arch = "aarch64")]
fn gather_strided(line: &[i16], buf: &mut [i16], s: usize) {
    let mut i = 0usize;
    if s == 2 {
        use core::arch::aarch64::*;
        // 8 outputs read 16 inputs; stay inside `line`.
        while 2 * i + 16 <= line.len() {
            #[allow(unsafe_code)]
            unsafe {
                let pair = vld2q_s16(line.as_ptr().add(2 * i));
                vst1q_s16(buf.as_mut_ptr().add(i), pair.0);
            }
            i += 8;
        }
    }
    for (b, &v) in buf[i..].iter_mut().zip(line[i * s..].iter().step_by(s)) {
        *b = v;
    }
}

/// Inverse of [`gather_strided`]: write `buf` back to every `s`-th sample.
#[cfg(target_arch = "aarch64")]
fn scatter_strided(buf: &[i16], line: &mut [i16], s: usize) {
    let mut i = 0usize;
    if s == 2 {
        use core::arch::aarch64::*;
        while 2 * i + 16 <= line.len() {
            #[allow(unsafe_code)]
            unsafe {
                let p = line.as_mut_ptr().add(2 * i);
                let pair = vld2q_s16(p);
                vst2q_s16(p, int16x8x2_t(vld1q_s16(buf.as_ptr().add(i)), pair.1));
            }
            i += 8;
        }
    }
    for (&b, v) in buf[i..].iter().zip(line[i * s..].iter_mut().step_by(s)) {
        *v = b;
    }
}

/// Portable forward row pass at scale `s` (every `s`-th row and sample).
#[cfg(any(test, not(target_arch = "aarch64")))]
fn forward_row_pass_scalar(data: &mut [i16], width: usize, height: usize, stride: usize, s: usize) {
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

/// Load 8 active columns starting at `p`: consecutive at `S == 1`, the even
/// lanes of 16 samples at `S == 2`.
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[inline(always)]
unsafe fn ld_cols<const S: usize>(p: *const i16) -> core::arch::aarch64::int16x8_t {
    use core::arch::aarch64::*;
    if S == 1 { vld1q_s16(p) } else { vld2q_s16(p).0 }
}

/// Store 8 active columns at `p`, keeping the inactive odd lanes at `S == 2`.
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[inline(always)]
unsafe fn st_cols<const S: usize>(p: *mut i16, v: core::arch::aarch64::int16x8_t) {
    use core::arch::aarch64::*;
    if S == 1 {
        vst1q_s16(p, v)
    } else {
        vst2q_s16(p, int16x8x2_t(v, vld2q_s16(p).1))
    }
}

/// NEON inner predict for the column pass at s=`S` (1 or 2).
///
/// Processes 8 active columns per iteration.  All 5 row offsets are for
/// the currently-active odd row k.  Performs:
///   data[k0+col] -= ((9*(p1+n1) - (p3+n3) + 8) >> 4)
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "neon")]
pub(super) unsafe fn forward_col_predict_neon<const S: usize>(
    data: &mut [i16],
    km3_off: usize,
    km1_off: usize,
    k0_off: usize,
    kp1_off: usize,
    kp3_off: usize,
    width: usize,
) {
    use core::arch::aarch64::*;
    let ptr = data.as_mut_ptr();
    let d8 = vdupq_n_s32(8i32);
    let mut col = 0usize;
    // 8 active columns span 8*S samples; stay inside the row.
    while col + 8 * S <= width {
        let p3 = ld_cols::<S>(ptr.add(km3_off + col));
        let p1 = ld_cols::<S>(ptr.add(km1_off + col));
        let cur = ld_cols::<S>(ptr.add(k0_off + col));
        let n1 = ld_cols::<S>(ptr.add(kp1_off + col));
        let n3 = ld_cols::<S>(ptr.add(kp3_off + col));
        let a_lo = vaddq_s32(vmovl_s16(vget_low_s16(p1)), vmovl_s16(vget_low_s16(n1)));
        let a_hi = vaddq_s32(vmovl_high_s16(p1), vmovl_high_s16(n1));
        let c_lo = vaddq_s32(vmovl_s16(vget_low_s16(p3)), vmovl_s16(vget_low_s16(n3)));
        let c_hi = vaddq_s32(vmovl_high_s16(p3), vmovl_high_s16(n3));
        let nine_a_lo = vaddq_s32(vshlq_n_s32::<3>(a_lo), a_lo);
        let nine_a_hi = vaddq_s32(vshlq_n_s32::<3>(a_hi), a_hi);
        let delta_lo = vshrq_n_s32::<4>(vsubq_s32(vaddq_s32(nine_a_lo, d8), c_lo));
        let delta_hi = vshrq_n_s32::<4>(vsubq_s32(vaddq_s32(nine_a_hi, d8), c_hi));
        let delta = vcombine_s16(vmovn_s32(delta_lo), vmovn_s32(delta_hi));
        st_cols::<S>(ptr.add(k0_off + col), vsubq_s16(cur, delta));
        col += 8 * S;
    }
    while col < width {
        let p1 = *data.get_unchecked(km1_off + col) as i32;
        let n1 = *data.get_unchecked(kp1_off + col) as i32;
        let p3 = *data.get_unchecked(km3_off + col) as i32;
        let n3 = *data.get_unchecked(kp3_off + col) as i32;
        *data.get_unchecked_mut(k0_off + col) =
            pred_inner_fwd(*data.get_unchecked(k0_off + col) as i32, p1, n1, p3, n3) as i16;
        col += S;
    }
}

/// NEON col-pass lift for s=`S` (1 or 2): one even row, 8 active columns per
/// iteration.
///
/// State slices (`prev3`, `prev1`, `next1`) hold one i16 per active column
/// (values bounded by i16 after predict).  Performs:
///   data[k0+col] += ((9*(p1+n1) - (p3+n3) + 16) >> 5)
/// then advances state: prev3 ← prev1, prev1 ← next1, next1 ← n3.
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn, clippy::too_many_arguments)]
#[target_feature(enable = "neon")]
pub(super) unsafe fn forward_col_lift_neon_row<const S: usize>(
    data: &mut [i16],
    k0_off: usize,
    n3_off: usize, // ignored when !has_n3
    has_n3: bool,
    prev3: &mut [i16],
    prev1: &mut [i16],
    next1: &mut [i16],
    width: usize,
) {
    use core::arch::aarch64::*;
    let ptr = data.as_mut_ptr();
    let p3p = prev3.as_mut_ptr();
    let p1p = prev1.as_mut_ptr();
    let n1p = next1.as_mut_ptr();
    let d16 = vdupq_n_s32(16i32);
    let mut ci = 0usize;
    while S * ci + 8 * S <= width {
        let col = S * ci;
        let p3_s = vld1q_s16(p3p.add(ci) as *const i16);
        let p1_s = vld1q_s16(p1p.add(ci) as *const i16);
        let n1_s = vld1q_s16(n1p.add(ci) as *const i16);
        let n3_s = if has_n3 {
            ld_cols::<S>(ptr.add(n3_off + col))
        } else {
            vdupq_n_s16(0)
        };
        let cur_s = ld_cols::<S>(ptr.add(k0_off + col));
        let a_lo = vaddq_s32(vmovl_s16(vget_low_s16(p1_s)), vmovl_s16(vget_low_s16(n1_s)));
        let a_hi = vaddq_s32(vmovl_high_s16(p1_s), vmovl_high_s16(n1_s));
        let c_lo = vaddq_s32(vmovl_s16(vget_low_s16(p3_s)), vmovl_s16(vget_low_s16(n3_s)));
        let c_hi = vaddq_s32(vmovl_high_s16(p3_s), vmovl_high_s16(n3_s));
        let nine_a_lo = vaddq_s32(vshlq_n_s32::<3>(a_lo), a_lo);
        let nine_a_hi = vaddq_s32(vshlq_n_s32::<3>(a_hi), a_hi);
        let delta_lo = vshrq_n_s32::<5>(vsubq_s32(vaddq_s32(nine_a_lo, d16), c_lo));
        let delta_hi = vshrq_n_s32::<5>(vsubq_s32(vaddq_s32(nine_a_hi, d16), c_hi));
        let delta_s = vcombine_s16(vmovn_s32(delta_lo), vmovn_s32(delta_hi));
        st_cols::<S>(ptr.add(k0_off + col), vaddq_s16(cur_s, delta_s));
        // advance state
        vst1q_s16(p3p.add(ci), p1_s);
        vst1q_s16(p1p.add(ci), n1_s);
        vst1q_s16(n1p.add(ci), n3_s);
        ci += 8;
    }
    // scalar tail
    while S * ci < width {
        let col = S * ci;
        let p3 = *prev3.get_unchecked(ci) as i32;
        let p1 = *prev1.get_unchecked(ci) as i32;
        let n1 = *next1.get_unchecked(ci) as i32;
        let n3 = if has_n3 {
            *data.get_unchecked(n3_off + col) as i32
        } else {
            0
        };
        *data.get_unchecked_mut(k0_off + col) =
            lift(*data.get_unchecked(k0_off + col) as i32, p1, n1, p3, n3) as i16;
        *prev3.get_unchecked_mut(ci) = p1 as i16;
        *prev1.get_unchecked_mut(ci) = n1 as i16;
        *next1.get_unchecked_mut(ci) = n3 as i16;
        ci += 1;
    }
}

/// Forward column pass (analysis) at scale `s`.
pub(super) fn forward_col_pass(
    data: &mut [i16],
    width: usize,
    height: usize,
    stride: usize,
    s: usize,
) {
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
            #[cfg(target_arch = "aarch64")]
            if s <= 2 {
                #[allow(unsafe_code)]
                unsafe {
                    if s == 1 {
                        forward_col_predict_neon::<1>(
                            data, km3_off, km1_off, k0_off, kp1_off, kp3_off, width,
                        );
                    } else {
                        forward_col_predict_neon::<2>(
                            data, km3_off, km1_off, k0_off, kp1_off, kp3_off, width,
                        );
                    }
                }
                k += 2;
                continue;
            }
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
    // AArch64 NEON path at s≤2: i16 state, 8 active columns/iter
    #[cfg(target_arch = "aarch64")]
    if s <= 2 {
        let num_cols = width.div_ceil(col_step);
        let mut prev3: Vec<i16> = vec![0i16; num_cols];
        let mut prev1: Vec<i16> = vec![0i16; num_cols];
        let mut next1: Vec<i16> = if kmax >= 1 {
            let off = (1 << sd) * stride;
            data[off..off + width]
                .iter()
                .step_by(col_step)
                .copied()
                .collect()
        } else {
            vec![0i16; num_cols]
        };
        let mut k = 0usize;
        while k <= kmax {
            let k0_off = (k << sd) * stride;
            let has_n3 = k + 3 <= kmax;
            let n3_off = if has_n3 { ((k + 3) << sd) * stride } else { 0 };
            #[allow(unsafe_code)]
            unsafe {
                if s == 1 {
                    forward_col_lift_neon_row::<1>(
                        data, k0_off, n3_off, has_n3, &mut prev3, &mut prev1, &mut next1, width,
                    );
                } else {
                    forward_col_lift_neon_row::<2>(
                        data, k0_off, n3_off, has_n3, &mut prev3, &mut prev1, &mut next1, width,
                    );
                }
            }
            k += 2;
        }
        return;
    }
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
pub(super) fn forward_wavelet_transform(
    data: &mut [i16],
    width: usize,
    height: usize,
    stride: usize,
) {
    let mut s = 1usize;
    while s <= 16 {
        forward_row_pass(data, width, height, stride, s);
        forward_col_pass(data, width, height, stride, s);
        s <<= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Portable reference: the scalar row pass, with the column pass done as
    /// a row pass on the transposed plane.
    fn reference_transform(data: &mut [i16], width: usize, height: usize) {
        let mut t = vec![0i16; width * height];
        let mut s = 1usize;
        while s <= 16 {
            forward_row_pass_scalar(data, width, height, width, s);
            for y in 0..height {
                for x in 0..width {
                    t[x * height + y] = data[y * width + x];
                }
            }
            forward_row_pass_scalar(&mut t, height, width, height, s);
            for y in 0..height {
                for x in 0..width {
                    data[y * width + x] = t[x * height + y];
                }
            }
            s <<= 1;
        }
    }

    // The NEON paths (s=1 rows/cols, s=2 cols, gathered s≥2 rows) must match
    // the portable passes bit for bit, including every tail length.
    #[test]
    fn forward_transform_matches_portable_reference() {
        let mut seed = 0x9e37_79b9u32;
        for (width, height) in [
            (1, 1),
            (2, 3),
            (7, 5),
            (17, 33),
            (31, 18),
            (64, 64),
            (97, 61),
            (130, 47),
        ] {
            let plane: Vec<i16> = (0..width * height)
                .map(|_| {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    ((seed % 16384) as i32 - 8192) as i16
                })
                .collect();
            let mut fast = plane.clone();
            forward_wavelet_transform(&mut fast, width, height, width);
            let mut slow = plane;
            reference_transform(&mut slow, width, height);
            assert_eq!(fast, slow, "{width}x{height}");
        }
    }
}
