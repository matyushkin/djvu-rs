use super::*;

/// The byte-wise `aligned_hamming` equals a per-pixel count under the
/// same center alignment, for every size pair within ±3 px.
#[test]
fn aligned_hamming_matches_per_pixel_count() {
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut random_bitmap = |w: u32, h: u32| {
        let mut bm = Bitmap::new(w, h);
        for y in 0..h {
            for x in 0..w {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                bm.set(x, y, seed.is_multiple_of(3));
            }
        }
        // Junk in the row padding bits must not count.
        if !w.is_multiple_of(8) {
            let stride = bm.row_stride();
            for row in bm.data.chunks_exact_mut(stride) {
                row[stride - 1] |= 0xFF >> (w % 8);
            }
        }
        bm
    };
    // Widths around 64 cover `aligned_hamming_words` (both at most 64 px)
    // and pairs that only the byte path takes.
    for (cw, ch) in [
        (1u32, 1u32),
        (7, 5),
        (8, 8),
        (9, 13),
        (17, 4),
        (33, 21),
        (62, 6),
        (64, 7),
        (66, 5),
        (90, 4),
    ] {
        let cand = random_bitmap(cw, ch);
        for dw in -3i32..=3 {
            for dh in -3i32..=3 {
                let (mw, mh) = (
                    (cw as i32 + dw).max(1) as u32,
                    (ch as i32 + dh).max(1) as u32,
                );
                let reference = random_bitmap(mw, mh);
                let row_shift = ((mh as i32 - 1) >> 1) - ((ch as i32 - 1) >> 1);
                let col_shift = ((mw as i32 - 1) >> 1) - ((cw as i32 - 1) >> 1);
                let mut want = 0;
                for y in 0..ch as i32 {
                    let my = mh as i32 - 1 - (ch as i32 - 1 - y + row_shift);
                    for x in 0..cw as i32 {
                        let mx = x + col_shift;
                        let r = (0..mh as i32).contains(&my)
                            && (0..mw as i32).contains(&mx)
                            && reference.get(mx as u32, my as u32);
                        want += u32::from(cand.get(x as u32, y as u32) != r);
                    }
                }
                assert_eq!(
                    aligned_hamming(&cand, &reference, u32::MAX),
                    want,
                    "{cw}x{ch} vs {mw}x{mh}"
                );
                let inside = mw <= cw && mh <= ch;
                assert!(
                    grid_bound(&ink_grid(&cand), &ink_grid(&reference), inside) <= want,
                    "grid {cw}x{ch} vs {mw}x{mh}"
                );
                if cw <= 64 && mw <= 64 {
                    let (mut c, mut m) = (Vec::new(), Vec::new());
                    assert!(push_row_words(&cand, &mut c));
                    assert!(push_row_words(&reference, &mut m));
                    assert_eq!(
                        aligned_hamming_words(&c, cw, &m, mw, u32::MAX),
                        want,
                        "words {cw}x{ch} vs {mw}x{mh}"
                    );
                }
            }
        }
    }
}

/// `ink_grid` counts each pixel in the cell of its offsets from the centre,
/// and two pixels that `aligned_hamming` lines up share a cell, so
/// `grid_bound` is 0 for a one-pixel pair that lines up.
#[test]
fn ink_grid_cells_follow_the_alignment() {
    // Cell band of an offset from the centre: < -2, -2..0, 0..2, >= 2.
    let band = |off: i32| (off >= -2) as usize + (off >= 0) as usize + (off >= 2) as usize;
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    for (cw, ch) in [(1u32, 1u32), (5, 7), (8, 8), (9, 4), (12, 11), (21, 3)] {
        let mut cand = Bitmap::new(cw, ch);
        let mut want = [0u32; 16];
        for y in 0..ch as i32 {
            for x in 0..cw as i32 {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                if seed.is_multiple_of(3) {
                    cand.set(x as u32, y as u32, true);
                    let r = ch as i32 - 1 - y - ((ch as i32 - 1) >> 1);
                    want[4 * band(r) + band(x - ((cw as i32 - 1) >> 1))] += 1;
                }
            }
        }
        assert_eq!(ink_grid(&cand), want, "{cw}x{ch}");
        for dw in -3i32..=3 {
            for dh in -3i32..=3 {
                let (mw, mh) = (cw as i32 + dw, ch as i32 + dh);
                if mw < 1 || mh < 1 {
                    continue;
                }
                let row_shift = ((mh - 1) >> 1) - ((ch as i32 - 1) >> 1);
                let col_shift = ((mw - 1) >> 1) - ((cw as i32 - 1) >> 1);
                for y in 0..ch as i32 {
                    let my = mh - 1 - (ch as i32 - 1 - y + row_shift);
                    for x in 0..cw as i32 {
                        let mx = x + col_shift;
                        if !(0..mh).contains(&my) || !(0..mw).contains(&mx) {
                            continue;
                        }
                        let mut c = Bitmap::new(cw, ch);
                        c.set(x as u32, y as u32, true);
                        let mut m = Bitmap::new(mw as u32, mh as u32);
                        m.set(mx as u32, my as u32, true);
                        assert_eq!(
                            grid_bound(&ink_grid(&c), &ink_grid(&m), true),
                            0,
                            "{cw}x{ch} ({x},{y}) vs {mw}x{mh} ({mx},{my})"
                        );
                    }
                }
            }
        }
    }
}

use crate as jb2;
use djvu_bitmap::Bitmap;

fn make_bitmap(w: u32, h: u32, f: impl Fn(u32, u32) -> bool) -> Bitmap {
    let mut bm = Bitmap::new(w, h);
    for y in 0..h {
        for x in 0..w {
            bm.set(x, y, f(x, y));
        }
    }
    bm
}

/// A page of ring glyphs whose size and edge pixels vary a little, as
/// scanned copies of one letter do.
fn noisy_ring_page() -> Bitmap {
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut page = Bitmap::new(640, 400);
    for row in 0..10u32 {
        for col in 0..20u32 {
            let (gw, gh) = (20 + (next() % 3) as u32, 24 + (next() % 3) as u32);
            let (cx, cy) = (gw as f32 / 2.0, gh as f32 / 2.0);
            for y in 0..gh {
                for x in 0..gw {
                    let dx = (x as f32 + 0.5 - cx) / cx;
                    let dy = (y as f32 + 0.5 - cy) / cy;
                    let r = dx * dx + dy * dy;
                    let noise = next().is_multiple_of(16);
                    if (0.45..=1.0).contains(&r) != noise {
                        page.set(col * 32 + 4 + x, row * 40 + 4 + y, true);
                    }
                }
            }
        }
    }
    page
}

#[test]
fn lossless_roundtrips_and_beats_plain_dict() {
    let page = noisy_ring_page();
    let lossless = encode_jb2_lossless(&page);
    let decoded = jb2::decode(&lossless, None).expect("decode failed");
    assert_eq!(decoded.data, page.data, "lossless output must be exact");
    let plain = encode_jb2_dict(&page);
    assert!(
        lossless.len() < plain.len(),
        "aligned refinement {} B vs plain dict {} B",
        lossless.len(),
        plain.len()
    );
}

#[test]
fn lossless_keeps_direct_when_smaller() {
    // One page-sized component with no repeats: direct coding keeps the
    // context across the whole page and wins.
    let page = make_bitmap(300, 200, |x, y| (x / 3 + y / 5) % 7 < 3 || x == y);
    let lossless = encode_jb2_lossless(&page);
    let direct = encode_jb2(&page);
    assert!(lossless.len() <= direct.len());
    let decoded = jb2::decode(&lossless, None).expect("decode failed");
    assert_eq!(decoded.data, page.data);
}

#[test]
fn lossless_falls_back_to_direct_over_record_limit() {
    // One isolated dot per component: more components than the decoder
    // accepts records on one page.
    let (w, h) = (600u32, 480u32);
    let page = make_bitmap(w, h, |x, y| x % 2 == 0 && y % 2 == 0);
    assert!((w / 2 * h / 2) as usize + 2 > crate::MAX_RECORDS);
    let lossless = encode_jb2_lossless(&page);
    assert_eq!(lossless, encode_jb2(&page));
    let decoded = jb2::decode(&lossless, None).expect("decode failed");
    assert_eq!(decoded.data, page.data);
}

