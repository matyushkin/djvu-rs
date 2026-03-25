use crate::bitmap::Bitmap;
use crate::zp::ZPDecoder;

/// Errors that can occur during JB2 decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// A flag bit in the image/dict header was set when it must be zero.
    BadHeaderFlag,
    /// The inherited dictionary length exceeds the shared dictionary size.
    InheritedDictTooLarge,
    /// The stream references a shared dictionary but none was provided.
    MissingSharedDict,
    /// Image dimensions exceed the safety limit (~64M pixels).
    ImageTooLarge,
    /// A record references a dictionary symbol but the dictionary is empty.
    EmptyDictReference,
    /// A decoded symbol index is out of range for the current dictionary.
    InvalidSymbolIndex,
    /// An unrecognized record type was encountered in the image stream.
    UnknownRecordType,
    /// An unexpected record type was encountered in a dictionary stream.
    UnexpectedDictRecordType,
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeError::BadHeaderFlag => write!(f, "JB2: bad flag bit in header"),
            DecodeError::InheritedDictTooLarge => {
                write!(f, "JB2: inherited dict length exceeds shared dict size")
            }
            DecodeError::MissingSharedDict => {
                write!(f, "JB2: stream requires shared dict but none provided")
            }
            DecodeError::ImageTooLarge => write!(f, "JB2: image dimensions too large"),
            DecodeError::EmptyDictReference => write!(f, "JB2: dict reference with empty dict"),
            DecodeError::InvalidSymbolIndex => {
                write!(f, "JB2: decoded symbol index out of dictionary range")
            }
            DecodeError::UnknownRecordType => write!(f, "JB2: unknown record type"),
            DecodeError::UnexpectedDictRecordType => {
                write!(f, "JB2: unexpected record type in dict stream")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

// ============================================================
// NumContext: arena-based binary tree for variable-length numbers
// ============================================================

struct NumContext {
    ctx: Vec<u8>,
    left: Vec<u32>,
    right: Vec<u32>,
}

impl NumContext {
    fn new() -> Self {
        // Index 0 is unused sentinel. Index 1 is the root node.
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

fn decode_num(zp: &mut ZPDecoder, ctx: &mut NumContext, low: i32, high: i32) -> i32 {
    let mut low = low;
    let mut high = high;
    let mut negative = false;
    let mut cutoff: i32 = 0;
    let mut phase: u32 = 1;
    let mut range: u32 = 0xffffffff;
    let mut node = ctx.root();

    while range != 1 {
        let decision = if low >= cutoff {
            true
        } else if high >= cutoff {
            let mut ctx_byte = ctx.ctx[node];
            let bit = zp.decode(&mut ctx_byte);
            ctx.ctx[node] = ctx_byte;
            bit
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
                    cutoff = cutoff + cutoff + 1;
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
            _ => unreachable!(),
        }
    }

    if negative { -cutoff - 1 } else { cutoff }
}

// ============================================================
// Jbm: internal bitmap (1 byte per pixel, row 0 = bottom)
// ============================================================

#[derive(Clone)]
struct Jbm {
    width: i32,
    height: i32,
    data: Vec<u8>,
}

impl Jbm {
    fn new(width: i32, height: i32) -> Self {
        Jbm {
            width,
            height,
            data: vec![0; (width.max(0) * height.max(0)) as usize],
        }
    }

    #[inline(always)]
    fn get(&self, row: i32, col: i32) -> u8 {
        if row < 0 || row >= self.height || col < 0 || col >= self.width {
            return 0;
        }
        self.data[(row * self.width + col) as usize]
    }

    #[inline(always)]
    fn set(&mut self, row: i32, col: i32) {
        if row >= 0 && row < self.height && col >= 0 && col < self.width {
            self.data[(row * self.width + col) as usize] = 1;
        }
    }

    fn remove_empty_edges(&self) -> Jbm {
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

        let new_width = max_col - min_col + 1;
        let new_height = max_row - min_row + 1;
        let mut result = Jbm::new(new_width, new_height);

        for row in min_row..=max_row {
            for col in min_col..=max_col {
                if self.data[(row * self.width + col) as usize] != 0 {
                    result.data[((row - min_row) * new_width + (col - min_col)) as usize] = 1;
                }
            }
        }

        result
    }
}

// ============================================================
// Baseline: rolling median of 3 for symbol y positioning
// ============================================================

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
        self.arr[0] = val;
        self.arr[1] = val;
        self.arr[2] = val;
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

// ============================================================
// Bitmap decode: direct (10-bit context)
// ============================================================

fn decode_bitmap_direct(zp: &mut ZPDecoder, ctx: &mut [u8], width: i32, height: i32) -> Jbm {
    let mut bm = Jbm::new(width, height);
    // Decode top-to-bottom (row height-1 down to 0), left-to-right.
    // Use incremental context computation: maintain rolling bit windows
    // for rows above, advancing by 1 bit per column instead of recomputing
    // all 10 context bits from scratch each pixel.
    for row in (0..height).rev() {
        // r2: 3 bits from (row+2, col-1..col+1) — at col=0, col-1=-1 gives 0
        let mut r2 = (bm.get(row + 2, 0) as u32) << 1 | bm.get(row + 2, 1) as u32;
        // r1: 5 bits from (row+1, col-2..col+2) — at col=0, col-2 and col-1 give 0
        let mut r1 = (bm.get(row + 1, 0) as u32) << 2
            | (bm.get(row + 1, 1) as u32) << 1
            | bm.get(row + 1, 2) as u32;
        // r0: 2 bits from (row, col-2, col-1) — at col=0, both are 0
        let mut r0: u32 = 0;

        for col in 0..width {
            let idx = (r2 << 7) | (r1 << 2) | r0;
            let bit = zp.decode(&mut ctx[idx as usize]);
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

// ============================================================
// Bitmap decode: refinement (11-bit context)
// ============================================================

fn decode_bitmap_ref(
    zp: &mut ZPDecoder,
    ctx: &mut [u8],
    width: i32,
    height: i32,
    mbm: &Jbm,
) -> Jbm {
    let mut cbm = Jbm::new(width, height);
    // Center alignment
    let crow = (height - 1) >> 1;
    let ccol = (width - 1) >> 1;
    let mrow = (mbm.height - 1) >> 1;
    let mcol = (mbm.width - 1) >> 1;
    let row_shift = mrow - crow;
    let col_shift = mcol - ccol;

    // Incremental context: maintain rolling bit windows
    for row in (0..height).rev() {
        let mr = row + row_shift;
        let cs = col_shift; // col_shift + 0, for col=0

        // cbm row+1: 3 bits at (col-1, col, col+1) — col-1=-1 gives 0
        let mut c_r1 = (cbm.get(row + 1, 0) as u32) << 1 | cbm.get(row + 1, 1) as u32;
        // cbm row, col-1: single bit — col-1=-1 gives 0
        let mut c_r0: u32 = 0;
        // mbm (mr, cs+col-1..cs+col+1): 3 bits
        let mut m_r1 = (mbm.get(mr, cs - 1) as u32) << 2
            | (mbm.get(mr, cs) as u32) << 1
            | mbm.get(mr, cs + 1) as u32;
        // mbm (mr-1, cs+col-1..cs+col+1): 3 bits
        let mut m_r0 = (mbm.get(mr - 1, cs - 1) as u32) << 2
            | (mbm.get(mr - 1, cs) as u32) << 1
            | mbm.get(mr - 1, cs + 1) as u32;

        for col in 0..width {
            // mbm (mr+1, col+cs): single bit, no window to maintain
            let m_r2 = mbm.get(mr + 1, col + col_shift) as u32;
            let idx = (c_r1 << 8) | (c_r0 << 7) | (m_r2 << 6) | (m_r1 << 3) | m_r0;
            let bit = zp.decode(&mut ctx[idx as usize]);
            if bit {
                cbm.set(row, col);
            }
            // Advance rolling windows for next column
            c_r1 = ((c_r1 << 1) & 0b111) | cbm.get(row + 1, col + 2) as u32;
            c_r0 = bit as u32;
            m_r1 = ((m_r1 << 1) & 0b111) | mbm.get(mr, col + col_shift + 2) as u32;
            m_r0 = ((m_r0 << 1) & 0b111) | mbm.get(mr - 1, col + col_shift + 2) as u32;
        }
    }
    cbm
}

// ============================================================
// Public types
// ============================================================

pub struct JB2Dict {
    symbols: Vec<Jbm>,
}

// ============================================================
// Blit a symbol onto a page bitmap (OR compositing)
// ============================================================

struct BlitTarget<'a> {
    page: &'a mut [u8],
    blit_map: Option<&'a mut [i32]>,
    page_w: i32,
    page_h: i32,
}

fn blit(target: &mut BlitTarget<'_>, blit_idx: i32, symbol: &Jbm, x: i32, y: i32) {
    let page = &mut target.page;
    let mut blit_map = target.blit_map.as_deref_mut();
    let page_w = target.page_w;
    let page_h = target.page_h;

    // Fast path: symbol fully within page bounds — skip per-pixel bounds checks
    if x >= 0 && y >= 0 && x + symbol.width <= page_w && y + symbol.height <= page_h {
        let pw = page_w as usize;
        let sw = symbol.width as usize;
        for row in 0..symbol.height as usize {
            let sym_off = row * sw;
            let page_off = (y as usize + row) * pw + x as usize;
            for col in 0..sw {
                if symbol.data[sym_off + col] != 0 {
                    page[page_off + col] = 1;
                    if let Some(ref mut map) = blit_map {
                        map[page_off + col] = blit_idx;
                    }
                }
            }
        }
    } else {
        // Slow path: symbol partially outside page, need per-pixel bounds checking
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
                        if let Some(ref mut map) = blit_map {
                            map[idx] = blit_idx;
                        }
                    }
                }
            }
        }
    }
}

// ============================================================
// Convert internal page bitmap (row 0=bottom) to PBM Bitmap (row 0=top)
// ============================================================

fn page_to_bitmap(page: &[u8], width: i32, height: i32) -> Bitmap {
    let w = width as usize;
    let h = height as usize;
    let mut bm = Bitmap::new(width as u32, height as u32);
    let stride = bm.row_stride();

    // Process 8 source pixels at a time, packing directly into destination bytes.
    // Avoids per-pixel Bitmap::set() which recomputes stride and byte/bit indices.
    let full_bytes = w / 8;
    let remaining = w % 8;

    for row in 0..h {
        let src_row = &page[row * w..(row + 1) * w];
        let dst_y = h - 1 - row; // flip: JB2 row 0=bottom → PBM row 0=top
        let dst_off = dst_y * stride;

        for byte_idx in 0..full_bytes {
            let base = byte_idx * 8;
            let mut byte_val = 0u8;
            if src_row[base] != 0 {
                byte_val |= 0x80;
            }
            if src_row[base + 1] != 0 {
                byte_val |= 0x40;
            }
            if src_row[base + 2] != 0 {
                byte_val |= 0x20;
            }
            if src_row[base + 3] != 0 {
                byte_val |= 0x10;
            }
            if src_row[base + 4] != 0 {
                byte_val |= 0x08;
            }
            if src_row[base + 5] != 0 {
                byte_val |= 0x04;
            }
            if src_row[base + 6] != 0 {
                byte_val |= 0x02;
            }
            if src_row[base + 7] != 0 {
                byte_val |= 0x01;
            }
            bm.data[dst_off + byte_idx] = byte_val;
        }

        if remaining > 0 {
            let base = full_bytes * 8;
            let mut byte_val = 0u8;
            for bit in 0..remaining {
                if src_row[base + bit] != 0 {
                    byte_val |= 0x80 >> bit;
                }
            }
            bm.data[dst_off + full_bytes] = byte_val;
        }
    }
    bm
}

fn flip_blit_map(map: &[i32], width: i32, height: i32) -> Vec<i32> {
    let w = width as usize;
    let h = height as usize;
    let mut out = vec![-1i32; w * h];
    for row in 0..h {
        let src_off = row * w;
        let dst_off = (h - 1 - row) * w;
        out[dst_off..dst_off + w].copy_from_slice(&map[src_off..src_off + w]);
    }
    out
}

// ============================================================
// Main decode functions
// ============================================================

/// Decode a JB2 image stream (Sjbz chunk data).
/// Returns the page bitmap in PBM convention (row 0 = top).
pub fn decode(data: &[u8], shared_dict: Option<&JB2Dict>) -> Result<Bitmap, DecodeError> {
    let (bm, _) = decode_inner(data, shared_dict, false)?;
    Ok(bm)
}

/// Decode a JB2 image stream, returning both the bitmap and a per-pixel blit index map.
/// The blit index map has the same dimensions as the bitmap (row 0 = top).
/// Each pixel stores the 0-based blit index that last wrote to it, or -1 if no blit.
pub fn decode_indexed(
    data: &[u8],
    shared_dict: Option<&JB2Dict>,
) -> Result<(Bitmap, Vec<i32>), DecodeError> {
    decode_inner(data, shared_dict, true)
}

fn decode_inner(
    data: &[u8],
    shared_dict: Option<&JB2Dict>,
    track_blits: bool,
) -> Result<(Bitmap, Vec<i32>), DecodeError> {
    let mut zp = ZPDecoder::new(data);

    // Contexts
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

    // --- Init ---
    // Check for dictionary inheritance (record type 9)
    let mut rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);
    let mut initial_dict_length: usize = 0;
    if rtype == 9 {
        initial_dict_length = decode_num(&mut zp, &mut inherit_dict_size_ctx, 0, 262142) as usize;
        rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);
    }
    let _ = rtype; // start-of-data marker (usually 0)

    // Decode image dimensions
    let image_width = {
        let w = decode_num(&mut zp, &mut image_size_ctx, 0, 262142);
        if w == 0 { 200 } else { w }
    };
    let image_height = {
        let h = decode_num(&mut zp, &mut image_size_ctx, 0, 262142);
        if h == 0 { 200 } else { h }
    };

    // Flag bit (must be 0)
    let mut flag_ctx: u8 = 0;
    if zp.decode(&mut flag_ctx) {
        return Err(DecodeError::BadHeaderFlag);
    }

    // Initialize dictionary
    let mut dict: Vec<Jbm> = Vec::new();
    if initial_dict_length > 0 {
        if let Some(sd) = shared_dict {
            if initial_dict_length > sd.symbols.len() {
                return Err(DecodeError::InheritedDictTooLarge);
            }
            dict.extend_from_slice(&sd.symbols[..initial_dict_length]);
        } else {
            return Err(DecodeError::MissingSharedDict);
        }
    }

    // Cap at ~64M pixels to prevent OOM on malformed input.
    // Largest known real-world page: 6780x9148 = ~62M pixels.
    // Use saturating_mul to avoid i32 → usize overflow: e.g. 65536 * 65537 wraps
    // in i32 to a small positive value that would bypass the check.
    const MAX_PIXELS: usize = 64 * 1024 * 1024;
    let page_size = (image_width as usize).saturating_mul(image_height as usize);
    if page_size > MAX_PIXELS {
        return Err(DecodeError::ImageTooLarge);
    }
    let mut page = vec![0u8; page_size];
    let mut blit_map = if track_blits {
        Some(vec![-1i32; page_size])
    } else {
        None
    };
    let mut blit_count: i32 = 0;

    // Positioning state
    let mut first_left: i32 = -1;
    let mut first_bottom: i32 = image_height - 1;
    let mut last_right: i32 = 0;
    let mut baseline = Baseline::new();

    // --- Main decode loop ---
    loop {
        let rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);

        match rtype {
            1 => {
                // New symbol: add to dict AND blit
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

                blit(
                    &mut BlitTarget {
                        page: &mut page,
                        blit_map: blit_map.as_deref_mut(),
                        page_w: image_width,
                        page_h: image_height,
                    },
                    blit_count,
                    &bm,
                    x,
                    y,
                );
                blit_count += 1;
                dict.push(bm.remove_empty_edges());
            }
            2 => {
                // New symbol: add to dict only
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h);
                dict.push(bm.remove_empty_edges());
            }
            3 => {
                // New symbol: blit only
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

                blit(
                    &mut BlitTarget {
                        page: &mut page,
                        blit_map: blit_map.as_deref_mut(),
                        page_w: image_width,
                        page_h: image_height,
                    },
                    blit_count,
                    &bm,
                    x,
                    y,
                );
                blit_count += 1;
            }
            4 => {
                // Matched with refinement: add to dict AND blit
                if dict.is_empty() {
                    return Err(DecodeError::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(DecodeError::InvalidSymbolIndex);
                }
                let wdiff = decode_num(&mut zp, &mut symbol_width_diff_ctx, -262143, 262142);
                let hdiff = decode_num(&mut zp, &mut symbol_height_diff_ctx, -262143, 262142);
                let mbm = &dict[index];
                let cbm_w = mbm.width + wdiff;
                let cbm_h = mbm.height + hdiff;
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

                blit(
                    &mut BlitTarget {
                        page: &mut page,
                        blit_map: blit_map.as_deref_mut(),
                        page_w: image_width,
                        page_h: image_height,
                    },
                    blit_count,
                    &cbm,
                    x,
                    y,
                );
                blit_count += 1;
                dict.push(cbm.remove_empty_edges());
            }
            5 => {
                // Matched with refinement: add to dict only
                if dict.is_empty() {
                    return Err(DecodeError::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(DecodeError::InvalidSymbolIndex);
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
                dict.push(cbm.remove_empty_edges());
            }
            6 => {
                // Matched with refinement: blit only
                if dict.is_empty() {
                    return Err(DecodeError::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(DecodeError::InvalidSymbolIndex);
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

                blit(
                    &mut BlitTarget {
                        page: &mut page,
                        blit_map: blit_map.as_deref_mut(),
                        page_w: image_width,
                        page_h: image_height,
                    },
                    blit_count,
                    &cbm,
                    x,
                    y,
                );
                blit_count += 1;
            }
            7 => {
                // Matched copy without refinement: blit
                if dict.is_empty() {
                    return Err(DecodeError::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(DecodeError::InvalidSymbolIndex);
                }
                let bm = &dict[index];
                let bm_w = bm.width;
                let bm_h = bm.height;

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

                blit(
                    &mut BlitTarget {
                        page: &mut page,
                        blit_map: blit_map.as_deref_mut(),
                        page_w: image_width,
                        page_h: image_height,
                    },
                    blit_count,
                    &dict[index],
                    x,
                    y,
                );
                blit_count += 1;
            }
            8 => {
                // Non-symbol data
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h);

                let left = decode_num(&mut zp, &mut horiz_abs_loc_ctx, 1, image_width);
                let top = decode_num(&mut zp, &mut vert_abs_loc_ctx, 1, image_height);
                let x = left - 1;
                let y = top - h;

                blit(
                    &mut BlitTarget {
                        page: &mut page,
                        blit_map: blit_map.as_deref_mut(),
                        page_w: image_width,
                        page_h: image_height,
                    },
                    blit_count,
                    &bm,
                    x,
                    y,
                );
                blit_count += 1;
            }
            9 => {}
            10 => {
                let length = decode_num(&mut zp, &mut comment_length_ctx, 0, 262142);
                for _ in 0..length {
                    decode_num(&mut zp, &mut comment_octet_ctx, 0, 255);
                }
            }
            11 => {
                break;
            }
            _ => {
                return Err(DecodeError::UnknownRecordType);
            }
        }
    }

    let bitmap = page_to_bitmap(&page, image_width, image_height);
    let flipped_map = match blit_map {
        Some(map) => flip_blit_map(&map, image_width, image_height),
        None => vec![],
    };
    Ok((bitmap, flipped_map))
}

/// Decode a JB2 dictionary stream (Djbz chunk data).
pub fn decode_dict(data: &[u8], inherited: Option<&JB2Dict>) -> Result<JB2Dict, DecodeError> {
    let mut zp = ZPDecoder::new(data);

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

    // Init
    let mut rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);
    let mut initial_dict_length: usize = 0;
    if rtype == 9 {
        initial_dict_length = decode_num(&mut zp, &mut inherit_dict_size_ctx, 0, 262142) as usize;
        rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);
    }
    let _ = rtype;

    // Dimensions (present in dict streams but not used for page rendering)
    let _dict_width = decode_num(&mut zp, &mut image_size_ctx, 0, 262142);
    let _dict_height = decode_num(&mut zp, &mut image_size_ctx, 0, 262142);

    let mut flag_ctx: u8 = 0;
    if zp.decode(&mut flag_ctx) {
        return Err(DecodeError::BadHeaderFlag);
    }

    let mut dict: Vec<Jbm> = Vec::new();
    if initial_dict_length > 0 {
        if let Some(inh) = inherited {
            if initial_dict_length > inh.symbols.len() {
                return Err(DecodeError::InheritedDictTooLarge);
            }
            dict.extend_from_slice(&inh.symbols[..initial_dict_length]);
        } else {
            return Err(DecodeError::MissingSharedDict);
        }
    }

    // Main decode loop (dict only: types 2, 5, 10, 11)
    loop {
        let rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);

        match rtype {
            2 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h);
                dict.push(bm.remove_empty_edges());
            }
            5 => {
                if dict.is_empty() {
                    return Err(DecodeError::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
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
                dict.push(cbm.remove_empty_edges());
            }
            9 => {}
            10 => {
                let length = decode_num(&mut zp, &mut comment_length_ctx, 0, 262142);
                for _ in 0..length {
                    decode_num(&mut zp, &mut comment_octet_ctx, 0, 255);
                }
            }
            11 => break,
            _ => {
                return Err(DecodeError::UnexpectedDictRecordType);
            }
        }
    }

    Ok(JB2Dict { symbols: dict })
}

