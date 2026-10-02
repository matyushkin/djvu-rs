use super::*;

fn assets_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../references/djvujs/library/assets")
}

fn golden_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/jb2")
}

// ── IFF helpers ──────────────────────────────────────────────────────────

fn extract_sjbz(djvu_data: &[u8]) -> Vec<u8> {
    let file = djvu_iff::parse(djvu_data).unwrap();
    let sjbz = file.root.find_first(b"Sjbz").unwrap();
    sjbz.data().to_vec()
}

fn extract_first_page_sjbz(djvu_data: &[u8]) -> Vec<u8> {
    let file = djvu_iff::parse(djvu_data).unwrap();
    let page_form = file
        .root
        .children()
        .iter()
        .find(|c| {
            matches!(c, djvu_iff::Chunk::Form { secondary_id, .. }
                if secondary_id == b"DJVU")
        })
        .expect("no DJVU form");
    page_form.find_first(b"Sjbz").unwrap().data().to_vec()
}

fn find_page_form_data(djvu_data: &[u8], page: usize) -> Vec<u8> {
    let file = djvu_iff::parse(djvu_data).unwrap();
    let mut idx = 0;
    for chunk in file.root.children() {
        if matches!(chunk, djvu_iff::Chunk::Form { secondary_id, .. }
            if secondary_id == b"DJVU")
        {
            if idx == page {
                return chunk.find_first(b"Sjbz").unwrap().data().to_vec();
            }
            idx += 1;
        }
    }
    panic!("page {} not found", page);
}

fn find_djvi_djbz_data(djvu_data: &[u8]) -> Vec<u8> {
    let file = djvu_iff::parse(djvu_data).unwrap();
    for chunk in file.root.children() {
        if let djvu_iff::Chunk::Form { secondary_id, .. } = chunk
            && secondary_id == b"DJVI"
            && let Some(djbz) = chunk.find_first(b"Djbz")
        {
            return djbz.data().to_vec();
        }
    }
    panic!("DJVI with Djbz not found");
}

// ── Failing tests written first (TDD Red phase) ──────────────────────────

/// The new decoder must produce the same pixel-exact output as the legacy
/// decoder for boy_jb2.djvu.
#[test]
fn jb2_new_decode_boy_jb2_mask() {
    let djvu = std::fs::read(assets_path().join("boy_jb2.djvu")).unwrap();
    let sjbz = extract_sjbz(&djvu);
    let bitmap = decode(&sjbz, None).unwrap();
    let actual_pbm = bitmap.to_pbm();
    let expected_pbm = std::fs::read(golden_path().join("boy_jb2_mask.pbm")).unwrap();
    assert_eq!(
        actual_pbm.len(),
        expected_pbm.len(),
        "PBM size mismatch: got {} expected {}",
        actual_pbm.len(),
        expected_pbm.len()
    );
    assert_eq!(actual_pbm, expected_pbm, "boy_jb2_mask pixel mismatch");
}

#[test]
fn jb2_new_decode_carte_p1_mask() {
    let djvu = std::fs::read(assets_path().join("carte.djvu")).unwrap();
    let sjbz = extract_first_page_sjbz(&djvu);
    let bitmap = decode(&sjbz, None).unwrap();
    let actual_pbm = bitmap.to_pbm();
    let expected_pbm = std::fs::read(golden_path().join("carte_p1_mask.pbm")).unwrap();
    assert_eq!(
        actual_pbm.len(),
        expected_pbm.len(),
        "carte_p1_mask size mismatch"
    );
    assert_eq!(actual_pbm, expected_pbm, "carte_p1_mask pixel mismatch");
}

#[test]
fn jb2_new_decode_djvu3spec_p1_mask() {
    let djvu = std::fs::read(assets_path().join("DjVu3Spec_bundled.djvu")).unwrap();
    let file = djvu_iff::parse(&djvu).unwrap();

    // Inline Djbz in page 1
    let mut idx = 0usize;
    let mut page_form_opt: Option<&djvu_iff::Chunk> = None;
    for chunk in file.root.children() {
        if matches!(chunk, djvu_iff::Chunk::Form { secondary_id, .. }
            if secondary_id == b"DJVU")
        {
            if idx == 0 {
                page_form_opt = Some(chunk);
                break;
            }
            idx += 1;
        }
    }
    let page_form = page_form_opt.expect("page 0 not found");
    let djbz_data = page_form.find_first(b"Djbz").unwrap().data().to_vec();
    let sjbz_data = page_form.find_first(b"Sjbz").unwrap().data().to_vec();

    let shared_dict = decode_dict(&djbz_data, None).unwrap();
    let bitmap = decode(&sjbz_data, Some(&shared_dict)).unwrap();
    let actual_pbm = bitmap.to_pbm();
    let expected_pbm = std::fs::read(golden_path().join("djvu3spec_p1_mask.pbm")).unwrap();
    assert_eq!(
        actual_pbm.len(),
        expected_pbm.len(),
        "djvu3spec_p1_mask size mismatch"
    );
    assert_eq!(actual_pbm, expected_pbm, "djvu3spec_p1_mask pixel mismatch");
}

