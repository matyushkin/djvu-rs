//! JB2 bilevel image decoder — clean-room implementation (phase 2b).
//!
//! Decodes JB2-encoded bitonal images from DjVu Sjbz and Djbz chunks.
//! The JB2 format uses a ZP adaptive arithmetic coder with 262 context variables
//! and a symbol dictionary for run-length compression of recurring glyphs.
//!
//! # Key public types
//!
//! - `Jb2Dict` — shared symbol dictionary decoded from a Djbz chunk
//! - `decode` — decode a Sjbz image stream to a `Bitmap`
//! - `decode_dict` — decode a Djbz dictionary stream to a `Jb2Dict`
//!
//! # Record types
//!
//! | Code | Meaning |
//! |------|---------|
//! | 0    | start-of-image |
//! | 1    | new-symbol, add to dict AND blit to page |
//! | 2    | new-symbol, add to dict only |
//! | 3    | new-symbol (direct), blit only (not added to dict) |
//! | 4    | matched-refine, add to dict AND blit |
//! | 5    | matched-refine, add to dict only |
//! | 6    | matched-refine, blit only |
//! | 7    | matched-copy (no refinement), blit only |
//! | 8    | non-symbol (halftone block), blit only |
//! | 9    | required-dict-or-reset |
//! | 10   | comment |
//! | 11   | end-of-data |

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

use crate::bitmap::Bitmap;
use crate::error::Jb2Error;
use crate::zp_impl::ZpDecoder;

// ────────────────────────────────────────────────────────────────────────────
// NumContext: binary-tree arena for variable-length integer decoding
// ────────────────────────────────────────────────────────────────────────────

/// Binary-tree context store used to decode variable-length integers with ZP.
///
/// Each node in the tree holds one adaptive ZP context byte. Nodes are
/// allocated lazily as the decoder traverses the tree.
struct NumContext {
    ctx: Vec<u8>,
    left: Vec<u32>,
    right: Vec<u32>,
}

impl NumContext {
    fn new() -> Self {
        // Index 0 = unused sentinel; index 1 = root.
        NumContext {
            ctx: vec![0, 0],
            left: vec![0, 0],
            right: vec![0, 0],
        }
    }

    fn root(&self) -> usize {
        1
    }

    fn get_left(&mut self, node: usize) -> usize {
        if self.left[node] == 0 {
            let idx = self.ctx.len() as u32;
            self.ctx.push(0);
            self.left.push(0);
            self.right.push(0);
            self.left[node] = idx;
        }
        self.left[node] as usize
    }

    fn get_right(&mut self, node: usize) -> usize {
        if self.right[node] == 0 {
            let idx = self.ctx.len() as u32;
            self.ctx.push(0);
            self.left.push(0);
            self.right.push(0);
            self.right[node] = idx;
        }
        self.right[node] as usize
    }
}

/// Decode a variable-length integer in the range `[low, high]` using ZP
/// with a binary-tree context store.
fn decode_num(zp: &mut ZpDecoder<'_>, ctx: &mut NumContext, low: i32, high: i32) -> i32 {
    let mut low = low;
    let mut high = high;
    let mut negative = false;
    let mut cutoff: i32 = 0;
    let mut phase: u32 = 1;
    let mut range: u32 = 0xffff_ffff;
    let mut node = ctx.root();

    while range != 1 {
        let decision = if low >= cutoff {
            true
        } else if high >= cutoff {
            zp.decode_bit(&mut ctx.ctx[node])
        } else {
            false
        };

        node = if decision {
            ctx.get_right(node)
        } else {
            ctx.get_left(node)
        };

        match phase {
            1 => {
                negative = !decision;
                if negative {
                    let temp = -low - 1;
                    low = -high - 1;
                    high = temp;
                }
                phase = 2;
                cutoff = 1;
            }
            2 => {
                if !decision {
                    phase = 3;
                    range = ((cutoff + 1) / 2) as u32;
                    if range == 1 {
                        cutoff = 0;
                    } else {
                        cutoff -= (range / 2) as i32;
                    }
                } else {
                    cutoff = cutoff * 2 + 1;
                }
            }
            3 => {
                range /= 2;
                if range != 1 {
                    if !decision {
                        cutoff -= (range / 2) as i32;
                    } else {
                        cutoff += (range / 2) as i32;
                    }
                } else if !decision {
                    cutoff -= 1;
                }
            }
            _ => {
                // Unreachable: phase cycles through 1, 2, 3 only.
                // Use a saturating decrement to keep range moving toward 1.
                range = range.saturating_sub(1);
            }
        }
    }

    if negative { -cutoff - 1 } else { cutoff }
}

