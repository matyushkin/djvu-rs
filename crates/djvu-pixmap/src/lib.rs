#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{format, vec, vec::Vec};
#[cfg(feature = "std")]
use std::{format, vec, vec::Vec};

/// An RGBA pixel image, 4 bytes per pixel.
///
/// Row-major, top-to-bottom. Alpha is always 255 for DjVu pages.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pixmap {
    pub width: u32,
    pub height: u32,
    /// RGBA pixel data, row-major. Length = width * height * 4.
    pub data: Vec<u8>,
}

impl AsRef<[u8]> for Pixmap {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

/// Why [`Pixmap::try_new`] refused to allocate.
///
/// Both variants describe the requested size, so a caller can report it or
/// map it into its own limit error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PixmapError {
    /// `width * height` does not fit in `usize` on this target.
    Overflow {
        /// Requested width in pixels.
        width: u32,
        /// Requested height in pixels.
        height: u32,
    },
    /// `width * height` exceeds [`Pixmap::MAX_PIXELS`].
    TooLarge {
        /// Requested width in pixels.
        width: u32,
        /// Requested height in pixels.
        height: u32,
        /// Requested pixel count (`width * height`).
        pixels: usize,
        /// The ceiling it exceeded, [`Pixmap::MAX_PIXELS`].
        max: usize,
    },
}

impl core::fmt::Display for PixmapError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PixmapError::Overflow { width, height } => {
                write!(f, "pixmap {width}x{height} overflows the pixel count")
            }
            PixmapError::TooLarge {
                width,
                height,
                pixels,
                max,
            } => {
                write!(
                    f,
                    "pixmap {width}x{height} = {pixels} pixels exceeds the limit of {max}"
                )
            }
        }
    }
}

impl core::error::Error for PixmapError {}

impl Pixmap {
    /// Maximum pixels per pixmap (64 megapixels, 256 MiB of RGBA).
    ///
    /// Anything beyond this is a runaway DPI. [`Pixmap::try_new`] refuses
    /// it with [`PixmapError::TooLarge`] instead of attempting the
    /// allocation.
    pub const MAX_PIXELS: usize = 64 * 1024 * 1024;

    /// Create a new pixmap filled with the given RGBA color.
    ///
    /// # Errors
    ///
    /// [`PixmapError::Overflow`] when `width * height` does not fit in
    /// `usize`; [`PixmapError::TooLarge`] when it exceeds
    /// [`Pixmap::MAX_PIXELS`]. Nothing is allocated in either case.
    pub fn try_new(
        width: u32,
        height: u32,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    ) -> Result<Self, PixmapError> {
        let Some(pixel_count) = (width as usize).checked_mul(height as usize) else {
            return Err(PixmapError::Overflow { width, height });
        };
        if pixel_count > Self::MAX_PIXELS {
            return Err(PixmapError::TooLarge {
                width,
                height,
                pixels: pixel_count,
                max: Self::MAX_PIXELS,
            });
        }
        // Fast path: all channels equal — single memset.
        if r == g && g == b && b == a {
            return Ok(Pixmap {
                width,
                height,
                data: vec![r; pixel_count * 4],
            });
        }
        // General path: repeat the 4-byte RGBA pattern `pixel_count` times.
        // `slice::repeat` uses a doubling memcpy strategy and is highly optimised.
        let data = [r, g, b, a].repeat(pixel_count);
        Ok(Pixmap {
            width,
            height,
            data,
        })
    }

