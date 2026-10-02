//! Djbz dictionary streams and multi-page symbol sharing (#194).

use super::*;

pub(super) const SHARED_DICT_PIXEL_BUDGET: usize = 4 * 1024 * 1024;

/// Encode a sequence of bilevel symbols as a JB2 **Djbz** chunk payload.
///
/// Each symbol is emitted as record-type-2 (new symbol, direct, dict-only) in
/// the order given. The decoder side ([`crate::decode_dict`]) reconstructs
/// a [`crate::Jb2Dict`] whose symbol indices match this input order, so
/// downstream Sjbz streams encoded with [`encode_jb2_dict_with_shared`] using
/// the same `&[Bitmap]` reference will round-trip cleanly.
///
/// The Djbz contains no positioning information — symbols are abstract
/// glyph bitmaps, not blits. The page Sjbz alone places them.
pub fn encode_jb2_djbz(symbols: &[Bitmap]) -> Vec<u8> {
    let mut zp = ZpEncoder::new();
    let mut record_type_ctx = NumContext::new();
    let mut image_size_ctx = NumContext::new();
    let mut symbol_width_ctx = NumContext::new();
    let mut symbol_height_ctx = NumContext::new();
    let mut direct_bitmap_ctx = vec![0u8; 1024];
    let mut flag_ctx: u8 = 0;

    // Preamble: start-of-image (rec 0) — no rec-9 since a Djbz never inherits
    // from another dict in this encoder. Dimensions are written but unused on
    // the decode side (see `decode_dictionary` in jb2.rs:1990).
    encode_num(&mut zp, &mut record_type_ctx, 0, 11, 0);
    encode_num(&mut zp, &mut image_size_ctx, 0, 262142, 0);
    encode_num(&mut zp, &mut image_size_ctx, 0, 262142, 0);
    zp.encode_bit(&mut flag_ctx, false);

    // Symbol body: rec-2 per entry.
    for sym in symbols {
        encode_num(&mut zp, &mut record_type_ctx, 0, 11, 2);
        encode_num(&mut zp, &mut symbol_width_ctx, 0, 262142, sym.width as i32);
        encode_num(
            &mut zp,
            &mut symbol_height_ctx,
            0,
            262142,
            sym.height as i32,
        );
        encode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, sym);
    }

    // End-of-data.
    encode_num(&mut zp, &mut record_type_ctx, 0, 11, 11);
    zp.finish()
}

/// Cluster CCs from `pages` and return the bitmaps that should live in a
/// shared Djbz: any (w, h, packed-data) signature that appears on `>=
/// page_threshold` distinct pages, represented by the first-seen CC.
///
/// Returns shared symbols in deterministic order (sorted by first-seen page,
/// then first-seen position within that page). Pages without enough repetition
/// produce an empty shared dict.
///
/// **Byte-exact dedup only.** Tried Hamming-distance clustering for #194
/// Phase 2 (`cluster_shared_symbols_tunable` with `diff_fraction > 0`):
/// no measurable byte saving on the 517-page `pathogenic_bacteria_1896`
/// corpus (< 0.05% delta from byte-exact across 0%/1%/2% Hamming) and
/// `diff_fraction = 3%` introduced per-page Sjbz decode mismatches under
/// rec-6 refinement against shared reps. Byte-exact clustering already
/// captures the multi-page win (−13.0% bundle vs independent on the same
/// corpus). See CLAUDE.md "Multi-page shared Djbz dictionary, Phase 2"
/// investigation for measurements.
pub fn cluster_shared_symbols(pages: &[Bitmap], page_threshold: usize) -> Vec<Bitmap> {
    cluster_shared_symbols_tunable(pages, page_threshold, 0)
}

/// Reference-slice variant of [`cluster_shared_symbols`] (#565): identical
/// clustering over borrowed masks, so a caller holding masks inside larger
/// per-page structs doesn't have to clone every bitmap into a contiguous
/// `Vec<Bitmap>` first.
pub fn cluster_shared_symbols_from_refs(pages: &[&Bitmap], page_threshold: usize) -> Vec<Bitmap> {
    cluster_impl(pages, page_threshold)
}

/// Same as [`cluster_shared_symbols`], preserving the old benchmarking
/// signature that accepted a per-CC Hamming allowance. The allowance is now
/// ignored: #258 showed that Hamming shared clustering can produce invalid
/// page streams on long corpora, and prior measurements found no material
/// size win over byte-exact clustering.
///
/// Provided for corpus benchmarking — most callers want
/// [`cluster_shared_symbols`].
pub fn cluster_shared_symbols_tunable(
    pages: &[Bitmap],
    page_threshold: usize,
    _diff_fraction: u32,
) -> Vec<Bitmap> {
    let refs: Vec<&Bitmap> = pages.iter().collect();
    cluster_impl(&refs, page_threshold)
}

