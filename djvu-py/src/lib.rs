use std::ffi::{CString, c_int, c_void};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::Arc;

use pyo3::create_exception;
use pyo3::exceptions::{PyBufferError, PyIndexError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::pybacked::PyBackedBytes;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyBytes, PyDict, PyType};
use pythonize::{depythonize, pythonize};

use djvu_rs::annotation::{Annotation, MapArea};
use djvu_rs::cbz::CbzOptions;
use djvu_rs::djvm::{self, DjvmError, UnreachablePolicy};
use djvu_rs::djvu_document::{DjVuBookmark, DjVuDocument};
use djvu_rs::djvu_encode::{
    EncodeError as RustEncodeError, EncodeQuality, PageEncoder, classify_content,
    encode_djvm_layered_shared_streaming,
};
use djvu_rs::djvu_mut::{DjVuDocumentMut, MutError};
use djvu_rs::djvu_render::{Quality, RenderRect, RenderRequest, UserRotation};
use djvu_rs::editor::DocumentEditor;
use djvu_rs::epub::EpubOptions;
use djvu_rs::jb2_encode::encode_djvm_bundle_jb2_with_thumbnails;
use djvu_rs::metadata::DjVuMetadata;
use djvu_rs::pdf::PdfOptions;
use djvu_rs::segment::{SegmentOptions, segment_page};
use djvu_rs::text::TextLayer;
use djvu_rs::tiff_export::{TiffBilevelCompression, TiffMode, TiffOptions};

create_exception!(
    djvu_rs,
    Error,
    pyo3::exceptions::PyException,
    "Base exception for all djvu-rs Python binding errors."
);
create_exception!(
    djvu_rs,
    DecodeError,
    Error,
    "Document parse, decode, or render failure."
);
create_exception!(
    djvu_rs,
    IoError,
    Error,
    "Filesystem or I/O failure while opening a DjVu document."
);
create_exception!(
    djvu_rs,
    ExportError,
    Error,
    "Conversion to PDF, EPUB, CBZ or TIFF failed."
);
/// `djvu_rs.PageIndexError`: page index out of range.
///
/// It inherits both `djvu_rs.Error` and the built-in `IndexError`, so
/// `except djvu_rs.Error` and `except IndexError` both catch it.
/// `create_exception!` takes one base only, so the module initialiser builds
/// the class with `type(name, bases, namespace)` and stores it here.
struct PageIndexError;

static PAGE_INDEX_ERROR: PyOnceLock<Py<PyType>> = PyOnceLock::new();

impl PageIndexError {
    fn type_object(py: Python<'_>) -> PyResult<&Bound<'_, PyType>> {
        PAGE_INDEX_ERROR
            .get_or_try_init(py, || {
                let namespace = PyDict::new(py);
                namespace.set_item("__doc__", "Page index is out of range for this document.")?;
                namespace.set_item("__module__", "djvu_rs")?;
                let bases = (py.get_type::<Error>(), py.get_type::<PyIndexError>());
                let class = py
                    .get_type::<PyType>()
                    .call1(("PageIndexError", bases, namespace))?;
                Ok(class.cast_into::<PyType>()?.unbind())
            })
            .map(|class| class.bind(py))
    }

    fn new_err(message: String) -> PyErr {
        Python::attach(|py| match Self::type_object(py) {
            Ok(class) => PyErr::from_type(class.clone(), message),
            Err(error) => error,
        })
    }
}
create_exception!(
    djvu_rs,
    EditError,
    Error,
    "An edit was rejected, or the edited document could not be saved."
);
create_exception!(
    djvu_rs,
    EncodeError,
    Error,
    "An image could not be encoded to DjVu."
);

// ---- Document model as Python data ----------------------------------------
//
// Annotations, text layers, metadata and bookmarks cross the boundary as plain
// dicts and lists in the serde shape of the Rust models (via `pythonize`), so
// a value read from a page can be changed and written straight back. Missing
// optional keys take their defaults: `{"title": "..."}` is valid metadata.

fn to_python<'py, T: serde::Serialize + ?Sized>(
    py: Python<'py>,
    value: &T,
) -> PyResult<Bound<'py, PyAny>> {
    pythonize(py, value).map_err(PyErr::from)
}

/// Read `value` as the Rust model `T`; `what` names it in the error.
fn from_python<T: serde::de::DeserializeOwned>(
    value: &Bound<'_, PyAny>,
    what: &str,
) -> PyResult<T> {
    depythonize(value)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("invalid {what}: {e}")))
}

fn document_metadata<'py>(
    py: Python<'py>,
    doc: &DjVuDocument,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    let meta = doc
        .metadata()
        .map_err(|e| DecodeError::new_err(format!("{e}")))?;
    meta.map(|m| to_python(py, &m)).transpose()
}

fn page_annotations<'py>(
    py: Python<'py>,
    doc: &DjVuDocument,
    index: usize,
) -> PyResult<Option<(Bound<'py, PyAny>, Bound<'py, PyAny>)>> {
    let page = doc
        .page(index)
        .map_err(|e| PageIndexError::new_err(format!("{e}")))?;
    let parsed = py.detach(|| {
        page.annotations()
            .map_err(|e| DecodeError::new_err(format!("{e}")))
    })?;
    parsed
        .map(|(annotation, areas)| Ok((to_python(py, &annotation)?, to_python(py, &areas)?)))
        .transpose()
}

fn page_text_layer<'py>(
    py: Python<'py>,
    doc: &DjVuDocument,
    index: usize,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    let page = doc
        .page(index)
        .map_err(|e| PageIndexError::new_err(format!("{e}")))?;
    let layer = py.detach(|| {
        page.text_layer()
            .map_err(|e| DecodeError::new_err(format!("{e}")))
    })?;
    layer.map(|l| to_python(py, &l)).transpose()
}

fn edit_error(e: MutError) -> PyErr {
    match e {
        MutError::PageOutOfRange { .. } => PageIndexError::new_err(format!("{e}")),
        other => EditError::new_err(format!("{other}")),
    }
}

// ---- Structural edits --------------------------------------------------------
//
// Module-level functions over `djvu_rs::djvm`. Each takes whole documents as
// bytes and returns a new document as bytes; none of them decodes a page. All
// release the GIL: the work is copying, and a book can be hundreds of megabytes.

fn djvm_error(e: DjvmError) -> PyErr {
    match e {
        DjvmError::PageRangeOutOfBounds { .. } | DjvmError::PageIndexOutOfBounds { .. } => {
            PageIndexError::new_err(format!("{e}"))
        }
        DjvmError::Iff(_) | DjvmError::Doc(_) => DecodeError::new_err(format!("{e}")),
        // The crate words this one for `to_indirect`; say what applies here.
        DjvmError::NotBundledDjvm => {
            EditError::new_err("the document is not a bundled multi-page (FORM:DJVM) file")
        }
        other => EditError::new_err(format!("{other}")),
    }
}

