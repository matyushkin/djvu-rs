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
//! The canonical INFO chunk size is 10 bytes, but `width`/`height` are the
//! only fields DjVuLibre treats as mandatory — real-world files (and
//! `djvudump`/`ddjvu`) accept anything down to 4 bytes, defaulting the
//! version/dpi/gamma/flags fields that are absent. `tests/fixtures/carte.djvu`
//! (a real-world bundled document, byte-exact against DjVuLibre's own
//! tolerance — see `djvu_document::tests::parse_carte_with_short_info_chunk`)
//! ships a 5-byte INFO chunk (width, height, version byte only, no
//! dpi/gamma/flags), so this parser must accept it too rather than reject the
//! whole document as truncated.

use crate::error::IffError;

/// Page rotation encoded in INFO flags bits 0–1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// Returns [`IffError::Truncated`] if the data is shorter than 4 bytes
    /// (not enough to recover `width`/`height`). Anything from 4 bytes up
    /// parses successfully; fields beyond what's present (version, dpi,
    /// gamma, flags) fall back to their DjVuLibre-compatible defaults —
    /// matching real-world short INFO chunks (e.g. `carte.djvu`'s 5-byte
    /// chunk) that DjVuLibre itself accepts.
    pub fn parse(data: &[u8]) -> Result<Self, IffError> {
        if data.len() < 4 {
            return Err(IffError::Truncated);
        }

        // width and height are big-endian u16
        let width = u16::from_be_bytes(data[0..2].try_into().map_err(|_| IffError::Truncated)?);
        let height = u16::from_be_bytes(data[2..4].try_into().map_err(|_| IffError::Truncated)?);

        // DPI is little-endian u16 at offset 6; absent (short chunk) → default.
        let dpi = match data.get(6..8) {
            Some(b) => u16::from_le_bytes(b.try_into().map_err(|_| IffError::Truncated)?),
            None => 300,
        };

        // Gamma: byte value / 10.0 (e.g. 22 → 2.2); absent → default.
        let gamma_byte = data.get(8).copied().unwrap_or(0);
        let gamma = if gamma_byte == 0 {
            2.2_f32 // default gamma when not specified
        } else {
            gamma_byte as f32 / 10.0
        };

        // Flags byte, bits 0–2: rotation per DjVu spec.
        // Real-world DjVu files use three specific flag values:
        //   5 → CW 90°    2 → 180°    6 → CW 270° (= CCW 90°)
        // Other values (including 1, 3) are treated as no rotation,
        // matching DjVuLibre behavior. Absent (short chunk) → no rotation.
        let flags = data.get(9).copied().unwrap_or(0);
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
    fn shorter_than_width_height_is_error() {
        let data = [0u8; 3]; // can't even recover width/height
        assert_eq!(PageInfo::parse(&data).unwrap_err(), IffError::Truncated);
    }

    #[test]
    fn empty_is_error() {
        assert_eq!(PageInfo::parse(&[]).unwrap_err(), IffError::Truncated);
    }

    /// Real-world short INFO chunk (`tests/fixtures/carte.djvu`: width, height,
    /// and a single version byte, no dpi/gamma/flags) must parse rather than
    /// error — DjVuLibre (`djvudump`/`ddjvu`) accepts this file, so rejecting
    /// it as `Truncated` was a parser-strictness bug, not a corrupt fixture.
    #[test]
    fn carte_style_five_byte_info_parses_with_defaults() {
        let data = [0x10, 0x68, 0x09, 0xFC, 0x11]; // width=4200, height=2556, version=17
        let info = PageInfo::parse(&data).expect("short INFO chunk should parse");
        assert_eq!(info.width, 4200);
        assert_eq!(info.height, 2556);
        assert_eq!(info.dpi, 300, "dpi should default when absent");
        assert!(
            (info.gamma - 2.2).abs() < 0.01,
            "gamma should default to 2.2 when absent"
        );
        assert_eq!(info.rotation, Rotation::None, "flags absent → no rotation");
    }

    /// Nine bytes (one short of the canonical 10) still lacks only the flags
    /// byte — dpi and gamma are present and must be honored, not defaulted.
    #[test]
    fn nine_bytes_parses_dpi_and_gamma_defaults_only_flags() {
        let mut data = chicken_info_bytes().to_vec();
        data.truncate(9);
        let info = PageInfo::parse(&data).expect("should parse");
        assert_eq!(info.width, 181);
        assert_eq!(info.height, 240);
        assert_eq!(info.dpi, 100);
        assert!((info.gamma - 2.2).abs() < 0.01);
        assert_eq!(info.rotation, Rotation::None);
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
