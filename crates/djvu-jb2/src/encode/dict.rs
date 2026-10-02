//! Public entry points of the dictionary (symbol-matching) encoder.

use super::*;

/// Encode a bilevel [`Bitmap`] into a JB2 stream using a **symbol dictionary**.
///
/// Performs connected-component (CC) extraction, exact-match deduplication,
/// near-duplicate refinement matching, and emits one of:
///  * record type 1 — new symbol (direct, stored in dict + blitted)
///  * record type 6 — matched refinement (blit only, encodes diff vs an
///    existing dict entry of identical size using the 11-bit context)
///  * record type 7 — matched copy (no refinement, blit only)
///
/// Lossless. Matches [`crate::decode`] for round-trip.
///
/// ## Limitations
/// - Refinement matching only considers dict entries of identical (w, h).
///   Cross-size matching (which the format permits via wdiff/hdiff) needs
///   per-pixel resampling to compute the Hamming distance and is left to
///   a future phase.
/// - Components > 16 MP are encoded as-is; the decoder will reject them via
///   `MAX_SYMBOL_PIXELS`. For scanned text pages this is not a practical issue.
pub fn encode_jb2_dict(bitmap: &Bitmap) -> Vec<u8> {
    encode_jb2_dict_with_shared(bitmap, &[])
}

/// Encode a bilevel [`Bitmap`] into a JB2 stream that inherits its initial
/// symbol library from a previously-encoded shared dictionary (Djbz).
///
/// Same as [`encode_jb2_dict`] but emits a "required-dict-or-reset"
/// (record type 9) preamble announcing `shared_symbols.len()` inherited
/// entries. Per-symbol matches that hit any of the shared symbols are
/// emitted as record-7 (matched copy) referencing the shared index, so
/// the per-page Sjbz never re-transmits glyphs already present in the
/// shared Djbz.
///
/// `shared_symbols` must be the **identical bitmap sequence** the matching
/// Djbz was built from (see [`encode_jb2_djbz`]).
///
/// Round-trip: pass the resulting Sjbz bytes plus
/// `decode_dict(djbz_bytes, None)` to [`crate::decode`].
pub fn encode_jb2_dict_with_shared(bitmap: &Bitmap, shared_symbols: &[Bitmap]) -> Vec<u8> {
    encode_jb2_dict_with_options(bitmap, shared_symbols, &Jb2EncodeOptions::default())
}

/// Encode like [`encode_jb2_dict_with_shared`] but with caller-specified
/// [`Jb2EncodeOptions`]. The default options reproduce the lossless
/// behavior of [`encode_jb2_dict_with_shared`]; raising
/// `opts.lossy_threshold` enables rec-7 substitution for near-duplicate
/// CCs (see [`Jb2EncodeOptions::lossy_threshold`]).
///
/// Lossy output: when `lossy_threshold > 0`, the decoded page is no
/// longer pixel-exact relative to the input; reconstruction error per CC
/// is bounded by the threshold (Hamming as a fraction of pixel count).
/// Lossless output (default) round-trips byte-for-byte through
/// [`crate::decode`].
pub fn encode_jb2_dict_with_options(
    bitmap: &Bitmap,
    shared_symbols: &[Bitmap],
    opts: &Jb2EncodeOptions,
) -> Vec<u8> {
    encode_jb2_dict_with_blits(bitmap, shared_symbols, opts).0
}

/// Encode a bilevel page losslessly at the smallest size this crate reaches.
///
/// Uses the symbol dictionary with center-aligned refinement
/// ([`AlignedRefine::LOSSLESS`]): each connected component is a new symbol, an
/// exact copy, or a refinement of a similar earlier symbol. The page is also
/// coded with [`encode_jb2`], and the smaller stream is returned: direct
/// coding wins on pages of few, large, unique shapes such as maps. Either way
/// the decoded page is pixel-identical to `bitmap`.
///
/// A page whose components would exceed the decoder's record or symbol-pixel
/// limits always gets [`encode_jb2`], so the output decodes wherever the
/// direct encoder's does. The check is conservative: it counts every
/// component as decoded.
pub fn encode_jb2_lossless(bitmap: &Bitmap) -> Vec<u8> {
    encode_jb2_lossless_with_shared(bitmap, &[])
}

