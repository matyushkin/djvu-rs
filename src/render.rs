use crate::bitmap::Bitmap;
use crate::document::{Page, Palette, Rotation};
use crate::error::Error;
use crate::pixmap::Pixmap;

// ── Gamma correction ────────────────────────────────────────────────────────

/// Build a 256-entry gamma correction LUT.
///
/// `lut[i] = round(255 * (i/255)^(1/gamma))`
///
/// Returns identity when `gamma` is ≤ 0, non-finite, or ≈ 1.0.
fn build_gamma_lut(gamma: f32) -> [u8; 256] {
    let mut lut = [0u8; 256];
    if gamma <= 0.0 || !gamma.is_finite() || (gamma - 1.0).abs() < 1e-4 {
        for (i, v) in lut.iter_mut().enumerate() {
            *v = i as u8;
        }
        return lut;
    }
    let inv_gamma = 1.0 / gamma;
    for (i, v) in lut.iter_mut().enumerate() {
        let linear = i as f32 / 255.0;
        let corrected = linear.powf(inv_gamma);
        *v = (corrected * 255.0 + 0.5) as u8;
    }
    lut
}

/// Apply gamma correction to every pixel in the pixmap (in-place).
fn apply_gamma(pm: &mut Pixmap, lut: &[u8; 256]) {
    let len = pm.data.len();
    let mut i = 0;
    while i + 3 < len {
        pm.data[i] = lut[pm.data[i] as usize];
        pm.data[i + 1] = lut[pm.data[i + 1] as usize];
        pm.data[i + 2] = lut[pm.data[i + 2] as usize];
        // skip alpha at i+3
        i += 4;
    }
}

/// Render a DjVu page to an RGBA pixmap at native resolution.
pub fn render(page: &Page) -> Result<Pixmap, Error> {
    render_to_size(page, page.info.width as u32, page.info.height as u32)
}

/// Render a DjVu page to an RGBA pixmap at a target size.
///
/// The target size is in pre-rotation coordinates (i.e. matching the page's
/// native width/height orientation). Rotation is applied after compositing.
pub fn render_to_size(page: &Page, w: u32, h: u32) -> Result<Pixmap, Error> {
    render_to_size_inner(page, w, h, 0)
}

/// Like `render_to_size`, but applies morphological dilation to the mask
/// bitmap `dilate_passes` times before compositing. Each pass thickens every
/// black stroke by ~1 pixel in each direction, improving legibility when the
/// page is displayed at reduced size.
pub fn render_to_size_bold(
    page: &Page,
    w: u32,
    h: u32,
    dilate_passes: u32,
) -> Result<Pixmap, Error> {
    render_to_size_inner(page, w, h, dilate_passes)
}

fn render_to_size_inner(page: &Page, w: u32, h: u32, dilate_passes: u32) -> Result<Pixmap, Error> {
    let mut output = composite_page(page, w, h, dilate_passes)?;
    // Skip gamma for pure bilevel pages: composite_bilevel only produces 0 and 255
    // values, and build_gamma_lut(γ) always maps 0→0 and 255→255, so gamma is a
    // mathematical no-op.  Skipping saves one full read+write pass over the buffer.
    let is_pure_bilevel =
        page.bg44_chunk_count() == 0 && !page.has_palette() && !page.has_foreground();
    if !is_pure_bilevel {
        let lut = build_gamma_lut(page.info.gamma);
        apply_gamma(&mut output, &lut);
    }
    Ok(apply_rotation(output, page.info.rotation))
}

/// Coarse rendering: decode only the first BG44 chunk for a fast blurry preview.
///
/// Skips mask and foreground decoding entirely — shows only the background
/// layer scaled to the target size. Returns `None` if the page has ≤1 BG44
/// chunk (use `render_to_size` instead — it's already fast enough).
pub fn render_to_size_coarse(page: &Page, w: u32, h: u32) -> Result<Option<Pixmap>, Error> {
    let page_w = page.info.width as u32;
    let page_h = page.info.height as u32;

    let bg = match page.decode_background_coarse()? {
        Some(bg) => bg,
        None => return Ok(None),
    };

    let mut output = composite_bg_only(w, h, &bg, page_w, page_h);
    let lut = build_gamma_lut(page.info.gamma);
    apply_gamma(&mut output, &lut);
    Ok(Some(apply_rotation(output, page.info.rotation)))
}

/// Progressive rendering: returns a Vec of increasingly refined pixmaps.
///
/// For pages with multiple BG44 chunks, yields a coarse preview after the
/// first chunk and progressively sharper images after each subsequent chunk.
/// The last frame is identical to `render_to_size()`.
///
/// For pages without a background layer (bilevel, palette), returns a single
/// frame equivalent to `render_to_size()`.
pub fn render_to_size_progressive(page: &Page, w: u32, h: u32) -> Result<Vec<Pixmap>, Error> {
    let rotation = page.info.rotation;
    let page_w = page.info.width as u32;
    let page_h = page.info.height as u32;
    let gamma_lut = build_gamma_lut(page.info.gamma);

    // Only worth progressive rendering for multi-chunk backgrounds.
    let bg_chunks = page.bg44_chunk_count();
    if bg_chunks <= 1 {
        // Single chunk or no BG44 — just render normally.
        let mut output = composite_page(page, w, h, 0)?;
        apply_gamma(&mut output, &gamma_lut);
        return Ok(vec![apply_rotation(output, rotation)]);
    }

    let has_palette = page.has_palette();
    if has_palette {
        // Palette-based pages don't use IW44 — no progressive benefit.
        let mut output = composite_page(page, w, h, 0)?;
        apply_gamma(&mut output, &gamma_lut);
        return Ok(vec![apply_rotation(output, rotation)]);
    }

    // Decode mask and foreground once (they don't have progressive chunks).
    let mask = page.decode_mask()?;
    let fg = page.decode_foreground()?;

    // Decode background progressively.
    let bg_frames = match page.decode_background_progressive()? {
        Some(frames) => frames,
        None => {
            // No background — single render.
            let mut output = match (&mask, &fg) {
                (Some(mask), None) => composite_bilevel(w, h, mask, page_w, page_h),
                _ => Pixmap::white(w, h),
            };
            apply_gamma(&mut output, &gamma_lut);
            return Ok(vec![apply_rotation(output, rotation)]);
        }
    };

    let mut results = Vec::with_capacity(bg_frames.len());

    for bg in &bg_frames {
        let mut output = match (&mask, &fg) {
            (None, _) => composite_bg_only(w, h, bg, page_w, page_h),
            (Some(mask), Some(fg)) => composite_3layer(w, h, mask, bg, fg, page_w, page_h),
            (Some(mask), None) => composite_mask_bg(w, h, mask, bg, page_w, page_h),
        };
        apply_gamma(&mut output, &gamma_lut);
        results.push(apply_rotation(output, rotation));
    }

    Ok(results)
}

