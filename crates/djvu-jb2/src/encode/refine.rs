//! Finding a dictionary reference for a component: copy and refinement matches.

use super::*;

/// Minimum pixel area for a CC to be considered for refinement matching.
///
/// Sub-32-pixel CCs (typical: dust, single anti-aliasing fragments) encode
/// in only a handful of bytes via record-1; the per-record overhead of a
/// record-6 (matched-refinement coordinate header + 11-bit refinement
/// context state) outweighs any saving even at low Hamming distance.
pub(super) const REFINEMENT_MIN_PIXELS: u64 = 32;

#[cfg(feature = "experimental")]
pub(super) fn scaled_hamming(cand: &Bitmap, reference: &Bitmap) -> u32 {
    let mut diff = 0u32;
    for y in 0..cand.height {
        let ry = (u64::from(y) * u64::from(reference.height) / u64::from(cand.height)) as u32;
        for x in 0..cand.width {
            let rx = (u64::from(x) * u64::from(reference.width) / u64::from(cand.width)) as u32;
            if cand.get(x, y) != reference.get(rx, ry) {
                diff += 1;
            }
        }
    }
    diff
}

/// Find the closest same-size dict entry within a Hamming-distance budget,
/// for use as a **lossy copy** target (record-7) — the encoder pretends the
/// near-duplicate is byte-exact, the decoder produces the dict entry's pixels
/// instead of the original CC. Visual loss is bounded by the threshold.
///
/// Used by [`Jb2EncodeOptions::lossy_threshold`] (#224 Phase 4); independent
/// of `find_refinement_ref`, which gated record-6 (lossless refinement).
pub(super) fn find_lossy_copy_ref(
    cand: &Bitmap,
    dict_entries: &[&Bitmap],
    same_size_indices: &[usize],
    threshold: f32,
) -> Option<usize> {
    if same_size_indices.is_empty() || threshold <= 0.0 {
        return None;
    }
    let pixel_count = (cand.width as u64) * (cand.height as u64);
    if pixel_count < REFINEMENT_MIN_PIXELS {
        return None;
    }
    // Hamming budget in pixel count, rounded to the nearest integer.
    let max_diff = ((pixel_count as f64) * (threshold as f64)).round() as u32;
    let mut best: Option<(usize, u32)> = None;
    for &i in same_size_indices {
        let ref_bm = dict_entries[i];
        debug_assert_eq!(ref_bm.width, cand.width);
        debug_assert_eq!(ref_bm.height, cand.height);
        let d = packed_hamming(&cand.data, &ref_bm.data);
        if d > max_diff {
            continue;
        }
        match best {
            None => best = Some((i, d)),
            Some((_, bd)) if d < bd => best = Some((i, d)),
            _ => {}
        }
    }
    best.map(|(i, _)| i)
}

/// Find the closest **cross-size** dict entry suitable for a lossless record-6
/// matched refinement (#322 experiment).
///
/// Candidates are dict entries whose width and height each differ from `cand`
/// by at most `max_dim_delta` (and are not exactly `cand`'s size). Distance is
/// scored with [`scaled_hamming`] — nearest-neighbor resampling of the
/// candidate into `cand`'s grid — and accepted when within
/// `pixel_count × max_hamming_fraction` flipped pixels. Returns the dict index
/// of the best (lowest-distance) accepted candidate.
///
/// The match only selects the *reference* glyph; the emitted refinement bitmap
/// reproduces `cand` exactly, so this is lossless regardless of the score.
#[cfg(feature = "experimental")]
pub(super) fn find_cross_size_refine_ref(
    cand: &Bitmap,
    dict_entries: &[&Bitmap],
    by_size: &BTreeMap<(u32, u32), Vec<usize>>,
    max_dim_delta: u32,
    max_hamming_fraction: f32,
) -> Option<usize> {
    let pixel_count = (cand.width as u64) * (cand.height as u64);
    if pixel_count < REFINEMENT_MIN_PIXELS {
        return None;
    }
    let max_diff = ((pixel_count as f64) * (max_hamming_fraction as f64)).round() as u32;
    let min_w = cand.width.saturating_sub(max_dim_delta);
    let max_w = cand.width.saturating_add(max_dim_delta);
    let min_h = cand.height.saturating_sub(max_dim_delta);
    let max_h = cand.height.saturating_add(max_dim_delta);
    let mut best: Option<(usize, u32)> = None;
    for w in min_w..=max_w {
        for h in min_h..=max_h {
            if w == cand.width && h == cand.height {
                continue;
            }
            let Some(indices) = by_size.get(&(w, h)) else {
                continue;
            };
            for &idx in indices {
                let d = scaled_hamming(cand, dict_entries[idx]);
                if d > max_diff {
                    continue;
                }
                match best {
                    None => best = Some((idx, d)),
                    Some((_, bd)) if d < bd => best = Some((idx, d)),
                    _ => {}
                }
            }
        }
    }
    best.map(|(i, _)| i)
}

