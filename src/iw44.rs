use crate::pixmap::Pixmap;
use crate::zp_impl::ZpDecoder;

/// Errors that can occur during IW44 decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// A chunk is too short to contain the required header fields.
    ChunkTooShort,
    /// The first chunk header is too short (needs at least 9 bytes).
    HeaderTooShort,
    /// Image width or height is zero.
    ZeroDimension,
    /// Image dimensions exceed the safety limit (~256M pixels).
    ImageTooLarge,
    /// A subsequent chunk was encountered before the first chunk.
    MissingFirstChunk,
    /// The subsample parameter must be >= 1.
    InvalidSubsample,
    /// No codec has been initialized (no chunks decoded yet).
    MissingCodec,
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeError::ChunkTooShort => write!(f, "IW44: chunk too short"),
            DecodeError::HeaderTooShort => write!(f, "IW44: first chunk header too short"),
            DecodeError::ZeroDimension => write!(f, "IW44: zero dimension"),
            DecodeError::ImageTooLarge => write!(f, "IW44: image dimensions too large"),
            DecodeError::MissingFirstChunk => {
                write!(f, "IW44: subsequent chunk before first chunk")
            }
            DecodeError::InvalidSubsample => write!(f, "IW44: subsample must be >= 1"),
            DecodeError::MissingCodec => write!(f, "IW44: no codec initialized"),
        }
    }
}

impl std::error::Error for DecodeError {}

