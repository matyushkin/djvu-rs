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

/// How many pages to pull from a corpus doc to synthesize a TH44 bundle.
/// Kept small: bundle encoding (JB2 shared-dict clustering / IW44 BG44) is
/// the expensive one-time setup step here, not what's being measured.
const TH44_BUNDLE_PAGES: usize = 20;
const TH44_COLOR_BUNDLE_PAGES: usize = 6;

/// Build an in-memory bilevel DJVM bundle whose pages carry real content
/// (masks pulled from `pathogenic_bacteria_1896.djvu`) *and* embedded `TH44`
/// thumbnails, via the #476 encoder (`encode_djvm_bundle_jb2_with_thumbnails`).
///
/// No corpus file embeds a decodable TH44 (`D5_TH44_PREVIEW`, round-9), so the
/// grid API's fast path has nothing to measure against in the existing
/// fixtures — this synthesizes one instead of committing a new binary asset.
fn synthesize_th44_bilevel_bundle() -> Option<Vec<u8>> {
    let data = load_large_doc_bytes()?;
    let doc = djvu_rs::djvu_document::DjVuDocument::parse(&data).ok()?;
    let mut pages = Vec::with_capacity(TH44_BUNDLE_PAGES);
    // Not every page in a mixed-content corpus doc has a JB2 mask (some are
    // pure photo/background pages) — scan forward and skip those, rather than
    // requiring the first N pages specifically to all be bilevel.
    for i in 0..doc.page_count() {
        if pages.len() >= TH44_BUNDLE_PAGES {
            break;
        }
        let Ok(page) = doc.page(i) else { continue };
        if let Ok(Some(bitmap)) = page.extract_mask() {
            pages.push(bitmap);
        }
    }
    if pages.is_empty() {
        return None;
    }
    Some(djvu_rs::jb2_encode::encode_djvm_bundle_jb2_with_thumbnails(
        &pages,
        2, // default shared-dict threshold, matches the encoder's own tests
        djvu_rs::jb2_encode::BUNDLE_DEFAULT_DPI,
        true,
    ))
}

/// Build an in-memory colour-layered DJVM bundle (BG44 + JB2 mask — the
/// scenario `D5_TH44_PREVIEW` measured its 20–30× on) with embedded `TH44`
/// thumbnails, sourced from real page renders of `colorbook.djvu`.
fn synthesize_th44_color_bundle() -> Option<Vec<u8>> {
    let path = corpus_path()
        .parent()?
        .parent()?
        .join("references/djvujs/library/assets/colorbook.djvu");
    let data = std::fs::read(&path).ok()?;
    let doc = djvu_rs::Document::from_bytes(data).ok()?;
    let n = TH44_COLOR_BUNDLE_PAGES.min(doc.page_count());
    let mut pixmaps = Vec::with_capacity(n);
    for i in 0..n {
        let page = doc.page(i).ok()?;
        let pm = page
            .render_to_size(page.width() / 2, page.height() / 2)
            .ok()?;
        pixmaps.push(pm);
    }
    if pixmaps.is_empty() {
        return None;
    }
    djvu_rs::djvu_encode::encode_djvm_layered_shared_with_thumbnails(
        &pixmaps,
        djvu_rs::djvu_encode::EncodeQuality::Quality,
        300,
        None,
        2,
        true,
    )
    .ok()
}

// All four `thumbnails_*_grid_*` benches below re-parse `Document::from_bytes`
// fresh *inside* the timed closure (mirrors `bench_open_and_render_first`):
// a thumbnail grid is a **cold-open** workload — a viewer building a strip
// for pages nobody has scrolled to yet. Reusing one `Document` across
// iterations would let `RenderOnly` hit each page's per-page render cache
// after the first iteration and look artificially free; that cache hit is
// real for *re-rendering an already-open page*, but not for the first-open
// grid-building scenario this API targets.