/// Composite page layers at the given size without applying rotation.
fn composite_page(page: &Page, w: u32, h: u32, dilate_passes: u32) -> Result<Pixmap, Error> {
    let page_w = page.info.width as u32;
    let page_h = page.info.height as u32;

    let has_palette = page.has_palette();

    let output = if has_palette {
        render_with_palette(page, w, h, page_w, page_h, dilate_passes)?
    } else {
        let mask = page.decode_mask()?;
        let mask = mask.map(|m| dilate_mask(m, dilate_passes));
        let bg = page.decode_background()?;
        let fg = page.decode_foreground()?;

        match (&mask, &bg, &fg) {
            (None, Some(bg), _) => composite_bg_only(w, h, bg, page_w, page_h),
            (Some(mask), None, None) => composite_bilevel(w, h, mask, page_w, page_h),
            (Some(mask), Some(bg), Some(fg)) => {
                composite_3layer(w, h, mask, bg, fg, page_w, page_h)
            }
            (Some(mask), Some(bg), None) => composite_mask_bg(w, h, mask, bg, page_w, page_h),
            (Some(mask), None, Some(fg)) => composite_mask_fg(w, h, mask, fg, page_w, page_h),
            (None, None, _) => Pixmap::white(w, h),
        }
    };

    Ok(output)
}

fn dilate_mask(mask: Bitmap, passes: u32) -> Bitmap {
    mask.dilate_n(passes)
}

/// Dilate the mask bitmap and propagate blit indices to newly-set pixels.
/// Each new foreground pixel inherits the blit index of the neighbor that
/// caused it to be set, so palette color lookup remains correct.
fn dilate_mask_indexed(mask: Bitmap, blit_map: Vec<i32>, passes: u32) -> (Bitmap, Vec<i32>) {
    let mut m = mask;
    let mut bm = blit_map;
    for _ in 0..passes {
        let prev = m.clone();
        m = prev.dilate();
        let w = m.width as usize;
        let h = m.height as usize;
        let mut new_bm = bm.clone();
        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                // Only update pixels that were just added by dilation
                if m.get(x as u32, y as u32) && !prev.get(x as u32, y as u32) {
                    // Find a neighbor that was set in the original mask
                    let bi = if x > 0 && prev.get((x - 1) as u32, y as u32) {
                        bm[(y) * w + (x - 1)]
                    } else if x + 1 < w && prev.get((x + 1) as u32, y as u32) {
                        bm[(y) * w + (x + 1)]
                    } else if y > 0 && prev.get(x as u32, (y - 1) as u32) {
                        bm[(y - 1) * w + x]
                    } else if y + 1 < h && prev.get(x as u32, (y + 1) as u32) {
                        bm[(y + 1) * w + x]
                    } else {
                        -1 // fallback to black
                    };
                    new_bm[idx] = bi;
                }
            }
        }
        bm = new_bm;
    }
    (m, bm)
}

// ============================================================
// BG-only: upscale background to page dimensions
// ============================================================

fn composite_bg_only(w: u32, h: u32, bg: &Pixmap, page_w: u32, page_h: u32) -> Pixmap {
    // Fast path: when rendering at non-native DPI (w ≠ page_w or h ≠ page_h),
    // scale directly to the output size in one bilinear pass.  This eliminates
    // the w×h nearest-neighbour copy loop that the old two-step approach used.
    if w != page_w || h != page_h {
        return scale_bilinear_direct(bg, w, h);
    }
    // Native-DPI path: keep the original virtual-geometry logic so that golden
    // pixel tests (which compare at native DPI) are not disturbed.
    let scaled_bg = scale_layer_bilinear(bg, page_w, page_h);
    // Common case: virtual-geometry rounds to exact page size — return directly.
    if scaled_bg.width == w && scaled_bg.height == h {
        return scaled_bg;
    }
    // Rare case: virtual-geometry rounded down by 1–2 pixels in either dimension.
    // Use bulk row copies and replicate edge pixels instead of per-pixel sampling.
    let sw = scaled_bg.width as usize;
    let sh = scaled_bg.height as usize;
    let ow = w as usize;
    let oh = h as usize;
    let copy_w = sw.min(ow);
    let copy_h = sh.min(oh);
    let mut out = Pixmap::white(w, h);
    for y in 0..copy_h {
        let src_off = y * sw * 4;
        let dst_off = y * ow * 4;
        out.data[dst_off..dst_off + copy_w * 4]
            .copy_from_slice(&scaled_bg.data[src_off..src_off + copy_w * 4]);
        // Replicate last column if output is wider
        if ow > copy_w {
            let last = (src_off + (copy_w - 1) * 4, src_off + copy_w * 4);
            let last_col: [u8; 4] = scaled_bg.data[last.0..last.1]
                .try_into()
                .unwrap_or([255, 255, 255, 255]);
            for ox in copy_w..ow {
                out.data[dst_off + ox * 4..dst_off + ox * 4 + 4].copy_from_slice(&last_col);
            }
        }
    }
    // Replicate last row if output is taller
    if oh > copy_h {
        let last_row_start = (copy_h - 1) * ow * 4;
        let last_row: Vec<u8> = out.data[last_row_start..last_row_start + ow * 4].to_vec();
        for oy in copy_h..oh {
            let dst_off = oy * ow * 4;
            out.data[dst_off..dst_off + ow * 4].copy_from_slice(&last_row);
        }
    }
    out
}

