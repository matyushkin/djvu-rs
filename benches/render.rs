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

    for &dpi in &[72u32, 144u32, 300u32, 600u32] {
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
            resampling: djvu_rs::djvu_render::Resampling::Bilinear,
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
        resampling: djvu_rs::djvu_render::Resampling::Bilinear,
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
        resampling: djvu_rs::djvu_render::Resampling::Bilinear,
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
        resampling: djvu_rs::djvu_render::Resampling::Bilinear,
    };
    c.bench_function("render_corpus_bilevel", |b| {
        b.iter(|| {
            let _ = djvu_rs::djvu_render::render_pixmap(black_box(page), black_box(&opts));
        });
    });
}

/// Benchmark 0.5× scaling with Bilinear vs Lanczos3 resampling on a color page.
///
/// Lanczos3 re-renders at native resolution first, so the difference reveals
/// the cost of the two-pass separable kernel vs the built-in bilinear compositor.
fn bench_render_scaled(c: &mut Criterion) {
    let doc = match load_doc("boy.djvu") {
        Some(d) => d,
        None => {
            eprintln!("skipping bench_render_scaled: boy.djvu not found");
            return;
        }
    };
    let page = match doc.page(0) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("skipping bench_render_scaled: failed to get page 0");
            return;
        }
    };

    let native_w = page.width() as u32;
    let native_h = page.height() as u32;
    let half_w = (native_w / 2).max(1);
    let half_h = (native_h / 2).max(1);

    let mut group = c.benchmark_group("render_scaled_0.5x");

    let opts_bilinear = djvu_rs::djvu_render::RenderOptions {
        width: half_w,
        height: half_h,
        scale: 0.5,
        bold: 0,
        aa: false,
        rotation: djvu_rs::djvu_render::UserRotation::None,
        permissive: false,
        resampling: djvu_rs::djvu_render::Resampling::Bilinear,
    };
    group.bench_function("bilinear", |b| {
        b.iter(|| {
            let _ = djvu_rs::djvu_render::render_pixmap(black_box(page), black_box(&opts_bilinear));
        });
    });

    let opts_lanczos = djvu_rs::djvu_render::RenderOptions {
        width: half_w,
        height: half_h,
        scale: 0.5,
        bold: 0,
        aa: false,
        rotation: djvu_rs::djvu_render::UserRotation::None,
        permissive: false,
        resampling: djvu_rs::djvu_render::Resampling::Lanczos3,
    };
    group.bench_function("lanczos3", |b| {
        b.iter(|| {
            let _ = djvu_rs::djvu_render::render_pixmap(black_box(page), black_box(&opts_lanczos));
        });
    });

    group.finish();
}

/// Benchmark rendering a large color page available in the references directory.
///
/// Uses `colorbook.djvu` (2260×3669 px, 400 dpi, color IW44), rendered at 150 dpi
/// so the output (848×1377 px) is representative of a typical document-viewer request
/// and comparable to the corpus `watchmaker.djvu` benchmark.
fn bench_render_colorbook(c: &mut Criterion) {
    let doc = match load_doc("colorbook.djvu") {
        Some(d) => d,
        None => {
            eprintln!("skipping bench_render_colorbook: colorbook.djvu not found");
            return;
        }
    };
    let page = match doc.page(0) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("skipping bench_render_colorbook: failed to get page 0");
            return;
        }
    };

    let native_dpi = page.dpi() as f32;
    let target_dpi = 150_f32;
    let scale = target_dpi / native_dpi;
    let w = ((page.width() as f32 * scale).round() as u32).max(1);
    let h = ((page.height() as f32 * scale).round() as u32).max(1);

    let opts = djvu_rs::djvu_render::RenderOptions {
        width: w,
        height: h,
        scale,
        bold: 0,
        aa: false,
        rotation: djvu_rs::djvu_render::UserRotation::None,
        permissive: false,
        resampling: djvu_rs::djvu_render::Resampling::Bilinear,
    };

    c.bench_function("render_colorbook", |b| {
        b.iter(|| {
            let _ = djvu_rs::djvu_render::render_pixmap(black_box(page), black_box(&opts));
        });
    });
}

/// Benchmark full DjVu→PDF export pipeline (render + DCTDecode JPEG compression).
fn bench_pdf_export(c: &mut Criterion) {
    let path = corpus_path().join("watchmaker.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_pdf_export: watchmaker.djvu not found in tests/corpus/");
            return;
        }
    };
    let doc = match djvu_rs::DjVuDocument::parse(&data) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_pdf_export: failed to parse watchmaker.djvu");
            return;
        }
    };

    c.bench_function("pdf_export_single_page", |b| {
        b.iter(|| {
            let _ = djvu_rs::pdf::djvu_to_pdf(black_box(&doc));
        });
    });
}

criterion_group!(
    benches,
    bench_render_at_dpi,
    bench_render_coarse,
    bench_render_colorbook,
    bench_render_corpus_color,
    bench_render_corpus_bilevel,
    bench_render_scaled,
    bench_pdf_export,
);
criterion_main!(benches);
