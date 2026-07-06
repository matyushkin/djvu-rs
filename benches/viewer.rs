//! C4_TILE_CACHE scenario bench — a scripted interactive pan/zoom session.
//!
//! Round-8 triage flagged the tile-cache question as **NEEDS-INFRA**: no bench
//! exercised the actual interactive-viewer access pattern (repeated,
//! *overlapping* viewport renders), so nobody could tell whether a composited-
//! output tile cache would pay for itself. C3_ZOOM_SCOPE (round 14) showed
//! region/zoom rendering is already linear in viewport pixels with no fixed
//! per-call overhead (~5.5 ns/px) — but that diagnostic never asked what
//! happens when *consecutive* viewports overlap, which is exactly what a real
//! pan gesture does.
//!
//! This harness scripts: open → first full render → zoom 2× at centre → pan
//! across the page in 12 overlapping viewport steps → zoom 4× → pan again.
//! Two page kinds are covered: a colour page (BG44 + FG44/FGbz) and a bilevel
//! page (JB2 mask only, the fast bilevel compositor path).
//!
//! For each pan sequence we bench two variants built from the *existing*
//! `render_region` API (no new production code needed to answer Phase 1):
//!
//! - `full_recomposite`: what the viewer does today — every step re-renders
//!   the entire viewport rectangle, independent of the previous frame.
//! - `incremental_strip`: a proxy for a tile-cache-backed viewer — every step
//!   after the first renders *only* the newly-exposed strip (the part of the
//!   new viewport that did not overlap the previous one); the rest would come
//!   from cached tiles.
//!
//! The ratio `incremental_strip / full_recomposite` (both totals, and the
//! per-step `BenchmarkId`s) directly estimates the compositor-only dividend a
//! tile cache could capture, using the real compositor — not a per-pixel cost
//! model.

#![allow(deprecated)]

use std::hint::black_box;
use std::path::PathBuf;
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use djvu_rs::djvu_document::DjVuPage;
use djvu_rs::djvu_render::{RenderOptions, RenderRect, Resampling, UserRotation, render_region};

fn assets_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/djvujs/library/assets")
}

fn corpus_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/corpus")
}

fn render_opts(width: u32, height: u32) -> RenderOptions {
    RenderOptions {
        width,
        height,
        scale: 0.0,
        bold: 0,
        aa: false,
        rotation: UserRotation::None,
        permissive: false,
        resampling: Resampling::Bilinear,
        mask_aa: false,
    }
}

/// Number of steps in a simulated pan gesture.
const PAN_STEPS: usize = 12;
/// Fractional overlap between consecutive viewport rectangles during a pan.
const OVERLAP: f32 = 0.25;

/// One step of a horizontal pan: the full on-screen viewport rectangle, and
/// (for steps after the first) the newly-exposed strip a tile-cache viewer
/// would actually need to composite from scratch.
struct PanStep {
    /// The rectangle the viewer displays this frame (what `full_recomposite`
    /// renders in full).
    viewport: RenderRect,
    /// The part of `viewport` not covered by the previous step's viewport
    /// (what `incremental_strip` renders). Equals `viewport` for step 0.
    fresh: RenderRect,
}

/// Build a `PAN_STEPS`-long horizontal pan across a `full_w × full_h` render,
/// with a viewport sized so that an `OVERLAP` fraction is shared between
/// consecutive steps and the whole sequence fits within `full_w`.
///
/// `advance = viewport_w * (1 - OVERLAP)`; solving
/// `viewport_w + (PAN_STEPS - 1) * advance <= full_w` for the largest
/// `viewport_w` that keeps a fixed overlap fraction gives
/// `viewport_w = full_w / (1 + (PAN_STEPS - 1) * (1 - OVERLAP))`.
fn build_pan_sequence(full_w: u32, full_h: u32) -> Vec<PanStep> {
    let denom = 1.0 + (PAN_STEPS - 1) as f32 * (1.0 - OVERLAP);
    let viewport_w = ((full_w as f32 / denom).floor() as u32).max(16);
    let viewport_h = ((viewport_w as f32 * 0.75) as u32).clamp(16, full_h.max(16));
    let advance = ((viewport_w as f32) * (1.0 - OVERLAP)).round() as u32;
    let max_x = full_w.saturating_sub(viewport_w);
    let y = (full_h.saturating_sub(viewport_h)) / 2;

    let mut steps = Vec::with_capacity(PAN_STEPS);
    let mut prev_x: u32 = 0;
    for i in 0..PAN_STEPS {
        let x = (i as u32 * advance).min(max_x);
        let viewport = RenderRect {
            x,
            y,
            width: viewport_w,
            height: viewport_h,
        };
        let fresh = if i == 0 {
            viewport
        } else if x > prev_x {
            // New content is exposed on the trailing (right) edge.
            let fresh_x0 = (prev_x + viewport_w).min(x + viewport_w);
            let fresh_w = (x + viewport_w).saturating_sub(fresh_x0);
            RenderRect {
                x: fresh_x0,
                y,
                width: fresh_w.max(1),
                height: viewport_h,
            }
        } else {
            // Pan saturated at the page edge (rounding) — nothing new.
            RenderRect {
                x,
                y,
                width: 1,
                height: viewport_h,
            }
        };
        steps.push(PanStep { viewport, fresh });
        prev_x = x;
    }
    steps
}