// ============================================================
// Mask-only: black where mask=1, white where mask=0
// ============================================================

/// Write one bilevel mask row into the output RGBA row buffer.
///
/// Called from `composite_bilevel` — extracted to allow both sequential and parallel
/// paths to share the same per-row logic without duplicating the inner loop.
///
/// # Caller contract
/// `out_row` must have length `pw * 4` and must be pre-filled with white (0xFF bytes)
/// so that pixels with unset mask bits keep their background colour and the alpha
/// channel (byte `p + 3`) stays 255 without being explicitly written here.
#[inline]
fn bilevel_row(mask_row: &[u8], out_row: &mut [u8], pw: usize) {
    debug_assert_eq!(
        out_row.len(),
        pw * 4,
        "out_row length mismatch: expected {}, got {}",
        pw * 4,
        out_row.len()
    );
    let mut px = 0usize;
    for &byte in &mask_row[..pw.div_ceil(8)] {
        if byte == 0 {
            // All 8 pixels white — already white in out_row, skip.
            px += 8;
            continue;
        }
        let remaining = (pw - px).min(8);
        // Per-bit unpack: write R=0, G=0, B=0 for set bits.
        // Alpha stays 255 because the caller pre-fills out_row with 0xFF.
        // Bounds: bit < remaining <= pw - px, so px + bit < pw, so
        // p = (px + bit) * 4 <= (pw - 1) * 4 and p + 2 < pw * 4 == out_row.len().
        for bit in 0..remaining {
            if byte & (0x80 >> bit) != 0 {
                let p = (px + bit) * 4;
                out_row[p] = 0;
                out_row[p + 1] = 0;
                out_row[p + 2] = 0;
            }
        }
        px += 8;
    }
}

fn composite_bilevel(w: u32, h: u32, mask: &Bitmap, page_w: u32, page_h: u32) -> Pixmap {
    let mut out = Pixmap::white(w, h);

    // Fast path: when output matches page size exactly, process mask bytes directly.
    // Rows are independent → parallelise under the `parallel` feature.
    if w == page_w && h == page_h && w == mask.width && h == mask.height {
        let stride = mask.row_stride();
        let pw = w as usize;
        let row_bytes = pw * 4;

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            out.data
                .par_chunks_mut(row_bytes)
                .enumerate()
                .for_each(|(y, out_row)| {
                    let mask_row = &mask.data[y * stride..(y + 1) * stride];
                    bilevel_row(mask_row, out_row, pw);
                });
        }
        #[cfg(not(feature = "parallel"))]
        for y in 0..h as usize {
            let mask_row = &mask.data[y * stride..(y + 1) * stride];
            let out_row = &mut out.data[y * row_bytes..(y + 1) * row_bytes];
            bilevel_row(mask_row, out_row, pw);
        }
        return out;
    }

    let col_map = build_coord_map(w, page_w);
    let row_map = build_coord_map(h, page_h);
    for (oy, &my) in row_map.iter().enumerate() {
        if my >= mask.height {
            continue;
        }
        for (ox, &mx) in col_map.iter().enumerate() {
            if mx < mask.width && mask.get(mx, my) {
                out.set_rgb(ox as u32, oy as u32, 0, 0, 0);
            }
        }
    }
    out
}

// ============================================================
// Mask + BG: background where mask=0, black text where mask=1
// ============================================================

fn composite_mask_bg(
    w: u32,
    h: u32,
    mask: &Bitmap,
    bg: &Pixmap,
    page_w: u32,
    page_h: u32,
) -> Pixmap {
    // Pass 1: fill output unconditionally with scaled background (branch-free, cache-friendly).
    let mut out = composite_bg_only(w, h, bg, page_w, page_h);
    // Pass 2: overwrite only masked pixels with black using precomputed integer coord tables.
    let col_map = build_coord_map(w, page_w);
    let row_map = build_coord_map(h, page_h);
    for (oy, &py) in row_map.iter().enumerate() {
        if py >= mask.height {
            continue;
        }
        for (ox, &px) in col_map.iter().enumerate() {
            if px < mask.width && mask.get(px, py) {
                out.set_rgb(ox as u32, oy as u32, 0, 0, 0);
            }
        }
    }
    out
}

// ============================================================
// Mask + FG (no BG): foreground color where mask=1, white elsewhere
// ============================================================

fn composite_mask_fg(
    w: u32,
    h: u32,
    mask: &Bitmap,
    fg: &Pixmap,
    page_w: u32,
    page_h: u32,
) -> Pixmap {
    let col_map = build_coord_map(w, page_w);
    let row_map = build_coord_map(h, page_h);
    let fg_samp = NearestSampler::new(fg, page_w, page_h);
    let mut out = Pixmap::white(w, h);
    for (oy, &py) in row_map.iter().enumerate() {
        if py >= mask.height {
            continue;
        }
        for (ox, &px) in col_map.iter().enumerate() {
            if px < mask.width && mask.get(px, py) {
                let (r, g, b) = fg_samp.sample(fg, px, py);
                out.set_rgb(ox as u32, oy as u32, r, g, b);
            }
        }
    }
    out
}

// ============================================================
// 3-layer: mask selects between FG and BG
// ============================================================

fn composite_3layer(
    w: u32,
    h: u32,
    mask: &Bitmap,
    bg: &Pixmap,
    fg: &Pixmap,
    page_w: u32,
    page_h: u32,
) -> Pixmap {
    // Pass 1: fill output unconditionally with scaled background (branch-free, cache-friendly).
    let mut out = composite_bg_only(w, h, bg, page_w, page_h);
    // Pass 2: overwrite only masked pixels with FG color using precomputed integer coord tables.
    let col_map = build_coord_map(w, page_w);
    let row_map = build_coord_map(h, page_h);
    let fg_samp = NearestSampler::new(fg, page_w, page_h);
    for (oy, &py) in row_map.iter().enumerate() {
        if py >= mask.height {
            continue;
        }
        for (ox, &px) in col_map.iter().enumerate() {
            if px < mask.width && mask.get(px, py) {
                let (r, g, b) = fg_samp.sample(fg, px, py);
                out.set_rgb(ox as u32, oy as u32, r, g, b);
            }
        }
    }
    out
}