/// A page above 16 MP with more components than the record limit falls
/// back to direct tiles, and those tiles must decode: the decoder's page
/// budget used to stop at 16 MP of symbol pixels.
#[test]
fn lossless_large_page_falls_back_to_decodable_tiles() {
    let (w, h) = (5120u32, 4096u32);
    assert!((w * h) as usize > 16 * 1024 * 1024);
    let page = make_bitmap(w, h, |x, y| x % 8 == 0 && y % 8 == 0);
    let lossless = encode_jb2_lossless(&page);
    assert_eq!(lossless, encode_jb2(&page));
    let decoded = jb2::decode(&lossless, None).expect("decode failed");
    assert_eq!(decoded.data, page.data);
}

/// Square outline `size` px wide with a `t` px stroke, placed at `x0`;
/// `defect` clears one pixel of the top edge so outlines differ slightly.
fn outline(x: u32, y: u32, x0: u32, size: u32, t: u32, defect: Option<u32>) -> bool {
    if x < x0 || x >= x0 + size || y >= size {
        return false;
    }
    let (lx, ly) = (x - x0, y);
    if ly == 0 && defect == Some(lx) {
        return false;
    }
    lx < t || ly < t || lx >= size - t || ly >= size - t
}

/// A component whose bounding box is over the decoder's 16 MP per-symbol
/// limit is split into pieces, so the dictionary stream decodes.
#[test]
fn dict_splits_component_over_symbol_limit() {
    let size = 4200u32;
    assert!((size * size) as usize > crate::MAX_SYMBOL_PIXELS);
    let page = make_bitmap(size, size, |x, y| outline(x, y, 0, size, 2, None));
    for stream in [encode_jb2_dict(&page), encode_jb2_lossless(&page)] {
        let decoded = jb2::decode(&stream, None).expect("decode failed");
        assert_eq!(decoded.data, page.data);
    }
}

/// Nested outlines: their bounding boxes add up past the per-page budget,
/// so the large ones are cut finer and the stream still decodes.
#[test]
fn dict_splits_finer_when_boxes_exceed_page_budget() {
    let size = 7000u32;
    let page = make_bitmap(size, size, |x, y| {
        (0..4u32).any(|k| {
            let m = k * 600;
            x >= m && y >= m && outline(x - m, y - m, 0, size - 2 * m, 2, None)
        })
    });
    let boxes: usize = (0..4usize).map(|k| (7000 - 1200 * k).pow(2)).sum();
    assert!(boxes > crate::MAX_PAGE_SYMBOL_WORK);
    let decoded = jb2::decode(&encode_jb2_dict(&page), None).expect("decode failed");
    assert_eq!(decoded.data, page.data);
}

/// Three near-identical 9 MP outlines: refining both copies would cost
/// 9 + 2 × 36 M work units, over the decoder's 64 M page budget. The
/// encoder refines only while the page stays within it.
#[test]
fn aligned_refinement_stays_within_page_budget() {
    let (size, pitch) = (3000u32, 3010u32);
    let page = make_bitmap(3 * pitch, size, |x, y| {
        let i = x / pitch;
        outline(x, y, i * pitch, size, 3, Some(100 + i))
    });
    let px = (size * size) as usize;
    assert!(px + 2 * px * crate::REFINE_PIXEL_WORK > crate::MAX_PAGE_SYMBOL_WORK);
    let (stream, _) = encode_jb2_dict_with_blits_refined(
        &page,
        &[],
        &Jb2EncodeOptions::default(),
        Some(AlignedRefine::LOSSLESS),
    );
    let decoded = jb2::decode(&stream, None).expect("decode failed");
    assert_eq!(decoded.data, page.data);
}

#[test]
fn lossless_empty_page() {
    assert!(encode_jb2_lossless(&Bitmap::new(0, 5)).is_empty());
    let blank = Bitmap::new(33, 17);
    let decoded = jb2::decode(&encode_jb2_lossless(&blank), None).expect("decode failed");
    assert_eq!(decoded.data, blank.data);
}

fn roundtrip(bm: &Bitmap) -> Bitmap {
    let encoded = encode_jb2(bm);
    jb2::decode(&encoded, None).expect("decode failed")
}

#[test]
fn all_white_roundtrip() {
    let src = Bitmap::new(32, 32);
    let decoded = roundtrip(&src);
    assert_eq!(decoded.width, 32);
    assert_eq!(decoded.height, 32);
    for y in 0..32u32 {
        for x in 0..32u32 {
            assert!(!decoded.get(x, y), "expected white at ({x},{y})");
        }
    }
}

#[test]
fn all_black_roundtrip() {
    let src = make_bitmap(32, 32, |_, _| true);
    let decoded = roundtrip(&src);
    for y in 0..32u32 {
        for x in 0..32u32 {
            assert!(decoded.get(x, y), "expected black at ({x},{y})");
        }
    }
}

#[test]
fn checkerboard_roundtrip() {
    let src = make_bitmap(16, 16, |x, y| (x + y) % 2 == 0);
    let decoded = roundtrip(&src);
    for y in 0..16u32 {
        for x in 0..16u32 {
            assert_eq!(decoded.get(x, y), (x + y) % 2 == 0, "mismatch at ({x},{y})");
        }
    }
}

#[test]
fn single_pixel_roundtrip() {
    // A 1×1 bitmap with a single black pixel.
    let src = make_bitmap(1, 1, |_, _| true);
    let decoded = roundtrip(&src);
    assert_eq!(decoded.width, 1);
    assert_eq!(decoded.height, 1);
    assert!(decoded.get(0, 0));
}

#[test]
fn larger_image_roundtrip() {
    let src = make_bitmap(64, 64, |x, y| (x * 17 + y * 31) % 5 != 0);
    let decoded = roundtrip(&src);
    assert_eq!(decoded.width, 64);
    assert_eq!(decoded.height, 64);
    let mut mismatches = 0u32;
    for y in 0..64u32 {
        for x in 0..64u32 {
            if decoded.get(x, y) != src.get(x, y) {
                mismatches += 1;
            }
        }
    }
    assert_eq!(
        mismatches, 0,
        "{mismatches} pixel mismatches in 64×64 roundtrip"
    );
}

#[test]
fn encoded_is_nonempty() {
    let src = Bitmap::new(8, 8);
    let encoded = encode_jb2(&src);
    assert!(!encoded.is_empty());
}

/// Regression for the JB2 post-EOF guard wrongly rejecting valid pages
/// (email report, 2026-06; companion to the IW44 early-exit bug).
///
/// `encode_jb2` tiles at 1024 rows, row-major. These images pack a
/// high-entropy first tile (rows 0..1024) that drains the ZP byte buffer,
/// followed by **multiple** solid trailing tiles (rows ≥1024). After the
/// first solid tile consumes the last real bytes, a later solid tile's
/// header (a large, >4096px symbol) is read while `zp.is_exhausted()` is
/// already true — but it decodes correctly from the ~4 bytes of ZP
/// look-ahead. The previous `is_exhausted() && pixels > 4096` guard
/// returned `Truncated` here; the `synthetic_bytes()` guard does not,
/// because no synthetic `0xFF` padding has actually been consumed yet.
///
/// Verified to fail (decode returns `Err(Truncated)` for 2100/3100) if the
/// guard is reverted to `is_exhausted()`.
#[test]
fn large_symbol_at_eof_not_wrongly_truncated() {
    for &h in &[2100u32, 3100] {
        let w = 200u32;
        let src = make_bitmap(w, h, |x, y| {
            if y < 1024 {
                // First tile: well-mixed hash with no spatial correlation,
                // so JB2's 10-bit context model can't compress it — this is
                // what actually drains the byte buffer.
                let mut s = x
                    .wrapping_mul(374761393)
                    .wrapping_add(y.wrapping_mul(668265263));
                s = (s ^ (s >> 13)).wrapping_mul(1274126177);
                (s ^ (s >> 16)) & 1 == 0
            } else {
                // Solid trailing tiles: tiny compressed, large decoded.
                true
            }
        });
        let encoded = encode_jb2(&src);
        let decoded = jb2::decode(&encoded, None)
            .unwrap_or_else(|e| panic!("{w}x{h} valid page wrongly rejected: {e:?}"));
        assert_eq!((decoded.width, decoded.height), (w, h));
        // Full pixel-exact verification: the page must decode in its
        // entirety (every tile), not be truncated mid-stream.
        for y in 0..h {
            for x in 0..w {
                assert_eq!(
                    decoded.get(x, y),
                    src.get(x, y),
                    "{w}x{h} pixel mismatch at ({x},{y})"
                );
            }
        }
    }
}