/// Join documents into one bundled multi-page document, in the given order.
#[pyfunction]
fn merge<'py>(py: Python<'py>, documents: Vec<PyBackedBytes>) -> PyResult<Bound<'py, PyBytes>> {
    let merged = py.detach(|| {
        let parts: Vec<&[u8]> = documents.iter().map(|d| &**d).collect();
        djvm::merge(&parts).map_err(djvm_error)
    })?;
    Ok(PyBytes::new(py, &merged))
}

/// Copy pages `start..end` (0-based, `end` excluded) into a new document.
#[pyfunction]
fn split<'py>(
    py: Python<'py>,
    document: PyBackedBytes,
    start: usize,
    end: usize,
) -> PyResult<Bound<'py, PyBytes>> {
    let part = py.detach(|| djvm::split(&document, start, end).map_err(djvm_error))?;
    Ok(PyBytes::new(py, &part))
}

/// Remove the pages at the given 0-based indices from a bundled document.
///
/// Shared data that only the removed pages used is dropped as well.
#[pyfunction]
fn remove_pages<'py>(
    py: Python<'py>,
    document: PyBackedBytes,
    indices: Vec<usize>,
) -> PyResult<Bound<'py, PyBytes>> {
    let rebuilt = py.detach(|| {
        djvm::remove_pages(&document, &indices, UnreachablePolicy::GarbageCollect)
            .map(|removal| removal.document)
            .map_err(djvm_error)
    })?;
    Ok(PyBytes::new(py, &rebuilt))
}

// ---- Encoding ----------------------------------------------------------------
//
// `encode_page` turns one image into a single-page DjVu file. The pixels are
// copied out of the Python object first, so the codec work runs without the GIL.

/// Copy `image` into an RGBA pixmap.
///
/// Accepts a `Pixmap`, a PIL image, a numpy array of `uint8` (H x W, H x W x 3
/// or H x W x 4), or raw RGBA `bytes` together with `size`.
fn image_to_rgba(image: &Bound<'_, PyAny>, size: Option<(u32, u32)>) -> PyResult<djvu_rs::Pixmap> {
    let (width, height, channels, data): (u32, u32, usize, Vec<u8>) = if let Some((w, h)) = size {
        let bytes: PyBackedBytes = image.extract().map_err(|_| {
            PyTypeError::new_err("with `size`, the image must be RGBA bytes or a bytearray")
        })?;
        (w, h, 4, bytes.to_vec())
    } else if let Ok(pixmap) = image.cast::<Pixmap>() {
        let pixmap = pixmap.borrow();
        (pixmap.width, pixmap.height, 4, pixmap.data.clone())
    } else if image.hasattr("convert")? && image.hasattr("size")? {
        // A PIL image, in any mode.
        let rgba = image.call_method1("convert", ("RGBA",))?;
        let (w, h): (u32, u32) = rgba.getattr("size")?.extract()?;
        let bytes: PyBackedBytes = rgba.call_method0("tobytes")?.extract()?;
        (w, h, 4, bytes.to_vec())
    } else if image.hasattr("shape")? && image.hasattr("tobytes")? {
        // A numpy array.
        if image.getattr("dtype")?.str()?.to_string_lossy() != "uint8" {
            return Err(PyTypeError::new_err("the array must have dtype uint8"));
        }
        let shape: Vec<usize> = image.getattr("shape")?.extract()?;
        let (h, w, channels) = match shape[..] {
            [h, w] => (h, w, 1),
            [h, w, c @ (1 | 3 | 4)] => (h, w, c),
            _ => {
                return Err(PyValueError::new_err(format!(
                    "the array must have the shape (H, W), (H, W, 3) or (H, W, 4), not {shape:?}"
                )));
            }
        };
        let too_large = || PyValueError::new_err("the array is too large");
        let w = u32::try_from(w).map_err(|_| too_large())?;
        let h = u32::try_from(h).map_err(|_| too_large())?;
        let bytes: PyBackedBytes = image.call_method0("tobytes")?.extract()?;
        (w, h, channels, bytes.to_vec())
    } else if image.extract::<PyBackedBytes>().is_ok() {
        return Err(PyTypeError::new_err(
            "raw RGBA bytes need `size=(width, height)`",
        ));
    } else {
        return Err(PyTypeError::new_err(
            "the image must be a Pixmap, a PIL image, a numpy array, or RGBA bytes with `size`",
        ));
    };

    if width == 0 || height == 0 {
        return Err(PyValueError::new_err("the image is empty"));
    }
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .filter(|n| n.checked_mul(channels) == Some(data.len()))
        .ok_or_else(|| {
            PyValueError::new_err(format!(
                "{} bytes do not match a {width} x {height} image with {channels} bytes per pixel",
                data.len()
            ))
        })?;
    let data = match channels {
        4 => data,
        3 => {
            let mut rgba = Vec::with_capacity(pixels * 4);
            for px in data.as_chunks::<3>().0 {
                rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
            rgba
        }
        _ => {
            let mut rgba = Vec::with_capacity(pixels * 4);
            for &v in &data {
                rgba.extend_from_slice(&[v, v, v, 255]);
            }
            rgba
        }
    };
    Ok(djvu_rs::Pixmap {
        width,
        height,
        data,
    })
}

/// The black-and-white version of `pixmap` for the `lossless` profile.
///
/// An image that is already pure black and white maps pixel for pixel. Any
/// other image goes through the page segmenter, as in the `djvu encode` CLI.
fn bilevel_mask(pixmap: &djvu_rs::Pixmap) -> djvu_rs::Bitmap {
    let is_black = |px: &[u8]| px[0] == 0 && px[1] == 0 && px[2] == 0;
    let is_white = |px: &[u8]| px[0] == 255 && px[1] == 255 && px[2] == 255;
    let pixels = pixmap.data.as_chunks::<4>().0;
    if !pixels.iter().all(|px| is_black(px) || is_white(px)) {
        return segment_page(pixmap, &SegmentOptions::default()).mask;
    }
    let mut mask = djvu_rs::Bitmap::new(pixmap.width, pixmap.height);
    let width = pixmap.width as usize;
    for (i, px) in pixels.iter().enumerate() {
        if is_black(px) {
            mask.set((i % width) as u32, (i / width) as u32, true);
        }
    }
    mask
}

/// Encode one image as a single-page DjVu file.
///
/// `quality` is `'auto'` (pick a profile from the image content),
/// `'lossless'` (black and white), `'quality'`, `'archival'` or `'photo'`.
#[pyfunction]
#[pyo3(signature = (image, *, size=None, dpi=300, quality="auto", text_layer=None, metadata=None))]
fn encode_page<'py>(
    py: Python<'py>,
    image: &Bound<'py, PyAny>,
    size: Option<(u32, u32)>,
    dpi: u16,
    quality: &str,
    text_layer: Option<&Bound<'py, PyAny>>,
    metadata: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyBytes>> {
    let requested = match quality {
        "auto" => None,
        "lossless" => Some(EncodeQuality::Lossless),
        "quality" => Some(EncodeQuality::Quality),
        "archival" => Some(EncodeQuality::Archival),
        "photo" => Some(EncodeQuality::Photo),
        other => {
            return Err(PyValueError::new_err(format!(
                "quality must be 'auto', 'lossless', 'quality', 'archival' or 'photo', not {other:?}"
            )));
        }
    };
    if dpi == 0 {
        return Err(PyValueError::new_err("dpi must be at least 1"));
    }
    let text_layer: Option<TextLayer> = text_layer
        .map(|layer| from_python(layer, "text layer"))
        .transpose()?;
    let metadata: Option<DjVuMetadata> = metadata
        .map(|metadata| from_python(metadata, "metadata"))
        .transpose()?;
    let pixmap = image_to_rgba(image, size)?;

    let encoded = py.detach(|| {
        let finish = |mut encoder: PageEncoder<'_>| {
            encoder = encoder.with_dpi(dpi);
            if let Some(layer) = text_layer.clone() {
                encoder = encoder.with_text_layer(layer);
            }
            if let Some(metadata) = metadata.clone() {
                encoder = encoder.with_metadata(metadata);
            }
            encoder.encode()
        };
        match requested.unwrap_or_else(|| classify_content(&pixmap)) {
            EncodeQuality::Lossless => {
                let mask = bilevel_mask(&pixmap);
                finish(PageEncoder::from_bitmap(&mask))
            }
            profile => finish(PageEncoder::from_pixmap(&pixmap).with_quality(profile)),
        }
        .map_err(|e| EncodeError::new_err(format!("{e}")))
    })?;
    Ok(PyBytes::new(py, &encoded))
}