// ============================================================
// Palette composite: mask + BG + FGbz palette colors
// ============================================================

fn render_with_palette(
    page: &Page,
    w: u32,
    h: u32,
    page_w: u32,
    page_h: u32,
    dilate_passes: u32,
) -> Result<Pixmap, Error> {
    let mask_indexed = page.decode_mask_indexed()?;
    let bg = page.decode_background()?;
    let palette = page.decode_palette()?;

    match (mask_indexed, bg, palette) {
        (Some((mask, blit_map)), Some(bg), Some(pal)) => {
            let (mask, blit_map) = dilate_mask_indexed(mask, blit_map, dilate_passes);
            Ok(composite_palette(
                w, h, &mask, &blit_map, &bg, &pal, page_w, page_h,
            ))
        }
        (Some((mask, blit_map)), None, Some(pal)) => {
            let (mask, blit_map) = dilate_mask_indexed(mask, blit_map, dilate_passes);
            Ok(composite_palette_no_bg(
                w, h, &mask, &blit_map, &pal, page_w, page_h,
            ))
        }
        (None, Some(bg), _) => Ok(composite_bg_only(w, h, &bg, page_w, page_h)),
        _ => Ok(Pixmap::white(w, h)),
    }
}

fn palette_color(pal: &Palette, blit_idx: i32) -> (u8, u8, u8) {
    if blit_idx < 0 {
        return (0, 0, 0);
    }
    let bi = blit_idx as usize;
    if bi < pal.indices.len() {
        let ci = pal.indices[bi] as usize;
        if ci < pal.colors.len() {
            return pal.colors[ci];
        }
    }
    // Invalid index mapping: render black to avoid silently painting wrong color.
    (0, 0, 0)
}

#[allow(clippy::too_many_arguments)]
fn composite_palette(
    w: u32,
    h: u32,
    mask: &Bitmap,
    blit_map: &[i32],
    bg: &Pixmap,
    pal: &Palette,
    page_w: u32,
    page_h: u32,
) -> Pixmap {
    let col_map = build_coord_map(w, page_w);
    let row_map = build_coord_map(h, page_h);
    let scaled_bg = scale_layer_bilinear(bg, page_w, page_h);
    let mut out = Pixmap::white(w, h);
    for (oy, &my) in row_map.iter().enumerate() {
        for (ox, &mx) in col_map.iter().enumerate() {
            let is_fg = mx < mask.width && my < mask.height && mask.get(mx, my);
            if is_fg {
                let mi = my as usize * mask.width as usize + mx as usize;
                let (r, g, b) = if mi < blit_map.len() {
                    palette_color(pal, blit_map[mi])
                } else {
                    (0, 0, 0)
                };
                out.set_rgb(ox as u32, oy as u32, r, g, b);
            } else {
                let (r, g, b) = sample_scaled(&scaled_bg, mx, my);
                out.set_rgb(ox as u32, oy as u32, r, g, b);
            }
        }
    }
    out
}

fn composite_palette_no_bg(
    w: u32,
    h: u32,
    mask: &Bitmap,
    blit_map: &[i32],
    pal: &Palette,
    page_w: u32,
    page_h: u32,
) -> Pixmap {
    let col_map = build_coord_map(w, page_w);
    let row_map = build_coord_map(h, page_h);
    let mut out = Pixmap::white(w, h);
    for (oy, &my) in row_map.iter().enumerate() {
        if my >= mask.height {
            continue;
        }
        for (ox, &mx) in col_map.iter().enumerate() {
            if mx < mask.width && mask.get(mx, my) {
                let mi = my as usize * mask.width as usize + mx as usize;
                let (r, g, b) = if mi < blit_map.len() {
                    palette_color(pal, blit_map[mi])
                } else {
                    (0, 0, 0)
                };
                out.set_rgb(ox as u32, oy as u32, r, g, b);
            }
        }
    }
    out
}

// ============================================================
// Precomputed geometry for sampling — avoids per-pixel recomputation
// ============================================================

/// Build a coordinate-mapping table for nearest-neighbour scaling.
///
/// Maps each output pixel `[0, out_dim)` to the nearest source pixel
/// `[0, page_dim)` using the half-pixel-centre convention:
/// `mapped = floor((i + 0.5) * page_dim / out_dim)`, computed as
/// `(2*i + 1) * page_dim / (2 * out_dim)` in integer arithmetic.
///
/// All arithmetic is integer-only — no f64 per pixel.
fn build_coord_map(out_dim: u32, page_dim: u32) -> Vec<u32> {
    let max = page_dim.saturating_sub(1);
    if out_dim == page_dim {
        return (0..out_dim).collect();
    }
    let out_dim_u64 = out_dim as u64;
    let page_dim_u64 = page_dim as u64;
    (0..out_dim)
        .map(|i| {
            let mapped = (2 * i as u64 + 1) * page_dim_u64 / (2 * out_dim_u64);
            (mapped as u32).min(max)
        })
        .collect()
}

/// Precomputed geometry for nearest-neighbor sampling from a layer.
struct NearestSampler {
    reduction: u32,
    virt_page_w_m1: u32,
    virt_page_h_m1: u32,
    y_shift: u32,
    src_w_m1: u32,
    src_h_m1: u32,
}

impl NearestSampler {
    fn new(src: &Pixmap, page_w: u32, page_h: u32) -> Self {
        let (reduction, _, _, virt_page_w, virt_page_h) =
            layer_virtual_geometry(src, page_w, page_h);
        let y_shift = src.height.saturating_mul(reduction).saturating_sub(page_h);
        NearestSampler {
            reduction,
            virt_page_w_m1: virt_page_w.saturating_sub(1),
            virt_page_h_m1: virt_page_h.saturating_sub(1),
            y_shift,
            src_w_m1: src.width.saturating_sub(1),
            src_h_m1: src.height.saturating_sub(1),
        }
    }

