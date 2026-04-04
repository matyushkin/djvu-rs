#[cfg(not(feature = "std"))]
use alloc::{format, vec, vec::Vec};

/// A 1-bit-per-pixel packed bitmap image.
///
/// Pixels are packed 8-per-byte, MSB first within each byte.
/// Each row is padded to a byte boundary.
/// Pixel value 1 = black, 0 = white (matching PBM convention).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bitmap {
    pub width: u32,
    pub height: u32,
    /// Packed pixel data, row-major. Row stride = `row_stride()` bytes.
    pub data: Vec<u8>,
}

impl Bitmap {
    /// Create a new all-white (0) bitmap.
    pub fn new(width: u32, height: u32) -> Self {
        let stride = Self::compute_row_stride(width);
        Bitmap {
            width,
            height,
            data: vec![0u8; stride * height as usize],
        }
    }

    /// Bytes per row (each row padded to byte boundary).
    #[inline]
    pub fn row_stride(&self) -> usize {
        Self::compute_row_stride(self.width)
    }

    fn compute_row_stride(width: u32) -> usize {
        (width as usize).div_ceil(8)
    }

    /// Get pixel value at (x, y). Returns `true` if black (1).
    /// Panics if out of bounds.
    #[inline]
    pub fn get(&self, x: u32, y: u32) -> bool {
        debug_assert!(x < self.width && y < self.height);
        let stride = self.row_stride();
        let byte_idx = y as usize * stride + (x as usize / 8);
        let bit_idx = 7 - (x % 8);
        (self.data[byte_idx] >> bit_idx) & 1 == 1
    }

    /// Set pixel value at (x, y). `val = true` means black (1).
    /// Panics if out of bounds.
    pub fn set(&mut self, x: u32, y: u32, val: bool) {
        debug_assert!(x < self.width && y < self.height);
        let stride = self.row_stride();
        let byte_idx = y as usize * stride + (x as usize / 8);
        let bit_idx = 7 - (x % 8);
        if val {
            self.data[byte_idx] |= 1 << bit_idx;
        } else {
            self.data[byte_idx] &= !(1 << bit_idx);
        }
    }

    /// OR a black pixel at (x, y). Used for JB2 blit compositing.
    pub fn set_black(&mut self, x: u32, y: u32) {
        let stride = self.row_stride();
        let byte_idx = y as usize * stride + (x as usize / 8);
        let bit_idx = 7 - (x % 8);
        self.data[byte_idx] |= 1 << bit_idx;
    }

    /// Return a new bitmap with each black pixel expanded to its 4-connected
    /// neighbors (1-pixel morphological dilation). Thickens every stroke by
    /// ~2 pixels total, improving legibility at reduced display sizes.
    pub fn dilate(&self) -> Bitmap {
        let mut out = self.clone();
        for y in 0..self.height {
            for x in 0..self.width {
                if self.get(x, y) {
                    if x > 0 {
                        out.set_black(x - 1, y);
                    }
                    if x + 1 < self.width {
                        out.set_black(x + 1, y);
                    }
                    if y > 0 {
                        out.set_black(x, y - 1);
                    }
                    if y + 1 < self.height {
                        out.set_black(x, y + 1);
                    }
                }
            }
        }
        out
    }

    /// Encode as PBM (binary, P4 format).
    /// This is the format produced by `ddjvu -format=pbm`.
    pub fn to_pbm(&self) -> Vec<u8> {
        let header = format!("P4\n{} {}\n", self.width, self.height);
        let stride = self.row_stride();
        let mut out = Vec::with_capacity(header.len() + stride * self.height as usize);
        out.extend_from_slice(header.as_bytes());
        // PBM P4 packs MSB-first, rows padded to byte boundary — same as our storage
        for y in 0..self.height as usize {
            let row_start = y * stride;
            out.extend_from_slice(&self.data[row_start..row_start + stride]);
        }
        out
    }