/// Encode a sequence of images as one multi-page DjVu file.
///
/// The pages are read one at a time by index, so a sequence that loads each
/// image on access keeps the memory use low. Letter shapes that repeat on
/// `shared_dict_pages` or more pages are stored once for the whole file.
#[pyfunction]
#[pyo3(signature = (pages, *, dpi=300, quality="auto", shared_dict_pages=2, thumbnails=false))]
fn encode_document<'py>(
    py: Python<'py>,
    pages: &Bound<'py, PyAny>,
    dpi: u16,
    quality: &str,
    shared_dict_pages: usize,
    thumbnails: bool,
) -> PyResult<Bound<'py, PyBytes>> {
    let requested = match quality {
        "auto" => None,
        "lossless" => Some(EncodeQuality::Lossless),
        "quality" => Some(EncodeQuality::Quality),
        "archival" => Some(EncodeQuality::Archival),
        other => {
            return Err(PyValueError::new_err(format!(
                "quality must be 'auto', 'lossless', 'quality' or 'archival', not {other:?}"
            )));
        }
    };
    if dpi == 0 {
        return Err(PyValueError::new_err("dpi must be at least 1"));
    }
    if shared_dict_pages == 0 {
        return Err(PyValueError::new_err(
            "shared_dict_pages must be at least 1",
        ));
    }
    // A generator has no length and no index: read it into a list first.
    let pages = if pages.hasattr("__len__")? && pages.hasattr("__getitem__")? {
        pages.clone()
    } else {
        pages
            .try_iter()
            .map_err(|_| PyTypeError::new_err("pages must be a sequence or an iterable of images"))?
            .collect::<PyResult<Vec<_>>>()?
            .into_pyobject(py)?
            .into_any()
    };
    let count = pages.len()?;
    if count == 0 {
        return Err(PyValueError::new_err("pages is empty"));
    }
    let pages = pages.unbind();

    let encoded = py.detach(|| -> PyResult<Vec<u8>> {
        let load = |index: usize| -> PyResult<djvu_rs::Pixmap> {
            Python::attach(|py| image_to_rgba(&pages.bind(py).get_item(index)?, None))
        };
        // The file has one profile. `auto` looks at every page first: all
        // black and white gives `lossless`, anything else gives `quality`.
        let profile = match requested {
            Some(profile) => profile,
            None => {
                let mut profile = EncodeQuality::Lossless;
                for index in 0..count {
                    if classify_content(&load(index)?) != EncodeQuality::Lossless {
                        profile = EncodeQuality::Quality;
                        break;
                    }
                }
                profile
            }
        };
        if profile == EncodeQuality::Lossless {
            let masks = (0..count)
                .map(|index| load(index).map(|pixmap| bilevel_mask(&pixmap)))
                .collect::<PyResult<Vec<_>>>()?;
            if masks.iter().any(|m| m.width > 0xFFFF || m.height > 0xFFFF) {
                return Err(EncodeError::new_err(
                    "a page side is larger than 65535 pixels",
                ));
            }
            return Ok(encode_djvm_bundle_jb2_with_thumbnails(
                &masks,
                shared_dict_pages,
                dpi,
                thumbnails,
            ));
        }
        encode_djvm_layered_shared_streaming(
            count,
            load,
            profile,
            dpi,
            None,
            shared_dict_pages,
            thumbnails,
            None,
            None,
        )
        .map_err(|e| match e {
            // A page that failed to load keeps its own Python exception.
            RustEncodeError::PageSource(source) => match source.downcast::<PyErr>() {
                Ok(err) => *err,
                Err(other) => EncodeError::new_err(format!("{other}")),
            },
            other => EncodeError::new_err(format!("{other}")),
        })
    })?;
    Ok(PyBytes::new(py, &encoded))
}

// ---- Export ------------------------------------------------------------------
//
// Every converter below takes the document this handle already parsed and hands
// it to the matching entry point in the Rust crate. Each comes in two forms:
// `to_x()` returns the bytes, `write_x(path)` streams straight to a file and
// holds only one page at a time. Prefer `write_x` for a long book.
//
// All of them release the GIL: an export renders every page, which is the most
// CPU-heavy thing this module does.

/// Open `path` for writing, buffered.
fn create(path: &str) -> PyResult<BufWriter<File>> {
    File::create(path)
        .map(BufWriter::new)
        .map_err(|e| IoError::new_err(format!("cannot write {path}: {e}")))
}

/// Finish a buffered write and report a failed flush, which `Drop` would eat.
fn finish(mut sink: BufWriter<File>, path: &str) -> PyResult<()> {
    sink.flush()
        .map_err(|e| IoError::new_err(format!("cannot write {path}: {e}")))
}

fn pdf_options(
    dpi: u32,
    jpeg_quality: Option<u8>,
    adaptive: bool,
    ccitt_g4: bool,
    mrc: bool,
) -> PdfOptions {
    PdfOptions::default()
        .with_jpeg_quality(jpeg_quality)
        .with_output_dpi(dpi)
        .with_adaptive_raster(adaptive)
        .with_ccitt_g4(ccitt_g4)
        .with_mrc(mrc)
}