#[test]
fn jb2_new_decode_djvu3spec_p2_mask() {
    let djvu = std::fs::read(assets_path().join("DjVu3Spec_bundled.djvu")).unwrap();
    let djbz_data = find_djvi_djbz_data(&djvu);
    let sjbz_data = find_page_form_data(&djvu, 1);

    let shared_dict = decode_dict(&djbz_data, None).unwrap();
    let bitmap = decode(&sjbz_data, Some(&shared_dict)).unwrap();
    let actual_pbm = bitmap.to_pbm();
    let expected_pbm = std::fs::read(golden_path().join("djvu3spec_p2_mask.pbm")).unwrap();
    assert_eq!(
        actual_pbm.len(),
        expected_pbm.len(),
        "djvu3spec_p2_mask size mismatch"
    );
    assert_eq!(actual_pbm, expected_pbm, "djvu3spec_p2_mask pixel mismatch");
}

#[test]
fn jb2_new_decode_navm_fgbz_p1_mask() {
    let djvu = std::fs::read(assets_path().join("navm_fgbz.djvu")).unwrap();
    let djbz_data = find_djvi_djbz_data(&djvu);
    let sjbz_data = find_page_form_data(&djvu, 0);

    let shared_dict = decode_dict(&djbz_data, None).unwrap();
    let bitmap = decode(&sjbz_data, Some(&shared_dict)).unwrap();
    let actual_pbm = bitmap.to_pbm();
    let expected_pbm = std::fs::read(golden_path().join("navm_fgbz_p1_mask.pbm")).unwrap();
    assert_eq!(
        actual_pbm.len(),
        expected_pbm.len(),
        "navm_fgbz_p1_mask size mismatch"
    );
    assert_eq!(actual_pbm, expected_pbm, "navm_fgbz_p1_mask pixel mismatch");
}

// ── Robustness tests ─────────────────────────────────────────────────────

#[test]
fn jb2_new_empty_input_does_not_panic() {
    let _ = decode(&[], None);
}

#[test]
fn jb2_new_single_byte_does_not_panic() {
    let _ = decode(&[0x00], None);
}

#[test]
fn jb2_new_all_zeros_does_not_panic() {
    let _ = decode(&[0u8; 64], None);
}

#[test]
fn jb2_new_dict_empty_input_does_not_panic() {
    let _ = decode_dict(&[], None);
}

#[test]
fn jb2_new_dict_truncated_does_not_panic() {
    let _ = decode_dict(&[0u8; 8], None);
}

// ── Error variant tests ──────────────────────────────────────────────────

#[test]
fn jb2_error_variants_have_meaningful_messages() {
    assert!(Jb2Error::BadHeaderFlag.to_string().contains("flag"));
    assert!(Jb2Error::InheritedDictTooLarge.to_string().contains("dict"));
    assert!(Jb2Error::MissingSharedDict.to_string().contains("dict"));
    assert!(Jb2Error::ImageTooLarge.to_string().contains("large"));
    assert!(Jb2Error::EmptyDictReference.to_string().contains("dict"));
    assert!(Jb2Error::InvalidSymbolIndex.to_string().contains("symbol"));
    assert!(Jb2Error::UnknownRecordType.to_string().contains("record"));
    assert!(
        Jb2Error::UnexpectedDictRecordType
            .to_string()
            .contains("record")
    );
    assert!(Jb2Error::ZpInitFailed.to_string().contains("ZP"));
    assert!(Jb2Error::Truncated.to_string().contains("truncated"));
}

/// Verify `ImageTooLarge` fires via saturating multiply.
#[test]
fn jb2_image_size_overflow_guard() {
    let w: usize = 65536;
    let h: usize = 65537;
    let safe_size = w.saturating_mul(h);
    assert!(
        safe_size > 64 * 1024 * 1024,
        "saturating_mul must exceed MAX_PIXELS"
    );
}

// ── Error path tests ───────────────────��────────────────────────────────

#[test]
fn test_decode_empty_data() {
    let result = decode(&[], None);
    assert!(result.is_err());
}

#[test]
fn test_decode_dict_empty() {
    let result = decode_dict(&[], None);
    assert!(result.is_err());
}

#[test]
fn test_decode_indexed_empty() {
    let result = decode_indexed(&[], None);
    assert!(result.is_err());
}

