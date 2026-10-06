//! The public encoder: colour and gray entry points, chunk assembly.

use super::*;

/// Convert one RGB pixel to IW44 YCbCr.
///
/// Uses DjVuLibre's fixed-point Pigeon transform:
/// ```text
/// Y  = ( 7·r + 14·g +  2·b) / 23 - 128
/// Cb = (-4·r -  8·g + 12·b) / 23
/// Cr = (32·r - 28·g -  4·b) / 69
/// ```
///
/// These components are the inverse-domain of the decoder's Pigeon
/// `YCbCr_to_RGB` transform.  Simple `B-G` / `R-G` differences are not: they
/// introduce a large colour error even before wavelet quantization.
#[inline(always)]
pub(super) fn rgb_to_ycbcr(r: u8, g: u8, b: u8) -> (i16, i16, i16) {
    let r = r as i32;
    let g = g as i32;
    let b = b as i32;
    // These are `int(component * 65536)` from DjVuLibre's `rgb_to_ycc`
    // float table.  Keep the same rounding (`+32768 >> 16`) and clamping as
    // `IW44Image::Transform::Encode::{RGB_to_Y,RGB_to_Cb,RGB_to_Cr}` so an
    // IW44 stream produced here has the same colour coordinate system as c44.
    let y = ((19_945 * r + 39_891 * g + 5_698 * b + 32_768) >> 16) - 128;
    let cb = (-11_397 * r - 22_795 * g + 34_192 * b + 32_768) >> 16;
    let cr = (30_393 * r - 26_594 * g - 3_799 * b + 32_768) >> 16;
    (
        y.clamp(-128, 127) as i16,
        cb.clamp(-128, 127) as i16,
        cr.clamp(-128, 127) as i16,
    )
}

/// Convert one row of RGBA pixels into the Y, Cb and Cr plane rows, each
/// scaled by 64 (`normalize()` divides by 64 on decode).
///
/// Writes the first `src.len() / 4` entries of each row. Indexing the
/// pixels directly, instead of `Pixmap::get_rgb` per pixel, lets the loop
/// vectorise.
#[inline]
pub(super) fn ycbcr_row(src: &[u8], y_row: &mut [i16], cb_row: &mut [i16], cr_row: &mut [i16]) {
    let px = src.as_chunks::<4>().0;
    let n = px.len();
    let (y_row, cb_row, cr_row) = (&mut y_row[..n], &mut cb_row[..n], &mut cr_row[..n]);
    for i in 0..n {
        let (y, cb, cr) = rgb_to_ycbcr(px[i][0], px[i][1], px[i][2]);
        y_row[i] = y * 64;
        cb_row[i] = cb * 64;
        cr_row[i] = cr * 64;
    }
}