#[test]
fn zero_dimension_returns_empty() {
    assert!(encode_jb2(&Bitmap::new(0, 0)).is_empty());
    assert!(encode_jb2(&Bitmap::new(8, 0)).is_empty());
    assert!(encode_jb2(&Bitmap::new(0, 8)).is_empty());
}

/// Round-trip across the 1 MP tile boundary (#198).
/// 2048×2048 = 4 MP forces a 2×2 tile grid (each tile 1024×1024 = 1 MP).
#[test]
fn tiled_2048x2048_roundtrip() {
    let src = make_bitmap(2048, 2048, |x, y| {
        // Pseudo-random pattern that stresses each tile differently.
        ((x.wrapping_mul(2654435761)) ^ y.wrapping_mul(40503)) & 7 == 0
    });
    let encoded = encode_jb2(&src);
    let decoded = jb2::decode(&encoded, None).expect("decode failed");
    assert_eq!(decoded.width, 2048);
    assert_eq!(decoded.height, 2048);
    for y in 0..2048u32 {
        for x in 0..2048u32 {
            assert_eq!(decoded.get(x, y), src.get(x, y), "mismatch at ({x},{y})");
        }
    }
}

/// Tile boundary not on a power-of-two stride — checks edge tiles smaller
/// than 1024 in either axis (#198).
#[test]
fn tiled_irregular_size_roundtrip() {
    let src = make_bitmap(1500, 1100, |x, y| (x * 13 + y * 7) % 11 == 0);
    let encoded = encode_jb2(&src);
    let decoded = jb2::decode(&encoded, None).expect("decode failed");
    assert_eq!(decoded.width, 1500);
    assert_eq!(decoded.height, 1100);
    let mut mismatches = 0u32;
    for y in 0..1100u32 {
        for x in 0..1500u32 {
            if decoded.get(x, y) != src.get(x, y) {
                mismatches += 1;
            }
        }
    }
    assert_eq!(mismatches, 0);
}

/// 1×1 single-pixel image — smallest round-trip case (#198 DoD).
#[test]
fn tiled_1x1_roundtrip() {
    for &px in &[false, true] {
        let src = make_bitmap(1, 1, |_, _| px);
        let encoded = encode_jb2(&src);
        let decoded = jb2::decode(&encoded, None).expect("decode failed");
        assert_eq!(decoded.width, 1);
        assert_eq!(decoded.height, 1);
        assert_eq!(decoded.get(0, 0), px, "1x1 pixel mismatch px={px}");
    }
}

/// 100×100 sub-tile image — single tile, exercise non-trivial geometry (#198 DoD).
#[test]
fn tiled_100x100_roundtrip() {
    let src = make_bitmap(100, 100, |x, y| (x ^ y) & 1 == 0);
    let encoded = encode_jb2(&src);
    let decoded = jb2::decode(&encoded, None).expect("decode failed");
    assert_eq!(decoded.width, 100);
    assert_eq!(decoded.height, 100);
    for y in 0..100u32 {
        for x in 0..100u32 {
            assert_eq!(decoded.get(x, y), src.get(x, y), "mismatch at ({x},{y})");
        }
    }
}

/// Direct tiles above 16 MP decode: every page pixel costs one unit of the
/// decoder's 64 M per-page work budget, so pages up to the 64 MP limit fit.
#[test]
fn tiled_above_16mp_roundtrip() {
    let (w, h) = (5000u32, 4000u32);
    assert!((w * h) as usize > 16 * 1024 * 1024);
    let src = make_bitmap(w, h, |x, y| (x * 7 + y * 3) % 29 == 0);
    let decoded = roundtrip(&src);
    assert_eq!((decoded.width, decoded.height), (w, h));
    assert_eq!(decoded.data, src.data);
}

/// 4096×4096 = 16 MP forces a 4×4 tile grid (#198 DoD).
/// Sparse pattern keeps this test light enough to run in CI.
#[test]
#[ignore = "16 MP pixel-by-pixel verify is slow; enable with --ignored"]
fn tiled_4096x4096_roundtrip() {
    let src = make_bitmap(4096, 4096, |x, y| {
        ((x.wrapping_mul(2654435761)) ^ y.wrapping_mul(40503)) & 31 == 0
    });
    let encoded = encode_jb2(&src);
    let decoded = jb2::decode(&encoded, None).expect("decode failed");
    assert_eq!(decoded.width, 4096);
    assert_eq!(decoded.height, 4096);
    for y in 0..4096u32 {
        for x in 0..4096u32 {
            assert_eq!(decoded.get(x, y), src.get(x, y), "mismatch at ({x},{y})");
        }
    }
}

// ── Dict-based encoder (Phase 1: record types 1 + 7) ──────────────────────

fn roundtrip_dict(bm: &Bitmap) -> Bitmap {
    let encoded = encode_jb2_dict(bm);
    jb2::decode(&encoded, None).expect("dict decode failed")
}

