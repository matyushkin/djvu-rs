//! ZP adaptive binary arithmetic encoder.
//!
//! Encoding counterpart to [`super::ZpDecoder`]. Produces byte streams
//! that the decoder can consume. Matches DjVuLibre's ZPCodec encoder.

use super::tables::{LPS_NEXT, MPS_NEXT, PROB, THRESHOLD};

/// ZP adaptive binary arithmetic encoder.
///
/// All internal registers (`a`, `subend`) are `u32` to match DjVuLibre's
/// `unsigned int` types. They hold u16-range values but intermediate
/// arithmetic can exceed 0xFFFF, which is critical for correct carry
/// propagation in `zemit`.
pub struct ZpEncoder {
    /// Current interval width — stored as u32 but logically u16 after shifts.
    a: u32,
    /// Sub-interval lower bound for bit emission — u32 for carry propagation.
    subend: u32,
    /// 24-bit shift buffer for carry propagation (initialized to 0xFFFFFF).
    buffer: u32,
    /// Pending zero-byte run count for carry propagation.
    nrun: i32,
    /// Delay counter: first 25 outbit calls are absorbed.
    delay: i32,
    /// Byte accumulator for output.
    byte: u8,
    /// Bits accumulated in `byte` (0..8).
    scount: u32,
    /// Output bytes.
    output: Vec<u8>,
}

impl Default for ZpEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ZpEncoder {
    pub fn new() -> Self {
        Self {
            a: 0,
            subend: 0,
            buffer: 0xffffff,
            nrun: 0,
            delay: 25,
            byte: 0,
            scount: 0,
            output: Vec::new(),
        }
    }

    /// Bytes flushed to the output buffer so far.
    ///
    /// Diagnostic accessor only — a read-only snapshot of `output.len()` mid-stream
    /// (subject to the encoder's ~25-bit initial delay and carry-propagation
    /// buffering, so it is a monotonic approximation, not an exact per-bit byte
    /// count). Does not affect encoding state or the emitted bytes in any way, so
    /// it cannot change what a decoder reads. Used by encoder-side size-attribution
    /// probes (e.g. per-band byte accounting) that need a running total without
    /// waiting for `finish()`.
    pub fn bytes_written(&self) -> usize {
        self.output.len()
    }

    /// Encode one bit using an adaptive probability context.
    ///
    /// Matches DjVuLibre's inline `encoder(int bit, BitContext &ctx)`:
    /// - LPS always calls encode_lps
    /// - MPS with z >= 0x8000 calls encode_mps
    /// - MPS with z < 0x8000 takes fast path (a = z, no shift)
    pub fn encode_bit(&mut self, ctx: &mut u8, bit: bool) {
        let state = *ctx as usize;
        let mps_bit = (state & 1) != 0;
        let z = self.a + PROB[state] as u32;

        if bit != mps_bit {
            self.encode_lps(ctx, z);
        } else if z >= 0x8000 {
            self.encode_mps(ctx, z);
        } else {
            // Fast path: MPS and z < 0x8000 — just update a, no shift
            self.a = z;
        }
    }

    /// Encode `n` copies of `bit` in one context: the same bytes and final
    /// context state as `n` calls to [`encode_bit`](Self::encode_bit).
    ///
    /// An MPS bit on the fast path (`a + p < 0x8000`) only adds `p` to `a`,
    /// so a run of them collapses into one multiply-add; only the bits that
    /// reach `0x8000` (one shift each) take the full MPS step.
    pub fn encode_run(&mut self, ctx: &mut u8, bit: bool, mut n: usize) {
        while n > 0 {
            let state = *ctx as usize;
            let p = PROB[state] as u32;
            if bit != ((state & 1) != 0) {
                self.encode_lps(ctx, self.a + p);
                n -= 1;
                continue;
            }
            if p == 0 {
                // Padding states only: `z = a` stays on the fast path.
                return;
            }
            // Fast steps before `a + p` reaches 0x8000 (`a < 0x8000` always).
            let fast = (0x7fff_u32.saturating_sub(self.a) / p) as usize;
            if fast >= n {
                self.a += n as u32 * p;
                return;
            }
            self.a += fast as u32 * p;
            self.encode_mps(ctx, self.a + p);
            n -= fast + 1;
        }
    }

