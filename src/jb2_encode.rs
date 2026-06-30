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
use crate::chunk_encode::encode_info;
use crate::iff;

/// Encoder-wide default page resolution, matching
/// [`crate::djvu_encode::PageEncoder`]'s 300 dpi. Callers that have no dpi to
/// supply to the bundling helpers can pass this.
pub const BUNDLE_DEFAULT_DPI: u16 = 300;

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
/// `dpi` is stamped into every page's `INFO` chunk; pass
/// [`BUNDLE_DEFAULT_DPI`] when no specific resolution is known.
///
/// [`Jb2Dict`]: crate::jb2::Jb2Dict
pub fn encode_djvm_bundle_jb2(
    pages: &[Bitmap],
    shared_dict_page_threshold: usize,
    dpi: u16,
) -> Vec<u8> {
    let shared = cluster_shared_symbols(pages, shared_dict_page_threshold);
    encode_djvm_bundle_jb2_with_shared(pages, &shared, dpi)
}

/// Same as [`encode_djvm_bundle_jb2`] but uses a caller-supplied shared
/// dictionary instead of running [`cluster_shared_symbols`]. Lets callers
/// drive cluster selection (e.g. corpus benchmarks measuring different
/// Hamming thresholds) while reusing the IFF/DIRM emission logic.
pub fn encode_djvm_bundle_jb2_with_shared(
    pages: &[Bitmap],
    shared: &[Bitmap],
    dpi: u16,
) -> Vec<u8> {
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
        // Canonical INFO (see crate::chunk_encode::encode_info). Fixes the prior
        // hand-rolled bytes that hard-coded dpi 100 and gamma byte 1 (≈ 0.1),
        // diverging from the single-page/layered encoder's real dpi + gamma 2.2.
        let info = encode_info(page.width as u16, page.height as u16, dpi);

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

    assemble_djvm_bundle(comp_form_bodies)
}

/// Append one IFF chunk (`id` + 32-bit big-endian length + data + pad-to-even)
/// to `out`. Shared by the DJVM page/dict body builders.
pub(crate) fn push_chunk(out: &mut Vec<u8>, id: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(id);
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(data);
    if !data.len().is_multiple_of(2) {
        out.push(0);
    }
}

/// Build a component FORM **body**: the 4-byte form type followed by each chunk.
/// The outer `FORM` wrapper + size is added later by the IFF emission seam.
pub(crate) fn build_form_body(form_type: &[u8; 4], chunks: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(form_type);
    for (id, data) in chunks {
        push_chunk(&mut body, id, data);
    }
    body
}

