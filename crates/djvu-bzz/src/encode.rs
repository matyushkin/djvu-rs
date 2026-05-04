//! BZZ compressor — pure-Rust implementation.
//!
//! Encoding counterpart to [`crate::bzz_decode`]. Combines:
//! 1. Forward Burrows-Wheeler Transform (BWT)
//! 2. Forward Move-To-Front (MTF) with frequency tracking
//! 3. ZP adaptive arithmetic encoding
//!
//! The output can be decoded by [`crate::bzz_decode`].

use djvu_zp::encoder::ZpEncoder;

/// Maximum block size (4 MB), matching DjVuLibre's MAXBLOCK.
const MAX_BLOCK_SIZE: usize = 4 * 1024 * 1024;

/// Number of ZP contexts per BZZ stream.
const CTX_COUNT: usize = 300;

/// Number of frequency slots in MTF ordering.
const FREQ_SLOTS: usize = 4;

/// Number of adaptive context IDs for the first two MTF levels.
const LEVEL_CTXIDS: usize = 3;

/// Compress `data` using the BZZ algorithm.
///
/// Returns the compressed byte stream that can be decompressed with
/// [`crate::bzz_decode`].
pub fn bzz_encode(data: &[u8]) -> Vec<u8> {
    let mut enc = ZpEncoder::new();
    let mut block_ctx = [0u8; CTX_COUNT];

    // Split input into blocks of at most MAX_BLOCK_SIZE
    let mut offset = 0;
    while offset < data.len() {
        let end = (offset + MAX_BLOCK_SIZE).min(data.len());
        let block = &data[offset..end];
        encode_one_block(&mut enc, &mut block_ctx, block);
        offset = end;
    }

    // End-of-stream marker: 24-bit zero block size
    encode_raw_bits(&mut enc, 0, 24);

    enc.finish()
}

/// Encode `bit_count` raw bits to the ZP stream (passthrough, MSB-first).
///
/// Mirrors `decode_raw_bits`: decoder does `n=1; while n < limit { n = (n<<1)|bit }; return n-limit`.
/// Encoder emits the same bits in tree order.
fn encode_raw_bits(enc: &mut ZpEncoder, value: u32, bit_count: u32) {
    // The decoder accumulates n starting from 1, shifting in bit_count bits.
    // Final value = n - (1 << bit_count). So n = (1 << bit_count) + value.
    // The bits emitted (MSB first) are positions bit_count-1 down to 0 of n,
    // but position bit_count (the leading 1) is implicit.
    for i in (0..bit_count).rev() {
        let bit = ((value >> i) & 1) != 0;
        enc.encode_passthrough(bit);
    }
}

/// Encode `bit_count` bits using a context binary tree.
///
/// Mirrors `decode_context_bits` in the decoder.
fn encode_context_bits(
    enc: &mut ZpEncoder,
    ctx: &mut [u8],
    ctx_base: usize,
    bit_count: u32,
    value: u32,
) {
    let subtree_offset = ctx_base.wrapping_sub(1);
    let limit = 1u32 << bit_count;
    let coded = limit + value;
    let mut n = 1u32;
    for i in (0..bit_count).rev() {
        let bit = ((coded >> i) & 1) != 0;
        enc.encode_bit(&mut ctx[subtree_offset + n as usize], bit);
        n = (n << 1) | (bit as u32);
    }
}

