//! Measurement-only statistics: refinement estimates and component stats.

use super::*;

/// Summary of an experiment-only cross-size refinement search.
///
/// This does not affect encoding. It estimates how many components currently
/// emitted as fresh record-1 symbols have a nearly matching dictionary symbol
/// with a slightly different bounding box.
#[cfg(feature = "experimental")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CrossSizeRefinementStats {
    /// Total connected components seen in reading order.
    pub total_ccs: usize,
    /// Components that do not have an exact same-size dictionary hit.
    pub fresh_ccs: usize,
    /// Fresh components large enough to consider for refinement.
    pub eligible_fresh_ccs: usize,
    /// Eligible fresh components with at least one near-size candidate.
    pub candidate_ccs: usize,
    /// Candidate components whose best normalized Hamming score is within
    /// the caller-provided pixel fraction.
    pub near_matches: usize,
    /// Sum of source component pixels for near matches.
    pub near_match_pixels: u64,
    /// Best normalized Hamming score observed for every near-size candidate.
    pub best_hamming: Vec<u32>,
    /// Approximate bytes current record-1 emissions spend on near-match symbols.
    ///
    /// This includes direct bitmap payload bytes plus a small fixed record
    /// overhead approximation. It intentionally excludes coordinate coding
    /// because both record-1 and hypothetical record-6 still place a symbol.
    pub estimated_rec1_bytes: u64,
    /// Approximate bytes a hypothetical cross-size record-6 path would spend
    /// for the same near-match symbols.
    ///
    /// Includes an estimated symbol-index/context overhead and a packed
    /// refinement-difference payload estimate. No encoder behavior depends on
    /// this value; it is measurement-only.
    pub estimated_cross_size_rec6_bytes: u64,
    /// Estimated byte delta: `cross_size_rec6 - current_rec1`.
    /// Negative means the hypothetical path may save bytes.
    pub estimated_byte_delta: i64,
}

#[cfg(feature = "experimental")]
pub(super) fn packed_bytes_for_pixels(pixels: u64) -> u64 {
    pixels.div_ceil(8)
}

#[cfg(feature = "experimental")]
pub(super) fn index_overhead_bytes(dict_len: usize) -> u64 {
    let bits = usize::BITS - dict_len.max(1).leading_zeros();
    u64::from(bits).div_ceil(8)
}

#[cfg(feature = "experimental")]
pub(super) fn estimate_record1_symbol_bytes(symbol: &Bitmap) -> u64 {
    const RECORD1_OVERHEAD_BYTES: u64 = 3; // record type + width + height, approximate.
    symbol.data.len() as u64 + RECORD1_OVERHEAD_BYTES
}

#[cfg(feature = "experimental")]
pub(super) fn estimate_cross_size_rec6_symbol_bytes(hamming: u32, dict_len: usize) -> u64 {
    const RECORD6_OVERHEAD_BYTES: u64 = 5; // record type + wdiff/hdiff + refinement flags, approximate.
    RECORD6_OVERHEAD_BYTES
        + index_overhead_bytes(dict_len)
        + packed_bytes_for_pixels(u64::from(hamming))
}

