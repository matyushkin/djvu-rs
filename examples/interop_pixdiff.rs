//! DjVuLibre interop pixel-diff harness (round-5 #14).
//!
//! Renders a page with our decoder and with DjVuLibre's `ddjvu` at the same
//! native resolution, then reports the per-pixel RGB difference distribution.
//! This is the quantitative "quality floor" for future render experiments: a
//! change that claims to improve quality can be judged against the reference,
//! and a change that must stay faithful can prove it did not drift.
//!
//! Not a byte-identity check — our chroma upsampling (#422 bilinear) and
//! anti-aliasing are deliberately different from (often better than) DjVuLibre,
//! so small diffs are expected. The harness surfaces the *distribution* so
//! regressions (a new large tail) are visible against the established baseline.
//!
//! Usage:
//!   cargo run --release --example interop_pixdiff -- <file.djvu> [page]
//!   cargo run --release --example interop_pixdiff -- --corpus   # sweep a set
//!
//! Requires `ddjvu` on PATH (DjVuLibre).
#![allow(deprecated)]

use djvu_rs::djvu_document::DjVuDocument;
use djvu_rs::djvu_render::{self, RenderOptions, Resampling, UserRotation};
use std::path::{Path, PathBuf};
use std::process::Command;

fn native_opts(w: u32, h: u32) -> RenderOptions {
    RenderOptions {
        width: w,
        height: h,
        scale: 1.0,
        bold: 0,
        aa: false,
        rotation: UserRotation::None,
        permissive: false,
        resampling: Resampling::Bilinear,
    }
}

/// Parse a binary PPM (P6). Returns (width, height, rgb_bytes).
fn parse_ppm(data: &[u8]) -> Option<(usize, usize, Vec<u8>)> {
    // Magic
    if &data.get(0..2)? != b"P6" {
        return None;
    }
    let mut pos = 2usize;
    // Read three ASCII integers (width, height, maxval), skipping whitespace and
    // '#' comment lines.
    let mut nums = [0usize; 3];
    for slot in nums.iter_mut() {
        // skip whitespace / comments
        loop {
            let b = *data.get(pos)?;
            if b == b'#' {
                while *data.get(pos)? != b'\n' {
                    pos += 1;
                }
            } else if b.is_ascii_whitespace() {
                pos += 1;
            } else {
                break;
            }
        }
        let mut v = 0usize;
        while let Some(&b) = data.get(pos) {
            if b.is_ascii_digit() {
                v = v * 10 + (b - b'0') as usize;
                pos += 1;
            } else {
                break;
            }
        }
        *slot = v;
    }
    // Exactly one whitespace byte separates the header from the pixel data.
    pos += 1;
    let (w, h, maxval) = (nums[0], nums[1], nums[2]);
    if maxval != 255 {
        return None;
    }
    let need = w * h * 3;
    let rgb = data.get(pos..pos + need)?.to_vec();
    Some((w, h, rgb))
}

struct DiffStats {
    w: usize,
    h: usize,
    mean_abs: f64,
    max_abs: u8,
    p50: u8,
    p95: u8,
    p99: u8,
    pct_gt2: f64,
    pct_gt8: f64,
    pct_gt32: f64,
}

/// Per-pixel, per-channel absolute difference distribution.
fn diff_stats(w: usize, h: usize, ours_rgba: &[u8], ref_rgb: &[u8]) -> DiffStats {
    // Histogram of absolute per-channel differences (0..=255).
    let mut hist = [0u64; 256];
    let mut sum: u64 = 0;
    let mut max_abs = 0u8;
    let n_channels = w * h * 3;
    for p in 0..(w * h) {
        for c in 0..3 {
            let a = ours_rgba[p * 4 + c];
            let b = ref_rgb[p * 3 + c];
            let d = a.abs_diff(b);
            hist[d as usize] += 1;
            sum += d as u64;
            if d > max_abs {
                max_abs = d;
            }
        }
    }
    let total = n_channels as u64;
    let pctile = |frac: f64| -> u8 {
        let target = (frac * total as f64) as u64;
        let mut acc = 0u64;
        for (d, &count) in hist.iter().enumerate() {
            acc += count;
            if acc >= target {
                return d as u8;
            }
        }
        255
    };
    let count_gt = |t: usize| -> f64 {
        let over: u64 = hist[(t + 1)..].iter().sum();
        100.0 * over as f64 / total as f64
    };
    DiffStats {
        w,
        h,
        mean_abs: sum as f64 / total as f64,
        max_abs,
        p50: pctile(0.50),
        p95: pctile(0.95),
        p99: pctile(0.99),
        pct_gt2: count_gt(2),
        pct_gt8: count_gt(8),
        pct_gt32: count_gt(32),
    }
}