pub(super) fn cluster_impl(pages: &[&Bitmap], page_threshold: usize) -> Vec<Bitmap> {
    if page_threshold < 2 || pages.len() < page_threshold {
        return Vec::new();
    }

    struct Cluster {
        rep: Bitmap,
        pages_seen: Vec<usize>,
        first_seen: (usize, usize),
    }

    // One size class: its clusters (in creation order) plus a `symbol_hash`
    // index over their reps for O(1) exact-match lookup. Since clustering is
    // byte-exact (see below), every rep in a bucket is distinct, so at most one
    // cluster can match a candidate — the hash index replaces the O(K) linear
    // `packed_hamming` scan the old code ran per CC (CLUSTER_BUCKET_HASH_DEDUP,
    // the clustering analog of CLUSTER_DEDUP #446 for the running-dict encoder).
    #[derive(Default)]
    struct SizeBucket {
        clusters: Vec<Cluster>,
        by_hash: BTreeMap<u64, Vec<usize>>,
    }

    // Byte-exact bucketing of one page's connected components, in CC order.
    // Kept as a local item so the parallel and sequential extract paths share
    // it; visits CCs in page order to keep `first_seen`/`pages_seen` identical.
    fn bucket_page_ccs(
        buckets: &mut BTreeMap<(u32, u32), SizeBucket>,
        ccs: &[Cc],
        page_idx: usize,
    ) {
        for (cc_idx, cc) in ccs.iter().enumerate() {
            let bm = &cc.bitmap;
            // Hamming shared clustering was rejected for #258: it produced
            // invalid page streams on the 517-page corpus while providing no
            // measured size win. Clustering is byte-exact for all callers, so a
            // candidate merges only into a rep with identical bytes.
            let bucket = buckets.entry((bm.width, bm.height)).or_default();
            let hash = symbol_hash(bm.width, bm.height, &bm.data);
            // Exact match: the unique rep (if any) whose bytes equal `bm`. The
            // per-hash verify guards against `symbol_hash` collisions so the
            // result stays byte-identical to the old full-scan `best` pick.
            let hit = bucket.by_hash.get(&hash).and_then(|cands| {
                cands
                    .iter()
                    .copied()
                    .find(|&i| bucket.clusters[i].rep.data == bm.data)
            });
            match hit {
                Some(i) => {
                    // #446: pages are visited in strictly non-decreasing `page_idx`
                    // order, so `pages_seen` is sorted and the current page, if
                    // already counted, is the last element. An O(1) `last()` check
                    // replaces the O(K) `contains` scan (O(P²)→O(P) total on a corpus
                    // where a cluster recurs on many pages).
                    if bucket.clusters[i].pages_seen.last() != Some(&page_idx) {
                        bucket.clusters[i].pages_seen.push(page_idx);
                    }
                }
                None => {
                    let idx = bucket.clusters.len();
                    bucket.clusters.push(Cluster {
                        rep: bm.clone(),
                        pages_seen: vec![page_idx],
                        first_seen: (page_idx, cc_idx),
                    });
                    bucket.by_hash.entry(hash).or_default().push(idx);
                }
            }
        }
    }

    let mut buckets: BTreeMap<(u32, u32), SizeBucket> = BTreeMap::new();

    // Connected-component extraction is independent per page and is the bulk of
    // the clustering cost; the bucketing that follows is order-dependent (it
    // must visit CCs in page order to keep `first_seen`/`pages_seen` and the
    // trim-priority tie-breaks byte-identical). So extract CCs for a bounded
    // batch of pages in parallel, then bucket that batch sequentially in order.
    // Batching (rather than one big `par_iter().collect()`) caps the transient
    // CC memory to `BATCH` pages — important for long bilevel corpora (e.g. the
    // 517-page `pathogenic_bacteria_1896`) where holding every page's CCs at
    // once would be a memory regression. Output is byte-identical to the old
    // strictly-sequential extract-then-bucket loop.
    const BATCH: usize = 32;
    let mut page_idx = 0usize;
    for chunk in pages.chunks(BATCH) {
        #[cfg(feature = "parallel")]
        let ccs_batch: Vec<Vec<Cc>> = {
            use rayon::prelude::*;
            chunk.par_iter().map(|bm| extract_ccs(bm)).collect()
        };
        #[cfg(not(feature = "parallel"))]
        let ccs_batch: Vec<Vec<Cc>> = chunk.iter().map(|bm| extract_ccs(bm)).collect();

        for ccs in &ccs_batch {
            bucket_page_ccs(&mut buckets, ccs, page_idx);
            page_idx += 1;
        }
    }

    let mut promoted: Vec<Cluster> = buckets
        .into_values()
        .flat_map(|b| b.clusters)
        .filter(|c| c.pages_seen.len() >= page_threshold)
        .collect();

    // Cap cumulative pixels at the decoder's per-stream symbol budget
    // (`MAX_TOTAL_SYMBOL_PIXELS` in src/jb2.rs). Without this guard, a long
    // bilevel corpus (e.g. 517-page `pathogenic_bacteria_1896.djvu` produces
    // ~78 MP of shared symbols at threshold 2) yields a `Djbz` that
    // `decode_dictionary` then rejects with `Jb2Error::ImageTooLarge`,
    // rendering the whole bundle undecodable. See #270.
    //
    // When trimming, prefer to keep the highest-value reps: those seen on
    // more pages save more bytes per byte of shared-dict footprint. Ties on
    // page count → smaller pixel cost wins (cheaper, less likely to push us
    // back over budget on the next item).
    let mut total_pixels: u64 = 0;
    let cap = SHARED_DICT_PIXEL_BUDGET as u64;
    let any_over_budget = promoted.iter().fold(0u64, |acc, c| {
        acc + (c.rep.width as u64) * (c.rep.height as u64)
    }) > cap;
    if any_over_budget {
        let mut by_value: Vec<usize> = (0..promoted.len()).collect();
        by_value.sort_by(|&a, &b| {
            promoted[b]
                .pages_seen
                .len()
                .cmp(&promoted[a].pages_seen.len())
                .then_with(|| {
                    let pa = (promoted[a].rep.width as u64) * (promoted[a].rep.height as u64);
                    let pb = (promoted[b].rep.width as u64) * (promoted[b].rep.height as u64);
                    pa.cmp(&pb)
                })
        });
        let mut keep = vec![false; promoted.len()];
        for &i in &by_value {
            let pix = (promoted[i].rep.width as u64) * (promoted[i].rep.height as u64);
            if total_pixels + pix > cap {
                continue;
            }
            keep[i] = true;
            total_pixels += pix;
        }
        let mut idx = 0;
        promoted.retain(|_| {
            let k = keep[idx];
            idx += 1;
            k
        });
    }

    promoted.sort_by_key(|c| c.first_seen);
    promoted.into_iter().map(|c| c.rep).collect()
}

