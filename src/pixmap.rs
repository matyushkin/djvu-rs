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

impl std::ops::Index<(u32, u32)> for Pixmap {
    type Output = [u8];

    /// Returns the 4 RGBA bytes at pixel (x, y).
    fn index(&self, (x, y): (u32, u32)) -> &[u8] {
        let idx = (y as usize * self.width as usize + x as usize) * 4;
        &self.data[idx..idx + 4]
    }
}

impl Pixmap {
    /// Create a new pixmap filled with the given RGBA color.
    pub fn new(width: u32, height: u32, r: u8, g: u8, b: u8, a: u8) -> Self {
        let pixel_count = width as usize * height as usize;
        let mut data = Vec::with_capacity(pixel_count * 4);
        for _ in 0..pixel_count {
            data.push(r);
            data.push(g);
            data.push(b);
            data.push(a);
        }
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
    #[inline]
    pub fn set_rgb(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        let idx = (y as usize * self.width as usize + x as usize) * 4;
        self.data[idx] = r;
        self.data[idx + 1] = g;
        self.data[idx + 2] = b;
        self.data[idx + 3] = 255;
    }

    /// Get RGB at (x, y).
    #[inline]
    pub fn get_rgb(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let idx = (y as usize * self.width as usize + x as usize) * 4;
        (self.data[idx], self.data[idx + 1], self.data[idx + 2])
    }

    /// Extract RGB pixel data (3 bytes per pixel), discarding alpha.
    pub fn to_rgb(&self) -> Vec<u8> {
        let pixel_count = self.width as usize * self.height as usize;
        let mut out = Vec::with_capacity(pixel_count * 3);
        for i in 0..pixel_count {
            let base = i * 4;
            out.push(self.data[base]);
            out.push(self.data[base + 1]);
            out.push(self.data[base + 2]);
        }
        out
    }

    /// Encode as PPM (binary, P6 format).
    /// This is the format produced by `ddjvu -format=ppm`.
    /// Discards alpha channel.
    pub fn to_ppm(&self) -> Vec<u8> {
        let header = format!("P6\n{} {}\n255\n", self.width, self.height);
        let pixel_count = self.width as usize * self.height as usize;
        let mut out = Vec::with_capacity(header.len() + pixel_count * 3);
        out.extend_from_slice(header.as_bytes());
        for i in 0..pixel_count {
            let base = i * 4;
            out.push(self.data[base]); // R
            out.push(self.data[base + 1]); // G
            out.push(self.data[base + 2]); // B
        }
        out
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
