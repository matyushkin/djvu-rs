//! The core image (Sjbz) decoder.

use super::*;

// ────────────────────────────────────────────────────────────────────────────
// Core image decode
// ────────────────────────────────────────────────────────────────────────────

pub(super) fn decode_image(data: &[u8], shared_dict: Option<&Jb2Dict>) -> Result<Bitmap, Jb2Error> {
    let mut pool = Vec::new();
    decode_image_with_pool(data, shared_dict, &mut pool, 0)
}

/// Decode a JB2 image stream, reusing `pool` as a scratch buffer for symbol bitmaps.
///
/// `pool` is resized up (never shrunk) across symbol decodes, eliminating
/// per-symbol heap allocations. Pass `&mut Vec::new()` to use a fresh pool,
/// or reuse a pool across multiple decode calls for additional savings.
///
/// `shift`: `0` blits each decoded symbol into a full-resolution page canvas
/// (the normal path). `>= 1` blits into a `1/2^shift`-resolution canvas
/// instead, OR-reducing each pixel into its downsampled cell — see
/// [`decode_downsampled`].
pub(super) fn decode_image_with_pool(
    data: &[u8],
    shared_dict: Option<&Jb2Dict>,
    pool: &mut Vec<u8>,
    shift: u32,
) -> Result<Bitmap, Jb2Error> {
    let mut zp = ZpDecoder::new(data).map_err(|_| Jb2Error::ZpInitFailed)?;

    // Contexts for variable-length integer decoding
    let mut record_type_ctx = NumContext::new();
    let mut image_size_ctx = NumContext::new();
    let mut symbol_width_ctx = NumContext::new();
    let mut symbol_height_ctx = NumContext::new();
    let mut inherit_dict_size_ctx = NumContext::new();
    let mut coord_ctx = CoordContexts::new();
    let mut symbol_index_ctx = NumContext::new();
    let mut symbol_width_diff_ctx = NumContext::new();
    let mut symbol_height_diff_ctx = NumContext::new();
    let mut horiz_abs_loc_ctx = NumContext::new();
    let mut vert_abs_loc_ctx = NumContext::new();
    let mut comment_length_ctx = NumContext::new();
    let mut comment_octet_ctx = NumContext::new();

    let mut direct_bitmap_ctx = [0u8; 1024];
    let mut refinement_bitmap_ctx = [0u8; 2048];
    let mut refinement_bitmap_ctx_p = [0x8000u16; 2048];
    let mut total_sym_pixels = 0usize;
    let mut total_blit_pixels = 0usize;

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

    // Populate initial dictionary from shared dict — zero-copy: borrow the
    // cached dict's symbol slice directly rather than deep-cloning it.
    let initial_symbols: &[Jbm] = if initial_dict_length > 0 {
        match shared_dict {
            Some(sd) => {
                if initial_dict_length > sd.symbols.len() {
                    return Err(Jb2Error::InheritedDictTooLarge);
                }
                &sd.symbols[..initial_dict_length]
            }
            None => return Err(Jb2Error::MissingSharedDict),
        }
    } else {
        &[]
    };
    let mut dict = JbmDict::new(initial_symbols);

    // Safety cap: ~64M pixels (same guard, but now the backing store is 8× smaller).
    const MAX_PIXELS: usize = 64 * 1024 * 1024;
    let page_size = (image_width as usize).saturating_mul(image_height as usize);
    if page_size > MAX_PIXELS {
        return Err(Jb2Error::ImageTooLarge);
    }
    // Use a packed 1-bit-per-pixel bitmap as the page buffer instead of a
    // byte-per-pixel Vec. This is 8× smaller (~1.8 MB vs ~14.5 MB for a 600 dpi
    // page), fitting in L2 cache and dramatically reducing cache pressure during blits.
    //
    // At `shift >= 1` the canvas itself is allocated at `1/2^shift` resolution
    // (`div_ceil` so a ragged edge still gets its own partial cell) and every
    // blit below OR-reduces into it instead of the full-resolution canvas —
    // see `decode_downsampled`.
    let (page_w, page_h) = if shift == 0 {
        (image_width as u32, image_height as u32)
    } else {
        (
            (image_width as u32).div_ceil(1u32 << shift),
            (image_height as u32).div_ceil(1u32 << shift),
        )
    };
    let mut page_bm = Bitmap::new(page_w, page_h);
    // Closure so every blit call site below stays a one-liner; `shift` is
    // invariant for the whole decode so the branch predicts perfectly, and
    // the `shift == 0` arm is byte-for-byte the pre-existing fast path.
    let blit = |page_bm: &mut Bitmap, sym: &Jbm, x: i32, y: i32| {
        if shift == 0 {
            blit_to_bitmap(page_bm, sym, x, y);
        } else {
            blit_to_bitmap_downsampled(page_bm, sym, x, y, image_width, image_height, shift);
        }
    };

    let mut layout = LayoutState::new(image_height);

    // Main decode loop — capped to prevent infinite spin when ZP input is exhausted
    let max_sym_px = MAX_PAGE_SYMBOL_WORK;
    let mut record_count = 0usize;
    loop {
        if zp.synthetic_bytes() > ZP_EOF_SLACK_BYTES {
            // ZP input is exhausted and the record loop is now spinning on
            // synthetic `0xFF` fill: every record decoded past this point comes
            // from padding, including types that decode no symbol (7/9/10) and
            // so never reach `check_symbol_decode_budget`. Bail well before
            // MAX_RECORDS so corrupt/truncated streams stop fast. A valid stream
            // reaches its type-11 end record while `synthetic_bytes` is still
            // within the look-ahead slack, so this never rejects a good page.
            return Err(Jb2Error::Truncated);
        }
        if record_count >= MAX_RECORDS {
            return Err(Jb2Error::TooManyRecords);
        }
        record_count += 1;
        let rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);

        match rtype {
            // 1 — new symbol, direct decode → add to dict AND blit
            1 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                check_symbol_decode_budget(&zp, w, h, 1, &mut total_sym_pixels, max_sym_px)?;
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h, pool)?;
                let (x, y) =
                    decode_symbol_coords(&mut zp, &mut coord_ctx, &mut layout, bm.width, bm.height);
                check_blit_budget(&bm, &mut total_blit_pixels)?;
                blit(&mut page_bm, &bm, x, y);
                dict.push(bm.crop_and_recycle(pool));
            }

            // 2 — new symbol, direct decode → add to dict only
            2 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                check_symbol_decode_budget(&zp, w, h, 1, &mut total_sym_pixels, max_sym_px)?;
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h, pool)?;
                dict.push(bm.crop_and_recycle(pool));
            }

            // 3 — new symbol, direct decode → blit only (not stored in dict)
            3 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                check_symbol_decode_budget(&zp, w, h, 1, &mut total_sym_pixels, max_sym_px)?;
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h, pool)?;
                let (x, y) =
                    decode_symbol_coords(&mut zp, &mut coord_ctx, &mut layout, bm.width, bm.height);
                check_blit_budget(&bm, &mut total_blit_pixels)?;
                blit(&mut page_bm, &bm, x, y);
                bm.recycle_into(pool);
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
                check_symbol_decode_budget(
                    &zp,
                    cbm_w,
                    cbm_h,
                    REFINE_PIXEL_WORK,
                    &mut total_sym_pixels,
                    max_sym_px,
                )?;
                let cbm = decode_bitmap_ref(
                    &mut zp,
                    &mut refinement_bitmap_ctx,
                    &mut refinement_bitmap_ctx_p,
                    cbm_w,
                    cbm_h,
                    &dict[index],
                    pool,
                )?;
                let (x, y) = decode_symbol_coords(
                    &mut zp,
                    &mut coord_ctx,
                    &mut layout,
                    cbm.width,
                    cbm.height,
                );
                check_blit_budget(&cbm, &mut total_blit_pixels)?;
                blit(&mut page_bm, &cbm, x, y);
                dict.push(cbm.crop_and_recycle(pool));
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
                check_symbol_decode_budget(
                    &zp,
                    cbm_w,
                    cbm_h,
                    REFINE_PIXEL_WORK,
                    &mut total_sym_pixels,
                    max_sym_px,
                )?;
                let cbm = decode_bitmap_ref(
                    &mut zp,
                    &mut refinement_bitmap_ctx,
                    &mut refinement_bitmap_ctx_p,
                    cbm_w,
                    cbm_h,
                    &dict[index],
                    pool,
                )?;
                dict.push(cbm.crop_and_recycle(pool));
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
                check_symbol_decode_budget(
                    &zp,
                    cbm_w,
                    cbm_h,
                    REFINE_PIXEL_WORK,
                    &mut total_sym_pixels,
                    max_sym_px,
                )?;
                let cbm = decode_bitmap_ref(
                    &mut zp,
                    &mut refinement_bitmap_ctx,
                    &mut refinement_bitmap_ctx_p,
                    cbm_w,
                    cbm_h,
                    &dict[index],
                    pool,
                )?;
                let (x, y) = decode_symbol_coords(
                    &mut zp,
                    &mut coord_ctx,
                    &mut layout,
                    cbm.width,
                    cbm.height,
                );
                check_blit_budget(&cbm, &mut total_blit_pixels)?;
                blit(&mut page_bm, &cbm, x, y);
                cbm.recycle_into(pool);
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
                let (x, y) = decode_symbol_coords(&mut zp, &mut coord_ctx, &mut layout, bm_w, bm_h);
                let sym = &dict[index];
                check_blit_budget(sym, &mut total_blit_pixels)?;
                blit(&mut page_bm, sym, x, y);
            }

            // 8 — non-symbol (halftone), absolute coordinates
            8 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                check_symbol_decode_budget(&zp, w, h, 1, &mut total_sym_pixels, max_sym_px)?;
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h, pool)?;
                let left = decode_num(&mut zp, &mut horiz_abs_loc_ctx, 1, image_width);
                let top = decode_num(&mut zp, &mut vert_abs_loc_ctx, 1, image_height);
                let x = left - 1;
                let y = top - h;
                check_blit_budget(&bm, &mut total_blit_pixels)?;
                blit(&mut page_bm, &bm, x, y);
                bm.recycle_into(pool);
            }

            // 9 — required-dict-or-reset (already consumed in preamble; ignore here)
            9 => {}

            // 10 — comment: skip bytes
            10 => {
                let length = decode_num(&mut zp, &mut comment_length_ctx, 0, 262142) as usize;
                // Consume ALL `length` octets: decode_num is ZP-stateful, so
                // skipping any (e.g. capping the loop) desynchronizes the
                // arithmetic coder for every following record — silent corruption.
                // `length` ≤ 262142 (decode_num range) already bounds the loop.
                for _ in 0..length {
                    decode_num(&mut zp, &mut comment_octet_ctx, 0, 255);
                }
            }

            // 11 — end-of-data
            11 => break,

            _ => return Err(Jb2Error::UnknownRecordType),
        }
    }

    Ok(page_bm)
}

