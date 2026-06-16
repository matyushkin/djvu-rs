//! JB2 bilevel image encoder (re-export shim).
//!
//! The encoder implementation lives in the standalone `djvu-jb2` crate
//! ([`djvu_jb2::encode`]); this module re-exports it so the historical
//! `djvu_rs::jb2_encode::*` paths keep working.
//!
//! The multi-page DJVM **bundling** helpers stay here: they compose the codec
//! output with this crate's container assembly (DIRM/IFF) and BZZ metadata
//! compression, which are not part of the JB2 codec crate.

pub use djvu_jb2::encode::*;

use crate::Bitmap;

/// Encode a multi-page bilevel document as a bundled DJVM with a shared Djbz.
///
/// CCs that appear on at least `shared_dict_page_threshold` distinct input
/// pages are promoted into a single shared [`Jb2Dict`] (Djbz) emitted as a
/// `FORM:DJVI` component. Each page's `FORM:DJVU` then carries a small
/// `INCL` chunk pointing at that DJVI plus a Sjbz that references the shared
/// dictionary by index.
///
/// Returns the full DjVu container bytes (with `AT&T` magic, ready to write
/// to a file). With `pages.len() < 2` or `shared_dict_page_threshold > pages.len()`,
/// no symbols qualify for sharing and the encoder degrades to per-page
/// independent encoding (still wrapped in DJVM).
///
/// [`Jb2Dict`]: crate::jb2::Jb2Dict
pub fn encode_djvm_bundle_jb2(pages: &[Bitmap], shared_dict_page_threshold: usize) -> Vec<u8> {
    let shared = cluster_shared_symbols(pages, shared_dict_page_threshold);
    encode_djvm_bundle_jb2_with_shared(pages, &shared)
}

