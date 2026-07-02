//! High-level page encoder — composes the codec primitives into a
//! complete `FORM:DJVU` page ready to wrap as a single-page document or
//! drop into a `FORM:DJVM` bundle.
//!
//! The encoder kit (`jb2_encode`, `iw44_encode`, `fgbz_encode`,
//! `smmr`, `bzz_encode`, `text_encode`, `navm_encode`) provides the
//! per-codec building blocks; this module orchestrates them so callers
//! don't have to hand-assemble IFF chunks.
//!
//! # Quick start
//!
//! Bilevel scan → single-page DjVu file:
//!
//! ```no_run
//! use djvu_rs::Bitmap;
//! use djvu_rs::djvu_encode::{PageEncoder, EncodeQuality};
//!
//! let mut bm = Bitmap::new(1024, 1280);
//! // … fill bm …
//! let bytes = PageEncoder::from_bitmap(&bm)
//!     .with_dpi(300)
//!     .with_quality(EncodeQuality::Lossless)
//!     .encode()
//!     .unwrap();
//! std::fs::write("scan.djvu", bytes).unwrap();
//! ```
//!
//! Color scan → layered DjVu (mask via JB2 + sub-sampled BG via IW44):
//!
//! ```no_run
//! use djvu_rs::Pixmap;
//! use djvu_rs::djvu_encode::{PageEncoder, EncodeQuality};
//!
//! let pm = Pixmap::white(1024, 1280);
//! let bytes = PageEncoder::from_pixmap(&pm)
//!     .with_dpi(300)
//!     .with_quality(EncodeQuality::Quality)
//!     .encode()
//!     .unwrap();
//! ```
//!
//! # Status
//!
//! - `Lossless` from a [`Bitmap`]: ships `INFO + Sjbz`. Pixel-exact.
//! - `Quality` from a [`Pixmap`]: ships `INFO + Sjbz + BG44… + FGbz`
//!   when foreground ink is detected. Lossy by codec definition; output
//!   is decodable end-to-end.
//! - `Archival` from a [`Pixmap`]: same layered chunk shape as `Quality`,
//!   with a denser background sample grid. This is a conservative archival
//!   profile, not a DjVuLibre-equivalent color text optimiser.
//! - `Lossless` from a [`Pixmap`] / `Quality` from a [`Bitmap`] are
//!   rejected: the combinations are mathematically meaningless
//!   (IW44 is lossy; bilevel input has nothing to put in BG44).

use crate::bitmap::Bitmap;
use crate::chunk_encode::{ChunkEncoder, EncodedChunk, FgbzChunk, encode_info};
use crate::fgbz_encode::FgbzColor;
use crate::iff::{Chunk, DjvuFile, emit};
use crate::iw44_encode::{Iw44EncodeOptions, encode_iw44_color};
use crate::jb2_encode::{self, Jb2EncodeOptions};
use crate::pixmap::Pixmap;
use crate::segment::{SegmentOptions, segment_page};

// ── Errors ────────────────────────────────────────────────────────────────────

/// Errors returned by [`PageEncoder::encode`].
#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    /// The requested combination of input + quality profile is not
    /// implemented yet. The message names the missing dependency
    /// (typically a sibling issue tracking the codec layer).
    #[error("page encoder: {0}")]
    Unsupported(&'static str),
}

// ── Quality profile ───────────────────────────────────────────────────────────

/// Encoder quality profile.
///
/// The profile drives codec selection (JB2 vs IW44, mask-only vs
/// layered, optional FGbz palette) and quality knobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EncodeQuality {
    /// Pixel-exact round-trip. Requires bilevel input
    /// ([`PageEncoder::from_bitmap`]); writes `INFO + Sjbz` (JB2).
    #[default]
    Lossless,
    /// Layered foreground/background encoding. Requires color input
    /// ([`PageEncoder::from_pixmap`]); writes `INFO + Sjbz + BG44…`
    /// plus `FGbz` when foreground ink is detected.
    Quality,
    /// Conservative archival color profile. Requires color input; writes
    /// the same layered chunks as `Quality`, but keeps a denser background
    /// sample grid. Bilevel input should use `Lossless`.
    Archival,
}

impl EncodeQuality {
    /// The default segmentation knobs for this profile.
    ///
    /// `Archival` lowers `bg_subsample` to 6 (see [`SegmentOptions::archival`])
    /// for a higher-resolution background; every other profile uses the plain
    /// defaults. This is the canonical `EncodeQuality → SegmentOptions` mapping
    /// — `PageEncoder::encode`, `encode_djvm_layered_shared`, and the CLI all
    /// call it instead of re-deriving the mapping inline.
    pub fn default_segment_options(self) -> SegmentOptions {
        match self {
            EncodeQuality::Archival => SegmentOptions::archival(),
            // `Lossless` never segments (bilevel input has no FG/BG split); it
            // returns the defaults only so this mapping is total. Callers must
            // gate on the profile before reaching `segment_page` — both
            // `PageEncoder::encode` and the CLI reject `Lossless` upstream.
            EncodeQuality::Quality | EncodeQuality::Lossless => SegmentOptions::default(),
        }
    }
}

// ── Encoder ──────────────────────────────────────────────────────────────────

enum Source<'a> {
    Bitmap(&'a Bitmap),
    Pixmap(&'a Pixmap),
}

impl Source<'_> {
    fn dimensions(&self) -> (u32, u32) {
        match self {
            Source::Bitmap(b) => (b.width, b.height),
            Source::Pixmap(p) => (p.width, p.height),
        }
    }
}

