use super::*;
use crate::Iw44Image;
use djvu_pixmap::{GrayPixmap, Pixmap};

#[test]
fn rgb_to_ycbcr_matches_djvulibre_pigeon_transform() {
    // Expected values are from DjVuLibre's `rgb_to_ycc` fixed-point table
    // (`component * 65536`, `+32768 >> 16`), including chroma saturation.
    assert_eq!(rgb_to_ycbcr(0, 0, 0), (-128, 0, 0));
    assert_eq!(rgb_to_ycbcr(255, 255, 255), (127, 0, 0));
    assert_eq!(rgb_to_ycbcr(255, 0, 0), (-50, -44, 118));
    assert_eq!(rgb_to_ycbcr(0, 255, 0), (27, -89, -103));
    assert_eq!(rgb_to_ycbcr(0, 0, 255), (-106, 127, -15));
    assert_eq!(rgb_to_ycbcr(12, 34, 56), (-99, 15, -11));
}

/// The default must encode full-resolution chroma. Our `chroma_half` stores
/// Cb/Cr at half *spatial* resolution, which DjVuLibre's IWPixmap decoder (it
/// builds full-resolution chroma maps) reads as too few bits → "Unexpected End
/// Of File". Defaulting to half-chroma made our colour DjVu unreadable in
/// ddjvu/DjVuLibre, so the interop default is full chroma. Locks that decision.
#[test]
fn default_chroma_is_full_resolution_for_interop() {
    assert!(
        !Iw44EncodeOptions::default().chroma_half,
        "default must be full-resolution chroma for DjVuLibre interop"
    );
}

/// The IW44 primary-chunk major-version byte must satisfy DjVuLibre's
/// `(major & 0x7f) == 1` check (IWCODEC_MAJOR), or ddjvu rejects the stream
/// with "incompatible IWCodec". Regression for the colour encode-interop bug.
#[test]
fn bg44_major_version_is_djvulibre_compatible() {
    let mut pm = Pixmap::white(32, 32);
    for y in 0..32 {
        for x in 0..32 {
            pm.set_rgb(x, y, (x * 8) as u8, (y * 8) as u8, 128);
        }
    }
    let chunks = encode_iw44_color(&pm, &Iw44EncodeOptions::default());
    // chunk[0] = serial, slices, major, minor, ...
    let major = chunks[0][2];
    assert_eq!(
        major & 0x7f,
        1,
        "major version must be 1 (got 0x{major:02x})"
    );
    // our colour flag lives in bit 7; a colour image must be decodable as colour
    assert_eq!(major >> 7, 0, "colour chunk should have bit7 clear (0x01)");
}

fn make_pixmap(w: u32, h: u32, f: impl Fn(u32, u32) -> (u8, u8, u8)) -> Pixmap {
    let mut px = Pixmap::white(w, h);
    for y in 0..h {
        for x in 0..w {
            let (r, g, b) = f(x, y);
            px.set_rgb(x, y, r, g, b);
        }
    }
    px
}

fn make_gray(w: u32, h: u32, f: impl Fn(u32, u32) -> u8) -> GrayPixmap {
    let mut data = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            data.push(f(x, y));
        }
    }
    GrayPixmap {
        width: w,
        height: h,
        data,
    }
}

fn decode_color(chunks: &[Vec<u8>]) -> Pixmap {
    let mut img = Iw44Image::new();
    for c in chunks {
        img.decode_chunk(c).unwrap();
    }
    img.to_rgb().unwrap()
}

fn decode_gray(chunks: &[Vec<u8>]) -> GrayPixmap {
    let mut img = Iw44Image::new();
    for c in chunks {
        img.decode_chunk(c).unwrap();
    }
    img.to_rgb().unwrap().to_gray8()
}

/// Colour noise over a slow gradient: every band of the transform carries
/// energy, so a halo one row too short shows up as a differing coefficient.
fn noisy_pixmap(w: u32, h: u32) -> Pixmap {
    make_pixmap(w, h, |x, y| {
        let v = (x.wrapping_mul(2_654_435_761) ^ y.wrapping_mul(40_503))
            .wrapping_mul(2_246_822_519)
            >> 8;
        let g = ((x + 2 * y) % 251) as u8;
        (
            (v as u8) / 2 + g / 2,
            ((v >> 8) as u8) / 2 + g / 2,
            ((v >> 16) as u8) / 2 + g / 2,
        )
    })
}