/// JB2_DICT_ORDER probe (diagnostic-only): three orderings of the same
/// byte-exact shared-symbol set [`cluster_shared_symbols`] produces, for
/// measuring whether dictionary index assignment affects `Sjbz`/`Djbz` size.
///
/// The JB2 format numbers dictionary entries by emission order — decoders
/// resolve rec-6/rec-7 references by that index, and a rec-6 refinement can
/// only reference an already-emitted entry. The whole shared block is always
/// emitted before any page-local symbol, so reordering *within* the shared
/// block alone can never violate that "reference precedes refiner" rule —
/// any permutation here is a legal encoder choice. This type exists purely to
/// feed [`encode_jb2_djbz`] / [`encode_jb2_dict_with_shared`] with different
/// shared-dict orderings for A/B size measurement; it ships no behavior
/// change (gated behind `experimental`, called from no default code path).
#[cfg(feature = "experimental")]
pub struct DictOrderVariants {
    /// Current shipped order: first-seen (page, then CC index within page).
    /// Identical to [`cluster_shared_symbols`]'s output.
    pub baseline: Vec<Bitmap>,
    /// Descending usage-frequency (number of distinct pages a symbol was
    /// promoted from), ties broken by first-seen order.
    pub by_frequency: Vec<Bitmap>,
    /// Grouped by `(width, height)` size bucket (ascending), first-seen order
    /// within each bucket — same-shaped symbols end up adjacent. This is the
    /// clustering pass's natural bucket-iteration order, before the final
    /// first-seen sort the shipped path applies.
    pub by_bucket: Vec<Bitmap>,
}

