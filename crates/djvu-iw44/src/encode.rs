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

use djvu_pixmap::{GrayPixmap, Pixmap};
use djvu_zp::encoder::ZpEncoder;

// Band/quant/state-flag/zigzag spec data is shared with the decoder (one source
// of truth) rather than re-declared here — see the `pub(crate)` items in `lib.rs`.
use crate::{
    ACTIVE, BAND_BUCKETS, NEW, QUANT_HI_INIT, QUANT_LO_INIT, UNK, ZERO, zigzag_col, zigzag_row,
};

// ---- Forward zigzag scatter tables (encoder-only) ----------------------------
//
// The encoder scatters block coefficients into the plane in *forward* order, so
// it needs `i -> (row, col)` lookup tables. The decoder keeps the *inverse*
// tables (`(row, col) -> i`); both are generated from the shared
// `zigzag_row`/`zigzag_col` bit-interleave functions.

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

/// NEON row pass for s=1 (forward analysis: odd pass first, then even pass).
///
/// Forward predict subtracts; forward lift adds.  This is the exact sign-dual
/// of `row_pass_neon_s1_row` in `iw44_new`.
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "neon")]
unsafe fn forward_row_neon_s1_row(data: &mut [i16], row_off: usize, width: usize) {
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

    for chunk in 0..odd_chunks {
        let pair1 = vld2q_s16(ptr.add(chunk * 16) as *const i16);
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
fn forward_row_pass(data: &mut [i16], width: usize, height: usize, stride: usize, s: usize) {
    let sd = s.trailing_zeros() as usize;
    let kmax = (width - 1) >> sd;
    let border = kmax.saturating_sub(3);

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

/// NEON inner predict for the column pass at s=1.
///
/// Processes 8 consecutive columns per iteration.  All 5 row offsets are for
/// the currently-active odd row k.  Performs:
///   data[k0+col] -= ((9*(p1+n1) - (p3+n3) + 8) >> 4)
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "neon")]
unsafe fn forward_col_predict_neon(
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
    while col + 8 <= width {
        let p3 = vld1q_s16(ptr.add(km3_off + col) as *const i16);
        let p1 = vld1q_s16(ptr.add(km1_off + col) as *const i16);
        let cur = vld1q_s16(ptr.add(k0_off + col) as *const i16);
        let n1 = vld1q_s16(ptr.add(kp1_off + col) as *const i16);
        let n3 = vld1q_s16(ptr.add(kp3_off + col) as *const i16);
        let a_lo = vaddq_s32(vmovl_s16(vget_low_s16(p1)), vmovl_s16(vget_low_s16(n1)));
        let a_hi = vaddq_s32(vmovl_high_s16(p1), vmovl_high_s16(n1));
        let c_lo = vaddq_s32(vmovl_s16(vget_low_s16(p3)), vmovl_s16(vget_low_s16(n3)));
        let c_hi = vaddq_s32(vmovl_high_s16(p3), vmovl_high_s16(n3));
        let nine_a_lo = vaddq_s32(vshlq_n_s32::<3>(a_lo), a_lo);
        let nine_a_hi = vaddq_s32(vshlq_n_s32::<3>(a_hi), a_hi);
        let delta_lo = vshrq_n_s32::<4>(vsubq_s32(vaddq_s32(nine_a_lo, d8), c_lo));
        let delta_hi = vshrq_n_s32::<4>(vsubq_s32(vaddq_s32(nine_a_hi, d8), c_hi));
        let delta = vcombine_s16(vmovn_s32(delta_lo), vmovn_s32(delta_hi));
        vst1q_s16(ptr.add(k0_off + col), vsubq_s16(cur, delta));
        col += 8;
    }
    while col < width {
        let p1 = *data.get_unchecked(km1_off + col) as i32;
        let n1 = *data.get_unchecked(kp1_off + col) as i32;
        let p3 = *data.get_unchecked(km3_off + col) as i32;
        let n3 = *data.get_unchecked(kp3_off + col) as i32;
        *data.get_unchecked_mut(k0_off + col) =
            pred_inner_fwd(*data.get_unchecked(k0_off + col) as i32, p1, n1, p3, n3) as i16;
        col += 1;
    }
}

/// NEON col-pass lift for s=1: one even row, 8 consecutive columns per iteration.
///
/// State slices (`prev3`, `prev1`, `next1`) are i16 (values bounded by i16 after
/// predict).  Performs:
///   data[k0+col] += ((9*(p1+n1) - (p3+n3) + 16) >> 5)
/// then advances state: prev3 ← prev1, prev1 ← next1, next1 ← n3.
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn, clippy::too_many_arguments)]
#[target_feature(enable = "neon")]
unsafe fn forward_col_lift_neon_row(
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
    let mut col = 0usize;
    while col + 8 <= width {
        let p3_s = vld1q_s16(p3p.add(col) as *const i16);
        let p1_s = vld1q_s16(p1p.add(col) as *const i16);
        let n1_s = vld1q_s16(n1p.add(col) as *const i16);
        let n3_s = if has_n3 {
            vld1q_s16(ptr.add(n3_off + col) as *const i16)
        } else {
            vdupq_n_s16(0)
        };
        let cur_s = vld1q_s16(ptr.add(k0_off + col) as *const i16);
        let a_lo = vaddq_s32(vmovl_s16(vget_low_s16(p1_s)), vmovl_s16(vget_low_s16(n1_s)));
        let a_hi = vaddq_s32(vmovl_high_s16(p1_s), vmovl_high_s16(n1_s));
        let c_lo = vaddq_s32(vmovl_s16(vget_low_s16(p3_s)), vmovl_s16(vget_low_s16(n3_s)));
        let c_hi = vaddq_s32(vmovl_high_s16(p3_s), vmovl_high_s16(n3_s));
        let nine_a_lo = vaddq_s32(vshlq_n_s32::<3>(a_lo), a_lo);
        let nine_a_hi = vaddq_s32(vshlq_n_s32::<3>(a_hi), a_hi);
        let delta_lo = vshrq_n_s32::<5>(vsubq_s32(vaddq_s32(nine_a_lo, d16), c_lo));
        let delta_hi = vshrq_n_s32::<5>(vsubq_s32(vaddq_s32(nine_a_hi, d16), c_hi));
        let delta_s = vcombine_s16(vmovn_s32(delta_lo), vmovn_s32(delta_hi));
        vst1q_s16(ptr.add(k0_off + col), vaddq_s16(cur_s, delta_s));
        // advance state
        vst1q_s16(p3p.add(col), p1_s);
        vst1q_s16(p1p.add(col), n1_s);
        vst1q_s16(n1p.add(col), n3_s);
        col += 8;
    }
    // scalar tail
    while col < width {
        let p3 = *prev3.get_unchecked(col) as i32;
        let p1 = *prev1.get_unchecked(col) as i32;
        let n1 = *next1.get_unchecked(col) as i32;
        let n3 = if has_n3 {
            *data.get_unchecked(n3_off + col) as i32
        } else {
            0
        };
        *data.get_unchecked_mut(k0_off + col) =
            lift(*data.get_unchecked(k0_off + col) as i32, p1, n1, p3, n3) as i16;
        *prev3.get_unchecked_mut(col) = p1 as i16;
        *prev1.get_unchecked_mut(col) = n1 as i16;
        *next1.get_unchecked_mut(col) = n3 as i16;
        col += 1;
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
            #[cfg(target_arch = "aarch64")]
            if s == 1 {
                #[allow(unsafe_code)]
                unsafe {
                    forward_col_predict_neon(
                        data, km3_off, km1_off, k0_off, kp1_off, kp3_off, width,
                    );
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
    // AArch64 NEON path at s=1: i16 state, 8 columns/iter
    #[cfg(target_arch = "aarch64")]
    if s == 1 {
        let mut prev3: Vec<i16> = vec![0i16; width];
        let mut prev1: Vec<i16> = vec![0i16; width];
        let mut next1: Vec<i16> = if kmax >= 1 {
            data[stride..stride + width].to_vec()
        } else {
            vec![0i16; width]
        };
        let mut k = 0usize;
        while k <= kmax {
            let k0_off = k * stride;
            let has_n3 = k + 3 <= kmax;
            let n3_off = if has_n3 { (k + 3) * stride } else { 0 };
            #[allow(unsafe_code)]
            unsafe {
                forward_col_lift_neon_row(
                    data, k0_off, n3_off, has_n3, &mut prev3, &mut prev1, &mut next1, width,
                );
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

// ---- NEON helpers for preliminary_flag_computation --------------------------

/// NEON-vectorized band≠0 bucket flag update for the encoder.
///
/// Reads 16 i32 reconstruction values at `base`, writes 16 u8 flags (UNK or
/// ACTIVE), and returns the bitwise-OR of all written flags.
///
/// Mirrors `prelim_flags_bucket_neon` in iw44_new but uses 4 × `vld1q_s32`
/// (i32 input) instead of 2 × `vld1q_s16` (i16 input).
#[cfg(all(feature = "std", target_arch = "aarch64"))]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "neon")]
unsafe fn prelim_flags_bucket_enc_neon(
    recon: &[i32; 1024],
    base: usize,
    bucket: &mut [u8; 16],
) -> u8 {
    use core::arch::aarch64::*;
    let ptr = recon.as_ptr().add(base);
    let c0 = vld1q_s32(ptr);
    let c1 = vld1q_s32(ptr.add(4));
    let c2 = vld1q_s32(ptr.add(8));
    let c3 = vld1q_s32(ptr.add(12));
    // eq: 0xFFFFFFFF where coef == 0, 0x00000000 where coef != 0
    let zero32 = vdupq_n_s32(0);
    let eq0 = vceqq_s32(c0, zero32); // uint32x4_t
    let eq1 = vceqq_s32(c1, zero32);
    let eq2 = vceqq_s32(c2, zero32);
    let eq3 = vceqq_s32(c3, zero32);
    // Narrow u32x4 → u16x4 → u8x8 in two steps (low bytes: 0xFF or 0x00)
    let n01 = vcombine_u16(vmovn_u32(eq0), vmovn_u32(eq1)); // uint16x8_t
    let n23 = vcombine_u16(vmovn_u32(eq2), vmovn_u32(eq3));
    let is_zero = vcombine_u8(vmovn_u16(n01), vmovn_u16(n23)); // 0xFF where ==0
    let is_nonzero = vmvnq_u8(is_zero); // 0xFF where !=0
    // result = UNK(8) if zero, ACTIVE(2) if nonzero
    // = UNK ^ ((UNK ^ ACTIVE) & is_nonzero) = 8 ^ (10 & is_nonzero)
    let xv = vdupq_n_u8(10);
    let uv = vdupq_n_u8(8);
    let out = veorq_u8(uv, vandq_u8(xv, is_nonzero));
    vst1q_u8(bucket.as_mut_ptr(), out);
    // Horizontal OR of 16 lanes
    let lo = vget_low_u8(out);
    let hi = vget_high_u8(out);
    let v4 = vorr_u8(lo, hi);
    let v2 = vorr_u8(v4, vext_u8::<4>(v4, v4));
    let v1 = vorr_u8(v2, vext_u8::<2>(v2, v2));
    let v0 = vorr_u8(v1, vext_u8::<1>(v1, v1));
    vget_lane_u8::<0>(v0)
}

/// NEON-vectorized band-0 flag update for the encoder.
///
/// Like `prelim_flags_bucket_enc_neon` but only updates entries where the
/// existing flag is not ZERO (1) — matches the decoder's `prelim_flags_band0_neon`.
#[cfg(all(feature = "std", target_arch = "aarch64"))]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn)]
#[target_feature(enable = "neon")]
unsafe fn prelim_flags_band0_enc_neon(recon: &[i32; 1024], old_flags: &mut [u8; 16]) -> u8 {
    use core::arch::aarch64::*;
    // Load old flags; build mask: 0xFF where flag != ZERO(1), else 0x00
    let old_u8 = vld1q_u8(old_flags.as_ptr());
    let one_u8 = vdupq_n_u8(1);
    let is_zero_state = vceqq_u8(old_u8, one_u8); // 0xFF where old==ZERO
    let should_update = vmvnq_u8(is_zero_state); // 0xFF where should update
    // Load 16 i32 reconstruction values for band 0 (indices 0..16)
    let ptr = recon.as_ptr();
    let c0 = vld1q_s32(ptr);
    let c1 = vld1q_s32(ptr.add(4));
    let c2 = vld1q_s32(ptr.add(8));
    let c3 = vld1q_s32(ptr.add(12));
    let zero32 = vdupq_n_s32(0);
    let eq0 = vceqq_s32(c0, zero32);
    let eq1 = vceqq_s32(c1, zero32);
    let eq2 = vceqq_s32(c2, zero32);
    let eq3 = vceqq_s32(c3, zero32);
    let n01 = vcombine_u16(vmovn_u32(eq0), vmovn_u32(eq1));
    let n23 = vcombine_u16(vmovn_u32(eq2), vmovn_u32(eq3));
    let is_zero = vcombine_u8(vmovn_u16(n01), vmovn_u16(n23));
    let is_nonzero = vmvnq_u8(is_zero);
    let xv = vdupq_n_u8(10);
    let uv = vdupq_n_u8(8);
    let new_flags = veorq_u8(uv, vandq_u8(xv, is_nonzero)); // UNK or ACTIVE
    // Blend: where should_update==0xFF take new_flags, else keep old_u8
    let out = vbslq_u8(should_update, new_flags, old_u8);
    vst1q_u8(old_flags.as_mut_ptr(), out);
    // Horizontal OR
    let lo = vget_low_u8(out);
    let hi = vget_high_u8(out);
    let v4 = vorr_u8(lo, hi);
    let v2 = vorr_u8(v4, vext_u8::<4>(v4, v4));
    let v1 = vorr_u8(v2, vext_u8::<2>(v2, v2));
    let v0 = vorr_u8(v1, vext_u8::<1>(v1, v1));
    vget_lane_u8::<0>(v0)
}

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
    #[allow(unsafe_code)]
    fn gather(&mut self, plane: &[i16], stride: usize) {
        // Safety invariant: `stride` = block_cols*32, `plane.len()` = stride * block_rows*32.
        // For any r < block_rows, c < block_cols, i < 1024:
        //   row  = ZIGZAG_ROW[i]  (∈ [0,31]) + r*32  ≤ block_rows*32 - 1
        //   col  = ZIGZAG_COL[i]  (∈ [0,31]) + c*32  ≤ stride - 1
        //   idx  = row * stride + col ≤ plane.len() - 1
        // The idx-in-bounds check is therefore always true; use get_unchecked to
        // eliminate the dead branch from the inner loop.
        let block_rows = self.blocks.len() / self.block_cols;
        for r in 0..block_rows {
            for c in 0..self.block_cols {
                let block = &mut self.blocks[r * self.block_cols + c];
                let row_base = r << 5;
                let col_base = c << 5;
                for (i, dst) in block.iter_mut().enumerate() {
                    let row = unsafe { *ZIGZAG_ROW.get_unchecked(i) } as usize + row_base;
                    let col = unsafe { *ZIGZAG_COL.get_unchecked(i) } as usize + col_base;
                    let idx = row * stride + col;
                    // SAFETY: see invariant above — idx < plane.len() always holds.
                    *dst = unsafe { *plane.get_unchecked(idx) };
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
                let base = j << 4;
                #[cfg(target_arch = "aarch64")]
                // SAFETY: NEON always available on aarch64; base+16 <= 1024 (max j=63).
                #[allow(unsafe_code)]
                let bstatetmp = unsafe {
                    prelim_flags_bucket_enc_neon(
                        &self.recon[block_idx],
                        base,
                        &mut self.coeffstate[boff],
                    )
                };
                #[cfg(not(target_arch = "aarch64"))]
                let bstatetmp = {
                    let mut b = 0u8;
                    for k in 0..16 {
                        let f = if self.recon[block_idx][base + k] == 0 {
                            UNK
                        } else {
                            ACTIVE
                        };
                        self.coeffstate[boff][k] = f;
                        b |= f;
                    }
                    b
                };
                self.bucketstate[boff] = bstatetmp;
                self.bbstate |= bstatetmp;
            }
        } else {
            // Band 0: coeffstate[0] is pre-initialized by is_null_slice
            #[cfg(target_arch = "aarch64")]
            // SAFETY: NEON always available on aarch64; recon[0..16] is valid.
            #[allow(unsafe_code)]
            let bstatetmp = unsafe {
                prelim_flags_band0_enc_neon(&self.recon[block_idx], &mut self.coeffstate[0])
            };
            #[cfg(not(target_arch = "aarch64"))]
            let bstatetmp = {
                let mut b = 0u8;
                for k in 0..16 {
                    if self.coeffstate[0][k] != ZERO {
                        self.coeffstate[0][k] = if self.recon[block_idx][k] == 0 {
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

    fn encode_slice(&mut self, zp: &mut ZpEncoder) {
        if !self.is_null_slice() {
            for block_idx in 0..self.blocks.len() {
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
                // #iw44-1: predict activation with the SAME threshold the activation
                // pass actually uses (|V| > 11s/16, line ~1096). The old `s/2` is too
                // low: coefficients in (s/2, 11s/16] were announced as block/bucket-NEW
                // but never activated, emitting wasted ZP bits with no reconstruction
                // change. Matching the gate removes that overhead (PSNR-neutral).
                if v > (s * 11 / 16).max(1) {
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
                v > (s * 11 / 16).max(1) // #iw44-1: match the real activation gate (see any_unk_activates)
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
            // Delay chroma to slice 10, matching DjVuLibre's c44 default
            // (`crcbdelay = 10`). The luma refines for 10 slices before any Cb/Cr
            // is coded; this trims ~10–18 % off colour BG44 with no perceptible
            // chroma loss, and aligns our stream with the standard c44 convention.
            chroma_delay: 10,
            // DjVuLibre interop: our `chroma_half` encodes Cb/Cr at half *spatial*
            // resolution (a smaller plane), but DjVuLibre's IWPixmap decoder always
            // builds full-resolution chroma maps and reads full-resolution chroma
            // slices — so a half-resolution stream leaves it short of bits ("Unexpected
            // End Of File"), making our colour DjVu unreadable in ddjvu/DjVuLibre.
            // Default to full-resolution chroma for interoperability. (Round-trips
            // through our own decoder either way.)
            chroma_half: false,
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
    //
    // Single pass: compute Y for every pixel; if chroma_half, accumulate 2×2
    // box-filter for Cb/Cr (matches DjVuLibre's chroma downsampling).
    if opts.chroma_half {
        // Single pass: fill Y and accumulate 2×2 box-filter chroma (matches
        // DjVuLibre's c44 downsampling).  Each chroma output cell receives
        // contributions from up to 4 source pixels with weight 16 each, so
        // a full 2×2 block sums to 64 — the same scale used by the 1:1 path.
        for row in 0..h {
            let wavelet_row = h - 1 - row;
            for col in 0..w {
                let (r, g, b) = pixmap.get_rgb(col as u32, row as u32);
                let (y, cb, cr) = rgb_to_ycbcr(r, g, b);
                y_plane[wavelet_row * stride + col] = (y as i32 * 64) as i16;
                let cc = col / 2;
                let cr_row = wavelet_row / 2;
                cb_plane[cr_row * c_stride + cc] += (cb as i32 * 16) as i16;
                cr_plane[cr_row * c_stride + cc] += (cr as i32 * 16) as i16;
            }
        }
    } else {
        for row in 0..h {
            let wavelet_row = h - 1 - row;
            for col in 0..w {
                let (r, g, b) = pixmap.get_rgb(col as u32, row as u32);
                let (y, cb, cr) = rgb_to_ycbcr(r, g, b);
                y_plane[wavelet_row * stride + col] = (y as i32 * 64) as i16;
                cb_plane[wavelet_row * c_stride + col] = (cb as i32 * 64) as i16;
                cr_plane[wavelet_row * c_stride + col] = (cr as i32 * 64) as i16;
            }
        }
    }

    // Transform + gather all three planes.  Each plane is independent, so with
    // the `parallel` feature they run concurrently on rayon threads, reducing
    // wall-time from Y+Cb+Cr sequential to max(Y, Cb, Cr).
    //
    // The threshold (512×512 = 262 144 px) ensures rayon overhead (~30 µs) is
    // only paid when the work per plane is large enough to justify it.  Below
    // that threshold sequential is faster (verified on M1 with 192×256 images).
    #[cfg(feature = "parallel")]
    let (mut y_enc, mut cb_enc, mut cr_enc) = if w * h > 512 * 512 {
        use rayon::join;
        let (ye, (cbe, cre)) = join(
            move || {
                forward_wavelet_transform(&mut y_plane, w, h, stride);
                let mut enc = PlaneEncoder::new(w, h);
                enc.gather(&y_plane, stride);
                enc
            },
            move || {
                join(
                    move || {
                        forward_wavelet_transform(&mut cb_plane, cw, ch, c_stride);
                        let mut enc = PlaneEncoder::new(cw, ch);
                        enc.gather(&cb_plane, c_stride);
                        enc
                    },
                    move || {
                        forward_wavelet_transform(&mut cr_plane, cw, ch, c_stride);
                        let mut enc = PlaneEncoder::new(cw, ch);
                        enc.gather(&cr_plane, c_stride);
                        enc
                    },
                )
            },
        );
        (ye, cbe, cre)
    } else {
        forward_wavelet_transform(&mut y_plane, w, h, stride);
        forward_wavelet_transform(&mut cb_plane, cw, ch, c_stride);
        forward_wavelet_transform(&mut cr_plane, cw, ch, c_stride);
        let mut y_enc = PlaneEncoder::new(w, h);
        let mut cb_enc = PlaneEncoder::new(cw, ch);
        let mut cr_enc = PlaneEncoder::new(cw, ch);
        y_enc.gather(&y_plane, stride);
        cb_enc.gather(&cb_plane, c_stride);
        cr_enc.gather(&cr_plane, c_stride);
        (y_enc, cb_enc, cr_enc)
    };
    #[cfg(not(feature = "parallel"))]
    let (mut y_enc, mut cb_enc, mut cr_enc) = {
        forward_wavelet_transform(&mut y_plane, w, h, stride);
        forward_wavelet_transform(&mut cb_plane, cw, ch, c_stride);
        forward_wavelet_transform(&mut cr_plane, cw, ch, c_stride);
        let mut y_enc = PlaneEncoder::new(w, h);
        let mut cb_enc = PlaneEncoder::new(cw, ch);
        let mut cr_enc = PlaneEncoder::new(cw, ch);
        y_enc.gather(&y_plane, stride);
        cb_enc.gather(&cb_plane, c_stride);
        cr_enc.gather(&cr_plane, c_stride);
        (y_enc, cb_enc, cr_enc)
    };

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
            // Major version byte. DjVuLibre's IWPixmap::decode_chunk rejects the
            // stream with "incompatible IWCodec" unless `(major & 0x7f) == 1`
            // (IWCODEC_MAJOR). We previously emitted 0 → our colour output was
            // unreadable by ddjvu/DjVuLibre. Bit 7 carries our grayscale flag (the
            // decoder reads `is_grayscale = major >> 7`); real DjVuLibre colour BG44
            // chunks use 0x01, so colour = 0x01, grayscale = 0x81.
            let majver: u8 = if is_color { 0x01 } else { 0x81 };
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
mod loss_diagnostics {
    //! IW44 reconstruction-loss property guards (originally issue #320).
    //!
    //! These began as an investigation localizing where a continuous-tone page's
    //! BG44 detail is lost during re-encode, among four candidate stages —
    //! forward wavelet transform, quantization threshold schedule, coefficient
    //! state transitions, and reconstruction tracking. #320 is closed; what
    //! remains worth guarding are the two structural properties the investigation
    //! established, now exercised on a deterministic synthetic continuous-tone
    //! image (a 1/f sum of octave sinusoids) rather than a decoded corpus page —
    //! the corpus path needed the monolith's `DjVuDocument` container parser,
    //! which is not part of this codec crate.
    //!
    //!  * **The forward transform is exact.** The production i16 forward
    //!    transform reproduces a full-precision i32 reference *exactly* (zero
    //!    diverging coefficients) and never overflows i16. This is an
    //!    input-agnostic correctness guard over the SIMD/scalar transform paths.
    //!    See `forward_transform_is_lossless`.
    //!
    //!  * **The loss is the progressive quantization / slice budget.** Per-band
    //!    reconstruction residual (true `blocks` vs the decoder-mirrored
    //!    `recon`) grows with band frequency: the coarse bands reconstruct well
    //!    while the finest band is never activated within the default 100-slice
    //!    budget (its `recon` stays all-zero). Because the low-frequency bands
    //!    *do* reconstruct, the coefficient state transitions and reconstruction
    //!    tracking are working — the diverging stage is the quantization
    //!    threshold schedule. See `loss_concentrates_in_high_frequency_bands`.
    //!
    //! These are diagnostics only; they do not change encoder behavior.

    use super::*;
    use djvu_pixmap::Pixmap;

    // ---- Full-precision i32 reference forward transform ----------------------
    //
    // Identical lifting/predict arithmetic to the production transform but in
    // i32 with no per-step truncation, so any difference vs production isolates
    // i16 rounding/overflow in the forward stage. (An earlier version of this
    // reference had a bug — it ran the column pass over every column instead of
    // the s·ℤ sublattice — which manufactured a spurious "divergence"; the
    // sublattice stepping below matches production's `forward_col_pass`.)

    fn ref_lift(cur: i32, p1: i32, n1: i32, p3: i32, n3: i32) -> i32 {
        let a = p1 + n1;
        let c = p3 + n3;
        cur + (((a << 3) + a - c + 16) >> 5)
    }
    fn ref_pred_inner(cur: i32, p1: i32, n1: i32, p3: i32, n3: i32) -> i32 {
        let a = p1 + n1;
        cur - (((a << 3) + a - (p3 + n3) + 8) >> 4)
    }
    fn ref_pred_avg(cur: i32, p: i32, n: i32) -> i32 {
        cur - ((p + n + 1) >> 1)
    }

    fn ref_row_pass(data: &mut [i32], width: usize, height: usize, stride: usize, s: usize) {
        let sd = s.trailing_zeros() as usize;
        let kmax = (width - 1) >> sd;
        let border = kmax.saturating_sub(3);
        for row in (0..height).step_by(s) {
            let off = row * stride;
            if kmax >= 1 {
                let p = data[off];
                let idx1 = off + (1 << sd);
                if kmax >= 2 {
                    let n = data[off + (2 << sd)];
                    data[idx1] = ref_pred_avg(data[idx1], p, n);
                } else {
                    data[idx1] -= p;
                }
                let mut k = 3usize;
                while k <= border {
                    let p1 = data[off + ((k - 1) << sd)];
                    let n1 = data[off + ((k + 1) << sd)];
                    let p3 = data[off + ((k - 3) << sd)];
                    let n3 = if k + 3 <= kmax {
                        data[off + ((k + 3) << sd)]
                    } else {
                        0
                    };
                    data[off + (k << sd)] = ref_pred_inner(data[off + (k << sd)], p1, n1, p3, n3);
                    k += 2;
                }
                while k <= kmax {
                    let p = data[off + ((k - 1) << sd)];
                    if k < kmax {
                        let n = data[off + ((k + 1) << sd)];
                        data[off + (k << sd)] = ref_pred_avg(data[off + (k << sd)], p, n);
                    } else {
                        data[off + (k << sd)] -= p;
                    }
                    k += 2;
                }
            }
            let mut prev3 = 0i32;
            let mut prev1 = 0i32;
            let mut next1 = if kmax >= 1 { data[off + (1 << sd)] } else { 0 };
            let mut k = 0usize;
            while k <= kmax {
                let n3 = if k + 3 <= kmax {
                    data[off + ((k + 3) << sd)]
                } else {
                    0
                };
                let idx = off + (k << sd);
                data[idx] = ref_lift(data[idx], prev1, next1, prev3, n3);
                prev3 = prev1;
                prev1 = next1;
                next1 = n3;
                k += 2;
            }
        }
    }

    fn ref_col_pass(data: &mut [i32], width: usize, height: usize, stride: usize, s: usize) {
        let sd = s.trailing_zeros() as usize;
        let kmax = (height - 1) >> sd;
        let border = kmax.saturating_sub(3);
        // Multiresolution: only the s·ℤ column sublattice (previous LL subband).
        if kmax >= 1 {
            let k1_off = (1 << sd) * stride;
            if kmax >= 2 {
                let kp1_off = (2 << sd) * stride;
                for col in (0..width).step_by(s) {
                    let p = data[col];
                    let n = data[kp1_off + col];
                    data[k1_off + col] = ref_pred_avg(data[k1_off + col], p, n);
                }
            } else {
                for col in (0..width).step_by(s) {
                    let p = data[col];
                    data[k1_off + col] -= p;
                }
            }
            let mut k = 3usize;
            while k <= border {
                let km3 = ((k - 3) << sd) * stride;
                let km1 = ((k - 1) << sd) * stride;
                let k0 = (k << sd) * stride;
                let kp1 = ((k + 1) << sd) * stride;
                let kp3 = ((k + 3) << sd) * stride;
                for col in (0..width).step_by(s) {
                    data[k0 + col] = ref_pred_inner(
                        data[k0 + col],
                        data[km1 + col],
                        data[kp1 + col],
                        data[km3 + col],
                        data[kp3 + col],
                    );
                }
                k += 2;
            }
            while k <= kmax {
                let km1 = ((k - 1) << sd) * stride;
                let k0 = (k << sd) * stride;
                if k < kmax {
                    let kp1 = ((k + 1) << sd) * stride;
                    for col in (0..width).step_by(s) {
                        data[k0 + col] =
                            ref_pred_avg(data[k0 + col], data[km1 + col], data[kp1 + col]);
                    }
                } else {
                    for col in (0..width).step_by(s) {
                        data[k0 + col] -= data[km1 + col];
                    }
                }
                k += 2;
            }
        }
        let num_cols = width.div_ceil(s);
        let mut prev3 = vec![0i32; num_cols];
        let mut prev1 = vec![0i32; num_cols];
        let mut next1: Vec<i32> = if kmax >= 1 {
            let off = (1 << sd) * stride;
            (0..width).step_by(s).map(|c| data[off + c]).collect()
        } else {
            vec![0i32; num_cols]
        };
        let mut k = 0usize;
        while k <= kmax {
            let k0 = (k << sd) * stride;
            let has_n3 = k + 3 <= kmax;
            let n3_off = if has_n3 { ((k + 3) << sd) * stride } else { 0 };
            for (ci, col) in (0..width).step_by(s).enumerate() {
                let n3 = if has_n3 { data[n3_off + col] } else { 0 };
                let idx = k0 + col;
                data[idx] = ref_lift(data[idx], prev1[ci], next1[ci], prev3[ci], n3);
                prev3[ci] = prev1[ci];
                prev1[ci] = next1[ci];
                next1[ci] = n3;
            }
            k += 2;
        }
    }

    /// Full-precision i32 forward transform; returns the largest magnitude that
    /// appears at any pass boundary (so callers can check i16 headroom).
    fn ref_transform(data: &mut [i32], width: usize, height: usize, stride: usize) -> i32 {
        let mut max_intermediate = 0i32;
        let mut s = 1usize;
        while s <= 16 {
            ref_row_pass(data, width, height, stride, s);
            for v in data.iter() {
                max_intermediate = max_intermediate.max(v.abs());
            }
            ref_col_pass(data, width, height, stride, s);
            for v in data.iter() {
                max_intermediate = max_intermediate.max(v.abs());
            }
            s <<= 1;
        }
        max_intermediate
    }

    // ---- Luma plane / band helpers -------------------------------------------

    /// Build the luma (Y) plane exactly like `encode_iw44_color`'s Y path.
    fn build_y_plane(px: &Pixmap) -> (Vec<i16>, usize, usize, usize) {
        let w = px.width as usize;
        let h = px.height as usize;
        let stride = w.div_ceil(32) * 32;
        let plane_h = h.div_ceil(32) * 32;
        let mut y_plane = vec![0i16; stride * plane_h];
        for row in 0..h {
            let wavelet_row = h - 1 - row;
            for col in 0..w {
                let (r, g, b) = px.get_rgb(col as u32, row as u32);
                let (y, _cb, _cr) = rgb_to_ycbcr(r, g, b);
                y_plane[wavelet_row * stride + col] = (y as i32 * 64) as i16;
            }
        }
        (y_plane, w, h, stride)
    }

    /// Deterministic continuous-tone test image with energy at every spatial
    /// scale: a 1/f sum of octave sinusoids (amplitude halves per octave, the
    /// highest octave at the 2-px Nyquist limit). This reproduces the structural
    /// properties of a photographic background — strong, easily-reconstructed
    /// low-frequency content plus genuine but progressively starved
    /// high-frequency detail — without needing a decoded corpus page. Grayscale
    /// (R=G=B) so the luma plane is well defined and chroma is ~0.
    fn synthetic_background(w: u32, h: u32) -> Pixmap {
        use core::f64::consts::PI;
        let mut px = Pixmap::white(w, h);
        for y in 0..h {
            for x in 0..w {
                let fx = x as f64 / w as f64;
                let fy = y as f64 / h as f64;
                let mut v = 128.0f64;
                let mut amp = 100.0f64;
                // Octaves 0..8: freq doubles, amplitude halves (≈1/f spectrum).
                for k in 0..8u32 {
                    let f = (1u32 << k) as f64;
                    v += amp * (2.0 * PI * f * fx + 0.5).sin() * (2.0 * PI * f * fy).cos();
                    amp *= 0.5;
                }
                let p = v.round().clamp(0.0, 255.0) as u8;
                px.set_rgb(x, y, p, p, p);
            }
        }
        px
    }

    /// Contiguous zigzag-coefficient index range covered by IW44 band `band`
    /// (band 0 → the first 16 coeffs; bands 1-9 → `BAND_BUCKETS[band]` × 16).
    fn band_idx_range(band: usize) -> (usize, usize) {
        if band == 0 {
            (0, 16)
        } else {
            let (from, to) = BAND_BUCKETS[band];
            (from * 16, (to + 1) * 16)
        }
    }

    /// Forward-transform + gather a luma PlaneEncoder, like `encode_iw44_color`.
    fn y_plane_encoder(px: &Pixmap) -> PlaneEncoder {
        let (mut y_plane, w, h, stride) = build_y_plane(px);
        forward_wavelet_transform(&mut y_plane, w, h, stride);
        let mut enc = PlaneEncoder::new(w, h);
        enc.gather(&y_plane, stride);
        enc
    }

    /// Per-band (energy = Σ|blocks|, residual = Σ|blocks − recon|).
    fn band_stats(enc: &PlaneEncoder) -> [(i64, i64); 10] {
        let mut out = [(0i64, 0i64); 10];
        for (band, slot) in out.iter_mut().enumerate() {
            let (lo, hi) = band_idx_range(band);
            let mut energy = 0i64;
            let mut resid = 0i64;
            for blk in 0..enc.blocks.len() {
                for idx in lo..hi {
                    let t = enc.blocks[blk][idx] as i64;
                    let r = enc.recon[blk][idx] as i64;
                    energy += t.abs();
                    resid += (t - r).abs();
                }
            }
            *slot = (energy, resid);
        }
        out
    }

    /// Relative per-band reconstruction residual in [0, 1].
    fn band_rel(stats: &[(i64, i64); 10]) -> [f64; 10] {
        let mut rel = [0.0f64; 10];
        for (i, &(energy, resid)) in stats.iter().enumerate() {
            rel[i] = if energy > 0 {
                resid as f64 / energy as f64
            } else {
                0.0
            };
        }
        rel
    }

    fn encode_all_slices(enc: &mut PlaneEncoder, total: usize) {
        let mut zp = ZpEncoder::new();
        for _ in 0..total {
            enc.encode_slice(&mut zp);
        }
        let _ = zp.finish();
    }

    // ---- Tests ---------------------------------------------------------------

    /// The forward wavelet transform is lossless for real luma backgrounds: it
    /// matches a full-precision i32 reference exactly and never overflows i16.
    /// This rules the forward transform out as a source of reconstruction loss.
    #[test]
    fn forward_transform_is_lossless() {
        for (iw, ih) in [(256u32, 320u32), (320, 256)] {
            let px = synthetic_background(iw, ih);
            let (mut i16_plane, w, h, stride) = build_y_plane(&px);
            let mut i32_plane: Vec<i32> = i16_plane.iter().map(|&v| v as i32).collect();

            forward_wavelet_transform(&mut i16_plane, w, h, stride);
            let max_intermediate = ref_transform(&mut i32_plane, w, h, stride);

            let mut diverged = 0usize;
            for r in 0..h {
                for c in 0..w {
                    let idx = r * stride + c;
                    if i16_plane[idx] as i32 != i32_plane[idx] {
                        diverged += 1;
                    }
                }
            }
            assert_eq!(
                diverged, 0,
                "{iw}x{ih}: i16 forward transform must equal the full-precision \
                 reference (diverged coefficients indicate a transform/SIMD bug)"
            );
            assert!(
                max_intermediate <= i16::MAX as i32,
                "{iw}x{ih}: forward transform overflowed i16 \
                 (max intermediate magnitude {max_intermediate} > 32767)"
            );
        }
    }

    /// The reconstruction loss concentrates in the high-frequency bands: with
    /// the default 100-slice budget the coarse bands reconstruct well while the
    /// finest band is never activated (its `recon` stays all-zero). The
    /// monotone-with-frequency residual isolates the loss to the quantization
    /// threshold schedule — coefficient state transitions and reconstruction
    /// tracking work, since the coarse bands *do* reconstruct.
    #[test]
    fn loss_concentrates_in_high_frequency_bands() {
        for (iw, ih) in [(256u32, 320u32), (320, 256)] {
            let px = synthetic_background(iw, ih);
            let mut enc = y_plane_encoder(&px);
            encode_all_slices(&mut enc, 100);
            let stats = band_stats(&enc);
            let rel = band_rel(&stats);

            // Coarsest band reconstructs well (< 40% residual).
            assert!(
                rel[0] < 0.40,
                "{iw}x{ih}: band 0 relative residual {:.3} unexpectedly high \
                 (coarse band should reconstruct well)",
                rel[0]
            );
            // Finest band is essentially unreconstructed within budget — its
            // recon is at most a hair above all-zero (never meaningfully
            // activated): residual ≥ 99% of the band energy.
            assert!(
                rel[9] >= 0.99,
                "{iw}x{ih}: band 9 relative residual {:.4} (expected ≈ 1.0; the \
                 finest band should be starved by the slice budget)",
                rel[9]
            );
            // High-frequency bands lose far more than low-frequency bands.
            let low = (rel[0] + rel[1] + rel[2]) / 3.0;
            let high = (rel[7] + rel[8] + rel[9]) / 3.0;
            assert!(
                high > low + 0.25,
                "{iw}x{ih}: residual must grow with frequency \
                 (low-band mean {low:.3}, high-band mean {high:.3})"
            );
        }
    }

    /// Prints the full per-band energy/residual table for manual investigation.
    /// Run with: `cargo test --features=std --lib \
    /// loss_diagnostics::print_band_table -- --ignored --nocapture`.
    #[test]
    #[ignore = "diagnostic dump for #320; run with --ignored --nocapture"]
    fn print_band_table() {
        for (iw, ih) in [(256u32, 320u32), (320, 256)] {
            let px = synthetic_background(iw, ih);
            let mut enc = y_plane_encoder(&px);
            encode_all_slices(&mut enc, 100);
            let stats = band_stats(&enc);
            println!("synthetic {iw}x{ih}: per-band [energy resid rel%] after 100 slices");
            let (mut te, mut tr) = (0i64, 0i64);
            for (band, &(energy, resid)) in stats.iter().enumerate() {
                let (lo, hi) = band_idx_range(band);
                let rel = if energy > 0 {
                    100.0 * resid as f64 / energy as f64
                } else {
                    0.0
                };
                println!(
                    "  band {band} idx[{lo}..{hi}): energy={energy} resid={resid} rel={rel:.1}%"
                );
                te += energy;
                tr += resid;
            }
            let rel = if te > 0 {
                100.0 * tr as f64 / te as f64
            } else {
                0.0
            };
            println!("  TOTAL: energy={te} resid={tr} rel={rel:.1}%");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Iw44Image;
    use djvu_pixmap::{GrayPixmap, Pixmap};

    /// The default must encode full-resolution chroma. Our `chroma_half` stores
    /// Cb/Cr at half *spatial* resolution, which DjVuLibre's IWPixmap decoder (it
    /// builds full-resolution chroma maps) reads as too few bits → "Unexpected End
    /// Of File". Defaulting to half-chroma made our colour DjVu unreadable in
    /// ddjvu/DjVuLibre, so the interop default is full chroma. Locks that decision.
    #[test]
    fn default_chroma_is_full_resolution_for_interop() {
        assert!(
            !Iw44EncodeOptions::default().chroma_half,
            "default must be full-resolution chroma for DjVuLibre interop"
        );
    }

    /// The IW44 primary-chunk major-version byte must satisfy DjVuLibre's
    /// `(major & 0x7f) == 1` check (IWCODEC_MAJOR), or ddjvu rejects the stream
    /// with "incompatible IWCodec". Regression for the colour encode-interop bug.
    #[test]
    fn bg44_major_version_is_djvulibre_compatible() {
        let mut pm = Pixmap::white(32, 32);
        for y in 0..32 {
            for x in 0..32 {
                pm.set_rgb(x, y, (x * 8) as u8, (y * 8) as u8, 128);
            }
        }
        let chunks = encode_iw44_color(&pm, &Iw44EncodeOptions::default());
        // chunk[0] = serial, slices, major, minor, ...
        let major = chunks[0][2];
        assert_eq!(
            major & 0x7f,
            1,
            "major version must be 1 (got 0x{major:02x})"
        );
        // our colour flag lives in bit 7; a colour image must be decodable as colour
        assert_eq!(major >> 7, 0, "colour chunk should have bit7 clear (0x01)");
    }

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

    // Lines 426-432: tail loop in forward_row_pass for widths not a multiple of 8.
    #[test]
    fn encode_odd_width_roundtrips() {
        // width=10 means the main 8-wide loop runs once (cols 0-7), then the
        // tail loop runs twice (cols 8-9), covering lines 426-432.
        let src = make_pixmap(10, 8, |x, y| ((x * 25) as u8, (y * 32) as u8, 128));
        let opts = Iw44EncodeOptions::default();
        let chunks = encode_iw44_color(&src, &opts);
        let decoded = decode_color(&chunks);
        assert_eq!((decoded.width, decoded.height), (10, 8));
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

    // Lines 1230, 1261-1268, 1441: chroma_half=false path in encode_iw44_color.
    #[test]
    fn encode_color_chroma_full_roundtrips() {
        let src = make_pixmap(32, 32, |x, y| ((x * 8) as u8, (y * 8) as u8, 128));
        let opts = Iw44EncodeOptions {
            chroma_half: false,
            ..Default::default()
        };
        let chunks = encode_iw44_color(&src, &opts);
        assert!(!chunks.is_empty());
        // delay_byte bit 7 should be set (0x80) for color + !chroma_half
        assert_eq!(chunks[0][8] & 0x80, 0x80, "delay_byte bit 7 must be set");
        let decoded = decode_color(&chunks);
        assert_eq!((decoded.width, decoded.height), (32, 32));
    }
}