#[cfg(feature = "std")]
/// Encode a color [`Pixmap`] into BG44 chunk payloads (one `Vec<u8>` per chunk).
///
/// Wrap each chunk in a `BG44` IFF chunk tag before embedding in a DjVu file.
pub fn encode_iw44_color(pixmap: &Pixmap, opts: &Iw44EncodeOptions) -> Vec<Vec<u8>> {
    let w = pixmap.width as usize;
    let h = pixmap.height as usize;
    let stride = w.div_ceil(32) * 32;
    let plane_h = h.div_ceil(32) * 32;

    // A page whose three planes would cost more than the grid is worth holding
    // beside it transforms one band at a time and never allocates them whole.
    // Every other page takes the path below, unchanged.
    if let Some(keep) = encode_band_keep_blocks(stride, plane_h / 32, 3) {
        let mut encs = [
            PlaneEncoder::new(w, h),
            PlaneEncoder::new(w, h),
            PlaneEncoder::new(w, h),
        ];
        forward_gather_banded(
            &mut encs,
            w,
            h,
            stride,
            keep,
            crate::BAND_HALO_BLOCKS,
            |band, bufs| fill_color_band(pixmap, band, stride, bufs),
        );
        let [mut y_enc, mut cb_enc, mut cr_enc] = encs;
        return encode_chunks(
            &mut y_enc,
            Some(&mut cb_enc),
            Some(&mut cr_enc),
            w as u16,
            h as u16,
            true,
            opts,
        );
    }

    // Each plane is the encoder's block storage, flattened: the gather turns
    // it into the coefficient grid in place (`PlaneEncoder::from_plane`).
    let mut y_plane = PlaneEncoder::new_plane(w, h);

    // `chroma_half` remains in the public options for source compatibility,
    // but its old half-plane encoding was not a valid IW44 v1.2 stream: both
    // DjVuLibre and our corrected decoder consume full-resolution Cb/Cr.
    let chroma_half = false;
    let (cw, ch) = if chroma_half {
        (w.div_ceil(2), h.div_ceil(2))
    } else {
        (w, h)
    };
    let c_stride = cw.div_ceil(32) * 32;
    let c_plane_h = ch.div_ceil(32) * 32;
    let mut cb_plane = PlaneEncoder::new_plane(cw, ch);
    let mut cr_plane = PlaneEncoder::new_plane(cw, ch);
    debug_assert_eq!(y_plane.len() * 1024, stride * plane_h);
    debug_assert_eq!(cb_plane.len() * 1024, c_stride * c_plane_h);

    // DjVu stores images bottom-to-top: wavelet row 0 = image bottom row.
    // The decoder's to_rgb flips via out_row = h-1-row, so mirror that here.
    // Scale by 64 because normalize() divides by 64 on decode.
    //
    // Single pass: compute Y for every pixel; if chroma_half, accumulate 2×2
    // box-filter for Cb/Cr (matches DjVuLibre's chroma downsampling).
    let (y_flat, cb_flat, cr_flat) = (
        y_plane.as_flattened_mut(),
        cb_plane.as_flattened_mut(),
        cr_plane.as_flattened_mut(),
    );
    if chroma_half {
        // Single pass: fill Y and accumulate 2×2 box-filter chroma (matches
        // DjVuLibre's c44 downsampling).  Each chroma output cell receives
        // contributions from up to 4 source pixels with weight 16 each, so
        // a full 2×2 block sums to 64 — the same scale used by the 1:1 path.
        for row in 0..h {
            let wavelet_row = h - 1 - row;
            for col in 0..w {
                let (r, g, b) = pixmap.get_rgb(col as u32, row as u32);
                let (y, cb, cr) = rgb_to_ycbcr(r, g, b);
                y_flat[wavelet_row * stride + col] = (y as i32 * 64) as i16;
                let cc = col / 2;
                let cr_row = wavelet_row / 2;
                cb_flat[cr_row * c_stride + cc] += (cb as i32 * 16) as i16;
                cr_flat[cr_row * c_stride + cc] += (cr as i32 * 16) as i16;
            }
        }
    } else {
        for row in 0..h {
            let off = (h - 1 - row) * stride;
            let c_off = (h - 1 - row) * c_stride;
            ycbcr_row(
                &pixmap.data[row * w * 4..(row + 1) * w * 4],
                &mut y_flat[off..off + w],
                &mut cb_flat[c_off..c_off + w],
                &mut cr_flat[c_off..c_off + w],
            );
        }
    }

    // Transform + gather all three planes.  Each plane is independent, so with
    // the `parallel` feature they run concurrently on rayon threads, reducing
    // wall-time from Y+Cb+Cr sequential to max(Y, Cb, Cr).
    //
    // The threshold (512×512 = 262 144 px) ensures rayon overhead (~30 µs) is
    // only paid when the work per plane is large enough to justify it.  Below
    // that threshold sequential is faster (verified on M1 with 192×256 images).
    #[cfg(feature = "parallel")]
    let (mut y_enc, mut cb_enc, mut cr_enc) = if w * h > 512 * 512 {
        use rayon::join;
        let (ye, (cbe, cre)) = join(
            move || {
                forward_wavelet_transform(y_plane.as_flattened_mut(), w, h, stride);
                PlaneEncoder::from_plane(w, h, y_plane)
            },
            move || {
                join(
                    move || {
                        forward_wavelet_transform(cb_plane.as_flattened_mut(), cw, ch, c_stride);
                        PlaneEncoder::from_plane(cw, ch, cb_plane)
                    },
                    move || {
                        forward_wavelet_transform(cr_plane.as_flattened_mut(), cw, ch, c_stride);
                        PlaneEncoder::from_plane(cw, ch, cr_plane)
                    },
                )
            },
        );
        (ye, cbe, cre)
    } else {
        forward_wavelet_transform(y_plane.as_flattened_mut(), w, h, stride);
        forward_wavelet_transform(cb_plane.as_flattened_mut(), cw, ch, c_stride);
        forward_wavelet_transform(cr_plane.as_flattened_mut(), cw, ch, c_stride);
        (
            PlaneEncoder::from_plane(w, h, y_plane),
            PlaneEncoder::from_plane(cw, ch, cb_plane),
            PlaneEncoder::from_plane(cw, ch, cr_plane),
        )
    };
    #[cfg(not(feature = "parallel"))]
    let (mut y_enc, mut cb_enc, mut cr_enc) = {
        forward_wavelet_transform(y_plane.as_flattened_mut(), w, h, stride);
        forward_wavelet_transform(cb_plane.as_flattened_mut(), cw, ch, c_stride);
        forward_wavelet_transform(cr_plane.as_flattened_mut(), cw, ch, c_stride);
        (
            PlaneEncoder::from_plane(w, h, y_plane),
            PlaneEncoder::from_plane(cw, ch, cb_plane),
            PlaneEncoder::from_plane(cw, ch, cr_plane),
        )
    };

    encode_chunks(
        &mut y_enc,
        Some(&mut cb_enc),
        Some(&mut cr_enc),
        w as u16,
        h as u16,
        true,
        opts,
    )
}

