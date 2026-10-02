//! Encoder options: `Iw44Target` and `Iw44EncodeOptions`.

#[cfg(feature = "std")]
/// Encode-stopping criterion for IW44 background encoding.
///
/// - `Slices` — encode exactly `total_slices` slices (the original behaviour).
/// - `Bpp(f32)` — stop as soon as the cumulative encoded size reaches
///   `bpp * width * height / 8` bytes. At least one slice is always emitted.
///   Values ≤ 0 are clamped to emit one slice; the `Slices` ceiling
///   (`total_slices`) still applies so this never encodes *more* than the
///   slice budget.
///
/// `Default` is `Slices`, preserving byte-identical output to previous versions.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Iw44Target {
    /// Encode exactly `total_slices` slices (default, legacy behaviour).
    #[default]
    Slices,
    /// Stop once cumulative payload reaches `bpp * w * h / 8` bytes.
    Bpp(f32),
}

#[cfg(feature = "std")]
/// Options for IW44 encoding.
#[derive(Clone, Copy, Debug)]
pub struct Iw44EncodeOptions {
    /// Number of slices per BG44 chunk (1..=99, default 10).
    pub slices_per_chunk: u8,
    /// Total number of slices to encode (default 100).
    pub total_slices: u8,
    /// Chroma delay — Y slices before Cb/Cr encoding begins (default 0).
    pub chroma_delay: u8,
    /// Legacy no-op retained for source compatibility.
    ///
    /// IW44 v1.2 colour streams always use full-resolution chroma planes;
    /// half-resolution output is not interoperable with DjVuLibre and is no
    /// longer emitted, regardless of this value.
    pub chroma_half: bool,
    /// Encode-stopping criterion. Default: [`Iw44Target::Slices`] (encode all
    /// `total_slices`, byte-identical to pre-target versions).
    pub target: Iw44Target,
}

#[cfg(feature = "std")]
impl Default for Iw44EncodeOptions {
    fn default() -> Self {
        Iw44EncodeOptions {
            slices_per_chunk: 10,
            total_slices: 100,
            // Delay chroma to slice 10, matching DjVuLibre's c44 default
            // (`crcbdelay = 10`). The luma refines for 10 slices before any Cb/Cr
            // is coded; this trims ~10–18 % off colour BG44 with no perceptible
            // chroma loss, and aligns our stream with the standard c44 convention.
            chroma_delay: 10,
            // DjVuLibre interop: our `chroma_half` encodes Cb/Cr at half *spatial*
            // resolution (a smaller plane), but DjVuLibre's IWPixmap decoder always
            // builds full-resolution chroma maps and reads full-resolution chroma
            // slices — so a half-resolution stream leaves it short of bits ("Unexpected
            // End Of File"), making our colour DjVu unreadable in ddjvu/DjVuLibre.
            // Default to full-resolution chroma for interoperability. (Round-trips
            // through our own decoder either way.)
            chroma_half: false,
            target: Iw44Target::Slices,
        }
    }
}
