//! Symbol placement: the rolling baseline and symbol coordinate decoding.

use super::*;

// ────────────────────────────────────────────────────────────────────────────
// Baseline: rolling median-of-3 for vertical symbol positioning
// ────────────────────────────────────────────────────────────────────────────

pub(super) struct Baseline {
    pub(super) arr: [i32; 3],
    pub(super) index: i32,
}

impl Baseline {
    pub(super) fn new() -> Self {
        Baseline {
            arr: [0, 0, 0],
            index: -1,
        }
    }

    pub(super) fn fill(&mut self, val: i32) {
        self.arr = [val, val, val];
    }

    pub(super) fn add(&mut self, val: i32) {
        self.index += 1;
        if self.index == 3 {
            self.index = 0;
        }
        self.arr[self.index as usize] = val;
    }

    pub(super) fn get_val(&self) -> i32 {
        let (a, b, c) = (self.arr[0], self.arr[1], self.arr[2]);
        if (a >= b && a <= c) || (a <= b && a >= c) {
            a
        } else if (b >= a && b <= c) || (b <= a && b >= c) {
            b
        } else {
            c
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Symbol coordinate decoding
// ────────────────────────────────────────────────────────────────────────────

/// ZP coder contexts used exclusively for symbol coordinate decoding.
pub(super) struct CoordContexts {
    pub(super) offset_type: u8,
    pub(super) hoff: NumContext,
    pub(super) voff: NumContext,
    pub(super) shoff: NumContext,
    pub(super) svoff: NumContext,
}

impl CoordContexts {
    pub(super) fn new() -> Self {
        Self {
            offset_type: 0,
            hoff: NumContext::new(),
            voff: NumContext::new(),
            shoff: NumContext::new(),
            svoff: NumContext::new(),
        }
    }
}

/// Running layout state for symbol positioning within a JB2 image.
pub(super) struct LayoutState {
    pub(super) first_left: i32,
    pub(super) first_bottom: i32,
    pub(super) last_right: i32,
    pub(super) baseline: Baseline,
}

impl LayoutState {
    pub(super) fn new(image_height: i32) -> Self {
        Self {
            first_left: -1,
            first_bottom: image_height - 1,
            last_right: 0,
            baseline: Baseline::new(),
        }
    }
}

pub(super) fn decode_symbol_coords(
    zp: &mut ZpDecoder<'_>,
    coord_ctx: &mut CoordContexts,
    layout: &mut LayoutState,
    sym_width: i32,
    sym_height: i32,
) -> (i32, i32) {
    let new_line = zp.decode_bit(&mut coord_ctx.offset_type);

    let (x, y) = if new_line {
        let hoff = decode_num(zp, &mut coord_ctx.hoff, -262143, 262142);
        let voff = decode_num(zp, &mut coord_ctx.voff, -262143, 262142);
        let nx = layout.first_left + hoff;
        let ny = layout.first_bottom + voff - sym_height + 1;
        layout.first_left = nx;
        layout.first_bottom = ny;
        layout.baseline.fill(ny);
        (nx, ny)
    } else {
        let hoff = decode_num(zp, &mut coord_ctx.shoff, -262143, 262142);
        let voff = decode_num(zp, &mut coord_ctx.svoff, -262143, 262142);
        (layout.last_right + hoff, layout.baseline.get_val() + voff)
    };

    layout.baseline.add(y);
    layout.last_right = x + sym_width - 1;
    (x, y)
}
