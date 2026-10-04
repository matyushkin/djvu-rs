//! Connected-component extraction, splitting and hashing.

use super::*;

/// A single connected component: its cropped bitmap and top-left bbox origin.
pub(super) struct Cc {
    /// Top-left x of the component in the source bitmap (0 = left edge).
    pub(super) x: u32,
    /// Top-left y of the component in the source bitmap (0 = top edge).
    pub(super) y: u32,
    /// Cropped bitmap: tight bbox, pixels of this component only.
    pub(super) bitmap: Bitmap,
    /// Count of black (foreground) pixels making up this component — the
    /// true "ink" area, unlike `bitmap.width * bitmap.height` (the bbox
    /// area, which overcounts for thin diagonal strokes). Used by
    /// [`Jb2EncodeOptions::despeckle`] to size-filter noise blobs before
    /// they ever reach clustering/dedup.
    pub(super) pixel_count: u32,
}

/// Extract all 8-connected components of black pixels from `bitmap`.
///
/// Uses iterative DFS on an unpacked byte grid; each component's cropped
/// bitmap is the minimal bounding box that contains its black pixels.
/// Ordering is raster-scan of the seed pixel (roughly top-to-bottom,
/// left-to-right).
pub(super) fn extract_ccs(bitmap: &Bitmap) -> Vec<Cc> {
    let w = bitmap.width as usize;
    let h = bitmap.height as usize;
    if w == 0 || h == 0 {
        return Vec::new();
    }

    // Pixels not yet visited, one bit each in the packed layout (MSB-first,
    // `stride` bytes per row): a DFS clears a pixel's bit when it pushes it.
    // Scanning packed words skips 64 white pixels at a time and needs no
    // byte-per-pixel copy of the page. Row padding bits are cleared so they
    // never seed a component.
    let stride = bitmap.row_stride();
    let mut bits = bitmap.data[..stride * h].to_vec();
    if !w.is_multiple_of(8) {
        let pad_mask = 0xFFu8 << (8 - w % 8);
        for row in bits.chunks_exact_mut(stride) {
            row[stride - 1] &= pad_mask;
        }
    }
    let mask = |x: usize| 0x80u8 >> (x & 7);

    let mut out = Vec::new();
    let mut stack: Vec<(u32, u32)> = Vec::new();
    let mut cc_pixels: Vec<(u32, u32)> = Vec::new();

    for y0 in 0..h {
        let row0 = y0 * stride;
        let mut bi = 0;
        while bi < stride {
            // Skip eight white bytes at a time.
            if bi + 8 <= stride
                && u64::from_ne_bytes(bits[row0 + bi..row0 + bi + 8].try_into().expect("8 bytes"))
                    == 0
            {
                bi += 8;
                continue;
            }
            let byte = bits[row0 + bi];
            if byte == 0 {
                bi += 1;
                continue;
            }
            // Leftmost unvisited pixel: the DFS only clears bits, so this is
            // the raster-order seed a per-pixel scan would find next.
            let x0 = bi * 8 + byte.leading_zeros() as usize;
            stack.clear();
            cc_pixels.clear();
            stack.push((x0 as u32, y0 as u32));
            bits[row0 + x0 / 8] &= !mask(x0);

            let mut min_x = x0;
            let mut max_x = x0;
            let mut min_y = y0;
            let mut max_y = y0;

            while let Some((cx, cy)) = stack.pop() {
                cc_pixels.push((cx, cy));
                let cxi = cx as usize;
                let cyi = cy as usize;
                if cxi < min_x {
                    min_x = cxi;
                }
                if cxi > max_x {
                    max_x = cxi;
                }
                if cyi < min_y {
                    min_y = cyi;
                }
                if cyi > max_y {
                    max_y = cyi;
                }

                let lo_x = cxi.saturating_sub(1);
                let hi_x = (cxi + 1).min(w - 1);
                let lo_y = cyi.saturating_sub(1);
                let hi_y = (cyi + 1).min(h - 1);
                for ny in lo_y..=hi_y {
                    let row_base = ny * stride;
                    for nx in lo_x..=hi_x {
                        let b = &mut bits[row_base + nx / 8];
                        if *b & mask(nx) != 0 {
                            *b &= !mask(nx);
                            stack.push((nx as u32, ny as u32));
                        }
                    }
                }
            }

            let cc_w = (max_x - min_x + 1) as u32;
            let cc_h = (max_y - min_y + 1) as u32;
            if (cc_w as usize) * (cc_h as usize) > crate::MAX_SYMBOL_PIXELS {
                split_cc(
                    &cc_pixels,
                    min_x as u32,
                    min_y as u32,
                    cc_w,
                    SPLIT_TILE,
                    &mut out,
                );
                continue;
            }
            let mut cc_bm = Bitmap::new(cc_w, cc_h);
            for &(px, py) in &cc_pixels {
                cc_bm.set(px - min_x as u32, py - min_y as u32, true);
            }
            out.push(Cc {
                x: min_x as u32,
                y: min_y as u32,
                bitmap: cc_bm,
                pixel_count: cc_pixels.len() as u32,
            });
        }
    }

    // Bounding boxes of nested components overlap, so on a page that is
    // mostly ink their areas can add up past the decoder's per-page budget
    // even after the split above. Cutting the large components finer crops
    // away the holes that hold other components. Only such pages change.
    let bbox_total = out.iter().fold(0usize, |t, cc| {
        t.saturating_add((cc.bitmap.width as usize) * (cc.bitmap.height as usize))
    });
    if bbox_total > crate::MAX_PAGE_SYMBOL_WORK {
        let mut fine = Vec::with_capacity(out.len());
        for cc in out {
            let (bw, bh) = (cc.bitmap.width, cc.bitmap.height);
            if bw.max(bh) <= FINE_SPLIT_TILE {
                fine.push(cc);
                continue;
            }
            cc_pixels.clear();
            for y in 0..bh {
                for x in 0..bw {
                    if cc.bitmap.get(x, y) {
                        cc_pixels.push((cc.x + x, cc.y + y));
                    }
                }
            }
            split_cc(&cc_pixels, cc.x, cc.y, bw, FINE_SPLIT_TILE, &mut fine);
        }
        out = fine;
    }

    out
}