#[allow(clippy::too_many_arguments)]
fn epub_options(
    dpi: u32,
    title: &str,
    author: &str,
    language: &str,
    modified: Option<String>,
    reflowable_text: bool,
    jpeg_quality: Option<u8>,
    adaptive: bool,
) -> EpubOptions {
    EpubOptions::default()
        .with_title(title.to_owned())
        .with_author(author.to_owned())
        .with_dpi(dpi)
        .with_language(language.to_owned())
        .with_modified(modified)
        .with_reflowable_text(reflowable_text)
        .with_jpeg_quality(jpeg_quality)
        .with_adaptive(adaptive)
}

/// Map a rotation in degrees onto the render enum. Only quarter turns exist.
fn user_rotation(degrees: i32) -> PyResult<UserRotation> {
    match degrees.rem_euclid(360) {
        0 => Ok(UserRotation::None),
        90 => Ok(UserRotation::Cw90),
        180 => Ok(UserRotation::Rot180),
        270 => Ok(UserRotation::Ccw90),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "rotation must be 0, 90, 180 or 270 degrees, not {other}"
        ))),
    }
}

fn tiff_options(mode: &str, scale: f32, bilevel_compression: &str) -> PyResult<TiffOptions> {
    let mode = match mode {
        "color" => TiffMode::Color,
        "bilevel" => TiffMode::Bilevel,
        other => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "mode must be 'color' or 'bilevel', not {other:?}"
            )));
        }
    };
    let bilevel_compression = match bilevel_compression {
        "deflate" => TiffBilevelCompression::Deflate,
        "g4" => TiffBilevelCompression::G4,
        other => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "bilevel_compression must be 'deflate' or 'g4', not {other:?}"
            )));
        }
    };
    Ok(TiffOptions::default()
        .with_mode(mode)
        .with_scale(scale)
        .with_bilevel_compression(bilevel_compression))
}

/// A DjVu document.
#[pyclass]
struct Document {
    inner: Arc<djvu_rs::Document>,
}

#[pymethods]
impl Document {
    /// Open a DjVu document from a file path.
    ///
    /// Releases the GIL while reading the file and parsing the document, so
    /// other Python threads can run concurrently (e.g. render other pages).
    #[staticmethod]
    fn open(py: Python<'_>, path: &str) -> PyResult<Self> {
        let doc = py.detach(|| {
            let data = std::fs::read(path).map_err(|e| IoError::new_err(format!("{e}")))?;
            djvu_rs::Document::from_bytes(data).map_err(|e| DecodeError::new_err(format!("{e}")))
        })?;
        Ok(Document {
            inner: Arc::new(doc),
        })
    }

    /// Open a DjVu document from bytes.
    ///
    /// Releases the GIL while parsing the document (the input bytes are
    /// copied out of the Python buffer up front, before the GIL is released).
    #[staticmethod]
    fn from_bytes(py: Python<'_>, data: &[u8]) -> PyResult<Self> {
        let data = data.to_vec();
        let doc = py.detach(move || {
            djvu_rs::Document::from_bytes(data).map_err(|e| DecodeError::new_err(format!("{e}")))
        })?;
        Ok(Document {
            inner: Arc::new(doc),
        })
    }

    /// Number of pages in the document.
    fn page_count(&self) -> usize {
        self.inner.page_count()
    }

    /// Get a page by index (0-based).
    fn page(&self, index: usize) -> PyResult<Page> {
        let p = self
            .inner
            .page(index)
            .map_err(|e| PageIndexError::new_err(format!("{e}")))?;
        Ok(Page {
            width: p.width(),
            height: p.height(),
            dpi: p.dpi(),
            doc: Arc::clone(&self.inner),
            index,
        })
    }

