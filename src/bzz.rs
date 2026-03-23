use crate::zp::ZPDecoder;

/// Errors that can occur during BZZ decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// The BWT block did not contain an end-of-block marker.
    MissingEndOfBlock,
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeError::MissingEndOfBlock => write!(f, "BZZ: missing end-of-block marker"),
        }
    }
}

impl std::error::Error for DecodeError {}

const FREQMAX: usize = 4;
const CTXIDS: usize = 3;
const NUM_CONTEXTS: usize = 300;

/// Decode a BZZ-compressed stream.
///
/// Returns the decompressed bytes.
pub fn decode(data: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let mut zp = ZPDecoder::new(data);
    let mut output = Vec::new();
    let mut ctx = [0u8; NUM_CONTEXTS];

    loop {
        let size = decode_raw(&mut zp, 24);
        if size == 0 {
            break;
        }
        let block = decode_block(&mut zp, &mut ctx, size as usize)?;
        output.extend_from_slice(&block);
    }

    Ok(output)
}

/// Decode a raw N-bit value from ZP (no context, passthrough).
fn decode_raw(zp: &mut ZPDecoder, bits: u32) -> u32 {
    let mut n: u32 = 1;
    let m: u32 = 1 << bits;
    while n < m {
        let b = zp.decode_passthrough() as u32;
        n = (n << 1) | b;
    }
    n - m
}

/// Decode an N-bit value from ZP using context tree.
/// Note: ctxoff is decremented by 1 before indexing (matching djvulibre convention).
fn decode_binary(zp: &mut ZPDecoder, ctx: &mut [u8], ctxoff: usize, bits: u32) -> u32 {
    let base = ctxoff - 1;
    let mut n: u32 = 1;
    let m: u32 = 1 << bits;
    while n < m {
        let b = zp.decode(&mut ctx[base + n as usize]) as u32;
        n = (n << 1) | b;
    }
    n - m
}