/// Hamming distance between `cand` and `reference` under the refinement
/// decoder's alignment (centers matched as in `encode_bitmap_ref`), counted over
/// `cand`'s box. Stops early once the count exceeds `limit`.
pub(super) fn aligned_hamming(cand: &Bitmap, reference: &Bitmap, limit: u32) -> u32 {
    let ch = cand.height as i32;
    let mh = reference.height as i32;
    let row_shift = ((mh - 1) >> 1) - ((ch - 1) >> 1);
    let col_shift = ((reference.width as i32 - 1) >> 1) - ((cand.width as i32 - 1) >> 1);
    let cs = cand.row_stride();
    let ms = reference.row_stride();
    let last_mask = |w: u32| {
        if w.is_multiple_of(8) {
            0xFF
        } else {
            0xFFu8 << (8 - w % 8)
        }
    };
    let (c_last, m_last) = (last_mask(cand.width), last_mask(reference.width));
    let mut diff = 0u32;
    for y in 0..ch {
        let crow = &cand.data[y as usize * cs..(y as usize + 1) * cs];
        // Top-down row `y` is Jbm row `ch - 1 - y`; the reference row follows.
        let my = mh - 1 - (ch - 1 - y + row_shift);
        let mrow = (0..mh)
            .contains(&my)
            .then(|| &reference.data[my as usize * ms..(my as usize + 1) * ms]);
        // Reference byte `i` with its padding bits cleared; 0 outside the row.
        let mbyte = |i: i32| -> u16 {
            match mrow {
                Some(row) if i >= 0 && (i as usize) < ms => {
                    let b = row[i as usize];
                    u16::from(if i as usize == ms - 1 { b & m_last } else { b })
                }
                _ => 0,
            }
        };
        for (j, &c) in crow.iter().enumerate() {
            // Reference bits `8j + col_shift ..` line up with cand byte `j`.
            let s = 8 * j as i32 + col_shift;
            let (i, r) = (s.div_euclid(8), s.rem_euclid(8));
            let m = (((mbyte(i) << 8) | mbyte(i + 1)) << r >> 8) as u8;
            let mask = if j == cs - 1 { c_last } else { 0xFF };
            diff += ((c ^ m) & mask).count_ones();
        }
        if diff > limit {
            return diff;
        }
    }
    diff
}

/// Append `bm`'s rows to `out`, one u64 each with the leftmost pixel in the
/// top bit and the row padding bits cleared. Returns `false`, appending
/// nothing, when `bm` is not 1 to 64 pixels wide.
pub(super) fn push_row_words(bm: &Bitmap, out: &mut Vec<u64>) -> bool {
    if !(1..=64).contains(&bm.width) {
        return false;
    }
    let width_mask = !0u64 << (64 - bm.width);
    for row in bm
        .data
        .chunks_exact(bm.row_stride())
        .take(bm.height as usize)
    {
        let v = row
            .iter()
            .enumerate()
            .fold(0u64, |v, (k, &b)| v | u64::from(b) << (56 - 8 * k));
        out.push(v & width_mask);
    }
    true
}

/// [`aligned_hamming`] on [`push_row_words`] rows: `cand` is `cw` pixels
/// wide, `reference` `mw`. Aligning a row is one shift, its distance one
/// popcount.
pub(super) fn aligned_hamming_words(
    cand: &[u64],
    cw: u32,
    reference: &[u64],
    mw: u32,
    limit: u32,
) -> u32 {
    let ch = cand.len() as i32;
    let mh = reference.len() as i32;
    let row_shift = ((mh - 1) >> 1) - ((ch - 1) >> 1);
    let col_shift = ((mw as i32 - 1) >> 1) - ((cw as i32 - 1) >> 1);
    let cand_mask = !0u64 << (64 - cw);
    let mut diff = 0u32;
    for (y, &c) in cand.iter().enumerate() {
        let my = mh - 1 - (ch - 1 - y as i32 + row_shift);
        let m = match reference.get(my as usize) {
            // Reference bit `x + col_shift` lines up with cand bit `x`.
            Some(&m) if my >= 0 => {
                if col_shift >= 0 {
                    m << col_shift
                } else {
                    m >> -col_shift
                }
            }
            _ => 0,
        };
        diff += ((c ^ m) & cand_mask).count_ones();
        if diff > limit {
            return diff;
        }
    }
    diff
}