#[cfg(feature = "std")]
/// Encode a grayscale [`GrayPixmap`] into BG44/FG44 chunk payloads.
pub fn encode_iw44_gray(pixmap: &GrayPixmap, opts: &Iw44EncodeOptions) -> Vec<Vec<u8>> {
    let w = pixmap.width as usize;
    let h = pixmap.height as usize;
    let stride = w.div_ceil(32) * 32;
    let plane_h = h.div_ceil(32) * 32;

    // As in `encode_iw44_color`: a plane too large to hold beside its grid is
    // transformed in bands.
    if let Some(keep) = encode_band_keep_blocks(stride, plane_h / 32, 1) {
        let mut encs = [PlaneEncoder::new(w, h)];
        forward_gather_banded(
            &mut encs,
            w,
            h,
            stride,
            keep,
            crate::BAND_HALO_BLOCKS,
            |band, bufs| fill_gray_band(pixmap, band, stride, bufs),
        );
        let [mut y_enc] = encs;
        return encode_chunks(&mut y_enc, None, None, w as u16, h as u16, false, opts);
    }

    let mut y_plane = PlaneEncoder::new_plane(w, h);
    let y_flat = y_plane.as_flattened_mut();

    // DjVu stores images bottom-to-top: wavelet row 0 = image bottom row.
    // The decoder's to_rgb flips via out_row = h-1-row, so we must mirror that.
    // The grayscale formula: coeff = (127 - p) * 64 (decoder gives gray = 127 - normalize(coeff)).
    for row in 0..h {
        let wavelet_row = h - 1 - row;
        for col in 0..w {
            let p = pixmap.get(col as u32, row as u32) as i32;
            y_flat[wavelet_row * stride + col] = ((127 - p) * 64) as i16;
        }
    }

    forward_wavelet_transform(y_flat, w, h, stride);

    let mut y_enc = PlaneEncoder::from_plane(w, h, y_plane);

    encode_chunks(&mut y_enc, None, None, w as u16, h as u16, false, opts)
}

