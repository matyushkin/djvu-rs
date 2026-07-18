//! CBZ (comic book archive) export.
//!
//! A CBZ is a ZIP of page images. Pages are rendered to RGBA, PNG-encoded and
//! stored uncompressed (PNG is already deflated). Building a page's PNG is
//! independent per page and CPU-heavy; only the ZIP writing must be serial (a
//! single `ZipWriter`). With the `parallel` feature bounded batches of PNGs are
//! built concurrently via rayon and written in index order — the same
//! render-parallel/write-serial split as the EPUB and PDF exporters (#298,
//! #598). Output bytes are identical to the sequential path.

use std::io::{Seek, Write};

use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::djvu_document::{DjVuDocument, DjVuPage, DocError};
use crate::djvu_render::{RenderError, RenderOptions, UserRotation, render_pixmap};

/// Errors during CBZ conversion.
#[derive(Debug, thiserror::Error)]
pub enum CbzError {
    /// Document model error.
    #[error("document error: {0}")]
    Doc(#[from] DocError),
    /// Render error.
    #[error("render error: {0}")]
    Render(#[from] RenderError),
    /// ZIP I/O error.
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    /// I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// PNG encoding error.
    #[error("png encode error: {0}")]
    Png(String),
}

/// Options for CBZ conversion.
#[derive(Debug, Clone)]
pub struct CbzOptions {
    /// Output resolution in DPI (pages are scaled from their native DPI).
    pub dpi: u32,
    /// Extra user rotation applied on top of the INFO-chunk rotation.
    pub rotation: UserRotation,
    /// 0-based page indices to export; `None` = all pages in order.
    pub pages: Option<Vec<usize>>,
}

impl Default for CbzOptions {
    fn default() -> Self {
        CbzOptions {
            dpi: 150,
            rotation: UserRotation::None,
            pages: None,
        }
    }
}

/// Convert a DjVu document to an in-memory CBZ archive.
pub fn djvu_to_cbz(doc: &DjVuDocument, opts: &CbzOptions) -> Result<Vec<u8>, CbzError> {
    let cursor = std::io::Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    write_pages(&mut zip, doc, opts)?;
    Ok(zip.finish()?.into_inner())
}

/// Write every requested page into `zip` as `page_%04d.png` entries.
///
/// Exposed crate-internally so the CLI can stream straight to a file instead
/// of buffering the archive.
pub fn write_pages<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    doc: &DjVuDocument,
    opts: &CbzOptions,
) -> Result<(), CbzError> {
    let entry_opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    // #629: pages render on cold clones so decode caches drop per page
    // instead of accumulating O(pages) on the document.
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        // Keep the parallel speedup while retaining only one bounded batch of
        // rendered PNGs. The ZIP writer remains serial, so each batch is
        // written in page order before the next batch is rendered.
        let chunk = rayon::current_num_threads().max(1) * 8;
        match &opts.pages {
            Some(indices) => {
                for (chunk_index, indices) in indices.chunks(chunk).enumerate() {
                    let pngs: Vec<Vec<u8>> = indices
                        .par_iter()
                        .map(|&i| build_page_png(&doc.page(i)?.clone(), opts))
                        .collect::<Result<_, CbzError>>()?;
                    for (offset, png) in pngs.iter().enumerate() {
                        write_page_png(zip, chunk_index * chunk + offset + 1, png, entry_opts)?;
                    }
                }
            }
            None => {
                let page_count = doc.page_count();
                let mut start = 0;
                while start < page_count {
                    let end = (start + chunk).min(page_count);
                    let pngs: Vec<Vec<u8>> = (start..end)
                        .into_par_iter()
                        .map(|i| build_page_png(&doc.page(i)?.clone(), opts))
                        .collect::<Result<_, CbzError>>()?;
                    for (offset, png) in pngs.iter().enumerate() {
                        write_page_png(zip, start + offset + 1, png, entry_opts)?;
                    }
                    start = end;
                }
            }
        }
    }

    #[cfg(not(feature = "parallel"))]
    match &opts.pages {
        Some(indices) => {
            for (index, &page_index) in indices.iter().enumerate() {
                let png = build_page_png(&doc.page(page_index)?.clone(), opts)?;
                write_page_png(zip, index + 1, &png, entry_opts)?;
            }
        }
        None => {
            for page_index in 0..doc.page_count() {
                let png = build_page_png(&doc.page(page_index)?.clone(), opts)?;
                write_page_png(zip, page_index + 1, &png, entry_opts)?;
            }
        }
    }
    Ok(())
}