/// [`encode_jb2_lossless`] for a page that inherits `shared_symbols` from a
/// shared `Djbz`, as [`encode_jb2_dict_with_shared`] does. A component can
/// copy or refine a shared symbol.
///
/// When direct coding wins, or the page exceeds the decoder's limits, the
/// stream does not reference the shared dictionary; it still decodes with or
/// without it.
pub fn encode_jb2_lossless_with_shared(bitmap: &Bitmap, shared_symbols: &[Bitmap]) -> Vec<u8> {
    let w = bitmap.width as i32;
    let h = bitmap.height as i32;
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let opts = Jb2EncodeOptions::default();
    let (ccs, order) = extract_and_order_ccs(bitmap, &opts);
    let direct = encode_jb2(bitmap);
    if !fits_decoder_budget(&ccs, !shared_symbols.is_empty()) {
        return direct;
    }
    let dict = encode_jb2_dict_with_ccs(
        w,
        h,
        ccs,
        order,
        shared_symbols,
        &opts,
        Some(AlignedRefine::LOSSLESS),
    )
    .0;
    // Direct tiles cost one work unit per page pixel in the decoder's
    // per-page budget; a page over it does not decode here.
    let direct_decodes = (bitmap.width as usize).saturating_mul(bitmap.height as usize)
        <= crate::MAX_PAGE_SYMBOL_WORK;
    if direct_decodes && direct.len() < dict.len() {
        direct
    } else {
        dict
    }
}

/// Whether one symbol record per component (plus start and end records, and
/// the inherited-dictionary record when `shared`) stays within the decoder's
/// page limits, counting every component as a direct symbol. Refinement costs
/// more, but the dictionary encoder only refines while the page stays within
/// budget, and [`extract_ccs`] already split components over the per-symbol
/// limit.
pub(super) fn fits_decoder_budget(ccs: &[Cc], shared: bool) -> bool {
    if ccs.len() + 2 + usize::from(shared) > crate::MAX_RECORDS {
        return false;
    }
    let total = ccs.iter().fold(0usize, |t, cc| {
        t.saturating_add((cc.bitmap.width as usize).saturating_mul(cc.bitmap.height as usize))
    });
    total <= crate::MAX_PAGE_SYMBOL_WORK
}

/// One emitted blit: its cropped shape and top-left position (top-down page
/// coordinates), in emission order — blit *i* here is blit index *i* on the
/// decoder side.
///
/// For the lossless paths (default options, despeckle, exact and rec-6
/// matches) the shape is pixel-identical to what the decoder reconstructs;
/// only lossy rec-7 substitution (`lossy_threshold > 0`) blits a near-twin
/// whose pixels can differ from this original component.
pub struct EncodedBlit {
    /// Top-left x of the blit in the page (0 = left edge).
    pub x: u32,
    /// Top-left y of the blit in the page (0 = top edge, top-down).
    pub y: u32,
    /// Cropped component bitmap (tight bbox, this component's pixels only).
    pub bitmap: Bitmap,
}

