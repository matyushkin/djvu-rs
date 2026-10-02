//! The record emitter behind every dictionary entry point: records 1, 4, 6, 7 and layout.

use super::*;

/// Encode a JB2 symbol dictionary from an *already-extracted* geometric
/// decomposition (`ccs`/`order`, from `extract_and_order_ccs` or converted
/// from a caller-held [`SymbolBox`] list via [`encode_jb2_dict_with_symbols`]),
/// skipping `extract_ccs`'s connected-component pass entirely.
///
/// This is the shared tail [`encode_jb2_dict_with_blits`] uses after its own
/// extraction — factored out so a caller that already ran the geometric
/// decomposition once (e.g. to precompute per-symbol colour data before the
/// shared dictionary is known, #step-3 of the encoder peak-memory plan) does
/// not have to pay for a second, redundant extraction pass in the real
/// encode. `w`/`h` are the source bitmap's dimensions (needed for the header
/// and bottom-up `y_jb2` coordinate conversion, since this entry point no
/// longer has the full bitmap).
pub(super) fn encode_jb2_dict_with_ccs(
    w: i32,
    h: i32,
    mut ccs: Vec<Cc>,
    order: Vec<usize>,
    shared_symbols: &[Bitmap],
    opts: &Jb2EncodeOptions,
    aligned: Option<AlignedRefine>,
) -> (Vec<u8>, Vec<EncodedBlit>) {
    let mut zp = ZpEncoder::new();
    let mut record_type_ctx = NumContext::new();
    let mut image_size_ctx = NumContext::new();
    let mut symbol_width_ctx = NumContext::new();
    let mut symbol_height_ctx = NumContext::new();
    let mut symbol_index_ctx = NumContext::new();
    let mut inherit_dict_size_ctx = NumContext::new();
    // Refinement contexts (records 4 and 6). Only refinement records touch
    // these, and constructing a `NumContext` never touches the ZP coder, so
    // output without refinement stays byte-identical.
    let mut symbol_width_diff_ctx = NumContext::new();
    let mut symbol_height_diff_ctx = NumContext::new();
    let mut refinement_bitmap_ctx = vec![0u8; 2048];
    let mut hoff_ctx = NumContext::new();
    let mut voff_ctx = NumContext::new();
    let mut shoff_ctx = NumContext::new();
    let mut svoff_ctx = NumContext::new();
    let mut direct_bitmap_ctx = vec![0u8; 1024];
    let mut offset_type_ctx: u8 = 0;
    let mut flag_ctx: u8 = 0;

    // Preamble.
    if !shared_symbols.is_empty() {
        // Required-dict-or-reset: announce the inherited library size before
        // start-of-image so the decoder pre-populates `dict` from `shared_dict`.
        encode_num(&mut zp, &mut record_type_ctx, 0, 11, 9);
        encode_num(
            &mut zp,
            &mut inherit_dict_size_ctx,
            0,
            262142,
            shared_symbols.len() as i32,
        );
    }
    encode_num(&mut zp, &mut record_type_ctx, 0, 11, 0);
    encode_num(&mut zp, &mut image_size_ctx, 0, 262142, w);
    encode_num(&mut zp, &mut image_size_ctx, 0, 262142, h);
    zp.encode_bit(&mut flag_ctx, false);

    // Layout state — mirrors `LayoutState::new` in jb2.rs:1187.
    let mut layout = EncoderLayout::new(h);

    // Exact-match dedup: symbol_hash(w, h, packed-data) → dict indices. Buckets
    // (compared against `dict_entries` on a hit) keep this byte-identical while
    // avoiding a bitmap-data clone per connected component. Pre-populated from
    // shared_symbols so cross-page identical glyphs encode as rec-7 (copy).
    let mut dedup: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    // Stored dict entries (parallel to the decoder's `dict` vector) — needed
    // so refinement matching can score Hamming distance against historical
    // glyphs. Held by reference: `shared_symbols` and this page's own `ccs`
    // both outlive the encode, so the dict never needs to own (clone) a bitmap.
    // Drops the per-page `shared_symbols` deep-copy a bundled multi-page encode
    // paid on every page for an identical shared dictionary
    // (SHARED_DICT_CLONE_PER_PAGE / swarm P2).
    let mut dict_entries: Vec<&Bitmap> = Vec::new();
    // Index of dict entries by (w, h) for O(1) lookup of refinement candidates.
    let mut by_size: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    // Black-pixel count per dict entry, for the aligned-refinement prefilter.
    let mut dict_ink: Vec<u32> = Vec::new();
    let ink = |bm: &Bitmap| -> u32 { bm.data.iter().map(|b| b.count_ones()).sum() };
    for sym in shared_symbols {
        if aligned.is_some() {
            dict_ink.push(if is_tight(sym) {
                ink(sym)
            } else {
                NOT_REFINABLE
            });
        }
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

    // The decoder's per-page work budget: one unit per direct pixel,
    // `REFINE_PIXEL_WORK` per refinement pixel. A refinement that would push
    // the page over it is coded as a new symbol instead, so large refined
    // components cannot make an otherwise decodable page fail.
    let mut decode_work = 0usize;

    for &cc_idx in &order {
        let cc = &ccs[cc_idx];
        let cc_w = cc.bitmap.width as i32;
        let cc_h = cc.bitmap.height as i32;
        let cc_px = (cc.bitmap.width as usize).saturating_mul(cc.bitmap.height as usize);
        let refine_fits = decode_work
            .saturating_add(cc_px.saturating_mul(crate::REFINE_PIXEL_WORK))
            <= crate::MAX_PAGE_SYMBOL_WORK;
        // JB2 uses bottom-up y: y_jb2 is the bottom y of the symbol.
        let x_jb2 = cc.x as i32;
        let y_jb2 = h - cc.y as i32 - cc_h;

        let dkey = symbol_hash(cc.bitmap.width, cc.bitmap.height, &cc.bitmap.data);
        // `pixel_count` is 0 on the `encode_jb2_dict_with_symbols` path.
        let cc_ink = if aligned.is_some() {
            ink(&cc.bitmap)
        } else {
            0
        };
        let exact_match = dedup.get(&dkey).and_then(|cands| {
            cands.iter().copied().find(|&i| {
                let d = dict_entries[i];
                d.width == cc.bitmap.width
                    && d.height == cc.bitmap.height
                    && d.data == cc.bitmap.data
            })
        });

        // Choose record type:
        //   exact match → 7  (matched copy, blit only)
        //   near match  → 6  (matched refinement, blit only)
        //   otherwise   → 1  (new symbol, direct, add to dict + blit)
        enum Action {
            New,
            Copy(usize),
            /// Record-6 matched refinement — same-size (Phase A1) or cross-size
            /// (#322) experiment.
            #[cfg(feature = "experimental")]
            Refine(usize),
            /// Center-aligned refinement; `true` = record 4 (adds to dict).
            RefineAligned(usize, bool),
        }
        let action = if let Some(idx) = exact_match {
            Action::Copy(idx)
        } else {
            let candidates = by_size
                .get(&(cc.bitmap.width, cc.bitmap.height))
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            // Phase 4 (#224): lossy rec-7 substitution. Tried before
            // refinement so a same-size near-twin produces a smaller
            // rec-7 (no refinement bitmap) instead of a larger rec-6.
            let lossy_copy = if opts.lossy_threshold > 0.0 {
                find_lossy_copy_ref(&cc.bitmap, &dict_entries, candidates, opts.lossy_threshold)
            } else {
                None
            };
            let aligned_ref = aligned.filter(|_| refine_fits).and_then(|a| {
                find_aligned_refine_ref(
                    &cc.bitmap,
                    cc_ink,
                    &dict_entries,
                    &dict_ink,
                    &by_size,
                    a.max_dim_delta,
                    a.max_hamming_fraction,
                )
                .map(|idx| (idx, a.add_to_dict))
            });
            if let Some(idx) = lossy_copy {
                Action::Copy(idx)
            } else if let Some((idx, add)) = aligned_ref {
                Action::RefineAligned(idx, add)
            } else {
                // Experiment: divert fresh components with a dictionary twin to a
                // lossless rec-6 refinement. Same-size (Phase A1) is tried first —
                // no resampling, so the refinement context stays pixel-aligned —
                // then the cross-size #322 path. Behind `experimental`; the default
                // build always emits `New`.
                #[cfg(feature = "experimental")]
                if !refine_fits {
                    Action::New
                } else {
                    let same_size = opts.same_size_rec6.and_then(|frac| {
                        find_same_size_refine_ref(&cc.bitmap, &dict_entries, candidates, frac)
                    });
                    if let Some(idx) = same_size {
                        Action::Refine(idx)
                    } else if let Some(probe) = opts.cross_size_rec6_probe {
                        match find_cross_size_refine_ref(
                            &cc.bitmap,
                            &dict_entries,
                            &by_size,
                            probe.max_dim_delta,
                            probe.max_hamming_fraction,
                        ) {
                            Some(idx) => Action::Refine(idx),
                            None => Action::New,
                        }
                    } else {
                        Action::New
                    }
                }
                #[cfg(not(feature = "experimental"))]
                {
                    Action::New
                }
            }
        };

        decode_work = decode_work.saturating_add(match action {
            Action::New => cc_px,
            Action::Copy(_) => 0,
            #[cfg(feature = "experimental")]
            Action::Refine(_) => cc_px.saturating_mul(crate::REFINE_PIXEL_WORK),
            Action::RefineAligned(..) => cc_px.saturating_mul(crate::REFINE_PIXEL_WORK),
        });

        let dict_size = dict_entries.len();
        match &action {
            Action::New => {
                encode_num(&mut zp, &mut record_type_ctx, 0, 11, 1);
                encode_num(&mut zp, &mut symbol_width_ctx, 0, 262142, cc_w);
                encode_num(&mut zp, &mut symbol_height_ctx, 0, 262142, cc_h);
                encode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, &cc.bitmap);
            }
            Action::Copy(dict_idx) => {
                encode_num(&mut zp, &mut record_type_ctx, 0, 11, 7);
                encode_num(
                    &mut zp,
                    &mut symbol_index_ctx,
                    0,
                    (dict_size - 1) as i32,
                    *dict_idx as i32,
                );
            }
            Action::RefineAligned(dict_idx, add) => {
                let reference = dict_entries[*dict_idx];
                let wdiff = cc_w - reference.width as i32;
                let hdiff = cc_h - reference.height as i32;
                encode_num(
                    &mut zp,
                    &mut record_type_ctx,
                    0,
                    11,
                    if *add { 4 } else { 6 },
                );
                encode_num(
                    &mut zp,
                    &mut symbol_index_ctx,
                    0,
                    (dict_size - 1) as i32,
                    *dict_idx as i32,
                );
                encode_num(&mut zp, &mut symbol_width_diff_ctx, -262143, 262142, wdiff);
                encode_num(&mut zp, &mut symbol_height_diff_ctx, -262143, 262142, hdiff);
                encode_bitmap_ref(&mut zp, &mut refinement_bitmap_ctx, &cc.bitmap, reference);
            }
            #[cfg(feature = "experimental")]
            Action::Refine(dict_idx) => {
                // Record type 6: matched refinement, blit only. The decoder
                // computes the child size as `dict[idx].dim + diff`, decodes the
                // refinement bitmap against that reference, then blits — it does
                // not extend the dict (handled by the `Action::New` guard below).
                let reference = dict_entries[*dict_idx];
                let wdiff = cc_w - reference.width as i32;
                let hdiff = cc_h - reference.height as i32;
                encode_num(&mut zp, &mut record_type_ctx, 0, 11, 6);
                encode_num(
                    &mut zp,
                    &mut symbol_index_ctx,
                    0,
                    (dict_size - 1) as i32,
                    *dict_idx as i32,
                );
                encode_num(&mut zp, &mut symbol_width_diff_ctx, -262143, 262142, wdiff);
                encode_num(&mut zp, &mut symbol_height_diff_ctx, -262143, 262142, hdiff);
                encode_bitmap_ref(&mut zp, &mut refinement_bitmap_ctx, &cc.bitmap, reference);
            }
        }

        // ── Coordinate coding (Phase 2: same-line vs new_line) ────────────
        //
        // Decide whether the symbol fits the running baseline / line:
        //   * shoff = x_jb2 - last_right     (small, often 0..16 for a font)
        //   * svoff = y_jb2 - baseline_value (small, near 0 if same line)
        //
        // If both are within typical text-line tolerances we encode with
        // offset_type=false (same-line); else fall back to offset_type=true
        // (new_line), exactly mirroring what the decoder does in
        // jb2.rs::decode_symbol_coords.
        let shoff = x_jb2 - layout.last_right;
        let svoff = y_jb2 - layout.baseline_get();
        let same_line = layout.same_line_seen
            && svoff.abs() <= SAME_LINE_BASELINE_TOL
            && (-SAME_LINE_OVERLAP_TOL..=SAME_LINE_GAP_MAX).contains(&shoff);

        if same_line {
            zp.encode_bit(&mut offset_type_ctx, false);
            encode_num(&mut zp, &mut shoff_ctx, -262143, 262142, shoff);
            encode_num(&mut zp, &mut svoff_ctx, -262143, 262142, svoff);
            // Decoder: x = last_right + shoff, y = baseline + svoff.
            let nx = layout.last_right + shoff;
            let ny = layout.baseline_get() + svoff;
            layout.baseline_add(ny);
            layout.last_right = nx + cc_w - 1;
        } else {
            zp.encode_bit(&mut offset_type_ctx, true);
            let hoff = x_jb2 - layout.first_left;
            let voff = y_jb2 + cc_h - 1 - layout.first_bottom;
            encode_num(&mut zp, &mut hoff_ctx, -262143, 262142, hoff);
            encode_num(&mut zp, &mut voff_ctx, -262143, 262142, voff);
            // Decoder: nx = first_left+hoff, ny = first_bottom+voff-h+1, then
            // first_left = nx, first_bottom = ny, baseline.fill(ny).
            let nx = layout.first_left + hoff;
            let ny = layout.first_bottom + voff - cc_h + 1;
            layout.first_left = nx;
            layout.first_bottom = ny;
            layout.baseline_fill(ny);
            layout.baseline_add(ny);
            layout.last_right = nx + cc_w - 1;
            layout.same_line_seen = true;
        }

        // Records 1 (new) and 4 (refine + add) extend the dict — types 6 and
        // 7 are blit-only and the decoder leaves the dict untouched.
        let extends_dict = matches!(action, Action::New | Action::RefineAligned(_, true));
        if extends_dict {
            if aligned.is_some() {
                dict_ink.push(cc_ink);
            }
            let next_idx = dict_entries.len();
            dedup.entry(dkey).or_default().push(next_idx);
            by_size
                .entry((cc.bitmap.width, cc.bitmap.height))
                .or_default()
                .push(next_idx);
            dict_entries.push(&cc.bitmap);
        }
    }

    encode_num(&mut zp, &mut record_type_ctx, 0, 11, 11);
    let bytes = zp.finish();

    // Hand back the emitted blits in emission order. The bitmaps are moved
    // out of `ccs` (no clones); `dict_entries`' borrows of them end here.
    drop(dict_entries);
    let blits = order
        .iter()
        .map(|&i| {
            let cc = &mut ccs[i];
            EncodedBlit {
                x: cc.x,
                y: cc.y,
                bitmap: core::mem::replace(&mut cc.bitmap, Bitmap::new(0, 0)),
            }
        })
        .collect();
    (bytes, blits)
}