    /// Document-level metadata as a dict, or None when the document has none.
    ///
    /// Keys: `title`, `author`, `subject`, `publisher`, `year`, `keywords`
    /// (each a string or None), and `extra`, a list of other `(key, value)`
    /// pairs in document order.
    fn metadata<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        document_metadata(py, self.inner.inner())
    }

    /// The table of contents as a list of dicts with `title`, `url` and
    /// `children` (the same shape, nested). Empty when there is none.
    fn bookmarks<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        to_python(py, self.inner.inner().bookmarks())
    }

    /// Convert the document to PDF and return the bytes.
    ///
    /// Args:
    ///     dpi: Output resolution. 0 means the page's own DPI (largest,
    ///         slowest). 150 is screen quality, 300 is print quality.
    ///     jpeg_quality: 1-100 for JPEG page images, or None for lossless
    ///         Deflate (larger files).
    ///     adaptive: Encode each page both ways and keep the smaller one.
    ///         Only meaningful with a jpeg_quality.
    ///     ccitt_g4: Also try CCITT Group 4 for bilevel masks and keep
    ///         whichever is smaller.
    ///     mrc: Embed the background layer alone and repaint the text from
    ///         the mask, instead of embedding the composited page.
    ///
    /// The PDF carries an invisible text layer, the bookmarks and the links.
    /// Use `write_pdf` for a long book: it holds one page at a time.
    #[pyo3(signature = (dpi=150, jpeg_quality=80, adaptive=false, ccitt_g4=false, mrc=false))]
    fn to_pdf<'py>(
        &self,
        py: Python<'py>,
        dpi: u32,
        jpeg_quality: Option<u8>,
        adaptive: bool,
        ccitt_g4: bool,
        mrc: bool,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let opts = pdf_options(dpi, jpeg_quality, adaptive, ccitt_g4, mrc);
        let buf = py.detach(|| {
            djvu_rs::pdf::djvu_to_pdf_with_options(self.inner.inner(), &opts)
                .map_err(|e| ExportError::new_err(format!("PDF export failed: {e}")))
        })?;
        Ok(PyBytes::new(py, &buf))
    }

    /// Convert the document to PDF, straight into the file at `path`.
    ///
    /// Takes the same arguments as `to_pdf`. Holds one page at a time, so the
    /// memory it needs does not grow with the page count.
    #[pyo3(signature = (path, dpi=150, jpeg_quality=80, adaptive=false, ccitt_g4=false, mrc=false))]
    #[allow(clippy::too_many_arguments)]
    fn write_pdf(
        &self,
        py: Python<'_>,
        path: &str,
        dpi: u32,
        jpeg_quality: Option<u8>,
        adaptive: bool,
        ccitt_g4: bool,
        mrc: bool,
    ) -> PyResult<()> {
        let opts = pdf_options(dpi, jpeg_quality, adaptive, ccitt_g4, mrc);
        let sink = create(path)?;
        py.detach(|| {
            let mut sink = sink;
            djvu_rs::pdf::djvu_to_pdf_to_writer(self.inner.inner(), &opts, &mut sink)
                .map_err(|e| ExportError::new_err(format!("PDF export failed: {e}")))?;
            finish(sink, path)
        })
    }

    /// Convert the document to EPUB 3 and return the bytes.
    ///
    /// Args:
    ///     dpi: Resolution of the page images.
    ///     title, author, language: OPF metadata. `language` is a BCP-47 tag.
    ///     modified: ISO 8601 timestamp for `dcterms:modified`. None uses the
    ///         current UTC time.
    ///     reflowable_text: Append the extracted paragraphs after each page
    ///         image, for readers that prefer flowing text.
    ///     jpeg_quality: 1-100 for JPEG page images, or None for PNG.
    ///     adaptive: Encode each page both ways and keep the smaller one.
    #[pyo3(signature = (dpi=150, title="DjVu Document", author="",
                        language="en", modified=None, reflowable_text=false,
                        jpeg_quality=None, adaptive=false))]
    #[allow(clippy::too_many_arguments)]
    fn to_epub<'py>(
        &self,
        py: Python<'py>,
        dpi: u32,
        title: &str,
        author: &str,
        language: &str,
        modified: Option<String>,
        reflowable_text: bool,
        jpeg_quality: Option<u8>,
        adaptive: bool,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let opts = epub_options(
            dpi,
            title,
            author,
            language,
            modified,
            reflowable_text,
            jpeg_quality,
            adaptive,
        );
        let buf = py.detach(|| {
            djvu_rs::epub::djvu_to_epub(self.inner.inner(), &opts)
                .map_err(|e| ExportError::new_err(format!("EPUB export failed: {e}")))
        })?;
        Ok(PyBytes::new(py, &buf))
    }

    /// Convert the document to EPUB 3, straight into the file at `path`.
    ///
    /// Takes the same arguments as `to_epub`.
    #[pyo3(signature = (path, dpi=150, title="DjVu Document", author="",
                        language="en", modified=None, reflowable_text=false,
                        jpeg_quality=None, adaptive=false))]
    #[allow(clippy::too_many_arguments)]
    fn write_epub(
        &self,
        py: Python<'_>,
        path: &str,
        dpi: u32,
        title: &str,
        author: &str,
        language: &str,
        modified: Option<String>,
        reflowable_text: bool,
        jpeg_quality: Option<u8>,
        adaptive: bool,
    ) -> PyResult<()> {
        let opts = epub_options(
            dpi,
            title,
            author,
            language,
            modified,
            reflowable_text,
            jpeg_quality,
            adaptive,
        );
        let sink = create(path)?;
        py.detach(|| {
            let mut sink = sink;
            djvu_rs::epub::djvu_to_epub_writer(self.inner.inner(), &opts, &mut sink)
                .map_err(|e| ExportError::new_err(format!("EPUB export failed: {e}")))?;
            finish(sink, path)
        })
    }

    /// Convert the document to a CBZ archive and return the bytes.
    ///
    /// Each page becomes one PNG entry, named `page_0001.png` and upward.
    ///
    /// Args:
    ///     dpi: Resolution of the page images.
    ///     rotation: Extra rotation in degrees: 0, 90, 180 or 270. It applies
    ///         on top of the rotation the page itself declares.
    ///     pages: 0-based page numbers to export, in the order given. None
    ///         exports every page.
    #[pyo3(signature = (dpi=150, rotation=0, pages=None))]
    fn to_cbz<'py>(
        &self,
        py: Python<'py>,
        dpi: u32,
        rotation: i32,
        pages: Option<Vec<usize>>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let opts = CbzOptions::default()
            .with_dpi(dpi)
            .with_rotation(user_rotation(rotation)?)
            .with_pages(pages);
        let buf = py.detach(|| {
            djvu_rs::cbz::djvu_to_cbz(self.inner.inner(), &opts)
                .map_err(|e| ExportError::new_err(format!("CBZ export failed: {e}")))
        })?;
        Ok(PyBytes::new(py, &buf))
    }

    /// Convert the document to a CBZ archive, straight into the file at `path`.
    ///
    /// Takes the same arguments as `to_cbz`.
    #[pyo3(signature = (path, dpi=150, rotation=0, pages=None))]
    fn write_cbz(
        &self,
        py: Python<'_>,
        path: &str,
        dpi: u32,
        rotation: i32,
        pages: Option<Vec<usize>>,
    ) -> PyResult<()> {
        let opts = CbzOptions::default()
            .with_dpi(dpi)
            .with_rotation(user_rotation(rotation)?)
            .with_pages(pages);
        let sink = create(path)?;
        py.detach(|| {
            let mut sink = sink;
            djvu_rs::cbz::djvu_to_cbz_writer(self.inner.inner(), &opts, &mut sink)
                .map_err(|e| ExportError::new_err(format!("CBZ export failed: {e}")))?;
            finish(sink, path)
        })
    }

    /// Convert the document to a multi-page TIFF and return the bytes.
    ///
    /// Args:
    ///     mode: "color" for full-colour pages, "bilevel" for the black and
    ///         white mask alone.
    ///     scale: Size factor against the page's own resolution.
    ///     bilevel_compression: "deflate" or "g4". "g4" is the archival
    ///         choice for scans and is usually much smaller. It applies only
    ///         in "bilevel" mode.
    #[pyo3(signature = (mode="color", scale=1.0, bilevel_compression="deflate"))]
    fn to_tiff<'py>(
        &self,
        py: Python<'py>,
        mode: &str,
        scale: f32,
        bilevel_compression: &str,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let opts = tiff_options(mode, scale, bilevel_compression)?;
        let buf = py.detach(|| {
            djvu_rs::tiff_export::djvu_to_tiff(self.inner.inner(), &opts)
                .map_err(|e| ExportError::new_err(format!("TIFF export failed: {e}")))
        })?;
        Ok(PyBytes::new(py, &buf))
    }

    /// Convert the document to a multi-page TIFF, straight into the file at
    /// `path`.
    ///
    /// Takes the same arguments as `to_tiff`.
    #[pyo3(signature = (path, mode="color", scale=1.0, bilevel_compression="deflate"))]
    fn write_tiff(
        &self,
        py: Python<'_>,
        path: &str,
        mode: &str,
        scale: f32,
        bilevel_compression: &str,
    ) -> PyResult<()> {
        let opts = tiff_options(mode, scale, bilevel_compression)?;
        let sink = create(path)?;
        py.detach(|| {
            let mut sink = sink;
            djvu_rs::tiff_export::djvu_to_tiff_writer(self.inner.inner(), &opts, &mut sink)
                .map_err(|e| ExportError::new_err(format!("TIFF export failed: {e}")))?;
            finish(sink, path)
        })
    }
}

/// A page within a DjVu document.
#[pyclass]
struct Page {
    width: u32,
    height: u32,
    dpi: u16,
    doc: Arc<djvu_rs::Document>,
    index: usize,
}

/// Output dims for an optional target DPI (native DPI when `None`). The size
/// is after the page's INFO rotation, the size the render comes out at.
fn dims_at(page: &djvu_rs::Page<'_>, dpi: Option<f32>) -> (u32, u32) {
    match dpi {
        Some(target) => page.size_at_dpi(target),
        None => (page.display_width(), page.display_height()),
    }
}