fn load_doc(dir: PathBuf, filename: &str) -> Option<djvu_rs::DjVuDocument> {
    let path = dir.join(filename);
    let data = std::fs::read(&path).ok()?;
    djvu_rs::DjVuDocument::parse(&data).ok()
}

/// Run the full scripted session for one page: open, full render, zoom 2×
/// pan, zoom 4× pan. `base_w` is the "fit to window" baseline width (the
/// zoom levels multiply this).
fn bench_viewer_session(c: &mut Criterion, group_prefix: &str, page: &DjVuPage, base_w: u32) {
    let native_w = page.width() as u32;
    let native_h = page.height() as u32;
    let base_h = ((base_w as f32 * native_h as f32) / native_w as f32).round() as u32;

    // ── Open → first full render ────────────────────────────────────────
    let full_opts = render_opts(base_w, base_h);
    {
        let mut group = c.benchmark_group(format!("{group_prefix}_open"));
        group.sample_size(20);
        group.bench_function("first_full_render", |b| {
            b.iter(|| {
                let _ = djvu_rs::djvu_render::render_pixmap(black_box(page), black_box(&full_opts));
            });
        });
        group.finish();
    }

    for &zoom in &[2u32, 4u32] {
        let full_w = base_w * zoom;
        let full_h = base_h * zoom;
        let zoom_opts = render_opts(full_w, full_h);

        // ── Zoom N× at centre ────────────────────────────────────────────
        let cx = full_w / 2;
        let cy = full_h / 2;
        let viewport_w = (full_w / 9).max(16);
        let viewport_h = (viewport_w * 3 / 4).max(16);
        let centre_rect = RenderRect {
            x: cx
                .saturating_sub(viewport_w / 2)
                .min(full_w.saturating_sub(viewport_w)),
            y: cy
                .saturating_sub(viewport_h / 2)
                .min(full_h.saturating_sub(viewport_h)),
            width: viewport_w,
            height: viewport_h,
        };
        {
            let mut group = c.benchmark_group(format!("{group_prefix}_zoom{zoom}x"));
            group.sample_size(20);
            group.bench_function("zoom_at_centre", |b| {
                b.iter(|| {
                    let _ = render_region(
                        black_box(page),
                        black_box(centre_rect),
                        black_box(&zoom_opts),
                    );
                });
            });
            group.finish();
        }

        // ── Pan across the page (12 overlapping steps) ─────────────────
        let steps = build_pan_sequence(full_w, full_h);

        {
            let mut group = c.benchmark_group(format!("{group_prefix}_pan{zoom}x_per_step"));
            group.sample_size(20);
            group.warm_up_time(Duration::from_millis(500));
            group.measurement_time(Duration::from_secs(1));
            for (i, step) in steps.iter().enumerate() {
                group.bench_with_input(BenchmarkId::new("full_recomposite", i), step, |b, step| {
                    b.iter(|| {
                        let _ = render_region(
                            black_box(page),
                            black_box(step.viewport),
                            black_box(&zoom_opts),
                        );
                    });
                });
                group.bench_with_input(
                    BenchmarkId::new("incremental_strip", i),
                    step,
                    |b, step| {
                        b.iter(|| {
                            let _ = render_region(
                                black_box(page),
                                black_box(step.fresh),
                                black_box(&zoom_opts),
                            );
                        });
                    },
                );
            }
            group.finish();
        }

        // ── Whole-sequence totals (one number per variant) ─────────────
        {
            let mut group = c.benchmark_group(format!("{group_prefix}_pan{zoom}x_total"));
            group.sample_size(10);
            group.warm_up_time(Duration::from_millis(500));
            group.measurement_time(Duration::from_secs(2));
            group.bench_function("full_recomposite_sequence", |b| {
                b.iter(|| {
                    for step in &steps {
                        let _ = render_region(
                            black_box(page),
                            black_box(step.viewport),
                            black_box(&zoom_opts),
                        );
                    }
                });
            });
            group.bench_function("incremental_strip_sequence", |b| {
                b.iter(|| {
                    for step in &steps {
                        let _ = render_region(
                            black_box(page),
                            black_box(step.fresh),
                            black_box(&zoom_opts),
                        );
                    }
                });
            });
            group.finish();
        }
    }
}

fn bench_viewer_color(c: &mut Criterion) {
    let doc = match load_doc(assets_path(), "colorbook.djvu") {
        Some(d) => d,
        None => {
            eprintln!("skipping bench_viewer_color: colorbook.djvu not found");
            return;
        }
    };
    let page = match doc.page(0) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("skipping bench_viewer_color: failed to get page 0");
            return;
        }
    };
    bench_viewer_session(c, "viewer_color", page, 1600);
}

fn bench_viewer_bilevel(c: &mut Criterion) {
    let doc = match load_doc(corpus_path(), "cable_1973_100133.djvu") {
        Some(d) => d,
        None => {
            eprintln!("skipping bench_viewer_bilevel: cable_1973_100133.djvu not found");
            return;
        }
    };
    let page = match doc.page(0) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("skipping bench_viewer_bilevel: failed to get page 0");
            return;
        }
    };
    bench_viewer_session(c, "viewer_bilevel", page, 1600);
}

criterion_group!(benches, bench_viewer_color, bench_viewer_bilevel);
criterion_main!(benches);