    #[inline(always)]
    fn sample(&self, src: &Pixmap, page_x: u32, page_y: u32) -> (u8, u8, u8) {
        let px = page_x.min(self.virt_page_w_m1);
        let py = page_y.saturating_add(self.y_shift).min(self.virt_page_h_m1);
        let sx = (px / self.reduction).min(self.src_w_m1);
        let sy = (py / self.reduction).min(self.src_h_m1);
        src.get_rgb(sx, sy)
    }
}

fn layer_virtual_geometry(src: &Pixmap, page_w: u32, page_h: u32) -> (u32, u32, u32, u32, u32) {
    let red_w = page_w.div_ceil(src.width);
    let red_h = page_h.div_ceil(src.height);
    let reduction = red_w.max(red_h).max(1);
    let virt_w = (page_w / reduction).max(1);
    let virt_h = (page_h / reduction).max(1);
    let virt_page_w = virt_w * reduction;
    let virt_page_h = virt_h * reduction;
    (reduction, virt_w, virt_h, virt_page_w, virt_page_h)
}

// ============================================================
// Separable bilinear scaler with SIMD vertical pass
// ============================================================

/// 4-bit fractional precision (16 sub-pixel positions), matching DjVuLibre.
const FRACBITS: u32 = 4;
const FRACMASK: u32 = (1 << FRACBITS) - 1; // 0xF

/// Bilinear lerp for one byte value: `(a*(16-f) + b*f + 8) >> 4`.
#[inline(always)]
fn lerp8(a: u8, b: u8, f: usize) -> u8 {
    let cf = 16 - f;
    ((a as usize * cf + b as usize * f + 8) >> 4) as u8
}

// ── SIMD vertical-pass helpers ───────────────────────────────────────────────
//
// The vertical pass reads two horizontally-interpolated rows (stored as RGBX,
// 4 bytes per pixel, alpha/pad byte = 0) and writes the bilinearly-blended
// result as RGBA (alpha = 255) into the output buffer.
//
// Processing 16 bytes (= 4 RGBA pixels) per SIMD iteration on AArch64 NEON:
//   1. Load 16 bytes from row0 and row1.
//   2. Zero-extend each 8 bytes → u16×8.
//   3. Compute (a*(16-fy) + b*fy + 8) >> 4  in u16 (no overflow: ≤ 4080+8).
//   4. Narrow back to u8.
//   5. OR with 0x000000FF mask per pixel to set alpha = 255.
//   6. Store 16 bytes to output.
//
// The pad (4th) byte in hbuf is always 0, so the lerp produces 0 there too.
// The OR with the alpha mask then correctly sets it to 255.

/// Copy `src` (RGBX, alpha/pad byte = 0) into `dst` setting alpha to 255.
///
/// Both slices must have length `n * 4` where n is the pixel count.
#[inline(always)]
fn copy_row_set_alpha(src: &[u8], dst: &mut [u8]) {
    debug_assert_eq!(src.len(), dst.len());
    #[cfg(target_arch = "aarch64")]
    if src.len() >= 16 && std::arch::is_aarch64_feature_detected!("neon") {
        // Safety: NEON availability confirmed at runtime; slices are in-bounds.
        #[allow(unsafe_code)]
        unsafe {
            copy_row_set_alpha_neon(src, dst)
        }
        return;
    }
    copy_row_set_alpha_scalar(src, dst);
}

