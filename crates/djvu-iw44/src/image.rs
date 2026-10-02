//! The public progressive decoder, `Iw44Image`.

use super::*;

// ---- Public API -------------------------------------------------------------

/// Progressive IW44 wavelet image decoder.
///
/// Holds three independent planar decoders (Y, Cb, Cr) whose ZP context tables
/// persist across chunks, enabling progressive refinement.
///
/// ## Usage
///
/// ```no_run
/// use djvu_iw44::Iw44Image;
///
/// let chunk_data: &[u8] = &[]; // BG44 chunk bytes from the DjVu file
/// let mut img = Iw44Image::new();
/// // Feed each BG44 chunk in document order:
/// img.decode_chunk(chunk_data)?;
/// // Convert to an RGB pixmap once all desired chunks are decoded:
/// let pixmap = img.to_rgb()?;
/// # Ok::<(), djvu_iw44::Iw44Error>(())
/// ```
#[derive(Clone, Debug)]
pub struct Iw44Image {
    /// Luma plane dimensions (pixels, before subsampling).
    pub width: u32,
    /// Luma plane dimensions (pixels, before subsampling).
    pub height: u32,
    /// `true` for color (YCbCr) images, `false` for grayscale.
    pub(super) is_color: bool,
    /// Number of Y slices decoded before chroma decoding starts.
    pub(super) delay: u8,
    /// DjVuLibre's `crcb_half`: the Cb/Cr planes are full size, but a
    /// full-resolution picture reconstructs them only down to scale 2 and
    /// repeats each value over its 2x2 block (`IW44Image::Map::image`, `fast`).
    pub(super) chroma_half: bool,
    /// Luma plane decoder.
    pub(super) y: Option<PlaneDecoder>,
    /// Blue-difference chroma plane decoder (color images only).
    pub(super) cb: Option<PlaneDecoder>,
    /// Red-difference chroma plane decoder (color images only).
    pub(super) cr: Option<PlaneDecoder>,
    /// Total slices decoded so far (used to implement the color-delay counter).
    pub(super) cslice: usize,
    /// Expected `serial` value of the next chunk passed to [`decode_chunk`](Self::decode_chunk),
    /// starting at 0. Mirrors DjVuLibre's `cserial` counter — every chunk must
    /// arrive in strict document order (0, 1, 2, …); a mismatch means the
    /// chunk sequence is corrupted or desynced (see [`Iw44Error::UnexpectedSerial`]).
    pub(super) next_serial: u32,
}

impl Default for Iw44Image {
    fn default() -> Self {
        Self::new()
    }
}

impl Iw44Image {
    /// Heap bytes held by this image's decoded coefficient planes.
    ///
    /// A cache-budget accounting helper: an `Iw44Image` keeps one
    /// `PlaneDecoder` per colour plane, and each holds `ceil(w/32) *
    /// ceil(h/32)` blocks. A block stores its first 16 coefficients inline and
    /// grows a heap tail only up to its highest non-zero bucket, so the cost
    /// tracks how much detail the chunks actually carried, not `w x h`
    /// (PERF_EXPERIMENTS.md IW44_SPARSE_BLOCKS). Both terms are reported: the
    /// inline block array and the sum of the heap tails.
    ///
    /// Call it after every chunk, not once: the tails grow as later chunks
    /// refine the image. A colour image also costs more than its luma plane
    /// alone — `PageLayers::cached_bytes` in the parent crate once sized a
    /// cached image as `w x h x 2` and under-reported colour pages by ~2.6x
    /// (PERF_EXPERIMENTS.md DECODE_CACHE_ACCOUNTING).
    ///
    /// Returns 0 before the first chunk is decoded (no plane is allocated yet).
    pub fn heap_bytes(&self) -> usize {
        [self.y.as_ref(), self.cb.as_ref(), self.cr.as_ref()]
            .into_iter()
            .flatten()
            .map(PlaneDecoder::heap_bytes)
            .sum()
    }

    /// Create a new, empty decoder.
    pub fn new() -> Self {
        Iw44Image {
            width: 0,
            height: 0,
            is_color: false,
            delay: 0,
            chroma_half: false,
            y: None,
            cb: None,
            cr: None,
            cslice: 0,
            next_serial: 0,
        }
    }