    /// Encode one bit in IW44 passthrough mode (threshold `z = 0x8000 + 3a/8`).
    ///
    /// Counterpart to [`ZpDecoder::decode_passthrough_iw44`](crate::ZpDecoder::decode_passthrough_iw44); must produce a
    /// stream that it correctly decodes.
    pub fn encode_passthrough_iw44(&mut self, bit: bool) {
        let z = 0x8000 + (3 * self.a / 8);
        // Invariant: self.a < 0x8000 (all encode paths maintain this).
        // Therefore z = 0x8000 + 3a/8 ∈ [0x8000, 0xB000) — always ≥ 0x8000.
        if !bit {
            self.a = z;
            // z ≥ 0x8000 always — single unconditional shift
            self.zemit(1 - (self.subend >> 15) as i32);
            self.subend = (self.subend << 1) & 0xffff;
            self.a = (self.a << 1) & 0xffff;
        } else {
            let z_comp = 0x10000 - z;
            self.subend += z_comp;
            self.a += z_comp;
            while self.a >= 0x8000 {
                self.zemit(1 - (self.subend >> 15) as i32);
                self.subend = (self.subend << 1) & 0xffff;
                self.a = (self.a << 1) & 0xffff;
            }
        }
    }

    pub fn encode_passthrough(&mut self, bit: bool) {
        let z = 0x8000 + (self.a >> 1);
        // Invariant: self.a < 0x8000, so z = 0x8000 + a/2 ∈ [0x8000, 0xC000) — always ≥ 0x8000.
        if !bit {
            // false (MPS-like): a = z, single unconditional shift
            self.a = z;
            self.zemit(1 - (self.subend >> 15) as i32);
            self.subend = (self.subend << 1) & 0xffff;
            self.a = (self.a << 1) & 0xffff;
        } else {
            // true (LPS-like): z_comp = 0x10000 - z
            let z_comp = 0x10000 - z;
            self.subend += z_comp;
            self.a += z_comp;
            while self.a >= 0x8000 {
                self.zemit(1 - (self.subend >> 15) as i32);
                self.subend = (self.subend << 1) & 0xffff;
                self.a = (self.a << 1) & 0xffff;
            }
        }
    }

    /// Flush the encoder and return the compressed byte stream.
    pub fn finish(mut self) -> Vec<u8> {
        // eflush: round subend up to disambiguate
        if self.subend > 0x8000 {
            self.subend = 0x10000;
        } else if self.subend > 0 {
            self.subend = 0x8000;
        }
        // Emit until buffer is flushed and subend is 0
        while self.buffer != 0xffffff || self.subend != 0 {
            self.zemit(1 - (self.subend >> 15) as i32);
            self.subend = (self.subend << 1) & 0xffff;
        }
        // Final bits
        self.outbit(1);
        while self.nrun > 0 {
            self.nrun -= 1;
            self.outbit(0);
        }
        // Pad remaining byte with 1s
        while self.scount > 0 {
            self.outbit(1);
        }
        self.delay = 0xff; // prevent further output
        // Ensure minimum 2 bytes for decoder initialization
        while self.output.len() < 2 {
            self.output.push(0xff);
        }
        self.output
    }

    fn encode_mps(&mut self, ctx: &mut u8, z: u32) {
        // Clamp z: d = 0x6000 + (z + a) / 4
        let d = 0x6000 + ((z + self.a) >> 2);
        let z = z.min(d);

        if (self.a & 0xffff) as u16 >= THRESHOLD[*ctx as usize] {
            *ctx = MPS_NEXT[*ctx as usize];
        }
        // Code MPS bit + single shift
        self.a = z;
        self.zemit(1 - (self.subend >> 15) as i32);
        self.subend = (self.subend << 1) & 0xffff;
        self.a = (self.a << 1) & 0xffff;
    }