#[inline(always)]
fn copy_row_set_alpha_scalar(src: &[u8], dst: &mut [u8]) {
    let n = src.len() / 4;
    for i in 0..n {
        let s = i * 4;
        dst[s] = src[s];
        dst[s + 1] = src[s + 1];
        dst[s + 2] = src[s + 2];
        dst[s + 3] = 255;
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
#[allow(unsafe_code)]
unsafe fn copy_row_set_alpha_neon(src: &[u8], dst: &mut [u8]) {
    use std::arch::aarch64::*;
    // SAFETY: aarch64 always has NEON; slices are aligned and in-bounds.
    let alpha_mask =
        unsafe { vld1q_u8([0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255].as_ptr()) };
    let chunks = src.len() / 16;
    for i in 0..chunks {
        let off = i * 16;
        unsafe {
            let v = vld1q_u8(src.as_ptr().add(off));
            vst1q_u8(dst.as_mut_ptr().add(off), vorrq_u8(v, alpha_mask));
        }
    }
    // Scalar tail
    let tail_start = chunks * 16;
    copy_row_set_alpha_scalar(&src[tail_start..], &mut dst[tail_start..]);
}

/// Bilinear lerp two RGBX rows → RGBA output, setting alpha = 255.
///
/// `fy` is the vertical fraction (0..16).
#[inline(always)]
fn lerp_rows(row0: &[u8], row1: &[u8], dst: &mut [u8], fy: usize) {
    debug_assert_eq!(row0.len(), row1.len());
    debug_assert_eq!(row0.len(), dst.len());
    if fy == 0 {
        copy_row_set_alpha(row0, dst);
        return;
    }
    #[cfg(target_arch = "aarch64")]
    if row0.len() >= 16 && std::arch::is_aarch64_feature_detected!("neon") {
        // Safety: NEON availability confirmed at runtime; slices are in-bounds.
        #[allow(unsafe_code)]
        unsafe {
            lerp_rows_neon(row0, row1, dst, fy as u16)
        }
        return;
    }
    lerp_rows_scalar(row0, row1, dst, fy);
}

#[inline(always)]
fn lerp_rows_scalar(row0: &[u8], row1: &[u8], dst: &mut [u8], fy: usize) {
    let n = dst.len() / 4;
    let cf = 16 - fy;
    for i in 0..n {
        let s = i * 4;
        dst[s] = ((row0[s] as usize * cf + row1[s] as usize * fy + 8) >> 4) as u8;
        dst[s + 1] = ((row0[s + 1] as usize * cf + row1[s + 1] as usize * fy + 8) >> 4) as u8;
        dst[s + 2] = ((row0[s + 2] as usize * cf + row1[s + 2] as usize * fy + 8) >> 4) as u8;
        dst[s + 3] = 255;
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
#[allow(unsafe_code)]
unsafe fn lerp_rows_neon(row0: &[u8], row1: &[u8], dst: &mut [u8], fy: u16) {
    use std::arch::aarch64::*;
    let cfy = 16 - fy;
    // SAFETY: aarch64 always has NEON; all register ops are safe.
    let (fy_v, cfy_v, eight, alpha_mask) = unsafe {
        (
            vdupq_n_u16(fy),
            vdupq_n_u16(cfy),
            vdupq_n_u16(8),
            vld1q_u8([0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255].as_ptr()),
        )
    };

    let chunks = dst.len() / 16;
    for i in 0..chunks {
        let off = i * 16;
        // SAFETY: off + 16 <= dst.len() by construction; row0/row1 have same len.
        unsafe {
            let a = vld1q_u8(row0.as_ptr().add(off));
            let b = vld1q_u8(row1.as_ptr().add(off));

            // Zero-extend lo/hi 8 bytes to u16
            let a_lo = vmovl_u8(vget_low_u8(a));
            let a_hi = vmovl_u8(vget_high_u8(a));
            let b_lo = vmovl_u8(vget_low_u8(b));
            let b_hi = vmovl_u8(vget_high_u8(b));

            // lerp = (a*cfy + b*fy + 8) >> 4
            let lo = vshrq_n_u16(
                vaddq_u16(
                    vaddq_u16(vmulq_u16(a_lo, cfy_v), vmulq_u16(b_lo, fy_v)),
                    eight,
                ),
                4,
            );
            let hi = vshrq_n_u16(
                vaddq_u16(
                    vaddq_u16(vmulq_u16(a_hi, cfy_v), vmulq_u16(b_hi, fy_v)),
                    eight,
                ),
                4,
            );

            // Narrow to u8, set alpha=255 via OR mask
            let result = vorrq_u8(vcombine_u8(vmovn_u16(lo), vmovn_u16(hi)), alpha_mask);
            vst1q_u8(dst.as_mut_ptr().add(off), result);
        }
    }

    // Scalar tail for the last <16 bytes (< 4 pixels)
    let tail_start = chunks * 16;
    lerp_rows_scalar(
        &row0[tail_start..],
        &row1[tail_start..],
        &mut dst[tail_start..],
        fy as usize,
    );
}

/// Precompute source coordinates for a scanline, matching BilinearSampler's f64 math.
/// Returns a Vec of packed u32: upper bits = integer coord, lower FRACBITS = fraction.
/// Uses center-pixel mapping: src = (dst + 0.5) * src_size / out_size - 0.5
fn prepare_coord(src_size: u32, out_size: u32) -> Vec<u32> {
    if out_size == 0 {
        return Vec::new();
    }
    let scale = src_size as f64 / out_size as f64;
    let max_src = src_size as f64 - 1.0;
    let mut coords = Vec::with_capacity(out_size as usize);
    for i in 0..out_size {
        let s = ((i as f64 + 0.5) * scale - 0.5).clamp(0.0, max_src);
        let si = s as u32;
        // Match BilinearSampler's fraction quantization: floor(frac * 16 + 0.5)
        let frac = ((s - si as f64) * 16.0 + 0.5).floor().clamp(0.0, 15.0) as u32;
        coords.push((si << FRACBITS) | frac);
    }
    coords
}

/// Horizontal bilinear pass for one source row.
///
/// Reads RGBA pixels from `src` at `src_row_off .. src_row_off + sw` and writes
/// the interpolated RGBX result (alpha/pad byte = 0) into `dst` (length = `ow * 4`).
/// `hcoord[dx]` encodes the fixed-point source x-coordinate for output column `dx`.
///
/// # Caller contract
/// - `src.len() >= (src_row_off + sw_m1 + 1) * 4 + 2` (enough source pixels for the row)
/// - `dst.len() >= ow * 4` (destination pre-sized by caller, zeroed for pad byte)
#[inline]
fn hpass_row(
    src: &[u8],
    src_row_off: usize,
    sw_m1: usize,
    hcoord: &[u32],
    ow: usize,
    dst: &mut [u8],
) {
    debug_assert!(
        src.len() >= (src_row_off + sw_m1 + 1) * 4,
        "src too short for hpass_row: len={}, need>={}",
        src.len(),
        (src_row_off + sw_m1 + 1) * 4
    );
    debug_assert!(
        dst.len() >= ow * 4,
        "dst too short for hpass_row: len={}, need>={}",
        dst.len(),
        ow * 4
    );
    for (dx, &coord) in hcoord.iter().enumerate().take(ow) {
        let ix = ((coord >> FRACBITS) as usize).min(sw_m1);
        let fx = (coord & FRACMASK) as usize;
        let ix1 = (ix + 1).min(sw_m1);
        let s0 = (src_row_off + ix) * 4;
        let s1 = (src_row_off + ix1) * 4;
        let d = dx * 4;
        if fx == 0 {
            dst[d] = src[s0];
            dst[d + 1] = src[s0 + 1];
            dst[d + 2] = src[s0 + 2];
            // dst[d+3] = 0 (pad; caller must zero-init hbuf)
        } else {
            dst[d] = lerp8(src[s0], src[s1], fx);
            dst[d + 1] = lerp8(src[s0 + 1], src[s1 + 1], fx);
            dst[d + 2] = lerp8(src[s0 + 2], src[s1 + 2], fx);
        }
    }
}

/// Pre-scale a source layer to page dimensions using separable bilinear interpolation.
///
/// Pass 1 (horizontal): interpolate along X into an intermediate RGBX buffer
/// (4 bytes per pixel, pad byte = 0) of size `sh × ow`.
///
/// Pass 2 (vertical): lerp two horizontal rows → RGBA output (alpha = 255),
/// accelerated with NEON on AArch64 (4 pixels / 16 bytes per SIMD iteration).
///
/// With the `parallel` feature, both passes run across all available CPU cores
/// via rayon, giving near-linear speedup on large pages.
fn scale_layer_bilinear(src: &Pixmap, page_w: u32, page_h: u32) -> Pixmap {
    let (_, virt_w, virt_h, virt_page_w, virt_page_h) = layer_virtual_geometry(src, page_w, page_h);

    let ow = virt_page_w;
    let oh = virt_page_h;
    let sw = src.width as usize;
    let sh = src.height as usize;

    if sw == 0 || sh == 0 || ow == 0 || oh == 0 {
        return Pixmap::white(ow.max(1), oh.max(1));
    }

    let hcoord = prepare_coord(virt_w, ow);
    let vcoord = prepare_coord(virt_h, oh);

    let sw_m1 = sw - 1;
    let sh_m1 = sh - 1;
    let ow_us = ow as usize;

    // Pass 1: horizontal interpolation → RGBX intermediate (pad byte = 0).
    // 4-byte stride matches the output stride, enabling branch-free SIMD in pass 2.
    let hstride = ow_us * 4;
    let mut hbuf: Vec<u8> = vec![0u8; sh * hstride];

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        hbuf.par_chunks_mut(hstride)
            .enumerate()
            .for_each(|(sy, dst_row)| {
                hpass_row(&src.data, sy * sw, sw_m1, &hcoord, ow_us, dst_row);
            });
    }
    #[cfg(not(feature = "parallel"))]
    for sy in 0..sh {
        hpass_row(
            &src.data,
            sy * sw,
            sw_m1,
            &hcoord,
            ow_us,
            &mut hbuf[sy * hstride..],
        );
    }

    // Pass 2: vertical interpolation — SIMD lerp (RGBX → RGBA, alpha=255).
    let mut out = Pixmap::white(ow, oh);

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        out.data
            .par_chunks_mut(ow_us * 4)
            .zip(vcoord.par_iter())
            .for_each(|(out_row, &coord)| {
                let iy = ((coord >> FRACBITS) as usize).min(sh_m1);
                let fy = (coord & FRACMASK) as usize;
                let iy1 = (iy + 1).min(sh_m1);
                lerp_rows(
                    &hbuf[iy * hstride..iy * hstride + ow_us * 4],
                    &hbuf[iy1 * hstride..iy1 * hstride + ow_us * 4],
                    out_row,
                    fy,
                );
            });
    }
    #[cfg(not(feature = "parallel"))]
    for (dy, &coord) in vcoord.iter().enumerate().take(oh as usize) {
        let iy = ((coord >> FRACBITS) as usize).min(sh_m1);
        let fy = (coord & FRACMASK) as usize;
        let iy1 = (iy + 1).min(sh_m1);
        let row0_off = iy * hstride;
        let row1_off = iy1 * hstride;
        let out_off = dy * ow_us * 4;
        lerp_rows(
            &hbuf[row0_off..row0_off + ow_us * 4],
            &hbuf[row1_off..row1_off + ow_us * 4],
            &mut out.data[out_off..out_off + ow_us * 4],
            fy,
        );
    }

    out
}

/// Sample from a pre-scaled pixmap using page coordinates.
/// The scaled pixmap covers virt_page_w × virt_page_h; coordinates beyond
/// its bounds are clamped.
#[inline(always)]
fn sample_scaled(scaled: &Pixmap, page_x: u32, page_y: u32) -> (u8, u8, u8) {
    let sx = page_x.min(scaled.width.saturating_sub(1));
    let sy = page_y.min(scaled.height.saturating_sub(1));
    scaled.get_rgb(sx, sy)
}

/// Bilinear scale `src` to exactly `(ow × oh)` pixels.
///
/// Unlike [`scale_layer_bilinear`], does not apply virtual-geometry rounding —
/// the output is always exactly `ow × oh`.  Used when the caller needs
/// the result at a precise output size (e.g. compositing at non-native DPI).
fn scale_bilinear_direct(src: &Pixmap, ow: u32, oh: u32) -> Pixmap {
    let sw = src.width as usize;
    let sh = src.height as usize;
    if sw == 0 || sh == 0 || ow == 0 || oh == 0 {
        return Pixmap::white(ow.max(1), oh.max(1));
    }
    let hcoord = prepare_coord(src.width, ow);
    let vcoord = prepare_coord(src.height, oh);
    let sw_m1 = sw - 1;
    let sh_m1 = sh - 1;
    let ow_us = ow as usize;
    let hstride = ow_us * 4;
    let mut hbuf: Vec<u8> = vec![0u8; sh * hstride];

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        hbuf.par_chunks_mut(hstride)
            .enumerate()
            .for_each(|(sy, dst_row)| {
                hpass_row(&src.data, sy * sw, sw_m1, &hcoord, ow_us, dst_row);
            });
    }
    #[cfg(not(feature = "parallel"))]
    for sy in 0..sh {
        hpass_row(
            &src.data,
            sy * sw,
            sw_m1,
            &hcoord,
            ow_us,
            &mut hbuf[sy * hstride..],
        );
    }

    let mut out = Pixmap::white(ow, oh);

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        out.data
            .par_chunks_mut(ow_us * 4)
            .zip(vcoord.par_iter())
            .for_each(|(out_row, &coord)| {
                let iy = ((coord >> FRACBITS) as usize).min(sh_m1);
                let fy = (coord & FRACMASK) as usize;
                let iy1 = (iy + 1).min(sh_m1);
                lerp_rows(
                    &hbuf[iy * hstride..iy * hstride + ow_us * 4],
                    &hbuf[iy1 * hstride..iy1 * hstride + ow_us * 4],
                    out_row,
                    fy,
                );
            });
    }
    #[cfg(not(feature = "parallel"))]
    for (dy, &coord) in vcoord.iter().enumerate().take(oh as usize) {
        let iy = ((coord >> FRACBITS) as usize).min(sh_m1);
        let fy = (coord & FRACMASK) as usize;
        let iy1 = (iy + 1).min(sh_m1);
        let row0_off = iy * hstride;
        let row1_off = iy1 * hstride;
        let out_off = dy * ow_us * 4;
        lerp_rows(
            &hbuf[row0_off..row0_off + ow_us * 4],
            &hbuf[row1_off..row1_off + ow_us * 4],
            &mut out.data[out_off..out_off + ow_us * 4],
            fy,
        );
    }
    out
}