fn assert_bitmaps_eq(a: &Bitmap, b: &Bitmap) {
    assert_eq!(a.width, b.width, "width mismatch");
    assert_eq!(a.height, b.height, "height mismatch");
    let mut mismatches = Vec::new();
    for y in 0..a.height {
        for x in 0..a.width {
            if a.get(x, y) != b.get(x, y) {
                mismatches.push((x, y, a.get(x, y), b.get(x, y)));
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} pixel mismatches: {:?}",
        mismatches.len(),
        mismatches
    );
}

#[test]
fn dict_all_white_roundtrip() {
    let src = Bitmap::new(32, 32);
    let decoded = roundtrip_dict(&src);
    assert_bitmaps_eq(&src, &decoded);
}

#[test]
fn dict_single_pixel_roundtrip() {
    let src = make_bitmap(16, 16, |x, y| x == 4 && y == 7);
    let decoded = roundtrip_dict(&src);
    assert_bitmaps_eq(&src, &decoded);
}

#[test]
fn dict_two_dots_dedup() {
    // Two identical 1-pixel CCs — dict size should be 1.
    let src = make_bitmap(32, 32, |x, y| (x == 3 && y == 5) || (x == 20 && y == 25));
    let decoded = roundtrip_dict(&src);
    assert_bitmaps_eq(&src, &decoded);
    // Assert deduplication happened by checking that the encoded stream
    // is *smaller* than encoding each CC as a fresh record-type-1 would be.
    // Indirect check: re-encode and make sure two CCs exist in the source.
    let ccs = extract_ccs(&src);
    assert_eq!(ccs.len(), 2);
}

#[test]
fn dict_letter_like_shapes() {
    // Two disconnected 3×5 rectangles — should dedup to 1 symbol.
    let src = make_bitmap(32, 32, |x, y| {
        (x < 3 && y < 5) || ((20..23).contains(&x) && (10..15).contains(&y))
    });
    let decoded = roundtrip_dict(&src);
    assert_bitmaps_eq(&src, &decoded);
}

#[test]
fn dict_checkerboard_many_ccs() {
    // 8×8 checkerboard: 32 single-pixel CCs, all identical → 1 dict entry.
    let src = make_bitmap(8, 8, |x, y| (x + y) % 2 == 0);
    let decoded = roundtrip_dict(&src);
    assert_bitmaps_eq(&src, &decoded);
}

#[test]
fn dict_two_different_shapes_multiple_occurrences() {
    // Shape A: 2x2 block.  Shape B: 1x3 vertical line.
    // Four copies of each, interleaved spatially.
    let src = make_bitmap(64, 64, |x, y| {
        // A: (0-1, 0-1), (30-31, 0-1), (0-1, 30-31), (30-31, 30-31)
        let in_a = |ax: u32, ay: u32| x >= ax && x < ax + 2 && y >= ay && y < ay + 2;
        // B: (10, 5-7), (40, 5-7), (10, 45-47), (40, 45-47)
        let in_b = |bx: u32, by: u32| x == bx && y >= by && y < by + 3;
        in_a(0, 0)
            || in_a(30, 0)
            || in_a(0, 30)
            || in_a(30, 30)
            || in_b(10, 5)
            || in_b(40, 5)
            || in_b(10, 45)
            || in_b(40, 45)
    });
    let decoded = roundtrip_dict(&src);
    assert_bitmaps_eq(&src, &decoded);
    let ccs = extract_ccs(&src);
    assert_eq!(ccs.len(), 8, "expected 4+4 CCs");
}

#[test]
fn dict_dimension_encoded_correctly() {
    // Non-multiple-of-8 dimensions stress row-stride handling.
    let src = make_bitmap(13, 7, |x, y| (x * 3 + y) % 5 == 0);
    let decoded = roundtrip_dict(&src);
    assert_bitmaps_eq(&src, &decoded);
}

#[test]
fn dict_zero_dimension_returns_empty() {
    assert!(encode_jb2_dict(&Bitmap::new(0, 0)).is_empty());
    assert!(encode_jb2_dict(&Bitmap::new(8, 0)).is_empty());
    assert!(encode_jb2_dict(&Bitmap::new(0, 8)).is_empty());
}

#[test]
fn dict_extract_ccs_counts() {
    // 3 non-touching black squares.
    let src = make_bitmap(30, 30, |x, y| {
        (x < 3 && y < 3)
            || ((10..13).contains(&x) && (10..13).contains(&y))
            || ((25..28).contains(&x) && (25..28).contains(&y))
    });
    let ccs = extract_ccs(&src);
    assert_eq!(ccs.len(), 3);
    for cc in &ccs {
        assert_eq!(cc.bitmap.width, 3);
        assert_eq!(cc.bitmap.height, 3);
    }
}

#[test]
fn dict_extract_ccs_8connected() {
    // Diagonal pair — 8-connected should merge into 1 CC.
    let src = make_bitmap(4, 4, |x, y| (x == 0 && y == 0) || (x == 1 && y == 1));
    let ccs = extract_ccs(&src);
    assert_eq!(ccs.len(), 1);
    assert_eq!(ccs[0].bitmap.width, 2);
    assert_eq!(ccs[0].bitmap.height, 2);
}

// ── Refinement matching (Phase 3 of #188): record type 6 ─────────────────

#[test]
fn refine_near_duplicate_glyphs_roundtrip() {
    // Two glyph-like shapes with the same bounding box and a 1-pixel diff
    // — well within REFINEMENT_DIFF_FRACTION (10%). The encoder should
    // emit record-1 for the first and record-6 for the second; the
    // decoder must reconstruct each shape exactly at its own location.
    //
    // Shape A: solid 5×5 block.
    // Shape B: same 5×5 block with one pixel flipped (4% diff).
    let src = make_bitmap(40, 12, |x, y| {
        // CC1 at (2, 2)..(7, 7): solid 5×5
        let in_a = (2..7).contains(&x) && (2..7).contains(&y);
        // CC2 at (20, 2)..(25, 7): solid 5×5 with (24, 6) flipped to white
        let in_b = (20..25).contains(&x) && (2..7).contains(&y) && !(x == 24 && y == 6);
        in_a || in_b
    });
    let decoded = roundtrip_dict(&src);
    assert_bitmaps_eq(&src, &decoded);
}

#[test]
fn refine_text_like_repeats_roundtrip() {
    // Six 7×9 "letters" — a plus sign and small variants — laid out in a
    // single row. Same size, low Hamming distance, so the encoder should
    // pick refinement encoding for the variants.
    let src = make_bitmap(80, 12, |x, y| {
        let local_x = x % 12;
        let local_y = y;
        let glyph_idx = x / 12;
        // Base glyph: a plus sign in a 7×9 box.
        let base = (local_x == 3 && (1..8).contains(&local_y))
            || (local_y == 4 && (1..7).contains(&local_x));
        // Each repeat flips one different pixel (introducing a tiny diff).
        let perturbed = match glyph_idx {
            1 => local_x == 0 && local_y == 0,
            2 => local_x == 6 && local_y == 8,
            3 => local_x == 6 && local_y == 0,
            4 => local_x == 0 && local_y == 8,
            _ => false,
        };
        base ^ perturbed
    });
    let decoded = roundtrip_dict(&src);
    assert_bitmaps_eq(&src, &decoded);
}

#[test]
fn refine_far_glyph_falls_back_to_new() {
    // A 5×5 block followed by an unrelated 5×5 X-pattern — Hamming
    // distance ≫ 10%, so refinement matching should *not* fire and the
    // encoder should emit two record-1 entries. Output must still
    // round-trip exactly.
    let src = make_bitmap(40, 12, |x, y| {
        let in_block = (2..7).contains(&x) && (2..7).contains(&y);
        let in_x = (20..25).contains(&x)
            && (2..7).contains(&y)
            && (x - 20 == y - 2 || x - 20 == 6 - (y - 2));
        in_block || in_x
    });
    let decoded = roundtrip_dict(&src);
    assert_bitmaps_eq(&src, &decoded);
}

#[test]
fn refine_packed_hamming_basic() {
    let a = vec![0b1010_1010u8, 0b0000_1111u8];
    let b = vec![0b1010_1011u8, 0b0000_1111u8];
    assert_eq!(packed_hamming(&a, &b), 1);
    let c = vec![0u8; 2];
    let d = vec![0xff; 2];
    assert_eq!(packed_hamming(&c, &d), 16);
}

// ── #322 cross-size record-6 refinement probe ────────────────────────────

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

#[test]
fn cross_size_rec6_probe_off_is_byte_identical() {
    // With the probe disabled (default options), the option-based encoder
    // must reproduce the shipped `encode_jb2_dict` byte stream exactly.
    let src = make_bitmap(80, 40, |x, y| {
        let a = (4..16).contains(&x) && (4..28).contains(&y);
        let b = (40..53).contains(&x) && (4..28).contains(&y);
        a || b
    });
    let shipped = encode_jb2_dict(&src);
    let opt = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
    assert_eq!(shipped, opt, "default options must match shipped output");
}

#[cfg(feature = "experimental")]
#[test]
fn cross_size_rec6_probe_roundtrips_solid_near_twins() {
    // Two solid rectangles differing only in width by one pixel. The first
    // is a fresh rec-1; the second is a cross-size near twin (resampled
    // Hamming 0) so the probe diverts it to a lossless rec-6 refinement.
    let src = make_bitmap(80, 40, |x, y| {
        let a = (4..16).contains(&x) && (4..28).contains(&y); // 12×24 solid
        let b = (40..53).contains(&x) && (4..28).contains(&y); // 13×24 solid
        a || b
    });

    let default_bytes = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
    let probe_bytes = encode_jb2_dict_with_options(&src, &[], &probe_opts());

    // The probe must actually take the refinement path (different bytes)…
    assert_ne!(
        default_bytes, probe_bytes,
        "probe should emit a rec-6 refinement, changing the byte stream"
    );
    // …and stay lossless.
    let decoded = jb2::decode(&probe_bytes, None).expect("probe decode failed");
    assert_bitmaps_eq(&src, &decoded);
}

#[cfg(feature = "experimental")]
#[test]
fn cross_size_rec6_probe_roundtrips_perturbed_glyphs() {
    // A column of near-duplicate near-solid "glyphs": a base block plus a
    // few variants that differ by one bounding-box pixel and a small corner
    // notch — keeping the resampled Hamming distance under the 5% budget so
    // the probe fires, while still exercising non-trivial refinement
    // bitmaps (a handful of differing pixels, not just solid blocks).
    let mut src = Bitmap::new(64, 130);
    let draw_block = |bm: &mut Bitmap, ox: u32, oy: u32, w: u32, h: u32, notch: bool| {
        for y in 0..h {
            for x in 0..w {
                // Solid fill minus a small 2×2 corner notch when requested.
                if notch && x >= w - 2 && y >= h - 2 {
                    continue;
                }
                bm.set(ox + x, oy + y, true);
            }
        }
    };
    draw_block(&mut src, 4, 2, 14, 18, false); // reference, 14×18 solid
    draw_block(&mut src, 4, 24, 15, 18, false); // +1 width, solid
    draw_block(&mut src, 4, 46, 14, 19, true); // +1 height, corner notch
    draw_block(&mut src, 4, 70, 15, 19, true); // +1/+1, corner notch
    draw_block(&mut src, 4, 94, 13, 18, false); // −1 width, solid

    let default_bytes = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
    let probe_bytes = encode_jb2_dict_with_options(&src, &[], &probe_opts());
    assert_ne!(
        default_bytes, probe_bytes,
        "probe should fire on near twins"
    );

    let decoded = jb2::decode(&probe_bytes, None).expect("probe decode failed");
    assert_bitmaps_eq(&src, &decoded);
}

// ── Same-size rec-6 refinement (docs/jb2-size-gap-plan.md Phase A1) ─────────

#[cfg(feature = "experimental")]
fn same_size_opts() -> Jb2EncodeOptions {
    Jb2EncodeOptions {
        same_size_rec6: Some(0.05),
        ..Jb2EncodeOptions::default()
    }
}

#[test]
fn lossy_text_preset_is_lossy_and_smaller() {
    // The lossy_text() preset sets the OCR-validated 0.02 operating point
    // (#572) and, on a page with same-size near-twin glyphs, must produce a
    // strictly smaller stream than the lossless default (it substitutes
    // near-twins as rec-7 copies).
    assert_eq!(Jb2EncodeOptions::lossy_text().lossy_threshold, 0.02);
    assert_eq!(
        Jb2EncodeOptions::with_lossy_threshold(0.07).lossy_threshold,
        0.07
    );

    // A 14×24 solid block plus a same-size near-twin (2×2 corner notch,
    // < 2% of pixels) — the twin is within the 0.02 budget.
    let mut src = Bitmap::new(64, 60);
    for (oy, notch) in [(2u32, false), (30u32, true)] {
        for y in 0..24 {
            for x in 0..14 {
                if notch && x >= 12 && y >= 22 {
                    continue;
                }
                src.set(4 + x, oy + y, true);
            }
        }
    }
    let lossless = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
    let lossy = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::lossy_text());
    assert!(
        lossy.len() < lossless.len(),
        "lossy_text should shrink a near-twin page: {} vs {}",
        lossy.len(),
        lossless.len()
    );
    // Still a valid, decodable stream.
    assert!(
        jb2::decode(&lossy, None).is_ok(),
        "lossy output must decode"
    );
}

