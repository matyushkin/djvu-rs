//! Multi-page shared-Djbz quality benchmark (#194 Phase 2).
//!
//! For each multi-page DjVu input, extracts every page's bilevel mask, then
//! compares three encoder modes:
//!   1. **Original** — sum of source Sjbz chunks (typically `cjb2`).
//!   2. **Independent** — `encode_jb2_dict(page)` per page, summed.
//!   3. **Bundled** — `encode_djvm_bundle_jb2(pages, threshold)` total bytes
//!      (Djbz + per-page Sjbz, IFF/DIRM overhead included).
//!
//! Win = bundled / independent < 1.0.
//!
//! On 517-page `pathogenic_bacteria_1896.djvu` (real cjb2 scan corpus) the
//! shipped byte-exact clustering (`--diff-fraction 0`) gives bundle = 87.0%
//! of independent (−13.0%); 1%/2% Hamming clustering matches it within 0.05%;
//! 3% introduces decode mismatches under rec-6 refinement. See CLAUDE.md
//! "Multi-page shared Djbz dictionary, Phase 2" for the full investigation.
//!
//! Usage:
//!     cargo run --release --example encode_quality_djbz -- <file.djvu> [<file2.djvu> ...]
//!         [--threshold N]   # default 2 (default for encode_djvm_bundle_jb2)
//!
//! Output: per-file JSON line on stdout + summary table on stderr.
//!
//! Verifies bundle round-trip (every page decodes pixel-exact).

use std::path::Path;
use std::process::ExitCode;

use djvu_rs::{
    Bitmap, DjVuDocument,
    jb2_encode::{
        CcStats, analyze_jb2_cc_stats, cluster_shared_symbols_tunable,
        encode_djvm_bundle_jb2_with_shared, encode_jb2_dict, encode_jb2_dict_with_shared,
        encode_jb2_djbz,
    },
};

struct FileResult {
    file: String,
    pages: usize,
    width_h_avg: (u32, u32),
    orig_total: usize,
    indep_total: usize,
    bundle_total: usize,
    roundtrip_ok: bool,
    #[allow(dead_code)]
    diff_fraction: u32,
}

fn collect_pages(path: &Path) -> Option<(Vec<Bitmap>, usize, (u32, u32))> {
    let data = std::fs::read(path).ok()?;
    let doc = DjVuDocument::parse(&data).ok()?;
    let mut pages = Vec::new();
    let mut orig_total = 0usize;
    let (mut sw, mut sh, mut n) = (0u64, 0u64, 0u64);
    for i in 0..doc.page_count() {
        let p = match doc.page(i) {
            Ok(p) => p,
            Err(_) => continue,
        };
        if let Some(s) = p.raw_chunk(b"Sjbz") {
            orig_total += s.len();
        }
        let bm = match p.extract_mask() {
            Ok(Some(b)) => b,
            _ => continue,
        };
        sw += bm.width as u64;
        sh += bm.height as u64;
        n += 1;
        pages.push(bm);
    }
    if pages.is_empty() {
        return None;
    }
    let avg = ((sw / n) as u32, (sh / n) as u32);
    Some((pages, orig_total, avg))
}

/// Aggregate per-page CC stats across all pages of a bundle.
fn aggregate_cc_stats(pages: &[Bitmap], shared: &[Bitmap]) -> CcStats {
    let mut agg = CcStats::default();
    for p in pages {
        let s = analyze_jb2_cc_stats(p, shared);
        agg.total_ccs += s.total_ccs;
        agg.rec_7_exact += s.rec_7_exact;
        agg.rec_6_refine_shared += s.rec_6_refine_shared;
        agg.rec_6_refine_local += s.rec_6_refine_local;
        agg.rec_1_new += s.rec_1_new;
        agg.pixels_rec_1 += s.pixels_rec_1;
        agg.pixels_rec_6 += s.pixels_rec_6;
        agg.pixels_rec_7 += s.pixels_rec_7;
        agg.rec_6_hamming.extend(s.rec_6_hamming);
    }
    agg
}

