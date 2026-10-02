//! Dictionary encoder options, `Jb2EncodeOptions`.

use super::*;

/// Tunable knobs for the JB2 dictionary encoder.
///
/// Default values reproduce the lossless behavior of [`encode_jb2_dict`]
/// and [`encode_jb2_dict_with_shared`].
#[derive(Debug, Clone, Copy)]
pub struct Jb2EncodeOptions {
    /// Hamming-distance threshold (as fraction of pixel count) for **lossy
    /// rec-7 substitution** (#224 Phase 4). When `> 0`, CCs that are not
    /// byte-exact but match a same-size dict entry within
    /// `pixel_count × lossy_threshold` flipped pixels are emitted as
    /// rec-7 (matched copy) — the decoder produces the dict entry's pixels
    /// instead of the original. Visual error per CC is bounded by the
    /// threshold; bytes shrink because rec-7 carries no refinement bitmap.
    ///
    /// `0.0` (default) = lossless: rec-7 fires only on byte-exact matches.
    /// `cjb2 -lossy` ships at roughly the equivalent of 0.04–0.05 here.
    ///
    /// **Measured operating points** (Branch B / round 19; `watchmaker`, a text
    /// scan where the JB2 mask is 67 % of the file; mask quality via the D1
    /// PSNR/SSIM harness):
    ///
    /// | `lossy_threshold` | Sjbz size | SSIM |
    /// |-------------------|-----------|------|
    /// | `0.02` | **−22 %** | 0.9993 |
    /// | `0.05` | −23 % | 0.9989 |
    /// | `0.08` | −24 % | 0.9986 |
    ///
    /// `0.02` is the sweet spot: a large size reduction at near-imperceptible
    /// loss (≈ 0.02 % of mask pixels flipped). Returns diminish sharply above it.
    /// See [`Jb2EncodeOptions::lossy_text`] for that preset. **Note:** the win is
    /// a *text*-document lever — on noisy high-dpi photo scans the same-size
    /// near-twin population is thin, so low thresholds barely shrink anything.
    pub lossy_threshold: f32,
    /// **Despeckle** pre-pass (cjb2's classic noise-removal move). `None`
    /// (default) keeps every connected component, exactly as extracted.
    /// `Some(max_px)` drops any component whose **foreground pixel count**
    /// (not bbox area — a diagonal speck's bbox overcounts) is `<= max_px`
    /// *before* clustering/dedup: the speck is never emitted as a symbol at
    /// all, so its dict-entry + coordinate-record cost disappears and it
    /// also stops diluting the near-twin population that
    /// [`lossy_threshold`](Self::lossy_threshold) matches against.
    ///
    /// This is lossy: the removed pixels are gone from the decoded page —
    /// there is no dict entry left to reconstruct them from. Intended for
    /// noisy high-dpi scans (binarization "salt" dust), where isolated
    /// 1–8 px blobs are overwhelmingly noise rather than content.
    ///
    /// **Measured operating points** (JB2_DESPECKLE; `pathogenic_bacteria_1896`,
    /// a 600 dpi noisy scan where prior levers — same-size and cross-size
    /// lossy substitution — found almost nothing to substitute; mask quality
    /// via the D1 SSIM harness):
    ///
    /// | `despeckle` | Sjbz size | SSIM |
    /// |-------------|-----------|------|
    /// | `2` | −0.94 % | 0.99950 |
    /// | `4` | −1.59 % | 0.99904 |
    /// | `8` | **−2.43 %** | 0.99845 |
    ///
    /// On clean text (`watchmaker`) despeckle at every tested level is a
    /// **byte-identical no-op** — real glyphs are all well above 8 px, so
    /// nothing is removed. Despeckle is a scan-specific lever: it is the
    /// first lossy lever found to move the noisy-scan corpus at all
    /// (`lossy_threshold` alone gives ≈ 0 % there — see its docs above)
    /// and does so at near-invisible cost (≤ 0.02 % of mask pixels flipped
    /// even at `8`). See [`Jb2EncodeOptions::lossy_scan`] for the combined
    /// preset and `PERF_EXPERIMENTS.md` (JB2_DESPECKLE) for the full sweep
    /// and a punctuation/diacritic-survival test.
    ///
    /// `None` (default) = lossless: no component is ever dropped,
    /// byte-identical to the shipped encoder.
    pub despeckle: Option<u32>,
    /// Experiment-only cross-size record-6 refinement (#322). `None` (default)
    /// keeps the shipped behavior — only record-1 (new) and record-7 (copy)
    /// are emitted. `Some(_)` enables the lossless cross-size refinement path
    /// described on [`CrossSizeRec6Probe`].
    #[cfg(feature = "experimental")]
    pub cross_size_rec6_probe: Option<CrossSizeRec6Probe>,
    /// Experiment-only **same-size** record-6 refinement (Phase A1 of
    /// `docs/jb2-size-gap-plan.md`). `None` (default) keeps the shipped
    /// behavior. `Some(frac)` diverts a fresh CC that has a **same-bounding-box**
    /// dictionary twin within `pixel_count × frac` flipped pixels to a lossless
    /// record-6 refinement (`wdiff = hdiff = 0`) against that twin, instead of a
    /// fresh record-1. Unlike the cross-size probe, no resampling is involved, so
    /// the refinement context stays pixel-aligned. Lossless (round-trip exact).
    #[cfg(feature = "experimental")]
    pub same_size_rec6: Option<f32>,
    /// Experiment-only center-aligned refinement; see [`AlignedRefine`].
    #[cfg(feature = "experimental")]
    pub aligned_refine: Option<AlignedRefine>,
}