// Band-to-bucket mapping: (from, to) inclusive
const BAND_BUCKETS: [(usize, usize); 10] = [
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

const QUANT_LO_INIT: [u32; 16] = [
    0x004000, 0x008000, 0x008000, 0x010000, 0x010000, 0x010000, 0x010000, 0x010000, 0x010000,
    0x010000, 0x010000, 0x010000, 0x020000, 0x020000, 0x020000, 0x020000,
];

const QUANT_HI_INIT: [u32; 10] = [
    0, 0x020000, 0x020000, 0x040000, 0x040000, 0x040000, 0x080000, 0x040000, 0x040000, 0x080000,
];

// Coefficient state flags
const ZERO: u8 = 1;
const ACTIVE: u8 = 2;
const NEW: u8 = 4;
const UNK: u8 = 8;

// Zigzag mapping: coefficient index (0..1023) → (row, col) within 32×32 block
// Derived from interleaved bit-reversal:
//   col = bit0*16 + bit2*8 + bit4*4 + bit6*2 + bit8
//   row = bit1*16 + bit3*8 + bit5*4 + bit7*2 + bit9
const fn zigzag_row(i: usize) -> u8 {
    let b1 = ((i >> 1) & 1) as u8;
    let b3 = ((i >> 3) & 1) as u8;
    let b5 = ((i >> 5) & 1) as u8;
    let b7 = ((i >> 7) & 1) as u8;
    let b9 = ((i >> 9) & 1) as u8;
    b1 * 16 + b3 * 8 + b5 * 4 + b7 * 2 + b9
}

const fn zigzag_col(i: usize) -> u8 {
    let b0 = (i & 1) as u8;
    let b2 = ((i >> 2) & 1) as u8;
    let b4 = ((i >> 4) & 1) as u8;
    let b6 = ((i >> 6) & 1) as u8;
    let b8 = ((i >> 8) & 1) as u8;
    b0 * 16 + b2 * 8 + b4 * 4 + b6 * 2 + b8
}

static ZIGZAG_ROW: [u8; 1024] = {
    let mut table = [0u8; 1024];
    let mut i = 0;
    while i < 1024 {
        table[i] = zigzag_row(i);
        i += 1;
    }
    table
};

static ZIGZAG_COL: [u8; 1024] = {
    let mut table = [0u8; 1024];
    let mut i = 0;
    while i < 1024 {
        table[i] = zigzag_col(i);
        i += 1;
    }
    table
};

fn normalize(val: i16) -> i32 {
    let v = ((val as i32) + 32) >> 6;
    v.clamp(-128, 127)
}

/// Per-channel wavelet decoder. Holds block coefficients and progressive decoding state.
struct IWDecoder {
    width: usize,
    height: usize,
    block_cols: usize,
    blocks: Vec<[i16; 1024]>,
    quant_lo: [u32; 16],
    quant_hi: [u32; 10],
    curband: usize,
    // ZP contexts (persistent across slices)
    decode_bucket_ctx: [u8; 1],
    decode_coef_ctx: [u8; 80],
    activate_coef_ctx: [u8; 16],
    increase_coef_ctx: [u8; 1],
    // Per-block temporary state
    coeffstate: [[u8; 16]; 16],
    bucketstate: [u8; 16],
    bbstate: u8,
}

impl IWDecoder {
    fn new(width: usize, height: usize) -> Self {
        let block_cols = width.div_ceil(32);
        let block_rows = height.div_ceil(32);
        let block_count = block_cols * block_rows;
        IWDecoder {
            width,
            height,
            block_cols,
            blocks: vec![[0i16; 1024]; block_count],
            quant_lo: QUANT_LO_INIT,
            quant_hi: QUANT_HI_INIT,
            curband: 0,
            decode_bucket_ctx: [0; 1],
            decode_coef_ctx: [0; 80],
            activate_coef_ctx: [0; 16],
            increase_coef_ctx: [0; 1],
            coeffstate: [[0; 16]; 16],
            bucketstate: [0; 16],
            bbstate: 0,
        }
    }

    fn decode_slice(&mut self, zp: &mut ZpDecoder) {
        if !self.is_null_slice() {
            for block_idx in 0..self.blocks.len() {
                self.preliminary_flag_computation(block_idx);
                if self.block_band_decoding_pass(zp) {
                    self.bucket_decoding_pass(zp, block_idx);
                    self.newly_active_coefficient_decoding_pass(zp, block_idx);
                }
                self.previously_active_coefficient_decoding_pass(zp, block_idx);
            }
        }
        self.finish_code_slice();
    }

    fn is_null_slice(&mut self) -> bool {
        if self.curband == 0 {
            let mut is_null = true;
            for i in 0..16 {
                let threshold = self.quant_lo[i];
                self.coeffstate[0][i] = ZERO;
                if threshold > 0 && threshold < 0x8000 {
                    self.coeffstate[0][i] = UNK;
                    is_null = false;
                }
            }
            is_null
        } else {
            let threshold = self.quant_hi[self.curband];
            !(threshold > 0 && threshold < 0x8000)
        }
    }

    fn preliminary_flag_computation(&mut self, block_idx: usize) {
        self.bbstate = 0;
        let (from, to) = BAND_BUCKETS[self.curband];

        if self.curband != 0 {
            for (boff, j) in (from..=to).enumerate() {
                let mut bstatetmp: u8 = 0;
                for k in 0..16 {
                    if self.blocks[block_idx][(j << 4) | k] == 0 {
                        self.coeffstate[boff][k] = UNK;
                    } else {
                        self.coeffstate[boff][k] = ACTIVE;
                    }
                    bstatetmp |= self.coeffstate[boff][k];
                }
                self.bucketstate[boff] = bstatetmp;
                self.bbstate |= bstatetmp;
            }
        } else {
            let mut bstatetmp: u8 = 0;
            for k in 0..16 {
                if self.coeffstate[0][k] != ZERO {
                    if self.blocks[block_idx][k] == 0 {
                        self.coeffstate[0][k] = UNK;
                    } else {
                        self.coeffstate[0][k] = ACTIVE;
                    }
                }
                bstatetmp |= self.coeffstate[0][k];
            }
            self.bucketstate[0] = bstatetmp;
            self.bbstate |= bstatetmp;
        }
    }

    fn block_band_decoding_pass(&mut self, zp: &mut ZpDecoder) -> bool {
        let (from, to) = BAND_BUCKETS[self.curband];
        let bcount = to - from + 1;
        let should_mark_new = bcount < 16
            || (self.bbstate & ACTIVE) != 0
            || ((self.bbstate & UNK) != 0 && zp.decode_bit(&mut self.decode_bucket_ctx[0]));
        if should_mark_new {
            self.bbstate |= NEW;
        }
        (self.bbstate & NEW) != 0
    }

    fn bucket_decoding_pass(&mut self, zp: &mut ZpDecoder, block_idx: usize) {
        let (from, to) = BAND_BUCKETS[self.curband];
        for (boff, i) in (from..=to).enumerate() {
            if (self.bucketstate[boff] & UNK) == 0 {
                continue;
            }
            let mut n: usize = 0;
            if self.curband != 0 {
                let t = 4 * i;
                for j in t..t + 4 {
                    if self.blocks[block_idx][j] != 0 {
                        n += 1;
                    }
                }
                if n == 4 {
                    n = 3;
                }
            }
            if (self.bbstate & ACTIVE) != 0 {
                n |= 4;
            }
            if zp.decode_bit(&mut self.decode_coef_ctx[n + self.curband * 8]) {
                self.bucketstate[boff] |= NEW;
            }
        }
    }

    fn newly_active_coefficient_decoding_pass(&mut self, zp: &mut ZpDecoder, block_idx: usize) {
        let (from, to) = BAND_BUCKETS[self.curband];
        let mut step = self.quant_hi[self.curband];
        for (boff, i) in (from..=to).enumerate() {
            if (self.bucketstate[boff] & NEW) != 0 {
                let shift: usize = if (self.bucketstate[boff] & ACTIVE) != 0 {
                    8
                } else {
                    0
                };
                let mut np: usize = 0;
                for j in 0..16 {
                    if (self.coeffstate[boff][j] & UNK) != 0 {
                        np += 1;
                    }
                }
                for j in 0..16 {
                    if (self.coeffstate[boff][j] & UNK) != 0 {
                        let ip = np.min(7);
                        if zp.decode_bit(&mut self.activate_coef_ctx[shift + ip]) {
                            let sign = if zp.decode_passthrough_iw44() {
                                -1i32
                            } else {
                                1i32
                            };
                            np = 0;
                            if self.curband == 0 {
                                step = self.quant_lo[j];
                            }
                            let s = step as i32;
                            let val = sign * (s + (s >> 1) - (s >> 3));
                            self.blocks[block_idx][(i << 4) | j] = val as i16;
                        }
                        np = np.saturating_sub(1);
                    }
                }
            }
        }
    }

    fn previously_active_coefficient_decoding_pass(
        &mut self,
        zp: &mut ZpDecoder,
        block_idx: usize,
    ) {
        let (from, to) = BAND_BUCKETS[self.curband];
        let mut step = self.quant_hi[self.curband];
        for (boff, i) in (from..=to).enumerate() {
            for j in 0..16 {
                if (self.coeffstate[boff][j] & ACTIVE) != 0 {
                    if self.curband == 0 {
                        step = self.quant_lo[j];
                    }
                    let coef = self.blocks[block_idx][(i << 4) | j];
                    let mut abs_coef = coef.unsigned_abs() as i32;
                    let s = step as i32;
                    let des = if abs_coef <= 3 * s {
                        let d = zp.decode_bit(&mut self.increase_coef_ctx[0]);
                        abs_coef += s >> 2;
                        d
                    } else {
                        zp.decode_passthrough_iw44()
                    };
                    if des {
                        abs_coef += s >> 1;
                    } else {
                        abs_coef += -s + (s >> 1);
                    }
                    self.blocks[block_idx][(i << 4) | j] = if coef < 0 {
                        -abs_coef as i16
                    } else {
                        abs_coef as i16
                    };
                }
            }
        }
    }

    fn finish_code_slice(&mut self) {
        self.quant_hi[self.curband] >>= 1;
        if self.curband == 0 {
            for i in 0..16 {
                self.quant_lo[i] >>= 1;
            }
        }
        self.curband += 1;
        if self.curband == 10 {
            self.curband = 0;
        }
    }

    fn get_bytemap(&self, subsample: usize) -> Bytemap {
        let full_width = self.width.div_ceil(32) * 32;
        let full_height = self.height.div_ceil(32) * 32;
        let block_rows = self.height.div_ceil(32);
        let mut bm = Bytemap {
            data: vec![0i16; full_width * full_height],
            stride: full_width,
        };

        for r in 0..block_rows {
            for c in 0..self.block_cols {
                let block = &self.blocks[r * self.block_cols + c];
                let row_base = r << 5;
                let col_base = c << 5;
                for i in 0..1024 {
                    let row = ZIGZAG_ROW[i] as usize + row_base;
                    let col = ZIGZAG_COL[i] as usize + col_base;
                    bm.data[row * full_width + col] = block[i];
                }
            }
        }

        inverse_wavelet_transform(&mut bm, self.width, self.height, subsample);
        bm
    }
}

struct Bytemap {
    data: Vec<i16>,
    stride: usize,
}

impl Bytemap {
    #[cfg(test)]
    fn get(&self, row: usize, col: usize) -> i32 {
        self.data[row * self.stride + col] as i32
    }

    #[cfg(test)]
    fn add(&mut self, row: usize, col: usize, val: i32) {
        self.data[row * self.stride + col] =
            (self.data[row * self.stride + col] as i32 + val) as i16;
    }

    #[cfg(test)]
    fn sub(&mut self, row: usize, col: usize, val: i32) {
        self.data[row * self.stride + col] =
            (self.data[row * self.stride + col] as i32 - val) as i16;
    }
}

fn inverse_wavelet_transform(bm: &mut Bytemap, width: usize, height: usize, subsample: usize) {
    let stride = bm.stride;
    let data = bm.data.as_mut_slice();
    let mut s_degree: u32 = 4;
    let mut s = 16usize;

    // Reusable state arrays for transposed column pass (allocated once, max size = width at s=1)
    let mut st0 = vec![0i32; width];
    let mut st1 = vec![0i32; width];
    let mut st2 = vec![0i32; width];

    while s >= subsample {
        let sd = s_degree as usize;

        // === Column pass (transposed: iterate rows then columns for cache efficiency) ===
        {
            let kmax = (height - 1) >> sd;
            let border = kmax.saturating_sub(3);
            let num_cols = width.div_ceil(s);

            // --- Lifting (even samples) ---
            for v in &mut st0[..num_cols] {
                *v = 0;
            }
            for v in &mut st1[..num_cols] {
                *v = 0;
            }
            if kmax >= 1 {
                let off = (1 << sd) * stride;
                for (ci, col) in (0..width).step_by(s).enumerate() {
                    st2[ci] = data[off + col] as i32;
                }
            } else {
                for v in &mut st2[..num_cols] {
                    *v = 0;
                }
            }

            let mut k = 0usize;
            while k <= kmax {
                let k_off = (k << sd) * stride;
                let has_n3 = k + 3 <= kmax;
                let n3_off = if has_n3 { ((k + 3) << sd) * stride } else { 0 };

                for (ci, col) in (0..width).step_by(s).enumerate() {
                    let p3 = st0[ci];
                    let p1 = st1[ci];
                    let n1 = st2[ci];
                    let n3 = if has_n3 { data[n3_off + col] as i32 } else { 0 };

                    let a = p1 + n1;
                    let c = p3 + n3;
                    let idx = k_off + col;
                    data[idx] = (data[idx] as i32 - (((a << 3) + a - c + 16) >> 5)) as i16;

                    st0[ci] = p1;
                    st1[ci] = n1;
                    st2[ci] = n3;
                }
                k += 2;
            }

            // --- Prediction (odd samples) ---
            if kmax >= 1 {
                // Phase 1: k = 1
                let km1_off = 0;
                let k_off = (1 << sd) * stride;

                if 2 <= kmax {
                    let kp1_off = (2 << sd) * stride;
                    for (ci, col) in (0..width).step_by(s).enumerate() {
                        let p = data[km1_off + col] as i32;
                        let n = data[kp1_off + col] as i32;
                        let idx = k_off + col;
                        data[idx] = (data[idx] as i32 + ((p + n + 1) >> 1)) as i16;
                        st0[ci] = p;
                        st1[ci] = n;
                    }
                } else {
                    for (ci, col) in (0..width).step_by(s).enumerate() {
                        let p = data[km1_off + col] as i32;
                        let idx = k_off + col;
                        data[idx] = (data[idx] as i32 + p) as i16;
                        st0[ci] = p;
                        st1[ci] = 0;
                    }
                }

                if border >= 3 {
                    let off = (4 << sd) * stride;
                    for (ci, col) in (0..width).step_by(s).enumerate() {
                        st2[ci] = data[off + col] as i32;
                    }
                }

                // Phase 2: k = 3, 5, ..., border
                let mut k = 3usize;
                while k <= border {
                    let k_off = (k << sd) * stride;
                    let n3_off = ((k + 3) << sd) * stride;

                    for (ci, col) in (0..width).step_by(s).enumerate() {
                        let p3 = st0[ci];
                        let p1 = st1[ci];
                        let n1 = st2[ci];
                        let n3 = data[n3_off + col] as i32;

                        let a = p1 + n1;
                        let idx = k_off + col;
                        data[idx] =
                            (data[idx] as i32 + (((a << 3) + a - (p3 + n3) + 8) >> 4)) as i16;

                        st0[ci] = p1;
                        st1[ci] = n1;
                        st2[ci] = n3;
                    }
                    k += 2;
                }

                // Phase 3: tail (k > border)
                while k <= kmax {
                    let k_off = (k << sd) * stride;

                    if k < kmax {
                        for (ci, col) in (0..width).step_by(s).enumerate() {
                            let p = st1[ci];
                            let n = st2[ci];
                            let idx = k_off + col;
                            data[idx] = (data[idx] as i32 + ((p + n + 1) >> 1)) as i16;
                            st1[ci] = n;
                            st2[ci] = 0;
                        }
                    } else {
                        for (ci, col) in (0..width).step_by(s).enumerate() {
                            let p = st1[ci];
                            let idx = k_off + col;
                            data[idx] = (data[idx] as i32 + p) as i16;
                            st1[ci] = st2[ci];
                            st2[ci] = 0;
                        }
                    }
                    k += 2;
                }
            }
        }

        // === Row pass (already cache-friendly, work directly on data slice) ===
        {
            let kmax = (width - 1) >> sd;
            let border = kmax.saturating_sub(3);

            for row in (0..height).step_by(s) {
                let off = row * stride;

                // Lifting (even samples)
                let mut prev1: i32 = 0;
                let mut next1: i32 = 0;
                let mut next3: i32 = if kmax >= 1 {
                    data[off + (1 << sd)] as i32
                } else {
                    0
                };
                let mut prev3: i32;
                let mut k = 0usize;
                while k <= kmax {
                    prev3 = prev1;
                    prev1 = next1;
                    next1 = next3;
                    next3 = if k + 3 <= kmax {
                        data[off + ((k + 3) << sd)] as i32
                    } else {
                        0
                    };
                    let a = prev1 + next1;
                    let c = prev3 + next3;
                    let idx = off + (k << sd);
                    data[idx] = (data[idx] as i32 - (((a << 3) + a - c + 16) >> 5)) as i16;
                    k += 2;
                }

                // Prediction (odd samples)
                if kmax >= 1 {
                    let mut k = 1usize;
                    prev1 = data[off + ((k - 1) << sd)] as i32;
                    if k < kmax {
                        next1 = data[off + ((k + 1) << sd)] as i32;
                        let idx = off + (k << sd);
                        data[idx] = (data[idx] as i32 + ((prev1 + next1 + 1) >> 1)) as i16;
                    } else {
                        let idx = off + (k << sd);
                        data[idx] = (data[idx] as i32 + prev1) as i16;
                    }

                    next3 = if border >= 3 {
                        data[off + ((k + 3) << sd)] as i32
                    } else {
                        0
                    };

                    k = 3;
                    while k <= border {
                        prev3 = prev1;
                        prev1 = next1;
                        next1 = next3;
                        next3 = data[off + ((k + 3) << sd)] as i32;
                        let a = prev1 + next1;
                        let idx = off + (k << sd);
                        data[idx] =
                            (data[idx] as i32 + (((a << 3) + a - (prev3 + next3) + 8) >> 4)) as i16;
                        k += 2;
                    }

                    while k <= kmax {
                        prev1 = next1;
                        next1 = next3;
                        next3 = 0;
                        if k < kmax {
                            let idx = off + (k << sd);
                            data[idx] = (data[idx] as i32 + ((prev1 + next1 + 1) >> 1)) as i16;
                        } else {
                            let idx = off + (k << sd);
                            data[idx] = (data[idx] as i32 + prev1) as i16;
                        }
                        k += 2;
                    }
                }
            }
        }

        s >>= 1;
        s_degree = s_degree.saturating_sub(1);
    }
}

/// IW44 progressive wavelet image decoder.
pub struct IW44Image {
    width: u16,
    height: u16,
    is_color: bool,
    delay: u8,
    chroma_half: bool,
    y_codec: Option<IWDecoder>,
    cb_codec: Option<IWDecoder>,
    cr_codec: Option<IWDecoder>,
    cslice: usize,
}

impl Default for IW44Image {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
pub struct NormalizedPlanes {
    pub width: u32,
    pub height: u32,
    pub y: Vec<i16>,
    pub cb: Option<Vec<i16>>,
    pub cr: Option<Vec<i16>>,
}

impl IW44Image {
    pub fn new() -> Self {
        IW44Image {
            width: 0,
            height: 0,
            is_color: false,
            delay: 0,
            chroma_half: false,
            y_codec: None,
            cb_codec: None,
            cr_codec: None,
            cslice: 0,
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    #[cfg(test)]
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Decode one BG44/FG44 chunk. Call multiple times for progressive chunks.
    pub fn decode_chunk(&mut self, data: &[u8]) -> Result<(), DecodeError> {
        if data.len() < 2 {
            return Err(DecodeError::ChunkTooShort);
        }
        let serial = data[0];
        let slices = data[1];
        let payload_start;

        if serial == 0 {
            if data.len() < 9 {
                return Err(DecodeError::HeaderTooShort);
            }
            let majver = data[2];
            let minor = data[3];
            let is_grayscale = (majver >> 7) != 0;
            let w = u16::from_be_bytes([data[4], data[5]]);
            let h = u16::from_be_bytes([data[6], data[7]]);
            let delay_byte = data[8];
            let delay = if minor >= 2 { delay_byte & 127 } else { 0 };
            let chroma_half = minor >= 2 && (delay_byte & 0x80) == 0;

            if w == 0 || h == 0 {
                return Err(DecodeError::ZeroDimension);
            }
            // Cap total pixels to prevent OOM on malformed input (~256M pixels)
            let pixels = w as u64 * h as u64;
            if pixels > 256 * 1024 * 1024 {
                return Err(DecodeError::ImageTooLarge);
            }

            self.width = w;
            self.height = h;
            self.is_color = !is_grayscale;
            self.delay = delay;
            self.chroma_half = self.is_color && chroma_half;
            self.cslice = 0;
            self.y_codec = Some(IWDecoder::new(w as usize, h as usize));
            if self.is_color {
                self.cb_codec = Some(IWDecoder::new(w as usize, h as usize));
                self.cr_codec = Some(IWDecoder::new(w as usize, h as usize));
            }
            payload_start = 9;
        } else {
            if self.y_codec.is_none() {
                return Err(DecodeError::MissingFirstChunk);
            }
            payload_start = 2;
        }

        let zp_data = &data[payload_start..];
        // ZpDecoder requires ≥ 2 bytes; pad with 0xff (same as legacy decoder's
        // `read_byte` fallback) so that zero-length ZP payloads are handled
        // gracefully rather than rejected.
        const EMPTY_ZP: &[u8] = &[0xff, 0xff];
        let zp_init = if zp_data.len() >= 2 {
            zp_data
        } else {
            EMPTY_ZP
        };
        let mut zp = ZpDecoder::new(zp_init).expect("zp_init is at least 2 bytes");

        for _ in 0..slices {
            self.cslice += 1;
            if let Some(ref mut y) = self.y_codec {
                y.decode_slice(&mut zp);
            }
            if self.is_color && self.cslice > self.delay as usize {
                if let Some(ref mut cb) = self.cb_codec {
                    cb.decode_slice(&mut zp);
                }
                if let Some(ref mut cr) = self.cr_codec {
                    cr.decode_slice(&mut zp);
                }
            }
        }

        Ok(())
    }

    /// Convert decoded image to a Pixmap. DjVu images are bottom-to-top; this flips to top-to-bottom.
    pub fn to_pixmap(&self) -> Result<Pixmap, DecodeError> {
        self.to_pixmap_subsample(1)
    }

    /// Convert decoded image to a Pixmap at reduced resolution (subsample=1,2,4,8,16).
    pub fn to_pixmap_subsample(&self, subsample: u32) -> Result<Pixmap, DecodeError> {
        if subsample == 0 {
            return Err(DecodeError::InvalidSubsample);
        }
        let y_codec = self.y_codec.as_ref().ok_or(DecodeError::MissingCodec)?;
        let sub = subsample as usize;
        let w = (self.width as usize).div_ceil(sub) as u32;
        let h = (self.height as usize).div_ceil(sub) as u32;

        let y_bm = y_codec.get_bytemap(sub);

        if self.is_color {
            let chroma_sub = if self.chroma_half { sub.max(2) } else { sub };
            let cb_bm = self
                .cb_codec
                .as_ref()
                .ok_or(DecodeError::MissingCodec)?
                .get_bytemap(chroma_sub);
            let cr_bm = self
                .cr_codec
                .as_ref()
                .ok_or(DecodeError::MissingCodec)?
                .get_bytemap(chroma_sub);
            let mut pm = Pixmap::new(w, h, 0, 0, 0, 255);
            for row in 0..h {
                let out_row = h - 1 - row;
                for col in 0..w {
                    let src_row = row as usize * sub;
                    let src_col = col as usize * sub;
                    let y_idx = src_row * y_bm.stride + src_col;
                    let chroma_row = if self.chroma_half {
                        src_row & !1
                    } else {
                        src_row
                    };
                    let chroma_col = if self.chroma_half {
                        src_col & !1
                    } else {
                        src_col
                    };
                    let c_idx = chroma_row * cb_bm.stride + chroma_col;
                    let y = normalize(y_bm.data[y_idx]);
                    let b = normalize(cb_bm.data[c_idx]);
                    let r = normalize(cr_bm.data[c_idx]);

                    let t2 = r + (r >> 1);
                    let t3 = y + 128 - (b >> 2);

                    let red = (y + 128 + t2).clamp(0, 255) as u8;
                    let green = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
                    let blue = (t3 + (b << 1)).clamp(0, 255) as u8;
                    pm.set_rgb(col, out_row, red, green, blue);
                }
            }
            Ok(pm)
        } else {
            let mut pm = Pixmap::new(w, h, 0, 0, 0, 255);
            for row in 0..h {
                let out_row = h - 1 - row;
                for col in 0..w {
                    let src_row = row as usize * sub;
                    let src_col = col as usize * sub;
                    let idx = src_row * y_bm.stride + src_col;
                    let val = normalize(y_bm.data[idx]);
                    let gray = (127 - val) as u8;
                    pm.set_rgb(col, out_row, gray, gray, gray);
                }
            }
            Ok(pm)
        }
    }

    #[cfg(test)]
    pub fn to_normalized_planes_subsample(
        &self,
        subsample: u32,
    ) -> Result<NormalizedPlanes, DecodeError> {
        if subsample == 0 {
            return Err(DecodeError::InvalidSubsample);
        }
        let y_codec = self.y_codec.as_ref().ok_or(DecodeError::MissingCodec)?;
        let sub = subsample as usize;
        let w = (self.width as usize).div_ceil(sub) as u32;
        let h = (self.height as usize).div_ceil(sub) as u32;
        let y_bm = y_codec.get_bytemap(sub);

        let mut y = vec![0i16; (w * h) as usize];
        for row in 0..h {
            let out_row = h - 1 - row;
            for col in 0..w {
                let src_row = row as usize * sub;
                let src_col = col as usize * sub;
                let idx = src_row * y_bm.stride + src_col;
                y[(out_row * w + col) as usize] = normalize(y_bm.data[idx]) as i16;
            }
        }

        if self.is_color {
            let chroma_sub = if self.chroma_half { sub.max(2) } else { sub };
            let cb_bm = self
                .cb_codec
                .as_ref()
                .ok_or(DecodeError::MissingCodec)?
                .get_bytemap(chroma_sub);
            let cr_bm = self
                .cr_codec
                .as_ref()
                .ok_or(DecodeError::MissingCodec)?
                .get_bytemap(chroma_sub);
            let mut cb = vec![0i16; (w * h) as usize];
            let mut cr = vec![0i16; (w * h) as usize];
            for row in 0..h {
                let out_row = h - 1 - row;
                for col in 0..w {
                    let src_row = row as usize * sub;
                    let src_col = col as usize * sub;
                    let chroma_row = if self.chroma_half {
                        src_row & !1
                    } else {
                        src_row
                    };
                    let chroma_col = if self.chroma_half {
                        src_col & !1
                    } else {
                        src_col
                    };
                    let c_idx = chroma_row * cb_bm.stride + chroma_col;
                    let out_idx = (out_row * w + col) as usize;
                    cb[out_idx] = normalize(cb_bm.data[c_idx]) as i16;
                    cr[out_idx] = normalize(cr_bm.data[c_idx]) as i16;
                }
            }
            Ok(NormalizedPlanes {
                width: w,
                height: h,
                y,
                cb: Some(cb),
                cr: Some(cr),
            })
        } else {
            Ok(NormalizedPlanes {
                width: w,
                height: h,
                y,
                cb: None,
                cr: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::implicit_saturating_sub,
        clippy::int_plus_one,
        clippy::manual_div_ceil
    )]

    use super::*;

    fn assets_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/djvujs/library/assets")
    }

    fn golden_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/iw44")
    }

    fn extract_bg44_chunks(file: &crate::iff::DjvuFile) -> Vec<&[u8]> {
        fn collect_from_djvu_form(chunk: &crate::iff::Chunk) -> Option<Vec<&[u8]>> {
            match chunk {
                crate::iff::Chunk::Form {
                    secondary_id,
                    children,
                    ..
                } => {
                    if secondary_id == b"DJVU" {
                        let v = children
                            .iter()
                            .filter_map(|c| match c {
                                crate::iff::Chunk::Leaf {
                                    id: [b'B', b'G', b'4', b'4'],
                                    data,
                                } => Some(data.as_slice()),
                                _ => None,
                            })
                            .collect::<Vec<_>>();
                        return Some(v);
                    }
                    for c in children {
                        if let Some(v) = collect_from_djvu_form(c) {
                            return Some(v);
                        }
                    }
                    None
                }
                _ => None,
            }
        }
        collect_from_djvu_form(&file.root).unwrap_or_default()
    }

    fn decode_chunks_with_options(
        chunks: &[&[u8]],
        preadvance_color_delay: bool,
    ) -> Result<IW44Image, DecodeError> {
        let mut img = IW44Image::new();
        for data in chunks {
            if data.len() < 2 {
                return Err(DecodeError::ChunkTooShort);
            }
            let serial = data[0];
            let slices = data[1];
            let payload_start;

            if serial == 0 {
                if data.len() < 9 {
                    return Err(DecodeError::HeaderTooShort);
                }
                let majver = data[2];
                let minor = data[3];
                let is_grayscale = (majver >> 7) != 0;
                let w = u16::from_be_bytes([data[4], data[5]]);
                let h = u16::from_be_bytes([data[6], data[7]]);
                let delay_byte = data[8];
                let delay = if minor >= 2 { delay_byte & 127 } else { 0 };
                let chroma_half = minor >= 2 && (delay_byte & 0x80) == 0;

                img.width = w;
                img.height = h;
                img.is_color = !is_grayscale;
                img.delay = delay;
                img.chroma_half = img.is_color && chroma_half;
                img.cslice = 0;
                img.y_codec = Some(IWDecoder::new(w as usize, h as usize));
                if img.is_color {
                    let mut cb = IWDecoder::new(w as usize, h as usize);
                    let mut cr = IWDecoder::new(w as usize, h as usize);
                    if preadvance_color_delay {
                        for _ in 0..delay {
                            cb.finish_code_slice();
                            cr.finish_code_slice();
                        }
                    }
                    img.cb_codec = Some(cb);
                    img.cr_codec = Some(cr);
                }
                payload_start = 9;
            } else {
                if img.y_codec.is_none() {
                    return Err(DecodeError::MissingFirstChunk);
                }
                payload_start = 2;
            }

            let zp_data = &data[payload_start..];
            let mut zp = ZpDecoder::new(zp_data).map_err(|_| DecodeError::ChunkTooShort)?;

            for _ in 0..slices {
                img.cslice += 1;
                if let Some(ref mut y) = img.y_codec {
                    y.decode_slice(&mut zp);
                }
                if img.is_color && img.cslice > img.delay as usize {
                    if let Some(ref mut cb) = img.cb_codec {
                        cb.decode_slice(&mut zp);
                    }
                    if let Some(ref mut cr) = img.cr_codec {
                        cr.decode_slice(&mut zp);
                    }
                }
            }
        }
        Ok(img)
    }

    fn reset_contexts(dec: &mut IWDecoder) {
        dec.decode_bucket_ctx = [0; 1];
        dec.decode_coef_ctx = [0; 80];
        dec.activate_coef_ctx = [0; 16];
        dec.increase_coef_ctx = [0; 1];
    }

    fn decode_chunks_with_context_resets(
        chunks: &[&[u8]],
        reset_each_chunk: bool,
        reset_each_slice: bool,
        reset_on_color_start: bool,
    ) -> Result<IW44Image, DecodeError> {
        let mut img = IW44Image::new();
        let mut color_started = false;
        for data in chunks {
            if data.len() < 2 {
                return Err(DecodeError::ChunkTooShort);
            }
            let serial = data[0];
            let slices = data[1];
            let payload_start;

            if serial == 0 {
                if data.len() < 9 {
                    return Err(DecodeError::HeaderTooShort);
                }
                let majver = data[2];
                let minor = data[3];
                let is_grayscale = (majver >> 7) != 0;
                let w = u16::from_be_bytes([data[4], data[5]]);
                let h = u16::from_be_bytes([data[6], data[7]]);
                let delay_byte = data[8];
                let delay = if minor >= 2 { delay_byte & 127 } else { 0 };
                let chroma_half = minor >= 2 && (delay_byte & 0x80) == 0;

                img.width = w;
                img.height = h;
                img.is_color = !is_grayscale;
                img.delay = delay;
                img.chroma_half = img.is_color && chroma_half;
                img.cslice = 0;
                img.y_codec = Some(IWDecoder::new(w as usize, h as usize));
                if img.is_color {
                    img.cb_codec = Some(IWDecoder::new(w as usize, h as usize));
                    img.cr_codec = Some(IWDecoder::new(w as usize, h as usize));
                }
                payload_start = 9;
            } else {
                if img.y_codec.is_none() {
                    return Err(DecodeError::MissingFirstChunk);
                }
                payload_start = 2;
            }

            if reset_each_chunk {
                if let Some(ref mut y) = img.y_codec {
                    reset_contexts(y);
                }
                if let Some(ref mut cb) = img.cb_codec {
                    reset_contexts(cb);
                }
                if let Some(ref mut cr) = img.cr_codec {
                    reset_contexts(cr);
                }
            }

            let zp_data = &data[payload_start..];
            let mut zp = ZpDecoder::new(zp_data).map_err(|_| DecodeError::ChunkTooShort)?;

            for _ in 0..slices {
                img.cslice += 1;
                if reset_each_slice {
                    if let Some(ref mut y) = img.y_codec {
                        reset_contexts(y);
                    }
                    if let Some(ref mut cb) = img.cb_codec {
                        reset_contexts(cb);
                    }
                    if let Some(ref mut cr) = img.cr_codec {
                        reset_contexts(cr);
                    }
                }

                if let Some(ref mut y) = img.y_codec {
                    y.decode_slice(&mut zp);
                }
                if img.is_color && img.cslice > img.delay as usize {
                    if reset_on_color_start && !color_started {
                        if let Some(ref mut cb) = img.cb_codec {
                            reset_contexts(cb);
                        }
                        if let Some(ref mut cr) = img.cr_codec {
                            reset_contexts(cr);
                        }
                        color_started = true;
                    }
                    if let Some(ref mut cb) = img.cb_codec {
                        cb.decode_slice(&mut zp);
                    }
                    if let Some(ref mut cr) = img.cr_codec {
                        cr.decode_slice(&mut zp);
                    }
                }
            }
        }
        Ok(img)
    }

    fn find_ppm_data_start(ppm: &[u8]) -> usize {
        let mut newlines = 0;
        for (i, &b) in ppm.iter().enumerate() {
            if b == b'\n' {
                newlines += 1;
                if newlines == 3 {
                    return i + 1;
                }
            }
        }
        0
    }

    fn assert_ppm_match(actual_ppm: &[u8], golden_file: &str) {
        let expected_ppm = std::fs::read(golden_path().join(golden_file)).unwrap();
        assert_eq!(
            actual_ppm.len(),
            expected_ppm.len(),
            "PPM size mismatch for {}: got {} expected {}",
            golden_file,
            actual_ppm.len(),
            expected_ppm.len()
        );
        if actual_ppm != expected_ppm {
            let header_end = find_ppm_data_start(actual_ppm);
            let actual_pixels = &actual_ppm[header_end..];
            let expected_pixels = &expected_ppm[header_end..];
            let total_pixels = actual_pixels.len() / 3;
            let diff_pixels = actual_pixels
                .chunks(3)
                .zip(expected_pixels.chunks(3))
                .filter(|(a, b)| a != b)
                .count();
            panic!(
                "{} pixel mismatch: {}/{} pixels differ ({:.1}%)",
                golden_file,
                diff_pixels,
                total_pixels,
                diff_pixels as f64 / total_pixels as f64 * 100.0
            );
        }
    }

    fn get_bytemap_custom(
        dec: &IWDecoder,
        subsample: usize,
        even_bias: i32,
        pred_bias: i32,
        mid_bias: i32,
    ) -> Bytemap {
        let full_width = ((dec.width + 31) / 32) * 32;
        let full_height = ((dec.height + 31) / 32) * 32;
        let block_rows = (dec.height + 31) / 32;
        let mut bm = Bytemap {
            data: vec![0i16; full_width * full_height],
            stride: full_width,
        };

        for r in 0..block_rows {
            for c in 0..dec.block_cols {
                let block = &dec.blocks[r * dec.block_cols + c];
                let row_base = r << 5;
                let col_base = c << 5;
                for i in 0..1024 {
                    let row = ZIGZAG_ROW[i] as usize + row_base;
                    let col = ZIGZAG_COL[i] as usize + col_base;
                    bm.data[row * full_width + col] = block[i];
                }
            }
        }

        inverse_wavelet_transform_custom(
            &mut bm, dec.width, dec.height, subsample, even_bias, pred_bias, mid_bias,
        );
        bm
    }

    fn inverse_wavelet_transform_custom(
        bm: &mut Bytemap,
        width: usize,
        height: usize,
        subsample: usize,
        even_bias: i32,
        pred_bias: i32,
        mid_bias: i32,
    ) {
        let mut s_degree: u32 = 4;
        let mut s = 16usize;
        while s >= subsample {
            let kmax = (height - 1) >> s_degree;
            let border = if kmax >= 3 { kmax - 3 } else { 0 };
            for i in (0..width).step_by(s) {
                let mut prev1: i32 = 0;
                let mut next1: i32 = 0;
                let mut next3: i32 = if 1 > kmax {
                    0
                } else {
                    bm.get(1 << s_degree, i)
                };
                let mut prev3: i32;
                let mut k = 0;
                while k <= kmax {
                    prev3 = prev1;
                    prev1 = next1;
                    next1 = next3;
                    next3 = if k + 3 > kmax {
                        0
                    } else {
                        bm.get((k + 3) << s_degree, i)
                    };
                    let a = prev1 + next1;
                    let c = prev3 + next3;
                    bm.sub(k << s_degree, i, ((a << 3) + a - c + even_bias) >> 5);
                    k += 2;
                }

                k = 1;
                prev1 = bm.get((k - 1) << s_degree, i);
                if k + 1 <= kmax {
                    next1 = bm.get((k + 1) << s_degree, i);
                    bm.add(k << s_degree, i, (prev1 + next1 + pred_bias) >> 1);
                } else {
                    bm.add(k << s_degree, i, prev1);
                }

                if border >= 3 {
                    next3 = bm.get((k + 3) << s_degree, i);
                }

                k = 3;
                while k <= border {
                    prev3 = prev1;
                    prev1 = next1;
                    next1 = next3;
                    next3 = bm.get((k + 3) << s_degree, i);
                    let a = prev1 + next1;
                    bm.add(
                        k << s_degree,
                        i,
                        ((a << 3) + a - (prev3 + next3) + mid_bias) >> 4,
                    );
                    k += 2;
                }

                while k <= kmax {
                    prev1 = next1;
                    next1 = next3;
                    next3 = 0;
                    if k + 1 <= kmax {
                        bm.add(k << s_degree, i, (prev1 + next1 + pred_bias) >> 1);
                    } else {
                        bm.add(k << s_degree, i, prev1);
                    }
                    k += 2;
                }
            }

            let kmax = (width - 1) >> s_degree;
            let border = if kmax >= 3 { kmax - 3 } else { 0 };
            for i in (0..height).step_by(s) {
                let mut prev1: i32 = 0;
                let mut next1: i32 = 0;
                let mut next3: i32 = if 1 > kmax {
                    0
                } else {
                    bm.get(i, 1 << s_degree)
                };
                let mut prev3: i32;
                let mut k = 0;
                while k <= kmax {
                    prev3 = prev1;
                    prev1 = next1;
                    next1 = next3;
                    next3 = if k + 3 > kmax {
                        0
                    } else {
                        bm.get(i, (k + 3) << s_degree)
                    };
                    let a = prev1 + next1;
                    let c = prev3 + next3;
                    bm.sub(i, k << s_degree, ((a << 3) + a - c + even_bias) >> 5);
                    k += 2;
                }

                k = 1;
                prev1 = bm.get(i, (k - 1) << s_degree);
                if k + 1 <= kmax {
                    next1 = bm.get(i, (k + 1) << s_degree);
                    bm.add(i, k << s_degree, (prev1 + next1 + pred_bias) >> 1);
                } else {
                    bm.add(i, k << s_degree, prev1);
                }

                if border >= 3 {
                    next3 = bm.get(i, (k + 3) << s_degree);
                }

                k = 3;
                while k <= border {
                    prev3 = prev1;
                    prev1 = next1;
                    next1 = next3;
                    next3 = bm.get(i, (k + 3) << s_degree);
                    let a = prev1 + next1;
                    bm.add(
                        i,
                        k << s_degree,
                        ((a << 3) + a - (prev3 + next3) + mid_bias) >> 4,
                    );
                    k += 2;
                }

                while k <= kmax {
                    prev1 = next1;
                    next1 = next3;
                    next3 = 0;
                    if k + 1 <= kmax {
                        bm.add(i, k << s_degree, (prev1 + next1 + pred_bias) >> 1);
                    } else {
                        bm.add(i, k << s_degree, prev1);
                    }
                    k += 2;
                }
            }

            s >>= 1;
            if s_degree > 0 {
                s_degree -= 1;
            }
        }
    }

    #[test]
    fn zigzag_table_spot_checks() {
        assert_eq!(ZIGZAG_ROW[0], 0);
        assert_eq!(ZIGZAG_COL[0], 0);
        assert_eq!(ZIGZAG_ROW[1], 0);
        assert_eq!(ZIGZAG_COL[1], 16);
        assert_eq!(ZIGZAG_ROW[2], 16);
        assert_eq!(ZIGZAG_COL[2], 0);
        assert_eq!(ZIGZAG_ROW[3], 16);
        assert_eq!(ZIGZAG_COL[3], 16);
    }

    #[test]
    fn iw44_decode_boy_bg() {
        let data = std::fs::read(assets_path().join("boy.djvu")).unwrap();
        let file = crate::iff::parse(&data).unwrap();
        let chunks = extract_bg44_chunks(&file);
        assert_eq!(chunks.len(), 1);

        let mut img = IW44Image::new();
        for c in &chunks {
            img.decode_chunk(c).unwrap();
        }
        assert_eq!(img.width(), 192);
        assert_eq!(img.height(), 256);

        let pm = img.to_pixmap().unwrap();
        assert_ppm_match(&pm.to_ppm(), "boy_bg.ppm");
    }

    #[test]
    fn iw44_decode_big_scanned_sub4() {
        let data = std::fs::read(assets_path().join("big-scanned-page.djvu")).unwrap();
        let file = crate::iff::parse(&data).unwrap();
        let chunks = extract_bg44_chunks(&file);
        assert_eq!(chunks.len(), 4);

        let mut img = IW44Image::new();
        for c in &chunks {
            img.decode_chunk(c).unwrap();
        }
        assert_eq!(img.width(), 6780);
        assert_eq!(img.height(), 9148);

        let pm = img.to_pixmap_subsample(4).unwrap();
        assert_ppm_match(&pm.to_ppm(), "big_scanned_sub4.ppm");
    }

    #[test]
    fn iw44_decode_chicken_bg() {
        let data = std::fs::read(assets_path().join("chicken.djvu")).unwrap();
        let file = crate::iff::parse(&data).unwrap();
        let chunks = extract_bg44_chunks(&file);
        assert_eq!(chunks.len(), 3);

        let mut img = IW44Image::new();
        for c in &chunks {
            img.decode_chunk(c).unwrap();
        }
        assert_eq!(img.width(), 181);
        assert_eq!(img.height(), 240);

        let pm = img.to_pixmap().unwrap();
        assert_ppm_match(&pm.to_ppm(), "chicken_bg.ppm");
    }

    #[test]
    #[ignore]
    fn debug_carte_bg_color_candidates() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/carte_bg_sub3.ppm");
        if !ref_path.exists() {
            return;
        }

        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let file = crate::iff::parse(&data).unwrap();
        let chunks = extract_bg44_chunks(&file);
        let mut img = IW44Image::new();
        for c in &chunks {
            img.decode_chunk(c).unwrap();
        }

        let y_bm = img.y_codec.as_ref().unwrap().get_bytemap(1);
        let cb_bm = img.cb_codec.as_ref().unwrap().get_bytemap(1);
        let cr_bm = img.cr_codec.as_ref().unwrap().get_bytemap(1);
        let w = img.width as u32;
        let h = img.height as u32;
        let expected = std::fs::read(ref_path).unwrap();

        let compare = |name: &str, build: &dyn Fn() -> Pixmap| {
            let actual = build().to_ppm();
            let header_end = find_ppm_data_start(&actual);
            let a = &actual[header_end..];
            let e = &expected[header_end..];
            let px = (a.len().min(e.len())) / 3;
            let mut diff_px = 0usize;
            for p in 0..px {
                let i = p * 3;
                if a[i] != e[i] || a[i + 1] != e[i + 1] || a[i + 2] != e[i + 2] {
                    diff_px += 1;
                }
            }
            eprintln!("carte bg color {} mismatch_px={}", name, diff_px);
        };

        let norm = |val: i16, offset: i32| -> i32 {
            let v = ((val as i32) + offset) >> 6;
            v.clamp(-128, 127)
        };

        let norm_sym = |val: i16| -> i32 {
            let v = if val >= 0 {
                ((val as i32) + 32) >> 6
            } else {
                -((((-val) as i32) + 32) >> 6)
            };
            v.clamp(-128, 127)
        };

        let build_pm = |y_off: i32, c_off: i32, mode: &str| -> Pixmap {
            let mut pm = Pixmap::new(w, h, 0, 0, 0, 255);
            for row in 0..h {
                let out_row = h - 1 - row;
                for col in 0..w {
                    let idx = row as usize * y_bm.stride + col as usize;
                    let y = norm(y_bm.data[idx], y_off);
                    let b = norm(cb_bm.data[idx], c_off);
                    let r = norm(cr_bm.data[idx], c_off);
                    let (t2, t3, green) = match mode {
                        "shift" => {
                            let t2 = r + (r >> 1);
                            let t3 = y + 128 - (b >> 2);
                            let green = t3 - (t2 >> 1);
                            (t2, t3, green)
                        }
                        "trunc_div" => {
                            let t2 = r + r / 2;
                            let t3 = y + 128 - b / 4;
                            let green = t3 - t2 / 2;
                            (t2, t3, green)
                        }
                        "mixed_bdiv" => {
                            let t2 = r + (r >> 1);
                            let t3 = y + 128 - b / 4;
                            let green = t3 - (t2 >> 1);
                            (t2, t3, green)
                        }
                        "mixed_gdiv" => {
                            let t2 = r + (r >> 1);
                            let t3 = y + 128 - (b >> 2);
                            let green = t3 - t2 / 2;
                            (t2, t3, green)
                        }
                        _ => unreachable!(),
                    };

                    let red = (y + 128 + t2).clamp(0, 255) as u8;
                    let blue = (t3 + (b << 1)).clamp(0, 255) as u8;
                    pm.set_rgb(col, out_row, red, green.clamp(0, 255) as u8, blue);
                }
            }
            pm
        };

        compare("current_shift", &|| build_pm(32, 32, "shift"));
        compare("trunc_div", &|| build_pm(32, 32, "trunc_div"));
        compare("mixed_bdiv", &|| build_pm(32, 32, "mixed_bdiv"));
        compare("mixed_gdiv", &|| build_pm(32, 32, "mixed_gdiv"));
        compare("chroma_off31_shift", &|| build_pm(32, 31, "shift"));
        compare("chroma_off33_shift", &|| build_pm(32, 33, "shift"));
        compare("chroma_off31_trunc_div", &|| build_pm(32, 31, "trunc_div"));
        compare("chroma_off33_trunc_div", &|| build_pm(32, 33, "trunc_div"));
        compare("sym_chroma_shift", &|| {
            let mut pm = Pixmap::new(w, h, 0, 0, 0, 255);
            for row in 0..h {
                let out_row = h - 1 - row;
                for col in 0..w {
                    let idx = row as usize * y_bm.stride + col as usize;
                    let y = norm(y_bm.data[idx], 32);
                    let b = norm_sym(cb_bm.data[idx]);
                    let r = norm_sym(cr_bm.data[idx]);
                    let t2 = r + (r >> 1);
                    let t3 = y + 128 - (b >> 2);
                    let red = (y + 128 + t2).clamp(0, 255) as u8;
                    let green = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
                    let blue = (t3 + (b << 1)).clamp(0, 255) as u8;
                    pm.set_rgb(col, out_row, red, green, blue);
                }
            }
            pm
        });
        compare("sym_all_shift", &|| {
            let mut pm = Pixmap::new(w, h, 0, 0, 0, 255);
            for row in 0..h {
                let out_row = h - 1 - row;
                for col in 0..w {
                    let idx = row as usize * y_bm.stride + col as usize;
                    let y = norm_sym(y_bm.data[idx]);
                    let b = norm_sym(cb_bm.data[idx]);
                    let r = norm_sym(cr_bm.data[idx]);
                    let t2 = r + (r >> 1);
                    let t3 = y + 128 - (b >> 2);
                    let red = (y + 128 + t2).clamp(0, 255) as u8;
                    let green = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
                    let blue = (t3 + (b << 1)).clamp(0, 255) as u8;
                    pm.set_rgb(col, out_row, red, green, blue);
                }
            }
            pm
        });
    }

    #[test]
    #[ignore]
    fn debug_carte_bg_wavelet_rounding_candidates() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/carte_bg_sub3.ppm");
        if !ref_path.exists() {
            return;
        }

        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let file = crate::iff::parse(&data).unwrap();
        let chunks = extract_bg44_chunks(&file);
        let mut img = IW44Image::new();
        for c in &chunks {
            img.decode_chunk(c).unwrap();
        }

        let y_dec = img.y_codec.as_ref().unwrap();
        let cb_dec = img.cb_codec.as_ref().unwrap();
        let cr_dec = img.cr_codec.as_ref().unwrap();
        let w = img.width as u32;
        let h = img.height as u32;
        let expected = std::fs::read(ref_path).unwrap();

        let compare = |even_bias: i32, pred_bias: i32, mid_bias: i32| {
            let y_bm = get_bytemap_custom(y_dec, 1, even_bias, pred_bias, mid_bias);
            let cb_bm = get_bytemap_custom(cb_dec, 1, even_bias, pred_bias, mid_bias);
            let cr_bm = get_bytemap_custom(cr_dec, 1, even_bias, pred_bias, mid_bias);
            let mut pm = Pixmap::new(w, h, 0, 0, 0, 255);
            for row in 0..h {
                let out_row = h - 1 - row;
                for col in 0..w {
                    let idx = row as usize * y_bm.stride + col as usize;
                    let y = normalize(y_bm.data[idx]);
                    let b = normalize(cb_bm.data[idx]);
                    let r = normalize(cr_bm.data[idx]);
                    let t2 = r + (r >> 1);
                    let t3 = y + 128 - (b >> 2);
                    let red = (y + 128 + t2).clamp(0, 255) as u8;
                    let green = (t3 - (t2 >> 1)).clamp(0, 255) as u8;
                    let blue = (t3 + (b << 1)).clamp(0, 255) as u8;
                    pm.set_rgb(col, out_row, red, green, blue);
                }
            }

            let actual = pm.to_ppm();
            let header_end = find_ppm_data_start(&actual);
            let a = &actual[header_end..];
            let e = &expected[header_end..];
            let px = (a.len().min(e.len())) / 3;
            let mut diff_px = 0usize;
            for p in 0..px {
                let i = p * 3;
                if a[i] != e[i] || a[i + 1] != e[i + 1] || a[i + 2] != e[i + 2] {
                    diff_px += 1;
                }
            }
            eprintln!(
                "carte bg wavelet even={} pred={} mid={} mismatch_px={}",
                even_bias, pred_bias, mid_bias, diff_px
            );
        };

        for even_bias in [15, 16, 17] {
            for pred_bias in [0, 1] {
                for mid_bias in [7, 8, 9] {
                    compare(even_bias, pred_bias, mid_bias);
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn debug_bg_header_profiles() {
        for file in [
            "carte.djvu",
            "colorbook.djvu",
            "navm_fgbz.djvu",
            "chicken.djvu",
        ] {
            let data = std::fs::read(assets_path().join(file)).unwrap();
            let parsed = crate::iff::parse(&data).unwrap();
            let chunks = extract_bg44_chunks(&parsed);
            if chunks.is_empty() {
                continue;
            }
            let mut img = IW44Image::new();
            img.decode_chunk(chunks[0]).unwrap();
            eprintln!(
                "{} bg44 chunks={} first: {}x{} color={} delay={} cslice={}",
                file,
                chunks.len(),
                img.width,
                img.height,
                img.is_color,
                img.delay,
                img.cslice
            );
            for (idx, chunk) in chunks.iter().enumerate() {
                let serial = chunk[0];
                let slices = chunk[1];
                eprintln!(
                    "  chunk {} serial={} slices={} len={}",
                    idx,
                    serial,
                    slices,
                    chunk.len()
                );
            }
        }
    }

    #[test]
    fn iw44_parse_crcb_half_mode() {
        for (file, expected_half) in [
            ("carte.djvu", true),
            ("colorbook.djvu", false),
            ("chicken.djvu", false),
            ("navm_fgbz.djvu", false),
        ] {
            let data = std::fs::read(assets_path().join(file)).unwrap();
            let parsed = crate::iff::parse(&data).unwrap();
            let chunks = extract_bg44_chunks(&parsed);
            if chunks.is_empty() {
                continue;
            }
            let mut img = IW44Image::new();
            img.decode_chunk(chunks[0]).unwrap();
            assert_eq!(img.chroma_half, expected_half, "{}", file);
        }
    }

    #[test]
    #[ignore]
    fn debug_carte_bg_progressive_chunk_mismatch() {
        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let file = crate::iff::parse(&data).unwrap();
        let chunks = extract_bg44_chunks(&file);

        for nchunks in 1..=chunks.len() {
            let ref_path =
                std::path::PathBuf::from(format!("/tmp/rdjvu_debug/carte_bg_{}_ref.ppm", nchunks));
            if !ref_path.exists() {
                continue;
            }

            let mut img = IW44Image::new();
            for chunk in chunks.iter().take(nchunks) {
                img.decode_chunk(chunk).unwrap();
            }
            let actual = img.to_pixmap().unwrap().to_ppm();
            let expected = std::fs::read(ref_path).unwrap();
            let header_end = find_ppm_data_start(&actual);
            let a = &actual[header_end..];
            let e = &expected[header_end..];
            let px = (a.len().min(e.len())) / 3;
            let mut diff_px = 0usize;
            let mut abs = [0u64; 3];
            for p in 0..px {
                let i = p * 3;
                if a[i] != e[i] || a[i + 1] != e[i + 1] || a[i + 2] != e[i + 2] {
                    diff_px += 1;
                }
                abs[0] += (a[i] as i32 - e[i] as i32).unsigned_abs() as u64;
                abs[1] += (a[i + 1] as i32 - e[i + 1] as i32).unsigned_abs() as u64;
                abs[2] += (a[i + 2] as i32 - e[i + 2] as i32).unsigned_abs() as u64;
            }
            eprintln!(
                "carte bg chunks={} mismatch_px={} mean_abs=({:.4},{:.4},{:.4})",
                nchunks,
                diff_px,
                abs[0] as f64 / px as f64,
                abs[1] as f64 / px as f64,
                abs[2] as f64 / px as f64
            );
        }
    }

    #[test]
    #[ignore]
    fn debug_carte_bg_progressive_luma_mismatch() {
        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let file = crate::iff::parse(&data).unwrap();
        let chunks = extract_bg44_chunks(&file);

        for nchunks in 1..=chunks.len() {
            let ref_path =
                std::path::PathBuf::from(format!("/tmp/rdjvu_debug/carte_bg_{}_ref.ppm", nchunks));
            if !ref_path.exists() {
                continue;
            }

            let mut img = IW44Image::new();
            for chunk in chunks.iter().take(nchunks) {
                img.decode_chunk(chunk).unwrap();
            }
            let actual = img.to_pixmap().unwrap().to_ppm();
            let expected = std::fs::read(ref_path).unwrap();
            let header_end = find_ppm_data_start(&actual);
            let a = &actual[header_end..];
            let e = &expected[header_end..];
            let px = (a.len().min(e.len())) / 3;
            let mut gray_diff_px = 0usize;
            let mut gray_abs = 0u64;
            let mut chroma_abs = [0u64; 2];

            for p in 0..px {
                let i = p * 3;
                let ar = a[i] as i32;
                let ag = a[i + 1] as i32;
                let ab = a[i + 2] as i32;
                let er = e[i] as i32;
                let eg = e[i + 1] as i32;
                let eb = e[i + 2] as i32;

                let ay = (77 * ar + 150 * ag + 29 * ab + 128) >> 8;
                let ey = (77 * er + 150 * eg + 29 * eb + 128) >> 8;
                if ay != ey {
                    gray_diff_px += 1;
                }
                gray_abs += (ay - ey).unsigned_abs() as u64;

                let acb = ab - ay;
                let ecb = eb - ey;
                let acr = ar - ay;
                let ecr = er - ey;
                chroma_abs[0] += (acb - ecb).unsigned_abs() as u64;
                chroma_abs[1] += (acr - ecr).unsigned_abs() as u64;
            }

            eprintln!(
                "carte bg chunks={} gray_diff_px={} ({:.1}%) mean_abs_gray={:.4} mean_abs_cb={} mean_abs_cr={}",
                nchunks,
                gray_diff_px,
                gray_diff_px as f64 / px as f64 * 100.0,
                gray_abs as f64 / px as f64,
                chroma_abs[0] as f64 / px as f64,
                chroma_abs[1] as f64 / px as f64
            );
        }
    }

    #[test]
    #[ignore]
    fn debug_colorbook_bg_luma_mismatch() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/colorbook_bg_ref.ppm");
        if !ref_path.exists() {
            return;
        }

        let data = std::fs::read(assets_path().join("colorbook.djvu")).unwrap();
        let file = crate::iff::parse(&data).unwrap();
        let chunks = extract_bg44_chunks(&file);
        let mut img = IW44Image::new();
        for chunk in &chunks {
            img.decode_chunk(chunk).unwrap();
        }

        let actual = img.to_pixmap().unwrap().to_ppm();
        let expected = std::fs::read(ref_path).unwrap();
        let header_end = find_ppm_data_start(&actual);
        let a = &actual[header_end..];
        let e = &expected[header_end..];
        let px = (a.len().min(e.len())) / 3;
        let mut diff_px = 0usize;
        let mut gray_diff_px = 0usize;
        let mut abs = [0u64; 3];
        let mut gray_abs = 0u64;
        let mut chroma_abs = [0u64; 2];

        for p in 0..px {
            let i = p * 3;
            let ar = a[i] as i32;
            let ag = a[i + 1] as i32;
            let ab = a[i + 2] as i32;
            let er = e[i] as i32;
            let eg = e[i + 1] as i32;
            let eb = e[i + 2] as i32;

            if ar != er || ag != eg || ab != eb {
                diff_px += 1;
            }

            abs[0] += (ar - er).unsigned_abs() as u64;
            abs[1] += (ag - eg).unsigned_abs() as u64;
            abs[2] += (ab - eb).unsigned_abs() as u64;

            let ay = (77 * ar + 150 * ag + 29 * ab + 128) >> 8;
            let ey = (77 * er + 150 * eg + 29 * eb + 128) >> 8;
            if ay != ey {
                gray_diff_px += 1;
            }
            gray_abs += (ay - ey).unsigned_abs() as u64;

            let acb = ab - ay;
            let ecb = eb - ey;
            let acr = ar - ay;
            let ecr = er - ey;
            chroma_abs[0] += (acb - ecb).unsigned_abs() as u64;
            chroma_abs[1] += (acr - ecr).unsigned_abs() as u64;
        }

        eprintln!(
            "colorbook bg diff_px={} ({:.1}%) mean_abs_rgb=({:.4},{:.4},{:.4}) gray_diff_px={} ({:.1}%) mean_abs_gray={:.4} mean_abs_cb={} mean_abs_cr={}",
            diff_px,
            diff_px as f64 / px as f64 * 100.0,
            abs[0] as f64 / px as f64,
            abs[1] as f64 / px as f64,
            abs[2] as f64 / px as f64,
            gray_diff_px,
            gray_diff_px as f64 / px as f64 * 100.0,
            gray_abs as f64 / px as f64,
            chroma_abs[0] as f64 / px as f64,
            chroma_abs[1] as f64 / px as f64
        );
    }

    #[test]
    #[ignore]
    fn debug_iw44_numeric_ranges() {
        for file in ["carte.djvu", "colorbook.djvu", "chicken.djvu"] {
            let data = std::fs::read(assets_path().join(file)).unwrap();
            let parsed = crate::iff::parse(&data).unwrap();
            let chunks = extract_bg44_chunks(&parsed);
            if chunks.is_empty() {
                continue;
            }

            let mut img = IW44Image::new();
            for chunk in &chunks {
                img.decode_chunk(chunk).unwrap();
            }

            let summarize_decoder = |label: &str, dec: &IWDecoder| {
                let mut coef_min = i16::MAX;
                let mut coef_max = i16::MIN;
                let mut coef_edge = 0usize;
                for block in &dec.blocks {
                    for &v in block {
                        coef_min = coef_min.min(v);
                        coef_max = coef_max.max(v);
                        if v <= i16::MIN + 512 || v >= i16::MAX - 512 {
                            coef_edge += 1;
                        }
                    }
                }

                let bm = dec.get_bytemap(1);
                let mut bm_min = i16::MAX;
                let mut bm_max = i16::MIN;
                let mut bm_edge = 0usize;
                for &v in &bm.data {
                    bm_min = bm_min.min(v);
                    bm_max = bm_max.max(v);
                    if v <= i16::MIN + 512 || v >= i16::MAX - 512 {
                        bm_edge += 1;
                    }
                }

                eprintln!(
                    "{} {} coef=[{},{}] coef_edge={} bm=[{},{}] bm_edge={}",
                    file, label, coef_min, coef_max, coef_edge, bm_min, bm_max, bm_edge
                );
            };

            summarize_decoder("Y", img.y_codec.as_ref().unwrap());
            if let Some(cb) = img.cb_codec.as_ref() {
                summarize_decoder("Cb", cb);
            }
            if let Some(cr) = img.cr_codec.as_ref() {
                summarize_decoder("Cr", cr);
            }
        }
    }

    #[test]
    #[ignore]
    fn debug_carte_bg_chunk1_block_profile() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/carte_bg_1_ref.ppm");
        if !ref_path.exists() {
            return;
        }

        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let file = crate::iff::parse(&data).unwrap();
        let chunks = extract_bg44_chunks(&file);
        let mut img = IW44Image::new();
        img.decode_chunk(chunks[0]).unwrap();

        let actual = img.to_pixmap().unwrap().to_ppm();
        let expected = std::fs::read(ref_path).unwrap();
        let header_end = find_ppm_data_start(&actual);
        let a = &actual[header_end..];
        let e = &expected[header_end..];
        let w = img.width as usize;
        let h = img.height as usize;
        let bw = w.div_ceil(32);
        let bh = h.div_ceil(32);
        let mut block_diff = vec![0usize; bw * bh];
        let mut block_abs = vec![[0u64; 3]; bw * bh];
        let mut total = 0usize;
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 3;
                let bi = (y / 32) * bw + (x / 32);
                let dr = (a[i] as i32 - e[i] as i32).unsigned_abs() as u64;
                let dg = (a[i + 1] as i32 - e[i + 1] as i32).unsigned_abs() as u64;
                let db = (a[i + 2] as i32 - e[i + 2] as i32).unsigned_abs() as u64;
                if dr != 0 || dg != 0 || db != 0 {
                    block_diff[bi] += 1;
                    total += 1;
                }
                block_abs[bi][0] += dr;
                block_abs[bi][1] += dg;
                block_abs[bi][2] += db;
            }
        }

        let mut ranked = Vec::new();
        for by in 0..bh {
            for bx in 0..bw {
                let i = by * bw + bx;
                ranked.push((block_diff[i], block_abs[i], bx, by));
            }
        }
        ranked.sort_unstable_by(|a, b| b.0.cmp(&a.0));

        eprintln!(
            "carte chunk1 total_mismatch_px={} blocks={}x{}",
            total, bw, bh
        );
        for (rank, (diff, abs, bx, by)) in ranked.into_iter().take(12).enumerate() {
            eprintln!(
                "rank={} block=({}, {}) diff_px={} mean_abs=({:.3},{:.3},{:.3})",
                rank + 1,
                bx,
                by,
                diff,
                abs[0] as f64 / (32 * 32) as f64,
                abs[1] as f64 / (32 * 32) as f64,
                abs[2] as f64 / (32 * 32) as f64
            );
        }
    }

    #[test]
    #[ignore]
    fn debug_carte_bg_delay_candidates() {
        let ref_path = std::path::Path::new("/tmp/rdjvu_debug/carte_bg_4_ref.ppm");
        if !ref_path.exists() {
            return;
        }

        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let file = crate::iff::parse(&data).unwrap();
        let chunks = extract_bg44_chunks(&file);
        let expected = std::fs::read(ref_path).unwrap();

        for delay in 7u8..=13u8 {
            let mut mutated: Vec<Vec<u8>> = chunks.iter().map(|c| c.to_vec()).collect();
            mutated[0][8] = delay;

            let mut img = IW44Image::new();
            let mut ok = true;
            for chunk in &mutated {
                if img.decode_chunk(chunk).is_err() {
                    ok = false;
                    break;
                }
            }
            if !ok {
                eprintln!("carte bg delay={} decode_error", delay);
                continue;
            }

            let actual = img.to_pixmap().unwrap().to_ppm();
            let header_end = find_ppm_data_start(&actual);
            let a = &actual[header_end..];
            let e = &expected[header_end..];
            let px = (a.len().min(e.len())) / 3;
            let mut diff_px = 0usize;
            let mut diff_bytes = 0usize;
            for p in 0..px {
                let i = p * 3;
                let dr = a[i] != e[i];
                let dg = a[i + 1] != e[i + 1];
                let db = a[i + 2] != e[i + 2];
                if dr || dg || db {
                    diff_px += 1;
                }
                diff_bytes += dr as usize + dg as usize + db as usize;
            }
            eprintln!(
                "carte bg delay={} diff_px={} diff_bytes={}",
                delay, diff_px, diff_bytes
            );
        }
    }

    #[test]
    #[ignore]
    fn debug_color_delay_preadvance_candidate() {
        let cases = [
            ("carte", "carte.djvu", "/tmp/rdjvu_debug/carte_bg_4_ref.ppm"),
            (
                "colorbook",
                "colorbook.djvu",
                "/tmp/rdjvu_debug/colorbook_bg_ref.ppm",
            ),
            ("chicken", "chicken.djvu", "__golden__"),
        ];

        for (tag, file_name, ref_path) in cases {
            let data = std::fs::read(assets_path().join(file_name)).unwrap();
            let parsed = crate::iff::parse(&data).unwrap();
            let chunks = extract_bg44_chunks(&parsed);
            if chunks.is_empty() {
                continue;
            }

            let expected = if ref_path == "__golden__" {
                std::fs::read(golden_path().join("chicken_bg.ppm")).unwrap()
            } else {
                let rp = std::path::Path::new(ref_path);
                if !rp.exists() {
                    continue;
                }
                std::fs::read(rp).unwrap()
            };

            for preadvance in [false, true] {
                let img = decode_chunks_with_options(&chunks, preadvance).unwrap();
                let actual = img.to_pixmap().unwrap().to_ppm();
                let header_end = find_ppm_data_start(&actual);
                let a = &actual[header_end..];
                let e = &expected[header_end..];
                let px = (a.len().min(e.len())) / 3;
                let mut diff_px = 0usize;
                let mut diff_bytes = 0usize;
                for p in 0..px {
                    let i = p * 3;
                    let dr = a[i] != e[i];
                    let dg = a[i + 1] != e[i + 1];
                    let db = a[i + 2] != e[i + 2];
                    if dr || dg || db {
                        diff_px += 1;
                    }
                    diff_bytes += dr as usize + dg as usize + db as usize;
                }
                eprintln!(
                    "{} preadvance={} diff_px={} diff_bytes={}",
                    tag, preadvance, diff_px, diff_bytes
                );
            }
        }
    }

    #[test]
    #[ignore]
    fn debug_context_reset_candidates() {
        let cases = [
            ("carte", "carte.djvu", "/tmp/rdjvu_debug/carte_bg_4_ref.ppm"),
            (
                "colorbook",
                "colorbook.djvu",
                "/tmp/rdjvu_debug/colorbook_bg_ref.ppm",
            ),
            ("chicken", "chicken.djvu", "__golden__"),
        ];
        let variants = [
            ("baseline", false, false, false),
            ("chunk", true, false, false),
            ("slice", false, true, false),
            ("color_start", false, false, true),
        ];

        for (tag, file_name, ref_path) in cases {
            let data = std::fs::read(assets_path().join(file_name)).unwrap();
            let parsed = crate::iff::parse(&data).unwrap();
            let chunks = extract_bg44_chunks(&parsed);
            if chunks.is_empty() {
                continue;
            }

            let expected = if ref_path == "__golden__" {
                std::fs::read(golden_path().join("chicken_bg.ppm")).unwrap()
            } else {
                let rp = std::path::Path::new(ref_path);
                if !rp.exists() {
                    continue;
                }
                std::fs::read(rp).unwrap()
            };

            for (name, reset_each_chunk, reset_each_slice, reset_on_color_start) in variants {
                let img = decode_chunks_with_context_resets(
                    &chunks,
                    reset_each_chunk,
                    reset_each_slice,
                    reset_on_color_start,
                )
                .unwrap();
                let actual = img.to_pixmap().unwrap().to_ppm();
                let header_end = find_ppm_data_start(&actual);
                let a = &actual[header_end..];
                let e = &expected[header_end..];
                let px = (a.len().min(e.len())) / 3;
                let mut diff_px = 0usize;
                let mut diff_bytes = 0usize;
                for p in 0..px {
                    let i = p * 3;
                    let dr = a[i] != e[i];
                    let dg = a[i + 1] != e[i + 1];
                    let db = a[i + 2] != e[i + 2];
                    if dr || dg || db {
                        diff_px += 1;
                    }
                    diff_bytes += dr as usize + dg as usize + db as usize;
                }
                eprintln!(
                    "{} variant={} diff_px={} diff_bytes={}",
                    tag, name, diff_px, diff_bytes
                );
            }
        }
    }

    // --- Phase 6.2: Edge case tests ---

    #[test]
    fn iw44_empty_input() {
        let mut img = IW44Image::new();
        assert!(img.decode_chunk(&[]).is_err());
    }

    #[test]
    fn iw44_single_byte() {
        let mut img = IW44Image::new();
        let _ = img.decode_chunk(&[0x00]);
    }

    #[test]
    fn iw44_truncated_header() {
        let mut img = IW44Image::new();
        // Only 3 bytes — not enough for a valid IW44 chunk header
        let _ = img.decode_chunk(&[0x00, 0x01, 0x02]);
    }

    #[test]
    fn iw44_to_pixmap_before_decode() {
        // No chunks decoded yet — should produce an empty or minimal image
        let img = IW44Image::new();
        let result = img.to_pixmap();
        assert!(
            result.is_err() || {
                let pm = result.unwrap();
                pm.width == 0 || pm.height == 0
            }
        );
    }

    #[test]
    fn iw44_all_zeros() {
        let mut img = IW44Image::new();
        let _ = img.decode_chunk(&[0u8; 64]);
    }

    #[test]
    fn iw44_fuzz_crash_regression() {
        // Fuzz artifact may not exist in vendored copy — skip gracefully.
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fuzz/artifacts/fuzz_iw44/crash-cd05b0f41ddae1e44952cccf5e2b2ae825908e5e");
        if let Ok(data) = std::fs::read(path) {
            let mut img = IW44Image::new();
            if img.decode_chunk(&data).is_ok() {
                let _ = img.to_pixmap();
            }
        }
    }
}
