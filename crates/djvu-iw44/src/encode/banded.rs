//! The banded forward transform: gather and fill one band of blocks at a time.

use super::*;

//
// A large page's input planes cost as much as its block grid: three
// full-resolution `i16` planes beside three dense grids of the same size
// (PERF_EXPERIMENTS.md ENCODE_SPARSE_RECON). The grid has to stay — every
// slice walks every block — but the planes are read exactly once, by
// `gather`, and only after the transform. So the transform runs over bands of
// block rows, mirroring `PlaneDecoder::reconstruct_window` on the decode side:
// each band carries [`crate::BAND_HALO_BLOCKS`] extra block rows on each side,
// which absorb the transform's vertical reach and are thrown away.

/// Bytes of forward-transform bands the encoder may hold at once, all planes
/// together. Chosen so a page at the banding threshold keeps the smallest band
/// [`crate::BAND_MIN_KEEP_BLOCKS`] allows: the grid is the memory that matters
/// on such a page, and a band is the only other thing of any size.
#[cfg(feature = "std")]
pub(super) const ENCODE_BAND_BUDGET_BYTES: usize = 32 * 1024 * 1024;

/// How many block rows one forward-transform band keeps, or `None` to
/// transform whole planes.
///
/// Same policy as the decoder's `band_keep_blocks`: planes under
/// [`crate::BAND_MIN_PLANE_BYTES`] together are transformed whole, exactly as
/// before; larger ones are banded only when a band is at most half the page.
/// `planes` is how many planes share the budget (three for colour, one for
/// grey), each `stride * 32 * 2` bytes per block row.
#[cfg(feature = "std")]
pub(super) fn encode_band_keep_blocks(
    stride: usize,
    block_rows: usize,
    planes: usize,
) -> Option<usize> {
    let per_block_row = stride * 32 * 2 * planes;
    if per_block_row.saturating_mul(block_rows) <= crate::BAND_MIN_PLANE_BYTES {
        return None;
    }
    let affordable =
        (ENCODE_BAND_BUDGET_BYTES / per_block_row).saturating_sub(2 * crate::BAND_HALO_BLOCKS);
    let keep = affordable.max(crate::BAND_MIN_KEEP_BLOCKS);
    (keep * 2 <= block_rows).then_some(keep)
}

/// One band of the forward transform: the block rows it keeps and the buffer
/// rows, halo included, it transforms to get them.
#[cfg(feature = "std")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct EncodeBand {
    /// Kept block rows `first..last`.
    pub(super) first: usize,
    pub(super) last: usize,
    /// Buffered block rows `lo..hi`: the kept rows plus the halo, clipped to
    /// the plane.
    pub(super) lo: usize,
    pub(super) hi: usize,
    /// Rows the transform treats as data. A band that reaches the bottom of
    /// the plane reports the image's own remaining height, so the transform's
    /// boundary handling lands where the image really ends.
    pub(super) logical: usize,
}

#[cfg(feature = "std")]
impl EncodeBand {
    pub(super) fn new(
        first: usize,
        keep: usize,
        halo: usize,
        block_rows: usize,
        height: usize,
    ) -> Self {
        let last = (first + keep).min(block_rows);
        let lo = first.saturating_sub(halo);
        let hi = (last + halo).min(block_rows);
        let logical = if hi == block_rows {
            height - lo * 32
        } else {
            (hi - lo) * 32
        };
        EncodeBand {
            first,
            last,
            lo,
            hi,
            logical,
        }
    }

    /// Rows the band's buffer holds.
    pub(super) fn rows(&self) -> usize {
        (self.hi - self.lo) * 32
    }
}