/// What the aligned refinement search keeps per dictionary entry: its ink
/// count (or [`NOT_REFINABLE`]) and, for entries at most 64 pixels wide,
/// its rows as [`push_row_words`] words, built once instead of per compare.
#[derive(Default)]
pub(super) struct RefineIndex {
    ink: Vec<u32>,
    /// Each entry's rows in `words`; empty for wide entries.
    rows: Vec<core::ops::Range<usize>>,
    words: Vec<u64>,
}

impl RefineIndex {
    /// Add the next dictionary entry, `bm`, with ink count `ink`.
    pub(super) fn push(&mut self, bm: &Bitmap, ink: u32) {
        let start = self.words.len();
        push_row_words(bm, &mut self.words);
        self.ink.push(ink);
        self.rows.push(start..self.words.len());
    }

    fn words(&self, idx: usize) -> Option<&[u64]> {
        let rows = self.rows[idx].clone();
        (!rows.is_empty()).then(|| &self.words[rows])
    }
}

/// `dict_ink` marker for a dict entry that must not be a refinement reference.
///
/// Both decoders align a refinement on the reference's content box: ours
/// crops dict entries to it, DjVuLibre aligns on the entry's bounding box. A
/// caller-supplied shared symbol with blank border rows or columns would
/// therefore decode against a different box than the encoder coded against.
pub(super) const NOT_REFINABLE: u32 = u32::MAX;

/// Whether every border row and column of `bm` holds ink, so the decoder's
/// content box equals the full bitmap.
pub(super) fn is_tight(bm: &Bitmap) -> bool {
    let (w, h) = (bm.width, bm.height);
    w > 0
        && h > 0
        && (0..w).any(|x| bm.get(x, 0))
        && (0..w).any(|x| bm.get(x, h - 1))
        && (0..h).any(|y| bm.get(0, y))
        && (0..h).any(|y| bm.get(w - 1, y))
}

/// Nearest dict entry within `max_dim_delta` per axis (same size included) by
/// [`aligned_hamming`], accepted within `area × max_hamming_fraction`.
///
/// `index` holds the black-pixel count of each dict entry. The ink
/// difference is a lower bound on the aligned distance (both ways when the
/// reference box fits inside `cand`'s), so most candidates are rejected
/// without a pixel scan. Buckets are scanned newest entry first, and the
/// budget shrinks to the best distance found so far.
pub(super) fn find_aligned_refine_ref(
    cand: &Bitmap,
    cand_ink: u32,
    dict_entries: &[&Bitmap],
    index: &RefineIndex,
    by_size: &BTreeMap<(u32, u32), Vec<usize>>,
    max_dim_delta: u32,
    max_hamming_fraction: f32,
) -> Option<usize> {
    let area = (cand.width as u64) * (cand.height as u64);
    if area < REFINEMENT_MIN_PIXELS {
        return None;
    }
    let mut limit = ((area as f64) * (max_hamming_fraction as f64)).round() as u32;
    let mut cand_words = Vec::new();
    let narrow = push_row_words(cand, &mut cand_words);
    let mut best: Option<usize> = None;
    for w in cand.width.saturating_sub(max_dim_delta)..=cand.width + max_dim_delta {
        for h in cand.height.saturating_sub(max_dim_delta)..=cand.height + max_dim_delta {
            let Some(indices) = by_size.get(&(w, h)) else {
                continue;
            };
            let inside = w <= cand.width && h <= cand.height;
            for &idx in indices.iter().rev() {
                let ink = index.ink[idx];
                if ink == NOT_REFINABLE {
                    continue;
                }
                let bound = if inside {
                    cand_ink.abs_diff(ink)
                } else {
                    cand_ink.saturating_sub(ink)
                };
                if bound > limit {
                    continue;
                }
                let reference = dict_entries[idx];
                let d = match index.words(idx) {
                    Some(words) if narrow => aligned_hamming_words(
                        &cand_words,
                        cand.width,
                        words,
                        reference.width,
                        limit,
                    ),
                    _ => aligned_hamming(cand, reference, limit),
                };
                if d < limit || (d == limit && best.is_none()) {
                    limit = d;
                    best = Some(idx);
                }
            }
        }
    }
    best
}

