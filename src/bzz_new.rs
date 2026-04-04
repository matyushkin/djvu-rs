//! BZZ decompressor — pure-Rust clean-room implementation.
//!
//! BZZ is the compression algorithm used in DjVu for directory and annotation
//! chunks (DIRM, NAVM, ANTz, etc.). It combines:
//! 1. ZP adaptive arithmetic coding (decoded first)
//! 2. Move-To-Front (MTF) inverse transform
//! 3. Burrows-Wheeler Transform (BWT) inverse transform
//!
//! This implementation is written from the DjVu v3 specification at
//! <https://www.sndjvu.org/spec.html> and does NOT derive from the legacy GPL code.
//!
//! Key public types:
//! - `bzz_decode` — decode a BZZ-compressed byte slice into a `Vec<u8>`

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

use crate::error::BzzError;
use crate::zp_impl::ZpDecoder;

/// Number of ZP contexts per BZZ block.
///
/// The MTF position hierarchy uses 262 contexts per block:
///
/// - 3 contexts for level 0 (MTF position 0)
/// - 3 contexts for level 1 (MTF position 1)
/// - 1 context + 1 sub-tree for level 2 (positions 2–3)
/// - 1 context + 3 sub-tree for level 3 (positions 4–7)
/// - ... and so on up to level 8
///
/// Total: 3 + 3 + 2 + 4 + 8 + 16 + 32 + 64 + 128 + 1 = 262.
/// We allocate 300 for headroom.
const CTX_COUNT: usize = 300;

/// Number of "frequency" slots tracked in the MTF order.
const FREQ_SLOTS: usize = 4;

/// Number of context IDs used for the first two MTF levels.
const LEVEL_CTXIDS: usize = 3;

/// Decode a BZZ-compressed byte slice.
///
/// BZZ streams consist of one or more blocks. Each block is preceded by a
/// 24-bit block size decoded in passthrough mode. A size of 0 signals the end
/// of the stream. Each block is decoded as follows:
/// 1. Decode `N` MTF values using ZP arithmetic coding with 262 contexts.
/// 2. Apply the inverse MTF transform to recover the BWT-encoded bytes.
/// 3. Apply the inverse BWT to recover the original bytes.
///
/// Returns the decompressed bytes, or an error if the stream is malformed.
pub fn bzz_decode(data: &[u8]) -> Result<Vec<u8>, BzzError> {
    let mut zp = ZpDecoder::new(data)?;
    let mut output = Vec::new();
    let mut block_ctx = [0u8; CTX_COUNT];

    loop {
        // Read 24-bit block size (passthrough, no context)
        let block_size = decode_raw_bits(&mut zp, 24);
        if block_size == 0 {
            // Size 0 = end-of-stream marker
            break;
        }

        let block = decode_one_block(&mut zp, &mut block_ctx, block_size as usize)?;
        output.extend_from_slice(&block);
    }

    Ok(output)
}

/// Compatibility alias for [`bzz_decode`].
///
/// This provides the same `decode(data)` API that the legacy modules used,
/// so that `document.rs` and other callers can use `crate::bzz::decode` without change.
pub fn decode(data: &[u8]) -> Result<Vec<u8>, BzzError> {
    bzz_decode(data)
}

/// Decode `bit_count` raw bits from the ZP stream (passthrough, no context).
///
/// The bits are decoded MSB-first using the arithmetic tree technique: start
/// with `n = 1` and repeatedly shift left and OR in the next passthrough bit
/// until `n >= 2^bit_count`. The result is `n - 2^bit_count`.
fn decode_raw_bits(zp: &mut ZpDecoder, bit_count: u32) -> u32 {
    let limit = 1u32 << bit_count;
    let mut n = 1u32;
    while n < limit {
        let bit = zp.decode_passthrough() as u32;
        n = (n << 1) | bit;
    }
    n - limit
}

