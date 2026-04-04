//! Parser for the DjVu INFO chunk, which contains per-page metadata.
//!
//! INFO chunk layout (from sndjvu.org spec):
//!
//! ```text
//! Offset  Size  Field
//! 0       2     width            big-endian u16
//! 2       2     height           big-endian u16
//! 4       1     minor_version
//! 5       1     major_version
//! 6       2     dpi              little-endian u16
//! 8       1     gamma_byte       actual_gamma = gamma_byte / 10.0
//! 9       1     flags            bits 0-1: rotation, bit 6: orientation
//! ```
//!
//! The minimum INFO chunk size is 10 bytes; some older files may omit the
//! trailing fields.

use crate::error::IffError;

/// Page rotation encoded in INFO flags bits 0–1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    /// 0° — natural orientation.
    None,
    /// 90° counter-clockwise.
    Ccw90,
    /// 180° rotation.
    Rot180,
    /// 90° clockwise (270° counter-clockwise).
    Cw90,
}

/// Metadata from the INFO chunk of a DjVu page.
#[derive(Debug, Clone, PartialEq)]
pub struct PageInfo {
    /// Page width in pixels.
    pub width: u16,
    /// Page height in pixels.
    pub height: u16,
    /// Resolution in dots per inch.
    pub dpi: u16,
    /// Display gamma (e.g. 2.2).
    pub gamma: f32,
    /// Page rotation.
    pub rotation: Rotation,
}

impl PageInfo {
    /// Parse a [`PageInfo`] from the raw bytes of an INFO chunk.
    ///
    /// # Errors
    ///
    /// Returns [`IffError::Truncated`] if the data is shorter than 10 bytes.
    pub fn parse(data: &[u8]) -> Result<Self, IffError> {
        if data.len() < 10 {
            return Err(IffError::Truncated);
        }

        // width and height are big-endian u16
        let width = u16::from_be_bytes(data[0..2].try_into().map_err(|_| IffError::Truncated)?);
        let height = u16::from_be_bytes(data[2..4].try_into().map_err(|_| IffError::Truncated)?);

        // DPI is little-endian u16 at offset 6
        let dpi = u16::from_le_bytes(data[6..8].try_into().map_err(|_| IffError::Truncated)?);

        // Gamma: byte value / 10.0 (e.g. 22 → 2.2)
        let gamma_byte = data[8];
        let gamma = if gamma_byte == 0 {
            2.2_f32 // default gamma when not specified
        } else {
            gamma_byte as f32 / 10.0
        };

        // Flags byte, bits 0–2: rotation per DjVu spec.
        // Real-world DjVu files use three specific flag values:
        //   5 → CW 90°    2 → 180°    6 → CW 270° (= CCW 90°)
        // Other values (including 1, 3) are treated as no rotation,
        // matching DjVuLibre behavior.
        let flags = data[9];
        let rotation = match flags & 0x07 {
            5 => Rotation::Cw90,
            2 => Rotation::Rot180,
            6 => Rotation::Ccw90,
            _ => Rotation::None,
        };

        Ok(PageInfo {
            width,
            height,
            dpi,
            gamma,
            rotation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// INFO bytes for chicken.djvu page: 181×240, 100 dpi, gamma 2.2, no rotation.
    fn chicken_info_bytes() -> [u8; 10] {
        [
            0x00, 0xB5, // width = 181
            0x00, 0xF0, // height = 240
            0x18, // minor version
            0x00, // major version
            0x64, 0x00, // dpi = 100 (little-endian)
            0x16, // gamma byte = 22 → 2.2
            0x00, // flags: no rotation
        ]
    }

    #[test]
    fn parse_chicken_info() {
        let info = PageInfo::parse(&chicken_info_bytes()).expect("should parse");
        assert_eq!(info.width, 181);
        assert_eq!(info.height, 240);
        assert_eq!(info.dpi, 100);
        assert!((info.gamma - 2.2).abs() < 0.01, "gamma should be 2.2");
        assert_eq!(info.rotation, Rotation::None);
    }

    #[test]
    fn too_short_is_error() {
        let data = [0u8; 9]; // one byte short
        assert_eq!(PageInfo::parse(&data).unwrap_err(), IffError::Truncated);
    }

    #[test]
    fn empty_is_error() {
        assert_eq!(PageInfo::parse(&[]).unwrap_err(), IffError::Truncated);
    }

    #[test]
    fn rotation_none() {
        let mut bytes = chicken_info_bytes();
        bytes[9] = 0x00; // flags bits 0-1 = 0
        let info = PageInfo::parse(&bytes).unwrap();
        assert_eq!(info.rotation, Rotation::None);
    }

    #[test]
    fn rotation_flag1_is_none() {
        let mut bytes = chicken_info_bytes();
        bytes[9] = 0x01;
        let info = PageInfo::parse(&bytes).unwrap();
        assert_eq!(info.rotation, Rotation::None);
    }

    #[test]
    fn rotation_flag2_is_180() {
        let mut bytes = chicken_info_bytes();
        bytes[9] = 0x02;
        let info = PageInfo::parse(&bytes).unwrap();
        assert_eq!(info.rotation, Rotation::Rot180);
    }

    #[test]
    fn rotation_flag5_is_cw90() {
        let mut bytes = chicken_info_bytes();
        bytes[9] = 0x05;
        let info = PageInfo::parse(&bytes).unwrap();
        assert_eq!(info.rotation, Rotation::Cw90);
    }

    #[test]
    fn rotation_flag6_is_ccw90() {
        let mut bytes = chicken_info_bytes();
        bytes[9] = 0x06;
        let info = PageInfo::parse(&bytes).unwrap();
        assert_eq!(info.rotation, Rotation::Ccw90);
    }

    #[test]
    fn gamma_zero_defaults_to_2_2() {
        let mut bytes = chicken_info_bytes();
        bytes[8] = 0x00; // gamma_byte = 0
        let info = PageInfo::parse(&bytes).unwrap();
        assert!(
            (info.gamma - 2.2).abs() < 0.01,
            "default gamma should be 2.2"
        );
    }

    #[test]
    fn parse_real_chicken_info_from_iff() {
        // Load the real chicken.djvu and verify INFO chunk parses correctly
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/djvujs/library/assets/chicken.djvu");
        let data = std::fs::read(&path).expect("chicken.djvu must exist");
        let form = crate::iff::parse_form(&data).expect("IFF parse failed");

        let info_chunk = form
            .chunks
            .iter()
            .find(|c| &c.id == b"INFO")
            .expect("INFO chunk must be present");

        let info = PageInfo::parse(info_chunk.data).expect("INFO parse failed");
        assert_eq!(info.width, 181);
        assert_eq!(info.height, 240);
        assert_eq!(info.dpi, 100);
    }
}