/// Builder-style page encoder.
///
/// Constructed from a [`Bitmap`] (bilevel) or [`Pixmap`] (RGBA) and
/// configured via the `with_*` methods, then finalised with
/// [`encode`](Self::encode).
pub struct PageEncoder<'a> {
    source: Source<'a>,
    dpi: u16,
    quality: EncodeQuality,
    segment_options: Option<SegmentOptions>,
    iw44_options: Option<Iw44EncodeOptions>,
    jb2_options: Option<Jb2EncodeOptions>,
}

impl<'a> PageEncoder<'a> {
    /// Start encoding a bilevel page. Defaults: 300 dpi, `Lossless`.
    pub fn from_bitmap(bitmap: &'a Bitmap) -> Self {
        Self {
            source: Source::Bitmap(bitmap),
            dpi: 300,
            quality: EncodeQuality::Lossless,
            segment_options: None,
            iw44_options: None,
            jb2_options: None,
        }
    }

    /// Start encoding a colour page. Defaults: 300 dpi, `Quality` (the
    /// only sensible profile for colour input — `Lossless` requires a
    /// `Bitmap`).
    pub fn from_pixmap(pixmap: &'a Pixmap) -> Self {
        Self {
            source: Source::Pixmap(pixmap),
            dpi: 300,
            quality: EncodeQuality::Quality,
            segment_options: None,
            iw44_options: None,
            jb2_options: None,
        }
    }

    /// Set the page resolution stored in the `INFO` chunk.
    ///
    /// Clamped to `[1, 65 535]` (the wire-format range of the dpi
    /// field). Values outside that range are silently saturated.
    pub fn with_dpi(mut self, dpi: u16) -> Self {
        self.dpi = dpi.max(1);
        self
    }

    /// Select an encoding profile. See [`EncodeQuality`] for the
    /// per-variant trade-offs and current support status.
    pub fn with_quality(mut self, quality: EncodeQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Override the segmentation knobs used by `Quality` / `Archival` color
    /// encodes. Defaults remain profile-specific and fixed-threshold.
    pub fn with_segment_options(mut self, opts: SegmentOptions) -> Self {
        self.segment_options = Some(opts);
        self
    }

    /// Override the IW44 background-codec knobs (slice schedule, chroma
    /// resolution/delay) used by the `Quality` / `Archival` color encodes.
    ///
    /// Defaults to [`Iw44EncodeOptions::default`] (DjVuLibre `c44`-compatible
    /// full-resolution chroma, delay 10). Ignored by the bilevel `Lossless`
    /// path, which writes no `BG44`.
    pub fn with_iw44_options(mut self, opts: Iw44EncodeOptions) -> Self {
        self.iw44_options = Some(opts);
        self
    }

    /// Override the JB2 mask-codec knobs (lossy connected-component threshold)
    /// used by the `Quality` / `Archival` color encodes' `Sjbz` dictionary.
    ///
    /// Defaults to [`Jb2EncodeOptions::default`] (lossless, byte-exact CC
    /// matching). The bilevel `Lossless` path emits a single direct-bitmap
    /// record and is unaffected.
    pub fn with_jb2_options(mut self, opts: Jb2EncodeOptions) -> Self {
        self.jb2_options = Some(opts);
        self
    }

    /// Produce the bytes of a single-page DjVu file (`FORM:DJVU`
    /// wrapped in the `AT&T` IFF container).
    pub fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let (w, h) = self.source.dimensions();
        let w = u16::try_from(w).map_err(|_| {
            EncodeError::Unsupported("page width exceeds INFO chunk limit (65 535 px)")
        })?;
        let h = u16::try_from(h).map_err(|_| {
            EncodeError::Unsupported("page height exceeds INFO chunk limit (65 535 px)")
        })?;
        let info = encode_info(w, h, self.dpi);

        match (&self.source, self.quality) {
            (Source::Bitmap(bm), EncodeQuality::Lossless) => Ok(encode_form_djvu(vec![
                Chunk::Leaf {
                    id: *b"INFO",
                    data: info,
                },
                Chunk::Leaf {
                    id: *b"Sjbz",
                    data: jb2_encode::encode_jb2(bm),
                },
            ])),
            (Source::Pixmap(pm), EncodeQuality::Quality | EncodeQuality::Archival) => {
                let segment_options = self
                    .segment_options
                    .unwrap_or_else(|| self.quality.default_segment_options());
                let seg = segment_page(pm, &segment_options);
                // Use the dictionary encoder for color profiles so FGbz can
                // address foreground colors per blitted component.
                let sjbz = jb2_encode::encode_jb2_dict_with_options(
                    &seg.mask,
                    &[],
                    &self.jb2_options.unwrap_or_default(),
                );
                let bg44_chunks =
                    encode_iw44_color(&seg.bg, &self.iw44_options.unwrap_or_default());
                let fgbz = foreground_fgbz(pm, &seg.mask, &sjbz, None);

                let mut chunks =
                    Vec::with_capacity(2 + bg44_chunks.len() + usize::from(fgbz.is_some()));
                chunks.push(Chunk::Leaf {
                    id: *b"INFO",
                    data: info,
                });
                chunks.push(Chunk::Leaf {
                    id: *b"Sjbz",
                    data: sjbz,
                });
                for body in bg44_chunks {
                    chunks.push(Chunk::Leaf {
                        id: *b"BG44",
                        data: body,
                    });
                }
                if let Some(chunk) = fgbz {
                    chunks.push(chunk.into_leaf());
                }
                Ok(encode_form_djvu(chunks))
            }
            (Source::Pixmap(_), EncodeQuality::Lossless) => Err(EncodeError::Unsupported(
                "Lossless requires bilevel input — use from_bitmap or switch to Quality",
            )),
            (Source::Bitmap(_), EncodeQuality::Quality) => Err(EncodeError::Unsupported(
                "Quality requires colour input — use from_pixmap or switch to Lossless",
            )),
            (Source::Bitmap(_), EncodeQuality::Archival) => Err(EncodeError::Unsupported(
                "Archival requires colour input — use from_pixmap or switch to Lossless",
            )),
        }
    }
}

