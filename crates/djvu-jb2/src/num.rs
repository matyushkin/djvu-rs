//! The binary-tree context store for variable-length integers, `NumContext`.

use super::*;

// ────────────────────────────────────────────────────────────────────────────
// NumContext: binary-tree arena for variable-length integer decoding
// ────────────────────────────────────────────────────────────────────────────

/// Binary-tree context store used to encode/decode variable-length integers
/// with ZP.
///
/// Each node in the tree holds one adaptive ZP context byte. Nodes are
/// allocated lazily as the coder traverses the tree. Shared between the
/// decoder (this module) and the `std`-only [`encode`] module.
pub(crate) struct NumContext {
    pub(crate) ctx: Vec<u8>,
    pub(super) left: Vec<u32>,
    pub(super) right: Vec<u32>,
}

impl NumContext {
    pub(crate) fn new() -> Self {
        // Index 0 = unused sentinel; index 1 = root.
        NumContext {
            ctx: vec![0, 0],
            left: vec![0, 0],
            right: vec![0, 0],
        }
    }

    pub(crate) fn root(&self) -> usize {
        1
    }

    pub(crate) fn get_left(&mut self, node: usize) -> usize {
        if self.left[node] == 0 {
            let idx = self.ctx.len() as u32;
            self.ctx.push(0);
            self.left.push(0);
            self.right.push(0);
            self.left[node] = idx;
        }
        self.left[node] as usize
    }

    pub(crate) fn get_right(&mut self, node: usize) -> usize {
        if self.right[node] == 0 {
            let idx = self.ctx.len() as u32;
            self.ctx.push(0);
            self.left.push(0);
            self.right.push(0);
            self.right[node] = idx;
        }
        self.right[node] as usize
    }
}

/// Decode a variable-length integer in the range `[low, high]` using ZP
/// with a binary-tree context store.
pub(super) fn decode_num(zp: &mut ZpDecoder<'_>, ctx: &mut NumContext, low: i32, high: i32) -> i32 {
    let mut low = low;
    let mut high = high;
    let mut negative = false;
    let mut cutoff: i32 = 0;
    let mut phase: u32 = 1;
    let mut range: u32 = 0xffff_ffff;
    let mut node = ctx.root();

    while range != 1 {
        let decision = if low >= cutoff {
            true
        } else if high >= cutoff {
            zp.decode_bit(&mut ctx.ctx[node])
        } else {
            false
        };

        node = if decision {
            ctx.get_right(node)
        } else {
            ctx.get_left(node)
        };

        match phase {
            1 => {
                negative = !decision;
                if negative {
                    let temp = -low - 1;
                    low = -high - 1;
                    high = temp;
                }
                phase = 2;
                cutoff = 1;
            }
            2 => {
                if !decision {
                    phase = 3;
                    range = ((cutoff + 1) / 2) as u32;
                    if range == 1 {
                        // range is already 1; set cutoff to 0 to terminate the loop.
                        cutoff = 0;
                    } else {
                        cutoff -= (range / 2) as i32;
                    }
                } else {
                    cutoff = cutoff * 2 + 1;
                }
            }
            3 => {
                range /= 2;
                if range == 0 {
                    range = 1;
                }
                if range != 1 {
                    if !decision {
                        cutoff -= (range / 2) as i32;
                    } else {
                        cutoff += (range / 2) as i32;
                    }
                } else if !decision {
                    cutoff -= 1;
                }
            }
            _ => {
                // Unreachable: phase cycles through 1, 2, 3 only.
                // Use a saturating decrement to keep range moving toward 1.
                range = range.saturating_sub(1);
            }
        }
    }

    if negative { -cutoff - 1 } else { cutoff }
}