// ── Despeckle pre-pass (JB2_DESPECKLE) ───────────────────────────────────

#[test]
fn despeckle_off_is_byte_identical() {
    // With `despeckle` unset (default: None), the encoder must reproduce
    // the shipped `encode_jb2_dict` byte stream exactly, even on a page
    // that contains isolated single-pixel specks (nothing gets filtered).
    let mut src = make_bitmap(80, 40, |x, y| {
        let a = (4..16).contains(&x) && (4..28).contains(&y);
        let b = (40..53).contains(&x) && (4..28).contains(&y);
        a || b
    });
    src.set(70, 5, true); // isolated 1px "speck"
    src.set(75, 35, true); // another isolated 1px "speck"

    let shipped = encode_jb2_dict(&src);
    let opt = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
    assert_eq!(shipped, opt, "default options must match shipped output");

    let opt_explicit_none = encode_jb2_dict_with_options(
        &src,
        &[],
        &Jb2EncodeOptions {
            despeckle: None,
            ..Jb2EncodeOptions::default()
        },
    );
    assert_eq!(shipped, opt_explicit_none);
}

#[test]
fn despeckle_removes_isolated_1px_specks_and_shrinks_output() {
    // A page with one real glyph (14x24 solid block) plus five isolated
    // 1-pixel "dust" specks scattered around it. `despeckle = 2` must
    // drop every speck (pixel_count = 1 <= 2) before they ever become
    // dict entries / coordinate records, shrinking the stream, while the
    // decoded page keeps the real glyph fully intact.
    let mut src = Bitmap::new(64, 40);
    for y in 4..28 {
        for x in 4..18 {
            src.set(x, y, true);
        }
    }
    let specks = [(30u32, 2u32), (35, 10), (40, 20), (50, 5), (55, 30)];
    for &(x, y) in &specks {
        src.set(x, y, true);
    }

    let lossless = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
    let despeckled = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::with_despeckle(2));
    assert!(
        despeckled.len() < lossless.len(),
        "despeckling 1px dust should shrink the stream: despeckled={} lossless={}",
        despeckled.len(),
        lossless.len()
    );

    let decoded = jb2::decode(&despeckled, None).expect("despeckled decode failed");
    assert_eq!(decoded.width, src.width);
    assert_eq!(decoded.height, src.height);
    // Every speck pixel must be gone.
    for &(x, y) in &specks {
        assert!(!decoded.get(x, y), "speck at ({x},{y}) should be removed");
    }
    // The real glyph must be pixel-exact.
    for y in 4..28 {
        for x in 4..18 {
            assert!(
                decoded.get(x, y),
                "glyph pixel ({x},{y}) must survive despeckle"
            );
        }
    }
}

#[test]
fn despeckle_preserves_punctuation_and_diacritic_dots() {
    // Regression guard for the failure mode called out in the task: a
    // despeckle pass that is too aggressive would eat periods, commas,
    // and dots of i/j along with real dust. Build a page with:
    //   - an 'i' stem (3x20) plus its dot (4x4, separated by a gap) —
    //     dots of i/j are small but not dust-sized.
    //   - an isolated period (4x4 solid block, standing alone).
    //   - a single 1x1 dust speck far away — the thing that *should* go.
    // At despeckle=8 (the most aggressive level in the measured sweep,
    // PERF_EXPERIMENTS.md JB2_DESPECKLE), the dot and period (16 px each)
    // must survive (16 > 8) while the 1px speck (1 <= 8) is removed.
    let mut src = Bitmap::new(80, 50);
    // 'i' stem.
    for y in 10..30 {
        for x in 10..13 {
            src.set(x, y, true);
        }
    }
    // 'i' dot: 4x4 block a few pixels above the stem.
    let dot_px: Vec<(u32, u32)> = (4..8).flat_map(|y| (9..13).map(move |x| (x, y))).collect();
    for &(x, y) in &dot_px {
        src.set(x, y, true);
    }
    // Isolated period: 4x4 block, standing alone.
    let period_px: Vec<(u32, u32)> = (40..44)
        .flat_map(|y| (40..44).map(move |x| (x, y)))
        .collect();
    for &(x, y) in &period_px {
        src.set(x, y, true);
    }
    // True dust: a single isolated pixel.
    let speck = (70u32, 45u32);
    src.set(speck.0, speck.1, true);

    for max_px in [2u32, 4, 8] {
        let opts = Jb2EncodeOptions::with_despeckle(max_px);
        let enc = encode_jb2_dict_with_options(&src, &[], &opts);
        let decoded = jb2::decode(&enc, None)
            .unwrap_or_else(|e| panic!("despeckle={max_px} decode failed: {e:?}"));

        for &(x, y) in &dot_px {
            assert!(
                decoded.get(x, y),
                "despeckle={max_px}: i-dot pixel ({x},{y}) must survive"
            );
        }
        for &(x, y) in &period_px {
            assert!(
                decoded.get(x, y),
                "despeckle={max_px}: period pixel ({x},{y}) must survive"
            );
        }
        assert!(
            !decoded.get(speck.0, speck.1),
            "despeckle={max_px}: 1px dust speck must be removed"
        );
    }
}

#[test]
fn lossy_scan_preset_values() {
    let preset = Jb2EncodeOptions::lossy_scan();
    assert_eq!(preset.despeckle, Some(8));
    assert_eq!(preset.lossy_threshold, 0.06);
}

#[test]
fn same_size_rec6_off_is_byte_identical() {
    // With `same_size_rec6` unset (default), the encoder must reproduce the
    // shipped `encode_jb2_dict` byte stream exactly.
    let src = make_bitmap(80, 40, |x, y| {
        let a = (4..16).contains(&x) && (4..28).contains(&y);
        let b = (40..53).contains(&x) && (4..28).contains(&y);
        a || b
    });
    let shipped = encode_jb2_dict(&src);
    let opt = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
    assert_eq!(shipped, opt, "default options must match shipped output");
}