/// Same-line tolerances (Phase 2 of #188) used to decide between new_line
/// and same-line coordinate coding. Values are in image pixels and chosen
/// to cover normal text glyph variation while still treating a real line
/// break as a new_line. Looser thresholds reduce shoff/svoff magnitudes
/// at the cost of forcing same-line coding when the receiver would have
/// preferred a fresh baseline; tighter thresholds do the opposite.
pub(super) const SAME_LINE_BASELINE_TOL: i32 = 16;

pub(super) const SAME_LINE_OVERLAP_TOL: i32 = 16;

pub(super) const SAME_LINE_GAP_MAX: i32 = 1000;

/// Mirror of jb2::LayoutState held encoder-side.
pub(super) struct EncoderLayout {
    pub(super) first_left: i32,
    pub(super) first_bottom: i32,
    pub(super) last_right: i32,
    pub(super) baseline: [i32; 3],
    pub(super) baseline_idx: i32,
    /// `false` until the first symbol has been emitted — same-line coding
    /// is invalid before then because there is no "previous" baseline.
    pub(super) same_line_seen: bool,
}

impl EncoderLayout {
    pub(super) fn new(image_height: i32) -> Self {
        Self {
            first_left: -1,
            first_bottom: image_height - 1,
            last_right: 0,
            baseline: [0, 0, 0],
            baseline_idx: -1,
            same_line_seen: false,
        }
    }

    pub(super) fn baseline_fill(&mut self, val: i32) {
        self.baseline = [val, val, val];
    }

    pub(super) fn baseline_add(&mut self, val: i32) {
        self.baseline_idx += 1;
        if self.baseline_idx == 3 {
            self.baseline_idx = 0;
        }
        self.baseline[self.baseline_idx as usize] = val;
    }

    pub(super) fn baseline_get(&self) -> i32 {
        let (a, b, c) = (self.baseline[0], self.baseline[1], self.baseline[2]);
        if (a >= b && a <= c) || (a <= b && a >= c) {
            a
        } else if (b >= a && b <= c) || (b <= a && b >= c) {
            b
        } else {
            c
        }
    }
}