/// Estimate cross-size refinement headroom without changing encoder output.
///
/// The JB2 format can encode record-6 refinements where the reference symbol
/// has a different `(w, h)`, but the shipped encoder intentionally only uses
/// exact record-7 copies. This helper mirrors the dictionary growth of
/// [`encode_jb2_dict_with_shared`] and, for fresh symbols, scores nearby
/// dictionary entries after nearest-neighbor normalization into the candidate
/// component's dimensions.
///
/// `max_dim_delta` limits candidates to entries with width/height differing
/// by at most that many pixels. `max_hamming_fraction` is the accepted
/// normalized Hamming budget relative to the candidate's pixel count.
#[cfg(feature = "experimental")]
pub fn analyze_jb2_cross_size_refinement(
    bitmap: &Bitmap,
    shared_symbols: &[Bitmap],
    max_dim_delta: u32,
    max_hamming_fraction: f32,
) -> CrossSizeRefinementStats {
    let mut stats = CrossSizeRefinementStats::default();
    if bitmap.width == 0 || bitmap.height == 0 {
        return stats;
    }

    let ccs = extract_ccs(bitmap);
    let mut order: Vec<usize> = (0..ccs.len()).collect();
    let bucket = (SAME_LINE_BASELINE_TOL.max(1)) as u32;
    order.sort_by_key(|&i| {
        let cc = &ccs[i];
        let bottom = cc.y + cc.bitmap.height;
        (bottom / bucket, cc.x)
    });

    let mut dedup: BTreeMap<(u32, u32, Vec<u8>), usize> = BTreeMap::new();
    let mut dict_entries: Vec<Bitmap> = Vec::new();
    for sym in shared_symbols {
        let idx = dict_entries.len();
        dedup.insert((sym.width, sym.height, sym.data.clone()), idx);
        dict_entries.push(sym.clone());
    }
    let mut by_size: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for (idx, sym) in dict_entries.iter().enumerate() {
        by_size
            .entry((sym.width, sym.height))
            .or_default()
            .push(idx);
    }

    for &cc_idx in &order {
        let cc = &ccs[cc_idx];
        stats.total_ccs += 1;

        let key = (cc.bitmap.width, cc.bitmap.height, cc.bitmap.data.clone());
        if dedup.contains_key(&key) {
            continue;
        }

        stats.fresh_ccs += 1;
        let pixels = u64::from(cc.bitmap.width) * u64::from(cc.bitmap.height);
        if pixels >= REFINEMENT_MIN_PIXELS {
            stats.eligible_fresh_ccs += 1;
            let mut best: Option<u32> = None;
            let min_w = cc.bitmap.width.saturating_sub(max_dim_delta);
            let max_w = cc.bitmap.width.saturating_add(max_dim_delta);
            let min_h = cc.bitmap.height.saturating_sub(max_dim_delta);
            let max_h = cc.bitmap.height.saturating_add(max_dim_delta);
            for w in min_w..=max_w {
                for h in min_h..=max_h {
                    if w == cc.bitmap.width && h == cc.bitmap.height {
                        continue;
                    }
                    let Some(indices) = by_size.get(&(w, h)) else {
                        continue;
                    };
                    for &idx in indices {
                        let d = scaled_hamming(&cc.bitmap, &dict_entries[idx]);
                        best = Some(best.map_or(d, |b| b.min(d)));
                    }
                }
            }
            if let Some(best) = best {
                stats.candidate_ccs += 1;
                stats.best_hamming.push(best);
                let max_diff = ((pixels as f64) * (max_hamming_fraction as f64)).round() as u32;
                if best <= max_diff {
                    stats.near_matches += 1;
                    stats.near_match_pixels += pixels;
                    let rec1 = estimate_record1_symbol_bytes(&cc.bitmap);
                    let rec6 = estimate_cross_size_rec6_symbol_bytes(best, dict_entries.len());
                    stats.estimated_rec1_bytes += rec1;
                    stats.estimated_cross_size_rec6_bytes += rec6;
                    stats.estimated_byte_delta += rec6 as i64 - rec1 as i64;
                }
            }
        }

        let next_idx = dict_entries.len();
        dedup.insert(key, next_idx);
        by_size
            .entry((cc.bitmap.width, cc.bitmap.height))
            .or_default()
            .push(next_idx);
        dict_entries.push(cc.bitmap.clone());
    }

    stats
}

/// Summary of an experiment-only **same-size** refinement search (Phase A0 of
/// `docs/jb2-size-gap-plan.md`).
///
/// Measurement only — does not change encoding. It counts, for the components
/// the default encoder emits as fresh record-1 symbols, how many have a
/// **same-bounding-box** dictionary twin within a small Hamming distance. Those
/// are the candidates for a lossless same-size record-6 refinement, the one
/// untried lever that avoids the resampling misalignment that made cross-size
/// rec-6 (#322) lose bytes. It proves a *population*, not a byte outcome — the
/// #301 lesson is that only a real emitter (Phase A1/A2) proves bytes.
#[cfg(feature = "experimental")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SameSizeRefinementStats {
    /// Total connected components seen in reading order.
    pub total_ccs: usize,
    /// Components with no exact same-`(w, h, data)` dictionary hit (i.e. the
    /// ones the default encoder emits as a fresh record-1 symbol).
    pub fresh_ccs: usize,
    /// Fresh components large enough to consider for refinement
    /// (`pixels >= REFINEMENT_MIN_PIXELS`).
    pub eligible_fresh_ccs: usize,
    /// Eligible fresh components with at least one same-size dictionary entry to
    /// score against.
    pub candidate_ccs: usize,
    /// Candidate components whose best (minimum) same-size Hamming distance is
    /// within 2 % / 5 % / 10 % of the component's pixel count.
    pub near_le_2pct: usize,
    pub near_le_5pct: usize,
    pub near_le_10pct: usize,
    /// Sum of component pixels for the ≤5 % near-twins (a proxy for how much
    /// direct-bitmap payload a refinement path could shrink).
    pub near_le_5pct_pixels: u64,
    /// Sum of the raw best Hamming *bytes* (`ceil(hamming/8)`) for ≤5 % near-twins
    /// — a crude floor on the refinement-bitmap payload if it coded one bit per
    /// differing pixel (the real ZP cost differs; this is only a scale hint).
    pub near_le_5pct_hamming_bytes: u64,
    /// Best Hamming fraction in per-mille (‰) for every candidate component, for
    /// histogramming the distance distribution.
    pub best_hamming_permille: Vec<u32>,
}