#[cfg(feature = "experimental")]
#[test]
fn same_size_rec6_roundtrips_near_twins() {
    // Two same-bounding-box near-twin glyphs: a 14×24 solid block and a copy
    // with a small 2×2 corner notch (same bbox, a few flipped pixels, well
    // under 5%). The first is a fresh rec-1; the second has a same-size twin,
    // so it diverts to a lossless same-size rec-6 refinement (wdiff=hdiff=0).
    let mut src = Bitmap::new(64, 60);
    let draw = |bm: &mut Bitmap, ox: u32, oy: u32, notch: bool| {
        for y in 0..24 {
            for x in 0..14 {
                if notch && x >= 12 && y >= 22 {
                    continue;
                }
                bm.set(ox + x, oy + y, true);
            }
        }
    };
    draw(&mut src, 4, 2, false); // reference
    draw(&mut src, 4, 30, true); // same-size near twin (2×2 notch)

    let default_bytes = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
    let same_bytes = encode_jb2_dict_with_options(&src, &[], &same_size_opts());

    // The same-size path must actually fire (different byte stream)…
    assert_ne!(
        default_bytes, same_bytes,
        "same_size_rec6 should emit a rec-6 refinement, changing the stream"
    );
    // …and stay lossless (round-trip pixel-exact).
    let decoded = jb2::decode(&same_bytes, None).expect("same-size decode failed");
    assert_bitmaps_eq(&src, &decoded);
}

// ── JB2_AUTO_REC6: adaptive same-size rec-6 auto-policy ─────────────────────

#[cfg(feature = "experimental")]
fn dense_near_twin_bitmap() -> Bitmap {
    // Ten glyph pairs, each a solid (14+row)x24 block plus a same-size
    // near-twin with a small 2x2 corner notch (well under the 5% Hamming
    // budget). The width varies per row so each row's reference is a
    // distinct fresh symbol (no cross-row exact-match collisions collapsing
    // repeats into rec-7 copies) while each row's twin only near-matches
    // its own row's same-size reference. Every pair contributes one fresh
    // reference (no dict twin yet) and one fresh near-twin, so the
    // population density is 50% — far above
    // SAME_SIZE_REC6_AUTO_DENSITY_THRESHOLD.
    let mut src = Bitmap::new(600, 400);
    let draw = |bm: &mut Bitmap, ox: u32, oy: u32, w: u32, notch: bool| {
        for y in 0..24 {
            for x in 0..w {
                if notch && x + 2 >= w && y >= 22 {
                    continue;
                }
                bm.set(ox + x, oy + y, true);
            }
        }
    };
    for row in 0..10u32 {
        let w = 14 + row;
        draw(&mut src, 4, 2 + row * 28, w, false); // fresh reference
        draw(&mut src, 4 + w + 6, 2 + row * 28, w, true); // fresh near-twin (same w,h)
    }
    src
}

#[cfg(feature = "experimental")]
fn sparse_no_twin_bitmap() -> Bitmap {
    // Ten distinct-size solid blocks, each a different (w, h) so none has
    // a same-size dictionary twin to score against: density = 0%.
    let mut src = Bitmap::new(400, 400);
    for row in 0..10u32 {
        let w = 8 + row;
        let h = 10 + row;
        for y in 0..h {
            for x in 0..w {
                src.set(4 + x, 2 + row * 20 + y, true);
            }
        }
    }
    src
}

#[cfg(feature = "experimental")]
#[test]
fn same_size_rec6_auto_fires_on_dense_near_twins() {
    let src = dense_near_twin_bitmap();
    let density = probe_same_size_rec6_density(&src, &[], SAME_SIZE_REC6_AUTO_SAMPLE_CCS);
    assert!(
        density >= SAME_SIZE_REC6_AUTO_DENSITY_THRESHOLD,
        "dense synthetic input should clear the auto-policy threshold: density={density}"
    );

    let opts = Jb2EncodeOptions::same_size_rec6_auto(&src, &[]);
    assert_eq!(
        opts.same_size_rec6,
        Some(SAME_SIZE_REC6_AUTO_FRAC),
        "auto-policy must enable same_size_rec6 on dense near-twin input"
    );

    // Firing must actually change (shrink or resize) the byte stream
    // relative to the lossless default and stay round-trip exact.
    let default_bytes = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
    let auto_bytes = encode_jb2_dict_with_options(&src, &[], &opts);
    assert_ne!(
        default_bytes, auto_bytes,
        "auto policy should divert near-twins to rec-6, changing the stream"
    );
    let decoded = jb2::decode(&auto_bytes, None).expect("auto-policy decode failed");
    assert_bitmaps_eq(&src, &decoded);
}

#[cfg(feature = "experimental")]
#[test]
fn same_size_rec6_auto_stays_off_on_sparse_input() {
    let src = sparse_no_twin_bitmap();
    let density = probe_same_size_rec6_density(&src, &[], SAME_SIZE_REC6_AUTO_SAMPLE_CCS);
    assert!(
        density < SAME_SIZE_REC6_AUTO_DENSITY_THRESHOLD,
        "sparse synthetic input should stay below the auto-policy threshold: density={density}"
    );

    let opts = Jb2EncodeOptions::same_size_rec6_auto(&src, &[]);
    assert_eq!(
        opts.same_size_rec6, None,
        "auto-policy must leave same_size_rec6 off on sparse input"
    );

    // Output must be byte-identical to the default encoder.
    let default_bytes = encode_jb2_dict_with_options(&src, &[], &Jb2EncodeOptions::default());
    let auto_bytes = encode_jb2_dict_with_options(&src, &[], &opts);
    assert_eq!(
        default_bytes, auto_bytes,
        "auto policy must stay off (byte-identical) on sparse input"
    );
}

#[cfg(feature = "experimental")]
#[test]
fn probe_same_size_rec6_density_bounds_the_scan() {
    // Bounding the probe to the first `fresh_cc_limit` fresh CCs must not
    // change the *decision* on data where the near-twin ratio is uniform
    // throughout (each pair independently contributes 1 fresh + 1 near
    // twin): scanning only the first pair already yields the same ~50%
    // density as scanning all ten.
    let src = dense_near_twin_bitmap();
    let full = same_size_refinement_scan(&src, &[], None);
    let bounded = same_size_refinement_scan(&src, &[], Some(2));
    assert!(
        bounded.fresh_ccs <= 2,
        "bounded scan must stop at the fresh-CC cap: got {}",
        bounded.fresh_ccs
    );
    assert!(
        full.fresh_ccs > bounded.fresh_ccs,
        "unbounded scan should see strictly more fresh CCs than the capped one"
    );
    // Both scans see a 50% density on this uniform-pair synthetic input.
    let full_density = full.near_le_5pct as f32 / full.fresh_ccs as f32;
    let bounded_density = bounded.near_le_5pct as f32 / bounded.fresh_ccs as f32;
    assert!((full_density - bounded_density).abs() < 1e-6);
}

// ── #194 multi-page shared Djbz ────────────────────────────────────────────

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
fn djbz_roundtrip_two_glyphs() {
    // Encode two distinct glyph bitmaps as a Djbz, decode it, and verify
    // the resulting Jb2Dict has exactly those two symbols in order.
    let mut a = Bitmap::new(4, 5);
    render_glyph(&mut a, 0, 0, &glyph_a());
    let mut b = Bitmap::new(4, 5);
    render_glyph(&mut b, 0, 0, &glyph_b());
    let djbz = encode_jb2_djbz(&[a.clone(), b.clone()]);
    assert!(!djbz.is_empty());

    // Sanity-decode by constructing a Sjbz that uses the shared dict
    // and checking the two glyphs round-trip.
    let dict = jb2::decode_dict(&djbz, None).expect("decode_dict");
    // Use the shared dict in a 1-page Sjbz that places both glyphs.
    let mut page = Bitmap::new(20, 8);
    render_glyph(&mut page, 2, 2, &glyph_a());
    render_glyph(&mut page, 10, 2, &glyph_b());
    let sjbz = encode_jb2_dict_with_shared(&page, &[a, b]);
    let decoded = jb2::decode(&sjbz, Some(&dict)).expect("decode");
    assert_decoded_eq(&page, &decoded);
}

#[test]
fn cluster_promotes_only_repeated_glyphs() {
    // A appears on both pages, B appears on only one. With threshold=2,
    // only A should be promoted.
    let mut p1 = Bitmap::new(20, 10);
    render_glyph(&mut p1, 2, 2, &glyph_a());
    render_glyph(&mut p1, 10, 2, &glyph_b());
    let mut p2 = Bitmap::new(20, 10);
    render_glyph(&mut p2, 2, 2, &glyph_a());
    // No B on page 2.

    let shared = cluster_shared_symbols(&[p1, p2], 2);
    assert_eq!(shared.len(), 1, "only A should cross the threshold");
    // A glyph is 4×5.
    assert_eq!(shared[0].width, 4);
    assert_eq!(shared[0].height, 5);
}