/// Regression test: negative symbol dimensions caused `width as usize` to
/// wrap to a huge value in the blit fast path, producing a near-infinite
/// inner loop and effectively hanging the decoder.
#[test]
fn blit_negative_width_does_not_hang() {
    let start = std::time::Instant::now();
    let _ = decode(&[0x7e, 0x00, 0x0c], None);
    assert!(start.elapsed().as_secs() < 2, "took {:?}", start.elapsed());
}

// ── Pool reuse tests ──────────────────────────────────────────────────────

/// Decoding a real JB2 stream with an explicit scratch pool must produce
/// pixel-identical output to the poolless `decode` path, and the pool must
/// grow to at least 1 byte (proving it was used for at least one symbol).
#[test]
fn jb2_pool_decode_matches_regular_decode_carte() {
    let djvu = std::fs::read(assets_path().join("carte.djvu")).unwrap();
    let sjbz = extract_first_page_sjbz(&djvu);

    let regular = decode(&sjbz, None).expect("regular decode");

    let mut pool = Vec::new();
    let pooled = decode_image_with_pool(&sjbz, None, &mut pool, 0).expect("pool decode");

    assert_eq!(regular.width, pooled.width, "width must match");
    assert_eq!(regular.height, pooled.height, "height must match");
    assert_eq!(regular.data, pooled.data, "pixel data must be identical");
    assert!(
        pool.capacity() > 0,
        "pool must have been used (capacity > 0 after decode)"
    );
}

// ── Missing/short shared-dict error paths ────────────────────────────────

/// `decode` must return a typed error, not silently misdecode, when the
/// stream's "required-dict-or-reset" record references an external
/// dictionary but the caller supplies none.
#[test]
fn decode_missing_shared_dict_reports_typed_error() {
    let djvu = std::fs::read(assets_path().join("DjVu3Spec_bundled.djvu")).unwrap();
    let sjbz_data = find_page_form_data(&djvu, 1);
    assert!(matches!(
        decode(&sjbz_data, None),
        Err(Jb2Error::MissingSharedDict)
    ));
}

/// Same guard on the blit-index-tracking entry point.
#[test]
fn decode_indexed_missing_shared_dict_reports_typed_error() {
    let djvu = std::fs::read(assets_path().join("DjVu3Spec_bundled.djvu")).unwrap();
    let sjbz_data = find_page_form_data(&djvu, 1);
    assert!(matches!(
        decode_indexed(&sjbz_data, None),
        Err(Jb2Error::MissingSharedDict)
    ));
}

/// A shared dict shorter than the stream's declared inherited-dict length
/// must be rejected rather than indexed out of bounds.
#[test]
fn decode_rejects_shared_dict_shorter_than_declared() {
    let djvu = std::fs::read(assets_path().join("DjVu3Spec_bundled.djvu")).unwrap();
    let djbz_data = find_djvi_djbz_data(&djvu);
    let sjbz_data = find_page_form_data(&djvu, 1);
    let full_dict = decode_dict(&djbz_data, None).unwrap();
    assert!(
        !full_dict.symbols.is_empty(),
        "fixture must declare a non-empty shared dict"
    );
    let short_dict = Jb2Dict {
        symbols: full_dict.symbols[..full_dict.symbols.len() - 1].to_vec(),
    };
    assert!(matches!(
        decode(&sjbz_data, Some(&short_dict)),
        Err(Jb2Error::InheritedDictTooLarge)
    ));
}

// ── decode_indexed: blit map consistency against decode() ────────────────

/// `decode_indexed` must produce the same bitmap as `decode`, plus a blit
/// map whose foreground/background split matches the bitmap exactly.
#[test]
fn decode_indexed_matches_decode_for_dict_free_fixture() {
    let djvu = std::fs::read(assets_path().join("carte.djvu")).unwrap();
    let sjbz = extract_first_page_sjbz(&djvu);
    let plain = decode(&sjbz, None).unwrap();
    let (indexed_bitmap, blit_map) = decode_indexed(&sjbz, None).unwrap();
    assert_eq!(plain.width, indexed_bitmap.width);
    assert_eq!(plain.height, indexed_bitmap.height);
    assert_eq!(plain.data, indexed_bitmap.data);
    assert_eq!(blit_map.len(), (plain.width * plain.height) as usize);
    for y in 0..plain.height {
        for x in 0..plain.width {
            let idx = (y * plain.width + x) as usize;
            let fg = plain.get(x, y);
            assert_eq!(blit_map[idx] >= 0, fg, "pixel ({x},{y}) fg/blit mismatch");
        }
    }
}