impl Default for Jb2EncodeOptions {
    fn default() -> Self {
        Self {
            lossy_threshold: 0.0,
            despeckle: None,
            #[cfg(feature = "experimental")]
            cross_size_rec6_probe: None,
            #[cfg(feature = "experimental")]
            same_size_rec6: None,
            #[cfg(feature = "experimental")]
            aligned_refine: None,
        }
    }
}

impl Jb2EncodeOptions {
    /// Recommended **lossy** preset for text documents: `lossy_threshold = 0.02`.
    ///
    /// Originally hand-picked (round 19: ≈ −22% Sjbz at SSIM 0.9993) and since
    /// **OCR-validated as the derived optimum** (#572, round 99): a
    /// `threshold × despeckle` grid scored by the *minimum* per-page Tesseract
    /// char agreement vs the lossless mask over 4 text pages holds 100% only
    /// through `0.04` (breaking to 99.93% at `0.06`), and the derivation rule
    /// — highest fully-agreeing setting backed off one grid step — lands
    /// exactly here. Opt-in: the encoder stays lossless unless you choose this
    /// (or set [`lossy_threshold`](Self::lossy_threshold) yourself). Best for
    /// text scans; on noisy photo scans the near-twin population is thin so it
    /// saves little — see [`lossy_scan`](Self::lossy_scan), whose calibrated
    /// threshold is looser.
    pub fn lossy_text() -> Self {
        Self::with_lossy_threshold(0.02)
    }

    /// Set the [`lossy_threshold`](Self::lossy_threshold) (builder style),
    /// leaving every other knob at its default. `0.0` keeps lossless behavior.
    #[allow(clippy::needless_update)] // spread sets the experimental fields when compiled in
    pub fn with_lossy_threshold(threshold: f32) -> Self {
        Self {
            lossy_threshold: threshold,
            ..Self::default()
        }
    }

    /// Set [`despeckle`](Self::despeckle) (builder style), leaving every
    /// other knob at its default. `max_px` is the largest foreground-pixel
    /// count a component may have and still be dropped as a speck.
    #[allow(clippy::needless_update)]
    pub fn with_despeckle(max_px: u32) -> Self {
        Self {
            despeckle: Some(max_px),
            ..Self::default()
        }
    }

