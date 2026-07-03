//! Document-level benchmarks: multi-page parsing, page iteration, text extraction.
//!
//! Uses the public domain corpus file `pathogenic_bacteria_1896.djvu` (520 pages, 25 MB)
//! to measure real-world performance on a large mixed-content document.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use std::path::PathBuf;

fn corpus_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/corpus")
}

fn load_large_doc_bytes() -> Option<Vec<u8>> {
    std::fs::read(corpus_path().join("pathogenic_bacteria_1896.djvu")).ok()
}

/// How long to parse the DJVM directory of a 520-page document.
fn bench_parse_multipage(c: &mut Criterion) {
    let data = match load_large_doc_bytes() {
        Some(d) => d,
        None => {
            eprintln!("skipping bench_parse_multipage: pathogenic_bacteria_1896.djvu not found");
            return;
        }
    };

    c.bench_function("parse_multipage_520p", |b| {
        b.iter(|| {
            let _ = djvu_rs::Document::from_bytes(black_box(data.clone()));
        });
    });
}

/// Iterate all 520 page headers (width/height/dpi) without rendering.
fn bench_iterate_pages(c: &mut Criterion) {
    let data = match load_large_doc_bytes() {
        Some(d) => d,
        None => {
            eprintln!("skipping bench_iterate_pages: pathogenic_bacteria_1896.djvu not found");
            return;
        }
    };
    let doc = match djvu_rs::Document::from_bytes(data) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_iterate_pages: failed to parse document");
            return;
        }
    };

    c.bench_function("iterate_pages_520p", |b| {
        b.iter(|| {
            let count = doc.page_count();
            for i in 0..count {
                if let Ok(page) = doc.page(black_box(i)) {
                    let _ = black_box((page.width(), page.height(), page.dpi()));
                }
            }
        });
    });
}

/// Render first page of the large document (mixed IW44+JB2 content).
fn bench_render_large_doc_first(c: &mut Criterion) {
    let data = match load_large_doc_bytes() {
        Some(d) => d,
        None => {
            eprintln!(
                "skipping bench_render_large_doc_first: pathogenic_bacteria_1896.djvu not found"
            );
            return;
        }
    };
    let doc = match djvu_rs::Document::from_bytes(data) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_render_large_doc_first: failed to parse document");
            return;
        }
    };
    let page = match doc.page(0) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("skipping bench_render_large_doc_first: failed to get page 0");
            return;
        }
    };

    c.bench_function("render_large_doc_first_page", |b| {
        b.iter(|| {
            let _ = black_box(page.render());
        });
    });
}

/// Render a mid-document page (page 260 of 520) — tests random-access performance.
fn bench_render_large_doc_mid(c: &mut Criterion) {
    let data = match load_large_doc_bytes() {
        Some(d) => d,
        None => {
            eprintln!(
                "skipping bench_render_large_doc_mid: pathogenic_bacteria_1896.djvu not found"
            );
            return;
        }
    };
    let doc = match djvu_rs::Document::from_bytes(data) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_render_large_doc_mid: failed to parse document");
            return;
        }
    };
    let page = match doc.page(260) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("skipping bench_render_large_doc_mid: failed to get page 260");
            return;
        }
    };

    c.bench_function("render_large_doc_mid_page", |b| {
        b.iter(|| {
            let _ = black_box(page.render());
        });
    });
}

/// Isolate JB2 decode for page 260 (the mid-page benchmark).
fn bench_decode_mask_mid(c: &mut Criterion) {
    let data = match load_large_doc_bytes() {
        Some(d) => d,
        None => {
            eprintln!("skipping bench_decode_mask_mid: pathogenic_bacteria_1896.djvu not found");
            return;
        }
    };
    let doc = match djvu_rs::Document::from_bytes(data) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_decode_mask_mid: failed to parse document");
            return;
        }
    };
    let page = match doc.page(260) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("skipping bench_decode_mask_mid: failed to get page 260");
            return;
        }
    };

    c.bench_function("decode_mask_mid_600dpi", |b| {
        b.iter(|| {
            let _ = black_box(page.decode_mask());
        });
    });
}

