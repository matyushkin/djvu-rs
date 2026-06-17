//! PNG file → [`Pixmap`] decoder.
//!
//! Provides [`decode_png_to_pixmap`], a thin wrapper around the `png` crate
//! that converts any 8-bit PNG color type to the RGBA [`Pixmap`] format used
//! throughout djvu-rs.

use std::path::Path;

use crate::Pixmap;

/// Decode an 8-bit PNG file at `path` into a [`Pixmap`].
///
/// Supports RGBA, RGB, GrayscaleAlpha, and Grayscale color types.
/// Returns an error for indexed-color PNGs and for bit depths other than 8.
pub fn decode_png_to_pixmap(path: &Path) -> Result<Pixmap, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info()?;
    let info = reader.info();
    let width = info.width;
    let height = info.height;
    let color = info.color_type;
    let depth = info.bit_depth;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf)?;
    buf.truncate(frame.buffer_size());

    if depth != png::BitDepth::Eight {
        return Err(format!(
            "{}: unsupported PNG bit depth {:?} (only 8-bit channels supported)",
            path.display(),
            depth
        )
        .into());
    }

    let mut data = Vec::with_capacity((width as usize) * (height as usize) * 4);
    match color {
        png::ColorType::Rgba => data.extend_from_slice(&buf),
        png::ColorType::Rgb => {
            for chunk in buf.chunks_exact(3) {
                data.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for chunk in buf.chunks_exact(2) {
                let g = chunk[0];
                data.extend_from_slice(&[g, g, g, chunk[1]]);
            }
        }
        png::ColorType::Grayscale => {
            for &g in &buf {
                data.extend_from_slice(&[g, g, g, 255]);
            }
        }
        png::ColorType::Indexed => {
            return Err(format!("{}: indexed PNG not supported", path.display()).into());
        }
    }

    Ok(Pixmap {
        width,
        height,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode raw pixel bytes into a PNG file and return the path.
    fn write_png(
        dir: &tempfile::TempDir,
        name: &str,
        width: u32,
        height: u32,
        color: png::ColorType,
        pixels: &[u8],
    ) -> std::path::PathBuf {
        let path = dir.path().join(name);
        let file = std::fs::File::create(&path).unwrap();
        let mut encoder = png::Encoder::new(file, width, height);
        encoder.set_color(color);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(pixels).unwrap();
        path
    }

    #[test]
    fn rgb_adds_alpha_255() {
        let dir = tempfile::tempdir().unwrap();
        // 1×1 red pixel in RGB
        let path = write_png(&dir, "rgb.png", 1, 1, png::ColorType::Rgb, &[255, 0, 0]);
        let pm = decode_png_to_pixmap(&path).unwrap();
        assert_eq!(pm.width, 1);
        assert_eq!(pm.height, 1);
        assert_eq!(pm.data, vec![255, 0, 0, 255]);
    }

    #[test]
    fn rgba_passthrough() {
        let dir = tempfile::tempdir().unwrap();
        // 1×1 semi-transparent blue pixel
        let path = write_png(
            &dir,
            "rgba.png",
            1,
            1,
            png::ColorType::Rgba,
            &[0, 0, 255, 128],
        );
        let pm = decode_png_to_pixmap(&path).unwrap();
        assert_eq!(pm.data, vec![0, 0, 255, 128]);
    }

    #[test]
    fn grayscale_expands_to_rgba() {
        let dir = tempfile::tempdir().unwrap();
        // 1×1 gray=200
        let path = write_png(&dir, "gray.png", 1, 1, png::ColorType::Grayscale, &[200]);
        let pm = decode_png_to_pixmap(&path).unwrap();
        assert_eq!(pm.data, vec![200, 200, 200, 255]);
    }

    #[test]
    fn grayscale_alpha_expands_to_rgba() {
        let dir = tempfile::tempdir().unwrap();
        // 1×1 gray=100, alpha=50
        let path = write_png(
            &dir,
            "graya.png",
            1,
            1,
            png::ColorType::GrayscaleAlpha,
            &[100, 50],
        );
        let pm = decode_png_to_pixmap(&path).unwrap();
        assert_eq!(pm.data, vec![100, 100, 100, 50]);
    }

    #[test]
    fn dimensions_preserved() {
        let dir = tempfile::tempdir().unwrap();
        // 3×2 RGB image
        let pixels = vec![0u8; 3 * 2 * 3];
        let path = write_png(&dir, "dim.png", 3, 2, png::ColorType::Rgb, &pixels);
        let pm = decode_png_to_pixmap(&path).unwrap();
        assert_eq!(pm.width, 3);
        assert_eq!(pm.height, 2);
        assert_eq!(pm.data.len(), 3 * 2 * 4);
    }

    #[test]
    fn nonexistent_file_returns_error() {
        let result = decode_png_to_pixmap(std::path::Path::new("/nonexistent/file.png"));
        assert!(result.is_err());
    }

    #[test]
    fn multi_pixel_rgb_row_order() {
        let dir = tempfile::tempdir().unwrap();
        // 2×1: red then green
        let path = write_png(
            &dir,
            "two.png",
            2,
            1,
            png::ColorType::Rgb,
            &[255, 0, 0, 0, 255, 0],
        );
        let pm = decode_png_to_pixmap(&path).unwrap();
        assert_eq!(&pm.data[0..4], &[255, 0, 0, 255]); // red pixel
        assert_eq!(&pm.data[4..8], &[0, 255, 0, 255]); // green pixel
    }

    #[test]
    fn sixteen_bit_depth_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("deep.png");
        {
            let file = std::fs::File::create(&path).unwrap();
            let mut encoder = png::Encoder::new(file, 1, 1);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Sixteen);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[0u8; 6]).unwrap(); // 1×1 RGB 16-bit = 6 bytes
        }
        let result = decode_png_to_pixmap(&path);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("unsupported") || msg.contains("bit depth") || msg.contains("Sixteen"), "msg={msg}");
    }

    #[test]
    fn indexed_color_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("indexed.png");
        {
            let file = std::fs::File::create(&path).unwrap();
            let mut encoder = png::Encoder::new(file, 1, 1);
            encoder.set_color(png::ColorType::Indexed);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_palette(vec![0, 0, 0]); // one-entry palette (black)
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[0u8]).unwrap(); // index 0
        }
        let result = decode_png_to_pixmap(&path);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("indexed") || msg.contains("Indexed"), "msg={msg}");
    }

    #[test]
    fn write_error_message_contains_path() {
        // write_png error path uses path.display() in the message
        let path = std::path::Path::new("/no/such/dir/x.png");
        let err = decode_png_to_pixmap(path).unwrap_err();
        assert!(err.to_string().contains("x.png"));
    }
}