/// Transform `planes` one band at a time and gather each band into its
/// encoder.
///
/// `fill(band, bufs)` writes the band's buffer rows for every plane: row `i`
/// of a buffer is the plane's absolute row `band.lo * 32 + i`, with the
/// padding rows and columns beyond the image zero, exactly as the whole-plane
/// path leaves them. `keep` and `halo` are in block rows; production passes
/// [`encode_band_keep_blocks`] and [`crate::BAND_HALO_BLOCKS`], the tests
/// force smaller values. Rows a band keeps are byte-identical to the
/// whole-plane transform when `halo` covers the transform's reach.
#[cfg(feature = "std")]
pub(super) fn forward_gather_banded<const N: usize>(
    encs: &mut [PlaneEncoder; N],
    width: usize,
    height: usize,
    stride: usize,
    keep: usize,
    halo: usize,
    mut fill: impl FnMut(&EncodeBand, &mut [Vec<i16>; N]),
) {
    debug_assert!(keep >= 1);
    let block_rows = height.div_ceil(32);
    let buf_rows = (keep + 2 * halo).min(block_rows) * 32;
    let mut bufs: [Vec<i16>; N] = core::array::from_fn(|_| vec![0i16; stride * buf_rows]);

    let mut first = 0usize;
    while first < block_rows {
        let band = EncodeBand::new(first, keep, halo, block_rows, height);
        fill(&band, &mut bufs);

        let rows = band.rows();
        let transform_one = |buf: &mut Vec<i16>, enc: &mut PlaneEncoder| {
            forward_wavelet_transform(&mut buf[..stride * rows], width, band.logical, stride);
            enc.gather_rows(
                &buf[..stride * rows],
                stride,
                band.lo,
                band.first,
                band.last,
            );
        };

        // The planes are independent; with the `parallel` feature they run
        // concurrently, as the whole-plane path does, and the threshold that
        // path uses is far below any page large enough to be banded.
        #[cfg(feature = "parallel")]
        {
            let mut pairs: Vec<(&mut Vec<i16>, &mut PlaneEncoder)> =
                bufs.iter_mut().zip(encs.iter_mut()).collect();
            rayon::scope(|s| {
                for (buf, enc) in pairs.iter_mut() {
                    let (buf, enc): (&mut Vec<i16>, &mut PlaneEncoder) = (buf, enc);
                    s.spawn(move |_| transform_one(buf, enc));
                }
            });
        }
        #[cfg(not(feature = "parallel"))]
        for (buf, enc) in bufs.iter_mut().zip(encs.iter_mut()) {
            transform_one(buf, enc);
        }

        first = band.last;
    }
}

/// Write one band of the three colour planes from `pixmap`. See
/// [`forward_gather_banded`] for the buffer layout.
#[cfg(feature = "std")]
pub(super) fn fill_color_band(
    pixmap: &Pixmap,
    band: &EncodeBand,
    stride: usize,
    bufs: &mut [Vec<i16>; 3],
) {
    let w = pixmap.width as usize;
    let h = pixmap.height as usize;
    let rows = band.rows();
    let [y_buf, cb_buf, cr_buf] = bufs;
    // DjVu stores images bottom-to-top: wavelet row `wr` is image row
    // `h - 1 - wr`. Scale by 64 because `normalize()` divides by 64 on decode.
    // Rows below the image and columns right of it are the zero padding the
    // whole-plane path has; the buffer is reused, so they are written every
    // band.
    for i in 0..rows {
        let wavelet_row = band.lo * 32 + i;
        let off = i * stride;
        if wavelet_row >= h {
            y_buf[off..off + stride].fill(0);
            cb_buf[off..off + stride].fill(0);
            cr_buf[off..off + stride].fill(0);
            continue;
        }
        let row = h - 1 - wavelet_row;
        let src = &pixmap.data[row * w * 4..(row + 1) * w * 4];
        let y_row = &mut y_buf[off..off + stride];
        let cb_row = &mut cb_buf[off..off + stride];
        let cr_row = &mut cr_buf[off..off + stride];
        for (col, px) in src.as_chunks::<4>().0.iter().enumerate() {
            let (y, cb, cr) = rgb_to_ycbcr(px[0], px[1], px[2]);
            y_row[col] = (y as i32 * 64) as i16;
            cb_row[col] = (cb as i32 * 64) as i16;
            cr_row[col] = (cr as i32 * 64) as i16;
        }
        y_row[w..].fill(0);
        cb_row[w..].fill(0);
        cr_row[w..].fill(0);
    }
}

/// Write one band of the grey plane from `pixmap`. See
/// [`forward_gather_banded`] for the buffer layout.
#[cfg(feature = "std")]
pub(super) fn fill_gray_band(
    pixmap: &GrayPixmap,
    band: &EncodeBand,
    stride: usize,
    bufs: &mut [Vec<i16>; 1],
) {
    let w = pixmap.width as usize;
    let h = pixmap.height as usize;
    let rows = band.rows();
    let [y_buf] = bufs;
    // Bottom-to-top, as above; grey is `(127 - p) * 64` because the decoder
    // gives `gray = 127 - normalize(coeff)`.
    for i in 0..rows {
        let wavelet_row = band.lo * 32 + i;
        let off = i * stride;
        let y_row = &mut y_buf[off..off + stride];
        if wavelet_row >= h {
            y_row.fill(0);
            continue;
        }
        let row = h - 1 - wavelet_row;
        let src = &pixmap.data[row * w..(row + 1) * w];
        for (dst, &p) in y_row[..w].iter_mut().zip(src) {
            *dst = ((127 - p as i32) * 64) as i16;
        }
        y_row[w..].fill(0);
    }
}