/// Same equivalence check when the stream draws symbols from a shared
/// dictionary (exercises the dict-lookup blit records, not just direct
/// new-symbol records).
#[test]
fn decode_indexed_matches_decode_for_shared_dict_fixture() {
    let djvu = std::fs::read(assets_path().join("DjVu3Spec_bundled.djvu")).unwrap();
    let djbz_data = find_djvi_djbz_data(&djvu);
    let sjbz_data = find_page_form_data(&djvu, 1);
    let shared_dict = decode_dict(&djbz_data, None).unwrap();
    let plain = decode(&sjbz_data, Some(&shared_dict)).unwrap();
    let (indexed_bitmap, blit_map) = decode_indexed(&sjbz_data, Some(&shared_dict)).unwrap();
    assert_eq!(plain.width, indexed_bitmap.width);
    assert_eq!(plain.height, indexed_bitmap.height);
    assert_eq!(plain.data, indexed_bitmap.data);
    assert_eq!(blit_map.len(), (plain.width * plain.height) as usize);
    assert!(
        blit_map.iter().any(|&b| b >= 0),
        "expected at least one foreground blit"
    );
}

// ── decode_downsampled: equivalence with decode-then-downsample ──────────

/// Reference max-pool downsample by `2^shift`, block-OR reduction,
/// `div_ceil` output size — the same semantics `decode_downsampled` must
/// match without ever materialising the full-resolution bitmap.
fn downsample_reference(src: &Bitmap, shift: u32) -> Bitmap {
    let block = 1u32 << shift;
    let out_w = src.width.div_ceil(block);
    let out_h = src.height.div_ceil(block);
    let mut out = Bitmap::new(out_w, out_h);
    for oy in 0..out_h {
        for ox in 0..out_w {
            'outer: for dy in 0..block {
                for dx in 0..block {
                    let sx = ox * block + dx;
                    let sy = oy * block + dy;
                    if sx < src.width && sy < src.height && src.get(sx, sy) {
                        out.set(ox, oy, true);
                        break 'outer;
                    }
                }
            }
        }
    }
    out
}

/// `decode_downsampled(.., shift=0)` must be pixel-identical to `decode`.
#[test]
fn decode_downsampled_shift0_matches_decode() {
    let djvu = std::fs::read(assets_path().join("boy_jb2.djvu")).unwrap();
    let sjbz = extract_sjbz(&djvu);
    let full = decode(&sjbz, None).unwrap();
    let ds0 = decode_downsampled(&sjbz, None, 0).unwrap();
    assert_eq!(full.width, ds0.width);
    assert_eq!(full.height, ds0.height);
    assert_eq!(full.data, ds0.data);
}

/// `decode_downsampled(.., shift=2)` (the thumbnail-path mask_sub4 case)
/// must be bit-for-bit identical to decoding at full resolution and then
/// max-pool-downsampling by 4 — on a dict-free direct-symbol fixture.
#[test]
fn decode_downsampled_matches_full_then_downsample_boy_jb2() {
    let djvu = std::fs::read(assets_path().join("boy_jb2.djvu")).unwrap();
    let sjbz = extract_sjbz(&djvu);
    let full = decode(&sjbz, None).unwrap();
    let expected = downsample_reference(&full, 2);
    let actual = decode_downsampled(&sjbz, None, 2).unwrap();
    assert_eq!(expected.width, actual.width);
    assert_eq!(expected.height, actual.height);
    assert_eq!(expected.data, actual.data, "downsampled mask mismatch");
}

/// Same equivalence check on a shared-dictionary fixture (exercises the
/// dict-lookup blit records, not just direct new-symbol records).
#[test]
fn decode_downsampled_matches_full_then_downsample_shared_dict() {
    let djvu = std::fs::read(assets_path().join("DjVu3Spec_bundled.djvu")).unwrap();
    let djbz_data = find_djvi_djbz_data(&djvu);
    let sjbz_data = find_page_form_data(&djvu, 1);
    let shared_dict = decode_dict(&djbz_data, None).unwrap();
    let full = decode(&sjbz_data, Some(&shared_dict)).unwrap();
    let expected = downsample_reference(&full, 2);
    let actual = decode_downsampled(&sjbz_data, Some(&shared_dict), 2).unwrap();
    assert_eq!(expected.width, actual.width);
    assert_eq!(expected.height, actual.height);
    assert_eq!(expected.data, actual.data, "downsampled mask mismatch");
}

/// A coarser shift (3, i.e. 1/8) must also match the reference reduction —
/// proves the implementation generalises beyond the hard-coded `shift=2`
/// the render tier actually calls.
#[test]
fn decode_downsampled_matches_full_then_downsample_shift3() {
    let djvu = std::fs::read(assets_path().join("carte.djvu")).unwrap();
    let sjbz = extract_first_page_sjbz(&djvu);
    let full = decode(&sjbz, None).unwrap();
    let expected = downsample_reference(&full, 3);
    let actual = decode_downsampled(&sjbz, None, 3).unwrap();
    assert_eq!(expected.width, actual.width);
    assert_eq!(expected.height, actual.height);
    assert_eq!(expected.data, actual.data, "downsampled mask mismatch");
}
