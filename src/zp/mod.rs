//! ZP adaptive binary arithmetic coder — pure-Rust clean-room implementation.
//!
//! This module implements the ZP (Z-Prime) adaptive binary arithmetic decoder
//! from the DjVu v3 specification (<https://www.sndjvu.org/spec.html>).
//!
//! Key public types:
//! - [`ZpDecoder`] — the ZP decoder state machine

#[cfg(feature = "std")]
pub(crate) mod encoder;
pub(crate) mod tables;

use crate::error::BzzError;
use tables::{LPS_NEXT, MPS_NEXT, PROB, THRESHOLD};

/// Count of leading 1-bits in each possible byte value.
///
/// `LEADING_ONES[x]` gives the number of leading 1-bits in `x`.
static LEADING_ONES: [u8; 256] = {
    let mut tbl = [0u8; 256];
    let mut i = 0u16;
    while i < 256 {
        let mut val = i as u8;
        let mut count = 0u8;
        while val & 0x80 != 0 {
            count += 1;
            val <<= 1;
        }
        tbl[i as usize] = count;
        i += 1;
    }
    tbl
};

/// Count leading 1-bits in a 16-bit value.
///
/// Returns the number of consecutive 1-bits starting from the most significant
/// bit. Used to determine the shift amount during renormalization.
#[inline(always)]
fn count_leading_ones(x: u16) -> u32 {
    if x >= 0xff00 {
        LEADING_ONES[(x & 0xff) as usize] as u32 + 8
    } else {
        LEADING_ONES[(x >> 8) as usize] as u32
    }
}

/// ZP (Z-Prime) adaptive binary arithmetic decoder.
///
/// Implements the decoder described in the DjVu v3 specification. The decoder
/// maintains a probability model for each context and adapts the model as bits
/// are decoded.
///
/// Context bytes encode both the probability state index and the current MPS
/// (most probable symbol) value. The low bit of the context byte indicates
/// the current MPS; the remaining bits encode the probability state.
pub(crate) struct ZpDecoder<'a> {
    /// Current interval width register.
    a: u16,
    /// Current code (value within the interval) register.
    c: u16,
    /// Cached upper bound for the fast decode path (= min(c, 0x7fff)).
    fence: u16,
    /// Bit buffer for feeding bits into the code register.
    bit_buf: u32,
    /// Number of valid bits remaining in `bit_buf`.
    bit_count: i32,
    /// Compressed input bytes.
    data: &'a [u8],
    /// Current read position within `data`.
    pos: usize,
}

impl<'a> ZpDecoder<'a> {
    /// Construct a new ZP decoder from the given compressed byte slice.
    ///
    /// Reads the initial code register from the first two bytes of `data`.
    ///
    /// # Errors
    ///
    /// Returns [`BzzError::TooShort`] if `data` has fewer than 2 bytes.
    pub(crate) fn new(data: &'a [u8]) -> Result<Self, BzzError> {
        if data.len() < 2 {
            return Err(BzzError::TooShort);
        }

        let mut dec = ZpDecoder {
            a: 0,
            c: 0,
            fence: 0,
            bit_buf: 0,
            bit_count: 0,
            data,
            pos: 0,
        };

        // Load the initial code register from the first two bytes
        let high = dec.read_byte() as u16;
        let low = dec.read_byte() as u16;
        dec.c = (high << 8) | low;

        // Pre-fill the bit buffer
        dec.refill_buffer();

        // Initialise the fence
        dec.fence = dec.c.min(0x7fff);

        Ok(dec)
    }

