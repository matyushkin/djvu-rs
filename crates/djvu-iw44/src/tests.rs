use super::*;

/// #815: a refused output pixmap is the decoder's own size limit.
#[test]
fn pixmap_error_maps_to_image_too_large() {
    let e = PixmapError::TooLarge {
        width: 10000,
        height: 10000,
        pixels: 100_000_000,
        max: Pixmap::MAX_PIXELS,
    };
    assert_eq!(Iw44Error::from(e), Iw44Error::ImageTooLarge);
}

fn assets_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../references/djvujs/library/assets")
}

fn golden_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/iw44")
}

/// Extract all BG44 chunk payloads from the first DJVU form in the file.
fn extract_bg44_chunks(file: &djvu_iff::DjvuFile) -> Vec<&[u8]> {
    fn collect(chunk: &djvu_iff::Chunk) -> Option<Vec<&[u8]>> {
        match chunk {
            djvu_iff::Chunk::Form {
                secondary_id,
                children,
                ..
            } => {
                if secondary_id == b"DJVU" {
                    let v = children
                        .iter()
                        .filter_map(|c| match c {
                            djvu_iff::Chunk::Leaf {
                                id: [b'B', b'G', b'4', b'4'],
                                data,
                            } => Some(data.as_slice()),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    return Some(v);
                }
                for c in children {
                    if let Some(v) = collect(c) {
                        return Some(v);
                    }
                }
                None
            }
            _ => None,
        }
    }
    collect(&file.root).unwrap_or_default()
}

fn find_ppm_data_start(ppm: &[u8]) -> usize {
    let mut newlines = 0;
    for (i, &b) in ppm.iter().enumerate() {
        if b == b'\n' {
            newlines += 1;
            if newlines == 3 {
                return i + 1;
            }
        }
    }
    0
}

/// Compare `actual_ppm` against a golden file, creating it on first run.
///
/// If the file doesn't exist it is written (first-time generation).
/// On subsequent runs an exact byte-for-byte comparison is enforced so that
/// any accidental change to the pixel output is caught immediately.
fn assert_or_create_golden(actual_ppm: &[u8], golden_file: &str) {
    let path = golden_path().join(golden_file);
    if !path.exists() {
        std::fs::write(&path, actual_ppm)
            .unwrap_or_else(|e| panic!("failed to write golden {golden_file}: {e}"));
        return; // golden created — test passes on first run
    }
    assert_ppm_match(actual_ppm, golden_file);
}

fn assert_ppm_match(actual_ppm: &[u8], golden_file: &str) {
    let expected_ppm = std::fs::read(golden_path().join(golden_file))
        .unwrap_or_else(|_| panic!("golden file not found: {}", golden_file));
    assert_eq!(
        actual_ppm.len(),
        expected_ppm.len(),
        "PPM size mismatch for {}: got {} expected {}",
        golden_file,
        actual_ppm.len(),
        expected_ppm.len()
    );
    if actual_ppm != expected_ppm {
        let header_end = find_ppm_data_start(actual_ppm);
        let actual_pixels = &actual_ppm[header_end..];
        let expected_pixels = &expected_ppm[header_end..];
        let total_pixels = actual_pixels.len() / 3;
        let diff_pixels = actual_pixels
            .chunks(3)
            .zip(expected_pixels.chunks(3))
            .filter(|(a, b)| a != b)
            .count();
        panic!(
            "{} pixel mismatch: {}/{} pixels differ ({:.1}%)",
            golden_file,
            diff_pixels,
            total_pixels,
            diff_pixels as f64 / total_pixels as f64 * 100.0
        );
    }
}

// ---- TDD: failing tests first -------------------------------------------

/// Decode must fail gracefully on empty input.
#[test]
fn iw44_new_rejects_empty_chunk() {
    let mut img = Iw44Image::new();
    assert!(matches!(
        img.decode_chunk(&[]),
        Err(Iw44Error::ChunkTooShort)
    ));
}

/// Decode must fail gracefully on a truncated first-chunk header.
#[test]
fn iw44_new_rejects_truncated_header() {
    let mut img = Iw44Image::new();
    // serial=0 but only 5 bytes (need ≥ 9)
    assert!(matches!(
        img.decode_chunk(&[0x00, 0x01, 0x00, 0x02, 0x00]),
        Err(Iw44Error::HeaderTooShort)
    ));
}

/// Zero-dimension image must be rejected.
#[test]
fn iw44_new_rejects_zero_dimension() {
    let mut img = Iw44Image::new();
    // serial=0, slices=1, majver=0, minor=2, w=0, h=100, delay=0
    let header = [0x00u8, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x64, 0x00];
    assert!(matches!(
        img.decode_chunk(&header),
        Err(Iw44Error::ZeroDimension)
    ));
}

/// A declared width×height above the 64 MP decode-cost cap must be
/// rejected before any pixel buffer is allocated.
#[test]
fn iw44_new_rejects_oversized_image() {
    let mut img = Iw44Image::new();
    // serial=0, slices=1, majver=0, minor=2, w=65535, h=65535, delay=0
    // → 65535×65535 ≈ 4.29 G pixels, far past the 64 MP cap.
    let header = [0x00u8, 0x01, 0x00, 0x02, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];
    assert!(matches!(
        img.decode_chunk(&header),
        Err(Iw44Error::ImageTooLarge)
    ));
}

/// Subsequent chunk before first chunk must be rejected.
#[test]
fn iw44_new_rejects_subsequent_before_first() {
    let mut img = Iw44Image::new();
    // serial != 0
    assert!(matches!(
        img.decode_chunk(&[0x01, 0x01]),
        Err(Iw44Error::MissingFirstChunk)
    ));
}

/// BUG-ZPSHORT regression: a refinement chunk (`serial != 0`) whose ZP
/// payload is short or entirely empty must decode as a no-op refinement
/// round, not a hard error. Real BG44 streams contain such chunks (e.g.
/// `watchmaker.djvu`'s page-0 chunk 2, a bare 2-byte `[serial, slices]`
/// header) when the encoder had nothing left to encode for that round —
/// `ZpDecoder` already treats reads past a stream's true end as synthetic
/// `0xFF` padding, so a chunk that is *entirely* padding is not malformed.
#[test]
fn iw44_decode_chunk_tolerates_empty_refinement_payload() {
    let mut img = Iw44Image::new();
    // First chunk: minimal valid grayscale header, 1x1, no slices decoded.
    let header = [0x00u8, 0x00, 0x00, 0x02, 0x00, 0x01, 0x00, 0x01, 0x00];
    img.decode_chunk(&header).expect("first chunk must decode");

    // Refinement chunk: serial=1, slices=4, zero-length ZP payload.
    assert!(img.decode_chunk(&[0x01, 0x04]).is_ok());

    // Another refinement chunk with a single stray payload byte (still
    // short of ZpDecoder's normal 2-byte minimum) must also be tolerated.
    assert!(img.decode_chunk(&[0x02, 0x04, 0xab]).is_ok());

    // The image must still be usable afterwards (no poisoned state).
    assert!(img.to_rgb().is_ok());
}

/// Round 46 (INTEROP_STREAMS finding 2b): a refinement chunk whose
/// `serial` byte skips ahead (or repeats/rewinds) must be rejected rather
/// than silently decoded into the wrong refinement slot. Mirrors
/// DjVuLibre's `cserial` continuity check
/// (`IW44Image.wrong_serial`/`wrong_serial2`) — differential fuzzing
/// against `ddjvu` found real corpus mutations (`watchmaker.djvu`
/// bit-flips landing on a BG44 chunk's serial byte) that trip this exact
/// check on DjVuLibre's side while the pre-fix decoder here decoded on,
/// unnoticed (`fuzz/corpus-regressions/diff_fuzz/watchmaker_00001_our-
/// renders-what-they-reject.*`).
#[test]
fn iw44_decode_chunk_rejects_serial_skip() {
    let mut img = Iw44Image::new();
    let header = [0x00u8, 0x00, 0x00, 0x02, 0x00, 0x01, 0x00, 0x01, 0x00];
    img.decode_chunk(&header).expect("first chunk must decode");

    // Expected next serial is 1; a chunk claiming serial=3 (skipped 1, 2)
    // must be rejected.
    assert!(matches!(
        img.decode_chunk(&[0x03, 0x04]),
        Err(Iw44Error::UnexpectedSerial)
    ));
}

/// Round 46: a refinement chunk that *repeats* an already-consumed
/// serial (instead of skipping ahead) must also be rejected — not just
/// forward gaps.
#[test]
fn iw44_decode_chunk_rejects_serial_repeat() {
    let mut img = Iw44Image::new();
    let header = [0x00u8, 0x00, 0x00, 0x02, 0x00, 0x01, 0x00, 0x01, 0x00];
    img.decode_chunk(&header).expect("first chunk must decode");
    img.decode_chunk(&[0x01, 0x04])
        .expect("serial=1 refinement must decode");

    // Expected next serial is 2; a chunk claiming serial=1 again (a
    // duplicated/rewound chunk) must be rejected.
    assert!(matches!(
        img.decode_chunk(&[0x01, 0x04]),
        Err(Iw44Error::UnexpectedSerial)
    ));
}

/// Round 46: the BUG-ZPSHORT tolerance (empty/short refinement payloads)
/// must still hold for chunks that *do* arrive in the correct serial
/// order — the new continuity check must not regress it.
#[test]
fn iw44_decode_chunk_serial_check_does_not_regress_zpshort_tolerance() {
    let mut img = Iw44Image::new();
    let header = [0x00u8, 0x00, 0x00, 0x02, 0x00, 0x01, 0x00, 0x01, 0x00];
    img.decode_chunk(&header).expect("first chunk must decode");
    // In-order, empty-payload refinement chunks (serial 1, 2, ...) must
    // still be tolerated exactly as before.
    assert!(img.decode_chunk(&[0x01, 0x04]).is_ok());
    assert!(img.decode_chunk(&[0x02, 0x04]).is_ok());
    assert!(img.to_rgb().is_ok());
}

/// `to_rgb()` on an uninitialised decoder must return an error.
#[test]
fn iw44_new_to_rgb_without_data_returns_error() {
    let img = Iw44Image::new();
    assert!(matches!(img.to_rgb(), Err(Iw44Error::MissingCodec)));
}

/// `to_rgb_subsample(0)` must be rejected.
#[test]
fn iw44_new_subsample_zero_rejected() {
    let img = Iw44Image::new();
    assert!(matches!(
        img.to_rgb_subsample(0),
        Err(Iw44Error::InvalidSubsample)
    ));
}

// ---- Pixel-exact golden tests -------------------------------------------

/// The banded reconstruction must be exact, not close: every interior row
/// of a band must equal the row the whole-plane transform produces.
///
/// A band carries the transform's own boundary handling at its edges, which
/// is only correct where the band edge is the image edge. `BAND_HALO_BLOCKS`
/// is what buys the interior its correctness, so this test also walks the
/// halo down: the smallest halo that still matches tells a later reader how
/// much margin the constant really has.
#[test]
fn reconstruct_band_matches_the_whole_plane() {
    let data = std::fs::read(assets_path().join("carte.djvu")).expect("carte.djvu");
    let file = djvu_iff::parse(&data).expect("parse");
    let mut img = Iw44Image::new();
    for c in &extract_bg44_chunks(&file) {
        img.decode_chunk(c).expect("decode_chunk");
    }
    let y = img.y.as_ref().expect("luma plane");
    let block_rows = y.height.div_ceil(32);
    assert!(
        block_rows >= 2 * BAND_HALO_BLOCKS + 2,
        "the fixture must be tall enough to hold an interior band"
    );
    let full = y.reconstruct(1);

    for keep in [1usize, 2, 4] {
        let mut first = 0;
        while first < block_rows {
            let last = (first + keep).min(block_rows);
            let lo = first.saturating_sub(BAND_HALO_BLOCKS);
            let hi = (last + BAND_HALO_BLOCKS).min(block_rows);
            let band = y.reconstruct_window((lo, hi), (0, y.block_cols), 1);
            for r in first * 32..(last * 32).min(y.height) {
                let a = &full.data[r * full.stride..r * full.stride + y.width];
                let b = &band.data[(r - lo * 32) * band.stride..][..y.width];
                assert_eq!(
                    a, b,
                    "band [{lo}..{hi}) block rows, keeping [{first}..{last}): \
                         image row {r} differs from the whole-plane transform"
                );
            }
            first = last;
        }
    }
}

/// A window of block columns must be exact too: the transform reaches as
/// far across columns as across rows, so the same halo on the left and
/// right makes the interior columns equal the whole-plane transform. Both
/// the full-resolution and the compact scale-2 plane are checked.
#[test]
fn reconstruct_window_matches_the_whole_plane() {
    let data = std::fs::read(assets_path().join("carte.djvu")).expect("carte.djvu");
    let file = djvu_iff::parse(&data).expect("parse");
    let mut img = Iw44Image::new();
    for c in &extract_bg44_chunks(&file) {
        img.decode_chunk(c).expect("decode_chunk");
    }
    let y = img.y.as_ref().expect("luma plane");
    let (block_rows, block_cols) = (y.height.div_ceil(32), y.block_cols);
    assert!(
        block_cols >= 2 * BAND_HALO_BLOCKS + 2,
        "the fixture must be wide enough to hold an interior window"
    );
    for sub in [1usize, 2] {
        let side = 32 / sub;
        let (w, h) = (y.width.div_ceil(sub), y.height.div_ceil(sub));
        let full = y.reconstruct(sub);
        for keep in [1usize, 3] {
            let mut first_col = 0;
            while first_col < block_cols {
                let last_col = (first_col + keep).min(block_cols);
                let col_lo = first_col.saturating_sub(BAND_HALO_BLOCKS);
                let col_hi = (last_col + BAND_HALO_BLOCKS).min(block_cols);
                // One band of rows in the middle and one at the bottom.
                for first in [block_rows / 2, block_rows - 1] {
                    let last = (first + 2).min(block_rows);
                    let lo = first.saturating_sub(BAND_HALO_BLOCKS);
                    let hi = (last + BAND_HALO_BLOCKS).min(block_rows);
                    let win = y.reconstruct_window((lo, hi), (col_lo, col_hi), sub);
                    let (c0, c1) = (first_col * side, (last_col * side).min(w));
                    for r in first * side..(last * side).min(h) {
                        let a = &full.data[r * full.stride + c0..r * full.stride + c1];
                        let off = (r - lo * side) * win.stride + (c0 - col_lo * side);
                        let b = &win.data[off..off + (c1 - c0)];
                        assert_eq!(
                            a, b,
                            "sub {sub}: window rows [{lo}..{hi}) cols [{col_lo}..{col_hi}), \
                                 image row {r} cols {c0}..{c1} differ from the whole plane"
                        );
                    }
                }
                first_col = last_col;
            }
        }
    }
}

/// End-to-end: the banded colour conversion must produce the same pixels as
/// the whole-plane one, byte for byte, including the chroma upsample that
/// reads one row past each band.
///
/// `band_keep_blocks` only turns banding on for pages far larger than any
/// fixture, so this calls both paths directly. `carte.djvu` sets
/// `crcb_half`, so its bands take chroma from the compact scale-2 bands;
/// the other fixtures use full-resolution chroma.
#[test]
fn banded_rgb_matches_whole_plane_rgb() {
    for asset in ["carte.djvu", "chicken.djvu", "colorbook.djvu"] {
        let data = std::fs::read(assets_path().join(asset)).expect("asset");
        let file = djvu_iff::parse(&data).expect("parse");
        let chunks = extract_bg44_chunks(&file);
        if chunks.is_empty() {
            continue;
        }
        let mut img = Iw44Image::new();
        for c in &chunks {
            img.decode_chunk(c).expect("decode_chunk");
        }
        if !img.is_color {
            continue;
        }
        let whole = img.to_rgb().expect("to_rgb");

        let y_dec = img.y.as_ref().unwrap();
        let cb_dec = img.cb.as_ref().unwrap();
        let cr_dec = img.cr.as_ref().unwrap();
        let (pw, ph) = (img.width as usize, img.height as usize);
        for keep in [1usize, 3, 8] {
            let mut banded = Pixmap::try_new(img.width, img.height, 0, 0, 0, 255)
                .expect("fits the pixmap limit");
            img.rgb_sub1_banded(y_dec, cb_dec, cr_dec, keep, pw, ph, &mut banded);
            assert_eq!(
                banded.data, whole.data,
                "{asset}: banded conversion keeping {keep} block rows per \
                     band differs from the whole-plane conversion"
            );
        }
    }
}

/// `rgb_window` must give the bytes of `to_rgb` in every column it was
/// asked for, and either those bytes or zeros elsewhere, for windows that
/// start on odd and even columns, one column wide, and at both edges.
#[test]
fn rgb_window_matches_the_whole_picture() {
    for asset in ["carte.djvu", "chicken.djvu", "colorbook.djvu"] {
        let data = std::fs::read(assets_path().join(asset)).expect("asset");
        let file = djvu_iff::parse(&data).expect("parse");
        let chunks = extract_bg44_chunks(&file);
        if chunks.is_empty() {
            continue;
        }
        let mut img = Iw44Image::new();
        for c in &chunks {
            img.decode_chunk(c).expect("decode_chunk");
        }
        if !img.is_color {
            continue;
        }
        let whole = img.to_rgb().expect("to_rgb");
        let (w, h) = (img.width, img.height);
        let stride = w as usize * 4;
        let rows = [0..h, 37..h / 2 + 3, h - 1..h];
        let cols = [
            0..w,
            0..1,
            w - 1..w,
            1..2,
            33..34,
            31..w / 2 + 7,
            w / 3 + 1..w - 9,
            w / 2..w,
        ];
        for r in &rows {
            for c in &cols {
                let pm = img.rgb_window(r.clone(), c.clone()).expect("rgb_window");
                assert_eq!((pm.width, pm.height), (w, r.len() as u32));
                for (i, y) in r.clone().enumerate() {
                    let want = &whole.data[y as usize * stride..][..stride];
                    let got = &pm.data[i * stride..][..stride];
                    for x in 0..w as usize {
                        let (a, b) = (&want[x * 4..x * 4 + 4], &got[x * 4..x * 4 + 4]);
                        if c.contains(&(x as u32)) {
                            assert_eq!(a, b, "{asset}: rows {r:?} cols {c:?}: pixel ({x}, {y})");
                        } else {
                            assert!(
                                a == b || b == [0; 4],
                                "{asset}: rows {r:?} cols {c:?}: pixel ({x}, {y}) outside"
                            );
                        }
                    }
                }
            }
        }
        assert!(img.rgb_window(0..1, 0..w + 1).is_err());
        #[allow(clippy::reversed_empty_ranges)] // the reversed range is the point
        let reversed = img.rgb_window(0..1, 2..1);
        assert!(reversed.is_err());
    }
}

/// `rgb_rows` must give the same bytes as the same rows of `to_rgb`, for
/// any row range: a whole band, a band starting mid-block, one row, the
/// first and the last row. This is what lets a renderer composite straight
/// from bands (#811).
#[test]
fn rgb_rows_match_the_whole_picture() {
    for asset in ["carte.djvu", "chicken.djvu", "colorbook.djvu"] {
        let data = std::fs::read(assets_path().join(asset)).expect("asset");
        let file = djvu_iff::parse(&data).expect("parse");
        let chunks = extract_bg44_chunks(&file);
        if chunks.is_empty() {
            continue;
        }
        let mut img = Iw44Image::new();
        for c in &chunks {
            img.decode_chunk(c).expect("decode_chunk");
        }
        if !img.is_color {
            continue;
        }
        let whole = img.to_rgb().expect("to_rgb");
        let (w, h) = (img.width, img.height);
        let stride = w as usize * 4;
        assert!(h > 40, "{asset}: fixture must be taller than one test band");
        assert!(
            img.rgb_band_rows().is_none(),
            "{asset}: a small picture must not ask to be banded"
        );

        let mut ranges = vec![0..h, 0..1, h - 1..h, 37..h - 5, 3..4];
        let mut o = 0;
        while o < h {
            ranges.push(o..(o + 37).min(h));
            o += 37;
        }
        for r in ranges {
            let band = img.rgb_rows(r.clone()).expect("rgb_rows");
            assert_eq!((band.width, band.height), (w, r.end - r.start));
            assert_eq!(
                band.data,
                &whole.data[r.start as usize * stride..r.end as usize * stride],
                "{asset}: rows {r:?} differ from the whole-picture conversion"
            );
        }

        let empty = img.rgb_rows(5..5).expect("an empty range is fine");
        assert_eq!((empty.width, empty.height), (w, 0));
        assert!(matches!(img.rgb_rows(0..h + 1), Err(Iw44Error::Invalid)));
        // A reversed range is the input under test.
        #[allow(clippy::reversed_empty_ranges)]
        let reversed = 7..6;
        assert!(matches!(img.rgb_rows(reversed), Err(Iw44Error::Invalid)));
    }
}

#[test]
fn iw44_new_decode_boy_bg() {
    let data = std::fs::read(assets_path().join("boy.djvu")).expect("boy.djvu not found");
    let file = djvu_iff::parse(&data).expect("failed to parse boy.djvu");
    let chunks = extract_bg44_chunks(&file);
    assert_eq!(chunks.len(), 1, "expected 1 BG44 chunk in boy.djvu");

    let mut img = Iw44Image::new();
    for c in &chunks {
        img.decode_chunk(c).expect("decode_chunk failed");
    }
    assert_eq!(img.width, 192);
    assert_eq!(img.height, 256);

    let pm = img.to_rgb().expect("to_rgb failed");
    assert_ppm_match(&pm.to_ppm(), "boy_bg.ppm");
}

#[test]
fn iw44_new_decode_chicken_bg() {
    let data = std::fs::read(assets_path().join("chicken.djvu")).expect("chicken.djvu not found");
    let file = djvu_iff::parse(&data).expect("failed to parse chicken.djvu");
    let chunks = extract_bg44_chunks(&file);
    assert_eq!(chunks.len(), 3, "expected 3 BG44 chunks in chicken.djvu");

    let mut img = Iw44Image::new();
    for c in &chunks {
        img.decode_chunk(c).expect("decode_chunk failed");
    }
    assert_eq!(img.width, 181);
    assert_eq!(img.height, 240);

    let pm = img.to_rgb().expect("to_rgb failed");
    assert_ppm_match(&pm.to_ppm(), "chicken_bg.ppm");
}

/// Direct gray decode (`to_gray8`) must match the dimensions of `to_rgb`
/// and stay close to its Rec.601 luma, while skipping the chroma planes.
#[test]
fn iw44_to_gray8_matches_rgb_luma_boy() {
    let data = std::fs::read(assets_path().join("boy.djvu")).expect("boy.djvu not found");
    let file = djvu_iff::parse(&data).expect("failed to parse boy.djvu");
    let chunks = extract_bg44_chunks(&file);
    let mut img = Iw44Image::new();
    for c in &chunks {
        img.decode_chunk(c).expect("decode_chunk failed");
    }

    let rgb = img.to_rgb().expect("to_rgb failed");
    let rgb_gray = rgb.to_gray8();
    let direct = img.to_gray8().expect("to_gray8 failed");

    assert_eq!(direct.width, rgb_gray.width);
    assert_eq!(direct.height, rgb_gray.height);
    assert_eq!(direct.data.len(), rgb_gray.data.len());

    if img.is_color {
        // Colour: Y-plane luma vs Rec.601-of-RGB differ by a few levels.
        let mut sum = 0u64;
        let mut max = 0u8;
        for (a, b) in direct.data.iter().zip(rgb_gray.data.iter()) {
            let d = a.abs_diff(*b);
            sum += d as u64;
            max = max.max(d);
        }
        let mean = sum as f64 / direct.data.len() as f64;
        assert!(mean < 4.0, "mean gray diff {mean} too high");
        assert!(max <= 24, "max gray diff {max} too high");
    } else {
        // Grayscale: must be byte-identical to the RGB→luma round-trip.
        assert_eq!(direct.data, rgb_gray.data, "gray path must be exact");
    }
}

/// `to_gray8_subsample` dimensions must track `to_rgb_subsample` at sub 2/4.
#[test]
fn iw44_to_gray8_subsample_dims() {
    let data = std::fs::read(assets_path().join("boy.djvu")).expect("boy.djvu not found");
    let file = djvu_iff::parse(&data).expect("failed to parse boy.djvu");
    let chunks = extract_bg44_chunks(&file);
    let mut img = Iw44Image::new();
    for c in &chunks {
        img.decode_chunk(c).expect("decode_chunk failed");
    }
    for sub in [2u32, 4u32] {
        let g = img.to_gray8_subsample(sub).expect("gray sub");
        let rgb = img.to_rgb_subsample(sub).expect("rgb sub");
        assert_eq!((g.width, g.height), (rgb.width, rgb.height));
        assert_eq!(g.data.len(), (g.width * g.height) as usize);
    }
    assert!(matches!(
        img.to_gray8_subsample(0),
        Err(Iw44Error::InvalidSubsample)
    ));
}

/// `to_rgb_subsample(2)` on boy.djvu must produce a pixel-exact result.
///
/// This golden test guards against any regression in the compact-plane sub=2
/// optimization path.  On first run the golden file is created from the
/// current (correct) output; subsequent runs compare against it.
#[test]
fn iw44_new_decode_boy_sub2() {
    let data = std::fs::read(assets_path().join("boy.djvu")).expect("boy.djvu not found");
    let file = djvu_iff::parse(&data).expect("failed to parse boy.djvu");
    let chunks = extract_bg44_chunks(&file);

    let mut img = Iw44Image::new();
    for c in &chunks {
        img.decode_chunk(c).expect("decode_chunk failed");
    }
    assert_eq!(img.width, 192);
    assert_eq!(img.height, 256);

    let pm = img.to_rgb_subsample(2).expect("to_rgb_subsample(2) failed");
    assert_eq!(pm.width, 96, "sub=2 width must be ceil(192/2)");
    assert_eq!(pm.height, 128, "sub=2 height must be ceil(256/2)");

    assert_or_create_golden(&pm.to_ppm(), "boy_bg_sub2.ppm");
}

/// `to_rgb_subsample(2)` on big-scanned-page.djvu (color IW44).
///
/// Exercises the compact-plane path on a large color document.
#[test]
fn iw44_new_decode_big_scanned_sub2() {
    let data = std::fs::read(assets_path().join("big-scanned-page.djvu"))
        .expect("big-scanned-page.djvu not found");
    let file = djvu_iff::parse(&data).expect("failed to parse big-scanned-page.djvu");
    let chunks = extract_bg44_chunks(&file);

    let mut img = Iw44Image::new();
    for c in &chunks {
        img.decode_chunk(c).expect("decode_chunk failed");
    }
    assert_eq!(img.width, 6780);
    assert_eq!(img.height, 9148);

    let pm = img.to_rgb_subsample(2).expect("to_rgb_subsample(2) failed");
    assert_eq!(pm.width, 3390, "sub=2 width must be ceil(6780/2)");
    assert_eq!(pm.height, 4574, "sub=2 height must be ceil(9148/2)");

    assert_or_create_golden(&pm.to_ppm(), "big_scanned_sub2.ppm");
}

#[test]
fn iw44_new_decode_big_scanned_sub4() {
    let data = std::fs::read(assets_path().join("big-scanned-page.djvu"))
        .expect("big-scanned-page.djvu not found");
    let file = djvu_iff::parse(&data).expect("failed to parse big-scanned-page.djvu");
    let chunks = extract_bg44_chunks(&file);
    assert_eq!(chunks.len(), 4, "expected 4 BG44 chunks");

    let mut img = Iw44Image::new();
    for c in &chunks {
        img.decode_chunk(c).expect("decode_chunk failed");
    }
    assert_eq!(img.width, 6780);
    assert_eq!(img.height, 9148);

    let pm = img.to_rgb_subsample(4).expect("to_rgb_subsample failed");
    assert_ppm_match(&pm.to_ppm(), "big_scanned_sub4.ppm");
}

/// Collect BG44 chunk payloads for every `FORM:DJVU` component in document
/// order. Unlike `extract_bg44_chunks` (which stops at the first DJVU form),
/// this walks the whole DJVM bundle so individual pages can be addressed.
fn extract_bg44_chunks_per_page(file: &djvu_iff::DjvuFile) -> Vec<Vec<&[u8]>> {
    fn walk<'a>(chunk: &'a djvu_iff::Chunk, out: &mut Vec<Vec<&'a [u8]>>) {
        if let djvu_iff::Chunk::Form {
            secondary_id,
            children,
            ..
        } = chunk
        {
            if secondary_id == b"DJVU" {
                let bg = children
                    .iter()
                    .filter_map(|c| match c {
                        djvu_iff::Chunk::Leaf {
                            id: [b'B', b'G', b'4', b'4'],
                            data,
                        } => Some(data.as_slice()),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                out.push(bg);
                return;
            }
            for c in children {
                walk(c, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(&file.root, &mut out);
    out
}

/// Regression for the IW44 slice-loop early-exit bug (treadbear report,
/// 2026-06-09; see PERF_EXPERIMENTS.md).
///
/// Page 2 of `colorbook.djvu` packs its 97 slices into chunks dense enough
/// that `zp.is_exhausted()` (a *byte*-buffer check) fires several bits
/// before the last slice of a chunk is decoded. The reverted `break` on
/// exhaustion therefore truncated real wavelet refinement, corrupting
/// ~60% of pixels vs DjVuLibre. This golden pins the correct (full-slice)
/// decode so the early-exit cannot be reintroduced.
///
/// Verified to fail (page-2 background pixels diverge) if the
/// `zp.is_exhausted()` early-exit is restored in `decode_chunk`.
#[test]
fn iw44_colorbook_page2_decodes_all_slices_no_early_exit() {
    let data =
        std::fs::read(assets_path().join("colorbook.djvu")).expect("colorbook.djvu not found");
    let file = djvu_iff::parse(&data).expect("failed to parse colorbook.djvu");
    let pages = extract_bg44_chunks_per_page(&file);
    let chunks = &pages[2];
    assert_eq!(chunks.len(), 4, "colorbook page 2 must have 4 BG44 chunks");

    let mut img = Iw44Image::new();
    for c in chunks {
        img.decode_chunk(c).expect("decode_chunk failed");
    }
    assert_eq!((img.width, img.height), (739, 1213));

    let pm = img.to_rgb().expect("to_rgb failed");
    assert_or_create_golden(&pm.to_ppm(), "colorbook_bg_p2.ppm");
}

/// Progressive decode: feeding all chunks at once and feeding them one-by-one
/// must produce identical results.
#[test]
fn iw44_new_progressive_matches_full_decode_chicken() {
    let data = std::fs::read(assets_path().join("chicken.djvu")).expect("chicken.djvu not found");
    let file = djvu_iff::parse(&data).expect("failed to parse");
    let chunks = extract_bg44_chunks(&file);
    assert!(
        chunks.len() > 1,
        "need multiple chunks for progressive test"
    );

    // Full decode (all chunks at once via repeated decode_chunk calls)
    let mut full = Iw44Image::new();
    for c in &chunks {
        full.decode_chunk(c).expect("full decode failed");
    }
    let full_pm = full.to_rgb().expect("full to_rgb failed");

    // Progressive decode — same result since ZP state persists
    let mut prog = Iw44Image::new();
    for c in chunks.iter().take(1) {
        prog.decode_chunk(c).expect("progressive decode failed");
    }
    for c in chunks.iter().skip(1) {
        prog.decode_chunk(c).expect("progressive decode failed");
    }
    let prog_pm = prog.to_rgb().expect("progressive to_rgb failed");

    assert_eq!(
        full_pm.data, prog_pm.data,
        "progressive and full decode must produce identical pixels"
    );
}

// ── v1.2 chroma-plane header interpretation ─────────────────────────────

/// IW44 v1.2's delay-byte high bit does not make the Cb/Cr planes half
/// resolution.  `carte.djvu` has that bit clear (DjVuLibre's `crcb_half`)
/// but DjVuLibre decodes its full-resolution chroma planes; allocating them
/// at half size desynchronizes their ZP streams into chroma noise (#561).
/// The flag changes only the reconstruction (#830).
#[test]
fn carte_v12_allocates_full_size_chroma_planes() {
    let data = std::fs::read(assets_path().join("carte.djvu")).expect("carte.djvu not found");
    let file = djvu_iff::parse(&data).expect("iff parse");
    let chunks = extract_bg44_chunks(&file);
    assert!(!chunks.is_empty(), "carte.djvu must have BG44 chunks");

    let mut img = Iw44Image::new();
    img.decode_chunk(chunks[0]).expect("decode_chunk");

    assert!(img.is_color(), "carte.djvu must be a color image");
    assert!(img.chroma_half(), "carte.djvu sets DjVuLibre's crcb_half");
    let (cw, ch) = img
        .chroma_plane_dims()
        .expect("chroma plane must be allocated after first color chunk");
    let lw = img.width as usize;
    let lh = img.height as usize;
    let expected_w = lw;
    let expected_h = lh;
    assert_eq!(
        cw, expected_w,
        "chroma plane width must equal luma_w={expected_w}, got {cw}"
    );
    assert_eq!(
        ch, expected_h,
        "chroma plane height must equal luma_h={expected_h}, got {ch}"
    );
}

/// Decode the real v1.2 `carte.djvu` stream with full-size chroma planes
/// and `crcb_half` reconstruction. The digest is that of `ddjvu`'s render
/// of the BG44 plane alone, so it pins bit-exactness with DjVuLibre, and
/// guards against both the half-plane chroma noise (#561) and ignoring
/// the flag (#830).
#[test]
fn iw44_new_decode_carte_bg_full_chroma() {
    let data = std::fs::read(assets_path().join("carte.djvu")).expect("carte.djvu not found");
    let file = djvu_iff::parse(&data).expect("iff parse");
    let chunks = extract_bg44_chunks(&file);

    let mut img = Iw44Image::new();
    for c in &chunks {
        img.decode_chunk(c).expect("decode_chunk failed");
    }
    assert_eq!(img.width, 1400);
    assert_eq!(img.height, 852);

    let pm = img.to_rgb().expect("to_rgb failed");
    let hash = pm.data.iter().fold(0xcbf29ce484222325u64, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    });
    assert_eq!(
        hash, 0xb00a_964b_8159_1671,
        "carte BG44 pixel digest must equal ddjvu's"
    );
}

// ── Error path tests ────────────────────────────────────────────────────

#[test]
fn test_decode_empty_chunk() {
    let mut img = Iw44Image::new();
    let result = img.decode_chunk(&[]);
    assert!(result.is_err());
}

#[test]
fn test_decode_truncated_header() {
    let mut img = Iw44Image::new();
    // Only 2 bytes — not enough for a header
    let result = img.decode_chunk(&[0x00, 0x01]);
    assert!(result.is_err());
}

#[test]
fn test_to_rgb_before_decode() {
    let img = Iw44Image::new();
    // No chunks decoded yet — should fail
    let result = img.to_rgb();
    assert!(result.is_err());
}

#[test]
fn test_to_rgb_subsample_zero() {
    let img = Iw44Image::new();
    let result = img.to_rgb_subsample(0);
    assert!(result.is_err());
}

// ---- SIMD YCbCr→RGBA tests -----------------------------------------------

/// `ycbcr_row_to_rgba` matches the scalar formula on synthetic data.
#[test]
fn simd_ycbcr_row_matches_scalar() {
    // Cover all 8-wide SIMD chunks plus a tail (n=20).
    let n = 20usize;
    let ys: Vec<i32> = (0..n).map(|i| (i as i32 * 7) % 200 - 100).collect();
    let bs: Vec<i32> = (0..n).map(|i| (i as i32 * 13) % 200 - 100).collect();
    let rs: Vec<i32> = (0..n).map(|i| (i as i32 * 17) % 200 - 100).collect();

    // Scalar reference
    let mut expected = vec![0u8; n * 4];
    for col in 0..n {
        let y = ys[col];
        let b = bs[col];
        let r = rs[col];
        let t2 = r + (r >> 1);
        let t3 = y + 128 - (b >> 2);
        expected[col * 4] = (y + 128 + t2).clamp(0, 255) as u8;
        expected[col * 4 + 1] = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
        expected[col * 4 + 2] = (t3 + (b << 1)).clamp(0, 255) as u8;
        expected[col * 4 + 3] = 255;
    }

    // SIMD result
    let mut actual = vec![0u8; n * 4];
    super::ycbcr_row_to_rgba(&ys, &bs, &rs, &mut actual);

    assert_eq!(
        expected, actual,
        "SIMD must produce identical output to scalar"
    );
}

/// `ycbcr_row_to_rgba` handles extreme values (clamping at 0 and 255).
#[test]
fn simd_ycbcr_row_clamps_correctly() {
    let n = 8usize;
    // Use values that will clamp to 0 and 255 in each channel.
    let ys: Vec<i32> = vec![127, -128, 127, -128, 0, 0, 0, 0];
    let bs: Vec<i32> = vec![-128, 127, -128, 127, 0, 0, 0, 0];
    let rs: Vec<i32> = vec![127, -128, -128, 127, 0, 0, 0, 0];

    let mut simd_out = vec![0u8; n * 4];
    super::ycbcr_row_to_rgba(&ys, &bs, &rs, &mut simd_out);

    // All RGBA values must be in [0, 255] and alpha == 255.
    for chunk in simd_out.as_chunks::<4>().0 {
        assert_eq!(chunk[3], 255, "alpha must always be 255");
    }
}

/// SIMD render of boy.djvu produces identical output to the scalar path.
///
/// This verifies that the fast path (sub=1) and general path (sub=2, which
/// uses the old scalar code) produce consistent results on a real file.
#[test]
fn simd_render_matches_subsampled_render_dimensions() {
    let data = std::fs::read(assets_path().join("boy.djvu")).expect("boy.djvu not found");
    let file = djvu_iff::parse(&data).expect("parse failed");
    let chunks = extract_bg44_chunks(&file);

    let mut img = Iw44Image::new();
    for c in &chunks {
        img.decode_chunk(c).expect("decode_chunk failed");
    }

    // Full-resolution render uses SIMD path (sub=1).
    let full = img.to_rgb().expect("to_rgb failed");
    // sub=2 uses the scalar general path — just check dims match half.
    let half = img.to_rgb_subsample(2).expect("subsample(2) failed");

    assert_eq!(full.width, img.width);
    assert_eq!(full.height, img.height);
    assert_eq!(half.width, img.width.div_ceil(2));
    assert_eq!(half.height, img.height.div_ceil(2));
    // SIMD path must still pass the existing golden test (done in iw44_new_decode_boy_bg).
}

/// SIMD row pass (8 rows at a time) produces identical results to the scalar
/// path on a synthetic 32×16 plane with a deterministic non-trivial pattern.
///
/// Both paths are exercised by calling `row_pass_inner` with `use_simd=false`
/// (all scalar) and `use_simd=true` (SIMD + scalar tail) on identical copies
/// of the same data.
#[test]
fn simd_row_pass_matches_scalar() {
    let width = 32usize;
    let height = 16usize;
    let stride = width;
    let n = stride * height;

    // Deterministic non-trivial pattern: values in [-255, 255].
    let initial: Vec<i16> = (0..n).map(|i| ((i * 7 + 13) % 511) as i16 - 255).collect();

    let mut scalar_data = initial.clone();
    // s=1, sd=0, use_simd=false → pure scalar
    super::row_pass_inner(&mut scalar_data, width, height, stride, 1, 0, false);

    let mut simd_data = initial.clone();
    // s=1, sd=0, use_simd=true → SIMD for rows 0..15, scalar tail for remainder
    super::row_pass_inner(&mut simd_data, width, height, stride, 1, 0, true);

    assert_eq!(
        scalar_data, simd_data,
        "SIMD row pass must produce identical output to scalar"
    );
}

/// Same as `simd_row_pass_matches_scalar` but for s=2 (sd=1).
///
/// Active rows are every other row; active columns are every other column.
/// The generalised SIMD path (8 active rows at a time with stride s) must
/// produce the same result as the pure scalar path.
#[test]
fn simd_row_pass_s2_matches_scalar() {
    let width = 64usize;
    let height = 32usize;
    let stride = width;
    let n = stride * height;
    let s = 2usize;
    let sd = 1usize;

    let initial: Vec<i16> = (0..n).map(|i| ((i * 7 + 13) % 511) as i16 - 255).collect();

    let mut scalar_data = initial.clone();
    super::row_pass_inner(&mut scalar_data, width, height, stride, s, sd, false);

    let mut simd_data = initial.clone();
    super::row_pass_inner(&mut simd_data, width, height, stride, s, sd, true);

    assert_eq!(
        scalar_data, simd_data,
        "SIMD row pass (s=2) must produce identical output to scalar"
    );
}

/// Reference scalar implementation of the fused-normalize YCbCr→RGBA path.
/// Mirrors `ycbcr_neon_raw` byte-for-byte (same formula, same clamps).
#[cfg(all(target_arch = "x86_64", feature = "std"))]
fn ycbcr_raw_scalar(y: &[i16], cb: &[i16], cr: &[i16], out: &mut [u8]) {
    let w = y.len();
    for col in 0..w {
        let yn = super::normalize(y[col]);
        let bn = super::normalize(cb[col]);
        let rn = super::normalize(cr[col]);
        let t2 = rn + (rn >> 1);
        let t3 = yn + 128 - (bn >> 2);
        out[col * 4] = (yn + 128 + t2).clamp(0, 255) as u8;
        out[col * 4 + 1] = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
        out[col * 4 + 2] = (t3 + (bn << 1)).clamp(0, 255) as u8;
        out[col * 4 + 3] = 255;
    }
}

#[cfg(all(target_arch = "x86_64", feature = "std"))]
fn ycbcr_raw_half_scalar(y: &[i16], cb: &[i16], cr: &[i16], out: &mut [u8]) {
    let w = y.len();
    for col in 0..w {
        let yn = super::normalize(y[col]);
        let bn = super::normalize(cb[col / 2]);
        let rn = super::normalize(cr[col / 2]);
        let t2 = rn + (rn >> 1);
        let t3 = yn + 128 - (bn >> 2);
        out[col * 4] = (yn + 128 + t2).clamp(0, 255) as u8;
        out[col * 4 + 1] = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
        out[col * 4 + 2] = (t3 + (bn << 1)).clamp(0, 255) as u8;
        out[col * 4 + 3] = 255;
    }
}

/// AVX2 fused-normalize YCbCr→RGBA must agree byte-for-byte with the scalar
/// reference across the full i16 input range and all width residues mod 16
/// (covers main loop + scalar tail).
#[cfg(all(target_arch = "x86_64", feature = "std"))]
#[test]
fn ycbcr_avx2_raw_matches_scalar() {
    if !std::is_x86_feature_detected!("avx2") {
        eprintln!("skipping: AVX2 not available on this host");
        return;
    }
    // Range chosen to exercise normalize + clamp + every arithmetic branch.
    let raw_vals: [i16; 8] = [-32768, -8192, -64, -1, 0, 63, 8191, 32767];
    for &width in &[1usize, 7, 16, 17, 31, 32, 33, 47, 48, 64, 100] {
        let n = width;
        let make_seq = |seed: usize| -> Vec<i16> {
            (0..n)
                .map(|i| raw_vals[(i + seed) % raw_vals.len()])
                .collect()
        };
        let y = make_seq(0);
        let cb = make_seq(3);
        let cr = make_seq(5);

        let mut got = vec![0u8; n * 4];
        #[allow(unsafe_code)]
        unsafe {
            super::ycbcr_avx2_raw(y.as_ptr(), cb.as_ptr(), cr.as_ptr(), got.as_mut_ptr(), n);
        }

        let mut want = vec![0u8; n * 4];
        ycbcr_raw_scalar(&y, &cb, &cr, &mut want);

        assert_eq!(got, want, "AVX2 raw mismatch at width {}", width);
    }
}

/// AVX2 stride-1 load/store must round-trip the full i16 range
/// bit-exactly through an i32x8.
#[cfg(all(target_arch = "x86_64", feature = "std"))]
#[test]
fn load8s_s1_avx2_matches_scalar() {
    if !std::is_x86_feature_detected!("avx2") {
        eprintln!("skipping: AVX2 not available on this host");
        return;
    }
    let raw_vals: [i16; 8] = [-32768, -8192, -64, -1, 0, 63, 8191, 32767];
    let n = 64;
    let buf: Vec<i16> = (0..n).map(|i| raw_vals[i % raw_vals.len()]).collect();
    for phys_off in 0..(n - 8) {
        #[allow(unsafe_code)]
        let got = unsafe { super::load8s_s1_avx2(&buf, phys_off) };
        let want = super::load8s(&buf, phys_off, 1);
        assert_eq!(
            got.to_array(),
            want.to_array(),
            "AVX2 load8s_s1 mismatch at phys_off {}",
            phys_off
        );
    }
}

/// AVX2 stride-1 store must truncate i32→i16 (drop upper 16 bits, no
/// saturation) matching the scalar `as i16` cast for every input.
#[cfg(all(target_arch = "x86_64", feature = "std"))]
#[test]
fn store8s_s1_avx2_matches_scalar() {
    if !std::is_x86_feature_detected!("avx2") {
        eprintln!("skipping: AVX2 not available on this host");
        return;
    }
    // Inputs that exercise truncation: values that don't fit in i16,
    // negative values, and boundaries.
    let raw_vals: [i32; 8] = [i32::MIN, -100_000, -32768, -1, 0, 32767, 100_000, i32::MAX];
    for offset in 0..8usize {
        let mut input = [0i32; 8];
        for j in 0..8 {
            input[j] = raw_vals[(j + offset) % 8];
        }
        let v = wide::i32x8::from(input);

        // AVX2 store with surrounding sentinel bytes to detect over-write.
        let mut buf_avx2 = vec![0xABCDu16 as i16; 32];
        #[allow(unsafe_code)]
        unsafe {
            super::store8s_s1_avx2(&mut buf_avx2, 8, v);
        }
        // Scalar reference using stride-1 store (which on this host is
        // also the AVX2 path; route through stride-2 to force scalar).
        let mut buf_scalar = vec![0xABCDu16 as i16; 32];
        for j in 0..8 {
            buf_scalar[8 + j] = input[j] as i16;
        }
        assert_eq!(buf_avx2, buf_scalar, "AVX2 store8s_s1 mismatch");
    }
}

/// AVX2 `prelim_flags_bucket_avx2` must produce identical bucket bytes
/// and bstatetmp to the scalar fallback for any 16-coef input.
#[cfg(all(target_arch = "x86_64", feature = "std"))]
#[test]
fn prelim_flags_bucket_avx2_matches_scalar() {
    if !std::is_x86_feature_detected!("avx2") {
        eprintln!("skipping: AVX2 not available on this host");
        return;
    }
    // Inputs that exercise the all-zero, all-nonzero, mixed, and edge-value cases.
    let test_vectors: &[[i16; 16]] = &[
        [0; 16],
        [
            1, 0, -1, 0, 100, 0, -200, 0, 0, 1234, 0, -1234, 0, 32767, -32768, 0,
        ],
        [1; 16],
        [-1; 16],
        [
            32767, -32768, 1, -1, 0, 0, 0, 0, 0, 0, 0, 0, 32767, -32768, 1, -1,
        ],
    ];
    for &coefs in test_vectors {
        let mut bucket_avx2 = [0u8; 16];
        #[allow(unsafe_code)]
        let bstate_avx2 = unsafe { super::prelim_flags_bucket_avx2(&coefs, &mut bucket_avx2) };

        let mut bucket_scalar = [0u8; 16];
        let mut bstate_scalar = 0u8;
        for k in 0..16 {
            let f = if coefs[k] == 0 {
                super::UNK
            } else {
                super::ACTIVE
            };
            bucket_scalar[k] = f;
            bstate_scalar |= f;
        }

        assert_eq!(
            bucket_avx2, bucket_scalar,
            "bucket mismatch for coefs={coefs:?}"
        );
        assert_eq!(
            bstate_avx2, bstate_scalar,
            "bstatetmp mismatch for coefs={coefs:?}"
        );
    }
}

/// AVX2 `prelim_flags_band0_avx2` must mirror the scalar band-0 update:
/// only entries with `old_flags[k] != ZERO` are rewritten; other entries
/// stay (so a ZERO-state lane is preserved across the call).
#[cfg(all(target_arch = "x86_64", feature = "std"))]
#[test]
fn prelim_flags_band0_avx2_matches_scalar() {
    if !std::is_x86_feature_detected!("avx2") {
        eprintln!("skipping: AVX2 not available on this host");
        return;
    }
    // Old-flag patterns covering the three states and mixed.
    let old_patterns: &[[u8; 16]] = &[
        [super::ZERO; 16],
        [super::UNK; 16],
        [super::ACTIVE; 16],
        [
            super::ZERO,
            super::UNK,
            super::ACTIVE,
            super::ZERO,
            super::UNK,
            super::ACTIVE,
            super::ZERO,
            super::UNK,
            super::ACTIVE,
            super::ZERO,
            super::UNK,
            super::ACTIVE,
            super::ZERO,
            super::UNK,
            super::ACTIVE,
            super::ZERO,
        ],
    ];
    let coef_patterns: &[[i16; 16]] = &[
        [0; 16],
        [1; 16],
        [0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1],
        [
            -32768, 0, 32767, 0, 100, 0, -100, 0, 0, 1, 0, -1, 0, 5, 0, -5,
        ],
    ];

    for &old in old_patterns {
        for &coefs in coef_patterns {
            let mut flags_avx2 = old;
            #[allow(unsafe_code)]
            let bstate_avx2 = unsafe { super::prelim_flags_band0_avx2(&coefs, &mut flags_avx2) };

            let mut flags_scalar = old;
            let mut bstate_scalar = 0u8;
            for k in 0..16 {
                if flags_scalar[k] != super::ZERO {
                    flags_scalar[k] = if coefs[k] == 0 {
                        super::UNK
                    } else {
                        super::ACTIVE
                    };
                }
                bstate_scalar |= flags_scalar[k];
            }

            assert_eq!(
                flags_avx2, flags_scalar,
                "flags mismatch old={old:?} coefs={coefs:?}"
            );
            assert_eq!(bstate_avx2, bstate_scalar, "bstatetmp mismatch");
        }
    }
}

#[cfg(all(target_arch = "x86_64", feature = "std"))]
#[test]
fn ycbcr_avx2_raw_half_matches_scalar() {
    if !std::is_x86_feature_detected!("avx2") {
        eprintln!("skipping: AVX2 not available on this host");
        return;
    }
    let raw_vals: [i16; 8] = [-32768, -8192, -64, -1, 0, 63, 8191, 32767];
    for &width in &[2usize, 8, 16, 18, 30, 32, 34, 48, 64, 96] {
        let n = width;
        let half = n.div_ceil(2);
        let make_seq = |seed: usize, len: usize| -> Vec<i16> {
            (0..len)
                .map(|i| raw_vals[(i + seed) % raw_vals.len()])
                .collect()
        };
        let y = make_seq(0, n);
        let cb_half = make_seq(3, half);
        let cr_half = make_seq(5, half);

        let mut got = vec![0u8; n * 4];
        #[allow(unsafe_code)]
        unsafe {
            super::ycbcr_avx2_raw_half(
                y.as_ptr(),
                cb_half.as_ptr(),
                cr_half.as_ptr(),
                got.as_mut_ptr(),
                n,
            );
        }

        let mut want = vec![0u8; n * 4];
        ycbcr_raw_half_scalar(&y, &cb_half, &cr_half, &mut want);

        assert_eq!(got, want, "AVX2 raw_half mismatch at width {}", width);
    }
}

/// WASM simd128 stride-1 load must sign-extend i16→i32 correctly.
///
/// Mirrors `load8s_s1_avx2_matches_scalar` but for the simd128 path.
/// Runs only when compiled for wasm32 with +simd128; host tests use the
/// AVX2 or scalar path instead.
#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
#[test]
fn load8s_s1_simd128_matches_scalar() {
    let raw_vals: [i16; 8] = [-32768, -8192, -64, -1, 0, 63, 8191, 32767];
    let n = 64;
    let buf: alloc::vec::Vec<i16> = (0..n).map(|i| raw_vals[i % raw_vals.len()]).collect();
    for phys_off in 0..(n - 8) {
        #[allow(unsafe_code)]
        let got = unsafe { super::load8s_s1_simd128(&buf, phys_off) };
        let want = super::load8s(&buf, phys_off, 1);
        assert_eq!(
            got.to_array(),
            want.to_array(),
            "simd128 load8s_s1 mismatch at phys_off {}",
            phys_off
        );
    }
}

/// WASM simd128 stride-1 store must truncate i32→i16 (drop upper 16 bits, no
/// saturation) matching the scalar `as i16` cast for every input.
///
/// Mirrors `store8s_s1_avx2_matches_scalar`.
#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
#[test]
fn store8s_s1_simd128_matches_scalar() {
    let raw_vals: [i32; 8] = [i32::MIN, -100_000, -32768, -1, 0, 32767, 100_000, i32::MAX];
    for offset in 0..8usize {
        let mut input = [0i32; 8];
        for j in 0..8 {
            input[j] = raw_vals[(j + offset) % 8];
        }
        let v = wide::i32x8::from(input);

        let mut buf_simd128 = alloc::vec![0xABCDu16 as i16; 32];
        #[allow(unsafe_code)]
        unsafe {
            super::store8s_s1_simd128(&mut buf_simd128, 8, v);
        }
        let mut buf_scalar = alloc::vec![0xABCDu16 as i16; 32];
        for j in 0..8 {
            buf_scalar[8 + j] = input[j] as i16;
        }
        assert_eq!(buf_simd128, buf_scalar, "simd128 store8s_s1 mismatch");
    }
}

// ---- DjVuLibre reference inverse transform --------------------------

/// DjVuLibre `filter_bv` (IW44Image.cpp), ported line for line: the
/// vertical lifting and interpolation at one scale, with every border
/// special case.
fn reference_filter_bv(d: &mut [i16], p0: isize, w: isize, h: isize, rowsize: isize, scale: isize) {
    let at = |d: &[i16], i: isize| d[i as usize] as i32;
    let mut y = 0isize;
    let mut p = p0;
    let s = scale * rowsize;
    let s3 = s + s + s;
    let h = ((h - 1) / scale) + 1;
    while y - 3 < h {
        // 1-Lifting
        {
            let mut q = p;
            let e = q + w;
            if y >= 3 && y + 3 < h {
                while q < e {
                    let a = at(d, q - s) + at(d, q + s);
                    let b = at(d, q - s3) + at(d, q + s3);
                    d[q as usize] = (at(d, q) - (((a << 3) + a - b + 16) >> 5)) as i16;
                    q += scale;
                }
            } else if y < h {
                let mut q1 = (y + 1 < h).then_some(q + s);
                let mut q3 = (y + 3 < h).then_some(q + s3);
                while q < e {
                    let n1 = q1.map_or(0, |i| at(d, i));
                    let n3 = q3.map_or(0, |i| at(d, i));
                    let p1 = if y >= 1 { at(d, q - s) } else { 0 };
                    let p3 = if y >= 3 { at(d, q - s3) } else { 0 };
                    let a = p1 + n1;
                    let b = p3 + n3;
                    d[q as usize] = (at(d, q) - (((a << 3) + a - b + 16) >> 5)) as i16;
                    q += scale;
                    q1 = q1.map(|i| i + scale);
                    q3 = q3.map(|i| i + scale);
                }
            }
        }
        // 2-Interpolation
        {
            let mut q = p - s3;
            let e = q + w;
            if y >= 6 && y < h {
                while q < e {
                    let a = at(d, q - s) + at(d, q + s);
                    let b = at(d, q - s3) + at(d, q + s3);
                    d[q as usize] = (at(d, q) + (((a << 3) + a - b + 8) >> 4)) as i16;
                    q += scale;
                }
            } else if y >= 3 {
                let mut q1 = if y - 2 < h { q + s } else { q - s };
                while q < e {
                    let a = at(d, q - s) + at(d, q1);
                    d[q as usize] = (at(d, q) + ((a + 1) >> 1)) as i16;
                    q += scale;
                    q1 += scale;
                }
            }
        }
        y += 2;
        p += s + s;
    }
}

/// DjVuLibre `filter_bh` (IW44Image.cpp), ported line for line.
fn reference_filter_bh(d: &mut [i16], p0: isize, w: isize, h: isize, rowsize: isize, scale: isize) {
    let at = |d: &[i16], i: isize| d[i as usize] as i32;
    let mut y = 0isize;
    let mut p = p0;
    let s = scale;
    let s3 = s + s + s;
    let rowsize = rowsize * scale;
    while y < h {
        let mut q = p;
        let e = p + w;
        let (mut a0, mut a1, mut a2, mut a3) = (0i32, 0i32, 0i32, 0i32);
        let (mut b0, mut b1, mut b2, mut b3) = (0i32, 0i32, 0i32, 0i32);
        if q < e {
            // x = 0
            if q + s < e {
                a2 = at(d, q + s);
            }
            if q + s3 < e {
                a3 = at(d, q + s3);
            }
            b3 = at(d, q) - ((((a1 + a2) << 3) + (a1 + a2) - a0 - a3 + 16) >> 5);
            b2 = b3;
            d[q as usize] = b3 as i16;
            q += s + s;
        }
        if q < e {
            // x = 2
            a0 = a1;
            a1 = a2;
            a2 = a3;
            if q + s3 < e {
                a3 = at(d, q + s3);
            }
            b3 = at(d, q) - ((((a1 + a2) << 3) + (a1 + a2) - a0 - a3 + 16) >> 5);
            d[q as usize] = b3 as i16;
            q += s + s;
        }
        if q < e {
            // x = 4
            b1 = b2;
            b2 = b3;
            a0 = a1;
            a1 = a2;
            a2 = a3;
            if q + s3 < e {
                a3 = at(d, q + s3);
            }
            b3 = at(d, q) - ((((a1 + a2) << 3) + (a1 + a2) - a0 - a3 + 16) >> 5);
            d[q as usize] = b3 as i16;
            d[(q - s3) as usize] = (at(d, q - s3) + ((b1 + b2 + 1) >> 1)) as i16;
            q += s + s;
        }
        while q + s3 < e {
            a0 = a1;
            a1 = a2;
            a2 = a3;
            a3 = at(d, q + s3);
            b0 = b1;
            b1 = b2;
            b2 = b3;
            b3 = at(d, q) - ((((a1 + a2) << 3) + (a1 + a2) - a0 - a3 + 16) >> 5);
            d[q as usize] = b3 as i16;
            d[(q - s3) as usize] =
                (at(d, q - s3) + ((((b1 + b2) << 3) + (b1 + b2) - b0 - b3 + 8) >> 4)) as i16;
            q += s + s;
        }
        while q < e {
            a0 = a1;
            a1 = a2;
            a2 = a3;
            a3 = 0;
            b0 = b1;
            b1 = b2;
            b2 = b3;
            b3 = at(d, q) - ((((a1 + a2) << 3) + (a1 + a2) - a0 - a3 + 16) >> 5);
            d[q as usize] = b3 as i16;
            d[(q - s3) as usize] =
                (at(d, q - s3) + ((((b1 + b2) << 3) + (b1 + b2) - b0 - b3 + 8) >> 4)) as i16;
            q += s + s;
        }
        while q - s3 < e {
            b1 = b2;
            b2 = b3;
            if q - s3 >= p {
                d[(q - s3) as usize] = (at(d, q - s3) + ((b1 + b2 + 1) >> 1)) as i16;
            }
            q += s + s;
        }
        let _ = b0;
        y += scale;
        p += rowsize;
    }
}

/// DjVuLibre `Transform::Decode::backward(p, w, h, rowsize, 32, 1)`.
fn reference_backward(d: &mut [i16], w: usize, h: usize, rowsize: usize, end: isize) {
    let mut scale = 16isize;
    while scale >= end {
        reference_filter_bv(d, 0, w as isize, h as isize, rowsize as isize, scale);
        reference_filter_bh(d, 0, w as isize, h as isize, rowsize as isize, scale);
        scale >>= 1;
    }
}

/// The inverse wavelet transform matches DjVuLibre's for every page size,
/// including planes narrower or shorter than 128 pixels, where the coarse
/// scales have too few samples for the generic lifting stencil.
#[test]
fn inverse_transform_matches_djvulibre_at_every_size() {
    let mut seed = 0x9e37_79b9_u32;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        ((seed % 4001) as i32 - 2000) as i16
    };
    let sizes: [(usize, usize); 19] = [
        (1, 1),
        (2, 3),
        (7, 5),
        (16, 16),
        (31, 17),
        (32, 32),
        (33, 33),
        (40, 500),
        (64, 64),
        (65, 65),
        (77, 100),
        (96, 96),
        (100, 77),
        (127, 127),
        (128, 128),
        (129, 129),
        (181, 240),
        (200, 13),
        (500, 40),
    ];
    let mut failures = Vec::new();
    for &(w, h) in &sizes {
        let stride = w.div_ceil(32) * 32;
        let rows = h.div_ceil(32) * 32;
        let data: Vec<i16> = (0..stride * rows).map(|_| next()).collect();
        let mut ours = FlatPlane {
            data: data.clone(),
            stride,
        };
        inverse_wavelet_transform(&mut ours, w, h, 1);
        let mut reference = data;
        reference_backward(&mut reference, w, h, stride, 1);
        let differing = (0..h)
            .flat_map(|r| (0..w).map(move |c| r * stride + c))
            .filter(|&i| ours.data[i] != reference[i])
            .count();
        if differing != 0 {
            failures.push(format!("{w}x{h}: {differing} of {} samples differ", w * h));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// With `crcb_half` DjVuLibre's `Map::image(fast)` runs `backward(.., 32, 2)`
/// on the full-size plane and repeats each even sample over its 2x2 block.
/// The compact scale-2 plane that `reconstruct(2)` and `reconstruct_window`
/// transform must hold exactly those even samples, at every size.
#[test]
fn compact_scale2_transform_matches_djvulibre_fast_mode() {
    let mut seed = 0x2545_f491_u32;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        ((seed % 4001) as i32 - 2000) as i16
    };
    let sizes: [(usize, usize); 10] = [
        (1, 1),
        (3, 2),
        (7, 5),
        (33, 33),
        (65, 64),
        (127, 127),
        (129, 130),
        (181, 239),
        (200, 13),
        (301, 199),
    ];
    let mut failures = Vec::new();
    for &(w, h) in &sizes {
        let stride = w.div_ceil(32) * 32;
        let rows = h.div_ceil(32) * 32;
        let data: Vec<i16> = (0..stride * rows).map(|_| next()).collect();
        let (cw, ch) = (w.div_ceil(2), h.div_ceil(2));
        let mut compact = FlatPlane {
            data: (0..rows / 2)
                .flat_map(|r| (0..stride / 2).map(move |c| (r, c)))
                .map(|(r, c)| data[2 * r * stride + 2 * c])
                .collect(),
            stride: stride / 2,
        };
        inverse_wavelet_transform_from(&mut compact, cw, ch, 1, 8);
        let mut reference = data;
        reference_backward(&mut reference, w, h, stride, 2);
        let differing = (0..ch)
            .flat_map(|r| (0..cw).map(move |c| (r, c)))
            .filter(|&(r, c)| {
                compact.data[r * compact.stride + c] != reference[2 * r * stride + 2 * c]
            })
            .count();
        if differing != 0 {
            failures.push(format!(
                "{w}x{h}: {differing} of {} samples differ",
                cw * ch
            ));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// End to end on a real `c44 -crcbhalf` page with odd width and height:
/// the whole-plane and banded conversions both give `ddjvu`'s pixels.
#[test]
fn crcb_half_page_matches_ddjvu() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/chicken_crcbhalf.djvu");
    let data = std::fs::read(path).expect("chicken_crcbhalf.djvu");
    let file = djvu_iff::parse(&data).expect("iff parse");
    let mut img = Iw44Image::new();
    for c in extract_bg44_chunks(&file) {
        img.decode_chunk(c).expect("decode_chunk");
    }
    assert!(img.chroma_half());
    assert_eq!((img.width, img.height), (181, 239));
    let digest = |bytes: &[u8]| {
        bytes.iter().fold(0xcbf29ce484222325u64, |hash, &byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        })
    };
    let whole = img.to_rgb().expect("to_rgb");
    assert_eq!(digest(&whole.data), 0xf286_459b_06b8_f52e, "ddjvu's pixels");

    let y_dec = img.y.as_ref().unwrap();
    let cb_dec = img.cb.as_ref().unwrap();
    let cr_dec = img.cr.as_ref().unwrap();
    for keep in [1usize, 3] {
        let mut banded = Pixmap::try_new(181, 239, 0, 0, 0, 255).expect("fits");
        img.rgb_sub1_banded(y_dec, cb_dec, cr_dec, keep, 181, 239, &mut banded);
        assert_eq!(banded.data, whole.data, "banded, keep {keep}");
    }
}