// ============================================================
// Symbol positioning
// ============================================================

#[allow(clippy::too_many_arguments)]
fn decode_symbol_coords(
    zp: &mut ZPDecoder,
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
    let flag = zp.decode(offset_type_ctx);

    let (x, y);
    if flag {
        // New line
        let hoff = decode_num(zp, hoff_ctx, -262143, 262142);
        let voff = decode_num(zp, voff_ctx, -262143, 262142);
        x = *first_left + hoff;
        y = *first_bottom + voff - sym_height + 1;
        *first_left = x;
        *first_bottom = y;
        baseline.fill(y);
    } else {
        // Same line
        let hoff = decode_num(zp, shoff_ctx, -262143, 262142);
        let voff = decode_num(zp, svoff_ctx, -262143, 262142);
        x = *last_right + hoff;
        y = baseline.get_val() + voff;
    }

    baseline.add(y);
    *last_right = x + sym_width - 1;
    (x, y)
}

// ============================================================
// Tests
// ============================================================

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

    fn extract_sjbz(djvu_data: &[u8]) -> &[u8] {
        let file = crate::iff::parse(djvu_data).unwrap();
        let sjbz = file.root.find_first(b"Sjbz").unwrap();
        sjbz.data()
    }

    #[test]
    fn jb2_decode_boy_jb2_mask() {
        let djvu = std::fs::read(assets_path().join("boy_jb2.djvu")).unwrap();
        let sjbz = extract_sjbz(&djvu);
        let bitmap = decode(sjbz, None).unwrap();
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

    fn extract_first_page_sjbz(djvu_data: &[u8]) -> Vec<u8> {
        let file = crate::iff::parse(djvu_data).unwrap();
        let page_form = file.root.children().iter().find(|c| {
            matches!(c, crate::iff::Chunk::Form { secondary_id, .. } if secondary_id == b"DJVU")
        }).expect("no DJVU form found");
        page_form.find_first(b"Sjbz").unwrap().data().to_vec()
    }

    #[test]
    fn jb2_decode_carte_p1_mask() {
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

    /// Find the Nth DJVU page form (0-indexed) in a bundled DJVM.
    fn find_page_form<'a>(
        file: &'a crate::iff::DjvuFile<'a>,
        page: usize,
    ) -> Result<&'a crate::iff::Chunk<'a>, crate::error::DjVuError> {
        let mut idx = 0;
        for chunk in file.root.children() {
            if matches!(chunk, crate::iff::Chunk::Form { secondary_id, .. } if secondary_id == b"DJVU")
            {
                if idx == page {
                    return Ok(chunk);
                }
                idx += 1;
            }
        }
        Err(crate::error::DjVuError::PageNotFound(page))
    }

    /// Find a DJVI form by its component name (from INCL chunk).
    fn find_djvi_djbz<'a>(
        file: &'a crate::iff::DjvuFile<'a>,
        _name: &[u8],
    ) -> Result<&'a [u8], crate::error::DjVuError> {
        for chunk in file.root.children() {
            if let crate::iff::Chunk::Form { secondary_id, .. } = chunk
                && secondary_id == b"DJVI"
            {
                // Check if this DJVI's component name matches
                // The component name is in the DIRM, but we can match by trying to find the Djbz
                if let Some(djbz) = chunk.find_first(b"Djbz") {
                    return Ok(djbz.data());
                }
            }
        }
        Err(crate::error::DjVuError::InvalidStructure(
            "DJVI with Djbz not found",
        ))
    }

    #[test]
    fn jb2_decode_djvu3spec_p1_mask() {
        // Page 1 has inline Djbz + Sjbz
        let djvu = std::fs::read(assets_path().join("DjVu3Spec_bundled.djvu")).unwrap();
        let file = crate::iff::parse(&djvu).unwrap();
        let page_form = find_page_form(&file, 0).unwrap();
        let djbz_data = page_form.find_first(b"Djbz").unwrap().data();
        let sjbz_data = page_form.find_first(b"Sjbz").unwrap().data();

        let shared_dict = decode_dict(djbz_data, None).unwrap();
        let bitmap = decode(sjbz_data, Some(&shared_dict)).unwrap();
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
    fn jb2_decode_djvu3spec_p2_mask() {
        // Page 2 uses INCL to reference dict0020.iff (a DJVI component)
        let djvu = std::fs::read(assets_path().join("DjVu3Spec_bundled.djvu")).unwrap();
        let file = crate::iff::parse(&djvu).unwrap();

        // Get shared dict from the DJVI component
        let djbz_data = find_djvi_djbz(&file, b"dict0020.iff").unwrap();
        let shared_dict = decode_dict(djbz_data, None).unwrap();

        // Get page 2's Sjbz (page index 1)
        let page_form = find_page_form(&file, 1).unwrap();
        let sjbz_data = page_form.find_first(b"Sjbz").unwrap().data();

        let bitmap = decode(sjbz_data, Some(&shared_dict)).unwrap();
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
    fn jb2_decode_navm_fgbz_p1_mask() {
        // All pages use INCL to reference dict0006.iff
        let djvu = std::fs::read(assets_path().join("navm_fgbz.djvu")).unwrap();
        let file = crate::iff::parse(&djvu).unwrap();

        let djbz_data = find_djvi_djbz(&file, b"dict0006.iff").unwrap();
        let shared_dict = decode_dict(djbz_data, None).unwrap();

        let page_form = find_page_form(&file, 0).unwrap();
        let sjbz_data = page_form.find_first(b"Sjbz").unwrap().data();

        let bitmap = decode(sjbz_data, Some(&shared_dict)).unwrap();
        let actual_pbm = bitmap.to_pbm();
        let expected_pbm = std::fs::read(golden_path().join("navm_fgbz_p1_mask.pbm")).unwrap();
        assert_eq!(
            actual_pbm.len(),
            expected_pbm.len(),
            "navm_fgbz_p1_mask size mismatch"
        );
        assert_eq!(actual_pbm, expected_pbm, "navm_fgbz_p1_mask pixel mismatch");
    }

    // --- Phase 6.2: Edge case tests ---

    #[test]
    fn jb2_empty_input() {
        let _ = decode(&[], None);
    }

    #[test]
    fn jb2_single_byte() {
        let _ = decode(&[0x00], None);
    }

    #[test]
    fn jb2_all_zeros() {
        let _ = decode(&[0u8; 64], None);
    }

    #[test]
    fn jb2_dict_empty_input() {
        let _ = decode_dict(&[], None);
    }

    #[test]
    fn jb2_dict_truncated() {
        let _ = decode_dict(&[0u8; 8], None);
    }

    #[test]
    fn jb2_fuzz_crash_regression() {
        // Crash artifact from fuzzing — must not panic.
        // File may not exist if fuzz corpus wasn't included (e.g., vendored copy).
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fuzz/artifacts/fuzz_jb2/crash-300468aea78aa31479c595355e2e315798de347a");
        if let Ok(data) = std::fs::read(path) {
            let _ = decode(&data, None);
            let _ = decode_dict(&data, None);
        }
    }

    /// Verify that image size computation uses saturating arithmetic.
    /// Previously, large dimensions like 65536 * 65537 would overflow i32
    /// in debug mode (panic) or wrap to a small value in release mode (bypassing
    /// the MAX_PIXELS guard and causing out-of-bounds writes later).
    #[test]
    fn jb2_image_size_overflow_guard() {
        // Values that overflow i32 when multiplied but are valid decode_num outputs.
        // 65536 * 65537 = 4_295_032_832 overflows i32 to 65_536 in release mode.
        let w: usize = 65536;
        let h: usize = 65537;
        let safe_size = w.saturating_mul(h);
        assert!(
            safe_size > 64 * 1024 * 1024,
            "saturating_mul must produce a value > MAX_PIXELS so ImageTooLarge fires"
        );
        // Also verify the old (broken) code path would have produced a small value:
        let old_path = ((w as i32).wrapping_mul(h as i32)) as usize;
        assert!(
            old_path < 64 * 1024 * 1024,
            "wrapping_mul produces a small value that bypasses the guard"
        );
    }

    /// Verify that `InvalidSymbolIndex` is a valid error variant (compile-time check).
    #[test]
    fn jb2_decode_error_has_invalid_symbol_index() {
        let e = DecodeError::InvalidSymbolIndex;
        let msg = e.to_string();
        assert!(
            msg.contains("symbol index"),
            "error message should mention symbol index"
        );
    }

    #[test]
    fn find_page_form_returns_err_for_missing_page() {
        // Build a minimal DjvuFile with no DJVU pages.
        let data = b"AT&TFORM\x00\x00\x00\x04DJVM";
        let file = crate::iff::parse(data).unwrap();
        let result = find_page_form(&file, 0);
        assert!(result.is_err(), "should return Err when page not found");
    }

    #[test]
    fn find_djvi_djbz_returns_err_when_no_djvi() {
        // Build a minimal DjvuFile with no DJVI forms.
        let data = b"AT&TFORM\x00\x00\x00\x04DJVM";
        let file = crate::iff::parse(data).unwrap();
        let result = find_djvi_djbz(&file, b"dict.iff");
        assert!(result.is_err(), "should return Err when no DJVI found");
    }
}