/// Encode one BZZ block.
fn encode_one_block(enc: &mut ZpEncoder, ctx: &mut [u8; CTX_COUNT], data: &[u8]) {
    // Forward BWT
    let (bwt_data, marker_pos) = forward_bwt(data);

    // Block size = data.len() + 1 (includes marker)
    let block_size = bwt_data.len();
    encode_raw_bits(enc, block_size as u32, 24);

    // Frequency shift: use 0 for simplicity (no adaptation speedup)
    // freq_shift = 0: encode_passthrough(false)
    enc.encode_passthrough(false);

    // Forward MTF + ZP encode
    let mut mtf_order: [u8; 256] = core::array::from_fn(|i| i as u8);
    // Reverse index: mtf_index[byte] = position in mtf_order
    let mut mtf_index: [u8; 256] = core::array::from_fn(|i| i as u8);
    let mut freq_counts = [0u32; FREQ_SLOTS];
    let mut freq_add: u32 = 4;
    let mut last_mtf_pos: u32 = 3;

    for (sym_idx, &byte) in bwt_data.iter().enumerate() {
        let ctx_id = (last_mtf_pos.min(LEVEL_CTXIDS as u32 - 1)) as usize;

        let mtf_position = if sym_idx == marker_pos {
            256u32
        } else {
            mtf_index[byte as usize] as u32
        };

        encode_mtf_position(enc, ctx, ctx_id, mtf_position);
        last_mtf_pos = mtf_position;

        if mtf_position != 256 {
            let sym = mtf_order[mtf_position as usize];

            // freq_shift=0: freq_add doubles each time
            freq_add = freq_add.wrapping_add(freq_add);
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

            // Bubble symbol toward front, updating reverse index
            let mut insert_at = mtf_position as usize;
            while insert_at >= FREQ_SLOTS {
                let prev_sym = mtf_order[insert_at - 1];
                mtf_order[insert_at] = prev_sym;
                mtf_index[prev_sym as usize] = insert_at as u8;
                insert_at -= 1;
            }
            while insert_at > 0 {
                let prev_freq = freq_counts[insert_at - 1];
                if combined_freq >= prev_freq {
                    let prev_sym = mtf_order[insert_at - 1];
                    mtf_order[insert_at] = prev_sym;
                    mtf_index[prev_sym as usize] = insert_at as u8;
                    freq_counts[insert_at] = prev_freq;
                    insert_at -= 1;
                } else {
                    break;
                }
            }
            mtf_order[insert_at] = sym;
            mtf_index[sym as usize] = insert_at as u8;
            if insert_at < FREQ_SLOTS {
                freq_counts[insert_at] = combined_freq;
            }
        }
    }
}

/// Encode an MTF position using the hierarchical context tree.
///
/// Mirrors the decoding hierarchy in `decode_mtf_phase`.
fn encode_mtf_position(enc: &mut ZpEncoder, ctx: &mut [u8; CTX_COUNT], ctx_id: usize, pos: u32) {
    let mut ctx_offset: usize = 0;

    // Level 0: position 0
    if pos == 0 {
        enc.encode_bit(&mut ctx[ctx_offset + ctx_id], true);
        return;
    }
    enc.encode_bit(&mut ctx[ctx_offset + ctx_id], false);
    ctx_offset += LEVEL_CTXIDS;

    // Level 1: position 1
    if pos == 1 {
        enc.encode_bit(&mut ctx[ctx_offset + ctx_id], true);
        return;
    }
    enc.encode_bit(&mut ctx[ctx_offset + ctx_id], false);
    ctx_offset += LEVEL_CTXIDS;

    // Level 2: positions [2, 3]
    if pos < 4 {
        enc.encode_bit(&mut ctx[ctx_offset], true);
        encode_context_bits(enc, ctx, ctx_offset + 1, 1, pos - 2);
        return;
    }
    enc.encode_bit(&mut ctx[ctx_offset], false);
    ctx_offset += 2;

    // Level 3: positions [4, 7]
    if pos < 8 {
        enc.encode_bit(&mut ctx[ctx_offset], true);
        encode_context_bits(enc, ctx, ctx_offset + 1, 2, pos - 4);
        return;
    }
    enc.encode_bit(&mut ctx[ctx_offset], false);
    ctx_offset += 4;

    // Level 4: positions [8, 15]
    if pos < 16 {
        enc.encode_bit(&mut ctx[ctx_offset], true);
        encode_context_bits(enc, ctx, ctx_offset + 1, 3, pos - 8);
        return;
    }
    enc.encode_bit(&mut ctx[ctx_offset], false);
    ctx_offset += 8;

    // Level 5: positions [16, 31]
    if pos < 32 {
        enc.encode_bit(&mut ctx[ctx_offset], true);
        encode_context_bits(enc, ctx, ctx_offset + 1, 4, pos - 16);
        return;
    }
    enc.encode_bit(&mut ctx[ctx_offset], false);
    ctx_offset += 16;

    // Level 6: positions [32, 63]
    if pos < 64 {
        enc.encode_bit(&mut ctx[ctx_offset], true);
        encode_context_bits(enc, ctx, ctx_offset + 1, 5, pos - 32);
        return;
    }
    enc.encode_bit(&mut ctx[ctx_offset], false);
    ctx_offset += 32;

    // Level 7: positions [64, 127]
    if pos < 128 {
        enc.encode_bit(&mut ctx[ctx_offset], true);
        encode_context_bits(enc, ctx, ctx_offset + 1, 6, pos - 64);
        return;
    }
    enc.encode_bit(&mut ctx[ctx_offset], false);
    ctx_offset += 64;

    // Level 8: positions [128, 255]
    if pos < 256 {
        enc.encode_bit(&mut ctx[ctx_offset], true);
        encode_context_bits(enc, ctx, ctx_offset + 1, 7, pos - 128);
        return;
    }
    enc.encode_bit(&mut ctx[ctx_offset], false);
    // pos == 256 → BWT marker (no additional bits)
}

