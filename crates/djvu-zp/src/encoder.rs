//! ZP adaptive binary arithmetic encoder.
//!
//! Encoding counterpart to [`super::ZpDecoder`]. Produces byte streams
//! that the decoder can consume. Matches DjVuLibre's ZPCodec encoder.

use super::tables::{LPS_NEXT, MPS_NEXT, PROB, THRESHOLD};

/// ZP adaptive binary arithmetic encoder.
///
/// `a` and `subend` are `u32` to match DjVuLibre's `unsigned int` types.
/// They hold u16-range values, but `subend` can exceed 0xFFFF between an
/// LPS add and its shifts; that excess is the carry the next emitted bits
/// subtract.
///
/// DjVuLibre pushes each emitted bit `b = 1 - (subend >> 15)` through a
/// 24-bit buffer and a Witten–Neal–Cleary follow-bit counter. The bytes
/// that scheme writes are the plain binary value of the emitted bits,
/// with borrows applied, minus the 24-bit `0xFFFFFF` start value and the
/// all-ones tail that `finish` flushes. This encoder builds that value
/// directly: a shift of `k` bits adds `2^k - 1 - (subend >> (16 - k))` to
/// `acc`, whole bytes leave from the top, and a borrow decrements bytes
/// already written.
pub struct ZpEncoder {
    /// Current interval width — stored as u32 but logically u16 after shifts.
    a: u32,
    /// Sub-interval lower bound for bit emission — u32 for carry propagation.
    subend: u32,
    /// The newest `nacc` emitted bits; starts as the 24-bit `0xFFFFFF`.
    acc: u64,
    /// Bits held in `acc` (24..56).
    nacc: u32,
    /// Leading bytes still to drop: the 24 start bits are never written.
    skip: u32,
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
            acc: 0xffffff,
            nacc: 24,
            skip: 3,
            output: Vec::new(),
        }
    }

    /// Bytes flushed to the output buffer so far.
    ///
    /// Diagnostic accessor only — a read-only snapshot of `output.len()` mid-stream
    /// (the encoder holds the newest 32–40 bits and drops its 24-bit start
    /// value, so it is a monotonic approximation, not an exact per-bit byte
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

    /// [`encode_bit`](Self::encode_bit), always inlined, with the LPS and
    /// shifting MPS steps kept out of line.
    ///
    /// Same bytes and context state as `encode_bit`. For a loop that codes a
    /// bit per symbol (IW44 coefficients, BZZ): the fast path is one add, so
    /// the call costs more than the code it inlines. JB2 keeps `encode_bit`,
    /// where inlining measured slower.
    #[inline(always)]
    pub fn encode_bit_inline(&mut self, ctx: &mut u8, bit: bool) {
        let state = *ctx as usize;
        let mps_bit = (state & 1) != 0;
        let z = self.a + PROB[state] as u32;

        if bit != mps_bit {
            self.encode_lps_cold(ctx, z);
        } else if z >= 0x8000 {
            self.encode_mps_cold(ctx, z);
        } else {
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
            self.shift(1);
        } else {
            let z_comp = 0x10000 - z;
            self.subend += z_comp;
            self.a += z_comp;
            self.renormalize();
        }
    }

    pub fn encode_passthrough(&mut self, bit: bool) {
        let z = 0x8000 + (self.a >> 1);
        // Invariant: self.a < 0x8000, so z = 0x8000 + a/2 ∈ [0x8000, 0xC000) — always ≥ 0x8000.
        if !bit {
            // false (MPS-like): a = z, single unconditional shift
            self.a = z;
            self.shift(1);
        } else {
            // true (LPS-like): z_comp = 0x10000 - z
            let z_comp = 0x10000 - z;
            self.subend += z_comp;
            self.a += z_comp;
            self.renormalize();
        }
    }

    /// Flush the encoder and return the compressed byte stream.
    pub fn finish(mut self) -> Vec<u8> {
        // eflush: round subend up to disambiguate, then emit its one bit.
        if self.subend > 0x8000 {
            self.subend = 0x10000;
        } else if self.subend > 0 {
            self.subend = 0x8000;
        }
        if self.subend != 0 {
            self.shift(1);
        }
        // DjVuLibre then emits ones until its 24-bit buffer is all ones and
        // never writes that buffer: drop the trailing ones of the last 24
        // bits, which is what is left of the value once those are cut off.
        let t = (self.acc as u32 | 0xff00_0000).trailing_ones().min(24);
        self.acc >>= t;
        self.nacc -= t;
        while self.nacc >= 8 {
            self.nacc -= 8;
            self.put_byte((self.acc >> self.nacc) as u8);
        }
        // Pad the last byte with ones.
        if self.nacc > 0 {
            let pad = 8 - self.nacc;
            self.put_byte(((self.acc << pad) | ((1 << pad) - 1)) as u8);
        }
        // Ensure minimum 2 bytes for decoder initialization
        while self.output.len() < 2 {
            self.output.push(0xff);
        }
        self.output
    }

    #[inline(never)]
    fn encode_mps_cold(&mut self, ctx: &mut u8, z: u32) {
        self.encode_mps(ctx, z);
    }

    #[inline(never)]
    fn encode_lps_cold(&mut self, ctx: &mut u8, z: u32) {
        self.encode_lps(ctx, z);
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
        self.shift(1);
    }

    fn encode_lps(&mut self, ctx: &mut u8, z: u32) {
        // Clamp z
        let d = 0x6000 + ((z + self.a) >> 2);
        let z = z.min(d);

        *ctx = LPS_NEXT[*ctx as usize];
        let z_comp = 0x10000 - z;
        self.subend += z_comp;
        self.a += z_comp;
        self.renormalize();
    }

    /// Shift `a` and `subend` left until `a < 0x8000`, emitting one bit
    /// per shift. `a < 0x10000` here, so the shift count is the number of
    /// leading ones of `a` as a 16-bit value.
    #[inline(always)]
    fn renormalize(&mut self) {
        debug_assert!(self.a < 0x10000);
        let k = (!(self.a << 16)).leading_zeros();
        if k > 0 {
            self.shift(k);
        }
    }

    /// Shift `a` and `subend` left by `k` (1..=16) bits and emit the bits
    /// DjVuLibre's per-bit loop would: `1 - (subend >> 15)` each, i.e.
    /// `2^k - 1 - (subend >> (16 - k))` together. `subend >= 0x10000`
    /// makes that negative: a borrow from the bits already emitted.
    #[inline(always)]
    fn shift(&mut self, k: u32) {
        let term = ((1u64 << k) - 1).wrapping_sub(u64::from(self.subend >> (16 - k)));
        self.subend = (self.subend << k) & 0xffff;
        self.a = (self.a << k) & 0xffff;
        let nacc = self.nacc + k;
        let acc = (self.acc << k).wrapping_add(term);
        if acc >> nacc != 0 {
            // The sum went negative: wrap to `nacc` bits and borrow one
            // from the bytes above `acc`.
            self.borrow();
        }
        self.acc = acc & ((1u64 << nacc) - 1);
        self.nacc = nacc;
        if nacc >= 40 {
            self.flush();
        }
    }

    /// Write the top bytes of `acc`, keeping 32..40 bits.
    #[cold]
    #[inline(never)]
    fn flush(&mut self) {
        while self.nacc >= 40 {
            self.nacc -= 8;
            self.put_byte((self.acc >> self.nacc) as u8);
        }
        self.acc &= (1u64 << self.nacc) - 1;
    }

    fn put_byte(&mut self, byte: u8) {
        if self.skip > 0 {
            self.skip -= 1;
        } else {
            self.output.push(byte);
        }
    }

    /// Subtract one from the bytes written so far. A borrow past the first
    /// byte lands in the dropped start bits and has no effect.
    #[cold]
    #[inline(never)]
    fn borrow(&mut self) {
        for byte in self.output.iter_mut().rev() {
            let (v, under) = byte.overflowing_sub(1);
            *byte = v;
            if !under {
                return;
            }
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

    /// DjVuLibre's per-bit `zemit` / `outbit` encoder, kept as the
    /// reference for the batched one.
    mod reference {
        use crate::tables::{LPS_NEXT, MPS_NEXT, PROB, THRESHOLD};

        pub struct Reference {
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

        impl Reference {
            pub(super) fn new() -> Self {
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

            /// Encode one bit using an adaptive probability context.
            ///
            /// Matches DjVuLibre's inline `encoder(int bit, BitContext &ctx)`:
            /// - LPS always calls encode_lps
            /// - MPS with z >= 0x8000 calls encode_mps
            /// - MPS with z < 0x8000 takes fast path (a = z, no shift)
            pub(super) fn encode_bit(&mut self, ctx: &mut u8, bit: bool) {
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

            /// Encode one bit in IW44 passthrough mode (threshold `z = 0x8000 + 3a/8`).
            ///
            /// Counterpart to [`ZpDecoder::decode_passthrough_iw44`](crate::ZpDecoder::decode_passthrough_iw44); must produce a
            /// stream that it correctly decodes.
            pub(super) fn encode_passthrough_iw44(&mut self, bit: bool) {
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

            pub(super) fn encode_passthrough(&mut self, bit: bool) {
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
            pub(super) fn finish(mut self) -> Vec<u8> {
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
    }

    /// The batched encoder writes the same bytes as DjVuLibre's per-bit
    /// scheme: random mixes of context bits (skewed, so long MPS runs and
    /// LPS carries both occur), both passthrough modes, and short streams
    /// that end inside the 24-bit start buffer.
    #[test]
    fn batched_emit_matches_per_bit_reference() {
        let mut rng: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng
        };
        for case in 0..4000 {
            let len = match case % 4 {
                0 => (next() % 8) as usize,
                1 => (next() % 64) as usize,
                2 => (next() % 600) as usize,
                _ => (next() % 20_000) as usize,
            };
            let skew = 1 + (next() % 40) as u32;
            let (mut new, mut old) = (ZpEncoder::new(), reference::Reference::new());
            let (mut new_ctx, mut old_ctx) = ([0u8; 4], [0u8; 4]);
            for _ in 0..len {
                let r = next();
                let bit = (r >> 8) as u32 % skew == 0;
                match r % 16 {
                    0 => {
                        new.encode_passthrough(bit);
                        old.encode_passthrough(bit);
                    }
                    1 => {
                        new.encode_passthrough_iw44(bit);
                        old.encode_passthrough_iw44(bit);
                    }
                    c => {
                        let c = c as usize % 4;
                        new.encode_bit(&mut new_ctx[c], bit);
                        old.encode_bit(&mut old_ctx[c], bit);
                    }
                }
            }
            assert_eq!(new_ctx, old_ctx);
            assert_eq!(new.finish(), old.finish(), "case {case}, {len} bits");
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

    #[test]
    fn encode_bit_inline_matches_encode_bit() {
        let mut rng: u32 = 0x1b87_3593;
        let mut next = || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            rng
        };
        let (mut inl, mut bit) = (ZpEncoder::new(), ZpEncoder::new());
        let (mut inl_ctx, mut bit_ctx) = ([0u8; 8], [0u8; 8]);
        for _ in 0..200_000 {
            let r = next();
            let c = (r & 7) as usize;
            // Skewed toward 0 so contexts reach the fast MPS path.
            let b = (r >> 8) % (2 + c as u32 * 5) == 0;
            inl.encode_bit_inline(&mut inl_ctx[c], b);
            bit.encode_bit(&mut bit_ctx[c], b);
        }
        assert_eq!(inl_ctx, bit_ctx);
        assert_eq!(inl.finish(), bit.finish());
    }
}