/// Extract `bitmap`'s connected components, apply the despeckle pre-pass
/// (`opts.despeckle`), and return them together with the reading-order
/// permutation `encode_jb2_dict_with_blits` uses to decide coordinate coding
/// and blit emission order.
///
/// This is the *geometric decomposition* step: connected-component
/// extraction, despeckle filtering, and the baseline-bucket reading-order
/// sort. None of it depends on `shared_symbols` — dictionary lookups (exact
/// match / lossy near-twin / refinement) only happen after this point, per
/// `cc_idx` in `order`. So for a fixed `bitmap` and `opts`, this function's
/// output — hence [`symbol_boxes_in_emission_order`]'s and
/// [`encode_jb2_dict_with_blits`]'s blit list — is identical regardless of
/// what dictionary (if any) is supplied.
pub(super) fn extract_and_order_ccs(
    bitmap: &Bitmap,
    opts: &Jb2EncodeOptions,
) -> (Vec<Cc>, Vec<usize>) {
    let mut ccs = extract_ccs(bitmap);

    // Despeckle pre-pass (JB2_DESPECKLE): drop isolated small components
    // *before* clustering/dedup so a speck never becomes a dict entry or a
    // coordinate record, and never dilutes the near-twin population that
    // `lossy_threshold` matches against. Filtering on `pixel_count` (true
    // ink area) rather than bbox area avoids over-crediting thin diagonal
    // strokes as "small". Lossy: the decoder has nothing left to
    // reconstruct these pixels from.
    if let Some(max_speck_px) = opts.despeckle {
        ccs.retain(|cc| cc.pixel_count > max_speck_px);
    }

    // Reading-order sort by baseline-bucket, then left-to-right.
    //
    // The JB2 coord stream's `same_line` mode is keyed off `y_jb2` (the bottom
    // edge of each symbol in JB2 bottom-up coords). Glyphs sharing a text
    // baseline have similar `y_jb2` values regardless of height (e.g. 't' vs
    // 'o'), but they differ in top-left `cc.y`. Sorting by `cc.y` therefore
    // interleaves glyphs from adjacent lines, defeating same-line coding.
    //
    // Bucketing by bottom-row in top-down coords (`cc.y + cc_h`), rounded to a
    // line-height grid, then by `x` within a bucket, gives proper reading order
    // for same-line detection. The bucket granularity is the same baseline
    // tolerance used in the same/new-line decision below.
    let mut order: Vec<usize> = (0..ccs.len()).collect();
    let bucket = (SAME_LINE_BASELINE_TOL.max(1)) as u32;
    order.sort_by_key(|&i| {
        let cc = &ccs[i];
        let bottom = cc.y + cc.bitmap.height;
        (bottom / bucket, cc.x)
    });

    (ccs, order)
}

/// One symbol's raw shape/position (before any dictionary encoding) — the
/// geometric decomposition [`encode_jb2_dict_with_blits`] uses to build its
/// blit list, computed independent of any shared dictionary.
///
/// Same shape as [`EncodedBlit`], with the same lossless-vs-lossy caveat: it
/// always carries the original connected component's pixels, never a
/// substituted dict entry, so it matches the real emitted blit only when the
/// component is emitted byte-exact (see [`symbol_boxes_in_emission_order`]).
pub struct SymbolBox {
    /// Top-left x of the symbol in the page (0 = left edge).
    pub x: u32,
    /// Top-left y of the symbol in the page (0 = top edge, top-down).
    pub y: u32,
    /// Cropped component bitmap (tight bbox, this component's pixels only).
    pub bitmap: Bitmap,
}

/// Encode a JB2 symbol dictionary from a `symbols` list already produced by
/// [`symbol_boxes_in_emission_order`] for the *same* `(bitmap, opts)`,
/// instead of re-running connected-component extraction on `bitmap` — the
/// entropy-encoded byte stream and blit list are identical to what
/// [`encode_jb2_dict_with_blits`] would produce for that `bitmap`, `opts`,
/// and `shared_symbols` (see [`symbol_boxes_in_emission_order`]'s doc
/// comment for the lossless-only caveat), but this entry point skips
/// `extract_ccs` entirely.
///
/// `symbols` is already in emission order (that's what
/// [`symbol_boxes_in_emission_order`] returns), so `order` here is just the
/// identity permutation.
pub fn encode_jb2_dict_with_symbols(
    mask_width: u32,
    mask_height: u32,
    symbols: Vec<SymbolBox>,
    shared_symbols: &[Bitmap],
    opts: &Jb2EncodeOptions,
) -> (Vec<u8>, Vec<EncodedBlit>) {
    encode_jb2_dict_with_symbols_refined(
        mask_width,
        mask_height,
        symbols,
        shared_symbols,
        opts,
        aligned_of(opts),
    )
}

/// [`encode_jb2_dict_with_symbols`] with center-aligned refinement set by
/// `aligned` (it replaces the experimental `opts.aligned_refine`). The blit
/// list is unchanged: a refinement reproduces its component exactly.
pub fn encode_jb2_dict_with_symbols_refined(
    mask_width: u32,
    mask_height: u32,
    symbols: Vec<SymbolBox>,
    shared_symbols: &[Bitmap],
    opts: &Jb2EncodeOptions,
    aligned: Option<AlignedRefine>,
) -> (Vec<u8>, Vec<EncodedBlit>) {
    let w = mask_width as i32;
    let h = mask_height as i32;
    if w == 0 || h == 0 {
        return (Vec::new(), Vec::new());
    }
    let order: Vec<usize> = (0..symbols.len()).collect();
    let ccs: Vec<Cc> = symbols
        .into_iter()
        .map(|s| Cc {
            x: s.x,
            y: s.y,
            bitmap: s.bitmap,
            // Despeckle filtering already ran when `symbols` was produced by
            // `symbol_boxes_in_emission_order`; `pixel_count` is only read
            // by that filter, never past this point, so it's dead here.
            pixel_count: 0,
        })
        .collect();
    encode_jb2_dict_with_ccs(w, h, ccs, order, shared_symbols, opts, aligned)
}

