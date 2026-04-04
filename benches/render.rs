//! Benchmarks for full DjVu page rendering at various DPI settings.
//!
//! Uses real DjVu test files from the references/ and tests/corpus/ directories.
//! Benchmarks are skipped gracefully if the test files are not found.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::path::PathBuf;

fn assets_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/djvujs/library/assets")
}

fn corpus_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/corpus")
}

/// Load a DjVuDocument from a test file, returning None if not found.
fn load_doc(filename: &str) -> Option<djvu_rs::DjVuDocument> {
    let path = assets_path().join(filename);
    let data = std::fs::read(&path).ok()?;
    djvu_rs::DjVuDocument::parse(&data).ok()
}

fn bench_render_at_dpi(c: &mut Criterion) {
    let doc = match load_doc("boy.djvu") {
        Some(d) => d,
        None => {
            eprintln!("skipping bench_render_at_dpi: boy.djvu not found");
            return;
        }
    };

    let page = match doc.page(0) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("skipping bench_render_at_dpi: failed to get page 0");
            return;
        }
    };

    let native_w = page.width() as u32;
    let native_h = page.height() as u32;
    let native_dpi = page.dpi() as f32;

    let mut group = c.benchmark_group("render_page");

    for &dpi in &[72u32, 144u32, 300u32] {
        let scale = dpi as f32 / native_dpi;
        let w = ((native_w as f32 * scale).round() as u32).max(1);
        let h = ((native_h as f32 * scale).round() as u32).max(1);

        let opts = djvu_rs::djvu_render::RenderOptions {
            width: w,
            height: h,
            scale,
            bold: 0,
            aa: false,
            rotation: djvu_rs::djvu_render::UserRotation::None,
            permissive: false,
        };

        group.bench_with_input(BenchmarkId::new("dpi", dpi), &opts, |b, opts| {
            b.iter(|| {
                let _ = djvu_rs::djvu_render::render_pixmap(black_box(page), black_box(opts));
            });
        });
    }

    group.finish();
}

fn bench_render_coarse(c: &mut Criterion) {
    let doc = match load_doc("boy.djvu") {
        Some(d) => d,
        None => {
            eprintln!("skipping bench_render_coarse: boy.djvu not found");
            return;
        }
    };

    let page = match doc.page(0) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("skipping bench_render_coarse: failed to get page 0");
            return;
        }
    };

    let opts = djvu_rs::djvu_render::RenderOptions {
        width: page.width() as u32,
        height: page.height() as u32,
        scale: 1.0,
        bold: 0,
        aa: false,
        rotation: djvu_rs::djvu_render::UserRotation::None,
            permissive: false,
    };

    c.bench_function("render_coarse", |b| {
        b.iter(|| {
            let _ = djvu_rs::djvu_render::render_coarse(black_box(page), black_box(&opts));
        });
    });
}

/// Benchmark rendering a color page from the public domain corpus.
fn bench_render_corpus_color(c: &mut Criterion) {
    let path = corpus_path().join("watchmaker.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!(
                "skipping bench_render_corpus_color: watchmaker.djvu not found in tests/corpus/"
            );
            return;
        }
    };
    let doc = match djvu_rs::DjVuDocument::parse(&data) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_render_corpus_color: failed to parse watchmaker.djvu");
            return;
        }
    };
    let page = match doc.page(0) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("skipping bench_render_corpus_color: failed to get page 0");
            return;
        }
    };
    let opts = djvu_rs::djvu_render::RenderOptions {
        width: page.width() as u32,
        height: page.height() as u32,
        scale: 1.0,
        bold: 0,
        aa: false,
        rotation: djvu_rs::djvu_render::UserRotation::None,
            permissive: false,
    };
    c.bench_function("render_corpus_color", |b| {
        b.iter(|| {
            let _ = djvu_rs::djvu_render::render_pixmap(black_box(page), black_box(&opts));
        });
    });
}

/// Benchmark rendering a bilevel (JB2-only) page from the public domain corpus.
fn bench_render_corpus_bilevel(c: &mut Criterion) {
    let path = corpus_path().join("cable_1973_100133.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!(
                "skipping bench_render_corpus_bilevel: cable_1973_100133.djvu not found in tests/corpus/"
            );
            return;
        }
    };
    let doc = match djvu_rs::DjVuDocument::parse(&data) {
        Ok(d) => d,
        Err(_) => {
            eprintln!(
                "skipping bench_render_corpus_bilevel: failed to parse cable_1973_100133.djvu"
            );
            return;
        }
    };
    let page = match doc.page(0) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("skipping bench_render_corpus_bilevel: failed to get page 0");
            return;
        }
    };
    let opts = djvu_rs::djvu_render::RenderOptions {
        width: page.width() as u32,
        height: page.height() as u32,
        scale: 1.0,
        bold: 0,
        aa: false,
        rotation: djvu_rs::djvu_render::UserRotation::None,
            permissive: false,
    };
    c.bench_function("render_corpus_bilevel", |b| {
        b.iter(|| {
            let _ = djvu_rs::djvu_render::render_pixmap(black_box(page), black_box(&opts));
        });
    });
}

criterion_group!(
    benches,
    bench_render_at_dpi,
    bench_render_coarse,
    bench_render_corpus_color,
    bench_render_corpus_bilevel
);
criterion_main!(benches);
