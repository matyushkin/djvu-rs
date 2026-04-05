//! `image::ImageDecoder` integration for DjVu pages.
//!
//! This module provides [`DjVuDecoder`], which implements the
//! [`image::ImageDecoder`] trait from the `image` crate, making djvu-rs a
//! first-class image format usable anywhere image-rs pipelines are used.
//!
//! ## Key public types
//!
//! - [`DjVuDecoder`] — implements `image::ImageDecoder` for a single DjVu page
//! - [`ImageCompatError`] — typed errors from this module
//!
//! ## Usage
//!
//! ```no_run
//! use djvu_rs::{DjVuDocument, image_compat::DjVuDecoder};
//! use image::ImageDecoder;
//!
//! let data = std::fs::read("file.djvu").unwrap();
//! let doc = DjVuDocument::parse(&data).unwrap();
//! let page = doc.page(0).unwrap();
//!
//! let decoder = DjVuDecoder::new(page).unwrap();
//! let (w, h) = decoder.dimensions();
//! let mut buf = vec![0u8; (w * h * 4) as usize];
//! decoder.read_image(&mut buf).unwrap();
//! ```

use std::io::Cursor;

use image::{
    ColorType, ImageDecoder, ImageResult,
    error::{DecodingError, ImageError, ImageFormatHint},
};

use crate::djvu_document::DjVuPage;
use crate::djvu_render::{RenderError, RenderOptions};

// ---- Error ------------------------------------------------------------------

/// Errors from the image-rs integration layer.
#[derive(Debug, thiserror::Error)]
pub enum ImageCompatError {
    /// The underlying render pipeline failed.
    #[error("render error: {0}")]
    Render(#[from] RenderError),
}

impl From<ImageCompatError> for ImageError {
    fn from(e: ImageCompatError) -> Self {
        ImageError::Decoding(DecodingError::new(
            ImageFormatHint::Name("DjVu".to_string()),
            e,
        ))
    }
}

// ---- DjVuDecoder ------------------------------------------------------------

/// An `image::ImageDecoder` for a single DjVu page.
///
/// By default renders at the native page resolution. Use [`DjVuDecoder::with_size`]
/// to override the output dimensions.
pub struct DjVuDecoder<'a> {
    page: &'a DjVuPage,
    width: u32,
    height: u32,
}

impl<'a> DjVuDecoder<'a> {
    /// Construct a decoder from a [`DjVuPage`] reference.
    ///
    /// The output dimensions default to the native page size from the INFO chunk.
    pub fn new(page: &'a DjVuPage) -> Result<Self, ImageCompatError> {
        Ok(Self {
            width: page.width() as u32,
            height: page.height() as u32,
            page,
        })
    }

    /// Override the output dimensions.
    ///
    /// The rendered image will be scaled to `width × height` using bilinear
    /// interpolation via [`RenderOptions`].
    #[must_use]
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Render the page into an RGBA byte buffer.
    fn render_to_vec(&self) -> Result<Vec<u8>, ImageCompatError> {
        let opts = RenderOptions {
            width: self.width,
            height: self.height,
            scale: self.width as f32 / self.page.width().max(1) as f32,
            ..RenderOptions::default()
        };
        let size = (self.width as usize)
            .saturating_mul(self.height as usize)
            .saturating_mul(4);
        let mut buf = vec![0u8; size];
        self.page.render_into(&opts, &mut buf)?;
        Ok(buf)
    }
}

// ---- ImageDecoder impl ------------------------------------------------------

impl<'a> ImageDecoder<'a> for DjVuDecoder<'a> {
    type Reader = Cursor<Vec<u8>>;

    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn color_type(&self) -> ColorType {
        ColorType::Rgba8
    }

    #[allow(deprecated)]
    fn into_reader(self) -> ImageResult<Self::Reader> {
        let data = self.render_to_vec().map_err(ImageError::from)?;
        Ok(Cursor::new(data))
    }

    fn read_image(self, buf: &mut [u8]) -> ImageResult<()> {
        let data = self.render_to_vec().map_err(ImageError::from)?;
        if buf.len() != data.len() {
            return Err(ImageError::Decoding(DecodingError::new(
                ImageFormatHint::Name("DjVu".to_string()),
                format!(
                    "buffer size mismatch: expected {}, got {}",
                    data.len(),
                    buf.len()
                ),
            )));
        }
        buf.copy_from_slice(&data);
        Ok(())
    }
}