/// Parse the ``quality`` argument of ``Page.render``.
fn parse_quality(quality: Option<&Bound<'_, PyAny>>) -> PyResult<Quality> {
    let Some(q) = quality else {
        return Ok(Quality::Full);
    };
    if let Ok(name) = q.extract::<String>() {
        return match name.as_str() {
            "full" => Ok(Quality::Full),
            "coarse" => Ok(Quality::Coarse),
            other => Err(PyValueError::new_err(format!(
                "unknown quality {other:?}: expected \"full\", \"coarse\", or a chunk index"
            ))),
        };
    }
    q.extract::<usize>().map(Quality::Step).map_err(|_| {
        PyValueError::new_err("quality must be None, \"full\", \"coarse\", or a chunk index")
    })
}

impl Page {
    /// The render path behind ``render``.
    ///
    /// `size` sides left `None` default to the render size at `dpi` (native
    /// DPI when `None`).
    fn render_request(
        &self,
        py: Python<'_>,
        size: Option<(Option<u32>, Option<u32>)>,
        dpi: Option<f32>,
        region: Option<(u32, u32, u32, u32)>,
        quality: Quality,
    ) -> PyResult<Pixmap> {
        let pixmap = py.detach(|| {
            let page = self
                .doc
                .page(self.index)
                .map_err(|e| PageIndexError::new_err(format!("{e}")))?;
            let (dw, dh) = dims_at(&page, dpi);
            let (w, h) = match size {
                Some((w, h)) => (w.unwrap_or(dw), h.unwrap_or(dh)),
                None => (dw, dh),
            };
            let mut request = RenderRequest::new(page.options_for_size(w, h)).quality(quality);
            if let Some((x, y, width, height)) = region {
                request = request
                    .region(RenderRect {
                        x,
                        y,
                        width,
                        height,
                    })
                    .cached(true);
            }
            page.render_request(&request)
                .map_err(|e| DecodeError::new_err(format!("render failed: {e}")))
        })?;
        Ok(Pixmap {
            width: pixmap.width,
            height: pixmap.height,
            data: pixmap.data,
        })
    }
}

#[pymethods]
impl Page {
    /// Page width in pixels.
    #[getter]
    fn width(&self) -> u32 {
        self.width
    }

    /// Page height in pixels.
    #[getter]
    fn height(&self) -> u32 {
        self.height
    }

    /// Page DPI.
    #[getter]
    fn dpi(&self) -> u16 {
        self.dpi
    }

    /// Render the page as RGBA bytes.
    ///
    /// Args:
    ///     dpi: Target DPI. If not specified, renders at native DPI.
    ///     size: ``(width, height)`` of the full render, in place of ``dpi``
    ///         (the zoom level as a pixel size).
    ///     region: ``(x, y, w, h)`` — render only this rectangle of the full
    ///         render, in the page's displayed orientation (after its INFO
    ///         rotation). Pixels outside the page are white. Served from the
    ///         composited-tile cache, so viewer-style pans and revisits cost
    ///         only their new tiles.
    ///     quality: ``None`` or ``"full"`` for the full render; ``"coarse"``
    ///         for a fast, blurry preview from the first background chunk;
    ///         an int ``n`` for background chunks ``0..=n`` (a progressive
    ///         step; ``bg44_chunk_count - 1`` is the full render).
    ///
    /// Returns:
    ///     Pixmap with width, height, and RGBA data.
    ///
    /// Raises:
    ///     ValueError: both ``dpi`` and ``size``, or an unknown ``quality``.
    ///     DecodeError: a decode error, a step past the last chunk, or a
    ///         coarse render of a page without a background.
    ///
    /// Releases the GIL for the (CPU-heavy) decode + compositing + resampling
    /// work, so other Python threads can render other pages concurrently.
    #[pyo3(signature = (dpi=None, *, size=None, region=None, quality=None))]
    fn render(
        &self,
        py: Python<'_>,
        dpi: Option<f32>,
        size: Option<(u32, u32)>,
        region: Option<(u32, u32, u32, u32)>,
        quality: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Pixmap> {
        if dpi.is_some() && size.is_some() {
            return Err(PyValueError::new_err("pass dpi or size, not both"));
        }
        let quality = parse_quality(quality)?;
        let size = size.map(|(w, h)| (Some(w), Some(h)));
        self.render_request(py, size, dpi, region, quality)
    }

    /// Number of BG44 refinement chunks (0 for bilevel pages).
    #[getter]
    fn bg44_chunk_count(&self) -> PyResult<usize> {
        let page = self
            .doc
            .page(self.index)
            .map_err(|e| PageIndexError::new_err(format!("{e}")))?;
        Ok(page.bg44_chunk_count())
    }

    /// Extract the text layer from this page.
    ///
    /// Returns None if no text layer is present. Releases the GIL for the
    /// decompression + zone-tree parse.
    fn text(&self, py: Python<'_>) -> PyResult<Option<String>> {
        py.detach(|| {
            let page = self
                .doc
                .page(self.index)
                .map_err(|e| PageIndexError::new_err(format!("{e}")))?;
            page.text()
                .map_err(|e| DecodeError::new_err(format!("{e}")))
        })
    }

    /// The page's annotations as `(annotation, areas)`, or None when the page
    /// has none.
    ///
    /// `annotation` is a dict with `background`, `zoom`, `mode` and `extra`
    /// (every other top-level form as S-expression text). `areas` is a list
    /// of map-area dicts with `url`, `target`, `description`, `shape`,
    /// `border`, `highlight` and `extra`. A shape is a one-key dict such as
    /// `{"Rect": {"x": 10, "y": 10, "width": 100, "height": 20}}`, in DjVu
    /// coordinates (origin at the bottom-left corner).
    fn annotations<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Option<(Bound<'py, PyAny>, Bound<'py, PyAny>)>> {
        page_annotations(py, self.doc.inner(), self.index)
    }

    /// The page's text layer as a dict, or None when the page has none.
    ///
    /// The dict has `text`, the full page text, and `zones`, a tree of zone
    /// dicts with `kind` (`"Page"`, `"Column"`, `"Region"`, `"Para"`,
    /// `"Line"`, `"Word"` or `"Character"`), `rect` (`x`, `y`, `width`,
    /// `height` in pixels, origin at the top-left corner), `text` and
    /// `children`.
    fn text_layer<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        page_text_layer(py, self.doc.inner(), self.index)
    }
}

/// A DjVu document opened for editing.
///
/// Changes stay in memory until `save` or `to_bytes`. The readers
/// (`metadata`, `bookmarks`, `page_annotations`, `page_text_layer`,
/// `document`) show the document with the changes made so far, so a value
/// can be read, changed and written back. Single-page and bundled documents
/// are supported; indirect documents and legacy BM44/PM44 pages raise
/// `EditError`.
#[pyclass]
struct Editor {
    doc: DjVuDocumentMut,
    /// The edited state, parsed for reading; dropped by every change.
    view: Option<Arc<djvu_rs::Document>>,
}

impl Editor {
    fn new(doc: DjVuDocumentMut) -> Self {
        Editor { doc, view: None }
    }