/// Isolate JB2 decode from composite: measure just `decode_mask()` on large page.
fn bench_decode_mask_large(c: &mut Criterion) {
    let data = match load_large_doc_bytes() {
        Some(d) => d,
        None => {
            eprintln!("skipping bench_decode_mask_large: pathogenic_bacteria_1896.djvu not found");
            return;
        }
    };
    let doc = match djvu_rs::Document::from_bytes(data) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_decode_mask_large: failed to parse document");
            return;
        }
    };
    let page = match doc.page(0) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("skipping bench_decode_mask_large: failed to get page 0");
            return;
        }
    };

    c.bench_function("decode_mask_large_600dpi", |b| {
        b.iter(|| {
            let _ = black_box(page.decode_mask());
        });
    });
}

/// Text layer extraction: extract plain text from watchmaker.djvu (has TXTz).
fn bench_text_extraction(c: &mut Criterion) {
    let path = corpus_path().join("watchmaker.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_text_extraction: watchmaker.djvu not found");
            return;
        }
    };
    let doc = match djvu_rs::Document::from_bytes(data) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_text_extraction: failed to parse watchmaker.djvu");
            return;
        }
    };
    let page = match doc.page(0) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("skipping bench_text_extraction: failed to get page 0");
            return;
        }
    };

    c.bench_function("text_extraction_single_page", |b| {
        b.iter(|| {
            let _ = black_box(page.text());
        });
    });
}

/// Decode the JB2 masks of the first 30 pages of a bundled shared-dictionary
/// document, re-parsing the document fresh each iteration so the shared-dict
/// caches start cold.
///
/// `DjVu3Spec_bundled.djvu` is 71 pages sharing 5 DJVI dictionaries (via INCL).
/// Exercises DOC_SHARED_DICT_CACHE: each page's mask decode needs the shared
/// dictionary, so with a per-page decode the same handful of dictionaries were
/// ZP-decoded ~30 times; the document-level shared decode pays it once per
/// unique dictionary. This is the multi-page reading / viewer-scroll workload.
fn bench_shared_dict_mask_decode(c: &mut Criterion) {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/DjVu3Spec_bundled.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_shared_dict_mask_decode: DjVu3Spec_bundled.djvu not found");
            return;
        }
    };

    c.bench_function("shared_dict_mask_decode_30p", |b| {
        b.iter(|| {
            let doc = djvu_rs::djvu_document::DjVuDocument::parse(black_box(&data))
                .expect("parse bundled doc");
            let n = 30.min(doc.page_count());
            for i in 0..n {
                if let Ok(page) = doc.page(i) {
                    let _ = black_box(page.extract_mask());
                }
            }
        });
    });
}

/// Cold open of a 520-page bundled document + render of page 1 only.
///
/// The product-visible win from LAZY_PAGE_CONSTRUCT: a viewer opening a large
/// book and showing the first page should not pay to copy all 520 pages' chunks.
/// Re-parses fresh each iteration (via `Document::from_bytes`) so the open cost
/// is included, then renders page 0.
fn bench_open_and_render_first(c: &mut Criterion) {
    let data = match load_large_doc_bytes() {
        Some(d) => d,
        None => {
            eprintln!("skipping bench_open_and_render_first: pathogenic not found");
            return;
        }
    };
    c.bench_function("open_and_render_first_page_520p", |b| {
        b.iter(|| {
            let doc =
                djvu_rs::Document::from_bytes(black_box(data.clone())).expect("open bundled doc");
            let page = doc.page(0).expect("page 0");
            let _ = black_box(page.render());
        });
    });
}

criterion_group!(
    benches,
    bench_parse_multipage,
    bench_iterate_pages,
    bench_open_and_render_first,
    bench_render_large_doc_first,
    bench_render_large_doc_mid,
    bench_decode_mask_large,
    bench_decode_mask_mid,
    bench_text_extraction,
    bench_shared_dict_mask_decode,
);
criterion_main!(benches);
