//! Guard: the read path holds bounded memory without the caller asking
//! (READ_CACHE_BOUNDED).
//!
//! Rendering memoises what it decoded. Until 0.33 nothing ever gave that
//! memory back on its own: the eviction API needed `&mut DjVuDocument`, every
//! render entry point holds a shared `&DjVuPage`, and a program that only
//! rendered therefore grew about 5 MB per page of a colour book until it ran
//! out. `djvu_rs::render_cache` closes that — a process-wide ceiling, swept
//! when a cache fill crosses it.
//!
//! The ceiling is process-global state, and `cargo test` runs a binary's tests
//! on parallel threads, so every test here takes `BUDGET_LOCK` and restores the
//! default ceiling before it returns.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use djvu_rs::djvu_document::{DjVuDocument, DjVuPage};
use djvu_rs::djvu_render::{RenderOptions, render_pixmap};
use djvu_rs::render_cache;

/// A colour book with 62 pages: its pages carry BG44, so a full-resolution
/// render really costs megabytes and a ceiling has something to bite on.
const SUBJECT: &str = "tests/fixtures/colorbook.djvu";

/// Serialises the tests in this file — they all move the process-wide ceiling.
static BUDGET_LOCK: Mutex<()> = Mutex::new(());

/// Take the lock, start from a cold cache, and put the default ceiling back
/// when the guard is dropped.
fn exclusive() -> MutexGuard<'static, ()> {
    let guard = BUDGET_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    render_cache::set_budget(render_cache::DEFAULT_BUDGET);
    render_cache::clear();
    guard
}

fn full_size(page: &DjVuPage) -> RenderOptions {
    RenderOptions {
        width: page.width() as u32,
        height: page.height() as u32,
        ..Default::default()
    }
}

/// Render the first `pages` pages of the subject and return the highest
/// process-wide resident total seen along the way.
fn render_pages(doc: &DjVuDocument, pages: usize) -> usize {
    let mut peak = 0;
    for i in 0..pages {
        let page = doc.page(i).expect("page in range");
        let _ = render_pixmap(page, &full_size(page)).expect("render must succeed");
        peak = peak.max(render_cache::resident_bytes());
    }
    peak
}

fn open_subject() -> DjVuDocument {
    let data = std::fs::read(SUBJECT).expect("fixture must exist");
    DjVuDocument::parse(&data).expect("fixture must parse")
}

#[test]
fn the_ceiling_is_on_by_default() {
    let _guard = exclusive();
    assert_eq!(
        render_cache::budget(),
        render_cache::DEFAULT_BUDGET,
        "a program that sets no ceiling must still get one; an unbounded \
         default is the defect READ_CACHE_BOUNDED fixes"
    );
    assert_eq!(
        render_cache::DEFAULT_BUDGET,
        256 * 1024 * 1024,
        "the documented default is 256 MiB; change the docs with the constant"
    );
}

#[test]
fn a_long_read_stays_under_the_ceiling() {
    let _guard = exclusive();
    let doc = open_subject();

    // One page first, to learn what a page of this book costs.
    let one = render_pages(&doc, 1);
    assert!(
        one > 1 << 20,
        "{SUBJECT} now costs only {one} B for a full-resolution page; this \
         guard needs a book whose pages cost megabytes. Replace the fixture."
    );

    // A ceiling of three pages, then render ten. Without the governor the
    // total would be ten pages' worth and keep climbing.
    let ceiling = one * 3;
    render_cache::set_budget(ceiling);
    let peak = render_pages(&doc, 10);

    // The page being rendered is never evicted, so the peak can exceed the
    // ceiling by up to one page's cache. Nothing else may.
    assert!(
        peak <= ceiling + one,
        "render cache peaked at {peak} B against a {ceiling} B ceiling \
         ({one} B/page). The sweep is not running, or it is protecting more \
         than the page being filled."
    );

    render_cache::set_budget(render_cache::DEFAULT_BUDGET);
}

#[test]
fn the_ceiling_can_be_lifted() {
    let _guard = exclusive();
    let doc = open_subject();

    let one = render_pages(&doc, 1);
    render_cache::clear();

    // `usize::MAX` is the documented opt-out: the 0.32 behaviour, for a caller
    // who manages memory itself.
    render_cache::set_budget(usize::MAX);
    let peak = render_pages(&doc, 6);
    assert!(
        peak > one * 4,
        "with the ceiling lifted the cache must grow freely: {peak} B after \
         six pages of {one} B each"
    );

    render_cache::set_budget(render_cache::DEFAULT_BUDGET);
}

#[test]
fn eviction_never_pulls_a_layer_out_from_under_a_reader() {
    let _guard = exclusive();
    let doc = open_subject();

    let page = doc.page(0).expect("page 0");
    let _ = render_pixmap(page, &full_size(page)).expect("render must succeed");
    let mask = page
        .decoded_mask()
        .expect("colorbook page 0 has a JB2 mask");
    let before = mask.data.len();
    assert!(before > 0, "the mask must hold real bytes");

    // The whole point of handing out shared handles: the cache can drop its
    // own copy at any moment and a reader that already has one is unaffected.
    render_cache::clear();
    assert_eq!(
        mask.data.len(),
        before,
        "a held layer must survive the eviction of the cache it came from"
    );
    assert_eq!(
        Arc::strong_count(&mask),
        1,
        "after the sweep the reader must hold the only handle; a leftover \
         reference means the cache did not really let go"
    );
    assert_eq!(
        doc.render_cache_bytes(),
        0,
        "clear() must leave the document's caches empty"
    );

    // And the layer is still reachable — it decodes again, identically.
    let again = page
        .decoded_mask()
        .expect("mask decodes again after eviction");
    assert_eq!(again.data, mask.data, "a rebuilt layer must be identical");

    render_cache::set_budget(render_cache::DEFAULT_BUDGET);
}

#[test]
fn a_page_can_drop_its_own_cache_through_a_shared_borrow() {
    let _guard = exclusive();
    let doc = open_subject();

    let page = doc.page(0).expect("page 0");
    let _ = render_pixmap(page, &full_size(page)).expect("render must succeed");
    assert!(
        page.render_cache_bytes() > 0,
        "the render must leave a cache"
    );

    // `&self`, not `&mut self`: this is what lets a render bound itself.
    page.evict_render_cache();
    assert_eq!(
        page.render_cache_bytes(),
        0,
        "evict_render_cache() through a shared borrow must free the page"
    );
    assert_eq!(
        render_cache::resident_bytes(),
        0,
        "the process-wide total must follow a per-page eviction"
    );
}
