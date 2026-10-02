//! The working symbol table and the core dictionary (Djbz) decoder.

use super::*;

// ────────────────────────────────────────────────────────────────────────────
// Working symbol table: zero-copy view of shared dict + local symbols
// ────────────────────────────────────────────────────────────────────────────

/// Two-part symbol table used during JB2 image/dict decode.
///
/// The `shared` slice refers directly to the cached shared dictionary's symbols
/// (no clone), while `local` holds symbols defined by the stream being decoded.
/// This avoids deep-copying the (potentially large) shared dictionary on every
/// `decode_mask()` call.
pub(super) struct JbmDict<'a> {
    pub(super) shared: &'a [Jbm],
    pub(super) local: Vec<Jbm>,
}

impl<'a> JbmDict<'a> {
    pub(super) fn new(shared: &'a [Jbm]) -> Self {
        JbmDict {
            shared,
            local: Vec::new(),
        }
    }
    pub(super) fn len(&self) -> usize {
        self.shared.len() + self.local.len()
    }
    pub(super) fn is_empty(&self) -> bool {
        self.shared.is_empty() && self.local.is_empty()
    }
    pub(super) fn push(&mut self, sym: Jbm) {
        self.local.push(sym);
    }
    pub(super) fn into_symbols(self) -> Vec<Jbm> {
        // Used by decode_dictionary to return the complete symbol list.
        let mut out = self.shared.to_vec();
        out.extend(self.local);
        out
    }
}

impl core::ops::Index<usize> for JbmDict<'_> {
    type Output = Jbm;
    #[inline(always)]
    fn index(&self, index: usize) -> &Jbm {
        let n = self.shared.len();
        if index < n {
            &self.shared[index]
        } else {
            &self.local[index - n]
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Core dictionary decode
// ────────────────────────────────────────────────────────────────────────────

pub(super) fn decode_dictionary(
    data: &[u8],
    inherited: Option<&Jb2Dict>,
) -> Result<Jb2Dict, Jb2Error> {
    let mut pool: Vec<u8> = Vec::new();
    decode_dictionary_with_pool(data, inherited, &mut pool)
}

pub(super) fn decode_dictionary_with_pool(
    data: &[u8],
    inherited: Option<&Jb2Dict>,
    pool: &mut Vec<u8>,
) -> Result<Jb2Dict, Jb2Error> {
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

    let mut direct_bitmap_ctx = [0u8; 1024];
    let mut refinement_bitmap_ctx = [0u8; 2048];
    let mut refinement_bitmap_ctx_p = [0x8000u16; 2048];
    let mut total_sym_pixels = 0usize;

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

    let initial_inh: &[Jbm] = if initial_dict_length > 0 {
        match inherited {
            Some(inh) => {
                if initial_dict_length > inh.symbols.len() {
                    return Err(Jb2Error::InheritedDictTooLarge);
                }
                &inh.symbols[..initial_dict_length]
            }
            None => return Err(Jb2Error::MissingSharedDict),
        }
    } else {
        &[]
    };
    let mut dict = JbmDict::new(initial_inh);

    // Dict streams only accept types 2, 5, 9, 10, 11
    let max_sym_px = MAX_TOTAL_SYMBOL_PIXELS;
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
            // 2 — new symbol, direct decode → add to dict
            2 => {
                let w = decode_num(&mut zp, &mut symbol_width_ctx, 0, 262142);
                let h = decode_num(&mut zp, &mut symbol_height_ctx, 0, 262142);
                check_symbol_decode_budget(&zp, w, h, 1, &mut total_sym_pixels, max_sym_px)?;
                let bm = decode_bitmap_direct(&mut zp, &mut direct_bitmap_ctx, w, h, pool)?;
                dict.push(bm.crop_and_recycle(pool));
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
                check_symbol_decode_budget(
                    &zp,
                    cbm_w,
                    cbm_h,
                    1,
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

            // 9 — required-dict-or-reset (ignored in dict streams)
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

            _ => return Err(Jb2Error::UnexpectedDictRecordType),
        }
    }

    Ok(Jb2Dict {
        symbols: dict.into_symbols(),
    })
}