/// Measure the same-size refinement candidate population without changing output
/// (Phase A0). Mirrors the default encoder's exact-dedup dictionary growth
/// (`encode_jb2_dict_with_shared`: exact record-7 copy or fresh record-1, no
/// refinement), then for every fresh, eligible component scores its minimum
/// Hamming distance against same-`(w, h)` dictionary entries.
#[cfg(feature = "experimental")]
pub fn analyze_jb2_same_size_refinement(
    bitmap: &Bitmap,
    shared_symbols: &[Bitmap],
) -> SameSizeRefinementStats {
    same_size_refinement_scan(bitmap, shared_symbols, None)
}

/// Shared scan core behind [`analyze_jb2_same_size_refinement`] (full
/// document, `fresh_cc_limit: None`) and [`probe_same_size_rec6_density`]
/// (bounded, `Some(max_ccs)` — the auto-policy's cheap density probe, JB2_AUTO_REC6).
/// Mirrors the default encoder's exact-dedup dictionary growth, then for
/// every fresh, eligible component scores its minimum Hamming distance
/// against same-`(w, h)` dictionary entries. When `fresh_cc_limit` is set,
/// scanning stops as soon as that many *fresh* CCs have been examined —
/// bounding the Hamming-scoring cost independent of page size, since the one
/// unavoidable fixed cost (`extract_ccs`) is paid by the real encoder anyway.
#[cfg(feature = "experimental")]
pub(super) fn same_size_refinement_scan(
    bitmap: &Bitmap,
    shared_symbols: &[Bitmap],
    fresh_cc_limit: Option<usize>,
) -> SameSizeRefinementStats {
    let mut stats = SameSizeRefinementStats::default();
    if bitmap.width == 0 || bitmap.height == 0 {
        return stats;
    }

    let ccs = extract_ccs(bitmap);
    // Same reading-order sort the real encoder uses, so `first_seen` reference
    // selection matches the shipped path.
    let mut order: Vec<usize> = (0..ccs.len()).collect();
    let bucket = (SAME_LINE_BASELINE_TOL.max(1)) as u32;
    order.sort_by_key(|&i| {
        let cc = &ccs[i];
        let bottom = cc.y + cc.bitmap.height;
        (bottom / bucket, cc.x)
    });

    // Exact-dedup dictionary, seeded from the shared symbols, exactly as the
    // encoder builds it (symbol_hash key + by_size index).
    let mut dedup: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    let mut dict_entries: Vec<&Bitmap> = Vec::new();
    let mut by_size: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for sym in shared_symbols {
        let idx = dict_entries.len();
        dedup
            .entry(symbol_hash(sym.width, sym.height, &sym.data))
            .or_default()
            .push(idx);
        by_size
            .entry((sym.width, sym.height))
            .or_default()
            .push(idx);
        dict_entries.push(sym);
    }

    for &cc_idx in &order {
        let cc = &ccs[cc_idx];
        stats.total_ccs += 1;
        let bm = &cc.bitmap;
        let dkey = symbol_hash(bm.width, bm.height, &bm.data);
        let exact = dedup.get(&dkey).is_some_and(|cands| {
            cands.iter().copied().any(|i| {
                let d = dict_entries[i];
                d.width == bm.width && d.height == bm.height && d.data == bm.data
            })
        });
        if exact {
            // Default encoder: record-7 copy, dict unchanged.
            continue;
        }

        stats.fresh_ccs += 1;
        let pixels = u64::from(bm.width) * u64::from(bm.height);
        if pixels >= REFINEMENT_MIN_PIXELS {
            stats.eligible_fresh_ccs += 1;
            if let Some(indices) = by_size.get(&(bm.width, bm.height)) {
                let mut best: Option<u32> = None;
                for &idx in indices {
                    let d = packed_hamming(&bm.data, &dict_entries[idx].data);
                    best = Some(best.map_or(d, |b| b.min(d)));
                }
                if let Some(best) = best {
                    stats.candidate_ccs += 1;
                    let frac_permille = ((u64::from(best) * 1000) / pixels.max(1)) as u32;
                    stats.best_hamming_permille.push(frac_permille);
                    if frac_permille <= 20 {
                        stats.near_le_2pct += 1;
                    }
                    if frac_permille <= 50 {
                        stats.near_le_5pct += 1;
                        stats.near_le_5pct_pixels += pixels;
                        stats.near_le_5pct_hamming_bytes += u64::from(best).div_ceil(8);
                    }
                    if frac_permille <= 100 {
                        stats.near_le_10pct += 1;
                    }
                }
            }
        }

        // Default encoder: fresh record-1, added to the dict.
        let next_idx = dict_entries.len();
        dedup.entry(dkey).or_default().push(next_idx);
        by_size
            .entry((bm.width, bm.height))
            .or_default()
            .push(next_idx);
        dict_entries.push(bm);

        if let Some(limit) = fresh_cc_limit
            && stats.fresh_ccs >= limit
        {
            break;
        }
    }

    stats
}

