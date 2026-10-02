//! `Jbm`, the internal bit-packed working bitmap (row 0 = bottom of the page).

use super::*;

// ────────────────────────────────────────────────────────────────────────────
// Jbm: internal bit-packed working bitmap (row 0 = bottom of page)
// ────────────────────────────────────────────────────────────────────────────

/// Internal working bitmap used during JB2 decoding.
///
/// Pixels are stored bit-packed: 1 bit per pixel, MSB-first within each byte,
/// rows padded to byte boundary (`row_stride_bytes`). Matches `Bitmap`'s
/// convention, which makes blit into `Bitmap` a shift-align copy rather than
/// a byte→bit pack.
/// Row 0 is the **bottom** of the image (DjVu convention).
#[derive(Clone)]
pub(super) struct Jbm {
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) data: Vec<u8>,
}

impl Jbm {
    #[inline(always)]
    pub(super) fn row_stride_bytes(width: i32) -> usize {
        (width.max(0) as usize).div_ceil(8)
    }

    #[inline(always)]
    pub(super) fn stride(&self) -> usize {
        Self::row_stride_bytes(self.width)
    }

    #[inline(always)]
    pub(super) fn storage_bytes(width: i32, height: i32) -> usize {
        Self::row_stride_bytes(width).saturating_mul(height.max(0) as usize)
    }

    pub(super) fn new(width: i32, height: i32) -> Self {
        let len = Self::storage_bytes(width, height);
        Jbm {
            width,
            height,
            data: vec![0u8; len],
        }
    }

    /// Return the pixel value at (row, col); out-of-bounds → 0.
    #[inline(always)]
    pub(super) fn get(&self, row: i32, col: i32) -> u8 {
        if row < 0 || row >= self.height || col < 0 || col >= self.width {
            return 0;
        }
        let stride = self.stride();
        let byte = self.data[row as usize * stride + (col as usize / 8)];
        (byte >> (7 - (col as usize & 7))) & 1
    }

    /// Set pixel at (row, col) to black (1). Caller must ensure in-bounds.
    #[inline(always)]
    pub(super) fn set_black(&mut self, row: usize, col: usize) {
        let stride = self.stride();
        self.data[row * stride + (col / 8)] |= 0x80u8 >> (col & 7);
    }

    /// Construct a `Jbm` using a reusable scratch buffer.
    ///
    /// The buffer is grown to at least `storage_bytes(width, height)` bytes
    /// (never shrunk), and the used portion is zeroed.  The old buffer
    /// contents are taken via `std::mem::take` so `pool` is left empty on
    /// return; the caller regains the buffer by calling
    /// [`Jbm::crop_and_recycle`] or [`Jbm::recycle_into`].
    pub(super) fn new_from_pool(width: i32, height: i32, pool: &mut Vec<u8>) -> Self {
        let bytes = Self::storage_bytes(width, height);
        if pool.len() < bytes {
            pool.resize(bytes, 0u8);
        }
        // Zero the portion we will use (including any bytes reused from a previous symbol).
        pool[..bytes].fill(0u8);
        let mut data = core::mem::take(pool);
        data.truncate(bytes);
        Jbm {
            width,
            height,
            data,
        }
    }

    /// Crop to content and return the original backing buffer to the pool.
    ///
    /// This is the pool-aware alternative to `crop_to_content()`: it performs
    /// the same crop but moves the (now-unused) full-size backing buffer back
    /// into `pool` so it can be reused for the next symbol decode.
    ///
    /// Fast path: if all four border edges already have content (i.e. the bitmap
    /// is already tight), skip the O(w×h) full scan and copy entirely — just
    /// return `self` directly.  This handles the common case where the JB2
    /// encoder already provided tight bounding box dimensions.
    pub(super) fn crop_and_recycle(self, pool: &mut Vec<u8>) -> Jbm {
        if self.width > 0 && self.height > 0 {
            let w = self.width as usize;
            let h = self.height as usize;
            let stride = self.stride();
            let last_col = w - 1;
            let data = &self.data;
            // Any bit set in the row's stride bytes. Padding bits (if any) are
            // guaranteed zero, so OR-ing the whole row is safe.
            let top_has = data[..stride].iter().any(|&b| b != 0);
            let bot_has = data[(h - 1) * stride..h * stride].iter().any(|&b| b != 0);
            let left_has = (0..h).any(|r| (data[r * stride] & 0x80) != 0);
            let right_has =
                (0..h).any(|r| (data[r * stride + last_col / 8] & (0x80u8 >> (last_col & 7))) != 0);
            if top_has && bot_has && left_has && right_has {
                // Already tight — return self directly without copying.
                // Pre-allocate the pool with the same capacity so the next
                // new_from_pool call can reuse it without a realloc.
                *pool = Vec::with_capacity(self.data.len());
                return self;
            }
        }
        let cropped = self.crop_to_content();
        // Move our data buffer back to the pool (it may be larger than `cropped.data`)
        *pool = self.data;
        cropped
    }

    /// Move the backing buffer back into `pool` without cropping.
    ///
    /// Used for symbols that are blitted but not stored in the dict.
    pub(super) fn recycle_into(self, pool: &mut Vec<u8>) {
        *pool = self.data;
    }

    /// Return a new Jbm with surrounding empty rows/columns removed.
    pub(super) fn crop_to_content(&self) -> Jbm {
        if self.width <= 0 || self.height <= 0 {
            return Jbm::new(0, 0);
        }
        let stride = self.stride();
        let mut min_row = self.height;
        let mut max_row: i32 = -1;
        let mut min_col = self.width;
        let mut max_col: i32 = -1;

        for row in 0..self.height {
            let row_bytes = &self.data[row as usize * stride..(row as usize + 1) * stride];
            // Find first/last nonzero byte in the row, then refine to column index.
            let mut byte_min: Option<usize> = None;
            let mut byte_max: Option<usize> = None;
            for (i, &b) in row_bytes.iter().enumerate() {
                if b != 0 {
                    if byte_min.is_none() {
                        byte_min = Some(i);
                    }
                    byte_max = Some(i);
                }
            }
            if let (Some(bmin), Some(bmax)) = (byte_min, byte_max) {
                let col_lo = bmin * 8 + row_bytes[bmin].leading_zeros() as usize;
                // leading zeros in a reversed sense: for MSB-first, the first set
                // bit position within the byte is `leading_zeros`.
                let col_hi = bmax * 8 + (7 - row_bytes[bmax].trailing_zeros() as usize);
                let col_hi = col_hi.min(self.width as usize - 1) as i32;
                let col_lo = col_lo as i32;
                min_row = min_row.min(row);
                max_row = max_row.max(row);
                min_col = min_col.min(col_lo);
                max_col = max_col.max(col_hi);
            }
        }

        if max_row < 0 {
            return Jbm::new(0, 0);
        }

        let nw = max_col - min_col + 1;
        let nh = max_row - min_row + 1;
        let mut out = Jbm::new(nw, nh);

        for row in min_row..=max_row {
            for col in min_col..=max_col {
                let src_byte = self.data[row as usize * stride + (col as usize / 8)];
                if (src_byte >> (7 - (col as usize & 7))) & 1 != 0 {
                    let out_row = (row - min_row) as usize;
                    let out_col = (col - min_col) as usize;
                    out.set_black(out_row, out_col);
                }
            }
        }
        out
    }
}
