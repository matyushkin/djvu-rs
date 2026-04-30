//! ZP adaptive binary arithmetic coder — pure-Rust clean-room implementation.
//!
//! This crate implements the ZP (Z-Prime) adaptive binary arithmetic coder
//! from the DjVu v3 specification (<https://www.sndjvu.org/spec.html>),
//! used by the JB2, IW44, and BZZ codecs that make up a DjVu file.
//!
//! ## Usage
//!
//! Decoder (no_std-capable, no allocations):
//!
//! ```
//! use djvu_zp::ZpDecoder;
//! let compressed: &[u8] = &[0x00, 0x00];
//! let mut dec = ZpDecoder::new(compressed)?;
//! let mut ctx = 0u8;
//! let _bit = dec.decode_bit(&mut ctx);
//! # Ok::<(), djvu_zp::ZpError>(())
//! ```
//!
//! Encoder (requires `std` feature, default-on):
//!
//! ```
//! # #[cfg(feature = "std")]
//! # {
//! use djvu_zp::encoder::ZpEncoder;
//! let mut enc = ZpEncoder::new();
//! let mut ctx = 0u8;
//! enc.encode_bit(&mut ctx, true);
//! let _bytes: Vec<u8> = enc.finish();
//! # }
//! ```
//!
//! ## Features
//!
//! - `std` (default) — enables [`encoder::ZpEncoder`].  The decoder works
//!   with or without `std` and never allocates.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
pub mod encoder;
pub mod tables;

use tables::{LPS_NEXT, MPS_NEXT, PROB, THRESHOLD};

/// Errors that can occur while initializing or decoding a ZP stream.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ZpError {
    /// Input is too short — the ZP coder needs at least 2 bytes to load the
    /// initial code register.
    #[error("ZP input is too short (need at least 2 bytes)")]
    TooShort,
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
pub struct ZpDecoder<'a> {
    /// Current interval width register (16-bit value held in low 16 bits).
    pub a: u32,
    /// Current code (value within the interval) register (16-bit value held in low 16 bits).
    pub c: u32,
    /// Cached upper bound for the fast decode path (= min(c, 0x7fff)).
    pub fence: u32,
    /// Bit buffer for feeding bits into the code register.
    pub bit_buf: u32,
    /// Number of valid bits remaining in `bit_buf`.
    pub bit_count: i32,
    /// Compressed input bytes.
    pub data: &'a [u8],
    /// Current read position within `data`.
    pub pos: usize,
}

impl<'a> ZpDecoder<'a> {
    /// Construct a new ZP decoder from the given compressed byte slice.
    ///
    /// Reads the initial code register from the first two bytes of `data`.
    ///
    /// # Errors
    ///
    /// Returns [`ZpError::TooShort`] if `data` has fewer than 2 bytes.
    pub fn new(data: &'a [u8]) -> Result<Self, ZpError> {
        if data.len() < 2 {
            return Err(ZpError::TooShort);
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
        let high = dec.read_byte() as u32;
        let low = dec.read_byte() as u32;
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
    pub fn decode_bit(&mut self, ctx: &mut u8) -> bool {
        let state = *ctx as usize;
        let mps_bit = state & 1; // low bit encodes the current MPS
        let z = self.a + PROB[state] as u32;

        // Fast path: interval stays within the fence — no renormalization needed
        if z <= self.fence {
            self.a = z;
            return mps_bit != 0;
        }

        // Clamp to the decision boundary
        let boundary = 0x6000u32 + ((self.a + z) >> 2);
        let z_clamped = z.min(boundary);

        if z_clamped > self.c {
            // LPS event: decoded bit is opposite of MPS
            let lps_bit = 1 - mps_bit;
            let complement = 0x10000u32 - z_clamped;
            self.a = (self.a + complement) & 0xffff;
            self.c = (self.c + complement) & 0xffff;
            *ctx = LPS_NEXT[state];
            self.renormalize();
            lps_bit != 0
        } else {
            // MPS event: decoded bit matches MPS
            if self.a >= THRESHOLD[state] as u32 {
                *ctx = MPS_NEXT[state];
            }
            self.bit_count -= 1;
            self.a = (z_clamped << 1) & 0xffff;
            self.c = (self.c << 1 | (self.bit_buf >> self.bit_count as u32) & 1) & 0xffff;
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
    pub fn is_exhausted(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Decode one bit in passthrough (context-free) mode.
    ///
    /// Used by BZZ to decode raw integer values (block size, BWT index).
    /// The threshold is `z = 0x8000 + (a >> 1)`.
    ///
    /// Returns `true` if the decoded bit is 1.
    #[inline(always)]
    pub fn decode_passthrough(&mut self) -> bool {
        let z = (0x8000u32 + (self.a >> 1)) as u16;
        self.passthrough_with_threshold(z)
    }

    /// Decode one bit in IW44 passthrough mode.
    ///
    /// The threshold is `z = 0x8000 + (3 * a / 8)`.
    ///
    /// Returns `true` if the decoded bit is 1.
    #[inline(always)]
    pub fn decode_passthrough_iw44(&mut self) -> bool {
        let z = (0x8000u32 + (3u32 * self.a) / 8) as u16;
        self.passthrough_with_threshold(z)
    }

    /// Internal passthrough decode with an explicit threshold `z`.
    #[inline(always)]
    fn passthrough_with_threshold(&mut self, z: u16) -> bool {
        if z as u32 > self.c {
            // Bit is 1
            let complement = 0x10000u32 - z as u32;
            self.a = (self.a + complement) & 0xffff;
            self.c = (self.c + complement) & 0xffff;
            self.renormalize();
            true
        } else {
            // Bit is 0
            self.bit_count -= 1;
            self.a = (z as u32 * 2) & 0xffff;
            self.c = (self.c << 1 | (self.bit_buf >> self.bit_count as u32) & 1) & 0xffff;
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
        let shift = (self.a as u16).leading_ones();
        self.bit_count -= shift as i32;
        self.a = (self.a << shift) & 0xffff;
        let mask = (1u32 << shift) - 1;
        self.c = ((self.c << shift) | (self.bit_buf >> self.bit_count as u32) & mask) & 0xffff;
        if self.bit_count < 16 {
            self.refill_buffer();
        }
        self.fence = self.c.min(0x7fff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zp_decoder_rejects_empty_input() {
        assert!(matches!(ZpDecoder::new(&[]), Err(ZpError::TooShort)));
    }

    #[test]
    fn zp_decoder_rejects_one_byte_input() {
        assert!(matches!(ZpDecoder::new(&[0x00]), Err(ZpError::TooShort)));
    }

    #[test]
    fn zp_decoder_accepts_two_byte_input() {
        assert!(ZpDecoder::new(&[0x00, 0x00]).is_ok());
        assert!(ZpDecoder::new(&[0xff, 0xff]).is_ok());
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
