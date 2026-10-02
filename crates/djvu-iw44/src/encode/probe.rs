//! Diagnostic-only encoder-side size attribution (IW44_ENTROPY_PROBE, round 51).
//!
//! Thread-local per-band counters: cumulative ZP output bytes attributed to
//! each of the 10 IW44 frequency bands (band 0 = DC/coarsest .. band 9 =
//! finest, see `BAND_BUCKETS`), plus per-band call/true counts for the four
//! bit categories the codec emits (block-band NEW, bucket NEW, coefficient
//! activation, coefficient refinement). This exists to answer "where do
//! textured-content bits concentrate" without guessing.
//!
//! Purely observational: reading `ZpEncoder::bytes_written()` and
//! incrementing a thread-local counter cannot change which bits get
//! encoded, so this feature — even enabled — never alters the emitted
//! stream (see `probe_does_not_change_output` in the test module below).
use std::cell::RefCell;

use djvu_pixmap::Pixmap;

/// Call count + true-count for one binary decision category.
#[derive(Clone, Copy, Default, Debug)]
pub struct BitCounter {
    pub calls: u64,
    pub true_count: u64,
}

impl BitCounter {
    fn record(&mut self, bit: bool) {
        self.calls += 1;
        if bit {
            self.true_count += 1;
        }
    }
}

/// Per-band accounting: bytes plus the four bit-category counters.
#[derive(Clone, Copy, Default, Debug)]
pub struct BandStats {
    /// Cumulative ZP output bytes attributed to slices of this band.
    pub bytes: u64,
    /// Block-band NEW bit (`block_band_encoding_pass`).
    pub block_band: BitCounter,
    /// Per-bucket NEW bit (`bucket_encoding_pass`).
    pub bucket_new: BitCounter,
    /// Per-coefficient activation bit (`newly_active_encoding_pass`).
    pub activate: BitCounter,
    /// Per-coefficient refinement bit (`previously_active_encoding_pass`).
    pub refine: BitCounter,
}

thread_local! {
    static STATS: RefCell<[BandStats; 10]> = RefCell::new([BandStats::default(); 10]);
}

/// Reset all counters to zero. Call before the encode you want to measure.
pub fn reset() {
    STATS.with(|s| *s.borrow_mut() = [BandStats::default(); 10]);
}

/// Snapshot the current per-band counters (index = band 0..=9).
pub fn snapshot() -> [BandStats; 10] {
    STATS.with(|s| *s.borrow())
}

pub(crate) fn add_bytes(band: usize, delta: u64) {
    STATS.with(|s| s.borrow_mut()[band].bytes += delta);
}
pub(crate) fn record_block_band(band: usize, bit: bool) {
    STATS.with(|s| s.borrow_mut()[band].block_band.record(bit));
}
pub(crate) fn record_bucket_new(band: usize, bit: bool) {
    STATS.with(|s| s.borrow_mut()[band].bucket_new.record(bit));
}
pub(crate) fn record_activate(band: usize, bit: bool) {
    STATS.with(|s| s.borrow_mut()[band].activate.record(bit));
}
pub(crate) fn record_refine(band: usize, bit: bool) {
    STATS.with(|s| s.borrow_mut()[band].refine.record(bit));
}

/// Per-band coefficient delta between the production forward transform and
/// the DjVuLibre-compatible scalar reference used by #578.
#[derive(Clone, Copy, Default, Debug)]
pub struct ForwardBandDiff {
    pub samples: u64,
    pub changed: u64,
    pub sum_abs_delta: u64,
    pub max_abs_delta: u16,
}

impl ForwardBandDiff {
    fn record(&mut self, delta: i32) {
        self.samples += 1;
        let abs_delta = delta.unsigned_abs() as u64;
        if abs_delta != 0 {
            self.changed += 1;
            self.sum_abs_delta += abs_delta;
            self.max_abs_delta = self.max_abs_delta.max(abs_delta as u16);
        }
    }
}

/// One Y/Cb/Cr plane's coefficient delta summary.
#[derive(Clone, Copy, Debug)]
pub struct ForwardPlaneDiff {
    pub plane: &'static str,
    pub samples: u64,
    pub changed: u64,
    pub sum_abs_delta: u64,
    pub max_abs_delta: u16,
    pub bands: [ForwardBandDiff; 10],
}

impl ForwardPlaneDiff {
    fn new(plane: &'static str) -> Self {
        Self {
            plane,
            samples: 0,
            changed: 0,
            sum_abs_delta: 0,
            max_abs_delta: 0,
            bands: [ForwardBandDiff::default(); 10],
        }
    }