/// Encode a directory of colour pages as a single bundled DJVM with a **shared
/// Djbz dictionary** across pages (layered Quality/Archival profile).
///
/// Connected components that appear on at least `shared_dict_page_threshold`
/// distinct pages are promoted into one shared `FORM:DJVI` Djbz; each page's
/// `FORM:DJVU` then carries `INCL` + a `Sjbz` that references the shared
/// dictionary, alongside its own `BG44`(s) and optional `FGbz`. This avoids the
/// per-page dictionary duplication of independent layered encoding (#452): on
/// text-heavy multi-page scans the mask shrinks ~35% (1.6× → ~1.04× of the
/// DjVuLibre baseline).
///
/// `FGbz` is rebuilt from the shared-dictionary `Sjbz` so its per-blit palette
/// indices match the emitted symbol stream. With fewer than two pages, or a
/// threshold larger than the page count, no symbols qualify and each page is
/// encoded with its own dictionary (still a valid bundle).
///
/// When `with_thumbnails` is `true`, each page's `FORM:DJVU` additionally
/// contains one or more `TH44` chunk(s) encoding a color IW44 thumbnail (long
/// side ≤ 128 px) of the full page image.  When `false` (the pre-feature
/// default), no `TH44` chunks are emitted and output is identical to the
/// previous behaviour.
pub fn encode_djvm_layered_shared(
    pixmaps: &[Pixmap],
    quality: EncodeQuality,
    dpi: u16,
    segment_options: Option<SegmentOptions>,
    shared_dict_page_threshold: usize,
) -> Result<Vec<u8>, EncodeError> {
    encode_djvm_layered_shared_impl(
        pixmaps,
        quality,
        dpi,
        segment_options,
        shared_dict_page_threshold,
        false,
    )
}

/// Like [`encode_djvm_layered_shared`] but with explicit thumbnail control.
///
/// Pass `with_thumbnails: true` to embed a `TH44` color thumbnail in each
/// page's `FORM:DJVU`; `false` is identical to [`encode_djvm_layered_shared`].
pub fn encode_djvm_layered_shared_with_thumbnails(
    pixmaps: &[Pixmap],
    quality: EncodeQuality,
    dpi: u16,
    segment_options: Option<SegmentOptions>,
    shared_dict_page_threshold: usize,
    with_thumbnails: bool,
) -> Result<Vec<u8>, EncodeError> {
    encode_djvm_layered_shared_impl(
        pixmaps,
        quality,
        dpi,
        segment_options,
        shared_dict_page_threshold,
        with_thumbnails,
    )
}

fn encode_djvm_layered_shared_impl(
    pixmaps: &[Pixmap],
    quality: EncodeQuality,
    dpi: u16,
    segment_options: Option<SegmentOptions>,
    shared_dict_page_threshold: usize,
    with_thumbnails: bool,
) -> Result<Vec<u8>, EncodeError> {
    if !matches!(quality, EncodeQuality::Quality | EncodeQuality::Archival) {
        return Err(EncodeError::Unsupported(
            "encode_djvm_layered_shared requires the Quality or Archival profile",
        ));
    }
    let opts = segment_options.unwrap_or_else(|| quality.default_segment_options());

    // Segment every page once (mask + background), then cluster the masks.
    // Segmentation is per-page independent (Sauvola + IW44 background build); with
    // the `parallel` feature the pages segment concurrently on rayon.
    #[cfg(feature = "parallel")]
    let segs: Vec<_> = {
        use rayon::prelude::*;
        pixmaps
            .par_iter()
            .map(|pm| segment_page(pm, &opts))
            .collect()
    };
    #[cfg(not(feature = "parallel"))]
    let segs: Vec<_> = pixmaps.iter().map(|pm| segment_page(pm, &opts)).collect();
    let masks: Vec<Bitmap> = segs.iter().map(|s| s.mask.clone()).collect();
    let shared = jb2_encode::cluster_shared_symbols(&masks, shared_dict_page_threshold);
    let has_shared = !shared.is_empty();

    let dict_id = "dict0001.djvi";
    let mut comps: Vec<(Vec<u8>, bool, String)> = Vec::new();
    // Decoded form of the shared dictionary, needed to resolve the per-page Sjbz
    // blit maps when rebuilding FGbz (the Sjbz references the dict via INCL).
    let shared_dict = if has_shared {
        let djbz = jb2_encode::encode_jb2_djbz(&shared);
        let dict = crate::jb2::decode_dict(&djbz, None).ok();
        let djvi_body = jb2_encode::build_form_body(b"DJVI", &[(*b"Djbz", djbz)]);
        comps.push((djvi_body, false, dict_id.to_string()));
        dict
    } else {
        None
    };

    // Each page's DJVU body is independent (JB2-dict Sjbz + IW44 background + FGbz +
    // optional TH44). Build one component per page; with the `parallel` feature the
    // pages encode concurrently on rayon, since JB2 + IW44 dominate the per-page cost.
    // Order is preserved by the indexed collect.
    let build_page = |idx: usize,
                      pm: &Pixmap,
                      seg: &crate::segment::SegmentedPage|
     -> Result<(Vec<u8>, bool, String), EncodeError> {
        let w = u16::try_from(pm.width)
            .map_err(|_| EncodeError::Unsupported("page width exceeds INFO chunk limit"))?;
        let h = u16::try_from(pm.height)
            .map_err(|_| EncodeError::Unsupported("page height exceeds INFO chunk limit"))?;

        let sjbz = if has_shared {
            jb2_encode::encode_jb2_dict_with_shared(&seg.mask, &shared)
        } else {
            jb2_encode::encode_jb2_dict(&seg.mask)
        };
        // FGbz is derived from this page's Sjbz blit map, so it must be built
        // from the shared-dictionary stream (passing the shared dict to resolve it).
        let fgbz = foreground_fgbz(pm, &seg.mask, &sjbz, shared_dict.as_ref());
        let bg44_chunks = encode_iw44_color(&seg.bg, &Iw44EncodeOptions::default());

        let mut chunks: Vec<([u8; 4], Vec<u8>)> = Vec::new();
        chunks.push((*b"INFO", encode_info(w, h, dpi)));
        if has_shared {
            chunks.push((*b"INCL", dict_id.as_bytes().to_vec()));
        }
        chunks.push((*b"Sjbz", sjbz));
        for body in bg44_chunks {
            chunks.push((*b"BG44", body));
        }
        if let Some(chunk) = fgbz
            && let Chunk::Leaf { id, data } = chunk.into_leaf()
        {
            chunks.push((id, data));
        }
        // Optionally embed a TH44 color thumbnail.  TH44 chunks sit inside the
        // page's FORM:DJVU body (after FGbz); the reader collects all of them.
        if with_thumbnails {
            let th44_payloads = crate::thumbnail::encode_th44_color(pm);
            for payload in th44_payloads {
                chunks.push((*b"TH44", payload));
            }
        }
        let body = jb2_encode::build_form_body(b"DJVU", &chunks);
        Ok((body, true, format!("p{:04}.djvu", idx + 1)))
    };

    #[cfg(feature = "parallel")]
    let page_comps: Vec<(Vec<u8>, bool, String)> = {
        use rayon::prelude::*;
        pixmaps
            .par_iter()
            .zip(&segs)
            .enumerate()
            .map(|(idx, (pm, seg))| build_page(idx, pm, seg))
            .collect::<Result<Vec<_>, _>>()?
    };
    #[cfg(not(feature = "parallel"))]
    let page_comps: Vec<(Vec<u8>, bool, String)> = pixmaps
        .iter()
        .zip(&segs)
        .enumerate()
        .map(|(idx, (pm, seg))| build_page(idx, pm, seg))
        .collect::<Result<Vec<_>, _>>()?;
    comps.extend(page_comps);

    Ok(jb2_encode::assemble_djvm_bundle(comps))
}

