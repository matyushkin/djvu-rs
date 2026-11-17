//! IW44 decoder re-export.
//!
//! The implementation lives in the standalone `djvu-iw44` crate; this module
//! re-exports it as `djvu_rs::iw44::*`. (The module was called `iw44_new`
//! before 0.23; 1.0 removed that alias.)

pub use djvu_iw44::*;