fn glyph_box8() -> Vec<&'static [u8]> {
    vec![
        b"########" as &[u8],
        b"#      #" as &[u8],
        b"#      #" as &[u8],
        b"#      #" as &[u8],
        b"#      #" as &[u8],
        b"#      #" as &[u8],
        b"#      #" as &[u8],
        b"########" as &[u8],
    ]
}

#[test]
fn cluster_tunable_keeps_near_duplicate_large_glyphs_separate() {
    // Hamming clustering was rejected for #258. The tunable API remains
    // available for benchmark compatibility, but all thresholds now use
    // byte-exact clustering.
    //
    // Use box outlines with one outline-pixel removed (so the noise alters
    // the same CC instead of producing a stray 1-pixel CC).
    let mut p1 = Bitmap::new(20, 20);
    render_glyph(&mut p1, 4, 4, &glyph_box8());
    p1.set(5, 4, false); // notch the top edge at x=5
    let mut p2 = Bitmap::new(20, 20);
    render_glyph(&mut p2, 4, 4, &glyph_box8());
    p2.set(6, 4, false); // notch at x=6 instead — different bit

    let shared = cluster_shared_symbols_tunable(&[p1.clone(), p2.clone()], 2, 4);
    assert!(
        shared.is_empty(),
        "tunable clustering must not promote noisy near-dupes"
    );

    // Default (byte-exact) keeps them separate — neither passes
    // page_threshold=2 since each variant only appears on one page.
    let shared_exact = cluster_shared_symbols(&[p1, p2], 2);
    assert!(
        shared_exact.is_empty(),
        "byte-exact default must not promote noisy near-dupes"
    );
}

#[test]
fn lossy_threshold_substitutes_near_duplicate_with_rec7() {
    // Three 6×6 CCs on one page:
    //   - "base" solid block (1st CC → rec-1, becomes dict entry 0)
    //   - "near_dup" solid block with one pixel off (Hamming = 1)
    //   - "another_near_dup" solid block with a different pixel off
    //     (Hamming = 1 from base, Hamming = 2 from near_dup)
    //
    // Lossless (threshold = 0) → 2 rec-6 refinements.
    // Lossy (threshold = 0.05 ≈ 2 pixels of 36) → 2 rec-7 copies of base.
    // Lossy bytes < lossless bytes (rec-7 is smaller — no refinement bitmap).
    let base = make_bitmap(6, 6, |_, _| true);
    let near_dup = make_bitmap(6, 6, |x, y| !(x == 3 && y == 3));
    let another = make_bitmap(6, 6, |x, y| !(x == 1 && y == 4));

    let stamp = |page: &mut Bitmap, ox: u32, oy: u32, src: &Bitmap| {
        for y in 0..src.height {
            for x in 0..src.width {
                if src.get(x, y) {
                    page.set(ox + x, oy + y, true);
                }
            }
        }
    };
    let mut page = make_bitmap(40, 12, |_, _| false);
    stamp(&mut page, 2, 2, &base);
    stamp(&mut page, 14, 2, &near_dup);
    stamp(&mut page, 26, 2, &another);

    let lossless = encode_jb2_dict_with_options(
        &page,
        &[],
        &Jb2EncodeOptions {
            lossy_threshold: 0.0,
            ..Jb2EncodeOptions::default()
        },
    );
    let lossy = encode_jb2_dict_with_options(
        &page,
        &[],
        &Jb2EncodeOptions {
            lossy_threshold: 0.05,
            ..Jb2EncodeOptions::default()
        },
    );

    assert!(
        lossy.len() < lossless.len(),
        "lossy should be smaller than lossless: lossy={} lossless={}",
        lossy.len(),
        lossless.len()
    );

    // Lossy output decodes; the decoded near-duplicate CCs should now
    // be byte-identical to `base` (not to their original perturbed
    // pixels — that's the deliberate visual loss).
    let decoded = jb2::decode(&lossy, None).expect("lossy decode");
    assert_eq!(decoded.width, page.width);
    assert_eq!(decoded.height, page.height);

    // The first CC region (base) is unchanged. The second/third
    // regions used to have one missing pixel each; in lossy mode the
    // decoder fills them in (the substitute rec-7 references the
    // solid base).
    //
    // Sanity: original page is missing pixel at (14+3, 2+3) = (17, 5)
    // and at (26+1, 2+4) = (27, 6). The lossy decode should have those
    // pixels set (because rec-7 copied the solid `base`).
    assert!(
        decoded.get(17, 5),
        "lossy decode should fill base at (17,5)"
    );
    assert!(
        decoded.get(27, 6),
        "lossy decode should fill base at (27,6)"
    );

    // Lossless decode preserves the holes faithfully.
    let decoded_lossless = jb2::decode(&lossless, None).expect("lossless decode");
    assert!(
        !decoded_lossless.get(17, 5),
        "lossless should preserve hole at (17,5)"
    );
    assert!(
        !decoded_lossless.get(27, 6),
        "lossless should preserve hole at (27,6)"
    );
}

#[test]
fn analyze_jb2_cc_stats_classifies_records() {
    // Three CCs on one page, each well-separated:
    //   1. solid 6×6 block             → byte-exact match against shared (rec-7)
    //   2. solid 6×6 minus one pixel   → rec-1; shared rec-6 is disabled
    //   3. solid 5×5 block             → unrelated, no same-size match   (rec-1)
    //
    // REFINEMENT_MIN_PIXELS = 32 forces the 5×5 path through rec-1 even
    // if the dict had a same-size entry. The 6×6 entries (36 pixels each)
    // would otherwise be eligible for rec-6 against the shared dict.
    let shared_glyph = make_bitmap(6, 6, |_, _| true);
    let near_dup = make_bitmap(6, 6, |x, y| !(x == 3 && y == 3));
    let unrelated = make_bitmap(5, 5, |_, _| true);

    let stamp = |page: &mut Bitmap, ox: u32, oy: u32, src: &Bitmap| {
        for y in 0..src.height {
            for x in 0..src.width {
                if src.get(x, y) {
                    page.set(ox + x, oy + y, true);
                }
            }
        }
    };
    let mut page = make_bitmap(40, 12, |_, _| false);
    stamp(&mut page, 2, 2, &shared_glyph);
    stamp(&mut page, 14, 2, &near_dup);
    stamp(&mut page, 26, 2, &unrelated);

    let stats = analyze_jb2_cc_stats(&page, &[shared_glyph]);
    assert_eq!(stats.rec_7_exact, 1, "expected one byte-exact rec-7 hit");
    assert_eq!(
        stats.rec_6_refine_shared, 0,
        "shared-dict near matches must not use rec-6"
    );
    assert_eq!(stats.rec_6_refine_local, 0);
    assert!(
        stats.rec_1_new >= 2,
        "expected near shared hit and unrelated CC to use rec-1 (got {})",
        stats.rec_1_new
    );
    assert!(stats.rec_6_hamming.is_empty());
    assert!(stats.pixels_rec_7 > 0);
    assert_eq!(stats.pixels_rec_6, 0);
    assert!(stats.pixels_rec_1 > 0);
    assert_eq!(
        stats.total_ccs,
        stats.rec_1_new + stats.rec_6_refine_local + stats.rec_6_refine_shared + stats.rec_7_exact
    );
}

#[cfg(feature = "experimental")]
#[test]
fn analyze_cross_size_refinement_counts_near_size_candidates() {
    let shared_glyph = make_bitmap(6, 6, |_, _| true);
    let taller_near = make_bitmap(6, 7, |_, _| true);
    let unrelated = make_bitmap(12, 12, |x, y| x == y);

    let stamp = |page: &mut Bitmap, ox: u32, oy: u32, src: &Bitmap| {
        for y in 0..src.height {
            for x in 0..src.width {
                if src.get(x, y) {
                    page.set(ox + x, oy + y, true);
                }
            }
        }
    };
    let mut page = make_bitmap(40, 16, |_, _| false);
    stamp(&mut page, 2, 2, &shared_glyph);
    stamp(&mut page, 14, 2, &taller_near);
    stamp(&mut page, 26, 2, &unrelated);

    let stats = analyze_jb2_cross_size_refinement(&page, &[shared_glyph], 1, 0.05);
    assert_eq!(stats.near_matches, 1);
    assert_eq!(stats.near_match_pixels, 42);
    assert!(stats.estimated_rec1_bytes > 0);
    assert!(stats.estimated_cross_size_rec6_bytes > 0);
    assert!(
        stats.candidate_ccs >= stats.near_matches,
        "near matches must be a subset of cross-size candidates"
    );
}

