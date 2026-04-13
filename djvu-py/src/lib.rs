use std::sync::Arc;

use pyo3::exceptions::{PyIOError, PyIndexError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// A DjVu document.
#[pyclass]
struct Document {
    inner: Arc<djvu_rs::Document>,
}

#[pymethods]
impl Document {
    /// Open a DjVu document from a file path.
    #[staticmethod]
    fn open(path: &str) -> PyResult<Self> {
        let data = std::fs::read(path).map_err(|e| PyIOError::new_err(format!("{e}")))?;
        let doc = djvu_rs::Document::from_bytes(data)
            .map_err(|e| PyValueError::new_err(format!("{e}")))?;
        Ok(Document {
            inner: Arc::new(doc),
        })
    }

    /// Open a DjVu document from bytes.
    #[staticmethod]
    fn from_bytes(data: &[u8]) -> PyResult<Self> {
        let doc = djvu_rs::Document::from_bytes(data.to_vec())
            .map_err(|e| PyValueError::new_err(format!("{e}")))?;
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
            .map_err(|e| PyIndexError::new_err(format!("{e}")))?;
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
    #[pyo3(signature = (dpi=None))]
    fn render(&self, dpi: Option<f32>) -> PyResult<Pixmap> {
        let page = self
            .doc
            .page(self.index)
            .map_err(|e| PyIndexError::new_err(format!("{e}")))?;

        let pixmap = if let Some(target_dpi) = dpi {
            let scale = target_dpi / self.dpi as f32;
            let w = ((self.width as f32 * scale).round() as u32).max(1);
            let h = ((self.height as f32 * scale).round() as u32).max(1);
            page.render_to_size(w, h)
        } else {
            page.render()
        }
        .map_err(|e| PyValueError::new_err(format!("render failed: {e}")))?;

        Ok(Pixmap {
            width: pixmap.width,
            height: pixmap.height,
            data: pixmap.data,
        })
    }

    /// Extract the text layer from this page.
    ///
    /// Returns None if no text layer is present.
    fn text(&self) -> PyResult<Option<String>> {
        let page = self
            .doc
            .page(self.index)
            .map_err(|e| PyIndexError::new_err(format!("{e}")))?;
        page.text()
            .map_err(|e| PyValueError::new_err(format!("{e}")))
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
}

/// Python module definition.
#[pymodule(name = "djvu_rs")]
fn djvu_rs_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Document>()?;
    m.add_class::<Page>()?;
    m.add_class::<Pixmap>()?;
    Ok(())
}