/// Same as [`encode_djvm_bundle_jb2`] but uses a caller-supplied shared
/// dictionary instead of running [`cluster_shared_symbols`]. Lets callers
/// drive cluster selection (e.g. corpus benchmarks measuring different
/// Hamming thresholds) while reusing the IFF/DIRM emission logic.
pub fn encode_djvm_bundle_jb2_with_shared(pages: &[Bitmap], shared: &[Bitmap]) -> Vec<u8> {
    let djbz_bytes = encode_jb2_djbz(shared);

    // ── Build component buffers (each = full FORM body, ready for IFF emit) ──
    //
    // DJVI component (only when there is something to share): contains a single
    // INFO chunk (none required by spec) + the Djbz.
    //
    // DJVU page components: INFO + INCL("dict0001.djvi") + Sjbz.
    let mut comp_form_bodies: Vec<(Vec<u8>, /*is_page*/ bool, String)> = Vec::new();

    let dict_id = "dict0001.djvi".to_string();
    let has_shared = !shared.is_empty();
    if has_shared {
        let mut djvi_body = Vec::new();
        djvi_body.extend_from_slice(b"DJVI");
        djvi_body.extend_from_slice(b"Djbz");
        djvi_body.extend_from_slice(&(djbz_bytes.len() as u32).to_be_bytes());
        djvi_body.extend_from_slice(&djbz_bytes);
        if !djbz_bytes.len().is_multiple_of(2) {
            djvi_body.push(0);
        }
        comp_form_bodies.push((djvi_body, false, dict_id.clone()));
    }

    let shared_ref: &[Bitmap] = shared;
    for (page_idx, page) in pages.iter().enumerate() {
        let sjbz = encode_jb2_dict_with_shared(page, shared_ref);
        let mut info = Vec::with_capacity(10);
        info.extend_from_slice(&(page.width as u16).to_be_bytes());
        info.extend_from_slice(&(page.height as u16).to_be_bytes());
        info.extend_from_slice(&[24, 0, 100, 0, 1, 0]); // version major, minor, dpi(le16), gamma, rotation

        let mut djvu_body = Vec::new();
        djvu_body.extend_from_slice(b"DJVU");
        djvu_body.extend_from_slice(b"INFO");
        djvu_body.extend_from_slice(&(info.len() as u32).to_be_bytes());
        djvu_body.extend_from_slice(&info);
        if !info.len().is_multiple_of(2) {
            djvu_body.push(0);
        }
        if has_shared {
            let incl_payload = dict_id.as_bytes();
            djvu_body.extend_from_slice(b"INCL");
            djvu_body.extend_from_slice(&(incl_payload.len() as u32).to_be_bytes());
            djvu_body.extend_from_slice(incl_payload);
            if !incl_payload.len().is_multiple_of(2) {
                djvu_body.push(0);
            }
        }
        djvu_body.extend_from_slice(b"Sjbz");
        djvu_body.extend_from_slice(&(sjbz.len() as u32).to_be_bytes());
        djvu_body.extend_from_slice(&sjbz);
        if !sjbz.len().is_multiple_of(2) {
            djvu_body.push(0);
        }

        let pid = format!("p{:04}.djvu", page_idx + 1);
        comp_form_bodies.push((djvu_body, true, pid));
    }

    // ── Build DIRM directly (bundled, with offsets) ──
    //
    // Reuses the shape of `crate::djvm::build_djvm` but inlined here because
    // we have FORM bodies (not full FORM chunks with header) to embed. The
    // simpler path: build full FORM chunks here, then call `iff::emit_form`.
    // Each component is a FORM chunk: { id: "FORM", body: <DJVU/DJVI ...> }.
    let comp_form_data: Vec<&[u8]> = comp_form_bodies
        .iter()
        .map(|(b, _, _)| b.as_slice())
        .collect();

    // DIRM payload: build matching the bundled-format layout in
    // `djvu_document.rs::parse` (flags=0x81 → bundled+1.0; count u16-be;
    // per-component offsets u32-be; bzz-compressed metadata table).
    let n = comp_form_bodies.len();
    let mut dirm = Vec::new();
    dirm.push(0x81); // bundled (high bit) + version 1
    dirm.extend_from_slice(&(n as u16).to_be_bytes());

    // Compute offsets after the DIRM chunk has been laid down.
    // Layout: AT&T (4) + FORM (4) + form_size (4) + "DJVM" (4) + "DIRM" (4) +
    //         dirm_size (4) + dirm_payload_with_offsets+bzz_meta + pad +
    //         each FORM chunk header (8) + body + pad.
    //
    // Offsets in the DIRM are *file-byte* offsets to the AT&T-stripped FORM
    // chunk header for each component. So offset[i] = position of "FORM" id
    // bytes for that component within the file.
    //
    // We don't know the DIRM size until we know the offsets; resolve via
    // two-pass: build metadata table first, then layout.
    let mut meta = Vec::new();
    for (body, _, _) in &comp_form_bodies {
        let total = body.len() + 8; // FORM + size + body
        meta.extend_from_slice(&(total as u32).to_be_bytes()[1..4]); // 24-bit size
    }
    for (_, is_page, _) in &comp_form_bodies {
        let flag = if *is_page { 1u8 } else { 0u8 };
        meta.push(flag);
    }
    for (_, _, id) in &comp_form_bodies {
        meta.extend_from_slice(id.as_bytes());
        meta.push(0);
    }
    for (_, _, id) in &comp_form_bodies {
        meta.extend_from_slice(id.as_bytes());
        meta.push(0);
    }
    meta.extend(core::iter::repeat_n(0u8, n)); // empty titles
    let bzz_meta = crate::bzz_encode::bzz_encode(&meta);

    // dirm payload final size = 1 (flags) + 2 (count) + 4*n (offsets) + bzz_meta.len()
    let dirm_size = 1 + 2 + 4 * n + bzz_meta.len();

    // Compute DJVM body size and component offsets.
    let dirm_chunk_total = 8 + dirm_size + (dirm_size & 1); // header + payload + pad
    let mut form_body_size: usize = 4; // "DJVM"
    form_body_size += dirm_chunk_total;
    let mut comp_offsets: Vec<u32> = Vec::with_capacity(n);
    for body in &comp_form_data {
        // File offset = AT&T(4) + FORM(4) + size(4) + DJVM(4) + dirm_chunk_total
        //             + sum-of-prior-comp-totals
        // The decoder treats DIRM offsets as byte offsets from start of file
        // pointing at the "FORM" id bytes of the component. Offset 0 of the
        // file = 'A' of "AT&T", so offset = 12 + 4 + dirm_chunk_total + prior.
        let off = 4 + 4 + 4 + 4 + dirm_chunk_total + (form_body_size - 4 - dirm_chunk_total);
        comp_offsets.push(off as u32);
        let tot = body.len() + 8;
        form_body_size += tot + (tot & 1); // pad component to even
    }

    // Now write final dirm payload with offsets.
    let _ = dirm; // computed above for reference; final form built fresh below.
    let mut dirm_full = Vec::with_capacity(dirm_size);
    dirm_full.push(0x81);
    dirm_full.extend_from_slice(&(n as u16).to_be_bytes());
    for off in &comp_offsets {
        dirm_full.extend_from_slice(&off.to_be_bytes());
    }
    dirm_full.extend_from_slice(&bzz_meta);
    debug_assert_eq!(dirm_full.len(), dirm_size);

    // Emit final file.
    let mut out = Vec::with_capacity(12 + form_body_size);
    out.extend_from_slice(b"AT&T");
    out.extend_from_slice(b"FORM");
    out.extend_from_slice(&(form_body_size as u32).to_be_bytes());
    out.extend_from_slice(b"DJVM");
    out.extend_from_slice(b"DIRM");
    out.extend_from_slice(&(dirm_size as u32).to_be_bytes());
    out.extend_from_slice(&dirm_full);
    if dirm_size & 1 == 1 {
        out.push(0);
    }
    for body in &comp_form_data {
        out.extend_from_slice(b"FORM");
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(body);
        if body.len() & 1 == 1 {
            out.push(0);
        }
    }

    out
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn render_glyph(bm: &mut Bitmap, x: u32, y: u32, glyph: &[&[u8]]) {
        for (gy, row) in glyph.iter().enumerate() {
            for (gx, &c) in row.iter().enumerate() {
                if c == b'#' {
                    bm.set(x + gx as u32, y + gy as u32, true);
                }
            }
        }
    }

    fn glyph_a() -> Vec<&'static [u8]> {
        vec![
            b" ## " as &[u8],
            b"#  #" as &[u8],
            b"####" as &[u8],
            b"#  #" as &[u8],
            b"#  #" as &[u8],
        ]
    }
    fn glyph_b() -> Vec<&'static [u8]> {
        vec![
            b"### " as &[u8],
            b"#  #" as &[u8],
            b"### " as &[u8],
            b"#  #" as &[u8],
            b"### " as &[u8],
        ]
    }

    fn make_text_page(words: &[&[u8]]) -> Bitmap {
        let mut bm = Bitmap::new(80, 30);
        let mut x = 4;
        for word in words {
            for &letter in *word {
                let g = match letter {
                    b'A' => glyph_a(),
                    b'B' => glyph_b(),
                    _ => continue,
                };
                render_glyph(&mut bm, x, 8, &g);
                x += 6;
            }
            x += 4;
        }
        bm
    }

    fn assert_decoded_eq(src: &Bitmap, decoded: &Bitmap) {
        assert_eq!(src.width, decoded.width, "width mismatch");
        assert_eq!(src.height, decoded.height, "height mismatch");
        let mut mismatches = 0u32;
        for y in 0..src.height {
            for x in 0..src.width {
                if src.get(x, y) != decoded.get(x, y) {
                    mismatches += 1;
                }
            }
        }
        assert_eq!(mismatches, 0, "{mismatches} pixel mismatches");
    }

    #[test]
    fn shared_dict_smaller_than_independent_for_repeated_pages() {
        // Build two identical text pages. Encoding them with a shared Djbz
        // should produce strictly smaller total bytes than two independent
        // dict encodings.
        let p1 = make_text_page(&[b"AABB", b"BABA"]);
        let p2 = make_text_page(&[b"AABB", b"BABA"]);

        let independent_total = encode_jb2_dict(&p1).len() + encode_jb2_dict(&p2).len();

        let bundle = encode_djvm_bundle_jb2(&[p1.clone(), p2.clone()], 2);
        assert!(!bundle.is_empty());

        // Round-trip via the document parser.
        let doc = crate::djvu_document::DjVuDocument::parse(&bundle).expect("parse DJVM");
        assert_eq!(doc.page_count(), 2);
        let d1 = doc
            .page(0)
            .expect("page 0")
            .extract_mask()
            .expect("extract_mask 0")
            .expect("mask 0 present");
        let d2 = doc
            .page(1)
            .expect("page 1")
            .extract_mask()
            .expect("extract_mask 1")
            .expect("mask 1 present");
        assert_decoded_eq(&p1, &d1);
        assert_decoded_eq(&p2, &d2);

        // Size win sanity check (bundle includes DIRM + IFF wrappers, so we
        // only assert the pure JB2 payload across (Djbz + 2× Sjbz) is smaller
        // than the pure 2× independent Sjbz).
        let shared = cluster_shared_symbols(&[p1.clone(), p2.clone()], 2);
        assert!(
            !shared.is_empty(),
            "two identical pages should produce shared symbols"
        );
        let djbz = encode_jb2_djbz(&shared);
        let sjbz1 = encode_jb2_dict_with_shared(&p1, &shared);
        let sjbz2 = encode_jb2_dict_with_shared(&p2, &shared);
        let shared_jb2_total = djbz.len() + sjbz1.len() + sjbz2.len();
        assert!(
            shared_jb2_total < independent_total,
            "expected shared jb2 < independent: shared={}  independent={}",
            shared_jb2_total,
            independent_total
        );
    }

    #[test]
    fn djvm_bundle_with_no_repeats_still_round_trips() {
        // Two pages with no shared CCs — bundle should still parse and decode
        // each page correctly (degraded path: empty Djbz / no shared dict).
        let mut p1 = Bitmap::new(20, 10);
        render_glyph(&mut p1, 2, 2, &glyph_a());
        let mut p2 = Bitmap::new(20, 10);
        render_glyph(&mut p2, 2, 2, &glyph_b());

        let bundle = encode_djvm_bundle_jb2(&[p1.clone(), p2.clone()], 2);
        let doc = crate::djvu_document::DjVuDocument::parse(&bundle).expect("parse DJVM");
        assert_eq!(doc.page_count(), 2);
        let d1 = doc
            .page(0)
            .expect("page 0")
            .extract_mask()
            .expect("extract_mask 0")
            .expect("mask 0 present");
        let d2 = doc
            .page(1)
            .expect("page 1")
            .extract_mask()
            .expect("extract_mask 1")
            .expect("mask 1 present");
        assert_decoded_eq(&p1, &d1);
        assert_decoded_eq(&p2, &d2);
    }

    // ── #322 cross-size record-6 probe corpus driver ─────────────────────────
    //
    // The probe types live behind the `experimental` feature in `djvu-jb2`; the
    // measurement driver consumes real corpus page masks via `DjVuDocument`, so
    // it lives here next to the container layer rather than in the codec crate.

    #[cfg(feature = "experimental")]
    const REC6_PROBE: CrossSizeRec6Probe = CrossSizeRec6Probe {
        max_dim_delta: 2,
        max_hamming_fraction: 0.05,
    };

    #[cfg(feature = "experimental")]
    fn probe_opts() -> Jb2EncodeOptions {
        Jb2EncodeOptions {
            cross_size_rec6_probe: Some(REC6_PROBE),
            ..Jb2EncodeOptions::default()
        }
    }

    /// Aggregated cross-size rec-6 probe measurement over a set of page masks.
    #[cfg(feature = "experimental")]
    #[derive(Default)]
    struct Rec6ProbeReport {
        pages: usize,
        pages_changed: usize,
        baseline_bytes: u64,
        probe_bytes: u64,
        roundtrip_failures: usize,
    }

    #[cfg(feature = "experimental")]
    fn measure_rec6_probe(path: &str, max_pages: usize) -> Rec6ProbeReport {
        let data = std::fs::read(path).unwrap();
        let doc = crate::DjVuDocument::parse(&data).unwrap();
        let opts = probe_opts();
        let mut report = Rec6ProbeReport::default();
        let n = doc.page_count().min(max_pages);
        for i in 0..n {
            let page = doc.page(i).unwrap();
            let Some(src) = page.extract_mask().unwrap() else {
                continue;
            };
            if src.width == 0 || src.height == 0 {
                continue;
            }
            report.pages += 1;

            let baseline = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
            let probe = encode_jb2_dict_with_options(&src, &[], &opts);
            report.baseline_bytes += baseline.len() as u64;
            report.probe_bytes += probe.len() as u64;
            if baseline != probe {
                report.pages_changed += 1;
            }

            // Round-trip the probe stream and require pixel-exact reconstruction
            // (the cross-size rec-6 refinement is lossless).
            match crate::jb2::decode(&probe, None) {
                Ok(decoded) => {
                    let same = decoded.width == src.width
                        && decoded.height == src.height
                        && (0..src.height)
                            .all(|y| (0..src.width).all(|x| decoded.get(x, y) == src.get(x, y)));
                    if !same {
                        report.roundtrip_failures += 1;
                    }
                }
                Err(_) => report.roundtrip_failures += 1,
            }
        }
        report
    }

    /// Experiment driver for #322. Ignored by default — it re-encodes corpus
    /// page masks (slow) and prints the measured byte deltas + round-trip
    /// status recorded in PERF_EXPERIMENTS.md. Run with:
    ///   cargo test --lib --release --features experimental cross_size_rec6_probe_corpus_measurement -- --ignored --nocapture
    #[cfg(feature = "experimental")]
    #[test]
    #[ignore = "slow corpus re-encode; measurement driver for #322"]
    fn cross_size_rec6_probe_corpus_measurement() {
        for (path, max_pages) in [
            ("tests/corpus/watchmaker.djvu", usize::MAX),
            ("tests/corpus/pathogenic_bacteria_1896.djvu", 40),
        ] {
            let r = measure_rec6_probe(path, max_pages);
            let delta = r.probe_bytes as i64 - r.baseline_bytes as i64;
            let pct = if r.baseline_bytes > 0 {
                100.0 * delta as f64 / r.baseline_bytes as f64
            } else {
                0.0
            };
            println!(
                "{path}: pages={} changed={} baseline={}B probe={}B delta={}B ({pct:+.3}%) roundtrip_failures={}",
                r.pages,
                r.pages_changed,
                r.baseline_bytes,
                r.probe_bytes,
                delta,
                r.roundtrip_failures
            );
            assert_eq!(
                r.roundtrip_failures, 0,
                "cross-size rec-6 probe must round-trip losslessly on {path}"
            );
        }
    }
}