    /// The current state, parsed once per change.
    fn view(&mut self, py: Python<'_>) -> PyResult<Arc<djvu_rs::Document>> {
        if let Some(view) = &self.view {
            return Ok(Arc::clone(view));
        }
        let doc = self.doc.clone();
        let view = py.detach(move || {
            let bytes = doc.try_into_bytes().map_err(edit_error)?;
            djvu_rs::Document::from_bytes(bytes).map_err(|e| DecodeError::new_err(format!("{e}")))
        })?;
        let view = Arc::new(view);
        self.view = Some(Arc::clone(&view));
        Ok(view)
    }

    fn changed(&mut self) {
        self.view = None;
    }

    fn serialize(&self, py: Python<'_>) -> PyResult<Vec<u8>> {
        let doc = self.doc.clone();
        py.detach(move || doc.try_into_bytes().map_err(edit_error))
    }
}

#[pymethods]
impl Editor {
    /// Open a DjVu file for editing.
    #[staticmethod]
    fn open(py: Python<'_>, path: &str) -> PyResult<Self> {
        let doc = py.detach(|| {
            let data = std::fs::read(path).map_err(|e| IoError::new_err(format!("{e}")))?;
            DjVuDocumentMut::from_bytes(&data).map_err(|e| DecodeError::new_err(format!("{e}")))
        })?;
        Ok(Editor::new(doc))
    }

    /// Open DjVu bytes for editing.
    #[staticmethod]
    fn from_bytes(py: Python<'_>, data: &[u8]) -> PyResult<Self> {
        let data = data.to_vec();
        let doc = py.detach(move || {
            DjVuDocumentMut::from_bytes(&data).map_err(|e| DecodeError::new_err(format!("{e}")))
        })?;
        Ok(Editor::new(doc))
    }

    /// Number of pages in the document.
    fn page_count(&self) -> usize {
        self.doc.page_count()
    }

    /// True once any change has been made.
    #[getter]
    fn modified(&self) -> bool {
        self.doc.is_dirty()
    }

    /// The document with the changes made so far, as a read-only `Document`
    /// (for rendering, text or export).
    fn document(&mut self, py: Python<'_>) -> PyResult<Document> {
        Ok(Document {
            inner: self.view(py)?,
        })
    }

    /// Document-level metadata as a dict, or None. See `Document.metadata`.
    fn metadata<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        let view = self.view(py)?;
        document_metadata(py, view.inner())
    }

    /// Replace the document-level metadata with the dict `metadata`.
    ///
    /// Keys left out are empty: `{"title": "Atlas"}` keeps no author.
    fn set_metadata(&mut self, metadata: &Bound<'_, PyAny>) -> PyResult<()> {
        let metadata: DjVuMetadata = from_python(metadata, "metadata")?;
        self.doc.set_metadata(&metadata).map_err(edit_error)?;
        self.changed();
        Ok(())
    }

    /// Remove the document-level metadata.
    fn remove_metadata(&mut self) -> PyResult<()> {
        self.doc.remove_metadata().map_err(edit_error)?;
        self.changed();
        Ok(())
    }

    /// The table of contents. See `Document.bookmarks`.
    fn bookmarks<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let view = self.view(py)?;
        to_python(py, view.inner().bookmarks())
    }

    /// Replace the table of contents with `bookmarks`, a list of dicts with
    /// `title`, `url` and optional `children`. An empty list removes it.
    ///
    /// Raises `EditError` for a single-page document: DjVu keeps bookmarks
    /// only in a bundled multi-page file.
    fn set_bookmarks(&mut self, bookmarks: &Bound<'_, PyAny>) -> PyResult<()> {
        let bookmarks: Vec<DjVuBookmark> = from_python(bookmarks, "bookmarks")?;
        self.doc.set_bookmarks(&bookmarks).map_err(edit_error)?;
        self.changed();
        Ok(())
    }

    /// The annotations of page `index`. See `Page.annotations`.
    fn page_annotations<'py>(
        &mut self,
        py: Python<'py>,
        index: usize,
    ) -> PyResult<Option<(Bound<'py, PyAny>, Bound<'py, PyAny>)>> {
        let view = self.view(py)?;
        page_annotations(py, view.inner(), index)
    }

    /// Replace the annotations of page `index`.
    ///
    /// `annotation` is a dict in the shape `Page.annotations` returns; keys
    /// left out take their defaults, so `{}` is valid. `areas` is a list of
    /// map-area dicts; each needs a `shape`. The page's `(metadata ...)`
    /// block is kept unless `annotation["extra"]` holds one.
    #[pyo3(signature = (index, annotation, areas=None))]
    fn set_page_annotations(
        &mut self,
        index: usize,
        annotation: &Bound<'_, PyAny>,
        areas: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let annotation: Annotation = from_python(annotation, "annotation")?;
        let areas: Vec<MapArea> = match areas {
            Some(areas) => from_python(areas, "map areas")?,
            None => Vec::new(),
        };
        self.doc
            .page_mut(index)
            .map_err(edit_error)?
            .set_annotations(&annotation, &areas);
        self.changed();
        Ok(())
    }

    /// Remove the annotations of page `index`.
    fn remove_page_annotations(&mut self, index: usize) -> PyResult<()> {
        self.doc
            .page_mut(index)
            .map_err(edit_error)?
            .remove_annotations();
        self.changed();
        Ok(())
    }

    /// The text layer of page `index`. See `Page.text_layer`.
    fn page_text_layer<'py>(
        &mut self,
        py: Python<'py>,
        index: usize,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        let view = self.view(py)?;
        page_text_layer(py, view.inner(), index)
    }

    /// Replace the text layer of page `index` with the dict `layer`, in the
    /// shape `Page.text_layer` returns.
    ///
    /// Each zone's `text` must occur in the layer's `text`: the file stores
    /// zones as spans of the page text.
    fn set_page_text_layer(
        &mut self,
        py: Python<'_>,
        index: usize,
        layer: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let layer: TextLayer = from_python(layer, "text layer")?;
        let doc = &mut self.doc;
        py.detach(|| {
            doc.page_mut(index)
                .map_err(edit_error)?
                .set_text_layer(&layer)
                .map_err(edit_error)
        })?;
        self.changed();
        Ok(())
    }

    /// Remove the text layer of page `index`.
    fn remove_page_text_layer(&mut self, index: usize) -> PyResult<()> {
        self.doc
            .page_mut(index)
            .map_err(edit_error)?
            .remove_text_layer();
        self.changed();
        Ok(())
    }

    /// The edited document as bytes.
    fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = self.serialize(py)?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// Write the edited document to `path`.
    ///
    /// The output is validated first and then replaces `path` in one step,
    /// through a temporary file beside it, so a failure leaves an existing
    /// file untouched. `path` may be the file this editor was opened from.
    fn save(&self, py: Python<'_>, path: &str) -> PyResult<()> {
        let bytes = self.serialize(py)?;
        py.detach(|| {
            DocumentEditor::commit_bytes(Path::new(path), &bytes)
                .map_err(|e| EditError::new_err(format!("cannot save {path}: {e}")))
        })
    }
}