    fn record(&mut self, band: usize, delta: i32) {
        self.samples += 1;
        let abs_delta = delta.unsigned_abs() as u64;
        if abs_delta != 0 {
            self.changed += 1;
            self.sum_abs_delta += abs_delta;
            self.max_abs_delta = self.max_abs_delta.max(abs_delta as u16);
        }
        self.bands[band].record(delta);
    }
}

/// Compare the production forward IW44 DWT against the DjVuLibre scalar
/// filter schedule on the same encoder input planes.
///
/// The reference keeps the DjVuLibre row/column boundary schedule separate
/// from the production implementation so #578 can validate whether transform
/// coefficient values explain the c44 size gap. It does not alter encoding.
pub fn forward_transform_diff_color(pixmap: &Pixmap) -> [ForwardPlaneDiff; 3] {
    let width = pixmap.width as usize;
    let height = pixmap.height as usize;
    let stride = width.div_ceil(32) * 32;
    let plane_height = height.div_ceil(32) * 32;

    let mut y_plane = vec![0i16; stride * plane_height];
    let mut cb_plane = vec![0i16; stride * plane_height];
    let mut cr_plane = vec![0i16; stride * plane_height];

    for row in 0..height {
        let wavelet_row = height - 1 - row;
        for col in 0..width {
            let (r, g, b) = pixmap.get_rgb(col as u32, row as u32);
            let (y, cb, cr) = super::rgb_to_ycbcr(r, g, b);
            let dst = wavelet_row * stride + col;
            y_plane[dst] = (y as i32 * 64) as i16;
            cb_plane[dst] = (cb as i32 * 64) as i16;
            cr_plane[dst] = (cr as i32 * 64) as i16;
        }
    }

    [
        compare_forward_plane("Y", &y_plane, width, height, stride),
        compare_forward_plane("Cb", &cb_plane, width, height, stride),
        compare_forward_plane("Cr", &cr_plane, width, height, stride),
    ]
}

fn compare_forward_plane(
    plane_name: &'static str,
    input: &[i16],
    width: usize,
    height: usize,
    stride: usize,
) -> ForwardPlaneDiff {
    let mut production = input.to_vec();
    let mut reference = input.to_vec();
    super::forward_wavelet_transform(&mut production, width, height, stride);
    djvulibre_forward_wavelet_transform(&mut reference, width, height, stride);

    let mut diff = ForwardPlaneDiff::new(plane_name);
    let block_rows = height.div_ceil(32);
    let block_cols = width.div_ceil(32);
    for block_row in 0..block_rows {
        for block_col in 0..block_cols {
            for row in 0..32usize {
                let plane_row = (block_row << 5) + row;
                let src_base = plane_row * stride + (block_col << 5);
                let inv_base = row << 5;
                for col in 0..32usize {
                    let coef_idx = crate::ZIGZAG_INV[inv_base + col] as usize;
                    let band = band_for_coefficient(coef_idx);
                    let delta =
                        production[src_base + col] as i32 - reference[src_base + col] as i32;
                    diff.record(band, delta);
                }
            }
        }
    }
    diff
}

fn band_for_coefficient(coef_idx: usize) -> usize {
    for (band, &(first_bucket, last_bucket)) in super::BAND_BUCKETS.iter().enumerate() {
        let first = first_bucket << 4;
        let last = ((last_bucket + 1) << 4) - 1;
        if (first..=last).contains(&coef_idx) {
            return band;
        }
    }
    unreachable!("coefficient index is always in one of the 10 IW44 bands")
}

fn djvulibre_forward_wavelet_transform(
    data: &mut [i16],
    width: usize,
    height: usize,
    stride: usize,
) {
    let mut scale = 1usize;
    while scale < 32 {
        djvulibre_filter_h(data, width, height, stride, scale);
        djvulibre_filter_v(data, width, height, stride, scale);
        scale <<= 1;
    }
}