/// Decode a single BZZ block.
fn decode_block(
    zp: &mut ZPDecoder,
    ctx: &mut [u8; NUM_CONTEXTS],
    size: usize,
) -> Result<Vec<u8>, DecodeError> {
    // Decode frequency shift (0, 1, or 2)
    let mut fshift: u32 = 0;
    if zp.decode_passthrough() {
        fshift += 1;
        if zp.decode_passthrough() {
            fshift += 1;
        }
    }

    // Initialize per-block state
    let mut mtf: [u8; 256] = core::array::from_fn(|i| i as u8);
    let mut freq = [0u32; FREQMAX];
    let mut fadd: u32 = 4;
    let mut mtfno: u32 = 3;
    let mut markerpos: i32 = -1;

    // Decode MTF-encoded symbols
    let mut data = vec![0u8; size];

    for (i, data_byte) in data.iter_mut().enumerate() {
        let ctxid = (mtfno.min(CTXIDS as u32 - 1)) as usize;
        let mut ctxoff: usize;

        // Hierarchical MTF position decoding
        // Level 0: mtfno == 0?
        ctxoff = 0;
        if zp.decode(&mut ctx[ctxoff + ctxid]) {
            mtfno = 0;
            *data_byte = mtf[0];
        }
        // Level 1: mtfno == 1?
        else {
            ctxoff += CTXIDS;
            if zp.decode(&mut ctx[ctxoff + ctxid]) {
                mtfno = 1;
                *data_byte = mtf[1];
            }
            // Level 2: mtfno in {2, 3}?
            else {
                ctxoff += CTXIDS;
                if zp.decode(&mut ctx[ctxoff]) {
                    mtfno = 2 + decode_binary(zp, ctx, ctxoff + 1, 1);
                    *data_byte = mtf[mtfno as usize];
                }
                // Level 3: mtfno in {4..7}?
                else {
                    ctxoff += 2;
                    if zp.decode(&mut ctx[ctxoff]) {
                        mtfno = 4 + decode_binary(zp, ctx, ctxoff + 1, 2);
                        *data_byte = mtf[mtfno as usize];
                    }
                    // Level 4: mtfno in {8..15}?
                    else {
                        ctxoff += 4;
                        if zp.decode(&mut ctx[ctxoff]) {
                            mtfno = 8 + decode_binary(zp, ctx, ctxoff + 1, 3);
                            *data_byte = mtf[mtfno as usize];
                        }
                        // Level 5: mtfno in {16..31}?
                        else {
                            ctxoff += 8;
                            if zp.decode(&mut ctx[ctxoff]) {
                                mtfno = 16 + decode_binary(zp, ctx, ctxoff + 1, 4);
                                *data_byte = mtf[mtfno as usize];
                            }
                            // Level 6: mtfno in {32..63}?
                            else {
                                ctxoff += 16;
                                if zp.decode(&mut ctx[ctxoff]) {
                                    mtfno = 32 + decode_binary(zp, ctx, ctxoff + 1, 5);
                                    *data_byte = mtf[mtfno as usize];
                                }
                                // Level 7: mtfno in {64..127}?
                                else {
                                    ctxoff += 32;
                                    if zp.decode(&mut ctx[ctxoff]) {
                                        mtfno = 64 + decode_binary(zp, ctx, ctxoff + 1, 6);
                                        *data_byte = mtf[mtfno as usize];
                                    }
                                    // Level 8: mtfno in {128..255}?
                                    else {
                                        ctxoff += 64;
                                        if zp.decode(&mut ctx[ctxoff]) {
                                            mtfno = 128 + decode_binary(zp, ctx, ctxoff + 1, 7);
                                            *data_byte = mtf[mtfno as usize];
                                        }
                                        // Level 9: marker (mtfno == 256)
                                        else {
                                            mtfno = 256;
                                            *data_byte = 0;
                                            markerpos = i as i32;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // MTF update (move decoded symbol to front)
        if mtfno < 256 {
            let sym = *data_byte;
            // Update frequency tracking
            fadd = fadd.wrapping_add(fadd >> fshift);
            if fadd > 0x10000000 {
                fadd >>= 24;
                for f in freq.iter_mut() {
                    *f >>= 24;
                }
            }

            let mut fc = fadd;
            if (mtfno as usize) < FREQMAX {
                fc += freq[mtfno as usize];
            }

            // Bubble-sort MTF by frequency
            let mut k = mtfno as usize;
            while k >= FREQMAX {
                mtf[k] = mtf[k - 1];
                k -= 1;
            }
            while k > 0 && fc >= freq[k - 1] {
                mtf[k] = mtf[k - 1];
                freq[k] = freq[k - 1];
                k -= 1;
            }
            mtf[k] = sym;
            freq[k] = fc;
        }
    }

    if markerpos < 0 {
        return Err(DecodeError::MissingEndOfBlock);
    }

    // Inverse Burrows-Wheeler Transform
    inverse_bwt(&mut data, markerpos as usize)
}

/// Inverse Burrows-Wheeler Transform.
///
/// `data` contains the BWT-transformed bytes with a marker at `markerpos`.
/// Returns the original uncompressed data (excluding the marker byte).
fn inverse_bwt(data: &mut [u8], markerpos: usize) -> Result<Vec<u8>, DecodeError> {
    let size = data.len();
    if size == 0 {
        return Ok(Vec::new());
    }

    // Build position array: encode (byte_value << 24) | count
    let mut count = [0u32; 256];
    let mut pos = vec![0u32; size];

    for i in 0..size {
        if i == markerpos {
            continue;
        }
        let c = data[i] as usize;
        pos[i] = ((c as u32) << 24) | (count[c] & 0xffffff);
        count[c] += 1;
    }

    // Compute cumulative counts (start positions in sorted order)
    // Position 0 is reserved for the marker
    let mut last: u32 = 1;
    for item in &mut count {
        let tmp = *item;
        *item = last;
        last += tmp;
    }

    // Reconstruct original data by following suffix chain
    let output_size = size - 1;
    let mut output = vec![0u8; output_size];
    let mut j: usize = 0;
    let mut last_idx = output_size;

    while last_idx > 0 {
        let n = pos[j];
        let c = (n >> 24) as u8;
        last_idx -= 1;
        output[last_idx] = c;
        j = (count[c as usize] + (n & 0xffffff)) as usize;
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn golden_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/bzz")
    }

    fn test_bzz_roundtrip(name: &str) {
        let bzz_data = std::fs::read(golden_path().join(format!("{}.bzz", name))).unwrap();
        let expected = std::fs::read(golden_path().join(format!("{}.txt", name))).unwrap();
        let decoded = decode(&bzz_data).unwrap();
        assert_eq!(
            decoded,
            expected,
            "BZZ decode mismatch for {}. Got {} bytes, expected {} bytes",
            name,
            decoded.len(),
            expected.len()
        );
    }

    #[test]
    fn bzz_decode_short() {
        test_bzz_roundtrip("test_short");
    }

    #[test]
    fn bzz_decode_long() {
        test_bzz_roundtrip("test_long");
    }

    #[test]
    fn bzz_decode_1byte() {
        test_bzz_roundtrip("test_1byte");
    }

    #[test]
    fn bzz_decode_real_dirm() {
        // BZZ payload extracted from DIRM chunk of navm_fgbz.djvu
        let bzz_data = std::fs::read(golden_path().join("navm_fgbz_dirm.bzz")).unwrap();
        let expected = std::fs::read(golden_path().join("navm_fgbz_dirm.bin")).unwrap();
        let decoded = decode(&bzz_data).unwrap();
        assert_eq!(decoded, expected);
    }

    // --- Phase 6.2: Edge case tests ---

    #[test]
    fn bzz_empty_input() {
        let result = decode(&[]);
        assert!(result.is_err() || result.unwrap().is_empty());
    }

    #[test]
    fn bzz_single_byte() {
        // Should not panic on minimal input
        let _ = decode(&[0x00]);
    }

    #[test]
    fn bzz_all_zeros() {
        let _ = decode(&[0u8; 32]);
    }

    #[test]
    fn bzz_all_ones() {
        let _ = decode(&[0xffu8; 32]);
    }

    #[test]
    fn bzz_truncated_header() {
        // Two bytes — not enough for block size
        let _ = decode(&[0x42, 0x5a]);
    }
}