// ============================================================
// Rotation
// ============================================================

/// Render a DjVu page at the requested target size with anti-aliased
/// downscaling and a contrast-boosting curve.
///
/// Internally renders at native resolution, then box-downsamples to the target
/// size. The `boldness` parameter (0.0 = neutral, 0.5–1.0 = typical) darkens
/// anti-aliased edge pixels, counteracting the perceptual thinning of dark
/// strokes on light backgrounds.
///
/// If the target size is >= native, this falls back to `render_to_size`.
pub fn render_aa(page: &Page, w: u32, h: u32, boldness: f32) -> Result<Pixmap, Error> {
    let page_w = page.info.width as u32;
    let page_h = page.info.height as u32;

    // No downscaling needed — just render normally
    if w >= page_w && h >= page_h {
        return render_to_size(page, w, h);
    }

    // Composite at native resolution without rotation (we rotate after downsample)
    let native = composite_page(page, page_w, page_h, 0)?;

    // Box-downsample with contrast boost, then apply rotation
    let downsampled = box_downsample_boost(&native, w, h, boldness);
    Ok(apply_rotation(downsampled, page.info.rotation))
}

/// Box-downsample a pixmap and darken anti-aliased edges.
///
/// For each output pixel, averages the corresponding rectangle of source pixels,
/// then applies a curve that pushes semi-transparent edges toward the darker end.
/// Pure black and pure white are unchanged; only intermediate values are affected.
fn box_downsample_boost(src: &Pixmap, tw: u32, th: u32, boldness: f32) -> Pixmap {
    // Build LUT for the boost curve (avoids per-pixel powf)
    let lut: [u8; 256] = core::array::from_fn(|i| {
        if boldness <= 0.0 || i == 0 || i == 255 {
            return i as u8;
        }
        let opacity = 1.0 - i as f32 / 255.0;
        let boosted = 1.0 - (1.0 - opacity).powf(1.0 + boldness);
        ((1.0 - boosted) * 255.0 + 0.5).clamp(0.0, 255.0) as u8
    });

    let sw = src.width as f64;
    let sh = src.height as f64;
    let tw_f = tw as f64;
    let th_f = th as f64;
    let mut out = Pixmap::white(tw, th);

    for y in 0..th {
        // Exact floating-point source rectangle for this output row
        let fy0 = y as f64 * sh / th_f;
        let fy1 = (y + 1) as f64 * sh / th_f;
        let iy0 = fy0.floor() as u32;
        let iy1 = (fy1.ceil() as u32).min(src.height);

        for x in 0..tw {
            // Exact floating-point source rectangle for this output column
            let fx0 = x as f64 * sw / tw_f;
            let fx1 = (x + 1) as f64 * sw / tw_f;
            let ix0 = fx0.floor() as u32;
            let ix1 = (fx1.ceil() as u32).min(src.width);

            // Area-weighted sum: pixels partially inside the rect are weighted
            // by the fraction of their area that overlaps.
            let mut r_sum = 0.0f64;
            let mut g_sum = 0.0f64;
            let mut b_sum = 0.0f64;
            let mut w_sum = 0.0f64;

            for iy in iy0..iy1 {
                let wy = (iy as f64 + 1.0).min(fy1) - (iy as f64).max(fy0);
                for ix in ix0..ix1 {
                    let wx = (ix as f64 + 1.0).min(fx1) - (ix as f64).max(fx0);
                    let w = wx * wy;
                    let (r, g, b) = src.get_rgb(ix, iy);
                    r_sum += r as f64 * w;
                    g_sum += g as f64 * w;
                    b_sum += b as f64 * w;
                    w_sum += w;
                }
            }

            let r_avg = (r_sum / w_sum + 0.5).clamp(0.0, 255.0) as u8;
            let g_avg = (g_sum / w_sum + 0.5).clamp(0.0, 255.0) as u8;
            let b_avg = (b_sum / w_sum + 0.5).clamp(0.0, 255.0) as u8;

            // Only apply boldness to near-white grayscale pixels (text
            // edges on white background).  Skip colored pixels (photos)
            // and darker grays (background boxes, already-dark edges).
            let mn = r_avg.min(g_avg).min(b_avg);
            let mx = r_avg.max(g_avg).max(b_avg);
            let is_grayscale = mx - mn < 40;
            let is_near_white = mn > 220;
            let (r, g, b) = if is_grayscale && is_near_white {
                (
                    lut[r_avg as usize],
                    lut[g_avg as usize],
                    lut[b_avg as usize],
                )
            } else {
                (r_avg, g_avg, b_avg)
            };
            out.set_rgb(x, y, r, g, b);
        }
    }

    out
}