    fn encode_lps(&mut self, ctx: &mut u8, z: u32) {
        // Clamp z
        let d = 0x6000 + ((z + self.a) >> 2);
        let z = z.min(d);

        *ctx = LPS_NEXT[*ctx as usize];
        let z_comp = 0x10000 - z;
        self.subend += z_comp;
        self.a += z_comp;
        while self.a >= 0x8000 {
            self.zemit(1 - (self.subend >> 15) as i32);
            self.subend = (self.subend << 1) & 0xffff;
            self.a = (self.a << 1) & 0xffff;
        }
    }

    /// Emit one bit through the 24-bit carry-propagation buffer.
    fn zemit(&mut self, b: i32) {
        self.buffer = (self.buffer << 1).wrapping_add(b as u32);
        let top = self.buffer >> 24;
        self.buffer &= 0xffffff;
        match top {
            1 => {
                self.outbit(1);
                while self.nrun > 0 {
                    self.nrun -= 1;
                    self.outbit(0);
                }
            }
            0xff => {
                self.outbit(0);
                while self.nrun > 0 {
                    self.nrun -= 1;
                    self.outbit(1);
                }
            }
            0 => {
                self.nrun += 1;
            }
            _ => {} // shouldn't happen
        }
    }

    /// Emit one bit to the output byte stream (with delay).
    fn outbit(&mut self, bit: i32) {
        if self.delay > 0 {
            if self.delay < 0xff {
                self.delay -= 1;
            }
            return;
        }
        self.byte = (self.byte << 1) | (bit as u8);
        self.scount += 1;
        if self.scount == 8 {
            self.output.push(self.byte);
            self.scount = 0;
            self.byte = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ZpDecoder;

    // Lines 34-35: ZpEncoder::default() delegates to ::new()
    #[test]
    fn default_is_same_as_new() {
        let enc = ZpEncoder::default();
        let compressed = enc.finish();
        let enc2 = ZpEncoder::new();
        let compressed2 = enc2.finish();
        assert_eq!(compressed, compressed2);
    }

    #[test]
    fn zp_roundtrip_passthrough_false() {
        let mut enc = ZpEncoder::new();
        for _ in 0..100 {
            enc.encode_passthrough(false);
        }
        let compressed = enc.finish();
        assert!(!compressed.is_empty());

        let mut dec = ZpDecoder::new(&compressed).expect("init");
        for i in 0..100 {
            let got = dec.decode_passthrough();
            assert!(!got, "expected false at bit {i}");
        }
    }

    #[test]
    fn zp_roundtrip_passthrough_true() {
        let mut enc = ZpEncoder::new();
        for _ in 0..100 {
            enc.encode_passthrough(true);
        }
        let compressed = enc.finish();
        assert!(!compressed.is_empty());

        let mut dec = ZpDecoder::new(&compressed).expect("init");
        for i in 0..100 {
            let got = dec.decode_passthrough();
            assert!(got, "expected true at bit {i}");
        }
    }

    #[test]
    fn zp_roundtrip_context_all_mps() {
        let n = 200;
        let mut enc = ZpEncoder::new();
        let mut ctx = 0u8;
        for _ in 0..n {
            enc.encode_bit(&mut ctx, false);
        }
        let compressed = enc.finish();
        let mut dec = ZpDecoder::new(&compressed).expect("init");
        let mut dec_ctx = 0u8;
        for i in 0..n {
            let got = dec.decode_bit(&mut dec_ctx);
            assert!(!got, "all-MPS mismatch at bit {i}");
        }
    }

    #[test]
    fn zp_roundtrip_context_all_lps() {
        let n = 200;
        let mut enc = ZpEncoder::new();
        let mut ctx = 0u8;
        for _ in 0..n {
            enc.encode_bit(&mut ctx, true);
        }
        let compressed = enc.finish();
        let mut dec = ZpDecoder::new(&compressed).expect("init");
        let mut dec_ctx = 0u8;
        for i in 0..n {
            let got = dec.decode_bit(&mut dec_ctx);
            assert!(got, "all-LPS mismatch at bit {i}");
        }
    }

    #[test]
    fn zp_roundtrip_context_bits() {
        let mut rng: u64 = 0xdead_beef;
        let n = 2000;
        let mut bits = Vec::with_capacity(n);
        let mut enc = ZpEncoder::new();
        let mut ctx = 0u8;
        for _ in 0..n {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let bit = (rng & 1) != 0;
            bits.push(bit);
            enc.encode_bit(&mut ctx, bit);
        }
        let compressed = enc.finish();
        let mut dec = ZpDecoder::new(&compressed).expect("init");
        let mut dec_ctx = 0u8;
        for (i, &expected) in bits.iter().enumerate() {
            let got = dec.decode_bit(&mut dec_ctx);
            assert_eq!(got, expected, "mismatch at bit {i}");
        }
    }

    #[test]
    fn zp_roundtrip_mixed() {
        let mut enc = ZpEncoder::new();
        let mut ctx = [0u8; 2];
        let mut seq: Vec<(bool, bool)> = Vec::new();

        for i in 0..500 {
            let is_pt = i % 5 == 0;
            let bit = (i * 13 + 7) % 3 != 0;
            seq.push((is_pt, bit));
            if is_pt {
                enc.encode_passthrough(bit);
            } else {
                enc.encode_bit(&mut ctx[i % 2], bit);
            }
        }
        let compressed = enc.finish();

        let mut dec = ZpDecoder::new(&compressed).expect("init");
        let mut dec_ctx = [0u8; 2];
        for (i, &(is_pt, expected)) in seq.iter().enumerate() {
            let got = if is_pt {
                dec.decode_passthrough()
            } else {
                dec.decode_bit(&mut dec_ctx[i % 2])
            };
            assert_eq!(got, expected, "mismatch at step {i} (pt={is_pt})");
        }
    }

    #[test]
    fn zp_roundtrip_multiple_contexts() {
        let mut rng: u64 = 42;
        let n = 1000;
        let nctx = 4;
        let mut bits = Vec::with_capacity(n);
        let mut enc = ZpEncoder::new();
        let mut ctx = vec![0u8; nctx];

        for i in 0..n {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let bit = (rng & 1) != 0;
            bits.push((i % nctx, bit));
            enc.encode_bit(&mut ctx[i % nctx], bit);
        }
        let compressed = enc.finish();

        let mut dec = ZpDecoder::new(&compressed).expect("init");
        let mut dec_ctx = vec![0u8; nctx];
        for (i, &(ci, expected)) in bits.iter().enumerate() {
            let got = dec.decode_bit(&mut dec_ctx[ci]);
            assert_eq!(got, expected, "mismatch at bit {i} ctx {ci}");
        }
    }

    /// `encode_run` must produce the same bytes and context state as one
    /// `encode_bit` per bit: runs of both values, short and long, mixed with
    /// single bits in a second context.
    #[test]
    fn encode_run_matches_encode_bit() {
        let mut rng: u32 = 0x2545_f491;
        let mut next = || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            rng
        };
        let (mut run_enc, mut bit_enc) = (ZpEncoder::new(), ZpEncoder::new());
        let (mut run_ctx, mut bit_ctx) = ([0u8; 2], [0u8; 2]);
        for _ in 0..3000 {
            let r = next();
            let bit = r % 7 == 0;
            let n = match r % 5 {
                0 => 0,
                1 => 1,
                2 => (r >> 8) as usize % 16,
                _ => (r >> 8) as usize % 70_000,
            };
            run_enc.encode_run(&mut run_ctx[0], bit, n);
            for _ in 0..n {
                bit_enc.encode_bit(&mut bit_ctx[0], bit);
            }
            assert_eq!(run_ctx, bit_ctx);
            let other = r & 0x100 != 0;
            run_enc.encode_bit(&mut run_ctx[1], other);
            bit_enc.encode_bit(&mut bit_ctx[1], other);
        }
        assert_eq!(run_enc.finish(), bit_enc.finish());
    }
}