    /// Recommended **lossy** preset for noisy high-dpi scans (JB2_DESPECKLE):
    /// `despeckle = 8` combined with `lossy_threshold = 0.02`.
    ///
    /// Measured on `pathogenic_bacteria_1896` (600 dpi scan): despeckle at
    /// `8` gives **−2.43 % Sjbz at SSIM 0.99845** (≤ 0.02 % of mask pixels
    /// flipped) — the first lossy lever measured to move this corpus at all
    /// (same-size and cross-size near-twin substitution both found ≈ 0 %
    /// headroom there; see [`lossy_threshold`](Self::lossy_threshold)'s
    /// docs). The stacked `lossy_threshold = 0.02` adds negligible extra
    /// size on this scan corpus (its near-twin population stays thin
    /// regardless of despeckling) but costs nothing and helps on any
    /// mixed/text-like content on the same page. On clean text
    /// (`watchmaker`) despeckle at every tested level (2/4/8) is a
    /// byte-identical no-op — real glyphs are all well above 8 px — so this
    /// preset is safe to try on both, but its win is scan-specific. See
    /// `PERF_EXPERIMENTS.md` (JB2_DESPECKLE) for the full despeckle x
    /// lossy_threshold sweep and a punctuation/diacritic-survival test.
    ///
    /// Opt-in: the encoder stays lossless unless you choose this (or set
    /// `despeckle`/`lossy_threshold` yourself).
    #[allow(clippy::needless_update)]
    pub fn lossy_scan() -> Self {
        Self {
            despeckle: Some(8),
            // OCR-calibrated (#572, round 99): over 4 scan pages the minimum
            // per-page char agreement holds 100% through 0.08 (where Sjbz
            // finally moves: −4.4%) and breaks at 0.10, so the derived point —
            // highest fully-agreeing setting backed off one step — is 0.06
            // (−0.70% vs −0.10% at the historical hand-picked 0.02). The text
            // corpus is stricter (breaks at 0.06); use `lossy_text` there.
            lossy_threshold: 0.06,
            ..Self::default()
        }
    }

    /// Auto-policy preset for [`same_size_rec6`](Self::same_size_rec6)
    /// (JB2_AUTO_REC6, the Phase A3 follow-up of
    /// `docs/jb2-size-gap-plan.md`): runs the cheap, bounded
    /// [`probe_same_size_rec6_density`] against `bitmap` and enables
    /// same-size rec-6 at [`SAME_SIZE_REC6_AUTO_FRAC`] when the near-twin
    /// density is at/above [`SAME_SIZE_REC6_AUTO_DENSITY_THRESHOLD`], leaving
    /// it `None` (shipped lossless behavior) otherwise. Every other knob
    /// stays at its default.
    ///
    /// Call this **once per document** — e.g. on its first page, or any
    /// single representative page — and reuse the returned [`Jb2EncodeOptions`]
    /// for every page's [`encode_jb2_dict_with_options`] call. That way the
    /// probe's one fixed `extract_ccs` pass is paid once per document rather
    /// than once per page, and every page gets a consistent policy (matching
    /// the validated per-corpus numbers, which were measured with one
    /// decision applied across all of a document's pages).
    ///
    /// **Validated:** on `watchmaker` (text, density 39.6 %) fires and
    /// reproduces the ≈ −11.67 % Sjbz win, lossless round-trip on every page.
    /// On `pathogenic_bacteria_1896` (600 dpi scan, density 0.9 %) and
    /// `conquete_paix` (density 1.7 %) it stays off, so output is
    /// byte-identical to the default encoder. Opt-in and behind
    /// `experimental`; not a stable API and not enabled by default — the
    /// maintainer's A3 decision keeps `same_size_rec6` an experimental,
    /// explicit-opt-in lever (see `PERF_EXPERIMENTS.md`).
    #[cfg(feature = "experimental")]
    pub fn same_size_rec6_auto(bitmap: &Bitmap, shared_symbols: &[Bitmap]) -> Self {
        let density =
            probe_same_size_rec6_density(bitmap, shared_symbols, SAME_SIZE_REC6_AUTO_SAMPLE_CCS);
        Self {
            same_size_rec6: if density >= SAME_SIZE_REC6_AUTO_DENSITY_THRESHOLD {
                Some(SAME_SIZE_REC6_AUTO_FRAC)
            } else {
                None
            },
            ..Self::default()
        }
    }
}

/// The aligned-refinement setting carried by `opts` (experiment builds only).
pub(super) fn aligned_of(_opts: &Jb2EncodeOptions) -> Option<AlignedRefine> {
    #[cfg(feature = "experimental")]
    {
        _opts.aligned_refine
    }
    #[cfg(not(feature = "experimental"))]
    {
        None
    }
}