/// TH44_GRID (bilevel corpus): grid thumbnail build via `Th44Only` — decodes
/// only the small embedded IW44 preview, never the page's JB2 background.
/// Compare against `bench_thumbnails_render_only_grid_bilevel`: TH44 wins
/// here too, though by a smaller and more parallel-sensitive margin than the
/// colour-layered case below (a bilevel JB2 decode+downsample is cheaper
/// per page than a BG44+JB2 composite, so there's less for TH44 to save).
fn bench_thumbnails_th44_only_grid_bilevel(c: &mut Criterion) {
    let bundle = match synthesize_th44_bilevel_bundle() {
        Some(b) => b,
        None => {
            eprintln!(
                "skipping bench_thumbnails_th44_only_grid_bilevel: could not synthesize bundle"
            );
            return;
        }
    };

    c.bench_function("thumbnails_th44_only_grid_bilevel_20p_128px_cold", |b| {
        b.iter(|| {
            let doc = djvu_rs::Document::from_bytes(black_box(bundle.clone()))
                .expect("parse synthesized TH44 bundle");
            let results =
                doc.thumbnails_with_strategy(128, 128, djvu_rs::ThumbnailStrategy::Th44Only);
            for r in &results {
                let _ = black_box(r);
            }
        });
    });
}

/// TH44_GRID (bilevel corpus): render-fallback path for the same grid — full
/// JB2 decode + downscale per page. The baseline `Th44Only` beats above.
fn bench_thumbnails_render_only_grid_bilevel(c: &mut Criterion) {
    let bundle = match synthesize_th44_bilevel_bundle() {
        Some(b) => b,
        None => {
            eprintln!(
                "skipping bench_thumbnails_render_only_grid_bilevel: could not synthesize bundle"
            );
            return;
        }
    };

    c.bench_function("thumbnails_render_only_grid_bilevel_20p_128px_cold", |b| {
        b.iter(|| {
            let doc = djvu_rs::Document::from_bytes(black_box(bundle.clone()))
                .expect("parse synthesized TH44 bundle");
            let results =
                doc.thumbnails_with_strategy(128, 128, djvu_rs::ThumbnailStrategy::RenderOnly);
            for r in &results {
                let _ = black_box(r);
            }
        });
    });
}

/// TH44_GRID (colour-layered corpus): the fast path in the scenario
/// `D5_TH44_PREVIEW` actually measured — a full BG44 (IW44) + JB2 mask
/// composite is expensive even at a small target size, so decoding the
/// small embedded IW44 preview instead is a large, real win. See
/// `examples/thumbnail_grid_quality.rs` for the matching SSIM measurement.
fn bench_thumbnails_th44_only_grid_color(c: &mut Criterion) {
    let bundle = match synthesize_th44_color_bundle() {
        Some(b) => b,
        None => {
            eprintln!(
                "skipping bench_thumbnails_th44_only_grid_color: could not synthesize bundle"
            );
            return;
        }
    };

    c.bench_function("thumbnails_th44_only_grid_color_6p_128px_cold", |b| {
        b.iter(|| {
            let doc = djvu_rs::Document::from_bytes(black_box(bundle.clone()))
                .expect("parse synthesized TH44 bundle");
            let results =
                doc.thumbnails_with_strategy(128, 128, djvu_rs::ThumbnailStrategy::Th44Only);
            for r in &results {
                let _ = black_box(r);
            }
        });
    });
}

/// TH44_GRID (colour-layered corpus): the render-fallback baseline for the
/// same grid — full BG44+JB2 decode and composite, downscaled per page.
fn bench_thumbnails_render_only_grid_color(c: &mut Criterion) {
    let bundle = match synthesize_th44_color_bundle() {
        Some(b) => b,
        None => {
            eprintln!(
                "skipping bench_thumbnails_render_only_grid_color: could not synthesize bundle"
            );
            return;
        }
    };

    c.bench_function("thumbnails_render_only_grid_color_6p_128px_cold", |b| {
        b.iter(|| {
            let doc = djvu_rs::Document::from_bytes(black_box(bundle.clone()))
                .expect("parse synthesized TH44 bundle");
            let results =
                doc.thumbnails_with_strategy(128, 128, djvu_rs::ThumbnailStrategy::RenderOnly);
            for r in &results {
                let _ = black_box(r);
            }
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
    bench_thumbnails_th44_only_grid_bilevel,
    bench_thumbnails_render_only_grid_bilevel,
    bench_thumbnails_th44_only_grid_color,
    bench_thumbnails_render_only_grid_color,
);
criterion_main!(benches);