/// Assemble a bundled DJVM from component FORM bodies (`(body, is_page, id)`):
/// a leading DIRM directory chunk + one component FORM each. Shared by the
/// masks-only ([`encode_djvm_bundle_jb2_with_shared`]) and layered bundlers.
pub(crate) fn assemble_djvm_bundle(comp_form_bodies: Vec<(Vec<u8>, bool, String)>) -> Vec<u8> {
    // ── DIRM metadata table (BZZ-compressed sizes/flags/ids/names/titles) ──
    //
    // Matches the bundled-format layout read by `djvu_document.rs::parse`:
    // flags=0x81 (bundled + v1.0), u16-be count, u32-be per-component offsets,
    // then this BZZ-compressed metadata blob.
    let n = comp_form_bodies.len();
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

    // ── Assemble through the IFF emission seam ──
    //
    // The bundle is a leading DIRM chunk followed by one component FORM per
    // page/dict. The DIRM carries a file-offset table pointing at each
    // component FORM, so this is the seam's documented two-pass shape: emit
    // once with a zeroed table to learn the offsets, refill the table, then
    // re-emit. `partial_emit_with_offsets` owns all framing/padding; only the
    // DIRM payload layout (bundled flag, count, offset table) lives here.
    let build_dirm = |offsets: &[u32]| -> Vec<u8> {
        let mut d = Vec::with_capacity(3 + 4 * n + bzz_meta.len());
        d.push(0x81); // bundled (high bit) + version 1
        d.extend_from_slice(&(n as u16).to_be_bytes());
        for &off in offsets {
            d.extend_from_slice(&off.to_be_bytes());
        }
        d.extend_from_slice(&bzz_meta);
        d
    };
    let emit = |dirm_data: Vec<u8>| -> (Vec<u8>, Vec<usize>) {
        let dirm = iff::Chunk::Leaf {
            id: *b"DIRM",
            data: dirm_data,
        };
        let mut parts: Vec<iff::EmitPart> = Vec::with_capacity(1 + n);
        parts.push(iff::EmitPart::Chunk(&dirm));
        parts.extend(
            comp_form_bodies
                .iter()
                .map(|(b, _, _)| iff::EmitPart::Form(b.as_slice())),
        );
        iff::partial_emit_with_offsets(*b"DJVM", &parts)
            .expect("DJVM bundle exceeds the 4 GiB IFF FORM limit")
    };

    // Pass 1: placeholder offsets → learn each component FORM's file offset
    // (`part_offsets[0]` is the DIRM itself; the rest are the components).
    let (_, part_offsets) = emit(build_dirm(&vec![0u32; n]));
    let comp_offsets: Vec<u32> = part_offsets[1..].iter().map(|&o| o as u32).collect();

    // Pass 2: real offsets written into the DIRM table.
    emit(build_dirm(&comp_offsets)).0
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

        let bundle = encode_djvm_bundle_jb2(&[p1.clone(), p2.clone()], 2, BUNDLE_DEFAULT_DPI);
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
    fn bundled_pages_carry_canonical_info_dpi_and_gamma() {
        // Regression: the bundled-mask path used to hand-roll INFO with a
        // hard-coded dpi 100 / gamma byte 1 (≈ 0.1), diverging from the
        // single-page encoder. It now routes through chunk_encode::encode_info.
        // A caller-supplied dpi (here a non-default 200) must be honoured —
        // proving it is threaded, not hard-coded — and gamma must read back as
        // the canonical 2.2 (the other half of the old divergence).
        let p1 = make_text_page(&[b"AABB", b"BABA"]);
        let p2 = make_text_page(&[b"ABAB", b"BABA"]);
        let bundle = encode_djvm_bundle_jb2(&[p1, p2], 2, 200);
        let doc = crate::djvu_document::DjVuDocument::parse(&bundle).expect("parse DJVM");
        assert_eq!(doc.page_count(), 2);
        for i in 0..2 {
            let page = doc.page(i).expect("page");
            assert_eq!(
                page.dpi(),
                200,
                "page {i} must carry the caller-supplied dpi (threaded, not the old hard-coded 100)"
            );
            assert!(
                (page.gamma() - 2.2).abs() < 1e-3,
                "page {i} gamma should be 2.2, was {} (old bug: ≈ 0.1)",
                page.gamma()
            );
        }
    }

    #[test]
    fn djvm_bundle_with_no_repeats_still_round_trips() {
        // Two pages with no shared CCs — bundle should still parse and decode
        // each page correctly (degraded path: empty Djbz / no shared dict).
        let mut p1 = Bitmap::new(20, 10);
        render_glyph(&mut p1, 2, 2, &glyph_a());
        let mut p2 = Bitmap::new(20, 10);
        render_glyph(&mut p2, 2, 2, &glyph_b());

        let bundle = encode_djvm_bundle_jb2(&[p1.clone(), p2.clone()], 2, BUNDLE_DEFAULT_DPI);
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

    #[test]
    fn djvm_bundle_dirm_offsets_point_at_component_forms() {
        // The bundled DIRM offset table is filled in by the emission seam's
        // two-pass `partial_emit_with_offsets`. Each offset must address the
        // `FORM` tag of its component within the file.
        let p1 = make_text_page(&[b"AABB", b"BABA"]);
        let p2 = make_text_page(&[b"AABB", b"BABA"]);
        let bundle = encode_djvm_bundle_jb2(&[p1, p2], 2, BUNDLE_DEFAULT_DPI);

        let form = iff::parse_form(&bundle).expect("parse DJVM");
        let dirm = form.chunks.iter().find(|c| &c.id == b"DIRM").expect("DIRM");
        let payload = crate::dirm::DirmPayload::decode(dirm.data).expect("decode DIRM");
        assert!(payload.is_bundled());
        assert!(
            !payload.offsets.is_empty(),
            "bundled DIRM must carry an offset table"
        );
        for &off in &payload.offsets {
            let off = off as usize;
            assert_eq!(
                &bundle[off..off + 4],
                b"FORM",
                "DIRM offset {off} must point at a component FORM tag"
            );
        }
        // One offset per component (1 shared DJVI + 2 page DJVUs here).
        assert_eq!(payload.offsets.len(), 3);
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
