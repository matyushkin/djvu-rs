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

/// Stage-level breakdown for the native-resolution corpus pages used in the
/// DjVuLibre comparison matrix.
///
/// The `render_pixmap` case includes output pixmap allocation and any adapter
/// copies; `render_into_reuse_buffer` isolates strict compositing into an
/// already-allocated RGBA buffer; `render_streaming_discard` measures the
/// row-streaming path without retaining the full output image. The decode stages
/// explain how much of the native color/bilevel gap is codec work vs output
/// materialization.
fn bench_render_native_stage_breakdown(c: &mut Criterion) {
    let cases = [
        ("watchmaker_color", "watchmaker.djvu"),
        ("cable_bilevel", "cable_1973_100133.djvu"),
    ];

    let mut group = c.benchmark_group("render_native_stages");

    for (label, filename) in cases {
        let path = corpus_path().join(filename);
        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(_) => {
                eprintln!("skipping render_native_stages/{label}: {filename} not found");
                continue;
            }
        };
        let doc = match djvu_rs::DjVuDocument::parse(&data) {
            Ok(d) => d,
            Err(_) => {
                eprintln!("skipping render_native_stages/{label}: parse failed");
                continue;
            }
        };
        let page = match doc.page(0) {
            Ok(p) => p,
            Err(_) => continue,
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
        let buf_len = opts.width as usize * opts.height as usize * 4;

        // Warm page-level decode caches so the render stages focus on hot render
        // work, matching the DjVuLibre render-only comparison harness.
        let _ = djvu_rs::djvu_render::render_pixmap(black_box(page), black_box(&opts));

        group.bench_function(BenchmarkId::new("render_pixmap", label), |b| {
            b.iter(|| {
                black_box(
                    djvu_rs::djvu_render::render_pixmap(black_box(page), black_box(&opts))
                        .expect("render_pixmap"),
                );
            });
        });

        let mut buf = vec![0u8; buf_len];
        group.bench_function(BenchmarkId::new("render_into_reuse_buffer", label), |b| {
            b.iter(|| {
                djvu_rs::djvu_render::render_into(
                    black_box(page),
                    black_box(&opts),
                    black_box(buf.as_mut_slice()),
                )
                .expect("render_into");
                black_box(&buf);
            });
        });

        group.bench_function(BenchmarkId::new("render_streaming_discard", label), |b| {
            b.iter(|| {
                let mut bytes = 0usize;
                djvu_rs::djvu_render::render_streaming(
                    black_box(page),
                    black_box(&opts),
                    |_, row| bytes = bytes.wrapping_add(row.len()),
                )
                .expect("render_streaming");
                black_box(bytes);
            });
        });

        if page.find_chunk(b"Sjbz").is_some() {
            group.bench_function(BenchmarkId::new("mask_decode", label), |b| {
                b.iter(|| {
                    black_box(page.extract_mask().expect("mask decode"));
                });
            });
        }

        if page.find_chunk(b"BG44").is_some() {
            group.bench_function(BenchmarkId::new("bg_to_rgb_warm", label), |b| {
                b.iter(|| {
                    if let Some(img) = page.decoded_bg44() {
                        black_box(img.to_rgb_subsample(1).expect("BG44 to RGB"));
                    }
                });
            });
        }
    }

    group.finish();
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

/// Micro-benchmarks for individual render pipeline stages on colorbook page 0.
///
/// Separates: background decode, mask decode, FG44 decode, composite.
fn bench_render_colorbook_stages(c: &mut Criterion) {
    let path = assets_path().join("colorbook.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_render_colorbook_stages: colorbook.djvu not found");
            return;
        }
    };
    let doc = match djvu_rs::DjVuDocument::parse(&data) {
        Ok(d) => d,
        Err(_) => return,
    };
    let page = match doc.page(0) {
        Ok(p) => p,
        Err(_) => return,
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

    // Warm the bg cache (decoded_bg44_partial) — we want to measure each stage in isolation.
    let _ = djvu_rs::djvu_render::render_pixmap(black_box(page), black_box(&opts));

    let mut group = c.benchmark_group("render_colorbook_stages");

    // Stage 1: full render (reference)
    group.bench_function("full_render", |b| {
        b.iter(|| {
            let _ = djvu_rs::djvu_render::render_pixmap(black_box(page), black_box(&opts));
        });
    });

    // Stage 2: background only (BG44 → pixmap, warm cache)
    group.bench_function("bg_only_warm", |b| {
        b.iter(|| {
            let _ = page.decoded_bg44_partial();
        });
    });

    // Stage 3: JB2 mask decode only
    group.bench_function("mask_decode", |b| {
        b.iter(|| {
            let _ = black_box(page.extract_mask());
        });
    });

    group.finish();
}

/// Cold-path benchmark: parse document + render at 150 dpi inside the loop.
///
/// Each iteration recreates the document (and therefore the IW44 decode cache),
/// measuring the true first-render cost: ZP arithmetic decode + wavelet + RGB.
/// This is the scenario that benefits most from partial chunk decode.
fn bench_render_colorbook_cold(c: &mut Criterion) {
    let path = assets_path().join("colorbook.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_render_colorbook_cold: colorbook.djvu not found");
            return;
        }
    };

    // Parse once to read page geometry; re-parse inside iter to reset caches.
    let doc0 = match djvu_rs::DjVuDocument::parse(&data) {
        Ok(d) => d,
        Err(_) => return,
    };
    let page0 = match doc0.page(0) {
        Ok(p) => p,
        Err(_) => return,
    };
    let native_dpi = page0.dpi() as f32;
    let target_dpi = 150_f32;
    let scale = target_dpi / native_dpi;
    let w = ((page0.width() as f32 * scale).round() as u32).max(1);
    let h = ((page0.height() as f32 * scale).round() as u32).max(1);

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

    c.bench_function("render_colorbook_cold", |b| {
        b.iter(|| {
            // Re-parse to get a fresh page with empty caches.
            let doc = djvu_rs::DjVuDocument::parse(black_box(&data)).unwrap();
            let page = doc.page(0).unwrap();
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

    c.bench_function("pdf_export_sequential", |b| {
        b.iter(|| {
            let _ = djvu_rs::pdf::djvu_to_pdf(black_box(&doc));
        });
    });

    // pdf_export_parallel calls the same API as pdf_export_sequential; the difference
    // is that this binary was compiled with --features parallel, so djvu_to_pdf uses
    // rayon internally. To compare the two, run each in a separate cargo bench invocation:
    //   cargo bench --bench render --features std          -- pdf_export_sequential
    //   cargo bench --bench render --features std,parallel -- pdf_export_parallel
    #[cfg(feature = "parallel")]
    c.bench_function("pdf_export_parallel", |b| {
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
    bench_render_colorbook_stages,
    bench_render_colorbook_cold,
    bench_render_corpus_color,
    bench_render_corpus_bilevel,
    bench_render_native_stage_breakdown,
    bench_render_scaled,
    bench_pdf_export,
);
criterion_main!(benches);