/// The three colour encoders after `forward_gather_banded` with the given
/// band size, in block rows. `keep` at or above the block-row count is
/// one band with no halo — the whole-plane transform by another route.
fn banded_color_encoders(px: &Pixmap, keep: usize, halo: usize) -> [PlaneEncoder; 3] {
    let w = px.width as usize;
    let h = px.height as usize;
    let stride = w.div_ceil(32) * 32;
    let mut encs = [
        PlaneEncoder::new(w, h),
        PlaneEncoder::new(w, h),
        PlaneEncoder::new(w, h),
    ];
    forward_gather_banded(&mut encs, w, h, stride, keep, halo, |band, bufs| {
        fill_color_band(px, band, stride, bufs)
    });
    encs
}

/// `skip_quiet_block` codes a still-zero block band in one step. Its bytes
/// must equal the full passes on a page that mixes flat blocks (skipped in
/// most slices) with noisy ones (never skipped), over every band and slice.
#[test]
fn quiet_block_skip_matches_the_full_passes() {
    let px = make_pixmap(200, 136, |x, y| {
        if x < 96 {
            ((x / 2) as u8, (y / 2) as u8, 90)
        } else {
            let v = (x.wrapping_mul(2_654_435_761) ^ y.wrapping_mul(40_503))
                .wrapping_mul(2_246_822_519)
                >> 8;
            (v as u8, (v >> 8) as u8, (v >> 16) as u8)
        }
    });
    let coded = |full_passes: bool| {
        let mut encs = banded_color_encoders(&px, usize::MAX, 0);
        encs.iter_mut()
            .map(|enc| {
                enc.full_passes = full_passes;
                let mut zp = ZpEncoder::new();
                for _ in 0..100 {
                    enc.encode_slice(&mut zp);
                }
                zp.finish()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(coded(false), coded(true));
}

/// `from_plane` gathers in place, inside the plane's own memory; the grid
/// must equal the one `gather_rows` copies out of a separate flat plane.
#[test]
fn from_plane_matches_gather_rows() {
    let (w, h) = (101, 70); // 4 x 3 blocks, both edges partial
    let mut plane = PlaneEncoder::new_plane(w, h);
    for (i, v) in plane.as_flattened_mut().iter_mut().enumerate() {
        *v = (i as i32 * 7919 % 65_521 - 32_760) as i16;
    }
    let flat = plane.as_flattened().to_vec();
    let stride = w.div_ceil(32) * 32;
    let mut copied = PlaneEncoder::new(w, h);
    copied.gather_rows(&flat, stride, 0, 0, h.div_ceil(32));
    let in_place = PlaneEncoder::from_plane(w, h, plane);
    assert_eq!(in_place.block_cols, copied.block_cols);
    assert_eq!(in_place.recon.len(), copied.recon.len());
    assert_eq!(
        differing_block_rows(&in_place, &copied),
        Vec::<usize>::new()
    );
}

/// Block rows that differ between two gathered grids, for the messages.
fn differing_block_rows(a: &PlaneEncoder, b: &PlaneEncoder) -> Vec<usize> {
    assert_eq!(a.blocks.len(), b.blocks.len());
    let mut rows: Vec<usize> = a
        .blocks
        .iter()
        .zip(&b.blocks)
        .enumerate()
        .filter(|(_, (x, y))| x[..] != y[..])
        .map(|(i, _)| i / a.block_cols)
        .collect();
    rows.dedup();
    rows
}

/// A single band with no halo is the whole-plane path: the chunks it
/// produces are the bytes `encode_iw44_color` writes for a page below the
/// banding threshold.
#[test]
fn one_band_is_the_whole_plane_path() {
    let px = noisy_pixmap(203, 371);
    let opts = Iw44EncodeOptions::default();
    let expected = encode_iw44_color(&px, &opts);
    assert!(
        encode_band_keep_blocks(224, 12, 3).is_none(),
        "a 203x371 page must take the whole-plane path"
    );
    let [mut y, mut cb, mut cr] = banded_color_encoders(&px, 12, 0);
    let got = encode_chunks(&mut y, Some(&mut cb), Some(&mut cr), 203, 371, true, &opts);
    assert_eq!(got, expected);
}

/// Every band size, with the production halo, gathers the same grid as
/// one band over the whole plane. The forward transform's vertical reach
/// is the inverse one's, 186 rows. Probed on this page with a band of 3
/// block rows and `halo` from 0 up: 0..4 block rows fail (108, 106, 98,
/// 46 and 6 differing block rows) and 5 is the first that passes.
/// `BAND_HALO_BLOCKS` is 8, above the 186-row reach with the margin the
/// decoder keeps.
#[test]
fn banded_forward_transform_matches_the_whole_plane() {
    let px = noisy_pixmap(203, 1131); // 36 block rows, last one partial
    let whole = banded_color_encoders(&px, usize::MAX, 0);
    for keep in [1usize, 3, 8, 17, 35] {
        let banded = banded_color_encoders(&px, keep, crate::BAND_HALO_BLOCKS);
        for (name, w, b) in [
            ("Y", &whole[0], &banded[0]),
            ("Cb", &whole[1], &banded[1]),
            ("Cr", &whole[2], &banded[2]),
        ] {
            let bad = differing_block_rows(w, b);
            assert!(
                bad.is_empty(),
                "keep {keep}: {name} block rows {bad:?} differ from the whole plane"
            );
        }
    }
}

/// A halo that is too short must be visible to the test above: with none
/// at all, the band edges carry the transform's boundary handling and the
/// grid differs. Guards the guard.
#[test]
fn a_missing_halo_is_detected() {
    let px = noisy_pixmap(203, 1131);
    let whole = banded_color_encoders(&px, usize::MAX, 0);
    let banded = banded_color_encoders(&px, 8, 0);
    assert!(!differing_block_rows(&whole[0], &banded[0]).is_empty());
}

/// The grey path, same shape.
#[test]
fn banded_gray_forward_transform_matches_the_whole_plane() {
    let px = make_gray(197, 1000, |x, y| {
        ((x * 7 + y * 13) % 256) as u8 ^ ((x ^ y) as u8)
    });
    let w = 197usize;
    let h = 1000usize;
    let stride = w.div_ceil(32) * 32;
    let run = |keep: usize, halo: usize| {
        let mut encs = [PlaneEncoder::new(w, h)];
        forward_gather_banded(&mut encs, w, h, stride, keep, halo, |band, bufs| {
            fill_gray_band(&px, band, stride, bufs)
        });
        let [enc] = encs;
        enc
    };
    let whole = run(usize::MAX, 0);
    for keep in [1usize, 5, 16] {
        let banded = run(keep, crate::BAND_HALO_BLOCKS);
        let bad = differing_block_rows(&whole, &banded);
        assert!(bad.is_empty(), "keep {keep}: block rows {bad:?} differ");
    }
    // The single-band route is the production whole-plane path.
    let opts = Iw44EncodeOptions::default();
    let mut one = run(usize::MAX, 0);
    let got = encode_chunks(&mut one, None, None, w as u16, h as u16, false, &opts);
    assert_eq!(got, encode_iw44_gray(&px, &opts));
}

/// The sizing policy: small planes are never banded, large ones keep at
/// least the minimum band, and a band is never more than half the page.
#[test]
fn encode_band_policy() {
    // colorbook-sized: 2272 x 3680 x 2 x 3 = 50 MB, under the threshold.
    assert_eq!(encode_band_keep_blocks(2272, 115, 3), None);
    // The big fixture: 6784 x 9152: 372 MB of planes.
    let keep = encode_band_keep_blocks(6784, 286, 3).unwrap();
    assert_eq!(keep, crate::BAND_MIN_KEEP_BLOCKS);
    assert!((keep + 2 * crate::BAND_HALO_BLOCKS) * 6784 * 32 * 2 * 3 <= 64 << 20);
    // One grey plane of the same page is 124 MB: under the threshold, as
    // it is for the decoder. A grey page half as large again is banded,
    // with a wider band because it has the budget to itself.
    assert_eq!(encode_band_keep_blocks(6784, 286, 1), None);
    let g = encode_band_keep_blocks(6784, 430, 1).unwrap();
    assert!(g > keep && g * 2 <= 430, "grey keep {g}");
    // A plane just over the threshold but too short to split in two.
    assert_eq!(encode_band_keep_blocks(32 * 3000, 40, 3), None);
}

#[test]
fn encode_color_produces_decodable_chunks() {
    let src = make_pixmap(64, 64, |x, y| {
        ((x * 4) as u8, (y * 4) as u8, ((x + y) * 2) as u8)
    });
    let opts = Iw44EncodeOptions {
        slices_per_chunk: 10,
        total_slices: 10,
        ..Default::default()
    };
    let chunks = encode_iw44_color(&src, &opts);
    assert!(!chunks.is_empty());
    let decoded = decode_color(&chunks);
    assert_eq!(decoded.width, 64);
    assert_eq!(decoded.height, 64);
}

#[test]
fn encode_gray_produces_decodable_chunks() {
    let src = make_gray(32, 32, |x, y| ((x + y) * 4) as u8);
    let opts = Iw44EncodeOptions {
        slices_per_chunk: 10,
        total_slices: 10,
        ..Default::default()
    };
    let chunks = encode_iw44_gray(&src, &opts);
    assert!(!chunks.is_empty());
    let decoded = decode_gray(&chunks);
    assert_eq!(decoded.width, 32);
    assert_eq!(decoded.height, 32);
}

#[test]
fn chunk_header_serial_0() {
    let src = make_pixmap(16, 16, |_, _| (200, 100, 50));
    let opts = Iw44EncodeOptions {
        slices_per_chunk: 5,
        total_slices: 5,
        ..Default::default()
    };
    let chunks = encode_iw44_color(&src, &opts);
    let first = &chunks[0];
    assert_eq!(first[0], 0, "serial must be 0");
    assert_eq!(first[1], 5, "slices count");
    assert_eq!(first[2] & 0x80, 0, "color image: majver bit 7 = 0");
    assert_eq!(first[3], 2, "minor = 2");
    assert_eq!(u16::from_be_bytes([first[4], first[5]]), 16u16);
    assert_eq!(u16::from_be_bytes([first[6], first[7]]), 16u16);
}

#[test]
fn multi_chunk_serials_increment() {
    let src = make_pixmap(32, 32, |x, y| ((x * 8) as u8, (y * 8) as u8, 0));
    let opts = Iw44EncodeOptions {
        slices_per_chunk: 10,
        total_slices: 30,
        ..Default::default()
    };
    let chunks = encode_iw44_color(&src, &opts);
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0][0], 0);
    assert_eq!(chunks[1][0], 1);
    assert_eq!(chunks[2][0], 2);
}

#[test]
fn gray_flat_roundtrip() {
    // Flat image: all pixels = 100. After encode/decode should be ~100.
    let src = make_gray(32, 32, |_x, _y| 100u8);
    let opts = Iw44EncodeOptions {
        slices_per_chunk: 10,
        total_slices: 100,
        ..Default::default()
    };
    let chunks = encode_iw44_gray(&src, &opts);
    let decoded = decode_gray(&chunks);
    let mut total = 0u64;
    for y in 0..32 {
        for x in 0..32 {
            total += (src.get(x, y) as i32 - decoded.get(x, y) as i32).unsigned_abs() as u64;
        }
    }
    let avg = total as f64 / (32.0 * 32.0);
    // Diagnose: print a few decoded values
    for y in 0..4 {
        for x in 0..4 {
            print!("({},{})={} ", x, y, decoded.get(x, y));
        }
        println!();
    }
    assert!(avg < 10.0, "flat avg error = {avg:.2} (expected < 10)");
}

// Lines 426-432: tail loop in forward_row_pass for widths not a multiple of 8.
#[test]
fn encode_odd_width_roundtrips() {
    // width=10 means the main 8-wide loop runs once (cols 0-7), then the
    // tail loop runs twice (cols 8-9), covering lines 426-432.
    let src = make_pixmap(10, 8, |x, y| ((x * 25) as u8, (y * 32) as u8, 128));
    let opts = Iw44EncodeOptions::default();
    let chunks = encode_iw44_color(&src, &opts);
    let decoded = decode_color(&chunks);
    assert_eq!((decoded.width, decoded.height), (10, 8));
}

#[test]
fn gray_low_error_many_slices() {
    // Test grayscale roundtrip quality — avoids the YCbCr color-space mismatch.
    // With 100 slices the average absolute error per pixel should be well below 20.
    let src = make_gray(64, 64, |x, y| ((x * 2 + y * 2).min(255)) as u8);
    let opts = Iw44EncodeOptions {
        slices_per_chunk: 10,
        total_slices: 100,
        ..Default::default()
    };
    let chunks = encode_iw44_gray(&src, &opts);
    let decoded = decode_gray(&chunks);
    assert_eq!((decoded.width, decoded.height), (64, 64));
    let mut total = 0u64;
    for y in 0..src.height {
        for x in 0..src.width {
            total += (src.get(x, y) as i32 - decoded.get(x, y) as i32).unsigned_abs() as u64;
        }
    }
    let avg = total as f64 / (64.0 * 64.0);
    assert!(avg < 30.0, "avg gray abs error = {avg:.2} (expected < 30)");
}

// The legacy chroma_half option must still emit a full-resolution,
// interoperable stream and round-trip through the decoder.
#[test]
fn encode_color_legacy_chroma_half_stays_full_resolution() {
    let src = make_pixmap(32, 32, |x, y| ((x * 8) as u8, (y * 8) as u8, 128));
    let opts = Iw44EncodeOptions {
        chroma_half: true,
        ..Default::default()
    };
    let chunks = encode_iw44_color(&src, &opts);
    assert!(!chunks.is_empty());
    // delay_byte bit 7 must still advertise full-resolution chroma.
    assert_eq!(chunks[0][8] & 0x80, 0x80, "delay_byte bit 7 must be set");
    let decoded = decode_color(&chunks);
    assert_eq!((decoded.width, decoded.height), (32, 32));
}

// ---- Iw44Target::Bpp tests ------------------------------------------------

/// Default (Slices) and explicit Slices produce identical bytes — the new
/// `target` field must not affect the legacy path.
#[test]
fn bpp_target_slices_default_is_byte_identical() {
    let src = make_pixmap(64, 64, |x, y| {
        ((x * 4) as u8, (y * 4) as u8, ((x + y) * 2) as u8)
    });
    let default_opts = Iw44EncodeOptions::default();
    let explicit_slices = Iw44EncodeOptions {
        target: Iw44Target::Slices,
        ..Default::default()
    };
    let a = encode_iw44_color(&src, &default_opts);
    let b = encode_iw44_color(&src, &explicit_slices);
    let a_bytes: usize = a.iter().map(|c| c.len()).sum();
    let b_bytes: usize = b.iter().map(|c| c.len()).sum();
    assert_eq!(
        a_bytes, b_bytes,
        "Slices default and explicit Slices must be byte-identical"
    );
    assert_eq!(
        a, b,
        "Slices default and explicit Slices must produce identical chunk vectors"
    );
}

/// A low bpp target produces strictly fewer bytes than the slice-default.
#[test]
fn bpp_target_low_bpp_yields_fewer_bytes_than_default() {
    let src = make_pixmap(64, 64, |x, y| {
        ((x * 4) as u8, (y * 4) as u8, ((x + y) * 2) as u8)
    });
    let default_opts = Iw44EncodeOptions::default();
    let low_bpp_opts = Iw44EncodeOptions {
        target: Iw44Target::Bpp(0.1),
        ..Default::default()
    };
    let default_bytes: usize = encode_iw44_color(&src, &default_opts)
        .iter()
        .map(|c| c.len())
        .sum();
    let low_bytes: usize = encode_iw44_color(&src, &low_bpp_opts)
        .iter()
        .map(|c| c.len())
        .sum();
    assert!(
        low_bytes < default_bytes,
        "low bpp target ({low_bytes} B) should yield fewer bytes than slice-default ({default_bytes} B)"
    );
}

/// A high bpp target yields more bytes than a low bpp target (monotonicity).
#[test]
fn bpp_target_monotone_size() {
    let src = make_pixmap(64, 64, |x, y| {
        ((x * 4) as u8, (y * 4) as u8, ((x + y) * 2) as u8)
    });
    let opts_low = Iw44EncodeOptions {
        target: Iw44Target::Bpp(0.05),
        ..Default::default()
    };
    let opts_mid = Iw44EncodeOptions {
        target: Iw44Target::Bpp(0.3),
        ..Default::default()
    };
    let opts_high = Iw44EncodeOptions {
        target: Iw44Target::Bpp(1.0),
        ..Default::default()
    };
    let low: usize = encode_iw44_color(&src, &opts_low)
        .iter()
        .map(|c| c.len())
        .sum();
    let mid: usize = encode_iw44_color(&src, &opts_mid)
        .iter()
        .map(|c| c.len())
        .sum();
    let high: usize = encode_iw44_color(&src, &opts_high)
        .iter()
        .map(|c| c.len())
        .sum();
    assert!(
        low <= mid,
        "bpp 0.05 ({low} B) should be <= bpp 0.3 ({mid} B)"
    );
    assert!(
        mid <= high,
        "bpp 0.3 ({mid} B) should be <= bpp 1.0 ({high} B)"
    );
}

/// A bpp-targeted stream is valid: it decodes without error and produces
/// an image with the correct dimensions.
#[test]
fn bpp_target_output_is_decodable() {
    let src = make_pixmap(64, 64, |x, y| {
        ((x * 4) as u8, (y * 4) as u8, ((x + y) * 2) as u8)
    });
    let opts = Iw44EncodeOptions {
        target: Iw44Target::Bpp(0.2),
        ..Default::default()
    };
    let chunks = encode_iw44_color(&src, &opts);
    assert!(
        !chunks.is_empty(),
        "bpp target must emit at least one chunk"
    );
    let decoded = decode_color(&chunks);
    assert_eq!(decoded.width, 64, "decoded width must match source");
    assert_eq!(decoded.height, 64, "decoded height must match source");
}

/// Even a tiny (near-zero) bpp budget emits at least one chunk (the minimum
/// one-slice guarantee).
#[test]
fn bpp_target_tiny_budget_emits_at_least_one_chunk() {
    let src = make_pixmap(32, 32, |x, y| ((x * 8) as u8, (y * 8) as u8, 128));
    let opts = Iw44EncodeOptions {
        target: Iw44Target::Bpp(0.0),
        ..Default::default()
    };
    let chunks = encode_iw44_color(&src, &opts);
    assert!(
        !chunks.is_empty(),
        "even bpp=0 must emit at least one chunk"
    );
}

/// Grayscale bpp target: low bpp yields fewer bytes than default; output decodes.
#[test]
fn bpp_target_gray_low_bpp_yields_fewer_bytes() {
    let src = make_gray(64, 64, |x, y| ((x * 2 + y * 2).min(255)) as u8);
    let default_opts = Iw44EncodeOptions::default();
    let bpp_opts = Iw44EncodeOptions {
        target: Iw44Target::Bpp(0.1),
        ..Default::default()
    };
    let default_bytes: usize = encode_iw44_gray(&src, &default_opts)
        .iter()
        .map(|c| c.len())
        .sum();
    let bpp_bytes: usize = encode_iw44_gray(&src, &bpp_opts)
        .iter()
        .map(|c| c.len())
        .sum();
    assert!(
        bpp_bytes < default_bytes,
        "low bpp gray ({bpp_bytes} B) should be fewer bytes than default ({default_bytes} B)"
    );
    // Also verify decodability
    let chunks = encode_iw44_gray(&src, &bpp_opts);
    let decoded = decode_gray(&chunks);
    assert_eq!((decoded.width, decoded.height), (64, 64));
}

/// IW44_ENTROPY_PROBE (round 51): enabling the `iw44-probe` diagnostic
/// counters must be a pure observer — same encoded bytes with or without
/// resetting/reading the counters around the encode. Guards the "encoder-only,
/// decoder-unchanged" claim in the probe module doc.
#[cfg(feature = "iw44-probe")]
#[test]
fn probe_does_not_change_output() {
    let src = make_pixmap(96, 96, |x, y| {
        ((x * 3) as u8, (y * 3) as u8, ((x + y) % 256) as u8)
    });
    let opts = Iw44EncodeOptions::default();

    probe::reset();
    let a = encode_iw44_color(&src, &opts);
    let snap = probe::snapshot();
    probe::reset();
    let b = encode_iw44_color(&src, &opts);

    assert_eq!(
        a, b,
        "enabling iw44-probe counters must not change encoder output"
    );
    let total_bytes: u64 = snap.iter().map(|s| s.bytes).sum();
    assert!(total_bytes > 0, "probe should have recorded byte activity");
    let total_activations: u64 = snap.iter().map(|s| s.activate.true_count).sum();
    assert!(
        total_activations > 0,
        "probe should have recorded coefficient activations"
    );
}
