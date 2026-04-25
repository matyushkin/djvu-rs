//! JB2 encoder quality benchmark — compares djvu-rs re-encoded Sjbz payload
//! size against the original Sjbz chunk in a DjVu file (typically produced
//! by `cjb2` from DjVuLibre).
//!
//! Also verifies lossless round-trip: the re-encoded stream must decode to
//! the exact same `Bitmap` as the original.
//!
//! Usage:
//!     cargo run --release --example encode_quality_jb2 -- <file.djvu> [<file2.djvu> ...]
//!
//! Output: one JSON line per page (jsonl), plus a final summary table on stderr.
//!
//! Fields per page:
//!   file            — path of the source DjVu
//!   page            — 0-based page index
//!   width, height   — bitmap dimensions
//!   orig_sjbz_bytes — original Sjbz payload size
//!   rs_sjbz_bytes   — djvu-rs re-encoded Sjbz payload size
//!   bpp_orig        — bits per pixel of the original
//!   bpp_rs          — bits per pixel of the re-encoded
//!   size_ratio      — rs_sjbz_bytes / orig_sjbz_bytes (>1.0 means djvu-rs worse)
//!   roundtrip_ok    — re-encoded decodes to the same bitmap

use std::path::Path;
use std::process::ExitCode;

use djvu_rs::{
    DjVuDocument, jb2,
    jb2_encode::{encode_jb2, encode_jb2_dict},
};

#[derive(Copy, Clone, Debug)]
enum RoundtripStatus {
    Ok,
    Mismatch,
    /// Decoder rejected the re-encoded stream — see issue #198
    /// (encoder emits whole image as one type-3 symbol > MAX_SYMBOL_PIXELS).
    DecodeError,
}

impl RoundtripStatus {
    fn as_str(&self) -> &'static str {
        match self {
            RoundtripStatus::Ok => "ok",
            RoundtripStatus::Mismatch => "mismatch",
            RoundtripStatus::DecodeError => "decode_error",
        }
    }
}

struct PageResult {
    file: String,
    page: usize,
    width: u32,
    height: u32,
    orig_sjbz_bytes: usize,
    rs_sjbz_bytes: usize,
    rs_dict_sjbz_bytes: usize,
    roundtrip: RoundtripStatus,
    roundtrip_dict: RoundtripStatus,
}

fn process_file(path: &Path) -> Vec<PageResult> {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("skip {}: {}", path.display(), e);
            return vec![];
        }
    };
    let doc = match DjVuDocument::parse(&data) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("skip {}: parse failed: {}", path.display(), e);
            return vec![];
        }
    };

    let mut out = Vec::new();
    for page_idx in 0..doc.page_count() {
        let page = match doc.page(page_idx) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let orig_sjbz = match page.raw_chunk(b"Sjbz") {
            Some(s) => s,
            None => continue,
        };
        let bitmap = match page.extract_mask() {
            Ok(Some(b)) => b,
            _ => continue,
        };

        let rs_encoded = encode_jb2(&bitmap);
        let rs_dict_encoded = encode_jb2_dict(&bitmap);

        let roundtrip = match jb2::decode(&rs_encoded, None) {
            Ok(b) => {
                if b.width == bitmap.width && b.height == bitmap.height && b.data == bitmap.data {
                    RoundtripStatus::Ok
                } else {
                    RoundtripStatus::Mismatch
                }
            }
            Err(_) => RoundtripStatus::DecodeError,
        };
        let roundtrip_dict = match jb2::decode(&rs_dict_encoded, None) {
            Ok(b) => {
                if b.width == bitmap.width && b.height == bitmap.height && b.data == bitmap.data {
                    RoundtripStatus::Ok
                } else {
                    RoundtripStatus::Mismatch
                }
            }
            Err(_) => RoundtripStatus::DecodeError,
        };

        out.push(PageResult {
            file: path.display().to_string(),
            page: page_idx,
            width: bitmap.width,
            height: bitmap.height,
            orig_sjbz_bytes: orig_sjbz.len(),
            rs_sjbz_bytes: rs_encoded.len(),
            rs_dict_sjbz_bytes: rs_dict_encoded.len(),
            roundtrip,
            roundtrip_dict,
        });
    }
    out
}

