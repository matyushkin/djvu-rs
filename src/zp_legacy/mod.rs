pub mod tables;

use tables::{DN, M, P, UP};

/// FFZ lookup table: count of leading 1-bits in a byte.
static FFZT: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut i = 0u16;
    while i < 256 {
        let mut val = i as u8;
        let mut count = 0u8;
        while val & 0x80 != 0 {
            count += 1;
            val <<= 1;
        }
        table[i as usize] = count;
        i += 1;
    }
    table
};

/// Count leading 1-bits in a 16-bit value.
#[inline]
fn ffz(x: u16) -> u32 {
    if x >= 0xff00 {
        FFZT[(x & 0xff) as usize] as u32 + 8
    } else {
        FFZT[(x >> 8) as usize] as u32
    }
}

/// ZP adaptive binary arithmetic decoder.
pub struct ZPDecoder<'a> {
    data: &'a [u8],
    pos: usize,
    a: u16,
    c: u16,
    f: u16,
    buffer: u32,
    scount: i32,
}

impl<'a> ZPDecoder<'a> {
    /// Create a new ZP decoder from input bytes.
    pub fn new(data: &'a [u8]) -> Self {
        let mut dec = ZPDecoder {
            data,
            pos: 0,
            a: 0,
            c: 0,
            f: 0,
            buffer: 0,
            scount: 0,
        };
        // Initialize code register from first two bytes
        let b0 = dec.read_byte() as u16;
        let b1 = dec.read_byte() as u16;
        dec.c = (b0 << 8) | b1;
        // Preload buffer
        dec.preload();
        // Set fence
        dec.f = dec.c.min(0x7fff);
        dec
    }

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

    #[inline(always)]
    fn preload(&mut self) {
        while self.scount <= 24 {
            let byte = self.read_byte();
            self.buffer = (self.buffer << 8) | byte as u32;
            self.scount += 8;
        }
    }

    /// Decode one bit using an adaptive context.
    /// `ctx` is a mutable reference to a context state byte.
    #[inline(always)]
    pub fn decode(&mut self, ctx: &mut u8) -> bool {
        let st = *ctx as usize;
        let b_mps = st & 1;
        // z can exceed u16 range (a + P[st] may be > 0xFFFF), so use u32
        let z = self.a as u32 + P[st] as u32;

        // Fast path: no renormalization needed
        if z <= self.f as u32 {
            self.a = z as u16;
            return b_mps != 0;
        }

        // Compute decision boundary
        let d = 0x6000u32 + ((self.a as u32 + z) >> 2);
        let z_clamped = if z > d { d } else { z };

        if z_clamped > self.c as u32 {
            // LPS path
            let b = 1 - b_mps;
            let z_comp = 0x10000u32 - z_clamped;
            self.a = self.a.wrapping_add(z_comp as u16);
            self.c = self.c.wrapping_add(z_comp as u16);
            *ctx = DN[st];

            // Renormalize
            let shift = ffz(self.a);
            self.scount -= shift as i32;
            self.a = ((self.a as u32) << shift) as u16;
            let mask = (1u32 << shift) - 1;
            self.c =
                ((self.c as u32) << shift | ((self.buffer >> self.scount as u32) & mask)) as u16;

            if self.scount < 16 {
                self.preload();
            }
            self.f = self.c.min(0x7fff);
            b != 0
        } else {
            // MPS path
            if self.a >= M[st] {
                *ctx = UP[st];
            }

            self.scount -= 1;
            self.a = (z_clamped << 1) as u16;
            self.c = ((self.c as u32) << 1 | ((self.buffer >> self.scount as u32) & 1)) as u16;

            if self.scount < 16 {
                self.preload();
            }
            self.f = self.c.min(0x7fff);
            b_mps != 0
        }
    }

    /// Decode one bit without adaptive context (standard passthrough).
    /// Uses threshold z = 0x8000 + (a >> 1).
    /// This is used by BZZ's decode_raw.
    #[inline(always)]
    pub fn decode_passthrough(&mut self) -> bool {
        let z = 0x8000u16.wrapping_add(self.a >> 1);
        self.decode_passthrough_with_z(z)
    }

    /// Decode one bit without adaptive context (IW44 variant).
    /// Uses threshold z = 0x8000 + (3*a >> 3).
    /// This is used by IW44 image decoding.
    #[inline(always)]
    pub fn decode_iw(&mut self) -> bool {
        let z = (0x8000u32 + (3u32 * self.a as u32) / 8) as u16;
        self.decode_passthrough_with_z(z)
    }

    #[inline(always)]
    fn decode_passthrough_with_z(&mut self, z: u16) -> bool {
        if z > self.c {
            // Bit is 1
            let z_comp = 0x10000u32 - z as u32;
            self.a = self.a.wrapping_add(z_comp as u16);
            self.c = self.c.wrapping_add(z_comp as u16);

            let shift = ffz(self.a);
            self.scount -= shift as i32;
            self.a = ((self.a as u32) << shift) as u16;
            let mask = (1u32 << shift) - 1;
            self.c =
                ((self.c as u32) << shift | ((self.buffer >> self.scount as u32) & mask)) as u16;

            if self.scount < 16 {
                self.preload();
            }
            self.f = self.c.min(0x7fff);
            true
        } else {
            // Bit is 0
            self.scount -= 1;
            self.a = z.wrapping_mul(2);
            self.c = ((self.c as u32) << 1 | ((self.buffer >> self.scount as u32) & 1)) as u16;

            if self.scount < 16 {
                self.preload();
            }
            self.f = self.c.min(0x7fff);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffz_table() {
        assert_eq!(FFZT[0x00], 0); // 00000000 → 0 leading 1s
        assert_eq!(FFZT[0x80], 1); // 10000000 → 1
        assert_eq!(FFZT[0xC0], 2); // 11000000 → 2
        assert_eq!(FFZT[0xFE], 7); // 11111110 → 7
        assert_eq!(FFZT[0xFF], 8); // 11111111 → 8
    }

    #[test]
    fn ffz_function() {
        assert_eq!(ffz(0x0000), 0);
        assert_eq!(ffz(0x8000), 1);
        assert_eq!(ffz(0xC000), 2);
        assert_eq!(ffz(0xFF00), 8);
        assert_eq!(ffz(0xFF80), 9);
        assert_eq!(ffz(0xFFFF), 16);
    }

    #[test]
    fn tables_have_correct_length() {
        assert_eq!(P.len(), 251);
        assert_eq!(M.len(), 251);
        assert_eq!(UP.len(), 251);
        assert_eq!(DN.len(), 251);
    }

    #[test]
    fn table_spot_checks() {
        // First 3 entries of P are 0x8000
        assert_eq!(P[0], 0x8000);
        assert_eq!(P[1], 0x8000);
        assert_eq!(P[2], 0x8000);
        assert_eq!(P[3], 0x6bbd);
        // Last entries
        assert_eq!(P[250], 0x481a);
        // UP[0] = 84
        assert_eq!(UP[0], 84);
        // DN[0] = 145
        assert_eq!(DN[0], 145);
        // M entries past 82 are 0
        assert_eq!(M[83], 0);
        assert_eq!(M[250], 0);
    }
}