/// Print a fixed-bin Hamming-distance histogram for rec-6 emissions.
/// Bins are in % of pixel-count (Hamming / pixel_count × 100), with the
/// caveat that we don't have per-CC pixel context in the aggregate, so
/// we report by absolute Hamming distance bucketed into power-of-two bins.
fn print_hamming_histogram(hamming: &[u32]) {
    if hamming.is_empty() {
        eprintln!("  rec-6 Hamming histogram: (no rec-6 emissions)");
        return;
    }
    let bins: [(u32, u32, &str); 7] = [
        (0, 0, "=0"),
        (1, 4, "1-4"),
        (5, 16, "5-16"),
        (17, 64, "17-64"),
        (65, 256, "65-256"),
        (257, 1024, "257-1024"),
        (1025, u32::MAX, ">1024"),
    ];
    let total = hamming.len();
    eprintln!("  rec-6 Hamming histogram ({} matches):", total);
    for (lo, hi, label) in bins {
        let n = hamming.iter().filter(|&&h| h >= lo && h <= hi).count();
        let pct = n as f64 / total as f64 * 100.0;
        eprintln!("    {:>10}: {:>6} ({:>5.1}%)", label, n, pct);
    }
}

fn process_file(
    path: &Path,
    threshold: usize,
    diff_fraction: u32,
    cc_stats: bool,
) -> Option<FileResult> {
    let (pages, orig_total, avg) = match collect_pages(path) {
        Some(t) => t,
        None => {
            eprintln!("skip {}: no JB2 pages", path.display());
            return None;
        }
    };

    let indep_total: usize = pages
        .iter()
        .map(|p: &Bitmap| encode_jb2_dict(p).len())
        .sum();

    // Build the bundle with a tunable cluster Hamming threshold so we can
    // sweep different fractions independently of the shipped default.
    let shared = cluster_shared_symbols_tunable(&pages, threshold, diff_fraction);
    let bundle = encode_djvm_bundle_jb2_with_shared(&pages, &shared);
    let bundle_total = bundle.len();

    // Diagnostic breakdown: how big is the shared dict, vs per-page Sjbz?
    let djbz_bytes = if shared.is_empty() {
        0
    } else {
        encode_jb2_djbz(&shared).len()
    };
    let sjbz_total: usize = pages
        .iter()
        .map(|p| encode_jb2_dict_with_shared(p, &shared).len())
        .sum();
    eprintln!(
        "  {}: diff={}% shared_syms={} djbz={}B sjbz_total={}B (jb2={}B, container_overhead={}B)",
        path.display(),
        diff_fraction,
        shared.len(),
        djbz_bytes,
        sjbz_total,
        djbz_bytes + sjbz_total,
        bundle_total.saturating_sub(djbz_bytes + sjbz_total),
    );

    if cc_stats {
        let agg = aggregate_cc_stats(&pages, &shared);
        eprintln!(
            "  CC accounting: total={} rec-1={} ({:.1}%) rec-6_shared={} ({:.1}%) \
             rec-6_local={} ({:.1}%) rec-7_exact={} ({:.1}%)",
            agg.total_ccs,
            agg.rec_1_new,
            agg.rec_1_new as f64 / agg.total_ccs.max(1) as f64 * 100.0,
            agg.rec_6_refine_shared,
            agg.rec_6_refine_shared as f64 / agg.total_ccs.max(1) as f64 * 100.0,
            agg.rec_6_refine_local,
            agg.rec_6_refine_local as f64 / agg.total_ccs.max(1) as f64 * 100.0,
            agg.rec_7_exact,
            agg.rec_7_exact as f64 / agg.total_ccs.max(1) as f64 * 100.0,
        );
        eprintln!(
            "  pixel volume: rec-1={} px  rec-6={} px  rec-7={} px",
            agg.pixels_rec_1, agg.pixels_rec_6, agg.pixels_rec_7,
        );
        print_hamming_histogram(&agg.rec_6_hamming);
    }

    // Verify round-trip — every page must decode pixel-exact.
    let roundtrip_ok = match DjVuDocument::parse(&bundle) {
        Ok(doc) if doc.page_count() == pages.len() => {
            (0..pages.len()).all(|i| match doc.page(i).and_then(|p| p.extract_mask()) {
                Ok(Some(d)) => {
                    d.width == pages[i].width
                        && d.height == pages[i].height
                        && d.data == pages[i].data
                }
                _ => false,
            })
        }
        _ => false,
    };

    Some(FileResult {
        file: path.display().to_string(),
        pages: pages.len(),
        width_h_avg: avg,
        orig_total,
        indep_total,
        bundle_total,
        roundtrip_ok,
        diff_fraction,
    })
}