/// An RGBA pixel buffer.
#[pyclass]
struct Pixmap {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

#[pymethods]
impl Pixmap {
    /// Image width in pixels.
    #[getter]
    fn width(&self) -> u32 {
        self.width
    }

    /// Image height in pixels.
    #[getter]
    fn height(&self) -> u32 {
        self.height
    }

    /// RGBA pixel data as bytes (length = width * height * 4).
    fn data<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.data)
    }

    /// Convert to a numpy array (requires numpy).
    ///
    /// Returns a numpy.ndarray with shape (height, width, 4) and dtype uint8.
    fn to_numpy<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let numpy = py.import("numpy")?;
        let frombuffer = numpy.getattr("frombuffer")?;
        let bytes = PyBytes::new(py, &self.data);
        let arr = frombuffer.call1((bytes, numpy.getattr("uint8")?))?;
        arr.call_method1("reshape", ((self.height, self.width, 4u32),))
    }

    /// Convert to a PIL Image (requires Pillow).
    ///
    /// Returns a PIL.Image.Image in RGBA mode.
    fn to_pil<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let pil = py.import("PIL.Image")?;
        let frombytes = pil.getattr("frombytes")?;
        let size = (self.width, self.height);
        let bytes = PyBytes::new(py, &self.data);
        frombytes.call1(("RGBA", size, bytes))
    }

    /// Zero-copy numpy view (requires numpy).
    ///
    /// Like [`to_numpy`](Self::to_numpy), but the array is backed directly by
    /// this `Pixmap`'s RGBA buffer via the buffer protocol — no `bytes` copy
    /// is made. The returned array keeps this `Pixmap` alive and should be
    /// treated as read-only (the underlying data is not writable).
    fn to_numpy_zerocopy<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyAny>> {
        let py = slf.py();
        let (height, width) = {
            let this = slf.borrow();
            (this.height, this.width)
        };
        let numpy = py.import("numpy")?;
        let arr = numpy
            .getattr("frombuffer")?
            .call1((slf, numpy.getattr("uint8")?))?;
        arr.call_method1("reshape", ((height, width, 4u32),))
    }

    /// Zero-copy PIL Image view (requires Pillow).
    ///
    /// Like [`to_pil`](Self::to_pil), but Pillow reads the pixel data
    /// directly out of this `Pixmap`'s buffer via the buffer protocol —
    /// no `bytes` copy is made. The returned image keeps this `Pixmap`
    /// alive and should be treated as read-only.
    fn to_pil_zerocopy<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyAny>> {
        let py = slf.py();
        let (height, width) = {
            let this = slf.borrow();
            (this.height, this.width)
        };
        let pil = py.import("PIL.Image")?;
        pil.getattr("frombuffer")?
            .call1(("RGBA", (width, height), slf, "raw", "RGBA", 0, 1))
    }

    /// Expose the RGBA data via the Python buffer protocol.
    ///
    /// This is what powers `memoryview(pixmap)`, `bytes(pixmap)`,
    /// `numpy.frombuffer(pixmap, ...)`, and the `*_zerocopy` helpers above —
    /// no data is copied out of this `Pixmap`; the buffer keeps it alive
    /// for as long as any view into it is held. The buffer is a flat,
    /// read-only `uint8` array of length `width * height * 4`.
    unsafe fn __getbuffer__(
        slf: Bound<'_, Self>,
        view: *mut pyo3::ffi::Py_buffer,
        flags: c_int,
    ) -> PyResult<()> {
        if view.is_null() {
            return Err(PyBufferError::new_err("View is null"));
        }
        if (flags & pyo3::ffi::PyBUF_WRITABLE) == pyo3::ffi::PyBUF_WRITABLE {
            return Err(PyBufferError::new_err("Pixmap buffer is read-only"));
        }

        let (data_ptr, len) = {
            let this = slf.borrow();
            (this.data.as_ptr() as *mut c_void, this.data.len() as isize)
        };

        // SAFETY: `view` is non-null (checked above) and comes from the
        // CPython buffer-protocol call, so it points to a valid, writable
        // `Py_buffer`. `slf.into_ptr()` hands over one owned reference,
        // which CPython will release via `Py_DECREF` when the consumer
        // calls `PyBuffer_Release` — that keeps this `Pixmap` (and its
        // `data` allocation) alive for the buffer's lifetime.
        unsafe {
            (*view).obj = slf.into_ptr();
            (*view).buf = data_ptr;
            (*view).len = len;
            (*view).readonly = 1;
            (*view).itemsize = 1;
            (*view).format = if (flags & pyo3::ffi::PyBUF_FORMAT) == pyo3::ffi::PyBUF_FORMAT {
                CString::new("B").unwrap().into_raw()
            } else {
                std::ptr::null_mut()
            };
            (*view).ndim = 1;
            (*view).shape = if (flags & pyo3::ffi::PyBUF_ND) == pyo3::ffi::PyBUF_ND {
                &mut (*view).len
            } else {
                std::ptr::null_mut()
            };
            (*view).strides = if (flags & pyo3::ffi::PyBUF_STRIDES) == pyo3::ffi::PyBUF_STRIDES {
                &mut (*view).itemsize
            } else {
                std::ptr::null_mut()
            };
            (*view).suboffsets = std::ptr::null_mut();
            (*view).internal = std::ptr::null_mut();
        }
        Ok(())
    }

    /// Release a buffer previously filled by [`__getbuffer__`](Self::__getbuffer__).
    unsafe fn __releasebuffer__(&self, view: *mut pyo3::ffi::Py_buffer) {
        // SAFETY: `view` was filled in by `__getbuffer__` above; `format` is
        // either null or an owned `CString` we allocated with `into_raw`.
        unsafe {
            if !(*view).format.is_null() {
                drop(CString::from_raw((*view).format));
            }
        }
    }
}

/// Python module definition.
#[pymodule(name = "djvu_rs")]
fn djvu_rs_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("Error", m.py().get_type::<Error>())?;
    m.add("DecodeError", m.py().get_type::<DecodeError>())?;
    m.add("IoError", m.py().get_type::<IoError>())?;
    m.add("ExportError", m.py().get_type::<ExportError>())?;
    m.add("PageIndexError", PageIndexError::type_object(m.py())?)?;
    m.add("EditError", m.py().get_type::<EditError>())?;
    m.add("EncodeError", m.py().get_type::<EncodeError>())?;
    m.add_class::<Document>()?;
    m.add_class::<Page>()?;
    m.add_class::<Editor>()?;
    m.add_class::<Pixmap>()?;
    m.add_function(wrap_pyfunction!(merge, m)?)?;
    m.add_function(wrap_pyfunction!(split, m)?)?;
    m.add_function(wrap_pyfunction!(remove_pages, m)?)?;
    m.add_function(wrap_pyfunction!(encode_page, m)?)?;
    m.add_function(wrap_pyfunction!(encode_document, m)?)?;
    Ok(())
}