/// Side of the square pieces cut from a component over the decoder's
/// per-symbol limit: 4096² = 16 MP, the limit itself (the decoder rejects
/// only symbols *above* it).
pub(super) const SPLIT_TILE: u32 = 4096;

/// Side of the pieces cut from large components when the page's bounding
/// boxes add up past the decoder's per-page budget.
pub(super) const FINE_SPLIT_TILE: u32 = 1024;

/// Append the pixels of one component as several pieces, one per `tile`-sized
/// grid cell it touches. Each piece is cropped to its own ink, so it stays
/// tight like a real component; the pieces do not overlap and together hold
/// exactly the component's pixels. Without this a page-wide component (a
/// scan's dark border, say) made the whole stream undecodable.
pub(super) fn split_cc(
    pixels: &[(u32, u32)],
    min_x: u32,
    min_y: u32,
    cc_w: u32,
    tile: u32,
    out: &mut Vec<Cc>,
) {
    let tiles_x = cc_w.div_ceil(tile) as usize;
    let tile_of = |(px, py): (u32, u32)| {
        ((py - min_y) / tile) as usize * tiles_x + ((px - min_x) / tile) as usize
    };
    /// One grid cell's ink bbox (x0, y0, x1, y1) and pixel count.
    type Cell = (u32, u32, u32, u32, u32);
    let mut cells: Vec<Option<Cell>> = Vec::new();
    for &p in pixels {
        let t = tile_of(p);
        if t >= cells.len() {
            cells.resize(t + 1, None);
        }
        let (px, py) = p;
        cells[t] = Some(match cells[t] {
            None => (px, py, px, py, 1),
            Some((x0, y0, x1, y1, n)) => (x0.min(px), y0.min(py), x1.max(px), y1.max(py), n + 1),
        });
    }
    let first = out.len();
    let mut slot = vec![usize::MAX; cells.len()];
    for (t, cell) in cells.iter().enumerate() {
        if let Some((x0, y0, x1, y1, n)) = *cell {
            slot[t] = out.len();
            out.push(Cc {
                x: x0,
                y: y0,
                bitmap: Bitmap::new(x1 - x0 + 1, y1 - y0 + 1),
                pixel_count: n,
            });
        }
    }
    debug_assert!(out.len() > first);
    for &p in pixels {
        let cc = &mut out[slot[tile_of(p)]];
        cc.bitmap.set(p.0 - cc.x, p.1 - cc.y, true);
    }
}

/// Hamming distance between two equal-sized packed bitmap byte buffers.
pub(super) fn packed_hamming(a: &[u8], b: &[u8]) -> u32 {
    debug_assert_eq!(a.len(), b.len());
    let mut total: u32 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        total += (x ^ y).count_ones();
    }
    total
}

/// FNV-1a hash of a symbol's `(w, h, packed-data)`, used as the bucket key for
/// exact-match dedup. Replaces a `BTreeMap` keyed by `(u32, u32, Vec<u8>)`, which
/// cloned the bitmap data on every connected-component lookup; the hash buckets
/// (`BTreeMap<u64, Vec<usize>>`) compare the actual data only on a hash hit, so
/// dedup stays byte-identical while avoiding the per-CC allocation.
#[inline]
pub(super) fn symbol_hash(w: u32, h: u32, data: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut mix = |bytes: &[u8]| {
        for &b in bytes {
            hash = (hash ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    mix(&w.to_le_bytes());
    mix(&h.to_le_bytes());
    mix(data);
    hash
}