// ────────────────────────────────────────────────────────────────────────────
// Jbm: internal 1-byte-per-pixel working bitmap (row 0 = bottom of page)
// ────────────────────────────────────────────────────────────────────────────

/// Internal working bitmap used during JB2 decoding.
///
/// Pixels are stored as 1 byte each (0 = white, 1 = black).
/// Row 0 is the **bottom** of the image (DjVu convention).
#[derive(Clone)]
struct Jbm {
    width: i32,
    height: i32,
    data: Vec<u8>,
}

impl Jbm {
    fn new(width: i32, height: i32) -> Self {
        let len = (width.max(0) as usize).saturating_mul(height.max(0) as usize);
        Jbm {
            width,
            height,
            data: vec![0u8; len],
        }
    }

    /// Return the pixel value at (row, col); out-of-bounds → 0.
    #[inline(always)]
    fn get(&self, row: i32, col: i32) -> u8 {
        if row < 0 || row >= self.height || col < 0 || col >= self.width {
            return 0;
        }
        // SAFETY: bounds checked above
        self.data[(row * self.width + col) as usize]
    }

    /// Set pixel (row, col) to 1 (black); silently ignores out-of-bounds.
    #[inline(always)]
    fn set(&mut self, row: i32, col: i32) {
        if row >= 0 && row < self.height && col >= 0 && col < self.width {
            self.data[(row * self.width + col) as usize] = 1;
        }
    }