/// Extract `bitmap`'s connected components in the same emission order (after
/// despeckle) that [`encode_jb2_dict_with_blits`] will use for its blit list
/// — *without* running the entropy encoder.
///
/// The geometric decomposition (connected-component extraction, despeckle,
/// reading-order sort) depends only on `bitmap` and `opts`, never on a
/// shared dictionary (see `extract_and_order_ccs`'s doc comment). So for
/// any `shared_symbols`,
/// `symbol_boxes_in_emission_order(bitmap, opts)[i]` describes the same
/// `(x, y, bitmap)` as `encode_jb2_dict_with_blits(bitmap, shared_symbols,
/// opts).1[i]` — **provided every emitted blit is byte-exact** (the
/// lossless case: `opts.lossy_threshold == 0.0`, no experimental cross-size
/// refinement). Under lossy rec-7 substitution the *emitted* blit can be a
/// near-twin dict entry instead of this original shape (same restriction
/// [`EncodedBlit`] and `foreground_fgbz_from_blits` in the main crate
/// document), so callers must not treat these as the true emitted shapes in
/// that case.
///
/// This lets a caller precompute per-symbol data (e.g. average colour under
/// each component, for `FGbz`) before the shared dictionary is known —
/// letting the pixmap that data is sampled from be dropped earlier in a
/// multi-page pipeline. See `djvu_encode/mod.rs`'s `PreparedPage::cc_colors`.
pub fn symbol_boxes_in_emission_order(bitmap: &Bitmap, opts: &Jb2EncodeOptions) -> Vec<SymbolBox> {
    if bitmap.width == 0 || bitmap.height == 0 {
        return Vec::new();
    }
    let (mut ccs, order) = extract_and_order_ccs(bitmap, opts);
    order
        .iter()
        .map(|&i| {
            let cc = &mut ccs[i];
            SymbolBox {
                x: cc.x,
                y: cc.y,
                bitmap: core::mem::replace(&mut cc.bitmap, Bitmap::new(0, 0)),
            }
        })
        .collect()
}

/// Encode like [`encode_jb2_dict_with_options`] and also return the emitted
/// blits (shape + placement, in emission order).
///
/// The byte stream is identical to [`encode_jb2_dict_with_options`] — the
/// blit list is metadata the encoder already owns, handed back so callers
/// (e.g. the FGbz palette builder, #612) don't have to decode the stream
/// they just produced to recover the per-blit layout.
pub fn encode_jb2_dict_with_blits(
    bitmap: &Bitmap,
    shared_symbols: &[Bitmap],
    opts: &Jb2EncodeOptions,
) -> (Vec<u8>, Vec<EncodedBlit>) {
    encode_jb2_dict_with_blits_refined(bitmap, shared_symbols, opts, aligned_of(opts))
}

/// [`encode_jb2_dict_with_blits`] with center-aligned refinement set by
/// `aligned` (it replaces the experimental `opts.aligned_refine`). With
/// `Some(AlignedRefine::LOSSLESS)` a component close to an earlier symbol is
/// coded as its refinement; the blit list is unchanged, since a refinement
/// reproduces its component exactly.
pub fn encode_jb2_dict_with_blits_refined(
    bitmap: &Bitmap,
    shared_symbols: &[Bitmap],
    opts: &Jb2EncodeOptions,
    aligned: Option<AlignedRefine>,
) -> (Vec<u8>, Vec<EncodedBlit>) {
    let w = bitmap.width as i32;
    let h = bitmap.height as i32;
    if w == 0 || h == 0 {
        return (Vec::new(), Vec::new());
    }

    let (ccs, order) = extract_and_order_ccs(bitmap, opts);
    encode_jb2_dict_with_ccs(w, h, ccs, order, shared_symbols, opts, aligned)
}
