//! C FFI bindings for djvu-rs.
//!
//! Exposes a stable C API for opening DjVu documents, querying metadata,
//! rendering pages, and extracting text. All functions are `extern "C"`
//! with no-panic guarantees via `catch_unwind`.

use std::ffi::CString;
use std::os::raw::c_char;
use std::slice;

use crate::Document;
use crate::pixmap::Pixmap;

/// Opaque document handle.
pub struct DjvuDoc {
    inner: Document,
}

/// Opaque pixmap handle.
pub struct DjvuPixmap {
    inner: Pixmap,
}

/// Error information. Caller provides a pointer; callee fills it.
#[repr(C)]
pub struct DjvuError {
    /// Error code: 0 = success, 1 = parse error, 2 = render error, 3 = out of range
    pub code: i32,
    /// Null-terminated error message (owned by the error struct, freed by djvu_error_free).
    pub message: *mut c_char,
}

fn set_error(err: *mut DjvuError, code: i32, msg: &str) {
    if err.is_null() {
        return;
    }
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("unknown error").unwrap());
    unsafe {
        (*err).code = code;
        (*err).message = c_msg.into_raw();
    }
}

fn clear_error(err: *mut DjvuError) {
    if err.is_null() {
        return;
    }
    unsafe {
        (*err).code = 0;
        (*err).message = std::ptr::null_mut();
    }
}

/// Free an error message string.
///
/// # Safety
///
/// `err` must be null or point to a valid `DjvuError` whose `message` field
/// was allocated by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn djvu_error_free(err: *mut DjvuError) {
    if err.is_null() {
        return;
    }
    unsafe {
        if !(*err).message.is_null() {
            drop(CString::from_raw((*err).message));
            (*err).message = std::ptr::null_mut();
        }
    }
}

/// Open a DjVu document from a byte buffer.
///
/// Returns NULL on error. Caller must free with `djvu_doc_free`.
///
/// # Safety
///
/// `data` must point to at least `len` readable bytes, or be null (in which
/// case the function returns NULL and sets `err`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn djvu_doc_open(
    data: *const u8,
    len: usize,
    err: *mut DjvuError,
) -> *mut DjvuDoc {
    clear_error(err);
    let result = std::panic::catch_unwind(|| {
        if data.is_null() || len == 0 {
            return Err("null or empty input".to_string());
        }
        let bytes = unsafe { slice::from_raw_parts(data, len) }.to_vec();
        Document::from_bytes(bytes)
            .map(|doc| Box::into_raw(Box::new(DjvuDoc { inner: doc })))
            .map_err(|e| format!("{e}"))
    });

    match result {
        Ok(Ok(ptr)) => ptr,
        Ok(Err(msg)) => {
            set_error(err, 1, &msg);
            std::ptr::null_mut()
        }
        Err(_) => {
            set_error(err, 1, "panic during document open");
            std::ptr::null_mut()
        }
    }
}

/// Free a document handle.
///
/// # Safety
///
/// `doc` must be null or a pointer returned by `djvu_doc_open` that has not
/// yet been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn djvu_doc_free(doc: *mut DjvuDoc) {
    if !doc.is_null() {
        unsafe { drop(Box::from_raw(doc)) };
    }
}

/// Get the number of pages in the document.
///
/// # Safety
///
/// `doc` must be null or a valid pointer returned by `djvu_doc_open`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn djvu_doc_page_count(doc: *const DjvuDoc) -> usize {
    if doc.is_null() {
        return 0;
    }
    unsafe { (*doc).inner.page_count() }
}

/// Get page width in pixels.
///
/// # Safety
///
/// `doc` must be a valid pointer returned by `djvu_doc_open`.
/// `err` must be null or point to a valid `DjvuError`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn djvu_page_width(
    doc: *const DjvuDoc,
    page: usize,
    err: *mut DjvuError,
) -> u32 {
    clear_error(err);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if doc.is_null() {
            return Err("null document".to_string());
        }
        let doc = unsafe { &(*doc).inner };
        doc.page(page)
            .map(|p| p.width())
            .map_err(|e| format!("{e}"))
    }));
    match result {
        Ok(Ok(w)) => w,
        Ok(Err(msg)) => {
            set_error(err, 3, &msg);
            0
        }
        Err(_) => {
            set_error(err, 3, "panic");
            0
        }
    }
}

/// Get page height in pixels.
///
/// # Safety
///
/// `doc` must be a valid pointer returned by `djvu_doc_open`.
/// `err` must be null or point to a valid `DjvuError`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn djvu_page_height(
    doc: *const DjvuDoc,
    page: usize,
    err: *mut DjvuError,
) -> u32 {
    clear_error(err);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if doc.is_null() {
            return Err("null document".to_string());
        }
        let doc = unsafe { &(*doc).inner };
        doc.page(page)
            .map(|p| p.height())
            .map_err(|e| format!("{e}"))
    }));
    match result {
        Ok(Ok(h)) => h,
        Ok(Err(msg)) => {
            set_error(err, 3, &msg);
            0
        }
        Err(_) => {
            set_error(err, 3, "panic");
            0
        }
    }
}

