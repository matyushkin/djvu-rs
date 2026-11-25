//! Error types of the codec layers.
//!
//! This module re-exports the errors of the workspace codec crates:
//! - [`IffError`] — errors from the IFF container parser (`djvu-iff`)
//! - [`BzzError`] — errors from the BZZ decompressor (`djvu-bzz`)
//! - [`Jb2Error`] — errors from the JB2 bilevel image decoder (`djvu-jb2`)
//! - [`Iw44Error`] — errors from the IW44 wavelet image decoder (`djvu-iw44`)
//!
//! The document model reports them wrapped in [`crate::DocError`], the
//! renderer in [`crate::djvu_render::RenderError`], and the
//! [`crate::Document`] facade in [`crate::Error`]. See also
//! `text::TextError` and `annotation::AnnotationError`.

pub use djvu_iff::IffError;

pub use djvu_jb2::Jb2Error;

pub use djvu_iw44::Iw44Error;

pub use djvu_bzz::BzzError;