fn write_page_png<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    number: usize,
    png: &[u8],
    entry_opts: SimpleFileOptions,
) -> Result<(), CbzError> {
    zip.start_file(format!("page_{number:04}.png"), entry_opts)?;
    zip.write_all(png)?;
    Ok(())
}

/// Render one page at the target DPI, apply user rotation, encode to PNG.
fn build_page_png(page: &DjVuPage, opts: &CbzOptions) -> Result<Vec<u8>, CbzError> {
    let (w, h) = crate::export_common::size_at_dpi(page, opts.dpi as f32);
    let pixmap = render_pixmap(
        page,
        &RenderOptions {
            width: w,
            height: h,
            ..RenderOptions::default()
        },
    )?;
    let pixmap = match opts.rotation {
        UserRotation::None => pixmap,
        UserRotation::Cw90 => pixmap.rotate_cw90(),
        UserRotation::Rot180 => pixmap.rotate_180(),
        UserRotation::Ccw90 => pixmap.rotate_ccw90(),
    };

    // Pages are always opaque (alpha=255 inline — ALPHA_INL), so encode RGB:
    // 25% less raw data into deflate, smaller archives, identical pixels
    // (#599).
    let mut rgb = Vec::with_capacity(pixmap.data.len() / 4 * 3);
    for px in pixmap.data.chunks_exact(4) {
        rgb.extend_from_slice(&px[..3]);
    }
    let mut buf = Vec::new();
    {
        let mut enc =
            png::Encoder::new(std::io::Cursor::new(&mut buf), pixmap.width, pixmap.height);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc
            .write_header()
            .map_err(|e| CbzError::Png(e.to_string()))?;
        writer
            .write_image_data(&rgb)
            .map_err(|e| CbzError::Png(e.to_string()))?;
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_doc(name: &str) -> DjVuDocument {
        let data = std::fs::read(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name),
        )
        .unwrap();
        DjVuDocument::parse(&data).unwrap()
    }

    /// The archive is a valid ZIP with one stored PNG entry per page, named
    /// `page_%04d.png` in order.
    #[test]
    fn cbz_has_one_png_entry_per_page() {
        let doc = load_doc("navm_fgbz.djvu");
        let cbz = djvu_to_cbz(&doc, &CbzOptions::default()).unwrap();
        let mut archive =
            zip::ZipArchive::new(std::io::Cursor::new(&cbz)).expect("valid zip archive");
        assert_eq!(archive.len(), doc.page_count());
        for i in 0..archive.len() {
            let entry = archive.by_index(i).unwrap();
            assert_eq!(entry.name(), format!("page_{:04}.png", i + 1));
        }
        // PNG magic on the first entry
        use std::io::Read;
        let mut first = archive.by_index(0).unwrap();
        let mut magic = [0u8; 8];
        first.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, b"\x89PNG\r\n\x1a\n");
    }

    /// Byte-determinism: two exports of the same document are identical (no
    /// wall-clock timestamps, no ordering nondeterminism from the parallel
    /// build).
    #[test]
    fn cbz_output_is_deterministic() {
        let doc = load_doc("boy.djvu");
        let a = djvu_to_cbz(&doc, &CbzOptions::default()).unwrap();
        let b = djvu_to_cbz(&doc, &CbzOptions::default()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn cbz_vec_and_writer_exports_are_byte_identical() {
        let doc = load_doc("boy.djvu");
        let opts = CbzOptions::default();
        let expected = djvu_to_cbz(&doc, &opts).unwrap();

        let cursor = std::io::Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(cursor);
        write_pages(&mut zip, &doc, &opts).unwrap();
        let actual = zip.finish().unwrap().into_inner();

        assert_eq!(actual, expected);
    }

    /// Page subset and rotation options are honoured.
    #[test]
    fn cbz_page_subset_and_rotation() {
        let doc = load_doc("navm_fgbz.djvu");
        let opts = CbzOptions {
            pages: Some(vec![0, 2]),
            rotation: UserRotation::Cw90,
            ..CbzOptions::default()
        };
        let cbz = djvu_to_cbz(&doc, &opts).unwrap();
        let archive = zip::ZipArchive::new(std::io::Cursor::new(&cbz)).unwrap();
        assert_eq!(archive.len(), 2);
    }
}