fn compare_page(path: &Path, page_no: usize) -> Result<DiffStats, String> {
    // Reference render via ddjvu → PPM.
    let out = std::env::temp_dir().join(format!(
        "interop_ref_{}_{}.ppm",
        path.file_stem().and_then(|s| s.to_str()).unwrap_or("x"),
        page_no
    ));
    let status = Command::new("ddjvu")
        .args([
            "-format=ppm",
            &format!("-page={page_no}"),
            &path.to_string_lossy(),
            &out.to_string_lossy(),
        ])
        .status()
        .map_err(|e| format!("failed to run ddjvu: {e}"))?;
    if !status.success() {
        return Err("ddjvu returned non-zero".into());
    }
    let ref_bytes = std::fs::read(&out).map_err(|e| format!("read ppm: {e}"))?;
    let _ = std::fs::remove_file(&out);
    let (rw, rh, ref_rgb) = parse_ppm(&ref_bytes).ok_or("could not parse ddjvu PPM")?;

    // Our render at the same native resolution.
    let data = std::fs::read(path).map_err(|e| format!("read djvu: {e}"))?;
    let doc = DjVuDocument::parse(&data).map_err(|e| format!("parse: {e}"))?;
    let page = doc
        .page(page_no - 1)
        .map_err(|e| format!("page {page_no}: {e}"))?;
    let (pw, ph) = (page.width() as usize, page.height() as usize);
    if (pw, ph) != (rw, rh) {
        return Err(format!(
            "dimension mismatch: ours {pw}x{ph} vs ddjvu {rw}x{rh}"
        ));
    }
    let pm = djvu_render::render_pixmap(page, &native_opts(pw as u32, ph as u32))
        .map_err(|e| format!("render: {e}"))?;
    Ok(diff_stats(rw, rh, &pm.data, &ref_rgb))
}

fn print_row(label: &str, s: &DiffStats) {
    println!(
        "{label:28} {}x{:<5}  mean={:5.2}  p50={:3} p95={:3} p99={:3} max={:3}  >2={:5.2}% >8={:5.2}% >32={:5.2}%",
        s.w, s.h, s.mean_abs, s.p50, s.p95, s.p99, s.max_abs, s.pct_gt2, s.pct_gt8, s.pct_gt32
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    if args.first().map(String::as_str) == Some("--corpus") {
        // A representative spread: bilevel, colour, palette, large.
        let files = [
            ("tests/corpus/watchmaker.djvu", 1),
            ("tests/corpus/cable_1973_100133.djvu", 1),
            ("references/djvujs/library/assets/colorbook.djvu", 1),
            ("references/djvujs/library/assets/navm_fgbz.djvu", 1),
            ("references/djvujs/library/assets/boy.djvu", 1),
            ("references/djvujs/library/assets/carte.djvu", 1),
        ];
        println!(
            "{:28} {:11}  {:11}  {:23} tail (% channels over)",
            "file", "dims", "mean-abs", "percentiles"
        );
        for (rel, page) in files {
            let path = manifest.join(rel);
            if !path.exists() {
                continue;
            }
            match compare_page(&path, page) {
                Ok(s) => print_row(path.file_name().and_then(|s| s.to_str()).unwrap_or(rel), &s),
                Err(e) => println!("{rel:28} SKIP: {e}"),
            }
        }
        return;
    }

    let Some(file) = args.first() else {
        eprintln!("usage: interop_pixdiff <file.djvu> [page] | --corpus");
        std::process::exit(2);
    };
    let page_no: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    match compare_page(Path::new(file), page_no) {
        Ok(s) => print_row(file, &s),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