// ── Internal helpers ─────────────────────────────────────────────────────────

fn encode_form_djvu(children: Vec<Chunk>) -> Vec<u8> {
    let file = DjvuFile {
        root: Chunk::Form {
            secondary_id: *b"DJVU",
            length: 0, // recomputed by emit
            children,
        },
    };
    emit(&file)
}

#[derive(Debug, Clone, Copy, Default)]
struct ColorAccum {
    r: u64,
    g: u64,
    b: u64,
    n: u64,
}

impl ColorAccum {
    fn add(&mut self, r: u8, g: u8, b: u8) {
        self.r += u64::from(r);
        self.g += u64::from(g);
        self.b += u64::from(b);
        self.n += 1;
    }

    fn color(self) -> Option<FgbzColor> {
        if self.n == 0 {
            return None;
        }
        Some(FgbzColor {
            r: (self.r / self.n) as u8,
            g: (self.g / self.n) as u8,
            b: (self.b / self.n) as u8,
        })
    }
}

fn foreground_fgbz(
    pm: &Pixmap,
    mask: &Bitmap,
    sjbz: &[u8],
    shared_dict: Option<&crate::jb2::Jb2Dict>,
) -> Option<EncodedChunk> {
    // The Sjbz may reference an external shared Djbz (layered shared-dict bundle),
    // so the dictionary must be supplied to decode its blit map.
    let (decoded_mask, blit_map) = crate::jb2::decode_indexed(sjbz, shared_dict).ok()?;
    if decoded_mask.width != mask.width || decoded_mask.height != mask.height {
        return None;
    }

    let max_blit = blit_map.iter().copied().filter(|&i| i >= 0).max()? as usize;
    let mut by_blit = vec![ColorAccum::default(); max_blit + 1];
    let w = mask.width as usize;
    // Row-slice the mask (bit-test the pre-sliced row byte), the blit map, and the
    // packed RGBA pixmap (`x*4` into a row slice) instead of per-pixel `mask.get`
    // (hidden `/8`) + `pm.get_rgb` (hidden `*4` + bounds). Same pixels, same
    // accumulation order → byte-identical palette. (PS4/PS5 class.)
    let mstride = mask.row_stride();
    for y in 0..mask.height as usize {
        let mrow = &mask.data[y * mstride..(y + 1) * mstride];
        let prow = &pm.data[y * w * 4..(y + 1) * w * 4];
        let brow = &blit_map[y * w..(y + 1) * w];
        for x in 0..w {
            if (mrow[x >> 3] >> (7 - (x & 7))) & 1 != 0 {
                let blit_idx = brow[x];
                if blit_idx < 0 {
                    continue;
                }
                let px = &prow[x * 4..x * 4 + 3];
                by_blit[blit_idx as usize].add(px[0], px[1], px[2]);
            }
        }
    }

    let mut palette: Vec<FgbzColor> = Vec::new();
    let mut indices: Vec<i16> = Vec::with_capacity(by_blit.len());
    for accum in by_blit {
        let color = accum.color().unwrap_or_default();
        let color_idx = match palette.iter().position(|&c| c == color) {
            Some(i) => i,
            None => {
                if palette.len() >= i16::MAX as usize {
                    return None;
                }
                palette.push(color);
                palette.len() - 1
            }
        };
        indices.push(color_idx as i16);
    }

    if palette.is_empty() || palette.iter().all(|c| c.r == 0 && c.g == 0 && c.b == 0) {
        return None;
    }

    let index_payload = if palette.len() > 1 {
        Some(indices.as_slice())
    } else {
        None
    };
    // Best-effort: the palette is bounded < i16::MAX above, so the FGbz
    // wire limits cannot trip here; `.ok()` keeps this a soft skip if a
    // future change relaxes that bound.
    FgbzChunk {
        palette: &palette,
        indices: index_payload,
    }
    .encode_chunk()
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iff::parse_form;
    use crate::jb2;

    fn checkerboard(w: u32, h: u32) -> Bitmap {
        let mut bm = Bitmap::new(w, h);
        for y in 0..h {
            for x in 0..w {
                if (x + y) % 2 == 0 {
                    bm.set_black(x, y);
                }
            }
        }
        bm
    }

    #[test]
    fn default_segment_options_maps_archival_to_dense_background() {
        // Single source of truth for the quality → segmentation mapping: only
        // Archival lowers bg_subsample; everything else uses the plain default.
        assert_eq!(
            EncodeQuality::Archival
                .default_segment_options()
                .bg_subsample,
            6,
            "Archival keeps a denser background grid"
        );
        assert_eq!(
            EncodeQuality::Quality
                .default_segment_options()
                .bg_subsample,
            SegmentOptions::default().bg_subsample,
        );
        assert_eq!(
            EncodeQuality::Lossless
                .default_segment_options()
                .bg_subsample,
            SegmentOptions::default().bg_subsample,
        );
        // archival() is the literal-free constructor those map onto.
        let arch = SegmentOptions::archival();
        assert_eq!(arch.bg_subsample, 6);
        assert_eq!(arch.threshold, SegmentOptions::default().threshold);
        assert_eq!(arch.bg_inpaint, SegmentOptions::default().bg_inpaint);
    }

    #[test]
    fn with_iw44_options_is_threaded_into_background_codec() {
        // Reaching the IW44 knobs through the builder must actually change the
        // emitted BG44 — fewer total slices ⇒ a strictly smaller background.
        let pm = mixed_lighting_fixture();
        let default_bytes = PageEncoder::from_pixmap(&pm)
            .with_quality(EncodeQuality::Quality)
            .encode()
            .expect("default encode");
        let trimmed = Iw44EncodeOptions {
            total_slices: 20,
            ..Iw44EncodeOptions::default()
        };
        let trimmed_bytes = PageEncoder::from_pixmap(&pm)
            .with_quality(EncodeQuality::Quality)
            .with_iw44_options(trimmed)
            .encode()
            .expect("trimmed encode");
        assert!(
            trimmed_bytes.len() < default_bytes.len(),
            "with_iw44_options(total_slices=20) should shrink output ({} vs {})",
            trimmed_bytes.len(),
            default_bytes.len()
        );
        // Still a valid, parseable DjVu page.
        let doc = crate::djvu_document::DjVuDocument::parse(&trimmed_bytes).expect("parse");
        assert!(!doc.page(0).expect("page").all_chunks(b"BG44").is_empty());
    }

    #[test]
    fn with_jb2_options_lossy_threshold_round_trips() {
        // The JB2 knob is reachable through the builder and still produces a
        // decodable mask (lossy CC substitution stays within the format).
        let pm = mixed_lighting_fixture();
        // Spell every field (cfg-gated like the Default impl) so this compiles
        // cleanly whether or not the `experimental` feature is active — neither
        // struct-update nor reassign-after-default triggers a clippy lint.
        let jb2 = Jb2EncodeOptions {
            lossy_threshold: 0.04,
            #[cfg(feature = "experimental")]
            cross_size_rec6_probe: None,
        };
        let bytes = PageEncoder::from_pixmap(&pm)
            .with_quality(EncodeQuality::Quality)
            .with_jb2_options(jb2)
            .encode()
            .expect("lossy jb2 encode");
        let doc = crate::djvu_document::DjVuDocument::parse(&bytes).expect("parse");
        let page = doc.page(0).expect("page");
        assert!(page.raw_chunk(b"Sjbz").is_some());
        page.extract_mask()
            .expect("mask decode")
            .expect("mask present");
    }

    #[test]
    fn lossless_bilevel_round_trips() {
        let bm = checkerboard(64, 48);
        let bytes = PageEncoder::from_bitmap(&bm)
            .with_dpi(150)
            .with_quality(EncodeQuality::Lossless)
            .encode()
            .expect("encode");

        let form = parse_form(&bytes).expect("parse_form");
        assert_eq!(&form.form_type, b"DJVU");

        let mut info_data: Option<&[u8]> = None;
        let mut sjbz_data: Option<&[u8]> = None;
        for chunk in &form.chunks {
            match &chunk.id {
                b"INFO" => info_data = Some(chunk.data),
                b"Sjbz" => sjbz_data = Some(chunk.data),
                _ => {}
            }
        }
        let info = info_data.expect("INFO chunk present");
        let sjbz = sjbz_data.expect("Sjbz chunk present");

        assert_eq!(u16::from_be_bytes([info[0], info[1]]), 64);
        assert_eq!(u16::from_be_bytes([info[2], info[3]]), 48);
        assert_eq!(u16::from_le_bytes([info[6], info[7]]), 150);

        let decoded = jb2::decode(sjbz, None).expect("jb2 decode");
        assert_eq!(decoded.width, bm.width);
        assert_eq!(decoded.height, bm.height);
        for y in 0..bm.height {
            for x in 0..bm.width {
                assert_eq!(decoded.get(x, y), bm.get(x, y), "mismatch at ({x},{y})");
            }
        }
    }

    #[test]
    fn defaults_are_300_dpi_lossless_for_bitmap() {
        let bm = Bitmap::new(8, 8);
        let enc = PageEncoder::from_bitmap(&bm);
        assert_eq!(enc.dpi, 300);
        assert_eq!(enc.quality, EncodeQuality::Lossless);
    }

    #[test]
    fn defaults_are_300_dpi_quality_for_pixmap() {
        let pm = Pixmap::white(8, 8);
        let enc = PageEncoder::from_pixmap(&pm);
        assert_eq!(enc.dpi, 300);
        assert_eq!(enc.quality, EncodeQuality::Quality);
        assert!(enc.segment_options.is_none());
    }

    #[test]
    fn with_dpi_clamps_zero_to_one() {
        let bm = Bitmap::new(8, 8);
        let enc = PageEncoder::from_bitmap(&bm).with_dpi(0);
        assert_eq!(enc.dpi, 1);
    }

    #[test]
    fn archival_bitmap_rejected() {
        let bm = Bitmap::new(16, 16);
        let err = PageEncoder::from_bitmap(&bm)
            .with_quality(EncodeQuality::Archival)
            .encode()
            .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("Archival"));
    }

    #[test]
    fn empty_bitmap_round_trips() {
        let bm = Bitmap::new(1, 1);
        let bytes = PageEncoder::from_bitmap(&bm).encode().expect("encode");
        let form = parse_form(&bytes).expect("parse");
        assert_eq!(&form.form_type, b"DJVU");
    }

    #[test]
    fn encode_rejects_pixmap_width_exceeding_u16() {
        // width = 70000 > 65535: try_from fails → EncodeError::Unsupported
        let pm = Pixmap {
            width: 70_000,
            height: 1,
            data: vec![],
        };
        let err = PageEncoder::from_pixmap(&pm)
            .with_quality(EncodeQuality::Quality)
            .encode()
            .unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("width") || msg.contains("65"),
            "unexpected: {msg}"
        );
    }

    #[test]
    fn encode_rejects_bitmap_height_exceeding_u16() {
        // height = 70000 > 65535: try_from fails → EncodeError::Unsupported
        let bm = Bitmap {
            width: 1,
            height: 70_000,
            data: vec![0u8; 70_000 / 8 + 1],
        };
        let err = PageEncoder::from_bitmap(&bm)
            .with_quality(EncodeQuality::Lossless)
            .encode()
            .unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("height") || msg.contains("65"),
            "unexpected: {msg}"
        );
    }

    #[test]
    fn quality_color_emits_info_sjbz_bg44() {
        // 64×64 page: white background with a black 16×16 ink square.
        let mut pm = Pixmap::white(64, 64);
        for y in 16..32 {
            for x in 16..32 {
                pm.set_rgb(x, y, 0, 0, 0);
            }
        }

        let bytes = PageEncoder::from_pixmap(&pm)
            .with_dpi(200)
            .with_quality(EncodeQuality::Quality)
            .encode()
            .expect("encode");

        let form = parse_form(&bytes).expect("parse_form");
        assert_eq!(&form.form_type, b"DJVU");

        let mut has_info = false;
        let mut has_sjbz = false;
        let mut bg44_count = 0;
        for chunk in &form.chunks {
            match &chunk.id {
                b"INFO" => has_info = true,
                b"Sjbz" => has_sjbz = true,
                b"BG44" => bg44_count += 1,
                _ => {}
            }
        }
        assert!(has_info, "INFO chunk missing");
        assert!(has_sjbz, "Sjbz chunk missing");
        assert!(
            bg44_count > 0,
            "expected at least one BG44 chunk, got {bg44_count}"
        );
    }

    #[test]
    fn quality_color_emits_fgbz_for_colored_foreground() {
        let mut pm = Pixmap::white(64, 64);
        for y in 16..32 {
            for x in 16..32 {
                pm.set_rgb(x, y, 180, 20, 20);
            }
        }

        let bytes = PageEncoder::from_pixmap(&pm)
            .with_quality(EncodeQuality::Quality)
            .encode()
            .expect("encode");

        let form = parse_form(&bytes).expect("parse_form");
        let fgbz = form
            .chunks
            .iter()
            .find(|chunk| &chunk.id == b"FGbz")
            .expect("FGbz chunk present");
        let (palette, indices) = crate::fgbz_encode::decode_fgbz(fgbz.data).expect("decode FGbz");
        assert_eq!(palette.len(), 1);
        assert!(indices.is_empty());
        assert!(palette[0].r > 0, "foreground red should be preserved");
    }

    #[test]
    fn quality_color_emits_per_blit_fgbz_indices() {
        let mut pm = Pixmap::white(80, 40);
        for y in 8..24 {
            for x in 8..24 {
                pm.set_rgb(x, y, 180, 20, 20);
            }
            for x in 48..64 {
                pm.set_rgb(x, y, 20, 40, 180);
            }
        }

        let bytes = PageEncoder::from_pixmap(&pm)
            .with_quality(EncodeQuality::Quality)
            .encode()
            .expect("encode");
        let doc = crate::djvu_document::DjVuDocument::parse(&bytes).expect("parse");
        let page = doc.page(0).expect("page");
        let fgbz = page.raw_chunk(b"FGbz").expect("FGbz present");
        let (palette, indices) = crate::fgbz_encode::decode_fgbz(fgbz).expect("decode FGbz");

        assert!(
            palette.len() >= 2,
            "expected at least two foreground colors, got {palette:?}"
        );
        assert!(
            indices.len() >= 2,
            "expected per-blit indices for two foreground components"
        );
        assert_ne!(
            indices[0], indices[1],
            "separate colored components should point at distinct palette entries"
        );

        let rendered = crate::Document::from_bytes(bytes)
            .expect("document")
            .page(0)
            .expect("page")
            .render()
            .expect("render");
        let left = rendered.get_rgb(12, 12);
        let right = rendered.get_rgb(52, 12);
        assert!(
            left.0 > left.2,
            "left foreground should render red-dominant, got {left:?}"
        );
        assert!(
            right.2 > right.0,
            "right foreground should render blue-dominant, got {right:?}"
        );
    }

    #[test]
    fn quality_color_accepts_adaptive_segment_options() {
        let pm = mixed_lighting_fixture();

        let bytes = PageEncoder::from_pixmap(&pm)
            .with_quality(EncodeQuality::Quality)
            .with_segment_options(adaptive_segment_options())
            .encode()
            .expect("encode");

        let doc = crate::djvu_document::DjVuDocument::parse(&bytes).expect("parse");
        let page = doc.page(0).expect("page");
        assert!(page.raw_chunk(b"Sjbz").is_some());
        assert!(!page.all_chunks(b"BG44").is_empty());
    }

    #[test]
    fn layered_shared_djbz_round_trips_with_incl() {
        // #452: two identical colour pages — their mask CCs are byte-exact across
        // pages, so they are promoted to one shared Djbz, and each page references
        // it via INCL while keeping its own BG44/FGbz.
        let pm = mixed_lighting_fixture();
        let pages = [pm.clone(), pm.clone()];
        let bytes = encode_djvm_layered_shared(&pages, EncodeQuality::Quality, 300, None, 2)
            .expect("layered shared encode");

        let doc = crate::djvu_document::DjVuDocument::parse(&bytes).expect("parse bundle");
        assert_eq!(doc.page_count(), 2);
        for i in 0..2 {
            let page = doc.page(i).expect("page");
            assert!(page.raw_chunk(b"Sjbz").is_some(), "page {i} Sjbz");
            assert!(!page.all_chunks(b"BG44").is_empty(), "page {i} BG44");
            assert!(
                page.raw_chunk(b"INCL").is_some(),
                "page {i} INCL → shared dict"
            );
            // The shared-dictionary Sjbz must still decode to the page mask.
            page.extract_mask()
                .expect("mask decode")
                .expect("mask present");
        }
        assert!(
            bytes.windows(4).any(|w| w == b"Djbz"),
            "shared Djbz form present"
        );
    }

    #[test]
    fn adaptive_segment_options_improve_decoded_mixed_lighting_fixture() {
        let pm = mixed_lighting_fixture();
        let fixed = PageEncoder::from_pixmap(&pm)
            .with_quality(EncodeQuality::Quality)
            .with_segment_options(SegmentOptions {
                bg_subsample: 6,
                ..SegmentOptions::default()
            })
            .encode()
            .expect("fixed encode");
        let adaptive = PageEncoder::from_pixmap(&pm)
            .with_quality(EncodeQuality::Quality)
            .with_segment_options(SegmentOptions {
                bg_subsample: 6,
                ..adaptive_segment_options()
            })
            .encode()
            .expect("adaptive encode");

        let fixed_render = render_encoded(&fixed);
        let adaptive_render = render_encoded(&adaptive);
        let fixed_err = mean_abs_rgb_diff(&pm, &fixed_render);
        let adaptive_err = mean_abs_rgb_diff(&pm, &adaptive_render);

        assert!(
            adaptive_err < fixed_err * 0.70,
            "adaptive decoded render should be closer to source ({adaptive_err:.2} vs {fixed_err:.2})"
        );
    }

    fn adaptive_segment_options() -> SegmentOptions {
        SegmentOptions {
            binarization: crate::segment::Binarization::Sauvola { window: 9, k: 0.34 },
            bg_inpaint: true,
            ..SegmentOptions::default()
        }
    }

    fn mixed_lighting_fixture() -> Pixmap {
        let mut pm = Pixmap::white(48, 24);
        for y in 0..24 {
            for x in 0..48 {
                let v = if x < 24 { 80 } else { 220 };
                pm.set_rgb(x, y, v, v, v);
            }
        }

        // Dark ink on dark paper.
        for y in 6..18 {
            pm.set_rgb(9, y, 40, 40, 40);
            pm.set_rgb(14, y, 40, 40, 40);
        }
        for x in 9..=14 {
            pm.set_rgb(x, 6, 40, 40, 40);
            pm.set_rgb(x, 12, 40, 40, 40);
        }

        // Light-gray ink on bright paper. Fixed threshold treats this as BG,
        // so the thin strokes wash into the BG44 sample cells.
        for y in 6..18 {
            pm.set_rgb(33, y, 140, 140, 140);
            pm.set_rgb(40, y, 140, 140, 140);
        }
        for x in 33..=40 {
            pm.set_rgb(x, 6, 140, 140, 140);
            pm.set_rgb(x, 12, 140, 140, 140);
            pm.set_rgb(x, 17, 140, 140, 140);
        }
        pm
    }

    fn render_encoded(bytes: &[u8]) -> Pixmap {
        let doc = crate::djvu_document::DjVuDocument::parse(bytes).expect("parse encoded doc");
        let page = doc.page(0).expect("page");
        let (width, height) = page.dimensions();
        let opts = crate::djvu_render::RenderOptions {
            width: u32::from(width),
            height: u32::from(height),
            ..crate::djvu_render::RenderOptions::default()
        };
        crate::djvu_render::render_pixmap(page, &opts).expect("render encoded page")
    }

    fn mean_abs_rgb_diff(expected: &Pixmap, actual: &Pixmap) -> f64 {
        assert_eq!(
            (expected.width, expected.height),
            (actual.width, actual.height)
        );
        let mut sum = 0u64;
        let mut n = 0u64;
        for (a, b) in expected
            .data
            .chunks_exact(4)
            .zip(actual.data.chunks_exact(4))
        {
            for c in 0..3 {
                sum += a[c].abs_diff(b[c]) as u64;
                n += 1;
            }
        }
        sum as f64 / n as f64
    }

    #[test]
    fn archival_color_emits_layered_djvu_with_fgbz() {
        let mut pm = Pixmap::white(48, 48);
        for y in 12..24 {
            for x in 12..24 {
                pm.set_rgb(x, y, 0, 90, 180);
            }
        }

        let bytes = PageEncoder::from_pixmap(&pm)
            .with_quality(EncodeQuality::Archival)
            .encode()
            .expect("encode");

        let doc = crate::djvu_document::DjVuDocument::parse(&bytes).expect("parse");
        let page = doc.page(0).expect("page");
        assert!(page.raw_chunk(b"Sjbz").is_some());
        assert!(!page.all_chunks(b"BG44").is_empty());
        assert!(page.raw_chunk(b"FGbz").is_some());
    }

    #[test]
    fn lossless_pixmap_rejected() {
        let pm = Pixmap::white(8, 8);
        let err = PageEncoder::from_pixmap(&pm)
            .with_quality(EncodeQuality::Lossless)
            .encode()
            .unwrap_err();
        assert!(format!("{err}").contains("Lossless"));
    }

    #[test]
    fn quality_bitmap_rejected() {
        let bm = Bitmap::new(8, 8);
        let err = PageEncoder::from_bitmap(&bm)
            .with_quality(EncodeQuality::Quality)
            .encode()
            .unwrap_err();
        assert!(format!("{err}").contains("Quality"));
    }

    #[test]
    fn quality_color_round_trips_through_document() {
        // End-to-end: encode a colour page at Quality, parse it back
        // through the high-level Document API, and confirm dimensions
        // + that the page has both an Sjbz and at least one BG44 chunk.
        let pm = Pixmap::white(32, 24);
        let bytes = PageEncoder::from_pixmap(&pm)
            .with_dpi(150)
            .with_quality(EncodeQuality::Quality)
            .encode()
            .expect("encode");

        let doc = crate::djvu_document::DjVuDocument::parse(&bytes).expect("parse");
        let page = doc.page(0).expect("page 0");
        assert_eq!(page.width(), 32);
        assert_eq!(page.height(), 24);
        assert_eq!(page.dpi(), 150);
        assert!(page.raw_chunk(b"Sjbz").is_some());
        assert!(!page.all_chunks(b"BG44").is_empty());
    }

    // ── TH44 thumbnail tests (layered encoder) ────────────────────────────────

    /// Layered bundle WITH thumbnails: each page FORM:DJVU contains TH44 chunk(s)
    /// that decode to a valid IW44 image at the expected reduced dimensions.
    #[test]
    fn layered_bundle_with_thumbnails_each_page_has_th44() {
        // Build two distinct colour pages.
        let mut p1 = Pixmap::white(64, 48);
        for y in 8..24 {
            for x in 8..24 {
                p1.set_rgb(x, y, 180, 20, 20);
            }
        }
        let p2 = Pixmap::white(64, 48);

        let bytes = encode_djvm_layered_shared_with_thumbnails(
            &[p1.clone(), p2.clone()],
            EncodeQuality::Quality,
            300,
            None,
            2,
            true,
        )
        .expect("encode layered with thumbnails");

        let doc = crate::djvu_document::DjVuDocument::parse(&bytes).expect("parse bundle");
        assert_eq!(doc.page_count(), 2);
        for i in 0..2 {
            let page = doc.page(i).expect("page");
            let thumb = page.thumbnail().expect("thumbnail() should not error");
            assert!(
                thumb.is_some(),
                "page {i} must carry a TH44 thumbnail when with_thumbnails=true"
            );
            let thumb = thumb.unwrap();
            let (tw, th) = crate::thumbnail::thumbnail_dimensions(
                if i == 0 { p1.width } else { p2.width },
                if i == 0 { p1.height } else { p2.height },
            );
            assert_eq!(
                thumb.width, tw,
                "page {i} thumbnail width should be {tw}, got {}",
                thumb.width
            );
            assert_eq!(
                thumb.height, th,
                "page {i} thumbnail height should be {th}, got {}",
                thumb.height
            );
        }
    }

    /// Layered bundle WITHOUT thumbnails: output must NOT contain any TH44 chunks.
    #[test]
    fn layered_bundle_without_thumbnails_has_no_th44() {
        let pm = Pixmap::white(64, 48);
        let bytes =
            encode_djvm_layered_shared(&[pm.clone(), pm], EncodeQuality::Quality, 300, None, 2)
                .expect("encode layered");

        let doc = crate::djvu_document::DjVuDocument::parse(&bytes).expect("parse bundle");
        assert_eq!(doc.page_count(), 2);
        for i in 0..2 {
            let page = doc.page(i).expect("page");
            let thumb = page.thumbnail().expect("thumbnail() should not error");
            assert!(
                thumb.is_none(),
                "page {i} must NOT carry a TH44 thumbnail when with_thumbnails=false"
            );
        }
    }
}
