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
                    data[k0 + col] = ref_pred_avg(data[k0 + col], data[km1 + col], data[kp1 + col]);
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
    let (y_plane, w, h, stride) = build_y_plane(px);
    let mut plane = PlaneEncoder::new_plane(w, h);
    let flat = plane.as_flattened_mut();
    flat.copy_from_slice(&y_plane);
    forward_wavelet_transform(flat, w, h, stride);
    PlaneEncoder::from_plane(w, h, plane)
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
                let r = enc.recon[blk].coef(idx) as i64;
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

#[test]
fn unit_coefficient_activates_at_final_quant_step() {
    // At s=1, 11*s/16 truncates to zero, so a nonzero unit coefficient
    // must activate. Keeping a floor of one permanently strands it at zero.
    let mut enc = PlaneEncoder::new(32, 32);
    let coefficient = 48 * 16; // first coefficient in high-frequency band 9
    enc.blocks[0][coefficient] = 1;

    let mut zp_enc = ZpEncoder::new();
    for _ in 0..250 {
        enc.encode_slice(&mut zp_enc);
    }
    let mut zp_data = zp_enc.finish();
    zp_data.resize(zp_data.len().max(2), 0xff);

    let mut dec = crate::PlaneDecoder::new(32, 32);
    let mut zp_dec = djvu_zp::ZpDecoder::new(&zp_data).unwrap();
    for _ in 0..250 {
        dec.decode_slice(&mut zp_dec);
    }

    assert_eq!(enc.recon[0].coef(coefficient), 1);
    assert_eq!(dec.blocks[0].coef(coefficient), 1);
}

#[test]
fn coefficient_waits_for_its_significance_step() {
    // Band 9 first becomes usable at s=16384. A value of 12000 must wait
    // for s=8192 rather than activating at the decoder's 11s/16 boundary.
    let mut enc = PlaneEncoder::new(32, 32);
    let coefficient = 48 * 16;
    enc.blocks[0][coefficient] = 12_000;

    let mut zp_enc = ZpEncoder::new();
    for _ in 0..70 {
        enc.encode_slice(&mut zp_enc);
    }
    let mut zp_data = zp_enc.finish();
    zp_data.resize(zp_data.len().max(2), 0xff);

    let mut dec = crate::PlaneDecoder::new(32, 32);
    let mut zp_dec = djvu_zp::ZpDecoder::new(&zp_data).unwrap();
    for _ in 0..70 {
        dec.decode_slice(&mut zp_dec);
    }

    // s=8192 activation reconstruction: s + s/2 - s/8 = 11264.
    assert_eq!(enc.recon[0].coef(coefficient), 11_264);
    assert_eq!(dec.blocks[0].coef(coefficient), 11_264);
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
            println!("  band {band} idx[{lo}..{hi}): energy={energy} resid={resid} rel={rel:.1}%");
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
