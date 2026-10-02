//! Band buckets, quantisation init, coefficient flags and zigzag scan tables.

// ---- Band-bucket mapping: 10 bands, each mapped to a range of buckets --------
//
// The band/quant/state-flag/zigzag definitions below are the IW44 *spec* — one
// source of truth shared between the decoder (this module) and the std-only
// [`encode`] module, which is why they are `pub(crate)` rather than private.

/// `BAND_BUCKETS[band]` = `(first_bucket, last_bucket)` inclusive.
pub(crate) const BAND_BUCKETS: [(usize, usize); 10] = [
    (0, 0),
    (1, 1),
    (2, 2),
    (3, 3),
    (4, 7),
    (8, 11),
    (12, 15),
    (16, 31),
    (32, 47),
    (48, 63),
];

/// Initial quantization step table for the low-frequency band (band 0).
pub(crate) const QUANT_LO_INIT: [u32; 16] = [
    0x004000, 0x008000, 0x008000, 0x010000, 0x010000, 0x010000, 0x010000, 0x010000, 0x010000,
    0x010000, 0x010000, 0x010000, 0x020000, 0x020000, 0x020000, 0x020000,
];

/// Initial quantization step table for high-frequency bands (bands 1–9).
pub(crate) const QUANT_HI_INIT: [u32; 10] = [
    0, 0x020000, 0x020000, 0x040000, 0x040000, 0x040000, 0x080000, 0x040000, 0x040000, 0x080000,
];

// ---- Coefficient state flags -------------------------------------------------

pub(crate) const ZERO: u8 = 1;
pub(crate) const ACTIVE: u8 = 2;
pub(crate) const NEW: u8 = 4;
pub(crate) const UNK: u8 = 8;

// ---- Zigzag scan tables ------------------------------------------------------
//
// Each coefficient index `i` (0..1024) maps to a `(row, col)` within the 32×32
// block via bit-interleaving: even bits → column, odd bits → row.

pub(crate) const fn zigzag_row(i: usize) -> u8 {
    let b1 = ((i >> 1) & 1) as u8;
    let b3 = ((i >> 3) & 1) as u8;
    let b5 = ((i >> 5) & 1) as u8;
    let b7 = ((i >> 7) & 1) as u8;
    let b9 = ((i >> 9) & 1) as u8;
    b1 * 16 + b3 * 8 + b5 * 4 + b7 * 2 + b9
}

pub(crate) const fn zigzag_col(i: usize) -> u8 {
    let b0 = (i & 1) as u8;
    let b2 = ((i >> 2) & 1) as u8;
    let b4 = ((i >> 4) & 1) as u8;
    let b6 = ((i >> 6) & 1) as u8;
    let b8 = ((i >> 8) & 1) as u8;
    b0 * 16 + b2 * 8 + b4 * 4 + b6 * 2 + b8
}

/// Inverse zigzag: `ZIGZAG_INV[row * 32 + col]` is the index `i` such that
/// `zigzag_row(i) == row as u8 && zigzag_col(i) == col as u8`.
///
/// Enables row-major scatter (sequential writes to the plane) at the cost of
/// gathering block coefficients in zigzag order (2 KB block fits in L1).
pub(super) static ZIGZAG_INV: [u16; 1024] = {
    let mut table = [0u16; 1024];
    let mut i = 0usize;
    while i < 1024 {
        let r = zigzag_row(i) as usize;
        let c = zigzag_col(i) as usize;
        table[r * 32 + c] = i as u16;
        i += 1;
    }
    table
};

/// Compact inverse zigzag for sub=2 (16×16 sub-block, 256 entries).
/// `ZIGZAG_INV_SUB2[row * 16 + col]` = index `i` in 0..256 such that
/// `zigzag_row(i) >> 1 == row && zigzag_col(i) >> 1 == col`.
pub(super) static ZIGZAG_INV_SUB2: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut i = 0usize;
    while i < 256 {
        let r = (zigzag_row(i) >> 1) as usize;
        let c = (zigzag_col(i) >> 1) as usize;
        table[r * 16 + c] = i as u8;
        i += 1;
    }
    table
};

/// Compact inverse zigzag for sub=4 (8×8 sub-block, 64 entries).
/// `ZIGZAG_INV_SUB4[row * 8 + col]` = index `i` in 0..64.
pub(super) static ZIGZAG_INV_SUB4: [u8; 64] = {
    let mut table = [0u8; 64];
    let mut i = 0usize;
    while i < 64 {
        let r = (zigzag_row(i) >> 2) as usize;
        let c = (zigzag_col(i) >> 2) as usize;
        table[r * 8 + c] = i as u8;
        i += 1;
    }
    table
};

/// Compact inverse zigzag for sub=8 (4×4 sub-block, 16 entries).
/// `ZIGZAG_INV_SUB8[row * 4 + col]` = index `i` in 0..16.
pub(super) static ZIGZAG_INV_SUB8: [u8; 16] = {
    let mut table = [0u8; 16];
    let mut i = 0usize;
    while i < 16 {
        let r = (zigzag_row(i) >> 3) as usize;
        let c = (zigzag_col(i) >> 3) as usize;
        table[r * 4 + c] = i as u8;
        i += 1;
    }
    table
};

// ---- Normalization -----------------------------------------------------------

/// Map a raw wavelet coefficient to a signed pixel offset in `[-128, 127]`.
#[inline]
pub(super) fn normalize(val: i16) -> i32 {
    let v = ((val as i32) + 32) >> 6;
    v.clamp(-128, 127)
}