/// Default bounded sample size for [`probe_same_size_rec6_density`] and
/// [`Jb2EncodeOptions::same_size_rec6_auto`]: the probe stops scanning once
/// this many *fresh* CCs have been examined. Large enough to be a stable
/// estimate — watchmaker's whole-page fresh population is 3 475 components,
/// and 1 000 samples converges well before that — yet small enough that the
/// probe's Hamming-scoring cost is capped regardless of page size: the
/// 821 330-fresh-CC `pathogenic_bacteria_1896` page pays the same bounded
/// scan a small page would.
#[cfg(feature = "experimental")]
pub const SAME_SIZE_REC6_AUTO_SAMPLE_CCS: usize = 1000;

/// Density threshold (fraction of sampled fresh CCs with a same-size ≤5 %
/// Hamming twin) at/above which [`Jb2EncodeOptions::same_size_rec6_auto`]
/// enables same-size rec-6.
///
/// Calibrated against **real emitted-byte deltas** on four corpora (not just
/// population counts — see the #301 lesson), JB2_AUTO_REC6 in
/// `PERF_EXPERIMENTS.md`:
///
/// | Corpus | density (≤5 % near-twins / fresh) | Sjbz delta at frac 2 % |
/// |--------|------------------------------------|------------------------|
/// | watchmaker | 39.6 % | **−11.67 %** |
/// | cable_1973_100133 | 12.4 % | −0.43 % |
/// | conquete_paix | 1.7 % | **+0.49 %** (loss) |
/// | pathogenic_bacteria_1896 | 0.9 % | +0.00 % (flat) |
///
/// The real-byte outcome flips from a loss to a win between 1.7 % and
/// 12.4 % density. `0.05` sits with roughly 2.9× margin above the measured
/// loss and 2.5× margin below the measured win — enabling on `cable` (a
/// small extra win) and `watchmaker` (the large win) while staying off for
/// `conquete_paix` and `pathogenic` (avoiding their losses/flat result).
#[cfg(feature = "experimental")]
pub const SAME_SIZE_REC6_AUTO_DENSITY_THRESHOLD: f32 = 0.05;

/// Refinement fraction [`Jb2EncodeOptions::same_size_rec6_auto`] applies once
/// density clears [`SAME_SIZE_REC6_AUTO_DENSITY_THRESHOLD`] — the validated
/// sweet spot from round 18 (a tighter threshold wins more: 2 % > 5 % > 8 %
/// on watchmaker).
#[cfg(feature = "experimental")]
pub const SAME_SIZE_REC6_AUTO_FRAC: f32 = 0.02;

