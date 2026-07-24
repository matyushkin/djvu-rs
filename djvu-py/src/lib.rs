use std::ffi::{CString, c_int, c_void};
use std::sync::Arc;

use pyo3::create_exception;
use pyo3::exceptions::PyBufferError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

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
    PageIndexError,
    pyo3::exceptions::PyIndexError,
    "Page index is out of range for this document."
);

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

impl Page {
    /// Output dims for an optional target DPI (native when `None`).
    fn dims_at(&self, dpi: Option<f32>) -> (u32, u32) {
        match dpi {
            Some(target) => {
                let scale = target / self.dpi as f32;
                (
                    ((self.width as f32 * scale).round() as u32).max(1),
                    ((self.height as f32 * scale).round() as u32).max(1),
                )
            }
            None => (self.width, self.height),
        }
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
    ///
    /// Returns:
    ///     Pixmap with width, height, and RGBA data.
    ///
    /// Releases the GIL for the (CPU-heavy) decode + compositing + resampling
    /// work, so other Python threads can render other pages concurrently.
    #[pyo3(signature = (dpi=None))]
    fn render(&self, py: Python<'_>, dpi: Option<f32>) -> PyResult<Pixmap> {
        let pixmap = py.detach(|| {
            let page = self
                .doc
                .page(self.index)
                .map_err(|e| PageIndexError::new_err(format!("{e}")))?;

            if let Some(target_dpi) = dpi {
                let scale = target_dpi / self.dpi as f32;
                let w = ((self.width as f32 * scale).round() as u32).max(1);
                let h = ((self.height as f32 * scale).round() as u32).max(1);
                page.render_to_size(w, h)
            } else {
                page.render()
            }
            .map_err(|e| DecodeError::new_err(format!("render failed: {e}")))
        })?;

        Ok(Pixmap {
            width: pixmap.width,
            height: pixmap.height,
            data: pixmap.data,
        })
    }

    /// Render a rectangular region of the page (#583).
    ///
    /// Args:
    ///     x, y, w, h: viewport rectangle in output pixels.
    ///     full_width, full_height: the full-render size the region is cut
    ///         from (the zoom level). Defaults to the native page size.
    ///
    /// Routed through the composited-tile cache, so viewer-style pans and
    /// revisits reuse tiles — O(viewport) work instead of O(page). Releases
    /// the GIL like `render`.
    #[pyo3(signature = (x, y, w, h, full_width=None, full_height=None))]
    #[allow(clippy::too_many_arguments)]
    fn render_region(
        &self,
        py: Python<'_>,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        full_width: Option<u32>,
        full_height: Option<u32>,
    ) -> PyResult<Pixmap> {
        let fw = full_width.unwrap_or(self.width).max(1);
        let fh = full_height.unwrap_or(self.height).max(1);
        let pixmap = py.detach(|| {
            let page = self
                .doc
                .page(self.index)
                .map_err(|e| PageIndexError::new_err(format!("{e}")))?;
            page.render_region(fw, fh, x, y, w, h)
                .map_err(|e| DecodeError::new_err(format!("render_region failed: {e}")))
        })?;
        Ok(Pixmap {
            width: pixmap.width,
            height: pixmap.height,
            data: pixmap.data,
        })
    }

    /// Fast coarse render — first BG44 chunk only (#583). A blurry but
    /// near-instant preview; returns None for bilevel-only pages.
    #[pyo3(signature = (dpi=None))]
    fn render_coarse(&self, py: Python<'_>, dpi: Option<f32>) -> PyResult<Option<Pixmap>> {
        let (w, h) = self.dims_at(dpi);
        let pm = py.detach(|| {
            let page = self
                .doc
                .page(self.index)
                .map_err(|e| PageIndexError::new_err(format!("{e}")))?;
            page.render_coarse(w, h)
                .map_err(|e| DecodeError::new_err(format!("render_coarse failed: {e}")))
        })?;
        Ok(pm.map(|p| Pixmap {
            width: p.width,
            height: p.height,
            data: p.data,
        }))
    }

    /// Progressive render (#583): decode BG44 chunks 0..=chunk_n plus all
    /// foreground layers. `chunk_n = bg44_chunk_count - 1` equals the full
    /// render.
    #[pyo3(signature = (chunk_n, dpi=None))]
    fn render_progressive(
        &self,
        py: Python<'_>,
        chunk_n: usize,
        dpi: Option<f32>,
    ) -> PyResult<Pixmap> {
        let (w, h) = self.dims_at(dpi);
        let pm = py.detach(|| {
            let page = self
                .doc
                .page(self.index)
                .map_err(|e| PageIndexError::new_err(format!("{e}")))?;
            page.render_progressive(w, h, chunk_n)
                .map_err(|e| DecodeError::new_err(format!("render_progressive failed: {e}")))
        })?;
        Ok(Pixmap {
            width: pm.width,
            height: pm.height,
            data: pm.data,
        })
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
    m.add("PageIndexError", m.py().get_type::<PageIndexError>())?;
    m.add_class::<Document>()?;
    m.add_class::<Page>()?;
    m.add_class::<Pixmap>()?;
    Ok(())
}