    /// Read the next byte from the input stream, returning `0xFF` on exhaustion.
    #[inline(always)]
    fn read_byte(&mut self) -> u8 {
        if self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;
            b
        } else {
            0xff
        }
    }

    /// Fill `bit_buf` with fresh bytes until it holds at least 24 bits.
    #[inline(always)]
    fn refill_buffer(&mut self) {
        while self.bit_count <= 24 {
            let byte = self.read_byte();
            self.bit_buf = (self.bit_buf << 8) | (byte as u32);
            self.bit_count += 8;
        }
    }

    /// Decode one bit using an adaptive probability context.
    ///
    /// `ctx` is a mutable context byte encoding the current probability state
    /// and MPS value. It is updated in-place after each call.
    ///
    /// Returns `true` if the decoded bit is 1.
    #[inline(always)]
    pub(crate) fn decode_bit(&mut self, ctx: &mut u8) -> bool {
        let state = *ctx as usize;
        let mps_bit = state & 1; // low bit encodes the current MPS
        // Compute updated interval (may overflow u16)
        let z = self.a as u32 + PROB[state] as u32;

        // Fast path: interval stays within the fence — no renormalization needed
        if z <= self.fence as u32 {
            self.a = z as u16;
            return mps_bit != 0;
        }

        // Clamp to the decision boundary
        let boundary = 0x6000u32 + ((self.a as u32 + z) >> 2);
        let z_clamped = z.min(boundary);

        if z_clamped > self.c as u32 {
            // LPS event: decoded bit is opposite of MPS
            let lps_bit = 1 - mps_bit;
            let complement = 0x10000u32 - z_clamped;
            self.a = self.a.wrapping_add(complement as u16);
            self.c = self.c.wrapping_add(complement as u16);
            *ctx = LPS_NEXT[state];
            self.renormalize();
            lps_bit != 0
        } else {
            // MPS event: decoded bit matches MPS
            if self.a >= THRESHOLD[state] {
                *ctx = MPS_NEXT[state];
            }
            self.bit_count -= 1;
            self.a = (z_clamped << 1) as u16;
            self.c = ((self.c as u32) << 1 | ((self.bit_buf >> self.bit_count as u32) & 1)) as u16;
            if self.bit_count < 16 {
                self.refill_buffer();
            }
            self.fence = self.c.min(0x7fff);
            mps_bit != 0
        }
    }

    /// Returns `true` once all real input bytes have been consumed.
    ///
    /// After exhaustion the coder returns `0xFF` bytes indefinitely, producing
    /// deterministic but meaningless bits. Callers may use this to skip
    /// remaining work that would otherwise loop on constant input.
    pub(crate) fn is_exhausted(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Decode one bit in passthrough (context-free) mode.
    ///
    /// Used by BZZ to decode raw integer values (block size, BWT index).
    /// The threshold is `z = 0x8000 + (a >> 1)`.
    ///
    /// Returns `true` if the decoded bit is 1.
    #[inline(always)]
    pub(crate) fn decode_passthrough(&mut self) -> bool {
        let z = 0x8000u16.wrapping_add(self.a >> 1);
        self.passthrough_with_threshold(z)
    }

    /// Decode one bit in passthrough mode using the IW44 variant threshold.
    ///
    /// The threshold is `z = 0x8000 + (3 * a / 8)`.
    #[inline(always)]
    pub(crate) fn decode_passthrough_iw44(&mut self) -> bool {
        let z = (0x8000u32 + (3u32 * self.a as u32) / 8) as u16;
        self.passthrough_with_threshold(z)
    }

    /// Internal passthrough decode with an explicit threshold `z`.
    #[inline(always)]
    fn passthrough_with_threshold(&mut self, z: u16) -> bool {
        if z > self.c {
            // Bit is 1
            let complement = 0x10000u32 - z as u32;
            self.a = self.a.wrapping_add(complement as u16);
            self.c = self.c.wrapping_add(complement as u16);
            self.renormalize();
            true
        } else {
            // Bit is 0
            self.bit_count -= 1;
            self.a = z.wrapping_mul(2);
            self.c = ((self.c as u32) << 1 | ((self.bit_buf >> self.bit_count as u32) & 1)) as u16;
            if self.bit_count < 16 {
                self.refill_buffer();
            }
            self.fence = self.c.min(0x7fff);
            false
        }
    }

    /// Renormalize after an LPS event.
    ///
    /// Shifts the interval register left until `a >= 0x8000`, pulling fresh bits
    /// into `c` from the bit buffer.
    #[inline(always)]
    fn renormalize(&mut self) {
        let shift = count_leading_ones(self.a);
        self.bit_count -= shift as i32;
        self.a = ((self.a as u32) << shift) as u16;
        let mask = (1u32 << shift) - 1;
        self.c =
            ((self.c as u32) << shift | ((self.bit_buf >> self.bit_count as u32) & mask)) as u16;
        if self.bit_count < 16 {
            self.refill_buffer();
        }
        self.fence = self.c.min(0x7fff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::BzzError;

    #[test]
    fn zp_decoder_rejects_empty_input() {
        assert!(matches!(ZpDecoder::new(&[]), Err(BzzError::TooShort)));
    }

    #[test]
    fn zp_decoder_rejects_one_byte_input() {
        assert!(matches!(ZpDecoder::new(&[0x00]), Err(BzzError::TooShort)));
    }

    #[test]
    fn zp_decoder_accepts_two_byte_input() {
        assert!(ZpDecoder::new(&[0x00, 0x00]).is_ok());
        assert!(ZpDecoder::new(&[0xff, 0xff]).is_ok());
    }

    #[test]
    fn leading_ones_table_spot_checks() {
        assert_eq!(LEADING_ONES[0x00], 0); // 00000000 → 0 leading 1s
        assert_eq!(LEADING_ONES[0x80], 1); // 10000000 → 1 leading 1
        assert_eq!(LEADING_ONES[0xC0], 2); // 11000000 → 2 leading 1s
        assert_eq!(LEADING_ONES[0xFE], 7); // 11111110 → 7 leading 1s
        assert_eq!(LEADING_ONES[0xFF], 8); // 11111111 → 8 leading 1s
    }

    #[test]
    fn count_leading_ones_spot_checks() {
        assert_eq!(count_leading_ones(0x0000), 0);
        assert_eq!(count_leading_ones(0x8000), 1);
        assert_eq!(count_leading_ones(0xC000), 2);
        assert_eq!(count_leading_ones(0xFF00), 8);
        assert_eq!(count_leading_ones(0xFF80), 9);
        assert_eq!(count_leading_ones(0xFFFF), 16);
    }

    #[test]
    fn zp_tables_spot_check() {
        // These values are from the DjVu v3 spec
        assert_eq!(PROB[0], 0x8000);
        assert_eq!(PROB[250], 0x481a);
        assert_eq!(MPS_NEXT[0], 84);
        assert_eq!(LPS_NEXT[0], 145);
        assert_eq!(THRESHOLD[83], 0);
        assert_eq!(THRESHOLD[250], 0);
    }
}