/// Decode `bit_count` bits using a context binary tree.
///
/// `ctx_base` is the index of the first context in the subtree (0-indexed
/// after the subtree root). The decoding traverses the binary tree by
/// accumulating the decoded bit into an integer, starting from 1.
fn decode_context_bits(zp: &mut ZpDecoder, ctx: &mut [u8], ctx_base: usize, bit_count: u32) -> u32 {
    // The subtree root is at ctx_base - 1; children at ctx_base, ctx_base+1, ...
    let subtree_offset = ctx_base.wrapping_sub(1);
    let limit = 1u32 << bit_count;
    let mut n = 1u32;
    while n < limit {
        let bit = zp.decode_bit(&mut ctx[subtree_offset + n as usize]) as u32;
        n = (n << 1) | bit;
    }
    n - limit
}

/// Decode one BZZ block of `block_size` symbols.
///
/// The block shares ZP contexts across invocations (reset only once per stream,
/// not per block). The MTF and BWT state are reset per block.
fn decode_one_block(
    zp: &mut ZpDecoder,
    ctx: &mut [u8; CTX_COUNT],
    block_size: usize,
) -> Result<Vec<u8>, BzzError> {
    // Decode the frequency shift parameter (0, 1, or 2)
    // This controls how aggressively the MTF order adapts
    let mut freq_shift: u32 = 0;
    if zp.decode_passthrough() {
        freq_shift += 1;
        if zp.decode_passthrough() {
            freq_shift += 1;
        }
    }

    // Per-block MTF state
    let mut mtf_order: [u8; 256] = core::array::from_fn(|i| i as u8);
    let mut freq_counts = [0u32; FREQ_SLOTS];
    let mut freq_add: u32 = 4;
    let mut last_mtf_pos: u32 = 3;
    let mut marker_at: Option<usize> = None;

    // Decode N symbols, where N = block_size (includes the BWT marker)
    let mut bwt_data = vec![0u8; block_size];

    for (sym_idx, output_byte) in bwt_data.iter_mut().enumerate() {
        // Determine context ID based on last MTF position (clamped to 0..LEVEL_CTXIDS-1)
        let ctx_id = (last_mtf_pos.min(LEVEL_CTXIDS as u32 - 1)) as usize;

        // Hierarchical MTF position decoding
        // Each level asks: "is the MTF position in this range?"
        // The contexts advance through the `ctx` array as we descend.
        let mtf_position;
        let mut ctx_offset: usize = 0;

        if zp.decode_bit(&mut ctx[ctx_offset + ctx_id]) {
            // Level 0: position is 0
            mtf_position = 0;
        } else {
            ctx_offset += LEVEL_CTXIDS;
            if zp.decode_bit(&mut ctx[ctx_offset + ctx_id]) {
                // Level 1: position is 1
                mtf_position = 1;
            } else {
                ctx_offset += LEVEL_CTXIDS;
                if zp.decode_bit(&mut ctx[ctx_offset]) {
                    // Level 2: position in [2, 3]
                    mtf_position = 2 + decode_context_bits(zp, ctx, ctx_offset + 1, 1);
                } else {
                    ctx_offset += 2;
                    if zp.decode_bit(&mut ctx[ctx_offset]) {
                        // Level 3: position in [4, 7]
                        mtf_position = 4 + decode_context_bits(zp, ctx, ctx_offset + 1, 2);
                    } else {
                        ctx_offset += 4;
                        if zp.decode_bit(&mut ctx[ctx_offset]) {
                            // Level 4: position in [8, 15]
                            mtf_position = 8 + decode_context_bits(zp, ctx, ctx_offset + 1, 3);
                        } else {
                            ctx_offset += 8;
                            if zp.decode_bit(&mut ctx[ctx_offset]) {
                                // Level 5: position in [16, 31]
                                mtf_position = 16 + decode_context_bits(zp, ctx, ctx_offset + 1, 4);
                            } else {
                                ctx_offset += 16;
                                if zp.decode_bit(&mut ctx[ctx_offset]) {
                                    // Level 6: position in [32, 63]
                                    mtf_position =
                                        32 + decode_context_bits(zp, ctx, ctx_offset + 1, 5);
                                } else {
                                    ctx_offset += 32;
                                    if zp.decode_bit(&mut ctx[ctx_offset]) {
                                        // Level 7: position in [64, 127]
                                        mtf_position =
                                            64 + decode_context_bits(zp, ctx, ctx_offset + 1, 6);
                                    } else {
                                        ctx_offset += 64;
                                        if zp.decode_bit(&mut ctx[ctx_offset]) {
                                            // Level 8: position in [128, 255]
                                            mtf_position = 128
                                                + decode_context_bits(zp, ctx, ctx_offset + 1, 7);
                                        } else {
                                            // Level 9: BWT end-of-block marker (position 256)
                                            mtf_position = 256;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        last_mtf_pos = mtf_position;

        if mtf_position == 256 {
            // BWT marker: records where the marker byte sits in the block
            *output_byte = 0;
            marker_at = Some(sym_idx);
        } else {
            // Retrieve the byte at this MTF position
            let sym = *mtf_order
                .get(mtf_position as usize)
                .ok_or(BzzError::InvalidBlockSize)?;
            *output_byte = sym;

            // Update frequency tracking
            freq_add = freq_add.wrapping_add(freq_add >> freq_shift);
            if freq_add > 0x1000_0000 {
                freq_add >>= 24;
                for f in freq_counts.iter_mut() {
                    *f >>= 24;
                }
            }

            let mut combined_freq = freq_add;
            if (mtf_position as usize) < FREQ_SLOTS {
                combined_freq = combined_freq.saturating_add(freq_counts[mtf_position as usize]);
            }

            // Bubble the symbol toward the front of the MTF order
            let mut insert_at = mtf_position as usize;
            while insert_at >= FREQ_SLOTS {
                *mtf_order
                    .get_mut(insert_at)
                    .ok_or(BzzError::InvalidBlockSize)? = *mtf_order
                    .get(insert_at - 1)
                    .ok_or(BzzError::InvalidBlockSize)?;
                insert_at -= 1;
            }
            while insert_at > 0 {
                let prev_freq = *freq_counts
                    .get(insert_at - 1)
                    .ok_or(BzzError::InvalidBlockSize)?;
                if combined_freq >= prev_freq {
                    *mtf_order
                        .get_mut(insert_at)
                        .ok_or(BzzError::InvalidBlockSize)? = *mtf_order
                        .get(insert_at - 1)
                        .ok_or(BzzError::InvalidBlockSize)?;
                    *freq_counts
                        .get_mut(insert_at)
                        .ok_or(BzzError::InvalidBlockSize)? = prev_freq;
                    insert_at -= 1;
                } else {
                    break;
                }
            }
            *mtf_order
                .get_mut(insert_at)
                .ok_or(BzzError::InvalidBlockSize)? = sym;
            if let Some(fc) = freq_counts.get_mut(insert_at) {
                *fc = combined_freq;
            }
        }
    }

    let marker_pos = marker_at.ok_or(BzzError::MissingMarker)?;

    // Inverse Burrows-Wheeler Transform
    inverse_bwt(&bwt_data, marker_pos)
}

/// Inverse Burrows-Wheeler Transform.
///
/// Given `bwt_data` — the BWT-encoded block — and `marker_pos` — the position
/// of the BWT end-of-block marker — reconstruct the original byte sequence.
///
/// The output has length `bwt_data.len() - 1` (the marker byte is excluded).
fn inverse_bwt(bwt_data: &[u8], marker_pos: usize) -> Result<Vec<u8>, BzzError> {
    let total = bwt_data.len();
    if total == 0 {
        return Ok(Vec::new());
    }

    // Count occurrences of each byte value (excluding the marker position)
    let mut byte_count = [0u32; 256];
    // Build the "rank" array: rank[i] = occurrence index of bwt_data[i] among its siblings
    let mut rank = vec![0u32; total];

    for i in 0..total {
        if i == marker_pos {
            continue;
        }
        let byte_val = bwt_data[i] as usize;
        // Encode rank as (byte_val << 24) | occurrence_index
        rank[i] = ((byte_val as u32) << 24) | (byte_count[byte_val] & 0x00ff_ffff);
        byte_count[byte_val] += 1;
    }

    // Compute cumulative counts: prefix sums give the sorted starting positions.
    // Position 0 in the sorted order is reserved for the marker.
    let mut sorted_start = [0u32; 256];
    let mut running = 1u32; // start at 1 to skip the marker slot
    for (byte_val, count) in byte_count.iter().enumerate() {
        sorted_start[byte_val] = running;
        running += count;
    }

    // Reconstruct original data by following the BWT "follow" permutation
    let output_len = total - 1;
    let mut output = vec![0u8; output_len];
    let mut follow = 0usize; // start at position 0 in the follow chain
    let mut remaining = output_len;

    while remaining > 0 {
        let encoded = rank[follow];
        let byte_val = (encoded >> 24) as u8;
        let occurrence = encoded & 0x00ff_ffff;
        remaining -= 1;
        output[remaining] = byte_val;
        // Next position = start of this byte's sorted block + occurrence rank
        follow = (sorted_start[byte_val as usize] + occurrence) as usize;
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn golden_bzz_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/bzz")
    }

    // --- TDD: failing tests written first ---

    #[test]
    fn empty_input_returns_error() {
        // Empty input: ZpDecoder::new requires at least 2 bytes
        let result = bzz_decode(&[]);
        assert!(
            result.is_err(),
            "expected error for empty input, got {:?}",
            result
        );
    }

    #[test]
    fn single_byte_does_not_panic() {
        // Should return an error, not panic
        let result = bzz_decode(&[0x00]);
        assert!(result.is_err(), "expected error for 1-byte input");
    }

    #[test]
    fn bzz_decode_known_short() {
        // Decode the short golden test vector produced by the DjVu reference encoder
        let compressed =
            std::fs::read(golden_bzz_path().join("test_short.bzz")).expect("test fixture missing");
        let expected =
            std::fs::read(golden_bzz_path().join("test_short.txt")).expect("test fixture missing");
        let decoded = bzz_decode(&compressed).expect("bzz_decode failed");
        assert_eq!(
            decoded, expected,
            "decoded output does not match expected for test_short"
        );
    }

    #[test]
    fn bzz_decode_known_long() {
        let compressed =
            std::fs::read(golden_bzz_path().join("test_long.bzz")).expect("test fixture missing");
        let expected =
            std::fs::read(golden_bzz_path().join("test_long.txt")).expect("test fixture missing");
        let decoded = bzz_decode(&compressed).expect("bzz_decode failed");
        assert_eq!(
            decoded, expected,
            "decoded output does not match for test_long"
        );
    }

    #[test]
    fn bzz_decode_known_1byte() {
        let compressed =
            std::fs::read(golden_bzz_path().join("test_1byte.bzz")).expect("test fixture missing");
        let expected =
            std::fs::read(golden_bzz_path().join("test_1byte.txt")).expect("test fixture missing");
        let decoded = bzz_decode(&compressed).expect("bzz_decode failed");
        assert_eq!(
            decoded, expected,
            "decoded output does not match for test_1byte"
        );
    }

    #[test]
    fn bzz_decode_real_dirm_chunk() {
        // BZZ payload extracted from the DIRM chunk of navm_fgbz.djvu
        // This exercises a real-world DjVu file's directory chunk
        let compressed = std::fs::read(golden_bzz_path().join("navm_fgbz_dirm.bzz"))
            .expect("test fixture missing");
        let expected = std::fs::read(golden_bzz_path().join("navm_fgbz_dirm.bin"))
            .expect("test fixture missing");
        let decoded = bzz_decode(&compressed).expect("bzz_decode failed on real DIRM chunk");
        assert!(
            !decoded.is_empty(),
            "decoded DIRM chunk should not be empty"
        );
        assert_eq!(
            decoded, expected,
            "decoded DIRM chunk does not match expected"
        );
    }

    #[test]
    fn zp_tables_spot_check() {
        // Verify that the new ZP coder tables have the correct spec-defined values
        use crate::zp_impl::tables::{LPS_NEXT, MPS_NEXT, PROB, THRESHOLD};
        assert_eq!(PROB[0], 0x8000, "P[0] should be 0x8000");
        assert_eq!(PROB[250], 0x481a, "P[250] should be 0x481a");
        assert_eq!(MPS_NEXT[0], 84, "UP[0] should be 84");
        assert_eq!(LPS_NEXT[0], 145, "DN[0] should be 145");
        assert_eq!(THRESHOLD[83], 0, "M[83] should be 0");
        assert_eq!(THRESHOLD[250], 0, "M[250] should be 0");
    }
}
