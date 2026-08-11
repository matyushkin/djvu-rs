//! Encoder input normalization policy (#694).
//!
//! Ingest converts archival image files into the internal RGBA [`Pixmap`] used
//! by segmentation and encoding. See `docs/encoder-ingestion.md` in the repo
//! root for the supported input matrix.

/// How semi-transparent pixels are handled when an ingest path must produce
/// opaque RGBA (not yet wired to CLI flags — defaults only in slice 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlphaCompositing {
    /// Keep the alpha channel unchanged. Downstream segmentation/encoding sees
    /// partial transparency as-is; no background colour is applied silently.
    #[default]
    Preserve,
    /// Composite onto a solid background before encoding (future CLI flag).
    CompositeOnBackground { red: u8, green: u8, blue: u8 },
}

/// Deterministic reduction of >8-bit samples to 8-bit channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DepthDownconversion {
    /// Keep the high 8 bits of each big-endian 16-bit sample (`sample >> 8`).
    /// No dithering; repeatable across platforms.
    #[default]
    TruncateHighByte,
}

/// Policy knobs applied while decoding raster inputs into [`Pixmap`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IngestPolicy {
    pub alpha: AlphaCompositing,
    pub depth_downconversion: DepthDownconversion,
}

impl IngestPolicy {
    /// Reduce one big-endian 16-bit channel sample to 8 bits.
    #[inline]
    pub fn downsample_u16_be(&self, hi: u8, lo: u8) -> u8 {
        let _ = (self.depth_downconversion, lo);
        hi
    }

    /// Reduce one native-endian 16-bit channel sample to 8 bits.
    #[inline]
    pub fn downsample_u16(&self, sample: u16) -> u8 {
        let _ = self.depth_downconversion;
        (sample >> 8) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_preserves_alpha_and_truncates_depth() {
        let policy = IngestPolicy::default();
        assert_eq!(policy.alpha, AlphaCompositing::Preserve);
        assert_eq!(
            policy.depth_downconversion,
            DepthDownconversion::TruncateHighByte
        );
        assert_eq!(policy.downsample_u16_be(0x12, 0x34), 0x12);
    }
}
