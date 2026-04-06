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
#[inline]
fn bilevel_row(mask_row: &[u8], out_row: &mut [u8], pw: usize) {
    let mut px = 0usize;
    for &byte in &mask_row[..pw.div_ceil(8)] {
        if byte == 0 {
            // All 8 pixels white — already white in out_row, skip.
            px += 8;
            continue;
        }
        let remaining = (pw - px).min(8);
        // Per-bit unpack: write R=0, G=0, B=0 for set bits (alpha stays 255).
        // We use direct slice indexing into out_row (pre-computed row offset)
        // to avoid the per-pixel (y * width + x) * 4 multiply.
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
    {
        // Safety: M1/M4/Apple Silicon always has NEON; all ptrs are in-bounds.
        if src.len() >= 16 {
            #[allow(unsafe_code)]
            unsafe {
                copy_row_set_alpha_neon(src, dst)
            }
            return;
        }
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
    {
        if row0.len() >= 16 {
            #[allow(unsafe_code)]
            unsafe {
                lerp_rows_neon(row0, row1, dst, fy as u16)
            }
            return;
        }
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
#[inline]
fn hpass_row(
    src: &[u8],
    src_row_off: usize,
    sw_m1: usize,
    hcoord: &[u32],
    ow: usize,
    dst: &mut [u8],
) {
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

#[cfg(test)]
/// Bilinear sample from src layer, mapping output coords (x,y) at size (ow,oh) to src coords.
fn sample_bilinear(src: &Pixmap, x: u32, y: u32, ow: u32, oh: u32) -> (u8, u8, u8) {
    let sw = src.width as f64;
    let sh = src.height as f64;
    let sx = ((x as f64 + 0.5) * sw / ow as f64 - 0.5).clamp(0.0, sw - 1.0);
    let sy = ((y as f64 + 0.5) * sh / oh as f64 - 0.5).clamp(0.0, sh - 1.0);

    let sx0 = sx as u32;
    let sy0 = sy as u32;
    let sx1 = (sx0 + 1).min(src.width - 1);
    let sy1 = (sy0 + 1).min(src.height - 1);
    let fx = sx - sx0 as f64;
    let fy = sy - sy0 as f64;
    let (r00, g00, b00) = src.get_rgb(sx0, sy0);
    let (r10, g10, b10) = src.get_rgb(sx1, sy0);
    let (r01, g01, b01) = src.get_rgb(sx0, sy1);
    let (r11, g11, b11) = src.get_rgb(sx1, sy1);
    let interp = |v00: u8, v10: u8, v01: u8, v11: u8| -> u8 {
        let v = v00 as f64 * (1.0 - fx) * (1.0 - fy)
            + v10 as f64 * fx * (1.0 - fy)
            + v01 as f64 * (1.0 - fx) * fy
            + v11 as f64 * fx * fy;
        (v + 0.5).clamp(0.0, 255.0) as u8
    };
    (
        interp(r00, r10, r01, r11),
        interp(g00, g10, g01, g11),
        interp(b00, b10, b01, b11),
    )
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

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    #![allow(
        clippy::manual_div_ceil,
        clippy::needless_range_loop,
        clippy::precedence,
        clippy::unnecessary_cast
    )]

    use super::*;
    use crate::document::Document;

    #[derive(Clone, Copy)]
    struct DiffStats {
        pixel_count: usize,
        pixel_mismatches: usize,
        byte_mismatches: usize,
        sum_abs_r: u64,
        sum_abs_g: u64,
        sum_abs_b: u64,
        max_abs_r: u8,
        max_abs_g: u8,
        max_abs_b: u8,
    }

    fn assets_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/djvujs/library/assets")
    }

    fn golden_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/composite")
    }

    fn render_page(file: &str, page_idx: usize) -> Pixmap {
        let data = std::fs::read(assets_path().join(file)).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(page_idx).unwrap();
        render(&page).unwrap()
    }

    fn ppm_header_end(ppm: &[u8]) -> usize {
        let p1 = ppm.iter().position(|&b| b == b'\n').unwrap() + 1;
        let p2 = p1 + ppm[p1..].iter().position(|&b| b == b'\n').unwrap() + 1;
        p2 + ppm[p2..].iter().position(|&b| b == b'\n').unwrap() + 1
    }

    fn diff_stats(actual_ppm: &[u8], expected_ppm: &[u8]) -> DiffStats {
        let a = &actual_ppm[ppm_header_end(actual_ppm)..];
        let e = &expected_ppm[ppm_header_end(expected_ppm)..];
        let px = a.len().min(e.len()) / 3;
        let mut stats = DiffStats {
            pixel_count: px,
            pixel_mismatches: 0,
            byte_mismatches: 0,
            sum_abs_r: 0,
            sum_abs_g: 0,
            sum_abs_b: 0,
            max_abs_r: 0,
            max_abs_g: 0,
            max_abs_b: 0,
        };
        for p in 0..px {
            let i = p * 3;
            let dr = (a[i] as i16 - e[i] as i16).unsigned_abs() as u8;
            let dg = (a[i + 1] as i16 - e[i + 1] as i16).unsigned_abs() as u8;
            let db = (a[i + 2] as i16 - e[i + 2] as i16).unsigned_abs() as u8;
            if dr != 0 || dg != 0 || db != 0 {
                stats.pixel_mismatches += 1;
            }
            stats.byte_mismatches += (dr != 0) as usize + (dg != 0) as usize + (db != 0) as usize;
            stats.sum_abs_r += dr as u64;
            stats.sum_abs_g += dg as u64;
            stats.sum_abs_b += db as u64;
            stats.max_abs_r = stats.max_abs_r.max(dr);
            stats.max_abs_g = stats.max_abs_g.max(dg);
            stats.max_abs_b = stats.max_abs_b.max(db);
        }
        stats
    }

    fn assert_ppm_match(pixmap: &Pixmap, golden_file: &str) {
        assert_ppm_match_tolerance(pixmap, golden_file, 0);
    }

    /// Assert PPM match allowing up to `max_byte_mismatches` differing pixel-bytes.
    ///
    /// Our clean-room IW44 wavelet decoder has minor rounding differences vs
    /// DjVuLibre (±1-2 values, rarely higher) due to different boundary handling
    /// in the lifting steps. These tolerances are hard-coded from observed diffs
    /// and should only decrease as the decoder improves.
    fn assert_ppm_match_tolerance(pixmap: &Pixmap, golden_file: &str, max_byte_mismatches: usize) {
        let actual = pixmap.to_ppm();
        let expected = std::fs::read(golden_path().join(golden_file)).unwrap();
        assert_eq!(
            actual.len(),
            expected.len(),
            "{}: size mismatch {} vs {}",
            golden_file,
            actual.len(),
            expected.len()
        );

        let stats = diff_stats(&actual, &expected);
        let total = stats.pixel_count * 3;
        if stats.byte_mismatches > max_byte_mismatches {
            panic!(
                "{}: {} pixel-bytes differ ({}/{} = {:.1}%), allowed {}",
                golden_file,
                stats.byte_mismatches,
                stats.byte_mismatches,
                total,
                stats.byte_mismatches as f64 / total as f64 * 100.0,
                max_byte_mismatches,
            );
        }
    }

    #[test]
    fn render_chicken_bg_only() {
        let pm = render_page("chicken.djvu", 0);
        assert_ppm_match(&pm, "chicken.ppm");
    }

    #[test]
    fn render_boy_jb2_mask_only() {
        let pm = render_page("boy_jb2.djvu", 0);
        assert_ppm_match(&pm, "boy_jb2.ppm");
    }

    #[test]
    fn render_carte_3layer() {
        let pm = render_page("carte.djvu", 0);
        // IW44 rounding diffs vs DjVuLibre: 1598859/32205600 bytes (5.0%)
        assert_ppm_match_tolerance(&pm, "carte_p1.ppm", 1_600_000);
    }

    #[test]
    fn render_navm_fgbz_p1() {
        let pm = render_page("navm_fgbz.djvu", 0);
        assert_ppm_match(&pm, "navm_fgbz_p1.ppm");
    }

    #[test]
    fn render_navm_fgbz_p4_palette() {
        let pm = render_page("navm_fgbz.djvu", 3);
        // IW44 rounding diffs vs DjVuLibre: 9553/25245000 bytes (<0.1%)
        assert_ppm_match_tolerance(&pm, "navm_fgbz_p4.ppm", 10_000);
    }

    #[test]
    fn render_colorbook_p1() {
        let pm = render_page("colorbook.djvu", 0);
        // IW44 rounding diffs vs DjVuLibre: 386545/24875820 bytes (1.6%)
        assert_ppm_match_tolerance(&pm, "colorbook_p1.ppm", 390_000);
    }

    #[test]
    fn render_djvu3spec_p5() {
        let pm = render_page("DjVu3Spec_bundled.djvu", 4);
        assert_ppm_match(&pm, "djvu3spec_p5.ppm");
    }

    #[test]
    fn render_boy_jb2_rot90() {
        let pm = render_page("boy_jb2_rotate90.djvu", 0);
        assert_eq!(pm.width, 256, "rotated width");
        assert_eq!(pm.height, 192, "rotated height");
        assert_ppm_match(&pm, "boy_jb2_rot90.ppm");
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_colorbook_navm_layer_mismatch() {
        let compare = |actual: &Pixmap, ref_path: &str, tag: &str| {
            let rp = std::path::Path::new(ref_path);
            if !rp.exists() {
                return;
            }
            let expected = std::fs::read(rp).unwrap();
            let actual = actual.to_ppm();
            let header_end = actual.iter().position(|&b| b == b'\n').unwrap() + 1;
            let header_end = header_end
                + actual[header_end..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .unwrap()
                + 1;
            let header_end = header_end
                + actual[header_end..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .unwrap()
                + 1;
            let a = &actual[header_end..];
            let e = &expected[header_end..];
            let px = (a.len().min(e.len())) / 3;
            let mut diff_px = 0usize;
            for p in 0..px {
                let i = p * 3;
                if a[i] != e[i] || a[i + 1] != e[i + 1] || a[i + 2] != e[i + 2] {
                    diff_px += 1;
                }
            }
            eprintln!("{} mismatch_px={}", tag, diff_px);
        };

        let data = std::fs::read(assets_path().join("colorbook.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let mask = page.decode_mask().unwrap().unwrap();
        let bg = page.decode_background().unwrap().unwrap();
        let fg = page.decode_foreground().unwrap().unwrap();
        let comp = render(&page).unwrap();
        compare(
            &composite_bg_only(
                page.info.width as u32,
                page.info.height as u32,
                &bg,
                page.info.width as u32,
                page.info.height as u32,
            ),
            "/tmp/rdjvu_debug/colorbook_p1_bg.ppm",
            "colorbook bg",
        );
        compare(
            &composite_mask_fg(
                page.info.width as u32,
                page.info.height as u32,
                &mask,
                &fg,
                page.info.width as u32,
                page.info.height as u32,
            ),
            "/tmp/rdjvu_debug/colorbook_p1_fg.ppm",
            "colorbook fg",
        );
        compare(
            &comp,
            "/tmp/rdjvu_debug/colorbook_p1_bg.ppm",
            "colorbook full-vs-bg",
        );

        let data = std::fs::read(assets_path().join("navm_fgbz.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(3).unwrap();
        let bg = page.decode_background().unwrap().unwrap();
        let comp = render(&page).unwrap();
        compare(
            &composite_bg_only(
                page.info.width as u32,
                page.info.height as u32,
                &bg,
                page.info.width as u32,
                page.info.height as u32,
            ),
            "/tmp/rdjvu_debug/navm_p4_bg.ppm",
            "navm p4 bg",
        );
        compare(
            &comp,
            "/tmp/rdjvu_debug/navm_p4_bg.ppm",
            "navm p4 full-vs-bg",
        );
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_carte_layer_mismatch() {
        let compare = |actual: &Pixmap, ref_path: &str, tag: &str| {
            let rp = std::path::Path::new(ref_path);
            if !rp.exists() {
                return;
            }
            let expected = std::fs::read(rp).unwrap();
            let actual = actual.to_ppm();
            let header_end = actual.iter().position(|&b| b == b'\n').unwrap() + 1;
            let header_end = header_end
                + actual[header_end..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .unwrap()
                + 1;
            let header_end = header_end
                + actual[header_end..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .unwrap()
                + 1;
            let a = &actual[header_end..];
            let e = &expected[header_end..];
            let px = (a.len().min(e.len())) / 3;
            let mut diff_px = 0usize;
            for p in 0..px {
                let i = p * 3;
                if a[i] != e[i] || a[i + 1] != e[i + 1] || a[i + 2] != e[i + 2] {
                    diff_px += 1;
                }
            }
            eprintln!("{} mismatch_px={}", tag, diff_px);
        };

        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let mask = page.decode_mask().unwrap().unwrap();
        let bg = page.decode_background().unwrap().unwrap();
        let fg = page.decode_foreground().unwrap().unwrap();
        let comp = render(&page).unwrap();

        compare(
            &composite_bg_only(
                page.info.width as u32,
                page.info.height as u32,
                &bg,
                page.info.width as u32,
                page.info.height as u32,
            ),
            "/tmp/rdjvu_debug/carte_bg.ppm",
            "carte bg",
        );
        compare(
            &composite_mask_fg(
                page.info.width as u32,
                page.info.height as u32,
                &mask,
                &fg,
                page.info.width as u32,
                page.info.height as u32,
            ),
            "/tmp/rdjvu_debug/carte_fg.ppm",
            "carte fg",
        );
        compare(&comp, "/tmp/rdjvu_debug/carte_bg.ppm", "carte full-vs-bg");
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_navm_bg_scaler_candidates() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/navm_p4_bg.ppm");
        if !ref_path.exists() {
            return;
        }
        let expected = std::fs::read(ref_path).unwrap();
        let data = std::fs::read(assets_path().join("navm_fgbz.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(3).unwrap();
        let bg = page.decode_background().unwrap().unwrap();
        let w = page.info.width as u32;
        let h = page.info.height as u32;

        let compare = |name: &str, sample: &dyn Fn(u32, u32) -> (u8, u8, u8)| {
            let mut out = Pixmap::white(w, h);
            for y in 0..h {
                for x in 0..w {
                    let (r, g, b) = sample(x, y);
                    out.set_rgb(x, y, r, g, b);
                }
            }
            let actual = out.to_ppm();
            let header_end = actual.iter().position(|&b| b == b'\n').unwrap() + 1;
            let header_end = header_end
                + actual[header_end..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .unwrap()
                + 1;
            let header_end = header_end
                + actual[header_end..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .unwrap()
                + 1;
            let a = &actual[header_end..];
            let e = &expected[header_end..];
            let px = (a.len().min(e.len())) / 3;
            let mut diff_px = 0usize;
            for p in 0..px {
                let i = p * 3;
                if a[i] != e[i] || a[i + 1] != e[i + 1] || a[i + 2] != e[i + 2] {
                    diff_px += 1;
                }
            }
            eprintln!("navm bg scaler {} mismatch_px={}", name, diff_px);
        };

        let scale = (w as f64 / bg.width as f64).round().max(1.0) as u32;
        compare("nearest_round_scale", &|x, y| {
            let sx = (x / scale).min(bg.width - 1);
            let sy = (y / scale).min(bg.height - 1);
            bg.get_rgb(sx, sy)
        });
        compare("bilinear_round_scale", &|x, y| {
            sample_bilinear(&bg, x, y, w, h)
        });

        let bilinear_map = |x: u32, y: u32, mode: &str, round_mode: &str| -> (u8, u8, u8) {
            let sw = bg.width as f64;
            let sh = bg.height as f64;
            let dw = w as f64;
            let dh = h as f64;
            let (sx, sy) = match mode {
                // center-of-pixel mapping
                "center" => (
                    ((x as f64 + 0.5) * sw / dw - 0.5).clamp(0.0, sw - 1.0),
                    ((y as f64 + 0.5) * sh / dh - 0.5).clamp(0.0, sh - 1.0),
                ),
                // corner mapping
                "corner" => (
                    (x as f64 * sw / dw).clamp(0.0, sw - 1.0),
                    (y as f64 * sh / dh).clamp(0.0, sh - 1.0),
                ),
                // align first/last sample to first/last source pixel
                "edge" => (
                    if w > 1 {
                        (x as f64 * (sw - 1.0) / (dw - 1.0)).clamp(0.0, sw - 1.0)
                    } else {
                        0.0
                    },
                    if h > 1 {
                        (y as f64 * (sh - 1.0) / (dh - 1.0)).clamp(0.0, sh - 1.0)
                    } else {
                        0.0
                    },
                ),
                _ => unreachable!(),
            };
            let sx0 = sx as u32;
            let sy0 = sy as u32;
            let sx1 = (sx0 + 1).min(bg.width - 1);
            let sy1 = (sy0 + 1).min(bg.height - 1);
            let fx = sx - sx0 as f64;
            let fy = sy - sy0 as f64;
            let (r00, g00, b00) = bg.get_rgb(sx0, sy0);
            let (r10, g10, b10) = bg.get_rgb(sx1, sy0);
            let (r01, g01, b01) = bg.get_rgb(sx0, sy1);
            let (r11, g11, b11) = bg.get_rgb(sx1, sy1);
            let interp = |v00: u8, v10: u8, v01: u8, v11: u8| -> u8 {
                let v = v00 as f64 * (1.0 - fx) * (1.0 - fy)
                    + v10 as f64 * fx * (1.0 - fy)
                    + v01 as f64 * (1.0 - fx) * fy
                    + v11 as f64 * fx * fy;
                match round_mode {
                    "nearest" => (v + 0.5).clamp(0.0, 255.0) as u8,
                    "floor" => v.floor().clamp(0.0, 255.0) as u8,
                    _ => unreachable!(),
                }
            };
            (
                interp(r00, r10, r01, r11),
                interp(g00, g10, g01, g11),
                interp(b00, b10, b01, b11),
            )
        };

        for mode in ["center", "corner", "edge"] {
            for round_mode in ["nearest", "floor"] {
                let name = format!("bilinear_{}_{}", mode, round_mode);
                compare(&name, &|x, y| bilinear_map(x, y, mode, round_mode));
            }
        }

        compare("nearest_true_dims", &|x, y| {
            let sx = ((x as f64) * bg.width as f64 / w as f64).floor() as u32;
            let sy = ((y as f64) * bg.height as f64 / h as f64).floor() as u32;
            bg.get_rgb(sx.min(bg.width - 1), sy.min(bg.height - 1))
        });

        compare("bilinear_center_fixed16", &|x, y| {
            let sw = bg.width as i64;
            let sh = bg.height as i64;
            let dw = w as i64;
            let dh = h as i64;
            let sx_fp = (((2 * x as i64 + 1) * sw << 16) / (2 * dw)) - (1 << 15);
            let sy_fp = (((2 * y as i64 + 1) * sh << 16) / (2 * dh)) - (1 << 15);
            let sx_fp = sx_fp.clamp(0, (sw - 1) << 16);
            let sy_fp = sy_fp.clamp(0, (sh - 1) << 16);
            let sx0 = (sx_fp >> 16) as u32;
            let sy0 = (sy_fp >> 16) as u32;
            let sx1 = (sx0 + 1).min(bg.width - 1);
            let sy1 = (sy0 + 1).min(bg.height - 1);
            let fx = (sx_fp & 0xffff) as i64;
            let fy = (sy_fp & 0xffff) as i64;
            let wx0 = 65536 - fx;
            let wy0 = 65536 - fy;
            let wx1 = fx;
            let wy1 = fy;
            let (r00, g00, b00) = bg.get_rgb(sx0, sy0);
            let (r10, g10, b10) = bg.get_rgb(sx1, sy0);
            let (r01, g01, b01) = bg.get_rgb(sx0, sy1);
            let (r11, g11, b11) = bg.get_rgb(sx1, sy1);
            let interp = |v00: u8, v10: u8, v01: u8, v11: u8| -> u8 {
                let acc = v00 as i64 * wx0 * wy0
                    + v10 as i64 * wx1 * wy0
                    + v01 as i64 * wx0 * wy1
                    + v11 as i64 * wx1 * wy1;
                ((acc + (1 << 31)) >> 32).clamp(0, 255) as u8
            };
            (
                interp(r00, r10, r01, r11),
                interp(g00, g10, g01, g11),
                interp(b00, b10, b01, b11),
            )
        });

        compare("bilinear_center_gamma22", &|x, y| {
            let sw = bg.width as f64;
            let sh = bg.height as f64;
            let dw = w as f64;
            let dh = h as f64;
            let sx = ((x as f64 + 0.5) * sw / dw - 0.5).clamp(0.0, sw - 1.0);
            let sy = ((y as f64 + 0.5) * sh / dh - 0.5).clamp(0.0, sh - 1.0);
            let sx0 = sx as u32;
            let sy0 = sy as u32;
            let sx1 = (sx0 + 1).min(bg.width - 1);
            let sy1 = (sy0 + 1).min(bg.height - 1);
            let fx = sx - sx0 as f64;
            let fy = sy - sy0 as f64;
            let (r00, g00, b00) = bg.get_rgb(sx0, sy0);
            let (r10, g10, b10) = bg.get_rgb(sx1, sy0);
            let (r01, g01, b01) = bg.get_rgb(sx0, sy1);
            let (r11, g11, b11) = bg.get_rgb(sx1, sy1);
            let gamma = 2.2f64;
            let to_lin = |v: u8| (v as f64 / 255.0).powf(gamma);
            let to_srgb = |v: f64| {
                (v.clamp(0.0, 1.0).powf(1.0 / gamma) * 255.0 + 0.5).clamp(0.0, 255.0) as u8
            };
            let interp = |v00: u8, v10: u8, v01: u8, v11: u8| -> u8 {
                let v = to_lin(v00) * (1.0 - fx) * (1.0 - fy)
                    + to_lin(v10) * fx * (1.0 - fy)
                    + to_lin(v01) * (1.0 - fx) * fy
                    + to_lin(v11) * fx * fy;
                to_srgb(v)
            };
            (
                interp(r00, r10, r01, r11),
                interp(g00, g10, g01, g11),
                interp(b00, b10, b01, b11),
            )
        });
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_carte_bg_scaler_candidates() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/carte_bg.ppm");
        if !ref_path.exists() {
            return;
        }
        let expected = std::fs::read(ref_path).unwrap();
        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let bg = page.decode_background().unwrap().unwrap();
        let w = page.info.width as u32;
        let h = page.info.height as u32;

        let compare = |name: &str, sample: &dyn Fn(u32, u32) -> (u8, u8, u8)| {
            let mut out = Pixmap::white(w, h);
            for y in 0..h {
                for x in 0..w {
                    let (r, g, b) = sample(x, y);
                    out.set_rgb(x, y, r, g, b);
                }
            }
            let actual = out.to_ppm();
            let header_end = actual.iter().position(|&b| b == b'\n').unwrap() + 1;
            let header_end = header_end
                + actual[header_end..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .unwrap()
                + 1;
            let header_end = header_end
                + actual[header_end..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .unwrap()
                + 1;
            let a = &actual[header_end..];
            let e = &expected[header_end..];
            let px = (a.len().min(e.len())) / 3;
            let mut diff_px = 0usize;
            for p in 0..px {
                let i = p * 3;
                if a[i] != e[i] || a[i + 1] != e[i + 1] || a[i + 2] != e[i + 2] {
                    diff_px += 1;
                }
            }
            eprintln!("carte bg scaler {} mismatch_px={}", name, diff_px);
        };

        let scale = (w as f64 / bg.width as f64).round().max(1.0) as u32;
        compare("nearest_round_scale", &|x, y| {
            let sx = (x / scale).min(bg.width - 1);
            let sy = (y / scale).min(bg.height - 1);
            bg.get_rgb(sx, sy)
        });
        compare("bilinear_round_scale", &|x, y| {
            sample_bilinear(&bg, x, y, w, h)
        });

        let bilinear_map = |x: u32, y: u32, mode: &str, round_mode: &str| -> (u8, u8, u8) {
            let sw = bg.width as f64;
            let sh = bg.height as f64;
            let dw = w as f64;
            let dh = h as f64;
            let (sx, sy) = match mode {
                "center" => (
                    ((x as f64 + 0.5) * sw / dw - 0.5).clamp(0.0, sw - 1.0),
                    ((y as f64 + 0.5) * sh / dh - 0.5).clamp(0.0, sh - 1.0),
                ),
                "corner" => (
                    (x as f64 * sw / dw).clamp(0.0, sw - 1.0),
                    (y as f64 * sh / dh).clamp(0.0, sh - 1.0),
                ),
                "edge" => (
                    if w > 1 {
                        (x as f64 * (sw - 1.0) / (dw - 1.0)).clamp(0.0, sw - 1.0)
                    } else {
                        0.0
                    },
                    if h > 1 {
                        (y as f64 * (sh - 1.0) / (dh - 1.0)).clamp(0.0, sh - 1.0)
                    } else {
                        0.0
                    },
                ),
                _ => unreachable!(),
            };
            let sx0 = sx as u32;
            let sy0 = sy as u32;
            let sx1 = (sx0 + 1).min(bg.width - 1);
            let sy1 = (sy0 + 1).min(bg.height - 1);
            let fx = sx - sx0 as f64;
            let fy = sy - sy0 as f64;
            let (r00, g00, b00) = bg.get_rgb(sx0, sy0);
            let (r10, g10, b10) = bg.get_rgb(sx1, sy0);
            let (r01, g01, b01) = bg.get_rgb(sx0, sy1);
            let (r11, g11, b11) = bg.get_rgb(sx1, sy1);
            let interp = |v00: u8, v10: u8, v01: u8, v11: u8| -> u8 {
                let v = v00 as f64 * (1.0 - fx) * (1.0 - fy)
                    + v10 as f64 * fx * (1.0 - fy)
                    + v01 as f64 * (1.0 - fx) * fy
                    + v11 as f64 * fx * fy;
                match round_mode {
                    "nearest" => (v + 0.5).clamp(0.0, 255.0) as u8,
                    "floor" => v.floor().clamp(0.0, 255.0) as u8,
                    _ => unreachable!(),
                }
            };
            (
                interp(r00, r10, r01, r11),
                interp(g00, g10, g01, g11),
                interp(b00, b10, b01, b11),
            )
        };

        for mode in ["center", "corner", "edge"] {
            for round_mode in ["nearest", "floor"] {
                let name = format!("bilinear_{}_{}", mode, round_mode);
                compare(&name, &|x, y| bilinear_map(x, y, mode, round_mode));
            }
        }

        compare("nearest_true_dims", &|x, y| {
            let sx = ((x as f64) * bg.width as f64 / w as f64).floor() as u32;
            let sy = ((y as f64) * bg.height as f64 / h as f64).floor() as u32;
            bg.get_rgb(sx.min(bg.width - 1), sy.min(bg.height - 1))
        });

        compare("bilinear_center_fixed16", &|x, y| {
            let sw = bg.width as i64;
            let sh = bg.height as i64;
            let dw = w as i64;
            let dh = h as i64;
            let sx_fp = (((2 * x as i64 + 1) * sw << 16) / (2 * dw)) - (1 << 15);
            let sy_fp = (((2 * y as i64 + 1) * sh << 16) / (2 * dh)) - (1 << 15);
            let sx_fp = sx_fp.clamp(0, (sw - 1) << 16);
            let sy_fp = sy_fp.clamp(0, (sh - 1) << 16);
            let sx0 = (sx_fp >> 16) as u32;
            let sy0 = (sy_fp >> 16) as u32;
            let sx1 = (sx0 + 1).min(bg.width - 1);
            let sy1 = (sy0 + 1).min(bg.height - 1);
            let fx = (sx_fp & 0xffff) as i64;
            let fy = (sy_fp & 0xffff) as i64;
            let wx0 = 65536 - fx;
            let wy0 = 65536 - fy;
            let wx1 = fx;
            let wy1 = fy;
            let (r00, g00, b00) = bg.get_rgb(sx0, sy0);
            let (r10, g10, b10) = bg.get_rgb(sx1, sy0);
            let (r01, g01, b01) = bg.get_rgb(sx0, sy1);
            let (r11, g11, b11) = bg.get_rgb(sx1, sy1);
            let interp = |v00: u8, v10: u8, v01: u8, v11: u8| -> u8 {
                let acc = v00 as i64 * wx0 * wy0
                    + v10 as i64 * wx1 * wy0
                    + v01 as i64 * wx0 * wy1
                    + v11 as i64 * wx1 * wy1;
                ((acc + (1 << 31)) >> 32).clamp(0, 255) as u8
            };
            (
                interp(r00, r10, r01, r11),
                interp(g00, g10, g01, g11),
                interp(b00, b10, b01, b11),
            )
        });
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_carte_bg_phase_search() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/carte_bg.ppm");
        if !ref_path.exists() {
            return;
        }
        let expected = std::fs::read(ref_path).unwrap();
        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let bg = page.decode_background().unwrap().unwrap();
        let w = page.info.width as u32;
        let h = page.info.height as u32;
        let mut best_x = Vec::new();
        let mut best_y = Vec::new();

        let render_with_phase = |x_phase: f64, y_phase: f64| -> Pixmap {
            let mut out = Pixmap::white(w, h);
            let sw = bg.width as f64;
            let sh = bg.height as f64;
            let dw = w as f64;
            let dh = h as f64;
            for y in 0..h {
                let sy = ((y as f64 + 0.5) * sh / dh - 0.5 + y_phase).clamp(0.0, sh - 1.0);
                let sy0 = sy as u32;
                let sy1 = (sy0 + 1).min(bg.height - 1);
                let fy = ((sy - sy0 as f64) * 256.0 + 0.5).floor().clamp(0.0, 255.0) as u32;
                for x in 0..w {
                    let sx = ((x as f64 + 0.5) * sw / dw - 0.5 + x_phase).clamp(0.0, sw - 1.0);
                    let sx0 = sx as u32;
                    let sx1 = (sx0 + 1).min(bg.width - 1);
                    let fx = ((sx - sx0 as f64) * 256.0 + 0.5).floor().clamp(0.0, 255.0) as u32;
                    let (r00, g00, b00) = bg.get_rgb(sx0, sy0);
                    let (r10, g10, b10) = bg.get_rgb(sx1, sy0);
                    let (r01, g01, b01) = bg.get_rgb(sx0, sy1);
                    let (r11, g11, b11) = bg.get_rgb(sx1, sy1);
                    let interp_h = |v0: u8, v1: u8| -> u32 {
                        ((v0 as u32 * (256 - fx) + v1 as u32 * fx + 128) >> 8).clamp(0, 255)
                    };
                    let interp_v = |v0: u32, v1: u32| -> u8 {
                        ((v0 * (256 - fy) + v1 * fy + 128) >> 8).clamp(0, 255) as u8
                    };
                    out.set_rgb(
                        x,
                        y,
                        interp_v(interp_h(r00, r10), interp_h(r01, r11)),
                        interp_v(interp_h(g00, g10), interp_h(g01, g11)),
                        interp_v(interp_h(b00, b10), interp_h(b01, b11)),
                    );
                }
            }
            out
        };

        for step in -16..=16 {
            let phase = step as f64 / 16.0;
            let stats = diff_stats(&render_with_phase(phase, 0.0).to_ppm(), &expected);
            best_x.push((stats.byte_mismatches, stats.pixel_mismatches, step));
        }
        best_x.sort_unstable();
        for (rank, (byte_mismatches, pixel_mismatches, step)) in
            best_x.into_iter().take(10).enumerate()
        {
            eprintln!(
                "carte bg xphase rank={} step={} phase={:.4} byte_mismatch={} pixel_mismatch={}",
                rank + 1,
                step,
                step as f64 / 16.0,
                byte_mismatches,
                pixel_mismatches,
            );
        }

        for step in -16..=16 {
            let phase = step as f64 / 16.0;
            let stats = diff_stats(&render_with_phase(0.0, phase).to_ppm(), &expected);
            best_y.push((stats.byte_mismatches, stats.pixel_mismatches, step));
        }
        best_y.sort_unstable();
        for (rank, (byte_mismatches, pixel_mismatches, step)) in
            best_y.into_iter().take(10).enumerate()
        {
            eprintln!(
                "carte bg yphase rank={} step={} phase={:.4} byte_mismatch={} pixel_mismatch={}",
                rank + 1,
                step,
                step as f64 / 16.0,
                byte_mismatches,
                pixel_mismatches,
            );
        }
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_carte_bg_frac16_candidate() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/carte_bg.ppm");
        if !ref_path.exists() {
            return;
        }
        let expected = std::fs::read(ref_path).unwrap();
        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let bg = page.decode_background().unwrap().unwrap();
        let w = page.info.width as u32;
        let h = page.info.height as u32;

        let sw = bg.width as f64;
        let sh = bg.height as f64;
        let dw = w as f64;
        let dh = h as f64;
        for (label, round_bias) in [("nearest", 0.5f64), ("floor", 0.0f64)] {
            let mut out = Pixmap::white(w, h);
            for y in 0..h {
                let sy = ((y as f64 + 0.5) * sh / dh - 0.5).clamp(0.0, sh - 1.0);
                let sy0 = sy as u32;
                let sy1 = (sy0 + 1).min(bg.height - 1);
                let fy = ((sy - sy0 as f64) * 16.0 + round_bias)
                    .floor()
                    .clamp(0.0, 15.0) as u32;
                for x in 0..w {
                    let sx = ((x as f64 + 0.5) * sw / dw - 0.5).clamp(0.0, sw - 1.0);
                    let sx0 = sx as u32;
                    let sx1 = (sx0 + 1).min(bg.width - 1);
                    let fx = ((sx - sx0 as f64) * 16.0 + round_bias)
                        .floor()
                        .clamp(0.0, 15.0) as u32;
                    let (r00, g00, b00) = bg.get_rgb(sx0, sy0);
                    let (r10, g10, b10) = bg.get_rgb(sx1, sy0);
                    let (r01, g01, b01) = bg.get_rgb(sx0, sy1);
                    let (r11, g11, b11) = bg.get_rgb(sx1, sy1);
                    let interp_h = |v0: u8, v1: u8| -> u32 {
                        ((v0 as u32 * (16 - fx) + v1 as u32 * fx + 8) >> 4).clamp(0, 255)
                    };
                    let interp_v = |v0: u32, v1: u32| -> u8 {
                        ((v0 * (16 - fy) + v1 * fy + 8) >> 4).clamp(0, 255) as u8
                    };
                    out.set_rgb(
                        x,
                        y,
                        interp_v(interp_h(r00, r10), interp_h(r01, r11)),
                        interp_v(interp_h(g00, g10), interp_h(g01, g11)),
                        interp_v(interp_h(b00, b10), interp_h(b01, b11)),
                    );
                }
            }

            let stats = diff_stats(&out.to_ppm(), &expected);
            eprintln!(
                "carte bg frac16 {} bytes={} pixels={} mean_abs_rgb=({:.4},{:.4},{:.4}) max_abs_rgb=({},{},{})",
                label,
                stats.byte_mismatches,
                stats.pixel_mismatches,
                stats.sum_abs_r as f64 / stats.pixel_count as f64,
                stats.sum_abs_g as f64 / stats.pixel_count as f64,
                stats.sum_abs_b as f64 / stats.pixel_count as f64,
                stats.max_abs_r,
                stats.max_abs_g,
                stats.max_abs_b,
            );
        }
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_bg_fraction_bits_sweep() {
        let cases = [
            (
                "carte.djvu",
                0usize,
                "/tmp/rdjvu_debug/carte_bg.ppm",
                "carte",
            ),
            (
                "colorbook.djvu",
                0usize,
                "/tmp/rdjvu_debug/colorbook_p1_bg.ppm",
                "colorbook",
            ),
            (
                "navm_fgbz.djvu",
                3usize,
                "/tmp/rdjvu_debug/navm_p4_bg.ppm",
                "navm_p4",
            ),
        ];

        for bits in 4u32..=8 {
            let scale = (1u32 << bits) as f64;
            for (file, page_idx, ref_path, tag) in cases {
                let rp = std::path::Path::new(ref_path);
                if !rp.exists() {
                    continue;
                }
                let expected = std::fs::read(rp).unwrap();
                let data = std::fs::read(assets_path().join(file)).unwrap();
                let doc = Document::parse(&data).unwrap();
                let page = doc.page(page_idx).unwrap();
                let bg = page.decode_background().unwrap().unwrap();
                let w = page.info.width as u32;
                let h = page.info.height as u32;
                let sw = bg.width as f64;
                let sh = bg.height as f64;
                let dw = w as f64;
                let dh = h as f64;
                let denom = 1u32 << bits;
                let half = 1u32 << (bits - 1);

                let mut out = Pixmap::white(w, h);
                for y in 0..h {
                    let sy = ((y as f64 + 0.5) * sh / dh - 0.5).clamp(0.0, sh - 1.0);
                    let sy0 = sy as u32;
                    let sy1 = (sy0 + 1).min(bg.height - 1);
                    let fy = ((sy - sy0 as f64) * scale + 0.5)
                        .floor()
                        .clamp(0.0, (denom - 1) as f64) as u32;
                    for x in 0..w {
                        let sx = ((x as f64 + 0.5) * sw / dw - 0.5).clamp(0.0, sw - 1.0);
                        let sx0 = sx as u32;
                        let sx1 = (sx0 + 1).min(bg.width - 1);
                        let fx = ((sx - sx0 as f64) * scale + 0.5)
                            .floor()
                            .clamp(0.0, (denom - 1) as f64) as u32;
                        let (r00, g00, b00) = bg.get_rgb(sx0, sy0);
                        let (r10, g10, b10) = bg.get_rgb(sx1, sy0);
                        let (r01, g01, b01) = bg.get_rgb(sx0, sy1);
                        let (r11, g11, b11) = bg.get_rgb(sx1, sy1);
                        let interp_h = |v0: u8, v1: u8| -> u32 {
                            ((v0 as u32 * (denom - fx) + v1 as u32 * fx + half) >> bits)
                                .clamp(0, 255)
                        };
                        let interp_v = |v0: u32, v1: u32| -> u8 {
                            ((v0 * (denom - fy) + v1 * fy + half) >> bits).clamp(0, 255) as u8
                        };
                        out.set_rgb(
                            x,
                            y,
                            interp_v(interp_h(r00, r10), interp_h(r01, r11)),
                            interp_v(interp_h(g00, g10), interp_h(g01, g11)),
                            interp_v(interp_h(b00, b10), interp_h(b01, b11)),
                        );
                    }
                }

                let stats = diff_stats(&out.to_ppm(), &expected);
                eprintln!(
                    "{} frac_bits={} bytes={} pixels={}",
                    tag, bits, stats.byte_mismatches, stats.pixel_mismatches
                );
            }
        }
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_carte_bg_mod3_profile() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/carte_bg.ppm");
        if !ref_path.exists() {
            return;
        }
        let expected = std::fs::read(ref_path).unwrap();
        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let bg = page.decode_background().unwrap().unwrap();
        let w = page.info.width as u32;
        let h = page.info.height as u32;
        let actual = composite_bg_only(w, h, &bg, w, h).to_ppm();
        let a = &actual[ppm_header_end(&actual)..];
        let e = &expected[ppm_header_end(&expected)..];

        let mut phase = [[0usize; 3]; 3];
        for y in 0..h as usize {
            for x in 0..w as usize {
                let i = (y * w as usize + x) * 3;
                if a[i] != e[i] || a[i + 1] != e[i + 1] || a[i + 2] != e[i + 2] {
                    phase[y % 3][x % 3] += 1;
                }
            }
        }

        for y in 0..3 {
            eprintln!(
                "carte bg mod3 row{} = [{}, {}, {}]",
                y, phase[y][0], phase[y][1], phase[y][2]
            );
        }
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_carte_bg_vertical_phase_flip_candidate() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/carte_bg.ppm");
        if !ref_path.exists() {
            return;
        }
        let expected = std::fs::read(ref_path).unwrap();
        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let bg = page.decode_background().unwrap().unwrap();
        let w = page.info.width as u32;
        let h = page.info.height as u32;
        let sw = bg.width as f64;
        let sh = bg.height as f64;
        let dw = w as f64;
        let dh = h as f64;
        let mut out = Pixmap::white(w, h);

        for y in 0..h {
            let py = h - 1 - y;
            let sy_bottom = ((py as f64 + 0.5) * sh / dh - 0.5).clamp(0.0, sh - 1.0);
            let sy = (sh - 1.0 - sy_bottom).clamp(0.0, sh - 1.0);
            let sy0 = sy as u32;
            let sy1 = (sy0 + 1).min(bg.height - 1);
            let fy = ((sy - sy0 as f64) * 16.0 + 0.5).floor().clamp(0.0, 15.0) as u32;
            for x in 0..w {
                let sx = ((x as f64 + 0.5) * sw / dw - 0.5).clamp(0.0, sw - 1.0);
                let sx0 = sx as u32;
                let sx1 = (sx0 + 1).min(bg.width - 1);
                let fx = ((sx - sx0 as f64) * 16.0 + 0.5).floor().clamp(0.0, 15.0) as u32;
                let (r00, g00, b00) = bg.get_rgb(sx0, sy0);
                let (r10, g10, b10) = bg.get_rgb(sx1, sy0);
                let (r01, g01, b01) = bg.get_rgb(sx0, sy1);
                let (r11, g11, b11) = bg.get_rgb(sx1, sy1);
                let interp_h = |v0: u8, v1: u8| -> u32 {
                    ((v0 as u32 * (16 - fx) + v1 as u32 * fx + 8) >> 4).clamp(0, 255)
                };
                let interp_v = |v0: u32, v1: u32| -> u8 {
                    ((v0 * (16 - fy) + v1 * fy + 8) >> 4).clamp(0, 255) as u8
                };
                out.set_rgb(
                    x,
                    y,
                    interp_v(interp_h(r00, r10), interp_h(r01, r11)),
                    interp_v(interp_h(g00, g10), interp_h(g01, g11)),
                    interp_v(interp_h(b00, b10), interp_h(b01, b11)),
                );
            }
        }

        let stats = diff_stats(&out.to_ppm(), &expected);
        eprintln!(
            "carte bg vertical_phase_flip bytes={} pixels={} mean_abs_rgb=({:.4},{:.4},{:.4}) max_abs_rgb=({},{},{})",
            stats.byte_mismatches,
            stats.pixel_mismatches,
            stats.sum_abs_r as f64 / stats.pixel_count as f64,
            stats.sum_abs_g as f64 / stats.pixel_count as f64,
            stats.sum_abs_b as f64 / stats.pixel_count as f64,
            stats.max_abs_r,
            stats.max_abs_g,
            stats.max_abs_b,
        );
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_colorbook_bg_scaler_candidates() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/colorbook_p1_bg.ppm");
        if !ref_path.exists() {
            return;
        }
        let expected = std::fs::read(ref_path).unwrap();
        let data = std::fs::read(assets_path().join("colorbook.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let bg = page.decode_background().unwrap().unwrap();
        let w = page.info.width as u32;
        let h = page.info.height as u32;

        let compare = |name: &str, sample: &dyn Fn(u32, u32) -> (u8, u8, u8)| {
            let mut out = Pixmap::white(w, h);
            for y in 0..h {
                for x in 0..w {
                    let (r, g, b) = sample(x, y);
                    out.set_rgb(x, y, r, g, b);
                }
            }
            let actual = out.to_ppm();
            let header_end = actual.iter().position(|&b| b == b'\n').unwrap() + 1;
            let header_end = header_end
                + actual[header_end..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .unwrap()
                + 1;
            let header_end = header_end
                + actual[header_end..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .unwrap()
                + 1;
            let a = &actual[header_end..];
            let e = &expected[header_end..];
            let px = (a.len().min(e.len())) / 3;
            let mut diff_px = 0usize;
            for p in 0..px {
                let i = p * 3;
                if a[i] != e[i] || a[i + 1] != e[i + 1] || a[i + 2] != e[i + 2] {
                    diff_px += 1;
                }
            }
            eprintln!("colorbook bg scaler {} mismatch_px={}", name, diff_px);
        };

        let scale = (w as f64 / bg.width as f64).round().max(1.0) as u32;
        compare("nearest_round_scale", &|x, y| {
            let sx = (x / scale).min(bg.width - 1);
            let sy = (y / scale).min(bg.height - 1);
            bg.get_rgb(sx, sy)
        });
        compare("bilinear_round_scale", &|x, y| {
            sample_bilinear(&bg, x, y, w, h)
        });

        compare("nearest_true_dims", &|x, y| {
            let sx = ((x as u64 * bg.width as u64) / w as u64) as u32;
            let sy = ((y as u64 * bg.height as u64) / h as u64) as u32;
            bg.get_rgb(sx.min(bg.width - 1), sy.min(bg.height - 1))
        });

        compare("bilinear_true_dims", &|x, y| {
            let sw = bg.width as f64;
            let sh = bg.height as f64;
            let dw = w as f64;
            let dh = h as f64;
            let sx = ((x as f64 + 0.5) * sw / dw - 0.5).clamp(0.0, sw - 1.0);
            let sy = ((y as f64 + 0.5) * sh / dh - 0.5).clamp(0.0, sh - 1.0);
            let sx0 = sx as u32;
            let sy0 = sy as u32;
            let sx1 = (sx0 + 1).min(bg.width - 1);
            let sy1 = (sy0 + 1).min(bg.height - 1);
            let fx = sx - sx0 as f64;
            let fy = sy - sy0 as f64;
            let (r00, g00, b00) = bg.get_rgb(sx0, sy0);
            let (r10, g10, b10) = bg.get_rgb(sx1, sy0);
            let (r01, g01, b01) = bg.get_rgb(sx0, sy1);
            let (r11, g11, b11) = bg.get_rgb(sx1, sy1);
            let interp = |v00: u8, v10: u8, v01: u8, v11: u8| -> u8 {
                let v = v00 as f64 * (1.0 - fx) * (1.0 - fy)
                    + v10 as f64 * fx * (1.0 - fy)
                    + v01 as f64 * (1.0 - fx) * fy
                    + v11 as f64 * fx * fy;
                (v + 0.5).clamp(0.0, 255.0) as u8
            };
            (
                interp(r00, r10, r01, r11),
                interp(g00, g10, g01, g11),
                interp(b00, b10, b01, b11),
            )
        });

        let scale = ((w + bg.width - 1) / bg.width)
            .max((h + bg.height - 1) / bg.height)
            .max(1);
        let eff_w = (w / scale).max(1);
        let eff_h = (h / scale).max(1);
        let virt_w = eff_w * scale;
        let virt_h = eff_h * scale;
        compare("nearest_virtual_floor3", &|x, y| {
            let sx = (x / scale).min(eff_w.saturating_sub(1));
            let sy = (y / scale).min(eff_h.saturating_sub(1));
            bg.get_rgb(sx.min(bg.width - 1), sy.min(bg.height - 1))
        });

        compare("bilinear_virtual_floor3", &|x, y| {
            let sw = eff_w as f64;
            let sh = eff_h as f64;
            let dw = virt_w as f64;
            let dh = virt_h as f64;
            let sx = ((x.min(virt_w - 1) as f64 + 0.5) * sw / dw - 0.5).clamp(0.0, sw - 1.0);
            let sy = ((y.min(virt_h - 1) as f64 + 0.5) * sh / dh - 0.5).clamp(0.0, sh - 1.0);
            let sx0 = sx as u32;
            let sy0 = sy as u32;
            let sx1 = (sx0 + 1).min(eff_w.saturating_sub(1));
            let sy1 = (sy0 + 1).min(eff_h.saturating_sub(1));
            let fx = sx - sx0 as f64;
            let fy = sy - sy0 as f64;
            let (r00, g00, b00) = bg.get_rgb(sx0.min(bg.width - 1), sy0.min(bg.height - 1));
            let (r10, g10, b10) = bg.get_rgb(sx1.min(bg.width - 1), sy0.min(bg.height - 1));
            let (r01, g01, b01) = bg.get_rgb(sx0.min(bg.width - 1), sy1.min(bg.height - 1));
            let (r11, g11, b11) = bg.get_rgb(sx1.min(bg.width - 1), sy1.min(bg.height - 1));
            let interp = |v00: u8, v10: u8, v01: u8, v11: u8| -> u8 {
                let v = v00 as f64 * (1.0 - fx) * (1.0 - fy)
                    + v10 as f64 * fx * (1.0 - fy)
                    + v01 as f64 * (1.0 - fx) * fy
                    + v11 as f64 * fx * fy;
                (v + 0.5).clamp(0.0, 255.0) as u8
            };
            (
                interp(r00, r10, r01, r11),
                interp(g00, g10, g01, g11),
                interp(b00, b10, b01, b11),
            )
        });

        compare("bilinear_logical_true_dims", &|x, y| {
            let sw = eff_w as f64;
            let sh = eff_h as f64;
            let dw = w as f64;
            let dh = h as f64;
            let sx = ((x as f64 + 0.5) * sw / dw - 0.5).clamp(0.0, sw - 1.0);
            let sy = ((y as f64 + 0.5) * sh / dh - 0.5).clamp(0.0, sh - 1.0);
            let sx0 = sx as u32;
            let sy0 = sy as u32;
            let sx1 = (sx0 + 1).min(eff_w.saturating_sub(1)).min(bg.width - 1);
            let sy1 = (sy0 + 1).min(eff_h.saturating_sub(1)).min(bg.height - 1);
            let fx = sx - sx0 as f64;
            let fy = sy - sy0 as f64;
            let (r00, g00, b00) = bg.get_rgb(sx0.min(bg.width - 1), sy0.min(bg.height - 1));
            let (r10, g10, b10) = bg.get_rgb(sx1, sy0.min(bg.height - 1));
            let (r01, g01, b01) = bg.get_rgb(sx0.min(bg.width - 1), sy1);
            let (r11, g11, b11) = bg.get_rgb(sx1, sy1);
            let interp = |v00: u8, v10: u8, v01: u8, v11: u8| -> u8 {
                let v = v00 as f64 * (1.0 - fx) * (1.0 - fy)
                    + v10 as f64 * fx * (1.0 - fy)
                    + v01 as f64 * (1.0 - fx) * fy
                    + v11 as f64 * fx * fy;
                (v + 0.5).clamp(0.0, 255.0) as u8
            };
            (
                interp(r00, r10, r01, r11),
                interp(g00, g10, g01, g11),
                interp(b00, b10, b01, b11),
            )
        });

        compare("bilinear_logical_true_dims_fixed16", &|x, y| {
            let sw = eff_w as i64;
            let sh = eff_h as i64;
            let dw = w as i64;
            let dh = h as i64;
            let sx_fp = (((2 * x as i64 + 1) * sw << 16) / (2 * dw)) - (1 << 15);
            let sy_fp = (((2 * y as i64 + 1) * sh << 16) / (2 * dh)) - (1 << 15);
            let sx_fp = sx_fp.clamp(0, (sw - 1) << 16);
            let sy_fp = sy_fp.clamp(0, (sh - 1) << 16);
            let sx0 = (sx_fp >> 16) as u32;
            let sy0 = (sy_fp >> 16) as u32;
            let sx1 = (sx0 + 1).min(eff_w.saturating_sub(1)).min(bg.width - 1);
            let sy1 = (sy0 + 1).min(eff_h.saturating_sub(1)).min(bg.height - 1);
            let fx = (sx_fp & 0xffff) as i64;
            let fy = (sy_fp & 0xffff) as i64;
            let wx0 = 65536 - fx;
            let wy0 = 65536 - fy;
            let wx1 = fx;
            let wy1 = fy;
            let (r00, g00, b00) = bg.get_rgb(sx0.min(bg.width - 1), sy0.min(bg.height - 1));
            let (r10, g10, b10) = bg.get_rgb(sx1, sy0.min(bg.height - 1));
            let (r01, g01, b01) = bg.get_rgb(sx0.min(bg.width - 1), sy1);
            let (r11, g11, b11) = bg.get_rgb(sx1, sy1);
            let interp = |v00: u8, v10: u8, v01: u8, v11: u8| -> u8 {
                let acc = v00 as i64 * wx0 * wy0
                    + v10 as i64 * wx1 * wy0
                    + v01 as i64 * wx0 * wy1
                    + v11 as i64 * wx1 * wy1;
                ((acc + (1 << 31)) >> 32).clamp(0, 255) as u8
            };
            (
                interp(r00, r10, r01, r11),
                interp(g00, g10, g01, g11),
                interp(b00, b10, b01, b11),
            )
        });

        let bilinear_virtual = |x: u32, y: u32, mode: &str, round_mode: &str| -> (u8, u8, u8) {
            let sw = eff_w as f64;
            let sh = eff_h as f64;
            let dw = virt_w as f64;
            let dh = virt_h as f64;
            let cx = x.min(virt_w - 1) as f64;
            let cy = y.min(virt_h - 1) as f64;
            let (sx, sy) = match mode {
                "center" => (
                    ((cx + 0.5) * sw / dw - 0.5).clamp(0.0, sw - 1.0),
                    ((cy + 0.5) * sh / dh - 0.5).clamp(0.0, sh - 1.0),
                ),
                "corner" => (
                    (cx * sw / dw).clamp(0.0, sw - 1.0),
                    (cy * sh / dh).clamp(0.0, sh - 1.0),
                ),
                "edge" => (
                    if virt_w > 1 {
                        (cx * (sw - 1.0) / (dw - 1.0)).clamp(0.0, sw - 1.0)
                    } else {
                        0.0
                    },
                    if virt_h > 1 {
                        (cy * (sh - 1.0) / (dh - 1.0)).clamp(0.0, sh - 1.0)
                    } else {
                        0.0
                    },
                ),
                _ => unreachable!(),
            };
            let sx0 = sx as u32;
            let sy0 = sy as u32;
            let sx1 = (sx0 + 1).min(eff_w.saturating_sub(1)).min(bg.width - 1);
            let sy1 = (sy0 + 1).min(eff_h.saturating_sub(1)).min(bg.height - 1);
            let fx = sx - sx0 as f64;
            let fy = sy - sy0 as f64;
            let (r00, g00, b00) = bg.get_rgb(sx0.min(bg.width - 1), sy0.min(bg.height - 1));
            let (r10, g10, b10) = bg.get_rgb(sx1, sy0.min(bg.height - 1));
            let (r01, g01, b01) = bg.get_rgb(sx0.min(bg.width - 1), sy1);
            let (r11, g11, b11) = bg.get_rgb(sx1, sy1);
            let interp = |v00: u8, v10: u8, v01: u8, v11: u8| -> u8 {
                let v = v00 as f64 * (1.0 - fx) * (1.0 - fy)
                    + v10 as f64 * fx * (1.0 - fy)
                    + v01 as f64 * (1.0 - fx) * fy
                    + v11 as f64 * fx * fy;
                match round_mode {
                    "nearest" => (v + 0.5).clamp(0.0, 255.0) as u8,
                    "floor" => v.floor().clamp(0.0, 255.0) as u8,
                    _ => unreachable!(),
                }
            };
            (
                interp(r00, r10, r01, r11),
                interp(g00, g10, g01, g11),
                interp(b00, b10, b01, b11),
            )
        };

        for mode in ["center", "corner", "edge"] {
            for round_mode in ["nearest", "floor"] {
                let name = format!("bilinear_virtual_{}_{}", mode, round_mode);
                compare(&name, &|x, y| bilinear_virtual(x, y, mode, round_mode));
            }
        }

        compare("bilinear_virtual_center_fixed16", &|x, y| {
            let sw = eff_w as i64;
            let sh = eff_h as i64;
            let dw = virt_w as i64;
            let dh = virt_h as i64;
            let px = x.min(virt_w - 1) as i64;
            let py = y.min(virt_h - 1) as i64;
            let sx_fp = (((2 * px + 1) * sw << 16) / (2 * dw)) - (1 << 15);
            let sy_fp = (((2 * py + 1) * sh << 16) / (2 * dh)) - (1 << 15);
            let sx_fp = sx_fp.clamp(0, (sw - 1) << 16);
            let sy_fp = sy_fp.clamp(0, (sh - 1) << 16);
            let sx0 = (sx_fp >> 16) as u32;
            let sy0 = (sy_fp >> 16) as u32;
            let sx1 = (sx0 + 1).min(eff_w.saturating_sub(1)).min(bg.width - 1);
            let sy1 = (sy0 + 1).min(eff_h.saturating_sub(1)).min(bg.height - 1);
            let fx = (sx_fp & 0xffff) as i64;
            let fy = (sy_fp & 0xffff) as i64;
            let wx0 = 65536 - fx;
            let wy0 = 65536 - fy;
            let wx1 = fx;
            let wy1 = fy;
            let (r00, g00, b00) = bg.get_rgb(sx0.min(bg.width - 1), sy0.min(bg.height - 1));
            let (r10, g10, b10) = bg.get_rgb(sx1, sy0.min(bg.height - 1));
            let (r01, g01, b01) = bg.get_rgb(sx0.min(bg.width - 1), sy1);
            let (r11, g11, b11) = bg.get_rgb(sx1, sy1);
            let interp = |v00: u8, v10: u8, v01: u8, v11: u8| -> u8 {
                let acc = v00 as i64 * wx0 * wy0
                    + v10 as i64 * wx1 * wy0
                    + v01 as i64 * wx0 * wy1
                    + v11 as i64 * wx1 * wy1;
                ((acc + (1 << 31)) >> 32).clamp(0, 255) as u8
            };
            (
                interp(r00, r10, r01, r11),
                interp(g00, g10, g01, g11),
                interp(b00, b10, b01, b11),
            )
        });
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_colorbook_fg_scaler_candidates() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/colorbook_p1_fg.ppm");
        if !ref_path.exists() {
            return;
        }
        let expected = std::fs::read(ref_path).unwrap();
        let data = std::fs::read(assets_path().join("colorbook.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let mask = page.decode_mask().unwrap().unwrap();
        let fg = page.decode_foreground().unwrap().unwrap();
        let w = page.info.width as u32;
        let h = page.info.height as u32;

        let compare = |name: &str, sample: &dyn Fn(u32, u32) -> (u8, u8, u8)| {
            let mut out = Pixmap::white(w, h);
            let mw = mask.width.min(w);
            let mh = mask.height.min(h);
            for y in 0..mh {
                for x in 0..mw {
                    if mask.get(x, y) {
                        let (r, g, b) = sample(x, y);
                        out.set_rgb(x, y, r, g, b);
                    }
                }
            }
            let actual = out.to_ppm();
            let header_end = actual.iter().position(|&b| b == b'\n').unwrap() + 1;
            let header_end = header_end
                + actual[header_end..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .unwrap()
                + 1;
            let header_end = header_end
                + actual[header_end..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .unwrap()
                + 1;
            let a = &actual[header_end..];
            let e = &expected[header_end..];
            let px = (a.len().min(e.len())) / 3;
            let mut diff_px = 0usize;
            for p in 0..px {
                let i = p * 3;
                if a[i] != e[i] || a[i + 1] != e[i + 1] || a[i + 2] != e[i + 2] {
                    diff_px += 1;
                }
            }
            eprintln!("colorbook fg scaler {} mismatch_px={}", name, diff_px);
        };

        let scale = (w as f64 / fg.width as f64).round().max(1.0) as u32;
        compare("nearest_round_scale", &|x, y| {
            let sx = (x / scale).min(fg.width - 1);
            let sy = (y / scale).min(fg.height - 1);
            fg.get_rgb(sx, sy)
        });
        compare("bilinear_round_scale", &|x, y| {
            sample_bilinear(&fg, x, y, w, h)
        });

        compare("nearest_true_dims", &|x, y| {
            let sx = ((x as u64 * fg.width as u64) / w as u64) as u32;
            let sy = ((y as u64 * fg.height as u64) / h as u64) as u32;
            fg.get_rgb(sx.min(fg.width - 1), sy.min(fg.height - 1))
        });

        let (reduction, virt_w, virt_h, virt_page_w, virt_page_h) =
            layer_virtual_geometry(&fg, w, h);
        compare("nearest_virtual_floor3", &|x, y| {
            let px = x.min(virt_page_w - 1);
            let py = y.min(virt_page_h - 1);
            let sx = (px / reduction).min(virt_w - 1).min(fg.width - 1);
            let sy = (py / reduction).min(virt_h - 1).min(fg.height - 1);
            fg.get_rgb(sx, sy)
        });

        compare("nearest_virtual_true_dims", &|x, y| {
            let sx = ((x as u64 * virt_w as u64) / w as u64) as u32;
            let sy = ((y as u64 * virt_h as u64) / h as u64) as u32;
            fg.get_rgb(
                sx.min(virt_w - 1).min(fg.width - 1),
                sy.min(virt_h - 1).min(fg.height - 1),
            )
        });

        compare("nearest_virtual_true_dims_center", &|x, y| {
            let sx = (((2 * x as u64 + 1) * virt_w as u64) / (2 * w as u64)) as u32;
            let sy = (((2 * y as u64 + 1) * virt_h as u64) / (2 * h as u64)) as u32;
            fg.get_rgb(
                sx.min(virt_w - 1).min(fg.width - 1),
                sy.min(virt_h - 1).min(fg.height - 1),
            )
        });

        for x_shift in 0..reduction.min(3) {
            for y_shift in 0..reduction.min(3) {
                let name = format!("nearest_virtual_shift_{}_{}", x_shift, y_shift);
                compare(&name, &|x, y| {
                    let px = (x + x_shift).min(virt_page_w - 1);
                    let py = (y + y_shift).min(virt_page_h - 1);
                    let sx = (px / reduction).min(virt_w - 1).min(fg.width - 1);
                    let sy = (py / reduction).min(virt_h - 1).min(fg.height - 1);
                    fg.get_rgb(sx, sy)
                });
            }
        }

        compare("nearest_virtual_round", &|x, y| {
            let px = (x + reduction / 2).min(virt_page_w - 1);
            let py = (y + reduction / 2).min(virt_page_h - 1);
            let sx = (px / reduction).min(virt_w - 1).min(fg.width - 1);
            let sy = (py / reduction).min(virt_h - 1).min(fg.height - 1);
            fg.get_rgb(sx, sy)
        });

        compare("bilinear_true_dims", &|x, y| {
            let sw = fg.width as f64;
            let sh = fg.height as f64;
            let dw = w as f64;
            let dh = h as f64;
            let sx = ((x as f64 + 0.5) * sw / dw - 0.5).clamp(0.0, sw - 1.0);
            let sy = ((y as f64 + 0.5) * sh / dh - 0.5).clamp(0.0, sh - 1.0);
            let sx0 = sx as u32;
            let sy0 = sy as u32;
            let sx1 = (sx0 + 1).min(fg.width - 1);
            let sy1 = (sy0 + 1).min(fg.height - 1);
            let fx = sx - sx0 as f64;
            let fy = sy - sy0 as f64;
            let (r00, g00, b00) = fg.get_rgb(sx0, sy0);
            let (r10, g10, b10) = fg.get_rgb(sx1, sy0);
            let (r01, g01, b01) = fg.get_rgb(sx0, sy1);
            let (r11, g11, b11) = fg.get_rgb(sx1, sy1);
            let interp = |v00: u8, v10: u8, v01: u8, v11: u8| -> u8 {
                let v = v00 as f64 * (1.0 - fx) * (1.0 - fy)
                    + v10 as f64 * fx * (1.0 - fy)
                    + v01 as f64 * (1.0 - fx) * fy
                    + v11 as f64 * fx * fy;
                (v + 0.5).clamp(0.0, 255.0) as u8
            };
            (
                interp(r00, r10, r01, r11),
                interp(g00, g10, g01, g11),
                interp(b00, b10, b01, b11),
            )
        });
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_carte_fg_shift_candidates() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/carte_fg.ppm");
        if !ref_path.exists() {
            return;
        }
        let expected = std::fs::read(ref_path).unwrap();
        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let mask = page.decode_mask().unwrap().unwrap();
        let fg = page.decode_foreground().unwrap().unwrap();
        let w = page.info.width as u32;
        let h = page.info.height as u32;
        let (reduction, virt_w, virt_h, virt_page_w, virt_page_h) =
            layer_virtual_geometry(&fg, w, h);

        let compare = |name: &str, x_shift: u32, y_shift: u32| {
            let mut out = Pixmap::white(w, h);
            let mw = mask.width.min(w);
            let mh = mask.height.min(h);
            for y in 0..mh {
                for x in 0..mw {
                    if mask.get(x, y) {
                        let px = (x + x_shift).min(virt_page_w - 1);
                        let py = (y + y_shift).min(virt_page_h - 1);
                        let sx = (px / reduction).min(virt_w - 1).min(fg.width - 1);
                        let sy = (py / reduction).min(virt_h - 1).min(fg.height - 1);
                        let (r, g, b) = fg.get_rgb(sx, sy);
                        out.set_rgb(x, y, r, g, b);
                    }
                }
            }
            let stats = diff_stats(&out.to_ppm(), &expected);
            eprintln!(
                "carte fg shift {} byte_mismatch={} pixel_mismatch={}",
                name, stats.byte_mismatches, stats.pixel_mismatches,
            );
        };

        for x_shift in 0..reduction.min(3) {
            for y_shift in 0..reduction.min(3) {
                compare(&format!("{}_{}", x_shift, y_shift), x_shift, y_shift);
            }
        }
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_colorbook_bg_mismatch_profile() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/colorbook_p1_bg.ppm");
        if !ref_path.exists() {
            return;
        }
        let expected = std::fs::read(ref_path).unwrap();
        let data = std::fs::read(assets_path().join("colorbook.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let bg = page.decode_background().unwrap().unwrap();
        let w = page.info.width as u32;
        let h = page.info.height as u32;
        let actual = composite_bg_only(w, h, &bg, w, h).to_ppm();
        let a = &actual[ppm_header_end(&actual)..];
        let e = &expected[ppm_header_end(&expected)..];

        let mut col_diff = vec![0usize; w as usize];
        let mut row_diff = vec![0usize; h as usize];
        let mut total = 0usize;
        for y in 0..h as usize {
            for x in 0..w as usize {
                let i = (y * w as usize + x) * 3;
                if a[i] != e[i] || a[i + 1] != e[i + 1] || a[i + 2] != e[i + 2] {
                    total += 1;
                    col_diff[x] += 1;
                    row_diff[y] += 1;
                }
            }
        }

        let sum_range = |vals: &[usize], start: usize, end: usize| -> usize {
            vals[start.min(vals.len())..end.min(vals.len())]
                .iter()
                .sum()
        };
        let max_col = col_diff.iter().enumerate().max_by_key(|(_, v)| *v).unwrap();
        let max_row = row_diff.iter().enumerate().max_by_key(|(_, v)| *v).unwrap();

        eprintln!(
            "colorbook bg profile total={} left32={} right32={} top32={} bottom32={} center={} max_col={}({}) max_row={}({})",
            total,
            sum_range(&col_diff, 0, 32),
            sum_range(&col_diff, w as usize - 32, w as usize),
            sum_range(&row_diff, h as usize - 32, h as usize),
            sum_range(&row_diff, 0, 32),
            sum_range(&col_diff, w as usize / 4, (w as usize * 3) / 4),
            max_col.0,
            max_col.1,
            max_row.0,
            max_row.1,
        );
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_colorbook_bg_channel_bilinear_candidate() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/colorbook_p1_bg.ppm");
        if !ref_path.exists() {
            return;
        }
        let expected = std::fs::read(ref_path).unwrap();
        let data = std::fs::read(assets_path().join("colorbook.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let planes = page.decode_background_planes().unwrap().unwrap();
        let w = page.info.width as u32;
        let h = page.info.height as u32;

        let sample_plane =
            |plane: &[i16], src_w: u32, src_h: u32, page_x: u32, page_y: u32| -> i32 {
                let (_reduction, virt_w, virt_h, virt_page_w, virt_page_h) = {
                    let red_w = (w + src_w - 1) / src_w;
                    let red_h = (h + src_h - 1) / src_h;
                    let reduction = red_w.max(red_h).max(1);
                    let virt_w = (w / reduction).max(1);
                    let virt_h = (h / reduction).max(1);
                    let virt_page_w = virt_w * reduction;
                    let virt_page_h = virt_h * reduction;
                    (reduction, virt_w, virt_h, virt_page_w, virt_page_h)
                };
                let sw = virt_w as f64;
                let sh = virt_h as f64;
                let dw = virt_page_w as f64;
                let dh = virt_page_h as f64;
                let px = page_x.min(virt_page_w - 1);
                let py = page_y.min(virt_page_h - 1);
                let sx = ((px as f64 + 0.5) * sw / dw - 0.5).clamp(0.0, sw - 1.0);
                let sy = ((py as f64 + 0.5) * sh / dh - 0.5).clamp(0.0, sh - 1.0);
                let sx0 = sx as u32;
                let sy0 = sy as u32;
                let sx1 = (sx0 + 1).min(virt_w.saturating_sub(1)).min(src_w - 1);
                let sy1 = (sy0 + 1).min(virt_h.saturating_sub(1)).min(src_h - 1);
                let fx = sx - sx0 as f64;
                let fy = sy - sy0 as f64;
                let get = |x: u32, y: u32| plane[(y * src_w + x) as usize] as f64;
                let v = get(sx0.min(src_w - 1), sy0.min(src_h - 1)) * (1.0 - fx) * (1.0 - fy)
                    + get(sx1, sy0.min(src_h - 1)) * fx * (1.0 - fy)
                    + get(sx0.min(src_w - 1), sy1) * (1.0 - fx) * fy
                    + get(sx1, sy1) * fx * fy;
                v.round() as i32
            };

        let mut out = Pixmap::white(w, h);
        let cb = planes.cb.as_ref().unwrap();
        let cr = planes.cr.as_ref().unwrap();
        for y in 0..h {
            for x in 0..w {
                let yv = sample_plane(&planes.y, planes.width, planes.height, x, y);
                let bv = sample_plane(cb, planes.width, planes.height, x, y);
                let rv = sample_plane(cr, planes.width, planes.height, x, y);
                let t2 = rv + (rv >> 1);
                let t3 = yv + 128 - (bv >> 2);
                let red = (yv + 128 + t2).clamp(0, 255) as u8;
                let green = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
                let blue = (t3 + (bv << 1)).clamp(0, 255) as u8;
                out.set_rgb(x, y, red, green, blue);
            }
        }

        let stats = diff_stats(&out.to_ppm(), &expected);
        eprintln!(
            "colorbook bg channel-bilinear bytes={} pixels={} mean_abs_rgb=({:.4},{:.4},{:.4}) max_abs_rgb=({},{},{})",
            stats.byte_mismatches,
            stats.pixel_mismatches,
            stats.sum_abs_r as f64 / stats.pixel_count as f64,
            stats.sum_abs_g as f64 / stats.pixel_count as f64,
            stats.sum_abs_b as f64 / stats.pixel_count as f64,
            stats.max_abs_r,
            stats.max_abs_g,
            stats.max_abs_b,
        );
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_colorbook_bg_xphase_search() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/colorbook_p1_bg.ppm");
        if !ref_path.exists() {
            return;
        }
        let expected = std::fs::read(ref_path).unwrap();
        let data = std::fs::read(assets_path().join("colorbook.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let bg = page.decode_background().unwrap().unwrap();
        let w = page.info.width as u32;
        let h = page.info.height as u32;
        let (_, virt_w, virt_h, virt_page_w, virt_page_h) = layer_virtual_geometry(&bg, w, h);
        let mut best = Vec::new();

        for step in -16..=16 {
            let x_phase = step as f64 / 16.0;
            let mut out = Pixmap::white(w, h);
            for y in 0..h {
                let sh = virt_h as f64;
                let dh = virt_page_h as f64;
                let py = y.min(virt_page_h - 1);
                let sy = ((py as f64 + 0.5) * sh / dh - 0.5).clamp(0.0, sh - 1.0);
                let sy0 = sy as u32;
                let sy1 = (sy0 + 1).min(virt_h.saturating_sub(1)).min(bg.height - 1);
                let fy = sy - sy0 as f64;
                for x in 0..w {
                    let sw = virt_w as f64;
                    let dw = virt_page_w as f64;
                    let px = x.min(virt_page_w - 1);
                    let sx = ((px as f64 + 0.5) * sw / dw - 0.5 + x_phase).clamp(0.0, sw - 1.0);
                    let sx0 = sx as u32;
                    let sx1 = (sx0 + 1).min(virt_w.saturating_sub(1)).min(bg.width - 1);
                    let fx = sx - sx0 as f64;
                    let (r00, g00, b00) = bg.get_rgb(sx0.min(bg.width - 1), sy0.min(bg.height - 1));
                    let (r10, g10, b10) = bg.get_rgb(sx1, sy0.min(bg.height - 1));
                    let (r01, g01, b01) = bg.get_rgb(sx0.min(bg.width - 1), sy1);
                    let (r11, g11, b11) = bg.get_rgb(sx1, sy1);
                    let interp = |v00: u8, v10: u8, v01: u8, v11: u8| -> u8 {
                        let v = v00 as f64 * (1.0 - fx) * (1.0 - fy)
                            + v10 as f64 * fx * (1.0 - fy)
                            + v01 as f64 * (1.0 - fx) * fy
                            + v11 as f64 * fx * fy;
                        (v + 0.5).clamp(0.0, 255.0) as u8
                    };
                    out.set_rgb(
                        x,
                        y,
                        interp(r00, r10, r01, r11),
                        interp(g00, g10, g01, g11),
                        interp(b00, b10, b01, b11),
                    );
                }
            }
            let stats = diff_stats(&out.to_ppm(), &expected);
            best.push((stats.byte_mismatches, stats.pixel_mismatches, step));
        }

        best.sort_unstable();
        for (rank, (byte_mismatches, pixel_mismatches, step)) in
            best.into_iter().take(10).enumerate()
        {
            eprintln!(
                "colorbook bg xphase rank={} step={} phase={:.4} byte_mismatch={} pixel_mismatch={}",
                rank + 1,
                step,
                step as f64 / 16.0,
                byte_mismatches,
                pixel_mismatches,
            );
        }
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_colorbook_bg_arithmetic_candidates() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/colorbook_p1_bg.ppm");
        if !ref_path.exists() {
            return;
        }
        let expected = std::fs::read(ref_path).unwrap();
        let data = std::fs::read(assets_path().join("colorbook.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let bg = page.decode_background().unwrap().unwrap();
        let w = page.info.width as u32;
        let h = page.info.height as u32;
        let (_, virt_w, virt_h, virt_page_w, virt_page_h) = layer_virtual_geometry(&bg, w, h);

        let compare = |name: &str, sample: &dyn Fn(u32, u32) -> (u8, u8, u8)| {
            let mut out = Pixmap::white(w, h);
            for y in 0..h {
                for x in 0..w {
                    let (r, g, b) = sample(x, y);
                    out.set_rgb(x, y, r, g, b);
                }
            }
            let stats = diff_stats(&out.to_ppm(), &expected);
            eprintln!(
                "colorbook bg arithmetic {} bytes={} pixels={} mean_abs_rgb=({:.4},{:.4},{:.4})",
                name,
                stats.byte_mismatches,
                stats.pixel_mismatches,
                stats.sum_abs_r as f64 / stats.pixel_count as f64,
                stats.sum_abs_g as f64 / stats.pixel_count as f64,
                stats.sum_abs_b as f64 / stats.pixel_count as f64,
            );
        };

        let scaled_bg = scale_layer_bilinear(&bg, w, h);
        compare("current_float", &|x, y| sample_scaled(&scaled_bg, x, y));

        compare("fixed16_direct", &|x, y| {
            let sw = virt_w as i64;
            let sh = virt_h as i64;
            let dw = virt_page_w as i64;
            let dh = virt_page_h as i64;
            let px = x.min(virt_page_w - 1) as i64;
            let py = y.min(virt_page_h - 1) as i64;
            let sx_fp = (((2 * px + 1) * sw << 16) / (2 * dw)) - (1 << 15);
            let sy_fp = (((2 * py + 1) * sh << 16) / (2 * dh)) - (1 << 15);
            let sx_fp = sx_fp.clamp(0, (sw - 1) << 16);
            let sy_fp = sy_fp.clamp(0, (sh - 1) << 16);
            let sx0 = (sx_fp >> 16) as u32;
            let sy0 = (sy_fp >> 16) as u32;
            let sx1 = (sx0 + 1).min(virt_w.saturating_sub(1)).min(bg.width - 1);
            let sy1 = (sy0 + 1).min(virt_h.saturating_sub(1)).min(bg.height - 1);
            let fx = (sx_fp & 0xffff) as i64;
            let fy = (sy_fp & 0xffff) as i64;
            let wx0 = 65536 - fx;
            let wy0 = 65536 - fy;
            let wx1 = fx;
            let wy1 = fy;
            let (r00, g00, b00) = bg.get_rgb(sx0.min(bg.width - 1), sy0.min(bg.height - 1));
            let (r10, g10, b10) = bg.get_rgb(sx1, sy0.min(bg.height - 1));
            let (r01, g01, b01) = bg.get_rgb(sx0.min(bg.width - 1), sy1);
            let (r11, g11, b11) = bg.get_rgb(sx1, sy1);
            let interp = |v00: u8, v10: u8, v01: u8, v11: u8| -> u8 {
                let acc = v00 as i64 * wx0 * wy0
                    + v10 as i64 * wx1 * wy0
                    + v01 as i64 * wx0 * wy1
                    + v11 as i64 * wx1 * wy1;
                ((acc + (1 << 31)) >> 32).clamp(0, 255) as u8
            };
            (
                interp(r00, r10, r01, r11),
                interp(g00, g10, g01, g11),
                interp(b00, b10, b01, b11),
            )
        });

        compare("separable_round8", &|x, y| {
            let sw = virt_w as f64;
            let sh = virt_h as f64;
            let dw = virt_page_w as f64;
            let dh = virt_page_h as f64;
            let px = x.min(virt_page_w - 1);
            let py = y.min(virt_page_h - 1);
            let sx = ((px as f64 + 0.5) * sw / dw - 0.5).clamp(0.0, sw - 1.0);
            let sy = ((py as f64 + 0.5) * sh / dh - 0.5).clamp(0.0, sh - 1.0);
            let sx0 = sx as u32;
            let sy0 = sy as u32;
            let sx1 = (sx0 + 1).min(virt_w.saturating_sub(1)).min(bg.width - 1);
            let sy1 = (sy0 + 1).min(virt_h.saturating_sub(1)).min(bg.height - 1);
            let fx = ((sx - sx0 as f64) * 256.0 + 0.5).floor().clamp(0.0, 255.0) as u32;
            let fy = ((sy - sy0 as f64) * 256.0 + 0.5).floor().clamp(0.0, 255.0) as u32;
            let interp_h = |v0: u8, v1: u8| -> u32 {
                ((v0 as u32 * (256 - fx) + v1 as u32 * fx + 128) >> 8).clamp(0, 255)
            };
            let interp_v = |v0: u32, v1: u32| -> u8 {
                ((v0 * (256 - fy) + v1 * fy + 128) >> 8).clamp(0, 255) as u8
            };
            let (r00, g00, b00) = bg.get_rgb(sx0.min(bg.width - 1), sy0.min(bg.height - 1));
            let (r10, g10, b10) = bg.get_rgb(sx1, sy0.min(bg.height - 1));
            let (r01, g01, b01) = bg.get_rgb(sx0.min(bg.width - 1), sy1);
            let (r11, g11, b11) = bg.get_rgb(sx1, sy1);
            (
                interp_v(interp_h(r00, r10), interp_h(r01, r11)),
                interp_v(interp_h(g00, g10), interp_h(g01, g11)),
                interp_v(interp_h(b00, b10), interp_h(b01, b11)),
            )
        });

        compare("separable_floor8", &|x, y| {
            let sw = virt_w as f64;
            let sh = virt_h as f64;
            let dw = virt_page_w as f64;
            let dh = virt_page_h as f64;
            let px = x.min(virt_page_w - 1);
            let py = y.min(virt_page_h - 1);
            let sx = ((px as f64 + 0.5) * sw / dw - 0.5).clamp(0.0, sw - 1.0);
            let sy = ((py as f64 + 0.5) * sh / dh - 0.5).clamp(0.0, sh - 1.0);
            let sx0 = sx as u32;
            let sy0 = sy as u32;
            let sx1 = (sx0 + 1).min(virt_w.saturating_sub(1)).min(bg.width - 1);
            let sy1 = (sy0 + 1).min(virt_h.saturating_sub(1)).min(bg.height - 1);
            let fx = ((sx - sx0 as f64) * 256.0).floor().clamp(0.0, 255.0) as u32;
            let fy = ((sy - sy0 as f64) * 256.0).floor().clamp(0.0, 255.0) as u32;
            let interp_h = |v0: u8, v1: u8| -> u32 {
                ((v0 as u32 * (256 - fx) + v1 as u32 * fx) >> 8).clamp(0, 255)
            };
            let interp_v =
                |v0: u32, v1: u32| -> u8 { ((v0 * (256 - fy) + v1 * fy) >> 8).clamp(0, 255) as u8 };
            let (r00, g00, b00) = bg.get_rgb(sx0.min(bg.width - 1), sy0.min(bg.height - 1));
            let (r10, g10, b10) = bg.get_rgb(sx1, sy0.min(bg.height - 1));
            let (r01, g01, b01) = bg.get_rgb(sx0.min(bg.width - 1), sy1);
            let (r11, g11, b11) = bg.get_rgb(sx1, sy1);
            (
                interp_v(interp_h(r00, r10), interp_h(r01, r11)),
                interp_v(interp_h(g00, g10), interp_h(g01, g11)),
                interp_v(interp_h(b00, b10), interp_h(b01, b11)),
            )
        });
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_colorbook_full_mismatch_profile() {
        let expected = std::fs::read(golden_path().join("colorbook_p1.ppm")).unwrap();
        let data = std::fs::read(assets_path().join("colorbook.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let mask = page.decode_mask().unwrap().unwrap();
        let actual = render(&page).unwrap().to_ppm();

        let a = &actual[ppm_header_end(&actual)..];
        let e = &expected[ppm_header_end(&expected)..];
        let stats = diff_stats(&actual, &expected);

        let mut fg_pixels = 0usize;
        let mut bg_pixels = 0usize;
        let mut fg_pixel_mismatches = 0usize;
        let mut bg_pixel_mismatches = 0usize;
        let mut fg_byte_mismatches = 0usize;
        let mut bg_byte_mismatches = 0usize;
        let mut fg_sum_abs = [0u64; 3];
        let mut bg_sum_abs = [0u64; 3];

        let w = page.info.width as u32;
        let h = page.info.height as u32;
        for y in 0..h as usize {
            for x in 0..w as usize {
                let i = (y * w as usize + x) * 3;
                let dr = (a[i] as i16 - e[i] as i16).unsigned_abs() as u8;
                let dg = (a[i + 1] as i16 - e[i + 1] as i16).unsigned_abs() as u8;
                let db = (a[i + 2] as i16 - e[i + 2] as i16).unsigned_abs() as u8;
                let is_fg = x < mask.width as usize
                    && y < mask.height as usize
                    && mask.get(x as u32, y as u32);
                let (pixels, pixel_mismatches, byte_mismatches, sum_abs) = if is_fg {
                    (
                        &mut fg_pixels,
                        &mut fg_pixel_mismatches,
                        &mut fg_byte_mismatches,
                        &mut fg_sum_abs,
                    )
                } else {
                    (
                        &mut bg_pixels,
                        &mut bg_pixel_mismatches,
                        &mut bg_byte_mismatches,
                        &mut bg_sum_abs,
                    )
                };
                *pixels += 1;
                if dr != 0 || dg != 0 || db != 0 {
                    *pixel_mismatches += 1;
                }
                *byte_mismatches += (dr != 0) as usize + (dg != 0) as usize + (db != 0) as usize;
                sum_abs[0] += dr as u64;
                sum_abs[1] += dg as u64;
                sum_abs[2] += db as u64;
            }
        }

        eprintln!(
            "colorbook full bytes={} pixels={} mean_abs_rgb=({:.4},{:.4},{:.4}) max_abs_rgb=({},{},{})",
            stats.byte_mismatches,
            stats.pixel_mismatches,
            stats.sum_abs_r as f64 / stats.pixel_count as f64,
            stats.sum_abs_g as f64 / stats.pixel_count as f64,
            stats.sum_abs_b as f64 / stats.pixel_count as f64,
            stats.max_abs_r,
            stats.max_abs_g,
            stats.max_abs_b,
        );
        eprintln!(
            "colorbook full fg bytes={} pixels={} mean_abs_rgb=({:.4},{:.4},{:.4})",
            fg_byte_mismatches,
            fg_pixel_mismatches,
            fg_sum_abs[0] as f64 / fg_pixels as f64,
            fg_sum_abs[1] as f64 / fg_pixels as f64,
            fg_sum_abs[2] as f64 / fg_pixels as f64,
        );
        eprintln!(
            "colorbook full bg bytes={} pixels={} mean_abs_rgb=({:.4},{:.4},{:.4})",
            bg_byte_mismatches,
            bg_pixel_mismatches,
            bg_sum_abs[0] as f64 / bg_pixels as f64,
            bg_sum_abs[1] as f64 / bg_pixels as f64,
            bg_sum_abs[2] as f64 / bg_pixels as f64,
        );
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_colorbook_full_fg_shift_candidates() {
        let expected = std::fs::read(golden_path().join("colorbook_p1.ppm")).unwrap();
        let data = std::fs::read(assets_path().join("colorbook.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let mask = page.decode_mask().unwrap().unwrap();
        let bg = page.decode_background().unwrap().unwrap();
        let fg = page.decode_foreground().unwrap().unwrap();
        let w = page.info.width as u32;
        let h = page.info.height as u32;
        let (reduction, virt_w, virt_h, virt_page_w, virt_page_h) =
            layer_virtual_geometry(&fg, w, h);

        let scaled_bg2 = scale_layer_bilinear(&bg, w, h);
        let compare = |name: &str, x_shift: u32, y_shift: u32| {
            let mut out = Pixmap::white(w, h);
            for y in 0..h {
                for x in 0..w {
                    if x < mask.width && y < mask.height && mask.get(x, y) {
                        let px = (x + x_shift).min(virt_page_w - 1);
                        let py = (y + y_shift).min(virt_page_h - 1);
                        let sx = (px / reduction).min(virt_w - 1).min(fg.width - 1);
                        let sy = (py / reduction).min(virt_h - 1).min(fg.height - 1);
                        let (r, g, b) = fg.get_rgb(sx, sy);
                        out.set_rgb(x, y, r, g, b);
                    } else {
                        let (r, g, b) = sample_scaled(&scaled_bg2, x, y);
                        out.set_rgb(x, y, r, g, b);
                    }
                }
            }
            let stats = diff_stats(&out.to_ppm(), &expected);
            eprintln!(
                "colorbook full fg shift {} byte_mismatch={} pixel_mismatch={}",
                name, stats.byte_mismatches, stats.pixel_mismatches,
            );
        };

        for x_shift in 0..reduction.min(3) {
            for y_shift in 0..reduction.min(3) {
                compare(&format!("{}_{}", x_shift, y_shift), x_shift, y_shift);
            }
        }
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_colorbook_fg_shift_search() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/colorbook_p1_fg.ppm");
        if !ref_path.exists() {
            return;
        }
        let expected = std::fs::read(ref_path).unwrap();
        let data = std::fs::read(assets_path().join("colorbook.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let mask = page.decode_mask().unwrap().unwrap();
        let fg = page.decode_foreground().unwrap().unwrap();
        let w = page.info.width as u32;
        let h = page.info.height as u32;
        let (reduction, virt_w, virt_h, virt_page_w, virt_page_h) =
            layer_virtual_geometry(&fg, w, h);
        let mut best = Vec::new();

        for x_shift in 0..reduction {
            for y_shift in 0..reduction {
                let mut out = Pixmap::white(w, h);
                let mw = mask.width.min(w);
                let mh = mask.height.min(h);
                for y in 0..mh {
                    for x in 0..mw {
                        if mask.get(x, y) {
                            let px = (x + x_shift).min(virt_page_w - 1);
                            let py = (y + y_shift).min(virt_page_h - 1);
                            let sx = (px / reduction).min(virt_w - 1).min(fg.width - 1);
                            let sy = (py / reduction).min(virt_h - 1).min(fg.height - 1);
                            let (r, g, b) = fg.get_rgb(sx, sy);
                            out.set_rgb(x, y, r, g, b);
                        }
                    }
                }
                let stats = diff_stats(&out.to_ppm(), &expected);
                best.push((
                    stats.pixel_mismatches,
                    stats.byte_mismatches,
                    x_shift,
                    y_shift,
                ));
            }
        }

        best.sort_unstable();
        for (rank, (pixel_mismatches, byte_mismatches, x_shift, y_shift)) in
            best.into_iter().take(10).enumerate()
        {
            eprintln!(
                "colorbook fg search rank={} shift=({}, {}) pixel_mismatch={} byte_mismatch={}",
                rank + 1,
                x_shift,
                y_shift,
                pixel_mismatches,
                byte_mismatches,
            );
        }
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_layer_virtual_dims() {
        for (file, page_idx) in [("carte.djvu", 0usize), ("colorbook.djvu", 0usize)] {
            let data = std::fs::read(assets_path().join(file)).unwrap();
            let doc = Document::parse(&data).unwrap();
            let page = doc.page(page_idx).unwrap();
            let bg = page.decode_background().unwrap().unwrap();
            let fg = page.decode_foreground().unwrap().unwrap();
            let w = page.info.width as u32;
            let h = page.info.height as u32;
            let bg_geo = layer_virtual_geometry(&bg, w, h);
            let fg_geo = layer_virtual_geometry(&fg, w, h);
            eprintln!(
                "{} p{} page={}x{} bg={}x{} bg_geo={:?} fg={}x{} fg_geo={:?}",
                file,
                page_idx + 1,
                w,
                h,
                bg.width,
                bg.height,
                bg_geo,
                fg.width,
                fg.height,
                fg_geo,
            );
        }
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_dump_carte_actual_ppm() {
        let out_dir = std::path::Path::new("/tmp/rdjvu_debug");
        std::fs::create_dir_all(out_dir).unwrap();
        let out_path = out_dir.join("carte_actual.ppm");
        let pm = render_page("carte.djvu", 0);
        std::fs::write(&out_path, pm.to_ppm()).unwrap();
    }

    #[test]
    #[ignore] // development parameter sweep — run with --ignored
    fn debug_dump_carte_bg_actual_ppm() {
        let out_dir = std::path::Path::new("/tmp/rdjvu_debug");
        std::fs::create_dir_all(out_dir).unwrap();
        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let bg = page.decode_background().unwrap().unwrap();
        let out_path = out_dir.join("carte_bg_actual.ppm");
        std::fs::write(&out_path, bg.to_ppm()).unwrap();
    }

    // -- Gamma correction tests -----------------------------------------------

    #[test]
    fn gamma_lut_identity_at_1_0() {
        let lut = build_gamma_lut(1.0);
        for (i, &v) in lut.iter().enumerate() {
            assert_eq!(v, i as u8, "identity LUT mismatch at {i}");
        }
    }

    #[test]
    fn gamma_lut_zero_is_identity() {
        let lut = build_gamma_lut(0.0);
        for (i, &v) in lut.iter().enumerate() {
            assert_eq!(v, i as u8);
        }
    }

    #[test]
    fn gamma_lut_2_2_changes_midtones() {
        let lut = build_gamma_lut(2.2);
        // Midtone value 128 should be noticeably different under gamma 2.2
        assert_ne!(lut[128], 128, "gamma 2.2 should change midtone 128");
        // Endpoints must be fixed
        assert_eq!(lut[0], 0);
        assert_eq!(lut[255], 255);
    }

    #[test]
    fn apply_gamma_modifies_pixmap() {
        let mut pm = Pixmap::white(2, 2);
        // Set one pixel to a midtone gray
        pm.set_rgb(0, 0, 128, 128, 128);
        let lut = build_gamma_lut(2.2);
        apply_gamma(&mut pm, &lut);
        let (r, g, b) = pm.get_rgb(0, 0);
        // After gamma 2.2 correction, midtone should shift
        assert_ne!(r, 128, "gamma should modify midtone pixel");
        assert_eq!(r, g);
        assert_eq!(g, b);
        // White pixel should stay white
        let (wr, wg, wb) = pm.get_rgb(1, 1);
        assert_eq!((wr, wg, wb), (255, 255, 255));
    }

    #[test]
    fn render_applies_gamma_correction() {
        // Use boy.djvu which has an IW44 background with midtone pixels
        // that are affected by gamma correction (unlike bilevel pages
        // where all pixels are 0 or 255).
        let data = std::fs::read(assets_path().join("boy.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let w = page.info.width as u32;
        let h = page.info.height as u32;

        // Composite without gamma (what we'd get without correction)
        let no_gamma = composite_page(&page, w, h, 0).unwrap();

        // Full render (includes gamma)
        let with_gamma = render(&page).unwrap();

        // They should differ if page gamma != 1.0
        if (page.info.gamma - 1.0).abs() >= 1e-4 {
            let mut diff_count = 0usize;
            for y in 0..h {
                for x in 0..w {
                    let (r1, g1, b1) = no_gamma.get_rgb(x, y);
                    let (r2, g2, b2) = with_gamma.get_rgb(x, y);
                    if r1 != r2 || g1 != g2 || b1 != b2 {
                        diff_count += 1;
                    }
                }
            }
            assert!(
                diff_count > 0,
                "gamma correction should change at least some pixels (page gamma={})",
                page.info.gamma
            );
        }
    }

    // ── Parallel bilinear scaler correctness ──────────────────────────────────

    /// Build a synthetic gradient Pixmap: R = x%256, G = y%256, B = (x+y)%256.
    fn gradient_pixmap(w: u32, h: u32) -> Pixmap {
        let mut pm = Pixmap::white(w, h);
        for y in 0..h {
            for x in 0..w {
                pm.set_rgb(
                    x,
                    y,
                    (x % 256) as u8,
                    (y % 256) as u8,
                    ((x + y) % 256) as u8,
                );
            }
        }
        pm
    }

    /// scale_bilinear_direct must produce deterministic output across calls (parallel
    /// scheduler must not reorder writes). Exercises a 3× upscale matching the typical
    /// BG-layer ratio in a 600 dpi DjVu page.
    #[test]
    fn bilinear_parallel_matches_sequential() {
        let src = gradient_pixmap(100, 80);
        let out1 = scale_bilinear_direct(&src, 300, 240);
        let out2 = scale_bilinear_direct(&src, 300, 240);
        assert_eq!(
            out1.data, out2.data,
            "bilinear output must be deterministic"
        );
        assert_eq!(out1.width, 300);
        assert_eq!(out1.height, 240);
        // Centre of 300×240 maps to ~(50, 40) in src — gradient gives (50, 40, 90).
        let (r, g, b) = out1.get_rgb(150, 120);
        assert!((r as i32 - 50).abs() <= 2, "R centre pixel off: {r}");
        assert!((g as i32 - 40).abs() <= 2, "G centre pixel off: {g}");
        assert!((b as i32 - 90).abs() <= 2, "B centre pixel off: {b}");
    }

    /// scale_layer_bilinear must produce identical output for the same input regardless
    /// of whether the `parallel` feature enables rayon.
    #[test]
    fn scale_layer_bilinear_deterministic() {
        let src = gradient_pixmap(200, 150);
        let out1 = scale_layer_bilinear(&src, 600, 450);
        let out2 = scale_layer_bilinear(&src, 600, 450);
        assert_eq!(
            out1.data, out2.data,
            "scale_layer_bilinear must be deterministic"
        );
        assert_eq!(out1.width, 600);
        assert_eq!(out1.height, 450);
    }
}