/// Same as `decode_image` but tracks per-pixel blit indices.
pub(super) fn decode_image_indexed(
    data: &[u8],
    shared_dict: Option<&Jb2Dict>,
) -> Result<(Bitmap, Vec<i32>), Jb2Error> {
    let mut pool = Vec::new();
    decode_image_indexed_with_pool(data, shared_dict, &mut pool)
}

pub(super) fn decode_image_indexed_with_pool(
    data: &[u8],
    shared_dict: Option<&Jb2Dict>,
    pool: &mut Vec<u8>,
) -> Result<(Bitmap, Vec<i32>), Jb2Error> {
    let mut zp = ZpDecoder::new(data).map_err(|_| Jb2Error::ZpInitFailed)?;

    let mut record_type_ctx = NumContext::new();
    let mut image_size_ctx = NumContext::new();
    let mut symbol_width_ctx = NumContext::new();
    let mut symbol_height_ctx = NumContext::new();
    let mut inherit_dict_size_ctx = NumContext::new();
    let mut coord_ctx = CoordContexts::new();
    let mut symbol_index_ctx = NumContext::new();
    let mut symbol_width_diff_ctx = NumContext::new();
    let mut symbol_height_diff_ctx = NumContext::new();
    let mut horiz_abs_loc_ctx = NumContext::new();
    let mut vert_abs_loc_ctx = NumContext::new();
    let mut comment_length_ctx = NumContext::new();
    let mut comment_octet_ctx = NumContext::new();

    let mut direct_bitmap_ctx = [0u8; 1024];
    let mut refinement_bitmap_ctx = [0u8; 2048];
    let mut refinement_bitmap_ctx_p = [0x8000u16; 2048];
    let mut total_sym_pixels = 0usize;
    let mut total_blit_pixels = 0usize;

    let mut rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);
    let mut initial_dict_length: usize = 0;
    if rtype == 9 {
        initial_dict_length = decode_num(&mut zp, &mut inherit_dict_size_ctx, 0, 262142) as usize;
        rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);
    }
    let _ = rtype;

    let image_width = {
        let w = decode_num(&mut zp, &mut image_size_ctx, 0, 262142);
        if w == 0 { 200 } else { w }
    };
    let image_height = {
        let h = decode_num(&mut zp, &mut image_size_ctx, 0, 262142);
        if h == 0 { 200 } else { h }
    };

    let mut flag_ctx: u8 = 0;
    if zp.decode_bit(&mut flag_ctx) {
        return Err(Jb2Error::BadHeaderFlag);
    }

    let initial_symbols_idx: &[Jbm] = if initial_dict_length > 0 {
        match shared_dict {
            Some(sd) => {
                if initial_dict_length > sd.symbols.len() {
                    return Err(Jb2Error::InheritedDictTooLarge);
                }
                &sd.symbols[..initial_dict_length]
            }
            None => return Err(Jb2Error::MissingSharedDict),
        }
    } else {
        &[]
    };
    let mut dict = JbmDict::new(initial_symbols_idx);

    const MAX_PIXELS: usize = 64 * 1024 * 1024;
    let page_size = (image_width as usize).saturating_mul(image_height as usize);
    if page_size > MAX_PIXELS {
        return Err(Jb2Error::ImageTooLarge);
    }
    let mut page = vec![0u8; page_size];
    let mut blit_map = vec![-1i32; page_size];

    let mut layout = LayoutState::new(image_height);
    let mut blit_count: i32 = 0;

    let max_sym_px = MAX_PAGE_SYMBOL_WORK;
    let mut record_count = 0usize;
    loop {
        if zp.synthetic_bytes() > ZP_EOF_SLACK_BYTES {
            // ZP input is exhausted and the record loop is now spinning on
            // synthetic `0xFF` fill: every record decoded past this point comes
            // from padding, including types that decode no symbol (7/9/10) and
            // so never reach `check_symbol_decode_budget`. Bail well before
            // MAX_RECORDS so corrupt/truncated streams stop fast. A valid stream
            // reaches its type-11 end record while `synthetic_bytes` is still
            // within the look-ahead slack, so this never rejects a good page.
            return Err(Jb2Error::Truncated);
        }
        if record_count >= MAX_RECORDS {
            return Err(Jb2Error::TooManyRecords);
        }
        record_count += 1;
        let rtype = decode_num(&mut zp, &mut record_type_ctx, 0, 11);

        match rtype {
            1 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                check_symbol_decode_budget(&zp, w, h, 1, &mut total_sym_pixels, max_sym_px)?;
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h, pool)?;
                let (x, y) =
                    decode_symbol_coords(&mut zp, &mut coord_ctx, &mut layout, bm.width, bm.height);
                check_blit_budget(&bm, &mut total_blit_pixels)?;
                blit_indexed(
                    &mut page,
                    &mut blit_map,
                    image_width,
                    image_height,
                    &bm,
                    x,
                    y,
                    blit_count,
                );
                blit_count += 1;
                dict.push(bm.crop_and_recycle(pool));
            }
            2 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                check_symbol_decode_budget(&zp, w, h, 1, &mut total_sym_pixels, max_sym_px)?;
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h, pool)?;
                dict.push(bm.crop_and_recycle(pool));
            }
            3 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                check_symbol_decode_budget(&zp, w, h, 1, &mut total_sym_pixels, max_sym_px)?;
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h, pool)?;
                let (x, y) =
                    decode_symbol_coords(&mut zp, &mut coord_ctx, &mut layout, bm.width, bm.height);
                check_blit_budget(&bm, &mut total_blit_pixels)?;
                blit_indexed(
                    &mut page,
                    &mut blit_map,
                    image_width,
                    image_height,
                    &bm,
                    x,
                    y,
                    blit_count,
                );
                blit_count += 1;
                bm.recycle_into(pool);
            }
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
                check_symbol_decode_budget(
                    &zp,
                    cbm_w,
                    cbm_h,
                    REFINE_PIXEL_WORK,
                    &mut total_sym_pixels,
                    max_sym_px,
                )?;
                let cbm = decode_bitmap_ref(
                    &mut zp,
                    &mut refinement_bitmap_ctx,
                    &mut refinement_bitmap_ctx_p,
                    cbm_w,
                    cbm_h,
                    &dict[index],
                    pool,
                )?;
                let (x, y) = decode_symbol_coords(
                    &mut zp,
                    &mut coord_ctx,
                    &mut layout,
                    cbm.width,
                    cbm.height,
                );
                check_blit_budget(&cbm, &mut total_blit_pixels)?;
                blit_indexed(
                    &mut page,
                    &mut blit_map,
                    image_width,
                    image_height,
                    &cbm,
                    x,
                    y,
                    blit_count,
                );
                blit_count += 1;
                dict.push(cbm.crop_and_recycle(pool));
            }
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
                check_symbol_decode_budget(
                    &zp,
                    cbm_w,
                    cbm_h,
                    REFINE_PIXEL_WORK,
                    &mut total_sym_pixels,
                    max_sym_px,
                )?;
                let cbm = decode_bitmap_ref(
                    &mut zp,
                    &mut refinement_bitmap_ctx,
                    &mut refinement_bitmap_ctx_p,
                    cbm_w,
                    cbm_h,
                    &dict[index],
                    pool,
                )?;
                dict.push(cbm.crop_and_recycle(pool));
            }
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
                check_symbol_decode_budget(
                    &zp,
                    cbm_w,
                    cbm_h,
                    REFINE_PIXEL_WORK,
                    &mut total_sym_pixels,
                    max_sym_px,
                )?;
                let cbm = decode_bitmap_ref(
                    &mut zp,
                    &mut refinement_bitmap_ctx,
                    &mut refinement_bitmap_ctx_p,
                    cbm_w,
                    cbm_h,
                    &dict[index],
                    pool,
                )?;
                let (x, y) = decode_symbol_coords(
                    &mut zp,
                    &mut coord_ctx,
                    &mut layout,
                    cbm.width,
                    cbm.height,
                );
                check_blit_budget(&cbm, &mut total_blit_pixels)?;
                blit_indexed(
                    &mut page,
                    &mut blit_map,
                    image_width,
                    image_height,
                    &cbm,
                    x,
                    y,
                    blit_count,
                );
                blit_count += 1;
                cbm.recycle_into(pool);
            }
            7 => {
                if dict.is_empty() {
                    return Err(Jb2Error::EmptyDictReference);
                }
                let index =
                    decode_num(&mut zp, &mut symbol_index_ctx, 0, dict.len() as i32 - 1) as usize;
                if index >= dict.len() {
                    return Err(Jb2Error::InvalidSymbolIndex);
                }
                let (x, y) = decode_symbol_coords(
                    &mut zp,
                    &mut coord_ctx,
                    &mut layout,
                    dict[index].width,
                    dict[index].height,
                );
                check_blit_budget(&dict[index], &mut total_blit_pixels)?;
                blit_indexed(
                    &mut page,
                    &mut blit_map,
                    image_width,
                    image_height,
                    &dict[index],
                    x,
                    y,
                    blit_count,
                );
                blit_count += 1;
            }
            8 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                check_symbol_decode_budget(&zp, w, h, 1, &mut total_sym_pixels, max_sym_px)?;
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h, pool)?;
                let left = decode_num(&mut zp, &mut horiz_abs_loc_ctx, 1, image_width);
                let top = decode_num(&mut zp, &mut vert_abs_loc_ctx, 1, image_height);
                check_blit_budget(&bm, &mut total_blit_pixels)?;
                blit_indexed(
                    &mut page,
                    &mut blit_map,
                    image_width,
                    image_height,
                    &bm,
                    left - 1,
                    top - h,
                    blit_count,
                );
                blit_count += 1;
                bm.recycle_into(pool);
            }
            9 => {}
            10 => {
                let length = decode_num(&mut zp, &mut comment_length_ctx, 0, 262142) as usize;
                // Consume ALL `length` octets: decode_num is ZP-stateful, so
                // skipping any (e.g. capping the loop) desynchronizes the
                // arithmetic coder for every following record — silent corruption.
                // `length` ≤ 262142 (decode_num range) already bounds the loop.
                for _ in 0..length {
                    decode_num(&mut zp, &mut comment_octet_ctx, 0, 255);
                }
            }
            11 => break,
            _ => return Err(Jb2Error::UnknownRecordType),
        }
    }

    let bm = page_to_bitmap(&page, image_width, image_height);
    flip_blit_map(&mut blit_map, image_width as usize, image_height as usize);
    Ok((bm, blit_map))
}
