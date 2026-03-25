//! Benchmarks for the core codec components: BZZ, JB2, and IW44.
//!
//! These benchmarks use real DjVu test files from the references/ directory.
//! If the test files are not found, benchmarks are skipped gracefully.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use std::path::PathBuf;

fn assets_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/djvujs/library/assets")
}

/// Extract the first BG44 chunk data from a DjVu file, if present.
fn first_bg44_chunk(data: &[u8]) -> Option<Vec<u8>> {
    let form = cos_djvu::iff::parse_form(data).ok()?;
    for chunk in &form.chunks {
        if &chunk.id == b"BG44" {
            return Some(chunk.data.to_vec());
        }
    }
    // For multi-page, look inside FORM sub-chunks
    None
}

/// Extract the first Sjbz chunk data from a DjVu file, if present.
fn first_sjbz_chunk(data: &[u8]) -> Option<Vec<u8>> {
    let form = cos_djvu::iff::parse_form(data).ok()?;
    for chunk in &form.chunks {
        if &chunk.id == b"Sjbz" {
            return Some(chunk.data.to_vec());
        }
    }
    None
}

/// Extract the first BZZ-encoded chunk data (DIRM or NAVM) from a multi-page DjVu.
fn first_bzz_payload(data: &[u8]) -> Option<Vec<u8>> {
    let form = cos_djvu::iff::parse_form(data).ok()?;
    for chunk in &form.chunks {
        if (&chunk.id == b"DIRM" || &chunk.id == b"NAVM") && chunk.data.len() > 1 {
            // Skip the 1-byte flags field
            return Some(chunk.data[1..].to_vec());
        }
    }
    None
}

fn bench_bzz_decode(c: &mut Criterion) {
    let path = assets_path().join("navm_fgbz.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_bzz_decode: navm_fgbz.djvu not found");
            return;
        }
    };

    let bzz_payload = match first_bzz_payload(&data) {
        Some(p) => p,
        None => {
            eprintln!("skipping bench_bzz_decode: no BZZ payload found");
            return;
        }
    };

    c.bench_function("bzz_decode", |b| {
        b.iter(|| {
            let _ = cos_djvu::bzz_new::bzz_decode(black_box(&bzz_payload));
        });
    });
}

fn bench_jb2_decode(c: &mut Criterion) {
    let path = assets_path().join("boy_jb2.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_jb2_decode: boy_jb2.djvu not found");
            return;
        }
    };

    let sjbz = match first_sjbz_chunk(&data) {
        Some(c) => c,
        None => {
            eprintln!("skipping bench_jb2_decode: no Sjbz chunk found");
            return;
        }
    };

    c.bench_function("jb2_decode", |b| {
        b.iter(|| {
            let _ = cos_djvu::jb2_new::decode(black_box(&sjbz), None);
        });
    });
}

fn bench_iw44_decode(c: &mut Criterion) {
    let path = assets_path().join("boy.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_iw44_decode: boy.djvu not found");
            return;
        }
    };

    let bg44 = match first_bg44_chunk(&data) {
        Some(c) => c,
        None => {
            eprintln!("skipping bench_iw44_decode: no BG44 chunk found");
            return;
        }
    };

    c.bench_function("iw44_decode_first_chunk", |b| {
        b.iter(|| {
            let mut img = cos_djvu::iw44_new::Iw44Image::new();
            let _ = img.decode_chunk(black_box(&bg44));
        });
    });
}

criterion_group!(
    benches,
    bench_bzz_decode,
    bench_jb2_decode,
    bench_iw44_decode
);
criterion_main!(benches);