/// Forward Burrows-Wheeler Transform.
///
/// Returns `(bwt_output, marker_pos)` where `bwt_output` has length `data.len() + 1`
/// (the extra byte is the BWT marker at `marker_pos`).
///
/// We treat the input as `S$ where `$` is a sentinel smaller than all data bytes.
/// Sort all n+1 rotations of `S$`. The last column gives the BWT output.
/// `marker_pos` is the row where `$` appears in the last column (= the row of
/// rotation 0, since rotation 0 ends with `$`).
fn forward_bwt(data: &[u8]) -> (Vec<u8>, usize) {
    let n = data.len();
    if n == 0 {
        return (vec![0], 0);
    }

    // Build suffix array of S$ (length m = n+1) using prefix doubling.
    // S$ = data[0..n] ++ sentinel, where sentinel < all data bytes.
    let m = n + 1;
    let sa = suffix_array_of_bwt_string(data);

    // Build BWT output from suffix array
    let mut bwt_output = Vec::with_capacity(m);
    let mut marker_pos = 0;
    for (row, &idx) in sa.iter().enumerate() {
        if idx == 0 {
            marker_pos = row;
            bwt_output.push(0); // dummy marker byte
        } else if idx == n {
            bwt_output.push(data[n - 1]);
        } else {
            bwt_output.push(data[idx - 1]);
        }
    }

    (bwt_output, marker_pos)
}