fn apply_rotation(src: Pixmap, rotation: Rotation) -> Pixmap {
    match rotation {
        Rotation::None => src,
        Rotation::Cw90 => rotate_cw90(&src),
        Rotation::Cw180 => rotate_180(&src),
        Rotation::Cw270 => rotate_cw270(&src),
    }
}

fn rotate_cw90(src: &Pixmap) -> Pixmap {
    let w = src.height;
    let h = src.width;
    let mut out = Pixmap::white(w, h);
    for y in 0..src.height {
        for x in 0..src.width {
            let (r, g, b) = src.get_rgb(x, y);
            let nx = src.height - 1 - y;
            let ny = x;
            out.set_rgb(nx, ny, r, g, b);
        }
    }
    out
}

fn rotate_180(src: &Pixmap) -> Pixmap {
    let mut out = Pixmap::white(src.width, src.height);
    for y in 0..src.height {
        for x in 0..src.width {
            let (r, g, b) = src.get_rgb(x, y);
            out.set_rgb(src.width - 1 - x, src.height - 1 - y, r, g, b);
        }
    }
    out
}

fn rotate_cw270(src: &Pixmap) -> Pixmap {
    let w = src.height;
    let h = src.width;
    let mut out = Pixmap::white(w, h);
    for y in 0..src.height {
        for x in 0..src.width {
            let (r, g, b) = src.get_rgb(x, y);
            let nx = y;
            let ny = src.width - 1 - x;
            out.set_rgb(nx, ny, r, g, b);
        }
    }
    out
}