    /// Returns the (width, height) of the Cb chroma plane as allocated.
    ///
    /// When `chroma_half=true` this should be `(ceil(w/2), ceil(h/2))`.
    /// Returns `None` if no color chunks have been decoded yet.
    #[cfg(test)]
    pub fn chroma_plane_dims(&self) -> Option<(usize, usize)> {
        self.cb.as_ref().map(|p| (p.width, p.height))
    }

    /// Returns `true` if the image is a color (YCbCr) image.
    #[cfg(test)]
    pub fn is_color(&self) -> bool {
        self.is_color
    }

    /// Returns `true` if chroma planes are stored at half resolution.
    #[cfg(test)]
    pub fn chroma_half(&self) -> bool {
        self.chroma_half
    }

    /// Decode one BG44/FG44/TH44 chunk.
    ///
    /// Call this once for each chunk in document order.  The ZP coder state
    /// is maintained internally so progressive refinement works automatically.
    ///
    /// ## Chunk format
    ///
    /// - First chunk (`serial == 0`): 9-byte header then ZP-coded payload.
    /// - Subsequent chunks: 2-byte header (`serial`, `slices`) then ZP payload.
    pub fn decode_chunk(&mut self, data: &[u8]) -> Result<(), Iw44Error> {
        if data.len() < 2 {
            return Err(Iw44Error::ChunkTooShort);
        }
        let serial = data[0];
        let slices = data[1];

        // Serial-number continuity check, mirroring DjVuLibre's `cserial`
        // counter in `IWBitmap::decode_chunk`/`IWPixmap::decode_chunk`
        // (`IW44Image.wrong_serial`/`wrong_serial2`): once a first chunk has
        // been decoded, every subsequent chunk must arrive with the exact
        // next serial in sequence (0, 1, 2, …). A mismatch — e.g. a bit-flip
        // landing on the serial byte itself, a dropped/duplicated chunk, or a
        // stray `serial == 0` chunk restarting mid-stream — is a corrupted or
        // desynced chunk sequence and must be rejected rather than silently
        // decoded into the wrong refinement slot (differential fuzzing
        // against `ddjvu` found real corpus mutations that trip this exact
        // check on DjVuLibre's side while we decoded on, unnoticed).
        //
        // The very first chunk fed to a fresh decoder (`self.y` still `None`)
        // is exempted here so `MissingFirstChunk` remains the more specific
        // diagnostic for "no first chunk decoded yet" when `serial != 0`.
        if self.y.is_some() && serial as u32 != self.next_serial {
            return Err(Iw44Error::UnexpectedSerial);
        }

        let payload_start = if serial == 0 {
            // First chunk — parse the 9-byte image header.
            if data.len() < 9 {
                return Err(Iw44Error::HeaderTooShort);
            }
            let majver = data[2];
            let minor = data[3];
            let is_grayscale = (majver >> 7) != 0;
            let w = u16::from_be_bytes([data[4], data[5]]);
            let h = u16::from_be_bytes([data[6], data[7]]);
            let delay_byte = data[8];
            let delay = if minor >= 2 { delay_byte & 127 } else { 0 };
            // A clear high bit is DjVuLibre's `crcb_half`. It never changes
            // the plane size: the Cb/Cr planes are always full size, and
            // allocating them at half size desynchronizes the adaptive ZP
            // streams into chroma noise (#561). It changes only how a
            // full-resolution picture is reconstructed (#830).
            let chroma_half = minor >= 2 && delay_byte & 0x80 == 0;

            if w == 0 || h == 0 {
                return Err(Iw44Error::ZeroDimension);
            }
            // Prevent OOM / slow decode on malformed input.
            // 64 MP allows real scanned documents (e.g. 6780×9148 ≈ 62 MP at 600 dpi)
            // while bounding worst-case decode cost. Measured at the cap
            // boundary (2026-08 fuzz slow-unit: color 1023×65535 ≈ 67.0 MP at
            // 99.9% of the cap, 241 slices in a 13-byte chunk): ~0.8 s and
            // ~400 MB peak (3 planes of 32×32 coefficient blocks) in a native
            // release build. The same input needs ~11.5 s under ASan+SanCov
            // fuzz instrumentation; that overhead is fuzz-only, not a codec
            // gap. See PERF_EXPERIMENTS.md (near-cap slow-unit triage).
            let pixels = w as u64 * h as u64;
            if pixels > 64 * 1024 * 1024 {
                return Err(Iw44Error::ImageTooLarge);
            }

            self.width = w as u32;
            self.height = h as u32;
            self.is_color = !is_grayscale;
            self.delay = delay;
            self.chroma_half = self.is_color && chroma_half;
            self.cslice = 0;
            self.y = Some(PlaneDecoder::new(w as usize, h as usize));
            if self.is_color {
                self.cb = Some(PlaneDecoder::new(w as usize, h as usize));
                self.cr = Some(PlaneDecoder::new(w as usize, h as usize));
            }
            9
        } else {
            if self.y.is_none() {
                return Err(Iw44Error::MissingFirstChunk);
            }
            2
        };

        // A refinement chunk's ZP payload may legitimately be shorter than
        // `ZpDecoder::new`'s 2-byte minimum — even empty — when the encoder had
        // nothing left to encode for these `slices` (observed on a real corpus
        // file: a `[serial, slices]` header with a zero-length payload). This is
        // not malformed input: `ZpDecoder` already treats reads past the end of
        // its buffer as synthetic `0xFF` padding (see `read_byte`), which is
        // exactly how a normal chunk's *trailing* padding is already decoded
        // (see the slice-loop comment below). Pad up to 2 bytes with `0xFF`
        // here so a short/empty payload takes that same, already-relied-upon
        // padding path through initialization too, instead of hard-erroring.
        let raw_zp_data = &data[payload_start..];
        let padded_zp_data;
        let zp_data: &[u8] = if raw_zp_data.len() >= 2 {
            raw_zp_data
        } else {
            padded_zp_data = [raw_zp_data.first().copied().unwrap_or(0xff), 0xff];
            &padded_zp_data
        };
        let mut zp = ZpDecoder::new(zp_data).map_err(|_| Iw44Error::ZpTooShort)?;

        for _ in 0..slices {
            self.cslice += 1;
            if let Some(ref mut y) = self.y {
                y.decode_slice(&mut zp);
            }
            if self.is_color && self.cslice > self.delay as usize {
                if let Some(ref mut cb) = self.cb {
                    cb.decode_slice(&mut zp);
                }
                if let Some(ref mut cr) = self.cr {
                    cr.decode_slice(&mut zp);
                }
            }
            // NOTE: do not early-exit on `zp.is_exhausted()` here. The ZP
            // coder is a continuous arithmetic bit stream and `is_exhausted()`
            // only reports that the *byte* buffer is drained — it fires several
            // bytes before the logical end of the stream (the decoder reads up
            // to 24 bits ahead via `refill_buffer`). The remaining slices still
            // decode legitimate wavelet refinement from the buffered bits and
            // arithmetic registers; skipping them truncates high-frequency
            // detail and produces chroma artifacts. The slice loop is already
            // bounded by `slices` (a u8, ≤255 per chunk) plus the 64 MP image
            // cap, so no early-exit is needed to bound decode time.
            // See PERF_EXPERIMENTS.md (#182 and the slice-loop follow-up).
        }

        self.next_serial = serial as u32 + 1;
        Ok(())
    }

