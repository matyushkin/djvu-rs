//! `image::ImageDecoder` integration for DjVu pages.
//!
//! This module provides [`DjVuDecoder`], which implements the
//! [`image::ImageDecoder`] and [`image::ImageDecoderRect`] traits from the
//! `image` crate, making djvu-rs a first-class image format usable anywhere
//! image-rs pipelines are used.
//!
//! ## Key public types
//!
//! - [`DjVuDecoder`] — implements `image::ImageDecoder` and `image::ImageDecoderRect`
//!   for a single DjVu page
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
    ColorType, ImageDecoder, ImageDecoderRect, ImageResult,
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

/// An `image::ImageDecoder` and `image::ImageDecoderRect` for a single DjVu page.
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

    /// Render the full page into an RGBA byte buffer.
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

// ---- ImageDecoderRect impl --------------------------------------------------

impl<'a> ImageDecoderRect<'a> for DjVuDecoder<'a> {
    /// Decode a rectangular region of the page.
    ///
    /// DjVu does not natively support partial rendering; this implementation
    /// renders the full page and copies out the requested rectangle.
    /// The `buf` slice must be at least `bytes_per_pixel * width * height` bytes.
    #[allow(deprecated)]
    fn read_rect_with_progress<F: Fn(image::Progress)>(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        buf: &mut [u8],
        _progress_callback: F,
    ) -> ImageResult<()> {
        let bytes_per_pixel = self.color_type().bytes_per_pixel() as usize;
        let row_stride = self.width as usize * bytes_per_pixel;
        let rect_row_bytes = width as usize * bytes_per_pixel;

        // Validate rectangle stays within image bounds.
        let x_end = x.checked_add(width).ok_or_else(|| {
            ImageError::Decoding(DecodingError::new(
                ImageFormatHint::Name("DjVu".to_string()),
                "rectangle x+width overflows u32",
            ))
        })?;
        let y_end = y.checked_add(height).ok_or_else(|| {
            ImageError::Decoding(DecodingError::new(
                ImageFormatHint::Name("DjVu".to_string()),
                "rectangle y+height overflows u32",
            ))
        })?;
        if x_end > self.width || y_end > self.height {
            return Err(ImageError::Decoding(DecodingError::new(
                ImageFormatHint::Name("DjVu".to_string()),
                format!(
                    "rectangle ({x},{y},{width},{height}) out of image bounds ({}×{})",
                    self.width, self.height
                ),
            )));
        }

        let full = self.render_to_vec().map_err(ImageError::from)?;

        for row in 0..height as usize {
            let src_y = y as usize + row;
            let src_start = src_y * row_stride + x as usize * bytes_per_pixel;
            let src_end = src_start + rect_row_bytes;
            let dst_start = row * rect_row_bytes;
            let dst_end = dst_start + rect_row_bytes;

            buf[dst_start..dst_end].copy_from_slice(&full[src_start..src_end]);
        }

        Ok(())
    }
}
