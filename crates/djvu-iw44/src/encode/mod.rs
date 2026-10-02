//! IW44 wavelet encoder — produces BG44/FG44/TH44 chunk payloads.
//!
//! ## Algorithm overview
//!
//! 1. Convert RGB → IW44 YCbCr (or accept grayscale directly).
//! 2. Apply the forward IW44 wavelet transform to each plane.
//! 3. Gather transformed coefficients into 32×32 blocks (zigzag scan).
//! 4. Progressively encode bands 0–9 using ZP arithmetic coding.
//! 5. Assemble BG44 chunk payloads with the required headers.
//!
//! The forward transform is the exact inverse of the analysis filter in
//! `iw44_new::inverse_wavelet_transform`. Passes run from s=1 (finest) to
//! s=16 (coarsest); within each pass the predict step is undone before the
//! lifting step (reversed vs the synthesis filter).

use djvu_pixmap::{GrayPixmap, Pixmap};
use djvu_zp::encoder::ZpEncoder;

// Band/quant/state-flag/zigzag spec data is shared with the decoder (one source
// of truth) rather than re-declared here — see the decoder modules of the crate root.
use crate::{
    ACTIVE, BAND_BUCKETS, CoefBlock, NEW, QUANT_HI_INIT, QUANT_LO_INIT, UNK, ZERO, band0_dispatch,
    prelim_flags_bucket,
};

#[cfg(feature = "iw44-probe")]
pub mod probe;

// ---- Modules ----------------------------------------------------------------
//
// One file per concern (#902). Every item a sibling needs is `pub(super)`:
// the same reach a private item had when this module was one file.

mod banded;
mod encoder;
mod options;
mod plane;
mod wavelet;

use banded::*;
pub use encoder::*;
pub use options::*;
use plane::*;
use wavelet::*;

#[cfg(test)]
mod loss_diagnostics;
#[cfg(test)]
mod tests;