    /// Create a new pixmap filled with the given RGBA color.
    ///
    /// Returns an empty 0×0 pixmap if `width * height` would exceed
    /// [`Pixmap::MAX_PIXELS`] or overflow `usize`. That silent fallback is
    /// why this constructor is deprecated: use [`Pixmap::try_new`] and
    /// handle the [`PixmapError`] instead.
    #[deprecated(
        since = "0.34.0",
        note = "use `Pixmap::try_new`, which reports an oversized request instead of returning an empty pixmap"
    )]
    pub fn new(width: u32, height: u32, r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::try_new(width, height, r, g, b, a).unwrap_or_default()
    }

    /// Create a white opaque pixmap.
    ///
    /// # Errors
    ///
    /// The same as [`Pixmap::try_new`].
    pub fn try_white(width: u32, height: u32) -> Result<Self, PixmapError> {
        Self::try_new(width, height, 255, 255, 255, 255)
    }

    /// Create a white opaque pixmap.
    ///
    /// Returns an empty 0×0 pixmap when the size is refused; see
    /// [`Pixmap::try_white`] for the variant that reports why.
    pub fn white(width: u32, height: u32) -> Self {
        Self::try_white(width, height).unwrap_or_default()
    }

    /// Set pixel at (x, y) to an RGB value (alpha = 255).
    /// Silently ignores out-of-bounds writes (e.g. on an empty overflow pixmap).
    #[inline]
    pub fn set_rgb(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        let idx = (y as usize * self.width as usize + x as usize) * 4;
        if let Some(pixel) = self.data.get_mut(idx..idx + 4) {
            pixel[0] = r;
            pixel[1] = g;
            pixel[2] = b;
            pixel[3] = 255;
        }
    }

    /// Get the 4 RGBA bytes at pixel (x, y), or `None` if out of bounds.
    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<&[u8]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = (y as usize * self.width as usize + x as usize) * 4;
        self.data.get(idx..idx + 4)
    }

    /// Get RGB at (x, y). Returns (0, 0, 0) for out-of-bounds reads.
    #[inline]
    pub fn get_rgb(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let idx = (y as usize * self.width as usize + x as usize) * 4;
        if let Some(pixel) = self.data.get(idx..idx + 4) {
            (pixel[0], pixel[1], pixel[2])
        } else {
            (0, 0, 0)
        }
    }

    /// Extract RGB pixel data (3 bytes per pixel), discarding alpha.
    pub fn to_rgb(&self) -> Vec<u8> {
        let pixel_count = self.data.len() / 4;
        let mut out = Vec::with_capacity(pixel_count * 3);
        for chunk in self.data.as_chunks::<4>().0 {
            out.push(chunk[0]);
            out.push(chunk[1]);
            out.push(chunk[2]);
        }
        out
    }

    /// Encode as PPM (binary, P6 format).
    /// This is the format produced by `ddjvu -format=ppm`.
    /// Discards alpha channel.
    pub fn to_ppm(&self) -> Vec<u8> {
        let header = format!("P6\n{} {}\n255\n", self.width, self.height);
        let pixel_count = self.data.len() / 4;
        let mut out = Vec::with_capacity(header.len() + pixel_count * 3);
        out.extend_from_slice(header.as_bytes());
        for chunk in self.data.as_chunks::<4>().0 {
            out.push(chunk[0]); // R
            out.push(chunk[1]); // G
            out.push(chunk[2]); // B
        }
        out
    }

    /// Rotate this pixmap 90° clockwise.
    pub fn rotate_cw90(&self) -> Self {
        let (w, h) = (self.width, self.height);
        let mut dst = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let src_off = ((y * w + x) * 4) as usize;
                let dst_x = h - 1 - y;
                let dst_y = x;
                let dst_off = ((dst_y * h + dst_x) * 4) as usize;
                dst[dst_off..dst_off + 4].copy_from_slice(&self.data[src_off..src_off + 4]);
            }
        }
        Pixmap {
            width: h,
            height: w,
            data: dst,
        }
    }

    /// Rotate this pixmap 180°.
    pub fn rotate_180(&self) -> Self {
        let (w, h) = (self.width, self.height);
        let mut dst = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let src_off = ((y * w + x) * 4) as usize;
                let dst_off = (((h - 1 - y) * w + (w - 1 - x)) * 4) as usize;
                dst[dst_off..dst_off + 4].copy_from_slice(&self.data[src_off..src_off + 4]);
            }
        }
        Pixmap {
            width: w,
            height: h,
            data: dst,
        }
    }

    /// Rotate this pixmap 90° counter-clockwise.
    pub fn rotate_ccw90(&self) -> Self {
        let (w, h) = (self.width, self.height);
        let mut dst = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let src_off = ((y * w + x) * 4) as usize;
                let dst_x = y;
                let dst_y = w - 1 - x;
                let dst_off = ((dst_y * h + dst_x) * 4) as usize;
                dst[dst_off..dst_off + 4].copy_from_slice(&self.data[src_off..src_off + 4]);
            }
        }
        Pixmap {
            width: h,
            height: w,
            data: dst,
        }
    }

    /// Convert to 8-bit grayscale using ITU-R BT.601 luminance weights.
    ///
    /// `Y = 0.299·R + 0.587·G + 0.114·B`
    ///
    /// Returns a [`GrayPixmap`] with `data.len() == width * height`.
    pub fn to_gray8(&self) -> GrayPixmap {
        let pixel_count = self.data.len() / 4;
        let mut data = Vec::with_capacity(pixel_count);
        for chunk in self.data.as_chunks::<4>().0 {
            let r = chunk[0] as u32;
            let g = chunk[1] as u32;
            let b = chunk[2] as u32;
            // Fixed-point: weights × 1024 → 306 + 601 + 117 = 1024
            let y = (r * 306 + g * 601 + b * 117) >> 10;
            data.push(y.min(255) as u8);
        }
        GrayPixmap {
            width: self.width,
            height: self.height,
            data,
        }
    }
}