#[cfg(feature = "std")]
pub(super) fn encode_chunks(
    y_enc: &mut PlaneEncoder,
    mut cb_enc: Option<&mut PlaneEncoder>,
    mut cr_enc: Option<&mut PlaneEncoder>,
    width: u16,
    height: u16,
    is_color: bool,
    opts: &Iw44EncodeOptions,
) -> Vec<Vec<u8>> {
    let slices_per_chunk = opts.slices_per_chunk.max(1) as usize;
    let total = opts.total_slices as usize;
    let delay = opts.chroma_delay as usize;

    // Compute byte budget for Bpp target.  `None` means no budget limit.
    // Budget = bpp * width * height / 8, clamped to at least 1 byte so we
    // always emit at least one slice.
    let byte_budget: Option<usize> = match opts.target {
        Iw44Target::Slices => None,
        Iw44Target::Bpp(bpp) => {
            let pixels = width as f64 * height as f64;
            // clamp bpp > 0; if ≤ 0 budget = 0 which still emits 1 slice
            let budget = if bpp > 0.0 {
                (bpp as f64 * pixels / 8.0).ceil() as usize
            } else {
                0
            };
            Some(budget)
        }
    };

    let mut chunks: Vec<Vec<u8>> = Vec::new();
    let mut slice_idx = 0usize;
    let mut serial: u8 = 0;
    let mut cslice = 0usize;
    // Running count of payload bytes emitted (chunk headers + zp data).
    let mut bytes_emitted: usize = 0;

    while slice_idx < total {
        let n = slices_per_chunk.min(total - slice_idx);
        let mut zp = ZpEncoder::new();

        for _ in 0..n {
            cslice += 1;
            y_enc.encode_slice(&mut zp);
            if is_color && cslice > delay {
                if let Some(cb) = cb_enc.as_deref_mut() {
                    cb.encode_slice(&mut zp);
                }
                if let Some(cr) = cr_enc.as_deref_mut() {
                    cr.encode_slice(&mut zp);
                }
            }
            slice_idx += 1;
            if slice_idx >= total {
                break;
            }
        }

        let mut zp_bytes = zp.finish();
        // Pad with 0xFF bytes to prevent the decoder's is_exhausted() guard from
        // firing before all `n` slices in this chunk are processed.  The ZP
        // decoder reads 2 bytes during construction plus 4 more in refill_buffer
        // (6 total), so `pos = 2` (= min(6, data.len())) after init.  If
        // data.len() == 2, pos == data.len() → is_exhausted() is immediately
        // true, the loop breaks after the first slice, and curband gets out of
        // sync.  Appending 0xFF bytes is safe: read_byte() already returns 0xFF
        // beyond the real data, so the decoded bit-stream is unchanged.
        let min_zp_len = n + 4; // enough for init + one refill per slice
        while zp_bytes.len() < min_zp_len {
            zp_bytes.push(0xFF);
        }
        let mut chunk = Vec::new();

        if serial == 0 {
            chunk.push(0u8); // serial
            chunk.push(n as u8); // slices
            // Major version byte. DjVuLibre's IWPixmap::decode_chunk rejects the
            // stream with "incompatible IWCodec" unless `(major & 0x7f) == 1`
            // (IWCODEC_MAJOR). We previously emitted 0 → our colour output was
            // unreadable by ddjvu/DjVuLibre. Bit 7 carries our grayscale flag (the
            // decoder reads `is_grayscale = major >> 7`); real DjVuLibre colour BG44
            // chunks use 0x01, so colour = 0x01, grayscale = 0x81.
            let majver: u8 = if is_color { 0x01 } else { 0x81 };
            chunk.push(majver);
            chunk.push(0x02); // minor = 2
            chunk.push((width >> 8) as u8);
            chunk.push(width as u8);
            chunk.push((height >> 8) as u8);
            chunk.push(height as u8);
            // delay_byte: bits 0-6 = chroma_delay, bit 7 advertises the
            // full-resolution chroma planes required by IW44 v1.2.
            let delay_byte = (opts.chroma_delay & 0x7F) | if is_color { 0x80 } else { 0x00 };
            chunk.push(delay_byte);
        } else {
            chunk.push(serial);
            chunk.push(n as u8);
        }
        chunk.extend_from_slice(&zp_bytes);
        bytes_emitted += chunk.len();
        chunks.push(chunk);
        serial = serial.wrapping_add(1);

        // Bpp budget check: stop after emitting at least one chunk (serial > 0
        // now since we just incremented it conceptually — serial was 0 for the
        // first chunk, so chunks.len() >= 1 ensures we've emitted at least one).
        if let Some(budget) = byte_budget
            && bytes_emitted >= budget
        {
            break;
        }
    }
    chunks
}
