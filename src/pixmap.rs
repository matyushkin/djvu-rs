//! Backwards-compatible pixmap module.
//!
//! The implementation lives in the standalone `djvu-pixmap` crate. This module
//! preserves the historical `djvu_rs::pixmap::{Pixmap, GrayPixmap}` path.

pub use djvu_pixmap::{GrayPixmap, Pixmap};