    /// Perform `passes` rounds of 4-connected morphological dilation using
    /// packed bitwise operations and two pre-allocated ping-pong buffers.
    /// Zero allocations after the initial setup; much faster than calling
    /// `dilate()` N times for passes > 1.
    ///
    /// Pixel layout (MSB-first): bit 7 of each byte = leftmost pixel in that
    /// group of 8. Horizontal dilation uses `byte >> 1` (right) and
    /// `byte << 1` (left) with cross-byte carries; vertical dilation ORs
    /// each row into its neighbours.
    pub fn dilate_n(self, passes: u32) -> Self {
        if passes == 0 {
            return self;
        }
        let width = self.width;
        let height = self.height;
        let stride = self.row_stride();
        let h = height as usize;

        let mut src = self.data;
        let mut dst = vec![0u8; stride * h];

        for _ in 0..passes {
            for b in dst.iter_mut() {
                *b = 0;
            }

            for y in 0..h {
                let row = y * stride;

                for i in 0..stride {
                    let v = src[row + i];
                    if v == 0 {
                        continue;
                    }

                    dst[row + i] |= v; // original
                    dst[row + i] |= v >> 1; // right neighbours (x+1) within byte
                    dst[row + i] |= v << 1; // left neighbours (x-1) within byte

                    // Carry: rightmost pixel of byte i → leftmost of byte i+1
                    if i + 1 < stride {
                        dst[row + i + 1] |= (v & 0x01) << 7;
                    }
                    // Carry: leftmost pixel of byte i → rightmost of byte i-1
                    if i > 0 {
                        dst[row + i - 1] |= (v & 0x80) >> 7;
                    }

                    // Vertical: down (y+1)
                    if y + 1 < h {
                        dst[(y + 1) * stride + i] |= v;
                    }
                    // Vertical: up (y-1)
                    if y > 0 {
                        dst[(y - 1) * stride + i] |= v;
                    }
                }
            }

            core::mem::swap(&mut src, &mut dst);
        }

        Bitmap {
            width,
            height,
            data: src,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_bitmap_is_all_white() {
        let bm = Bitmap::new(10, 5);
        for y in 0..5 {
            for x in 0..10 {
                assert!(!bm.get(x, y));
            }
        }
    }

    #[test]
    fn set_and_get_roundtrip() {
        let mut bm = Bitmap::new(16, 8);
        bm.set(0, 0, true);
        bm.set(15, 0, true);
        bm.set(7, 3, true);
        bm.set(0, 7, true);
        bm.set(15, 7, true);

        assert!(bm.get(0, 0));
        assert!(bm.get(15, 0));
        assert!(bm.get(7, 3));
        assert!(bm.get(0, 7));
        assert!(bm.get(15, 7));

        // Adjacent pixels should be unaffected
        assert!(!bm.get(1, 0));
        assert!(!bm.get(14, 0));
        assert!(!bm.get(6, 3));
        assert!(!bm.get(8, 3));
    }

    #[test]
    fn set_and_clear() {
        let mut bm = Bitmap::new(8, 1);
        bm.set(3, 0, true);
        assert!(bm.get(3, 0));
        bm.set(3, 0, false);
        assert!(!bm.get(3, 0));
    }

    #[test]
    fn non_byte_aligned_width() {
        // Width=10, so stride=2 bytes (16 bits, 6 padding bits)
        let mut bm = Bitmap::new(10, 2);
        assert_eq!(bm.row_stride(), 2);
        bm.set(9, 0, true);
        bm.set(9, 1, true);
        assert!(bm.get(9, 0));
        assert!(bm.get(9, 1));
    }

    #[test]
    fn to_pbm_format() {
        let mut bm = Bitmap::new(8, 2);
        // Row 0: pixel 0 and 7 black → 0b10000001 = 0x81
        bm.set(0, 0, true);
        bm.set(7, 0, true);
        // Row 1: all black → 0xFF
        for x in 0..8 {
            bm.set(x, 1, true);
        }
        let pbm = bm.to_pbm();
        let hdr = b"P4\n8 2\n";
        assert_eq!(&pbm[..hdr.len()], hdr);
        assert_eq!(pbm[hdr.len()], 0x81);
        assert_eq!(pbm[hdr.len() + 1], 0xFF);
        assert_eq!(pbm.len(), hdr.len() + 2);
    }

    #[test]
    fn to_pbm_non_byte_aligned() {
        // Width=3, stride=1 byte, only top 3 bits used
        let mut bm = Bitmap::new(3, 1);
        bm.set(0, 0, true);
        bm.set(2, 0, true);
        let pbm = bm.to_pbm();
        let hdr = b"P4\n3 1\n";
        assert_eq!(&pbm[..hdr.len()], hdr);
        // Bits: 1 0 1 00000 = 0b10100000 = 0xA0
        assert_eq!(pbm[hdr.len()], 0xA0);
    }

    // ── dilate_n tests ───────────────────────────────────────────────────────

    /// Helper: collect all set pixels as (x,y) pairs, sorted.
    fn set_pixels(bm: &Bitmap) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        for y in 0..bm.height {
            for x in 0..bm.width {
                if bm.get(x, y) {
                    out.push((x, y));
                }
            }
        }
        out
    }

    #[test]
    fn dilate_n_zero_passes_is_identity() {
        let mut bm = Bitmap::new(8, 8);
        bm.set(3, 3, true);
        let orig = set_pixels(&bm);
        let result = set_pixels(&bm.clone().dilate_n(0));
        assert_eq!(orig, result);
    }

    #[test]
    fn dilate_n_one_pass_matches_dilate() {
        let mut bm = Bitmap::new(16, 16);
        bm.set(7, 7, true);
        bm.set(0, 0, true);
        bm.set(15, 15, true);
        let expected = set_pixels(&bm.dilate());
        let got = set_pixels(&bm.clone().dilate_n(1));
        assert_eq!(expected, got);
    }

    #[test]
    fn dilate_n_two_passes_matches_dilate_twice() {
        let mut bm = Bitmap::new(20, 20);
        bm.set(10, 10, true);
        bm.set(1, 1, true);
        let expected = set_pixels(&bm.dilate().dilate());
        let got = set_pixels(&bm.clone().dilate_n(2));
        assert_eq!(expected, got);
    }

    #[test]
    fn dilate_n_three_passes_matches_dilate_three_times() {
        let mut bm = Bitmap::new(20, 20);
        bm.set(10, 10, true);
        let expected = set_pixels(&bm.dilate().dilate().dilate());
        let got = set_pixels(&bm.clone().dilate_n(3));
        assert_eq!(expected, got);
    }

    #[test]
    fn dilate_n_cross_byte_boundary() {
        // Pixel at position 7 (last in first byte) — its x+1 neighbour is pixel 8
        let mut bm = Bitmap::new(16, 1);
        bm.set(7, 0, true);
        let result = bm.clone().dilate_n(1);
        assert!(result.get(6, 0), "x-1 neighbour");
        assert!(result.get(7, 0), "original");
        assert!(result.get(8, 0), "x+1 neighbour (cross-byte)");
        // Pixel at position 8 (first in second byte) — its x-1 neighbour is pixel 7
        let mut bm2 = Bitmap::new(16, 1);
        bm2.set(8, 0, true);
        let result2 = bm2.clone().dilate_n(1);
        assert!(result2.get(7, 0), "x-1 neighbour (cross-byte)");
        assert!(result2.get(8, 0), "original");
        assert!(result2.get(9, 0), "x+1 neighbour");
    }

    #[test]
    fn dilate_n_boundary_pixels_dont_wrap() {
        // Top-left corner pixel — should only expand right and down
        let mut bm = Bitmap::new(16, 16);
        bm.set(0, 0, true);
        let result = bm.dilate_n(1);
        assert!(result.get(0, 0));
        assert!(result.get(1, 0));
        assert!(result.get(0, 1));
        // Nothing in last row/col from this pixel
        assert!(!result.get(15, 15));
    }
}
