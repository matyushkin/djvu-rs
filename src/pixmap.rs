#[cfg(not(feature = "std"))]
use alloc::{format, vec, vec::Vec};

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

impl Pixmap {
    /// Maximum pixels per pixmap (~64 megapixels = ~256 MB RGBA).
    /// Anything beyond this is a runaway DPI — return an empty pixmap
    /// so the caller gets a harmless blank instead of OOM or overflow.
    const MAX_PIXELS: usize = 64 * 1024 * 1024;

    /// Create a new pixmap filled with the given RGBA color.
    ///
    /// Returns an empty 0×0 pixmap if `width * height` would exceed
    /// 64 MiB pixels or overflow `usize`, preventing OOM from extreme
    /// DPI values.
    pub fn new(width: u32, height: u32, r: u8, g: u8, b: u8, a: u8) -> Self {
        let Some(pixel_count) = (width as usize).checked_mul(height as usize) else {
            return Self::default();
        };
        if pixel_count > Self::MAX_PIXELS {
            return Self::default();
        }
        // Fast path: all channels equal — single memset.
        if r == g && g == b && b == a {
            return Pixmap {
                width,
                height,
                data: vec![r; pixel_count * 4],
            };
        }
        // General path: repeat the 4-byte RGBA pattern `pixel_count` times.
        // `slice::repeat` uses a doubling memcpy strategy and is highly optimised.
        let data = [r, g, b, a].repeat(pixel_count);
        Pixmap {
            width,
            height,
            data,
        }
    }

    /// Create a white opaque pixmap.
    pub fn white(width: u32, height: u32) -> Self {
        Self::new(width, height, 255, 255, 255, 255)
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
        for chunk in self.data.chunks_exact(4) {
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
        for chunk in self.data.chunks_exact(4) {
            out.push(chunk[0]); // R
            out.push(chunk[1]); // G
            out.push(chunk[2]); // B
        }
        out
    }

    /// Convert to 8-bit grayscale using ITU-R BT.601 luminance weights.
    ///
    /// `Y = 0.299·R + 0.587·G + 0.114·B`
    ///
    /// Returns a [`GrayPixmap`] with `data.len() == width * height`.
    pub fn to_gray8(&self) -> GrayPixmap {
        let pixel_count = self.data.len() / 4;
        let mut data = Vec::with_capacity(pixel_count);
        for chunk in self.data.chunks_exact(4) {
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