fn djvulibre_filter_h(data: &mut [i16], width: usize, height: usize, stride: usize, scale: usize) {
    let scale = scale as isize;
    let width = width as isize;
    let stride = stride as isize;
    let scale3 = scale * 3;
    let mut row = 0isize;
    while row < height as isize {
        let base = row * stride;
        let mut offset = scale;
        let mut a0;
        let mut a1 = 0i32;
        let mut a2 = 0i32;
        let mut a3 = 0i32;
        let mut b0;
        let mut b1 = 0i32;
        let mut b2 = 0i32;
        let mut b3 = 0i32;

        if offset < width {
            a1 = sample(data, base + offset - scale);
            a2 = a1;
            a3 = a1;
            if offset + scale < width {
                a2 = sample(data, base + offset + scale);
            }
            if offset + scale3 < width {
                a3 = sample(data, base + offset + scale3);
            }
            b3 = sample(data, base + offset) - ((a1 + a2 + 1) >> 1);
            store(data, base + offset, b3);
            offset += scale * 2;
        }

        while offset + scale3 < width {
            a0 = a1;
            a1 = a2;
            a2 = a3;
            a3 = sample(data, base + offset + scale3);
            b0 = b1;
            b1 = b2;
            b2 = b3;
            b3 = sample(data, base + offset) - ((((a1 + a2) << 3) + (a1 + a2) - a0 - a3 + 8) >> 4);
            store(data, base + offset, b3);
            let even = base + offset - scale3;
            let current = sample(data, even);
            store(
                data,
                even,
                current + ((((b1 + b2) << 3) + (b1 + b2) - b0 - b3 + 16) >> 5),
            );
            offset += scale * 2;
        }

        while offset < width {
            a1 = a2;
            a2 = a3;
            b0 = b1;
            b1 = b2;
            b2 = b3;
            b3 = sample(data, base + offset) - ((a1 + a2 + 1) >> 1);
            store(data, base + offset, b3);
            let even = base + offset - scale3;
            let current = sample(data, even);
            store(
                data,
                even,
                current + ((((b1 + b2) << 3) + (b1 + b2) - b0 - b3 + 16) >> 5),
            );
            offset += scale * 2;
        }

        while offset - scale3 < width {
            b0 = b1;
            b1 = b2;
            b2 = b3;
            b3 = 0;
            let even = base + offset - scale3;
            if even >= base {
                let current = sample(data, even);
                store(
                    data,
                    even,
                    current + ((((b1 + b2) << 3) + (b1 + b2) - b0 - b3 + 16) >> 5),
                );
            }
            offset += scale * 2;
        }

        row += scale;
    }
}

fn djvulibre_filter_v(data: &mut [i16], width: usize, height: usize, stride: usize, scale: usize) {
    let scaled_height = ((height - 1) / scale) + 1;
    let row_step = (scale * stride) as isize;
    let row_step3 = row_step * 3;
    let mut y = 1usize;
    let mut row_base = row_step;

    while (y as isize) - 3 < scaled_height as isize {
        if y >= 3 && y + 3 < scaled_height {
            let mut col = 0usize;
            while col < width {
                let center = row_base + col as isize;
                let a = sample(data, center - row_step) + sample(data, center + row_step);
                let b = sample(data, center - row_step3) + sample(data, center + row_step3);
                let current = sample(data, center);
                store(data, center, current - (((a << 3) + a - b + 8) >> 4));
                col += scale;
            }
        } else if y < scaled_height {
            let mut col = 0usize;
            let neighbor_step = if y + 1 < scaled_height {
                row_step
            } else {
                -row_step
            };
            while col < width {
                let center = row_base + col as isize;
                let a = sample(data, center - row_step) + sample(data, center + neighbor_step);
                let current = sample(data, center);
                store(data, center, current - ((a + 1) >> 1));
                col += scale;
            }
        }

        let update_base = row_base - row_step3;
        if y >= 6 && y < scaled_height {
            let mut col = 0usize;
            while col < width {
                let center = update_base + col as isize;
                let a = sample(data, center - row_step) + sample(data, center + row_step);
                let b = sample(data, center - row_step3) + sample(data, center + row_step3);
                let current = sample(data, center);
                store(data, center, current + (((a << 3) + a - b + 16) >> 5));
                col += scale;
            }
        } else if y >= 3 {
            let mut col = 0usize;
            while col < width {
                let center = update_base + col as isize;
                let p1 = if y >= 4 {
                    sample(data, center - row_step)
                } else {
                    0
                };
                let n1 = if y - 2 < scaled_height {
                    sample(data, center + row_step)
                } else {
                    0
                };
                let p3 = if y >= 6 {
                    sample(data, center - row_step3)
                } else {
                    0
                };
                let n3 = if y < scaled_height {
                    sample(data, center + row_step3)
                } else {
                    0
                };
                let a = p1 + n1;
                let b = p3 + n3;
                let current = sample(data, center);
                store(data, center, current + (((a << 3) + a - b + 16) >> 5));
                col += scale;
            }
        }

        y += 2;
        row_base += row_step * 2;
    }
}

fn sample(data: &[i16], offset: isize) -> i32 {
    data[offset as usize] as i32
}

fn store(data: &mut [i16], offset: isize, value: i32) {
    data[offset as usize] = value as i16;
}