    /// Convert the decoded image to an RGB [`Pixmap`].
    ///
    /// This is the **only** place where the separate Y, Cb, Cr planes are
    /// interleaved into RGB pixels.  DjVu images are stored bottom-to-top;
    /// this method flips the output to top-to-bottom.
    ///
    /// Equivalent to `to_rgb_subsample(1)`.
    pub fn to_rgb(&self) -> Result<Pixmap, Iw44Error> {
        self.to_rgb_subsample(1)
    }

    /// Convert to an RGB [`Pixmap`] at reduced resolution.
    ///
    /// `subsample` must be ≥ 1.  A value of 1 gives full resolution; 2 gives
    /// half resolution in each dimension, etc.
    pub fn to_rgb_subsample(&self, subsample: u32) -> Result<Pixmap, Iw44Error> {
        if subsample == 0 {
            return Err(Iw44Error::InvalidSubsample);
        }
        let y_dec = self.y.as_ref().ok_or(Iw44Error::MissingCodec)?;
        let sub = subsample as usize;
        let w = (self.width as usize).div_ceil(sub) as u32;
        let h = (self.height as usize).div_ceil(sub) as u32;

        if self.is_color {
            // With `chroma_half` a full-resolution picture takes its chroma
            // from the scale-2 reconstruction, one value per 2x2 block. At
            // `sub >= 2` DjVuLibre stops at scale `sub` anyway, so the flag
            // changes nothing there.
            let chroma_sub = if self.chroma_half && sub == 1 { 2 } else { sub };
            let cb_dec = self.cb.as_ref().ok_or(Iw44Error::MissingCodec)?;
            let cr_dec = self.cr.as_ref().ok_or(Iw44Error::MissingCodec)?;

            let pw = w as usize;
            let ph = h as usize;

            // Fast path: sub=1 (most common — full-resolution render).
            // Pre-normalize Y/Cb/Cr into flat row buffers and apply the
            // YCbCr→RGBA formula 8 pixels at a time with SIMD.
            if sub == 1 {
                let mut pm = Pixmap::try_new(w, h, 0, 0, 0, 255)?;
                // A very large page is reconstructed a band at a time. The three
                // full-resolution `i16` planes cost 6 bytes per pixel — more
                // than the 4-byte output they feed — and are dropped the moment
                // the RGB is written. See `band_keep_blocks`.
                if let Some(keep) = band_keep_blocks(y_dec, self.chroma_half, 0) {
                    self.rgb_sub1_banded(y_dec, cb_dec, cr_dec, keep, pw, ph, &mut pm);
                    return Ok(pm);
                }
                let (y_plane, cb_plane, cr_plane) =
                    reconstruct_planes(y_dec, cb_dec, cr_dec, sub, chroma_sub);
                convert_rgb_rows(
                    self.chroma_half,
                    &y_plane,
                    0,
                    &cb_plane,
                    &cr_plane,
                    0,
                    (0, 0),
                    0..ph,
                    0..pw,
                    pw,
                    ph,
                    &mut pm.data,
                );
                return Ok(pm);
            }

            let (y_plane, cb_plane, cr_plane) =
                reconstruct_planes(y_dec, cb_dec, cr_dec, sub, chroma_sub);
            let mut pm = Pixmap::try_new(w, h, 0, 0, 0, 255)?;

            // Compact path: sub ≥ 2 with power-of-two subsample.
            //
            // `reconstruct(sub)` now returns a plane that is already at the
            // target resolution (ceil(w/sub) × ceil(h/sub)), so we access it
            // with sub=1 indexing.  Chroma planes are at the same output size.
            //
            // Uses SIMD via `ycbcr_row_to_rgba` (same as the sub=1 fast path).
            if (2..=8).contains(&sub) && sub.is_power_of_two() {
                for row in 0..ph {
                    let out_row = ph - 1 - row; // DjVu rows are bottom-to-top
                    let y_off = row * y_plane.stride;
                    let c_off = row * cb_plane.stride;
                    let row_start = out_row * pw * 4;
                    ycbcr_row_from_i16(
                        &y_plane.data[y_off..y_off + pw],
                        &cb_plane.data[c_off..c_off + pw],
                        &cr_plane.data[c_off..c_off + pw],
                        &mut pm.data[row_start..row_start + pw * 4],
                    );
                }
                return Ok(pm);
            }

            // Fallback scalar path for non-power-of-two or large sub values.
            for row in 0..h {
                let out_row = h - 1 - row;
                for col in 0..w {
                    let src_row = row as usize * sub;
                    let src_col = col as usize * sub;
                    let y_idx = src_row * y_plane.stride + src_col;
                    let c_idx = src_row * cb_plane.stride + src_col;

                    let y = normalize(y_plane.data[y_idx]);
                    let b = normalize(cb_plane.data[c_idx]);
                    let r = normalize(cr_plane.data[c_idx]);

                    let t2 = r + (r >> 1);
                    let t3 = y + 128 - (b >> 2);

                    let red = (y + 128 + t2).clamp(0, 255) as u8;
                    let green = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
                    let blue = (t3 + (b << 1)).clamp(0, 255) as u8;
                    pm.set_rgb(col, out_row, red, green, blue);
                }
            }
            Ok(pm)
        } else {
            // Grayscale: only the Y plane is needed.
            // For sub≥2 the plane is compact (at output resolution); for sub=1 it
            // is full-resolution.  Use compact-aware indexing.
            let y_plane = y_dec.reconstruct(sub);
            let is_compact = (2..=8).contains(&sub) && sub.is_power_of_two();
            let mut pm = Pixmap::try_new(w, h, 0, 0, 0, 255)?;
            for row in 0..h {
                let out_row = h - 1 - row;
                for col in 0..w {
                    let (src_row, src_col) = if is_compact {
                        (row as usize, col as usize)
                    } else {
                        (row as usize * sub, col as usize * sub)
                    };
                    let idx = src_row * y_plane.stride + src_col;
                    let val = normalize(y_plane.data[idx]);
                    // Grayscale: DjVu luma 0 maps to black, −128 → white
                    let gray = (127 - val) as u8;
                    pm.set_rgb(col, out_row, gray, gray, gray);
                }
            }
            Ok(pm)
        }
    }

