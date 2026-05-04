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

use crate::BzzError;
use djvu_zp::ZpDecoder;

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

/// Maximum allowed block size (4 MB), matching DjVuLibre's MAXBLOCK.
const MAX_BLOCK_SIZE: usize = 4 * 1024 * 1024;

/// Maximum allowed total decompressed output size (256 MB).
const MAX_OUTPUT_SIZE: usize = 256 * 1024 * 1024;

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
        let block_size = decode_raw_bits(&mut zp, 24);
        if block_size == 0 {
            break;
        }
        let block_size = block_size as usize;
        if block_size > MAX_BLOCK_SIZE {
            return Err(BzzError::BlockSizeTooLarge(block_size));
        }
        let block = decode_one_block(&mut zp, &mut block_ctx, block_size)?;
        if output.len() + block.len() > MAX_OUTPUT_SIZE {
            return Err(BzzError::OutputTooLarge);
        }
        output.extend_from_slice(&block);
    }

    Ok(output)
}

/// Decode a BZZ-compressed byte slice, applying inverse BWT in parallel.
///
/// Requires the `parallel` feature (which enables `rayon`). The ZP arithmetic
/// decoding and inverse-MTF phases are inherently sequential (ZP context state
/// is shared across all blocks). Once all blocks are decoded into their
/// BWT-encoded form, the independent inverse-BWT transforms are dispatched to
/// the rayon thread pool.
///
/// For single-block streams the performance is identical to [`bzz_decode`].
/// For multi-block streams each block's inverse-BWT runs on its own thread.
#[cfg(feature = "parallel")]
pub fn bzz_decode_parallel(data: &[u8]) -> Result<Vec<u8>, BzzError> {
    use rayon::prelude::*;

    let mut zp = ZpDecoder::new(data)?;
    let mut block_ctx = [0u8; CTX_COUNT];

    // Phase 1 — sequential: ZP decode + inverse-MTF → collect (bwt_data, marker_pos)
    let mut bwt_blocks: Vec<(Vec<u8>, usize)> = Vec::new();
    let mut total_size: usize = 0;
    loop {
        let block_size = decode_raw_bits(&mut zp, 24);
        if block_size == 0 {
            break;
        }
        let block_size = block_size as usize;
        if block_size > MAX_BLOCK_SIZE {
            return Err(BzzError::BlockSizeTooLarge(block_size));
        }
        total_size = total_size.saturating_add(block_size);
        if total_size > MAX_OUTPUT_SIZE {
            return Err(BzzError::OutputTooLarge);
        }
        let (bwt_data, marker_pos) =
            decode_one_block_bwt_only(&mut zp, &mut block_ctx, block_size)?;
        bwt_blocks.push((bwt_data, marker_pos));
    }

    // Phase 2 — parallel: inverse-BWT per block (each block is independent)
    let decoded_blocks: Result<Vec<Vec<u8>>, BzzError> = bwt_blocks
        .into_par_iter()
        .map(|(bwt_data, marker_pos)| inverse_bwt(&bwt_data, marker_pos))
        .collect();

    let mut output = Vec::new();
    for block in decoded_blocks? {
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

/// Decode one BZZ block, returning the raw BWT-encoded bytes and marker position.
///
/// Identical to [`decode_one_block`] except the inverse-BWT step is skipped.
/// Used by [`bzz_decode_parallel`] to separate the sequential ZP/MTF phase
/// from the parallelisable inverse-BWT phase.
#[cfg(feature = "parallel")]
fn decode_one_block_bwt_only(
    zp: &mut ZpDecoder,
    ctx: &mut [u8; CTX_COUNT],
    block_size: usize,
) -> Result<(Vec<u8>, usize), BzzError> {
    let (bwt_data, marker_pos) = decode_mtf_phase(zp, ctx, block_size)?;
    Ok((bwt_data, marker_pos))
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
    let (bwt_data, marker_pos) = decode_mtf_phase(zp, ctx, block_size)?;
    inverse_bwt(&bwt_data, marker_pos)
}

/// ZP + MTF decode phase: returns `(bwt_data, marker_pos)`.
///
/// Shared by [`decode_one_block`] and [`decode_one_block_bwt_only`].
///
/// The five hot ZP fields (`a`, `c`, `fence`, `bit_buf`, `bit_count`) are
/// extracted into stack locals at the top so LLVM can keep them in registers
/// throughout the block loop without spilling through the struct pointer.
#[inline(never)]
#[allow(unused_assignments)] // `fence` is primed by the first passthrough before decode_bit reads it
fn decode_mtf_phase(
    zp: &mut ZpDecoder,
    ctx: &mut [u8; CTX_COUNT],
    block_size: usize,
) -> Result<(Vec<u8>, usize), BzzError> {
    use djvu_zp::tables::{LPS_NEXT, MPS_NEXT, PROB, THRESHOLD};

    // ── Extract ZP state into locals ─────────────────────────────────────────
    let mut a = zp.a;
    let mut c = zp.c;
    let mut fence = c.min(0x7fff); // same invariant as zp.fence
    let mut bit_buf = zp.bit_buf;
    let mut bit_count = zp.bit_count;
    let data = zp.data;
    let mut pos = zp.pos;

    // ── ZP helper macros ──────────────────────────────────────────────────────
    macro_rules! read_byte {
        () => {{
            let b = if pos < data.len() { data[pos] } else { 0xff };
            pos = pos.wrapping_add(1);
            b as u32
        }};
    }
    macro_rules! refill {
        () => {
            while bit_count <= 24 {
                bit_buf = (bit_buf << 8) | read_byte!();
                bit_count += 8;
            }
        };
    }
    macro_rules! renorm {
        () => {{
            let shift = (a as u16).leading_ones();
            bit_count -= shift as i32;
            a = (a << shift) & 0xffff;
            let mask = (1u32 << (shift & 31)).wrapping_sub(1);
            c = ((c << shift) | (bit_buf >> (bit_count as u32 & 31)) & mask) & 0xffff;
            if bit_count < 16 {
                refill!();
            }
            fence = c.min(0x7fff);
        }};
    }

    /// Decode one adaptive bit using `ctx_byte` as the context.
    /// Returns `true` for the MPS event, `false` for LPS.
    macro_rules! decode_bit {
        ($ctx_byte:expr) => {{
            let state = $ctx_byte as usize;
            let mps = state & 1;
            let z = a + PROB[state] as u32;
            if z <= fence {
                a = z;
                mps != 0
            } else {
                let boundary = 0x6000u32 + ((a + z) >> 2);
                let z_clamped = z.min(boundary);
                if z_clamped > c {
                    let complement = 0x10000u32 - z_clamped;
                    a = (a + complement) & 0xffff;
                    c = (c + complement) & 0xffff;
                    $ctx_byte = LPS_NEXT[state];
                    renorm!();
                    (1 - mps) != 0
                } else {
                    if a >= THRESHOLD[state] as u32 {
                        $ctx_byte = MPS_NEXT[state];
                    }
                    bit_count -= 1;
                    a = (z_clamped << 1) & 0xffff;
                    c = (c << 1 | (bit_buf >> (bit_count as u32 & 31)) & 1) & 0xffff;
                    if bit_count < 16 {
                        refill!();
                    }
                    fence = c.min(0x7fff);
                    mps != 0
                }
            }
        }};
    }

    /// Decode one passthrough bit (no context).
    /// Threshold: z = 0x8000 + (a >> 1).
    macro_rules! decode_passthrough {
        () => {{
            let z = (0x8000u32 + (a >> 1)) as u16;
            if (z as u32) > c {
                let complement = 0x10000u32 - z as u32;
                a = (a + complement) & 0xffff;
                c = (c + complement) & 0xffff;
                renorm!();
                true
            } else {
                bit_count -= 1;
                a = (z as u32 * 2) & 0xffff;
                c = (c << 1 | (bit_buf >> (bit_count as u32 & 31)) & 1) & 0xffff;
                if bit_count < 16 {
                    refill!();
                }
                fence = c.min(0x7fff);
                false
            }
        }};
    }

    /// Decode `$n_bits` bits from a context sub-tree rooted at `ctx[$base - 1]`.
    /// Returns a `u32` in `[0, 2^n_bits)`.
    macro_rules! decode_ctx_bits {
        ($base:expr, $n_bits:expr) => {{
            let subtree_offset = ($base as usize).wrapping_sub(1);
            let limit = 1u32 << $n_bits;
            let mut nn = 1u32;
            while nn < limit {
                let bit = decode_bit!(ctx[subtree_offset + nn as usize]) as u32;
                nn = (nn << 1) | bit;
            }
            nn - limit
        }};
    }

    // ── Decode frequency-shift parameter ─────────────────────────────────────
    let mut freq_shift: u32 = 0;
    if decode_passthrough!() {
        freq_shift += 1;
        if decode_passthrough!() {
            freq_shift += 1;
        }
    }

    // ── Per-block MTF state ───────────────────────────────────────────────────
    let mut mtf_order: [u8; 256] = core::array::from_fn(|i| i as u8);
    let mut freq_counts = [0u32; FREQ_SLOTS];
    let mut freq_add: u32 = 4;
    let mut last_mtf_pos: u32 = 3;
    let mut marker_at: Option<usize> = None;

    let mut bwt_data = vec![0u8; block_size];

    for (sym_idx, output_byte) in bwt_data.iter_mut().enumerate() {
        let ctx_id = (last_mtf_pos.min(LEVEL_CTXIDS as u32 - 1)) as usize;

        let mtf_position;
        let mut ctx_offset: usize = 0;

        if decode_bit!(ctx[ctx_offset + ctx_id]) {
            mtf_position = 0;
        } else {
            ctx_offset += LEVEL_CTXIDS;
            if decode_bit!(ctx[ctx_offset + ctx_id]) {
                mtf_position = 1;
            } else {
                ctx_offset += LEVEL_CTXIDS;
                if decode_bit!(ctx[ctx_offset]) {
                    mtf_position = 2 + decode_ctx_bits!(ctx_offset + 1, 1);
                } else {
                    ctx_offset += 2;
                    if decode_bit!(ctx[ctx_offset]) {
                        mtf_position = 4 + decode_ctx_bits!(ctx_offset + 1, 2);
                    } else {
                        ctx_offset += 4;
                        if decode_bit!(ctx[ctx_offset]) {
                            mtf_position = 8 + decode_ctx_bits!(ctx_offset + 1, 3);
                        } else {
                            ctx_offset += 8;
                            if decode_bit!(ctx[ctx_offset]) {
                                mtf_position = 16 + decode_ctx_bits!(ctx_offset + 1, 4);
                            } else {
                                ctx_offset += 16;
                                if decode_bit!(ctx[ctx_offset]) {
                                    mtf_position = 32 + decode_ctx_bits!(ctx_offset + 1, 5);
                                } else {
                                    ctx_offset += 32;
                                    if decode_bit!(ctx[ctx_offset]) {
                                        mtf_position = 64 + decode_ctx_bits!(ctx_offset + 1, 6);
                                    } else {
                                        ctx_offset += 64;
                                        if decode_bit!(ctx[ctx_offset]) {
                                            mtf_position =
                                                128 + decode_ctx_bits!(ctx_offset + 1, 7);
                                        } else {
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
            *output_byte = 0;
            marker_at = Some(sym_idx);
        } else {
            let sym = *mtf_order
                .get(mtf_position as usize)
                .ok_or(BzzError::InvalidBlockSize)?;
            *output_byte = sym;

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

    // ── Write ZP state back ───────────────────────────────────────────────────
    zp.a = a;
    zp.c = c;
    zp.fence = fence;
    zp.bit_buf = bit_buf;
    zp.bit_count = bit_count;
    zp.pos = pos;

    let marker_pos = marker_at.ok_or(BzzError::MissingMarker)?;
    Ok((bwt_data, marker_pos))
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
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/bzz")
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

    /// Parallel BWT decode (feature = "parallel") produces identical output to
    /// the sequential path on a multi-block fixture.
    #[cfg(feature = "parallel")]
    #[test]
    fn parallel_bzz_matches_sequential() {
        let compressed =
            std::fs::read(golden_bzz_path().join("test_long.bzz")).expect("test fixture missing");
        let expected =
            std::fs::read(golden_bzz_path().join("test_long.txt")).expect("test fixture missing");

        let seq = bzz_decode(&compressed).expect("sequential decode failed");
        let par = bzz_decode_parallel(&compressed).expect("parallel decode failed");

        assert_eq!(seq, expected, "sequential output mismatch");
        assert_eq!(par, expected, "parallel output mismatch");
        assert_eq!(seq, par, "parallel and sequential outputs differ");
    }

    #[test]
    fn zp_tables_spot_check() {
        // Verify that the new ZP coder tables have the correct spec-defined values
        use djvu_zp::tables::{LPS_NEXT, MPS_NEXT, PROB, THRESHOLD};
        assert_eq!(PROB[0], 0x8000, "P[0] should be 0x8000");
        assert_eq!(PROB[250], 0x481a, "P[250] should be 0x481a");
        assert_eq!(MPS_NEXT[0], 84, "UP[0] should be 84");
        assert_eq!(LPS_NEXT[0], 145, "DN[0] should be 145");
        assert_eq!(THRESHOLD[83], 0, "M[83] should be 0");
        assert_eq!(THRESHOLD[250], 0, "M[250] should be 0");
    }
}