/// Build the three [`DictOrderVariants`] orderings from the same byte-exact
/// clustering pass [`cluster_shared_symbols`] runs. Mirrors that function's
/// dedup + pixel-budget trim exactly (so the *set* of promoted symbols is
/// identical to what the shipped encoder would use) and only changes what
/// happens after: instead of one fixed sort, it captures per-cluster
/// usage-count and bucket-position metadata to emit all three orderings.
#[cfg(feature = "experimental")]
pub fn cluster_shared_symbols_order_variants(
    pages: &[Bitmap],
    page_threshold: usize,
) -> DictOrderVariants {
    if page_threshold < 2 || pages.len() < page_threshold {
        return DictOrderVariants {
            baseline: Vec::new(),
            by_frequency: Vec::new(),
            by_bucket: Vec::new(),
        };
    }

    struct Cluster {
        rep: Bitmap,
        pages_seen: Vec<usize>,
        first_seen: (usize, usize),
    }

    #[derive(Default)]
    struct SizeBucket {
        clusters: Vec<Cluster>,
        by_hash: BTreeMap<u64, Vec<usize>>,
    }

    fn bucket_page_ccs(
        buckets: &mut BTreeMap<(u32, u32), SizeBucket>,
        ccs: &[Cc],
        page_idx: usize,
    ) {
        for (cc_idx, cc) in ccs.iter().enumerate() {
            let bm = &cc.bitmap;
            let bucket = buckets.entry((bm.width, bm.height)).or_default();
            let hash = symbol_hash(bm.width, bm.height, &bm.data);
            let hit = bucket.by_hash.get(&hash).and_then(|cands| {
                cands
                    .iter()
                    .copied()
                    .find(|&i| bucket.clusters[i].rep.data == bm.data)
            });
            match hit {
                Some(i) => {
                    if bucket.clusters[i].pages_seen.last() != Some(&page_idx) {
                        bucket.clusters[i].pages_seen.push(page_idx);
                    }
                }
                None => {
                    let idx = bucket.clusters.len();
                    bucket.clusters.push(Cluster {
                        rep: bm.clone(),
                        pages_seen: vec![page_idx],
                        first_seen: (page_idx, cc_idx),
                    });
                    bucket.by_hash.entry(hash).or_default().push(idx);
                }
            }
        }
    }

    let mut buckets: BTreeMap<(u32, u32), SizeBucket> = BTreeMap::new();
    const BATCH: usize = 32;
    let mut page_idx = 0usize;
    for chunk in pages.chunks(BATCH) {
        #[cfg(feature = "parallel")]
        let ccs_batch: Vec<Vec<Cc>> = {
            use rayon::prelude::*;
            chunk.par_iter().map(extract_ccs).collect()
        };
        #[cfg(not(feature = "parallel"))]
        let ccs_batch: Vec<Vec<Cc>> = chunk.iter().map(extract_ccs).collect();

        for ccs in &ccs_batch {
            bucket_page_ccs(&mut buckets, ccs, page_idx);
            page_idx += 1;
        }
    }

    let mut promoted: Vec<Cluster> = buckets
        .into_values()
        .flat_map(|b| b.clusters)
        .filter(|c| c.pages_seen.len() >= page_threshold)
        .collect();

    // Same pixel-budget trim as `cluster_shared_symbols_tunable`, so the
    // promoted *set* matches the shipped path exactly (only its order
    // differs below).
    let mut total_pixels: u64 = 0;
    let cap = SHARED_DICT_PIXEL_BUDGET as u64;
    let any_over_budget = promoted.iter().fold(0u64, |acc, c| {
        acc + (c.rep.width as u64) * (c.rep.height as u64)
    }) > cap;
    if any_over_budget {
        let mut by_value: Vec<usize> = (0..promoted.len()).collect();
        by_value.sort_by(|&a, &b| {
            promoted[b]
                .pages_seen
                .len()
                .cmp(&promoted[a].pages_seen.len())
                .then_with(|| {
                    let pa = (promoted[a].rep.width as u64) * (promoted[a].rep.height as u64);
                    let pb = (promoted[b].rep.width as u64) * (promoted[b].rep.height as u64);
                    pa.cmp(&pb)
                })
        });
        let mut keep = vec![false; promoted.len()];
        for &i in &by_value {
            let pix = (promoted[i].rep.width as u64) * (promoted[i].rep.height as u64);
            if total_pixels + pix > cap {
                continue;
            }
            keep[i] = true;
            total_pixels += pix;
        }
        let mut idx = 0;
        promoted.retain(|_| {
            let k = keep[idx];
            idx += 1;
            k
        });
    }

    // `promoted` right now is in bucket-iteration order: `(width, height)`
    // ascending (BTreeMap key order), then first-seen within each bucket
    // (creation order) — same-shaped symbols are already adjacent.
    let by_bucket: Vec<Bitmap> = promoted.iter().map(|c| c.rep.clone()).collect();

    let mut baseline_idx: Vec<usize> = (0..promoted.len()).collect();
    baseline_idx.sort_by_key(|&i| promoted[i].first_seen);
    let baseline: Vec<Bitmap> = baseline_idx
        .iter()
        .map(|&i| promoted[i].rep.clone())
        .collect();

    let mut freq_idx: Vec<usize> = (0..promoted.len()).collect();
    freq_idx.sort_by(|&a, &b| {
        promoted[b]
            .pages_seen
            .len()
            .cmp(&promoted[a].pages_seen.len())
            .then_with(|| promoted[a].first_seen.cmp(&promoted[b].first_seen))
    });
    let by_frequency: Vec<Bitmap> = freq_idx.iter().map(|&i| promoted[i].rep.clone()).collect();

    DictOrderVariants {
        baseline,
        by_frequency,
        by_bucket,
    }
}