    /// Convert a full-resolution colour page into `pm`, one band of block rows
    /// at a time, so the `i16` planes never exist whole.
    ///
    /// `keep` is how many block rows each band contributes to the output; each
    /// band reconstructs [`BAND_HALO_BLOCKS`] more on each side and throws them
    /// away, because only the interior of a band is exact.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn rgb_sub1_banded(
        &self,
        y_dec: &PlaneDecoder,
        cb_dec: &PlaneDecoder,
        cr_dec: &PlaneDecoder,
        keep: usize,
        pw: usize,
        ph: usize,
        pm: &mut Pixmap,
    ) {
        let block_rows = (self.height as usize).div_ceil(32);
        let mut first = 0usize;
        while first < block_rows {
            let last = (first + keep).min(block_rows);
            let (r0, r1) = (first * 32, (last * 32).min(ph));
            if r0 >= r1 {
                break;
            }
            let out = &mut pm.data[(ph - r1) * pw * 4..(ph - r0) * pw * 4];
            self.rgb_sub1_band(y_dec, cb_dec, cr_dec, r0, r1, 0..pw, pw, ph, out);
            first = last;
        }
    }

    /// Write image rows `r0..r1` of the full-resolution colour picture into
    /// `out`, reconstructing only the block rows that cover them plus a halo.
    ///
    /// `out` holds exactly those rows as RGBA, top to bottom (see
    /// [`convert_rgb_rows`]). The band may start on any row: the halo below
    /// and above is what makes its rows exact, wherever it starts.
    ///
    /// Only the block columns covering `cols`, plus the same halo, are
    /// reconstructed, and only those columns of `out` are written: every
    /// column of a block that `cols` touches, so at least `cols`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn rgb_sub1_band(
        &self,
        y_dec: &PlaneDecoder,
        cb_dec: &PlaneDecoder,
        cr_dec: &PlaneDecoder,
        r0: usize,
        r1: usize,
        cols: core::ops::Range<usize>,
        pw: usize,
        ph: usize,
        out: &mut [u8],
    ) {
        let block_rows = (self.height as usize).div_ceil(32);
        let (first, last) = (r0 / 32, r1.div_ceil(32));
        let lo = first.saturating_sub(BAND_HALO_BLOCKS);
        let hi = (last + BAND_HALO_BLOCKS).min(block_rows);

        // The same for columns. The written columns are whole blocks, so
        // they start on an even column, as `chroma_half` needs.
        let block_cols = pw.div_ceil(32);
        let (first_col, last_col) = (cols.start / 32, cols.end.div_ceil(32));
        let col_lo = first_col.saturating_sub(BAND_HALO_BLOCKS);
        let col_hi = (last_col + BAND_HALO_BLOCKS).min(block_cols);
        let written = first_col * 32..(last_col * 32).min(pw);

        // Chroma rows this band reads. With `chroma_half` they are rows of
        // the compact scale-2 plane, where image row `r` reads row `r / 2`
        // and a block row holds 16 rows. The halo is the same number of block
        // rows: the compact transform reaches half as far.
        let c_sub = if self.chroma_half { 2 } else { 1 };
        let c_side = 32 / c_sub;
        let (c_r0, c_r1) = (r0 / c_sub, r1.div_ceil(c_sub));
        let c_lo = (c_r0 / c_side).saturating_sub(BAND_HALO_BLOCKS);
        let c_hi = (c_r1.div_ceil(c_side) + BAND_HALO_BLOCKS).min(block_rows);
        let (c_x0, c_x1) = (written.start / c_sub, written.end.div_ceil(c_sub));
        let c_col_lo = (c_x0 / c_side).saturating_sub(BAND_HALO_BLOCKS);
        let c_col_hi = (c_x1.div_ceil(c_side) + BAND_HALO_BLOCKS).min(block_cols);
        let y_cols = (col_lo, col_hi);
        let c_cols = (c_col_lo, c_col_hi);

        #[cfg(feature = "parallel")]
        let (y_band, cb_band, cr_band) = {
            let (y, (cb, cr)) = rayon::join(
                || y_dec.reconstruct_window((lo, hi), y_cols, 1),
                || {
                    rayon::join(
                        || cb_dec.reconstruct_window((c_lo, c_hi), c_cols, c_sub),
                        || cr_dec.reconstruct_window((c_lo, c_hi), c_cols, c_sub),
                    )
                },
            );
            (y, cb, cr)
        };
        #[cfg(not(feature = "parallel"))]
        let (y_band, cb_band, cr_band) = (
            y_dec.reconstruct_window((lo, hi), y_cols, 1),
            cb_dec.reconstruct_window((c_lo, c_hi), c_cols, c_sub),
            cr_dec.reconstruct_window((c_lo, c_hi), c_cols, c_sub),
        );

        convert_rgb_rows(
            self.chroma_half,
            &y_band,
            lo * 32,
            &cb_band,
            &cr_band,
            c_lo * c_side,
            (col_lo * 32, c_col_lo * c_side),
            r0..r1,
            written,
            pw,
            ph,
            out,
        );
    }

    /// How many rows a caller that composites straight from bands of
    /// [`rgb_rows`](Self::rgb_rows) should take at a time, or `None` when the
    /// picture is small enough to convert whole with [`to_rgb`](Self::to_rgb).
    ///
    /// `Some` only for a colour picture whose full-resolution planes are large
    /// enough that `to_rgb` itself reconstructs them in bands. Such a caller
    /// never holds the whole RGB picture, so its band pays for its own RGB
    /// rows out of the same budget and keeps fewer rows than `to_rgb` does.
    pub fn rgb_band_rows(&self) -> Option<u32> {
        if !self.is_color {
            return None;
        }
        let y_dec = self.y.as_ref()?;
        band_keep_blocks(y_dec, self.chroma_half, 4).map(|keep| (keep * 32) as u32)
    }

    /// Rows `rows` of the full-resolution colour picture, top to bottom, as a
    /// pixmap of `self.width` by `rows.len()`.
    ///
    /// Byte-identical to the same rows of [`to_rgb`](Self::to_rgb), but only
    /// the block rows covering `rows` (plus a halo on each side) are
    /// reconstructed, so a caller that walks the picture in bands never holds
    /// more than one band of planes and one band of RGB. See
    /// [`rgb_band_rows`](Self::rgb_band_rows) for the band size that keeps
    /// within the reconstruction budget.
    ///
    /// # Errors
    ///
    /// [`Iw44Error::MissingCodec`] when the picture is not colour or has no
    /// planes yet; [`Iw44Error::Invalid`] when `rows` is not within the
    /// picture's height.
    pub fn rgb_rows(&self, rows: core::ops::Range<u32>) -> Result<Pixmap, Iw44Error> {
        self.rgb_window(rows, 0..self.width)
    }

    /// Rows `rows` of the full-resolution colour picture, like
    /// [`rgb_rows`](Self::rgb_rows), but with only columns `cols` decoded.
    ///
    /// The pixmap is still `self.width` wide, so a pixel keeps its column.
    /// Columns `cols` hold the picture, byte-identical to
    /// [`to_rgb`](Self::to_rgb). Every other column is either the picture
    /// too or all zeros, alpha included. Only the block columns covering
    /// `cols`, plus a halo, are reconstructed, so a narrow window of a wide
    /// picture costs a fraction of its full rows. The zero columns are
    /// allocated but never written.
    ///
    /// # Errors
    ///
    /// As [`rgb_rows`](Self::rgb_rows); also [`Iw44Error::Invalid`] when
    /// `cols` is not within the picture's width.
    pub fn rgb_window(
        &self,
        rows: core::ops::Range<u32>,
        cols: core::ops::Range<u32>,
    ) -> Result<Pixmap, Iw44Error> {
        if !self.is_color {
            return Err(Iw44Error::MissingCodec);
        }
        let y_dec = self.y.as_ref().ok_or(Iw44Error::MissingCodec)?;
        let cb_dec = self.cb.as_ref().ok_or(Iw44Error::MissingCodec)?;
        let cr_dec = self.cr.as_ref().ok_or(Iw44Error::MissingCodec)?;
        let (pw, ph) = (self.width as usize, self.height as usize);
        let (o0, o1) = (rows.start as usize, rows.end as usize);
        let (x0, x1) = (cols.start as usize, cols.end as usize);
        if o0 > o1 || o1 > ph || x0 > x1 || x1 > pw {
            return Err(Iw44Error::Invalid);
        }
        let mut pm = Pixmap::try_new(self.width, (o1 - o0) as u32, 0, 0, 0, 0)?;
        if o0 == o1 || x0 == x1 {
            return Ok(pm);
        }
        // Output row `o` is image row `ph - 1 - o`, so output rows `o0..o1`
        // are image rows `ph - o1..ph - o0`.
        self.rgb_sub1_band(
            y_dec,
            cb_dec,
            cr_dec,
            ph - o1,
            ph - o0,
            x0..x1,
            pw,
            ph,
            &mut pm.data,
        );
        Ok(pm)
    }

    /// Convert to a grayscale [`GrayPixmap`] at full resolution.
    ///
    /// See [`to_gray8_subsample`](Self::to_gray8_subsample).
    pub fn to_gray8(&self) -> Result<GrayPixmap, Iw44Error> {
        self.to_gray8_subsample(1)
    }

    /// Convert to a grayscale [`GrayPixmap`], decoding **only the Y (luma)
    /// plane** and skipping both chroma planes entirely.
    ///
    /// For a colour image the two chroma inverse-wavelet transforms and the
    /// YCbCr→RGBA conversion are the bulk of `to_rgb`'s cost; a grayscale
    /// consumer (OCR pre-pass, e-ink viewer, thumbnail grid, `render_gray8`)
    /// never needs them. This path reconstructs Y alone and writes one byte per
    /// pixel.
    ///
    /// # Fidelity
    ///
    /// - **Grayscale images:** byte-identical to `to_rgb_subsample(sub).to_gray8()`
    ///   (the R=G=B channels already equal `127 − Y`, and the Rec.601 weights
    ///   sum to 1024, so the luma round-trips exactly).
    /// - **Colour images:** returns the DjVu luma channel `clamp(Y + 128, 0,
    ///   255)`. This is the encoder's own luminance and is *not* bit-identical
    ///   to the Rec.601 luma of the reconstructed RGB (`to_gray8`), which mixes
    ///   in the chroma-derived R/G/B. The two differ by a few levels at most;
    ///   the Y channel is the more faithful luminance.
    pub fn to_gray8_subsample(&self, subsample: u32) -> Result<GrayPixmap, Iw44Error> {
        if subsample == 0 {
            return Err(Iw44Error::InvalidSubsample);
        }
        let y_dec = self.y.as_ref().ok_or(Iw44Error::MissingCodec)?;
        let sub = subsample as usize;
        let w = (self.width as usize).div_ceil(sub) as u32;
        let h = (self.height as usize).div_ceil(sub) as u32;

        // Reconstruct Y only — never touch cb/cr (their reconstruct() + the
        // YCbCr math are what this path exists to skip).
        let y_plane = y_dec.reconstruct(sub);
        let is_compact = (2..=8).contains(&sub) && sub.is_power_of_two();
        let is_color = self.is_color;

        let pw = w as usize;
        let ph = h as usize;
        let mut data = vec![0u8; pw * ph];
        for row in 0..ph {
            let out_row = ph - 1 - row; // DjVu rows are bottom-to-top
            let src_row = if is_compact { row } else { row * sub };
            let y_off = src_row * y_plane.stride;
            let dst = &mut data[out_row * pw..out_row * pw + pw];
            if is_color {
                // DjVu luma: gray = clamp(Y + 128, 0, 255).
                for (col, d) in dst.iter_mut().enumerate() {
                    let src_col = if is_compact { col } else { col * sub };
                    let val = normalize(y_plane.data[y_off + src_col]);
                    *d = (val + 128).clamp(0, 255) as u8;
                }
            } else {
                // Grayscale plane: gray = 127 − Y (matches to_rgb's R channel).
                for (col, d) in dst.iter_mut().enumerate() {
                    let src_col = if is_compact { col } else { col * sub };
                    let val = normalize(y_plane.data[y_off + src_col]);
                    *d = (127 - val) as u8;
                }
            }
        }
        Ok(GrayPixmap {
            width: w,
            height: h,
            data,
        })
    }
}