/// An 8-bit grayscale image, 1 byte per pixel.
///
/// Row-major, top-to-bottom. `data.len() == width * height`.
/// Produced by [`Pixmap::to_gray8`] or [`crate::djvu_render::render_gray8`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GrayPixmap {
    pub width: u32,
    pub height: u32,
    /// Grayscale pixel data, row-major. Length = `width * height`.
    pub data: Vec<u8>,
}

impl GrayPixmap {
    /// Get the luminance value at pixel (x, y).
    #[inline]
    pub fn get(&self, x: u32, y: u32) -> u8 {
        self.data[(y as usize * self.width as usize) + x as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_pixmap() {
        let pm = Pixmap::white(2, 2);
        assert_eq!(pm.data.len(), 16);
        for chunk in pm.data.chunks(4) {
            assert_eq!(chunk, &[255, 255, 255, 255]);
        }
    }

    #[test]
    fn set_get_rgb() {
        let mut pm = Pixmap::white(3, 3);
        pm.set_rgb(1, 1, 100, 150, 200);
        assert_eq!(pm.get_rgb(1, 1), (100, 150, 200));
        assert_eq!(pm.get_rgb(0, 0), (255, 255, 255));
    }

    #[test]
    fn rotate_cw90_swaps_dimensions() {
        let pm = Pixmap::white(4, 2);
        let r = pm.rotate_cw90();
        assert_eq!((r.width, r.height), (2, 4));
    }

    #[test]
    fn rotate_180_preserves_dimensions() {
        let pm = Pixmap::white(4, 2);
        let r = pm.rotate_180();
        assert_eq!((r.width, r.height), (4, 2));
    }

    #[test]
    fn rotate_ccw90_swaps_dimensions() {
        let pm = Pixmap::white(4, 2);
        let r = pm.rotate_ccw90();
        assert_eq!((r.width, r.height), (2, 4));
    }

    #[test]
    fn rotate_cw90_then_ccw90_is_identity() {
        let mut pm = Pixmap::white(3, 2);
        pm.set_rgb(0, 0, 255, 0, 0); // red top-left
        pm.set_rgb(2, 1, 0, 0, 255); // blue bottom-right
        let roundtrip = pm.rotate_cw90().rotate_ccw90();
        assert_eq!(roundtrip.data, pm.data);
        assert_eq!((roundtrip.width, roundtrip.height), (pm.width, pm.height));
    }

    #[test]
    fn rotate_180_twice_is_identity() {
        let mut pm = Pixmap::white(3, 2);
        pm.set_rgb(1, 0, 10, 20, 30);
        let roundtrip = pm.rotate_180().rotate_180();
        assert_eq!(roundtrip.data, pm.data);
    }

    #[test]
    fn rotate_cw90_moves_top_left_to_top_right() {
        // 2×1 pixmap: red pixel at (0,0), white at (1,0)
        let mut pm = Pixmap::white(2, 1);
        pm.set_rgb(0, 0, 255, 0, 0);
        // After CW90: 1×2, red should be at (0,0) in new coords
        // new_x = h-1-y = 1-1-0=0, new_y = x = 0 → (0,0)
        let r = pm.rotate_cw90();
        assert_eq!(r.width, 1);
        assert_eq!(r.height, 2);
        assert_eq!(r.get_rgb(0, 0), (255, 0, 0));
        assert_eq!(r.get_rgb(0, 1), (255, 255, 255));
    }

    // Lines 24-25: AsRef<[u8]> impl
    #[test]
    fn as_ref_returns_data_slice() {
        let pm = Pixmap::white(1, 1);
        let slice: &[u8] = pm.as_ref();
        assert_eq!(slice.len(), 4);
    }

    // `Pixmap::try_new` refusals: overflow and MAX_PIXELS exceeded; `new`
    // maps both to an empty pixmap.
    #[test]
    fn try_new_largest_request_is_refused() {
        // `u32::MAX * u32::MAX` overflows a 32-bit `usize` and is merely too
        // large on a 64-bit one; either way nothing is allocated.
        let err = Pixmap::try_new(u32::MAX, u32::MAX, 0, 0, 0, 0).unwrap_err();
        #[cfg(target_pointer_width = "32")]
        assert_eq!(
            err,
            PixmapError::Overflow {
                width: u32::MAX,
                height: u32::MAX
            }
        );
        #[cfg(not(target_pointer_width = "32"))]
        assert!(matches!(
            err,
            PixmapError::TooLarge {
                width: u32::MAX,
                height: u32::MAX,
                max: Pixmap::MAX_PIXELS,
                ..
            }
        ));
    }

    #[test]
    fn overflow_error_names_the_request() {
        let err = PixmapError::Overflow {
            width: 70000,
            height: 70000,
        };
        assert_eq!(
            format!("{err}"),
            "pixmap 70000x70000 overflows the pixel count"
        );
    }

    #[test]
    fn try_new_exceeds_max_pixels_reports_count() {
        // 10000 * 10000 = 100 M pixels > 64 Mi pixels on every target.
        let err = Pixmap::try_new(10000, 10000, 255, 0, 0, 255).unwrap_err();
        assert_eq!(
            err,
            PixmapError::TooLarge {
                width: 10000,
                height: 10000,
                pixels: 100_000_000,
                max: Pixmap::MAX_PIXELS
            }
        );
        assert!(format!("{err}").contains("100000000"));
    }

    #[test]
    fn try_new_at_max_pixels_allocates() {
        // Exactly the ceiling is allowed; one row of MAX_PIXELS pixels.
        let pm = Pixmap::try_new(Pixmap::MAX_PIXELS as u32, 1, 1, 2, 3, 4).unwrap();
        assert_eq!(pm.data.len(), Pixmap::MAX_PIXELS * 4);
        assert_eq!(&pm.data[..4], &[1, 2, 3, 4]);
    }

    #[test]
    #[allow(deprecated)]
    fn new_returns_empty_on_refusal() {
        let pm = Pixmap::new(u32::MAX, u32::MAX, 0, 0, 0, 0);
        assert_eq!((pm.width, pm.height, pm.data.len()), (0, 0, 0));
        let pm = Pixmap::new(10000, 10000, 255, 0, 0, 255);
        assert_eq!((pm.width, pm.height, pm.data.len()), (0, 0, 0));
    }

    #[test]
    fn try_white_matches_white() {
        let pm = Pixmap::try_white(3, 2).unwrap();
        assert_eq!(pm, Pixmap::white(3, 2));
        assert!(Pixmap::try_white(u32::MAX, u32::MAX).is_err());
        assert_eq!(Pixmap::white(u32::MAX, u32::MAX).width, 0);
    }

    // Lines 85-90: get_pixel() — bounds check and Some result
    #[test]
    fn get_pixel_out_of_bounds_returns_none() {
        let pm = Pixmap::white(2, 2);
        assert!(pm.get_pixel(2, 0).is_none());
        assert!(pm.get_pixel(0, 2).is_none());
    }

    #[test]
    fn get_pixel_in_bounds_returns_some() {
        let mut pm = Pixmap::white(2, 2);
        pm.set_rgb(1, 0, 10, 20, 30);
        let p = pm.get_pixel(1, 0).expect("in bounds");
        assert_eq!(&p[..3], &[10, 20, 30]);
    }

    // Line 100: get_rgb() out-of-bounds returns (0, 0, 0)
    #[test]
    fn get_rgb_out_of_bounds_returns_zero() {
        let pm = Pixmap::white(2, 2);
        assert_eq!(pm.get_rgb(5, 5), (0, 0, 0));
    }

    #[test]
    fn to_ppm_format() {
        let mut pm = Pixmap::white(2, 1);
        pm.set_rgb(0, 0, 255, 0, 0); // red
        pm.set_rgb(1, 0, 0, 0, 255); // blue
        let ppm = pm.to_ppm();
        let header = b"P6\n2 1\n255\n";
        assert_eq!(&ppm[..header.len()], header);
        assert_eq!(&ppm[header.len()..], &[255, 0, 0, 0, 0, 255]);
    }
}