fn bpp(bytes: usize, width: u32, height: u32) -> f64 {
    if width == 0 || height == 0 {
        return f64::NAN;
    }
    (bytes as f64 * 8.0) / (width as f64 * height as f64)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!(
            "usage: encode_quality_jb2 <file.djvu> [<file2.djvu> ...]\n\
             \n\
             Re-encodes every Sjbz chunk via djvu-rs and compares size to the\n\
             original. Also verifies lossless round-trip."
        );
        return ExitCode::from(2);
    }

    let mut all: Vec<PageResult> = Vec::new();
    for arg in &args {
        let path = Path::new(arg);
        all.extend(process_file(path));
    }

    for r in &all {
        println!(
            "{{\"file\":\"{}\",\"page\":{},\"width\":{},\"height\":{},\
             \"orig_sjbz_bytes\":{},\"rs_sjbz_bytes\":{},\"rs_dict_sjbz_bytes\":{},\
             \"bpp_orig\":{:.4},\"bpp_rs\":{:.4},\"bpp_rs_dict\":{:.4},\
             \"size_ratio\":{:.3},\"size_ratio_dict\":{:.3},\
             \"roundtrip\":\"{}\",\"roundtrip_dict\":\"{}\"}}",
            r.file,
            r.page,
            r.width,
            r.height,
            r.orig_sjbz_bytes,
            r.rs_sjbz_bytes,
            r.rs_dict_sjbz_bytes,
            bpp(r.orig_sjbz_bytes, r.width, r.height),
            bpp(r.rs_sjbz_bytes, r.width, r.height),
            bpp(r.rs_dict_sjbz_bytes, r.width, r.height),
            r.rs_sjbz_bytes as f64 / r.orig_sjbz_bytes.max(1) as f64,
            r.rs_dict_sjbz_bytes as f64 / r.orig_sjbz_bytes.max(1) as f64,
            r.roundtrip.as_str(),
            r.roundtrip_dict.as_str(),
        );
    }

    if all.is_empty() {
        eprintln!("no JB2 pages processed");
        return ExitCode::from(1);
    }

    let total_orig: usize = all.iter().map(|r| r.orig_sjbz_bytes).sum();
    let total_rs: usize = all.iter().map(|r| r.rs_sjbz_bytes).sum();
    let total_rs_dict: usize = all.iter().map(|r| r.rs_dict_sjbz_bytes).sum();
    let total_pixels: u64 = all.iter().map(|r| r.width as u64 * r.height as u64).sum();

    let roundtrip_ok = all
        .iter()
        .filter(|r| matches!(r.roundtrip, RoundtripStatus::Ok))
        .count();
    let roundtrip_mismatch = all
        .iter()
        .filter(|r| matches!(r.roundtrip, RoundtripStatus::Mismatch))
        .count();
    let roundtrip_decode_err = all
        .iter()
        .filter(|r| matches!(r.roundtrip, RoundtripStatus::DecodeError))
        .count();

    let roundtrip_dict_ok = all
        .iter()
        .filter(|r| matches!(r.roundtrip_dict, RoundtripStatus::Ok))
        .count();
    let roundtrip_dict_mismatch = all
        .iter()
        .filter(|r| matches!(r.roundtrip_dict, RoundtripStatus::Mismatch))
        .count();
    let roundtrip_dict_decode_err = all
        .iter()
        .filter(|r| matches!(r.roundtrip_dict, RoundtripStatus::DecodeError))
        .count();

    eprintln!();
    eprintln!("=== JB2 encoder quality summary ===");
    eprintln!("pages:                  {}", all.len());
    eprintln!();
    eprintln!("-- direct (record type 3, whole image) --");
    eprintln!("roundtrip ok:           {}", roundtrip_ok);
    eprintln!("roundtrip mismatch:     {}", roundtrip_mismatch);
    eprintln!(
        "roundtrip decode err:   {}   (issue #198: >1 MP single-symbol)",
        roundtrip_decode_err
    );
    eprintln!(
        "total rs size:       {:>10} bytes  ({:.4} bpp)",
        total_rs,
        (total_rs as f64 * 8.0) / total_pixels.max(1) as f64
    );
    eprintln!(
        "ratio rs / orig:     {:.3}×",
        total_rs as f64 / total_orig.max(1) as f64
    );
    eprintln!();
    eprintln!("-- dict (CC + rec types 1+7) --");
    eprintln!("roundtrip ok:           {}", roundtrip_dict_ok);
    eprintln!("roundtrip mismatch:     {}", roundtrip_dict_mismatch);
    eprintln!("roundtrip decode err:   {}", roundtrip_dict_decode_err);
    eprintln!(
        "total rs-dict size:  {:>10} bytes  ({:.4} bpp)",
        total_rs_dict,
        (total_rs_dict as f64 * 8.0) / total_pixels.max(1) as f64
    );
    eprintln!(
        "ratio rs-dict / orig: {:.3}×  (>1 = djvu-rs encoder worse)",
        total_rs_dict as f64 / total_orig.max(1) as f64
    );
    eprintln!();
    eprintln!(
        "total orig size:     {:>10} bytes  ({:.4} bpp)",
        total_orig,
        (total_orig as f64 * 8.0) / total_pixels.max(1) as f64
    );

    if roundtrip_mismatch > 0 || roundtrip_dict_mismatch > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