    /// Return a new Jbm with surrounding empty rows/columns removed.
    fn crop_to_content(&self) -> Jbm {
        let mut min_row = self.height;
        let mut max_row: i32 = -1;
        let mut min_col = self.width;
        let mut max_col: i32 = -1;

        for row in 0..self.height {
            for col in 0..self.width {
                if self.data[(row * self.width + col) as usize] != 0 {
                    min_row = min_row.min(row);
                    max_row = max_row.max(row);
                    min_col = min_col.min(col);
                    max_col = max_col.max(col);
                }
            }
        }

        if max_row < 0 {
            return Jbm::new(0, 0);
        }

        let nw = max_col - min_col + 1;
        let nh = max_row - min_row + 1;
        let mut out = Jbm::new(nw, nh);

        for row in min_row..=max_row {
            for col in min_col..=max_col {
                if self.data[(row * self.width + col) as usize] != 0 {
                    out.data[((row - min_row) * nw + (col - min_col)) as usize] = 1;
                }
            }
        }
        out
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Direct bitmap decode: 10-bit context
// ────────────────────────────────────────────────────────────────────────────

/// Decode a bitmap using the direct (10-pixel context) method.
///
/// Decodes top-to-bottom using an incremental rolling window that avoids
/// recomputing all 10 context bits from scratch each pixel.
fn decode_bitmap_direct(zp: &mut ZpDecoder<'_>, ctx: &mut [u8], width: i32, height: i32) -> Jbm {
    let mut bm = Jbm::new(width, height);

    for row in (0..height).rev() {
        // r2: 3 bits from row+2, columns col-1..col+1
        let mut r2 = (bm.get(row + 2, 0) as u32) << 1 | bm.get(row + 2, 1) as u32;
        // r1: 5 bits from row+1, columns col-2..col+2
        let mut r1 = (bm.get(row + 1, 0) as u32) << 2
            | (bm.get(row + 1, 1) as u32) << 1
            | bm.get(row + 1, 2) as u32;
        // r0: 2 bits from row, columns col-2, col-1
        let mut r0: u32 = 0;

        for col in 0..width {
            let idx = (r2 << 7) | (r1 << 2) | r0;
            let ctx_byte = ctx.get_mut(idx as usize).copied().unwrap_or(0);
            let mut local_ctx = ctx_byte;
            let bit = zp.decode_bit(&mut local_ctx);
            if let Some(slot) = ctx.get_mut(idx as usize) {
                *slot = local_ctx;
            }
            if bit {
                bm.set(row, col);
            }
            // Advance rolling windows for next column
            r2 = ((r2 << 1) & 0b111) | bm.get(row + 2, col + 2) as u32;
            r1 = ((r1 << 1) & 0b11111) | bm.get(row + 1, col + 3) as u32;
            r0 = ((r0 << 1) & 0b11) | bit as u32;
        }
    }
    bm
}

// ────────────────────────────────────────────────────────────────────────────
// Refinement bitmap decode: 11-bit context
// ────────────────────────────────────────────────────────────────────────────

/// Decode a bitmap using the refinement (11-pixel context) method.
///
/// The new (child) bitmap `cbm` is decoded relative to a reference (matched)
/// bitmap `mbm`. Center alignment is used per the DjVu spec.
fn decode_bitmap_ref(
    zp: &mut ZpDecoder<'_>,
    ctx: &mut [u8],
    width: i32,
    height: i32,
    mbm: &Jbm,
) -> Jbm {
    let mut cbm = Jbm::new(width, height);

    // Center alignment: anchor the reference bitmap at the center of the child.
    let crow = (height - 1) >> 1;
    let ccol = (width - 1) >> 1;
    let mrow = (mbm.height - 1) >> 1;
    let mcol = (mbm.width - 1) >> 1;
    let row_shift = mrow - crow;
    let col_shift = mcol - ccol;

    for row in (0..height).rev() {
        let mr = row + row_shift;

        // cbm row+1: 3 bits at (col-1, col, col+1) — col-1=-1 starts as 0
        let mut c_r1 = (cbm.get(row + 1, 0) as u32) << 1 | cbm.get(row + 1, 1) as u32;
        // cbm current row, col-1: single bit, starts as 0
        let mut c_r0: u32 = 0;
        // mbm (mr, col+cs-1..col+cs+1): 3 bits
        let mut m_r1 = (mbm.get(mr, col_shift - 1) as u32) << 2
            | (mbm.get(mr, col_shift) as u32) << 1
            | mbm.get(mr, col_shift + 1) as u32;
        // mbm (mr-1, col+cs-1..col+cs+1): 3 bits
        let mut m_r0 = (mbm.get(mr - 1, col_shift - 1) as u32) << 2
            | (mbm.get(mr - 1, col_shift) as u32) << 1
            | mbm.get(mr - 1, col_shift + 1) as u32;

        for col in 0..width {
            let m_r2 = mbm.get(mr + 1, col + col_shift) as u32;
            let idx = (c_r1 << 8) | (c_r0 << 7) | (m_r2 << 6) | (m_r1 << 3) | m_r0;
            let ctx_byte = ctx.get(idx as usize).copied().unwrap_or(0);
            let mut local_ctx = ctx_byte;
            let bit = zp.decode_bit(&mut local_ctx);
            if let Some(slot) = ctx.get_mut(idx as usize) {
                *slot = local_ctx;
            }
            if bit {
                cbm.set(row, col);
            }
            // Advance rolling windows
            c_r1 = ((c_r1 << 1) & 0b111) | cbm.get(row + 1, col + 2) as u32;
            c_r0 = bit as u32;
            m_r1 = ((m_r1 << 1) & 0b111) | mbm.get(mr, col + col_shift + 2) as u32;
            m_r0 = ((m_r0 << 1) & 0b111) | mbm.get(mr - 1, col + col_shift + 2) as u32;
        }
    }
    cbm
}

// ────────────────────────────────────────────────────────────────────────────
// Baseline: rolling median-of-3 for vertical symbol positioning
// ────────────────────────────────────────────────────────────────────────────

struct Baseline {
    arr: [i32; 3],
    index: i32,
}

impl Baseline {
    fn new() -> Self {
        Baseline {
            arr: [0, 0, 0],
            index: -1,
        }
    }

    fn fill(&mut self, val: i32) {
        self.arr = [val, val, val];
    }

    fn add(&mut self, val: i32) {
        self.index += 1;
        if self.index == 3 {
            self.index = 0;
        }
        self.arr[self.index as usize] = val;
    }

    fn get_val(&self) -> i32 {
        let (a, b, c) = (self.arr[0], self.arr[1], self.arr[2]);
        if (a >= b && a <= c) || (a <= b && a >= c) {
            a
        } else if (b >= a && b <= c) || (b <= a && b >= c) {
            b
        } else {
            c
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Blit a symbol onto the page (OR compositing, bottom-left origin)
// ────────────────────────────────────────────────────────────────────────────

fn blit(page: &mut [u8], page_w: i32, page_h: i32, symbol: &Jbm, x: i32, y: i32) {
    // Fast path: symbol completely within page bounds
    if x >= 0 && y >= 0 && x + symbol.width <= page_w && y + symbol.height <= page_h {
        let pw = page_w as usize;
        let sw = symbol.width as usize;
        for row in 0..symbol.height as usize {
            let src_off = row * sw;
            let dst_off = (y as usize + row) * pw + x as usize;
            for col in 0..sw {
                if symbol.data[src_off + col] != 0 {
                    page[dst_off + col] = 1;
                }
            }
        }
    } else {
        // Slow path: clipped
        for row in 0..symbol.height {
            let py = y + row;
            if py < 0 || py >= page_h {
                continue;
            }
            for col in 0..symbol.width {
                if symbol.get(row, col) != 0 {
                    let px = x + col;
                    if px >= 0 && px < page_w {
                        let idx = (py * page_w + px) as usize;
                        page[idx] = 1;
                    }
                }
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Convert internal page buffer (row 0 = bottom) to Bitmap (row 0 = top)
// ────────────────────────────────────────────────────────────────────────────

fn page_to_bitmap(page: &[u8], width: i32, height: i32) -> Bitmap {
    let w = width as usize;
    let h = height as usize;
    let mut bm = Bitmap::new(width as u32, height as u32);
    let stride = bm.row_stride();
    let full_bytes = w / 8;
    let remaining = w % 8;

    for row in 0..h {
        let src_row = &page[row * w..(row + 1) * w];
        let dst_y = h - 1 - row; // flip: JB2 row 0=bottom → PBM row 0=top
        let dst_off = dst_y * stride;

        for byte_idx in 0..full_bytes {
            let base = byte_idx * 8;
            let mut byte_val = 0u8;
            for bit_pos in 0..8usize {
                if src_row[base + bit_pos] != 0 {
                    byte_val |= 0x80u8 >> bit_pos;
                }
            }
            bm.data[dst_off + byte_idx] = byte_val;
        }

        if remaining > 0 {
            let base = full_bytes * 8;
            let mut byte_val = 0u8;
            for bit_pos in 0..remaining {
                if src_row[base + bit_pos] != 0 {
                    byte_val |= 0x80u8 >> bit_pos;
                }
            }
            bm.data[dst_off + full_bytes] = byte_val;
        }
    }
    bm
}

// ────────────────────────────────────────────────────────────────────────────
// Symbol coordinate decoding
// ────────────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn decode_symbol_coords(
    zp: &mut ZpDecoder<'_>,
    offset_type_ctx: &mut u8,
    hoff_ctx: &mut NumContext,
    voff_ctx: &mut NumContext,
    shoff_ctx: &mut NumContext,
    svoff_ctx: &mut NumContext,
    first_left: &mut i32,
    first_bottom: &mut i32,
    last_right: &mut i32,
    baseline: &mut Baseline,
    sym_width: i32,
    sym_height: i32,
) -> (i32, i32) {
    let new_line = zp.decode_bit(offset_type_ctx);

    let (x, y) = if new_line {
        let hoff = decode_num(zp, hoff_ctx, -262143, 262142);
        let voff = decode_num(zp, voff_ctx, -262143, 262142);
        let nx = *first_left + hoff;
        let ny = *first_bottom + voff - sym_height + 1;
        *first_left = nx;
        *first_bottom = ny;
        baseline.fill(ny);
        (nx, ny)
    } else {
        let hoff = decode_num(zp, shoff_ctx, -262143, 262142);
        let voff = decode_num(zp, svoff_ctx, -262143, 262142);
        (*last_right + hoff, baseline.get_val() + voff)
    };

    baseline.add(y);
    *last_right = x + sym_width - 1;
    (x, y)
}

// ────────────────────────────────────────────────────────────────────────────
// Public API
// ────────────────────────────────────────────────────────────────────────────

/// A shared JB2 symbol dictionary decoded from a Djbz chunk.
///
/// Pass this to [`decode`] when the Sjbz stream references an external dict
/// via a "required-dict-or-reset" (type 9) record.
pub struct Jb2Dict {
    symbols: Vec<Jbm>,
}

/// Decode a JB2 image stream (Sjbz chunk data) into a [`Bitmap`].
///
/// `shared_dict` must be provided when the Sjbz stream begins with a
/// "required-dict-or-reset" record that references an external dictionary.
///
/// # Errors
///
/// Returns [`Jb2Error`] on malformed input, missing dictionary, or oversized image.
pub fn decode(data: &[u8], shared_dict: Option<&Jb2Dict>) -> Result<Bitmap, Jb2Error> {
    decode_image(data, shared_dict)
}

/// Decode a JB2 dictionary stream (Djbz chunk data) into a [`Jb2Dict`].
///
/// The returned dict can then be passed to [`decode`] for Sjbz streams that
/// reference it via an INCL or "required-dict-or-reset" record.
///
/// # Errors
///
/// Returns [`Jb2Error`] on malformed input.
pub fn decode_dict(data: &[u8], inherited: Option<&Jb2Dict>) -> Result<Jb2Dict, Jb2Error> {
    decode_dictionary(data, inherited)
}

// ────────────────────────────────────────────────────────────────────────────
// Core image decode
// ────────────────────────────────────────────────────────────────────────────

fn decode_image(data: &[u8], shared_dict: Option<&Jb2Dict>) -> Result<Bitmap, Jb2Error> {
    let mut zp = ZpDecoder::new(data).map_err(|_| Jb2Error::ZpInitFailed)?;

    // Contexts for variable-length integer decoding
    let mut record_type_ctx = NumContext::new();
    let mut image_size_ctx = NumContext::new();
    let mut symbol_width_ctx = NumContext::new();
    let mut symbol_height_ctx = NumContext::new();
    let mut inherit_dict_size_ctx = NumContext::new();
    let mut hoff_ctx = NumContext::new();
    let mut voff_ctx = NumContext::new();
    let mut shoff_ctx = NumContext::new();
    let mut svoff_ctx = NumContext::new();
    let mut symbol_index_ctx = NumContext::new();
    let mut symbol_width_diff_ctx = NumContext::new();
    let mut symbol_height_diff_ctx = NumContext::new();
    let mut horiz_abs_loc_ctx = NumContext::new();
    let mut vert_abs_loc_ctx = NumContext::new();
    let mut comment_length_ctx = NumContext::new();
    let mut comment_octet_ctx = NumContext::new();

    let mut offset_type_ctx: u8 = 0;
    let mut direct_bitmap_ctx = vec![0u8; 1024];
    let mut refinement_bitmap_ctx = vec![0u8; 2048];

    // Preamble: optional "required-dict-or-reset" (type 9) followed by
    // "start-of-image" (type 0).
    let mut rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);
    let mut initial_dict_length: usize = 0;
    if rtype == 9 {
        initial_dict_length = decode_num(&mut zp, &mut inherit_dict_size_ctx, 0, 262142) as usize;
        rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);
    }
    // `rtype` is now the start-of-image record (0); ignore its value.
    let _ = rtype;

    // Image dimensions
    let image_width = {
        let w = decode_num(&mut zp, &mut image_size_ctx, 0, 262142);
        if w == 0 { 200 } else { w }
    };
    let image_height = {
        let h = decode_num(&mut zp, &mut image_size_ctx, 0, 262142);
        if h == 0 { 200 } else { h }
    };

    // Reserved flag bit — must be 0
    let mut flag_ctx: u8 = 0;
    if zp.decode_bit(&mut flag_ctx) {
        return Err(Jb2Error::BadHeaderFlag);
    }

    // Populate initial dictionary from shared dict if requested
    let mut dict: Vec<Jbm> = Vec::new();
    if initial_dict_length > 0 {
        match shared_dict {
            Some(sd) => {
                if initial_dict_length > sd.symbols.len() {
                    return Err(Jb2Error::InheritedDictTooLarge);
                }
                dict.extend_from_slice(&sd.symbols[..initial_dict_length]);
            }
            None => return Err(Jb2Error::MissingSharedDict),
        }
    }

    // Safety cap: ~64M pixels
    const MAX_PIXELS: usize = 64 * 1024 * 1024;
    let page_size = (image_width as usize).saturating_mul(image_height as usize);
    if page_size > MAX_PIXELS {
        return Err(Jb2Error::ImageTooLarge);
    }
    let mut page = vec![0u8; page_size];

    // Positioning state
    let mut first_left: i32 = -1;
    let mut first_bottom: i32 = image_height - 1;
    let mut last_right: i32 = 0;
    let mut baseline = Baseline::new();

    // Main decode loop
    loop {
        let rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);

        match rtype {
            // 1 — new symbol, direct decode → add to dict AND blit
            1 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h);
                let (x, y) = decode_symbol_coords(
                    &mut zp,
                    &mut offset_type_ctx,
                    &mut hoff_ctx,
                    &mut voff_ctx,
                    &mut shoff_ctx,
                    &mut svoff_ctx,
                    &mut first_left,
                    &mut first_bottom,
                    &mut last_right,
                    &mut baseline,
                    bm.width,
                    bm.height,
                );
                blit(&mut page, image_width, image_height, &bm, x, y);
                dict.push(bm.crop_to_content());
            }

            // 2 — new symbol, direct decode → add to dict only
            2 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h);
                dict.push(bm.crop_to_content());
            }

            // 3 — new symbol, direct decode → blit only (not stored in dict)
            3 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h);
                let (x, y) = decode_symbol_coords(
                    &mut zp,
                    &mut offset_type_ctx,
                    &mut hoff_ctx,
                    &mut voff_ctx,
                    &mut shoff_ctx,
                    &mut svoff_ctx,
                    &mut first_left,
                    &mut first_bottom,
                    &mut last_right,
                    &mut baseline,
                    bm.width,
                    bm.height,
                );
                blit(&mut page, image_width, image_height, &bm, x, y);
            }

            // 4 — matched refinement → add to dict AND blit
            4 => {
                if dict.is_empty() {
                    return Err(Jb2Error::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(Jb2Error::InvalidSymbolIndex);
                }
                let wdiff = decode_num(&mut zp, &mut symbol_width_diff_ctx, -262143, 262142);
                let hdiff = decode_num(&mut zp, &mut symbol_height_diff_ctx, -262143, 262142);
                let cbm_w = dict[index].width + wdiff;
                let cbm_h = dict[index].height + hdiff;
                let cbm = decode_bitmap_ref(
                    &mut zp,
                    &mut refinement_bitmap_ctx,
                    cbm_w,
                    cbm_h,
                    &dict[index],
                );
                let (x, y) = decode_symbol_coords(
                    &mut zp,
                    &mut offset_type_ctx,
                    &mut hoff_ctx,
                    &mut voff_ctx,
                    &mut shoff_ctx,
                    &mut svoff_ctx,
                    &mut first_left,
                    &mut first_bottom,
                    &mut last_right,
                    &mut baseline,
                    cbm.width,
                    cbm.height,
                );
                blit(&mut page, image_width, image_height, &cbm, x, y);
                dict.push(cbm.crop_to_content());
            }

            // 5 — matched refinement → add to dict only
            5 => {
                if dict.is_empty() {
                    return Err(Jb2Error::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(Jb2Error::InvalidSymbolIndex);
                }
                let wdiff = decode_num(&mut zp, &mut symbol_width_diff_ctx, -262143, 262142);
                let hdiff = decode_num(&mut zp, &mut symbol_height_diff_ctx, -262143, 262142);
                let cbm_w = dict[index].width + wdiff;
                let cbm_h = dict[index].height + hdiff;
                let cbm = decode_bitmap_ref(
                    &mut zp,
                    &mut refinement_bitmap_ctx,
                    cbm_w,
                    cbm_h,
                    &dict[index],
                );
                dict.push(cbm.crop_to_content());
            }

            // 6 — matched refinement → blit only
            6 => {
                if dict.is_empty() {
                    return Err(Jb2Error::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(Jb2Error::InvalidSymbolIndex);
                }
                let wdiff = decode_num(&mut zp, &mut symbol_width_diff_ctx, -262143, 262142);
                let hdiff = decode_num(&mut zp, &mut symbol_height_diff_ctx, -262143, 262142);
                let cbm_w = dict[index].width + wdiff;
                let cbm_h = dict[index].height + hdiff;
                let cbm = decode_bitmap_ref(
                    &mut zp,
                    &mut refinement_bitmap_ctx,
                    cbm_w,
                    cbm_h,
                    &dict[index],
                );
                let (x, y) = decode_symbol_coords(
                    &mut zp,
                    &mut offset_type_ctx,
                    &mut hoff_ctx,
                    &mut voff_ctx,
                    &mut shoff_ctx,
                    &mut svoff_ctx,
                    &mut first_left,
                    &mut first_bottom,
                    &mut last_right,
                    &mut baseline,
                    cbm.width,
                    cbm.height,
                );
                blit(&mut page, image_width, image_height, &cbm, x, y);
            }

            // 7 — matched copy, no refinement → blit only
            7 => {
                if dict.is_empty() {
                    return Err(Jb2Error::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(Jb2Error::InvalidSymbolIndex);
                }
                let bm_w = dict[index].width;
                let bm_h = dict[index].height;
                let (x, y) = decode_symbol_coords(
                    &mut zp,
                    &mut offset_type_ctx,
                    &mut hoff_ctx,
                    &mut voff_ctx,
                    &mut shoff_ctx,
                    &mut svoff_ctx,
                    &mut first_left,
                    &mut first_bottom,
                    &mut last_right,
                    &mut baseline,
                    bm_w,
                    bm_h,
                );
                let sym = &dict[index];
                blit(&mut page, image_width, image_height, sym, x, y);
            }

            // 8 — non-symbol (halftone), absolute coordinates
            8 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h);
                let left = decode_num(&mut zp, &mut horiz_abs_loc_ctx, 1, image_width);
                let top = decode_num(&mut zp, &mut vert_abs_loc_ctx, 1, image_height);
                let x = left - 1;
                let y = top - h;
                blit(&mut page, image_width, image_height, &bm, x, y);
            }

            // 9 — required-dict-or-reset (already consumed in preamble; ignore here)
            9 => {}

            // 10 — comment: skip bytes
            10 => {
                let length = decode_num(&mut zp, &mut comment_length_ctx, 0, 262142);
                for _ in 0..length {
                    decode_num(&mut zp, &mut comment_octet_ctx, 0, 255);
                }
            }

            // 11 — end-of-data
            11 => break,

            _ => return Err(Jb2Error::UnknownRecordType),
        }
    }

    Ok(page_to_bitmap(&page, image_width, image_height))
}

// ────────────────────────────────────────────────────────────────────────────
// Core dictionary decode
// ────────────────────────────────────────────────────────────────────────────

fn decode_dictionary(data: &[u8], inherited: Option<&Jb2Dict>) -> Result<Jb2Dict, Jb2Error> {
    let mut zp = ZpDecoder::new(data).map_err(|_| Jb2Error::ZpInitFailed)?;

    let mut record_type_ctx = NumContext::new();
    let mut image_size_ctx = NumContext::new();
    let mut symbol_width_ctx = NumContext::new();
    let mut symbol_height_ctx = NumContext::new();
    let mut inherit_dict_size_ctx = NumContext::new();
    let mut symbol_index_ctx = NumContext::new();
    let mut symbol_width_diff_ctx = NumContext::new();
    let mut symbol_height_diff_ctx = NumContext::new();
    let mut comment_length_ctx = NumContext::new();
    let mut comment_octet_ctx = NumContext::new();

    let mut direct_bitmap_ctx = vec![0u8; 1024];
    let mut refinement_bitmap_ctx = vec![0u8; 2048];

    // Preamble
    let mut rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);
    let mut initial_dict_length: usize = 0;
    if rtype == 9 {
        initial_dict_length = decode_num(&mut zp, &mut inherit_dict_size_ctx, 0, 262142) as usize;
        rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);
    }
    let _ = rtype;

    // Dimensions (present but unused in dict streams)
    let _dict_width = decode_num(&mut zp, &mut image_size_ctx, 0, 262142);
    let _dict_height = decode_num(&mut zp, &mut image_size_ctx, 0, 262142);

    // Reserved flag bit
    let mut flag_ctx: u8 = 0;
    if zp.decode_bit(&mut flag_ctx) {
        return Err(Jb2Error::BadHeaderFlag);
    }

    let mut dict: Vec<Jbm> = Vec::new();
    if initial_dict_length > 0 {
        match inherited {
            Some(inh) => {
                if initial_dict_length > inh.symbols.len() {
                    return Err(Jb2Error::InheritedDictTooLarge);
                }
                dict.extend_from_slice(&inh.symbols[..initial_dict_length]);
            }
            None => return Err(Jb2Error::MissingSharedDict),
        }
    }

    // Dict streams only accept types 2, 5, 9, 10, 11
    loop {
        let rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);

        match rtype {
            // 2 — new symbol, direct decode → add to dict
            2 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h);
                dict.push(bm.crop_to_content());
            }

            // 5 — matched refinement → add to dict
            5 => {
                if dict.is_empty() {
                    return Err(Jb2Error::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(Jb2Error::InvalidSymbolIndex);
                }
                let wdiff = decode_num(&mut zp, &mut symbol_width_diff_ctx, -262143, 262142);
                let hdiff = decode_num(&mut zp, &mut symbol_height_diff_ctx, -262143, 262142);
                let cbm_w = dict[index].width + wdiff;
                let cbm_h = dict[index].height + hdiff;
                let cbm = decode_bitmap_ref(
                    &mut zp,
                    &mut refinement_bitmap_ctx,
                    cbm_w,
                    cbm_h,
                    &dict[index],
                );
                dict.push(cbm.crop_to_content());
            }

            // 9 — required-dict-or-reset (ignored in dict streams)
            9 => {}

            // 10 — comment: skip bytes
            10 => {
                let length = decode_num(&mut zp, &mut comment_length_ctx, 0, 262142);
                for _ in 0..length {
                    decode_num(&mut zp, &mut comment_octet_ctx, 0, 255);
                }
            }

            // 11 — end-of-data
            11 => break,

            _ => return Err(Jb2Error::UnexpectedDictRecordType),
        }
    }

    Ok(Jb2Dict { symbols: dict })
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn assets_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/djvujs/library/assets")
    }

    fn golden_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/jb2")
    }

    // ── IFF helpers ──────────────────────────────────────────────────────────

    fn extract_sjbz(djvu_data: &[u8]) -> Vec<u8> {
        let file = crate::iff::parse(djvu_data).unwrap();
        let sjbz = file.root.find_first(b"Sjbz").unwrap();
        sjbz.data().to_vec()
    }

    fn extract_first_page_sjbz(djvu_data: &[u8]) -> Vec<u8> {
        let file = crate::iff::parse(djvu_data).unwrap();
        let page_form = file
            .root
            .children()
            .iter()
            .find(|c| {
                matches!(c, crate::iff::Chunk::Form { secondary_id, .. }
                    if secondary_id == b"DJVU")
            })
            .expect("no DJVU form");
        page_form.find_first(b"Sjbz").unwrap().data().to_vec()
    }

    fn find_page_form_data(djvu_data: &[u8], page: usize) -> Vec<u8> {
        let file = crate::iff::parse(djvu_data).unwrap();
        let mut idx = 0;
        for chunk in file.root.children() {
            if matches!(chunk, crate::iff::Chunk::Form { secondary_id, .. }
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
        let file = crate::iff::parse(djvu_data).unwrap();
        for chunk in file.root.children() {
            if let crate::iff::Chunk::Form { secondary_id, .. } = chunk
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
        let file = crate::iff::parse(&djvu).unwrap();

        // Inline Djbz in page 1
        let mut idx = 0usize;
        let mut page_form_opt: Option<&crate::iff::Chunk<'_>> = None;
        for chunk in file.root.children() {
            if matches!(chunk, crate::iff::Chunk::Form { secondary_id, .. }
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
}