/// Regression for #270: a clustered shared dict whose total symbol pixels
/// would exceed `MAX_TOTAL_SYMBOL_PIXELS` must be trimmed at clustering
/// time so the produced `Djbz` round-trips through `decode_dict`.
#[test]
fn cluster_shared_symbols_caps_total_pixel_budget() {
    let cap = SHARED_DICT_PIXEL_BUDGET;

    // Build many distinct same-size CCs, then stamp each twice (across two
    // pages) so they all promote at threshold 2. Sum > cap.
    let glyph_w: u32 = 96;
    let glyph_h: u32 = 96;
    let pixels_per_glyph = (glyph_w as usize) * (glyph_h as usize);
    let n_glyphs = (cap / pixels_per_glyph) + 64; // overshoot the cap

    let glyphs: Vec<Bitmap> = (0..n_glyphs)
        .map(|i| {
            make_bitmap(glyph_w, glyph_h, |x, y| {
                // Per-glyph pseudo-random pattern; ensures buckets are
                // populated by distinct (w, h, data) reps.
                let v = (x.wrapping_mul(2654435761) ^ y.wrapping_mul(40503)).wrapping_add(i as u32);
                (v & 0xff) < 128
            })
        })
        .collect();

    let page_w: u32 = 1024;
    let make_page = |start: usize, count: usize| -> Bitmap {
        // Pack glyphs in rows; canvas grows just enough to fit.
        let cols = (page_w / (glyph_w + 2)).max(1) as usize;
        let rows = count.div_ceil(cols);
        let canvas_h = (rows as u32) * (glyph_h + 2) + 2;
        let mut canvas = Bitmap::new(page_w, canvas_h);
        for (i, g) in glyphs[start..start + count].iter().enumerate() {
            let col = (i % cols) as u32;
            let row = (i / cols) as u32;
            let ox = col * (glyph_w + 2) + 1;
            let oy = row * (glyph_h + 2) + 1;
            for y in 0..glyph_h {
                for x in 0..glyph_w {
                    if g.get(x, y) {
                        canvas.set(ox + x, oy + y, true);
                    }
                }
            }
        }
        canvas
    };
    let p1 = make_page(0, n_glyphs);
    let p2 = make_page(0, n_glyphs);

    let shared = cluster_shared_symbols_tunable(&[p1, p2], 2, 0);
    let total: usize = shared
        .iter()
        .map(|s| (s.width as usize) * (s.height as usize))
        .sum();
    assert!(
        total <= cap,
        "cluster output {total} px must respect MAX_TOTAL_SYMBOL_PIXELS={cap}"
    );

    let djbz = encode_jb2_djbz(&shared);
    crate::decode_dict(&djbz, None)
        .expect("encoded shared Djbz must round-trip through decode_dict");
}

/// `encode_bitmap_direct`, which codes white runs in one call, emits the same
/// bytes and contexts as a plain per-pixel loop over the 10-pixel context.
#[test]
fn encode_bitmap_direct_matches_per_pixel_loop() {
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    for (w, h, sparsity) in [
        (1u32, 1u32, 2u64),
        (5, 3, 3),
        (8, 4, 5),
        (13, 9, 7),
        (64, 20, 40),
        (100, 37, 200),
        (257, 31, 1000),
        (1024, 6, 100_000),
    ] {
        let mut bm = Bitmap::new(w, h);
        for y in 0..h {
            for x in 0..w {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                bm.set(x, y, seed.is_multiple_of(sparsity));
            }
        }
        let mut fast_zp = ZpEncoder::new();
        let mut fast_ctx = vec![0u8; 1024];
        encode_bitmap_direct(&mut fast_zp, &mut fast_ctx, &bm);

        let px = |x: i32, y: i32| -> u32 {
            u32::from(x >= 0 && y >= 0 && x < w as i32 && bm.get(x as u32, y as u32))
        };
        let mut slow_zp = ZpEncoder::new();
        let mut slow_ctx = vec![0u8; 1024];
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let r2 = px(x - 1, y - 2) << 2 | px(x, y - 2) << 1 | px(x + 1, y - 2);
                let r1 = px(x - 2, y - 1) << 4
                    | px(x - 1, y - 1) << 3
                    | px(x, y - 1) << 2
                    | px(x + 1, y - 1) << 1
                    | px(x + 2, y - 1);
                let r0 = px(x - 2, y) << 1 | px(x - 1, y);
                let idx = (r2 << 7 | r1 << 2 | r0) as usize;
                slow_zp.encode_bit(&mut slow_ctx[idx], px(x, y) != 0);
            }
        }
        assert_eq!(fast_ctx, slow_ctx, "{w}x{h}");
        assert_eq!(fast_zp.finish(), slow_zp.finish(), "{w}x{h}");
    }
}

/// `extract_ccs` (packed-bit scan) finds the same components, in the same
/// order and with the same pixels, as a per-pixel raster scan with the same
/// DFS; row padding bits never become ink.
#[test]
fn extract_ccs_matches_per_pixel_scan() {
    let mut seed = 0x0123_4567_89ab_cdefu64;
    for (w, h, sparsity) in [
        (1u32, 1u32, 2u64),
        (7, 5, 2),
        (9, 9, 3),
        (64, 17, 5),
        (70, 40, 9),
        (131, 23, 40),
        (300, 11, 400),
    ] {
        let mut bm = Bitmap::new(w, h);
        for y in 0..h {
            for x in 0..w {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                bm.set(x, y, seed.is_multiple_of(sparsity));
            }
        }
        // Junk in the padding bits past `w` must be ignored.
        let stride = bm.row_stride();
        if !w.is_multiple_of(8) {
            for y in 0..h as usize {
                bm.data[y * stride + stride - 1] |= 0xFF >> (w % 8);
            }
        }

        let (wu, hu) = (w as usize, h as usize);
        let mut seen = vec![false; wu * hu];
        let mut expected = Vec::new();
        for y0 in 0..hu {
            for x0 in 0..wu {
                if seen[y0 * wu + x0] || !bm.get(x0 as u32, y0 as u32) {
                    continue;
                }
                seen[y0 * wu + x0] = true;
                let mut stack = vec![(x0, y0)];
                let mut pixels = Vec::new();
                while let Some((cx, cy)) = stack.pop() {
                    pixels.push((cx, cy));
                    for ny in cy.saturating_sub(1)..=(cy + 1).min(hu - 1) {
                        for nx in cx.saturating_sub(1)..=(cx + 1).min(wu - 1) {
                            if !seen[ny * wu + nx] && bm.get(nx as u32, ny as u32) {
                                seen[ny * wu + nx] = true;
                                stack.push((nx, ny));
                            }
                        }
                    }
                }
                let min_x = pixels.iter().map(|p| p.0).min().unwrap();
                let min_y = pixels.iter().map(|p| p.1).min().unwrap();
                expected.push((min_x as u32, min_y as u32, pixels));
            }
        }

        let ccs = extract_ccs(&bm);
        assert_eq!(ccs.len(), expected.len(), "{w}x{h}");
        for (cc, (x, y, pixels)) in ccs.iter().zip(&expected) {
            assert_eq!((cc.x, cc.y), (*x, *y), "{w}x{h}");
            assert_eq!(cc.pixel_count as usize, pixels.len(), "{w}x{h}");
            for &(px, py) in pixels {
                assert!(cc.bitmap.get(px as u32 - x, py as u32 - y), "{w}x{h}");
            }
            let ink = (0..cc.bitmap.height)
                .flat_map(|yy| (0..cc.bitmap.width).map(move |xx| (xx, yy)))
                .filter(|&(xx, yy)| cc.bitmap.get(xx, yy))
                .count();
            assert_eq!(ink, pixels.len(), "{w}x{h}");
        }
    }
}