fn main() -> ExitCode {
    let mut threshold = 2usize;
    // Default 0 = byte-exact, the shipped clustering. Higher values opt
    // into Hamming clustering for experimentation.
    let mut diff_fraction: u32 = 0;
    let mut cc_stats = false;
    let mut files: Vec<String> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--threshold" {
            threshold = args
                .next()
                .and_then(|v| v.parse().ok())
                .unwrap_or(threshold);
        } else if a == "--diff-fraction" {
            diff_fraction = args
                .next()
                .and_then(|v| v.parse().ok())
                .unwrap_or(diff_fraction);
        } else if a == "--cc-stats" {
            cc_stats = true;
        } else {
            files.push(a);
        }
    }

    if files.is_empty() {
        eprintln!(
            "usage: encode_quality_djbz [--threshold N] [--diff-fraction P] [--cc-stats] <file.djvu> [...]\n\
             \n\
             Encodes every multi-page input via encode_djvm_bundle_jb2 and\n\
             reports total bytes vs. independent per-page dict + original Sjbz.\n\
             \n\
             --threshold N      promote glyph clusters that span >= N pages (default 2)\n\
             --diff-fraction P  Hamming distance allowance, percent of pixels (0..=10, default 0 = byte-exact)\n\
             --cc-stats         print per-CC record-type accounting + Hamming histogram (#194 Phase 2.5)"
        );
        return ExitCode::from(2);
    }

    let mut all = Vec::new();
    for f in &files {
        if let Some(r) = process_file(Path::new(f), threshold, diff_fraction, cc_stats) {
            all.push(r);
        }
    }

    if all.is_empty() {
        eprintln!("no files processed");
        return ExitCode::from(1);
    }

    for r in &all {
        println!(
            "{{\"file\":\"{}\",\"pages\":{},\"avg_w\":{},\"avg_h\":{},\
             \"orig_bytes\":{},\"indep_bytes\":{},\"bundle_bytes\":{},\
             \"bundle_vs_indep\":{:.4},\"bundle_vs_orig\":{:.4},\
             \"threshold\":{},\"roundtrip_ok\":{}}}",
            r.file,
            r.pages,
            r.width_h_avg.0,
            r.width_h_avg.1,
            r.orig_total,
            r.indep_total,
            r.bundle_total,
            r.bundle_total as f64 / r.indep_total.max(1) as f64,
            r.bundle_total as f64 / r.orig_total.max(1) as f64,
            threshold,
            r.roundtrip_ok,
        );
    }

    let total_pages: usize = all.iter().map(|r| r.pages).sum();
    let total_orig: usize = all.iter().map(|r| r.orig_total).sum();
    let total_indep: usize = all.iter().map(|r| r.indep_total).sum();
    let total_bundle: usize = all.iter().map(|r| r.bundle_total).sum();
    let any_rt_fail = all.iter().any(|r| !r.roundtrip_ok);

    eprintln!();
    eprintln!(
        "=== shared-Djbz quality summary (threshold={}) ===",
        threshold
    );
    eprintln!("files: {}   pages: {}", all.len(), total_pages);
    eprintln!();
    eprintln!("{:<10} {:>14} {:>10}", "mode", "bytes", "vs orig");
    eprintln!(
        "{:<10} {:>14} {:>10}",
        "original",
        total_orig,
        format!("{:.3}×", 1.0)
    );
    eprintln!(
        "{:<10} {:>14} {:>10}",
        "independent",
        total_indep,
        format!("{:.3}×", total_indep as f64 / total_orig.max(1) as f64),
    );
    eprintln!(
        "{:<10} {:>14} {:>10}",
        "bundled",
        total_bundle,
        format!("{:.3}×", total_bundle as f64 / total_orig.max(1) as f64),
    );
    eprintln!();
    eprintln!(
        "shared-Djbz win (bundle/independent): {:.3}×  ({:+.1}%)",
        total_bundle as f64 / total_indep.max(1) as f64,
        (total_bundle as f64 / total_indep.max(1) as f64 - 1.0) * 100.0,
    );
    eprintln!();
    if any_rt_fail {
        eprintln!("⚠ round-trip FAILED on at least one file");
        return ExitCode::from(1);
    }
    eprintln!("✓ all bundle round-trips pixel-exact");
    ExitCode::SUCCESS
}