/// Get page DPI.
///
/// # Safety
///
/// `doc` must be a valid pointer returned by `djvu_doc_open`.
/// `err` must be null or point to a valid `DjvuError`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn djvu_page_dpi(
    doc: *const DjvuDoc,
    page: usize,
    err: *mut DjvuError,
) -> u32 {
    clear_error(err);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if doc.is_null() {
            return Err("null document".to_string());
        }
        let doc = unsafe { &(*doc).inner };
        doc.page(page)
            .map(|p| p.dpi() as u32)
            .map_err(|e| format!("{e}"))
    }));
    match result {
        Ok(Ok(d)) => d,
        Ok(Err(msg)) => {
            set_error(err, 3, &msg);
            0
        }
        Err(_) => {
            set_error(err, 3, "panic");
            0
        }
    }
}

/// Render a page at the given DPI. Returns NULL on error.
/// Caller must free with `djvu_pixmap_free`.
///
/// # Safety
///
/// `doc` must be a valid pointer returned by `djvu_doc_open`.
/// `err` must be null or point to a valid `DjvuError`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn djvu_page_render(
    doc: *const DjvuDoc,
    page: usize,
    dpi: f32,
    err: *mut DjvuError,
) -> *mut DjvuPixmap {
    clear_error(err);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if doc.is_null() {
            return Err("null document".to_string());
        }
        let doc = unsafe { &(*doc).inner };
        let p = doc.page(page).map_err(|e| format!("{e}"))?;
        let native_dpi = p.dpi() as f32;
        let scale = dpi / native_dpi;
        let w = ((p.width() as f32 * scale).round() as u32).max(1);
        let h = ((p.height() as f32 * scale).round() as u32).max(1);
        let pixmap = p.render_to_size(w, h).map_err(|e| format!("{e}"))?;
        Ok(Box::into_raw(Box::new(DjvuPixmap { inner: pixmap })))
    }));
    match result {
        Ok(Ok(ptr)) => ptr,
        Ok(Err(msg)) => {
            set_error(err, 2, &msg);
            std::ptr::null_mut()
        }
        Err(_) => {
            set_error(err, 2, "panic during render");
            std::ptr::null_mut()
        }
    }
}

/// Free a pixmap handle.
///
/// # Safety
///
/// `pm` must be null or a pointer returned by `djvu_page_render` that has not
/// yet been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn djvu_pixmap_free(pm: *mut DjvuPixmap) {
    if !pm.is_null() {
        unsafe { drop(Box::from_raw(pm)) };
    }
}

/// Get pixmap width.
///
/// # Safety
///
/// `pm` must be null or a valid pointer returned by `djvu_page_render`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn djvu_pixmap_width(pm: *const DjvuPixmap) -> u32 {
    if pm.is_null() {
        return 0;
    }
    unsafe { (*pm).inner.width }
}

/// Get pixmap height.
///
/// # Safety
///
/// `pm` must be null or a valid pointer returned by `djvu_page_render`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn djvu_pixmap_height(pm: *const DjvuPixmap) -> u32 {
    if pm.is_null() {
        return 0;
    }
    unsafe { (*pm).inner.height }
}

/// Get pointer to RGBA pixel data. Length = width * height * 4.
/// The pointer is valid until `djvu_pixmap_free` is called.
///
/// # Safety
///
/// `pm` must be null or a valid pointer returned by `djvu_page_render`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn djvu_pixmap_data(pm: *const DjvuPixmap) -> *const u8 {
    if pm.is_null() {
        return std::ptr::null();
    }
    unsafe { (*pm).inner.data.as_ptr() }
}

/// Get the length of the pixmap data buffer in bytes.
///
/// # Safety
///
/// `pm` must be null or a valid pointer returned by `djvu_page_render`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn djvu_pixmap_data_len(pm: *const DjvuPixmap) -> usize {
    if pm.is_null() {
        return 0;
    }
    unsafe { (*pm).inner.data.len() }
}

/// Extract text from a page. Returns NULL if no text layer.
/// Caller must free with `djvu_text_free`.
///
/// # Safety
///
/// `doc` must be a valid pointer returned by `djvu_doc_open`.
/// `err` must be null or point to a valid `DjvuError`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn djvu_page_text(
    doc: *const DjvuDoc,
    page: usize,
    err: *mut DjvuError,
) -> *mut c_char {
    clear_error(err);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if doc.is_null() {
            return Err("null document".to_string());
        }
        let doc = unsafe { &(*doc).inner };
        let p = doc.page(page).map_err(|e| format!("{e}"))?;
        match p.text() {
            Ok(Some(text)) => {
                let c = CString::new(text).map_err(|e| format!("{e}"))?;
                Ok(c.into_raw())
            }
            Ok(None) => Ok(std::ptr::null_mut()),
            Err(e) => Err(format!("{e}")),
        }
    }));
    match result {
        Ok(Ok(ptr)) => ptr,
        Ok(Err(msg)) => {
            set_error(err, 2, &msg);
            std::ptr::null_mut()
        }
        Err(_) => {
            set_error(err, 2, "panic during text extraction");
            std::ptr::null_mut()
        }
    }
}

/// Free a text string returned by `djvu_page_text`.
///
/// # Safety
///
/// `text` must be null or a pointer returned by `djvu_page_text` that has not
/// yet been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn djvu_text_free(text: *mut c_char) {
    if !text.is_null() {
        unsafe { drop(CString::from_raw(text)) };
    }
}