/// Find the closest **same-size** dict entry for a lossless record-6 matched
/// refinement (Phase A1 of `docs/jb2-size-gap-plan.md`).
///
/// Candidates are dict entries of exactly `cand`'s `(w, h)`. Distance is the
/// direct packed Hamming (no resampling — the reference aligns pixel-for-pixel),
/// accepted within `pixel_count × max_hamming_fraction` flipped pixels. Returns
/// the dict index of the nearest accepted candidate. The emitted refinement
/// bitmap reproduces `cand` exactly, so the result is lossless.
#[cfg(feature = "experimental")]
pub(super) fn find_same_size_refine_ref(
    cand: &Bitmap,
    dict_entries: &[&Bitmap],
    same_size_indices: &[usize],
    max_hamming_fraction: f32,
) -> Option<usize> {
    let pixel_count = (cand.width as u64) * (cand.height as u64);
    if pixel_count < REFINEMENT_MIN_PIXELS {
        return None;
    }
    let max_diff = ((pixel_count as f64) * (max_hamming_fraction as f64)).round() as u32;
    let mut best: Option<(usize, u32)> = None;
    for &idx in same_size_indices {
        let ref_bm = dict_entries[idx];
        debug_assert_eq!(ref_bm.width, cand.width);
        debug_assert_eq!(ref_bm.height, cand.height);
        let d = packed_hamming(&cand.data, &ref_bm.data);
        if d > max_diff {
            continue;
        }
        match best {
            None => best = Some((idx, d)),
            Some((_, bd)) if d < bd => best = Some((idx, d)),
            _ => {}
        }
    }
    best.map(|(i, _)| i)
}

/// Experiment-only knobs for the cross-size record-6 refinement emitter (#322).
///
/// When present in [`Jb2EncodeOptions::cross_size_rec6_probe`], fresh
/// connected components that have no exact dictionary hit are matched against
/// dictionary entries whose bounding box differs by at most `max_dim_delta`
/// pixels in each axis. If a near-twin is found within the normalized Hamming
/// budget, the component is emitted as a **lossless** record-6 matched
/// refinement (`wdiff`/`hdiff` + 11-bit refinement bitmap) referencing that
/// entry instead of a fresh record-1.
///
/// This is a measurement vehicle: the refinement bitmap reproduces the
/// component exactly (round-trip is pixel-lossless), but it is *not* wired into
/// any shipped encoder path. [`encode_jb2_dict`] /
/// [`encode_jb2_dict_with_shared`] leave it disabled, so default output is
/// byte-identical to before.
#[cfg(feature = "experimental")]
#[derive(Debug, Clone, Copy)]
pub struct CrossSizeRec6Probe {
    /// Maximum per-axis bounding-box difference (in pixels) between a fresh
    /// component and a candidate dictionary entry.
    pub max_dim_delta: u32,
    /// Accepted normalized Hamming budget as a fraction of the component's
    /// pixel count, scored after nearest-neighbor resampling of the candidate
    /// into the component's dimensions.
    pub max_hamming_fraction: f32,
}

/// Knobs for **center-aligned** record-4/6 refinement.
///
/// A fresh component with no exact dictionary twin is coded as a refinement of
/// the nearest dictionary entry whose box differs by at most `max_dim_delta`
/// pixels per axis (same size included). Distance is the Hamming count under
/// the decoder's own alignment: the reference is centered on the component
/// exactly as the refinement context sees it, with no resampling. The
/// refinement bitmap reproduces the component exactly, so output is lossless.
///
/// [`encode_jb2_lossless`] uses [`AlignedRefine::LOSSLESS`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlignedRefine {
    /// Maximum per-axis bounding-box difference (in pixels).
    pub max_dim_delta: u32,
    /// Accepted aligned Hamming budget as a fraction of the component's
    /// bounding-box area.
    pub max_hamming_fraction: f32,
    /// `true` emits record 4 (refine, add to the dictionary, blit), so the
    /// refined glyph can serve as a later reference; `false` emits record 6.
    pub add_to_dict: bool,
}

impl AlignedRefine {
    /// Settings of [`encode_jb2_lossless`]: ±2 px, 20 % of the box area,
    /// record 4. Measured in `PERF_EXPERIMENTS.md` ("JB2 aligned refinement").
    pub const LOSSLESS: Self = Self {
        max_dim_delta: 2,
        max_hamming_fraction: 0.2,
        add_to_dict: true,
    };
}
