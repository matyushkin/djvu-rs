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