/// Compute suffix array of the string S$ where S = data and $ is a sentinel
/// smaller than all bytes. Uses prefix doubling with radix sort (O(n log n)).
fn suffix_array_of_bwt_string(data: &[u8]) -> Vec<usize> {
    let n = data.len();
    let m = n + 1; // length of S$

    // Initial ranks: sentinel=0, data bytes = byte+1
    let mut rank = vec![0u32; m];
    for i in 0..n {
        rank[i] = data[i] as u32 + 1;
    }

    let mut sa: Vec<usize> = (0..m).collect();
    let mut new_rank = vec![0u32; m];
    let mut tmp = vec![0usize; m];
    let mut gap = 1usize;

    // Initial sort by first character (counting sort, 257 buckets)
    {
        let mut count = [0u32; 257];
        for &r in &rank {
            count[r as usize] += 1;
        }
        let mut sum = 0u32;
        for c in count.iter_mut() {
            let t = *c;
            *c = sum;
            sum += t;
        }
        for i in 0..m {
            sa[count[rank[i] as usize] as usize] = i;
            count[rank[i] as usize] += 1;
        }
        // Assign initial ranks
        new_rank[sa[0]] = 0;
        for i in 1..m {
            new_rank[sa[i]] =
                new_rank[sa[i - 1]] + if rank[sa[i]] != rank[sa[i - 1]] { 1 } else { 0 };
        }
        rank.copy_from_slice(&new_rank);
        if rank[sa[m - 1]] as usize == m - 1 {
            return sa;
        }
    }

    while gap < m {
        let num_ranks = rank[sa[m - 1]] as usize + 1;

        // Radix sort by (rank[i], rank[i+gap]): sort by second key first, then stable sort by first key.
        // Second key: rank[(i+gap)] if i+gap < m, else 0
        let second_key = |i: usize| -> u32 {
            let j = i + gap;
            if j < m { rank[j] } else { 0 }
        };

        // Counting sort by second key
        let mut count = vec![0u32; num_ranks + 1];
        for i in 0..m {
            count[second_key(i) as usize] += 1;
        }
        let mut sum = 0u32;
        for c in count.iter_mut() {
            let t = *c;
            *c = sum;
            sum += t;
        }
        for i in 0..m {
            let k = second_key(i) as usize;
            tmp[count[k] as usize] = i;
            count[k] += 1;
        }

        // Stable counting sort by first key
        count.iter_mut().for_each(|c| *c = 0);
        count.resize(num_ranks + 1, 0);
        for &i in &tmp {
            count[rank[i] as usize] += 1;
        }
        sum = 0;
        for c in count.iter_mut() {
            let t = *c;
            *c = sum;
            sum += t;
        }
        for &i in &tmp {
            let k = rank[i] as usize;
            sa[count[k] as usize] = i;
            count[k] += 1;
        }

        // Assign new ranks
        new_rank[sa[0]] = 0;
        for i in 1..m {
            let prev = sa[i - 1];
            let curr = sa[i];
            let same = rank[prev] == rank[curr] && second_key(prev) == second_key(curr);
            new_rank[curr] = new_rank[prev] + if same { 0 } else { 1 };
        }
        rank.copy_from_slice(&new_rank);

        if rank[sa[m - 1]] as usize == m - 1 {
            break;
        }
        gap *= 2;
    }

    sa
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bzz_decode;

    #[test]
    fn bzz_roundtrip_empty() {
        let compressed = bzz_encode(b"");
        let decoded = bzz_decode(&compressed).expect("decode");
        assert_eq!(decoded, b"");
    }

    #[test]
    fn bzz_roundtrip_single_byte() {
        let compressed = bzz_encode(b"A");
        let decoded = bzz_decode(&compressed).expect("decode");
        assert_eq!(decoded, b"A");
    }

    #[test]
    fn bzz_roundtrip_short_string() {
        let input = b"Hello, World!";
        let compressed = bzz_encode(input);
        let decoded = bzz_decode(&compressed).expect("decode");
        assert_eq!(decoded, input);
    }

    #[test]
    fn bzz_roundtrip_repeated() {
        let input = b"aaaaaaaaaa";
        let compressed = bzz_encode(input);
        let decoded = bzz_decode(&compressed).expect("decode");
        assert_eq!(decoded, input);
    }

    #[test]
    fn bzz_roundtrip_all_bytes() {
        let input: Vec<u8> = (0..=255).collect();
        let compressed = bzz_encode(&input);
        let decoded = bzz_decode(&compressed).expect("decode");
        assert_eq!(decoded, input);
    }

    #[test]
    fn bzz_roundtrip_golden_texts() {
        let golden = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/bzz");
        for name in ["test_short.txt", "test_long.txt", "test_1byte.txt"] {
            let path = golden.join(name);
            if path.exists() {
                let original = std::fs::read(&path).unwrap();
                let compressed = bzz_encode(&original);
                let decoded = bzz_decode(&compressed).expect("decode golden");
                assert_eq!(decoded, original, "roundtrip failed for {name}");
            }
        }
    }

    #[test]
    fn bzz_roundtrip_100kb_random() {
        let mut input = Vec::with_capacity(100 * 1024);
        let mut rng: u64 = 0xcafe_babe;
        for _ in 0..100 * 1024 {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            input.push((rng & 0xff) as u8);
        }
        let compressed = bzz_encode(&input);
        let decoded = bzz_decode(&compressed).expect("decode");
        assert_eq!(decoded, input);
    }

    #[test]
    fn bzz_roundtrip_compressible() {
        // Repetitive text-like data should compress well
        let pattern = b"The quick brown fox jumps over the lazy dog. ";
        let mut input = Vec::with_capacity(10_000);
        while input.len() < 10_000 {
            input.extend_from_slice(pattern);
        }
        input.truncate(10_000);
        let compressed = bzz_encode(&input);
        let decoded = bzz_decode(&compressed).expect("decode");
        assert_eq!(decoded, input);
        // BZZ should compress repetitive text significantly
        assert!(
            compressed.len() < input.len() / 2,
            "expected >50% compression, got {} → {} ({:.1}%)",
            input.len(),
            compressed.len(),
            compressed.len() as f64 / input.len() as f64 * 100.0
        );
    }

    #[test]
    fn bzz_roundtrip_1kb() {
        let mut input = Vec::with_capacity(1024);
        for i in 0..1024u32 {
            input.push((i.wrapping_mul(7).wrapping_add(13) % 256) as u8);
        }
        let compressed = bzz_encode(&input);
        let decoded = bzz_decode(&compressed).expect("decode");
        assert_eq!(decoded, input);
    }
}