/// Cheap, bounded density probe backing [`Jb2EncodeOptions::same_size_rec6_auto`]
/// (Phase A3 follow-up of `docs/jb2-size-gap-plan.md`, JB2_AUTO_REC6).
///
/// Scans at most `max_ccs` *fresh* connected components (same reading order
/// the encoder uses) and returns the fraction of them with a same-size
/// Hamming twin within 5 % — the metric validated in round 17/18 as
/// predictive of a real byte win (see
/// [`SAME_SIZE_REC6_AUTO_DENSITY_THRESHOLD`]'s table). Capping the scan
/// keeps the probe's incremental cost independent of page size; the fixed
/// `extract_ccs` pass is one the real encoder pays regardless of this
/// option, so the probe's marginal cost over a plain encode is just that
/// bounded Hamming scan.
#[cfg(feature = "experimental")]
pub fn probe_same_size_rec6_density(
    bitmap: &Bitmap,
    shared_symbols: &[Bitmap],
    max_ccs: usize,
) -> f32 {
    let stats = same_size_refinement_scan(bitmap, shared_symbols, Some(max_ccs.max(1)));
    if stats.fresh_ccs == 0 {
        return 0.0;
    }
    stats.near_le_5pct as f32 / stats.fresh_ccs as f32
}

/// Per-CC accounting of which JB2 record type a single page would emit
/// against a given shared dictionary, without performing the actual encode.
///
/// Phase 2.5 measurement aid (#194): mirrors the action-selection branch in
/// [`encode_jb2_dict_with_shared`] (rec-7 exact / rec-6 refinement / rec-1
/// new) and reports counts, pixel totals, and Hamming-distance distribution
/// for the rec-6 emissions, distinguishing references that resolve into the
/// shared Djbz vs ones that resolve into the page-local running dict.
///
/// Use this to answer questions like "how many CCs would actually benefit
/// from a tighter refinement threshold" or "how large is the rec-7 win
/// from the shared dict on this corpus" without round-tripping bytes.
#[derive(Debug, Default, Clone)]
pub struct CcStats {
    pub total_ccs: usize,
    /// rec-7: byte-exact match found in the running dict.
    pub rec_7_exact: usize,
    /// rec-6 against a slot inside the shared (cross-page) Djbz.
    pub rec_6_refine_shared: usize,
    /// rec-6 against a slot emitted earlier on the same page.
    pub rec_6_refine_local: usize,
    /// rec-1: no usable match, fresh emission.
    pub rec_1_new: usize,
    /// Hamming distances of rec-6 matches (one entry per rec-6 CC).
    pub rec_6_hamming: Vec<u32>,
    /// Pixel-count totals split by record type.
    pub pixels_rec_1: u64,
    pub pixels_rec_6: u64,
    pub pixels_rec_7: u64,
}

/// Walk `page`'s connected components in encoder order and accumulate
/// per-CC accounting against `shared_symbols` using the same action-
/// selection rules as [`encode_jb2_dict_with_shared`]. Pure observation —
/// no bytes are emitted.
pub fn analyze_jb2_cc_stats(page: &Bitmap, shared_symbols: &[Bitmap]) -> CcStats {
    let mut stats = CcStats::default();
    if page.width == 0 || page.height == 0 {
        return stats;
    }

    let ccs = extract_ccs(page);
    let mut order: Vec<usize> = (0..ccs.len()).collect();
    let bucket = (SAME_LINE_BASELINE_TOL.max(1)) as u32;
    order.sort_by_key(|&i| {
        let cc = &ccs[i];
        let bottom = cc.y + cc.bitmap.height;
        (bottom / bucket, cc.x)
    });

    let mut dedup: BTreeMap<(u32, u32, Vec<u8>), usize> = BTreeMap::new();
    let mut dict_entries: Vec<Bitmap> = Vec::new();
    let mut by_size: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for sym in shared_symbols {
        let idx = dict_entries.len();
        dedup.insert((sym.width, sym.height, sym.data.clone()), idx);
        by_size
            .entry((sym.width, sym.height))
            .or_default()
            .push(idx);
        dict_entries.push(sym.clone());
    }

    for &cc_idx in &order {
        let cc = &ccs[cc_idx];
        let pixels = (cc.bitmap.width as u64) * (cc.bitmap.height as u64);
        stats.total_ccs += 1;

        let key = (cc.bitmap.width, cc.bitmap.height, cc.bitmap.data.clone());
        if let Some(idx) = dedup.get(&key).copied() {
            stats.rec_7_exact += 1;
            stats.pixels_rec_7 += pixels;
            // rec-7 emits no new dict entry, no need to update tables.
            let _ = idx;
            continue;
        }

        stats.rec_1_new += 1;
        stats.pixels_rec_1 += pixels;
        let idx = dict_entries.len();
        dedup.insert(key, idx);
        by_size
            .entry((cc.bitmap.width, cc.bitmap.height))
            .or_default()
            .push(idx);
        dict_entries.push(cc.bitmap.clone());
    }

    stats
}
