//! Benchmarks for the core codec components: BZZ, JB2, and IW44.
//!
//! These benchmarks use real DjVu test files from the references/ and tests/corpus/
//! directories. If the test files are not found, benchmarks are skipped gracefully.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use std::path::PathBuf;

fn assets_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/djvujs/library/assets")
}

fn corpus_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/corpus")
}

/// Search legacy IFF chunks recursively for the first chunk with the given id.
fn find_chunk_legacy<'a>(
    chunks: &'a [djvu_rs::iff::Chunk<'a>],
    target: &[u8; 4],
) -> Option<Vec<u8>> {
    for chunk in chunks {
        match chunk {
            djvu_rs::iff::Chunk::Leaf { id, data } if id == target => {
                return Some(data.to_vec());
            }
            djvu_rs::iff::Chunk::Form { children, .. } => {
                if let Some(found) = find_chunk_legacy(children, target) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// Extract the first BG44 chunk data from a DjVu file, if present.
/// Searches recursively through FORM sub-chunks for multi-page files.
fn first_bg44_chunk(data: &[u8]) -> Option<Vec<u8>> {
    let file = djvu_rs::iff::parse(data).ok()?;
    find_chunk_legacy(file.root.children(), b"BG44")
}

/// Extract the first Sjbz chunk data from a DjVu file, if present.
/// Searches recursively through FORM sub-chunks for multi-page files.
fn first_sjbz_chunk(data: &[u8]) -> Option<Vec<u8>> {
    let file = djvu_rs::iff::parse(data).ok()?;
    find_chunk_legacy(file.root.children(), b"Sjbz")
}

/// Extract the first BZZ-encoded chunk data (DIRM or NAVM) from a multi-page DjVu.
fn first_bzz_payload(data: &[u8]) -> Option<Vec<u8>> {
    let form = djvu_rs::iff::parse_form(data).ok()?;
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
            let _ = djvu_rs::bzz_new::bzz_decode(black_box(&bzz_payload));
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
            let _ = djvu_rs::jb2_new::decode(black_box(&sjbz), None);
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
            let mut img = djvu_rs::iw44_new::Iw44Image::new();
            let _ = img.decode_chunk(black_box(&bg44));
        });
    });
}

/// Benchmark JB2 decoding using the public domain corpus bilevel scan.
fn bench_jb2_decode_corpus(c: &mut Criterion) {
    let path = corpus_path().join("cable_1973_100133.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!(
                "skipping bench_jb2_decode_corpus: cable_1973_100133.djvu not found in tests/corpus/"
            );
            return;
        }
    };
    let sjbz = match first_sjbz_chunk(&data) {
        Some(c) => c,
        None => {
            eprintln!("skipping bench_jb2_decode_corpus: no Sjbz chunk found");
            return;
        }
    };
    c.bench_function("jb2_decode_corpus_bilevel", |b| {
        b.iter(|| {
            let _ = djvu_rs::jb2_new::decode(black_box(&sjbz), None);
        });
    });
}

/// Benchmark IW44 decoding using the public domain corpus color page.
fn bench_iw44_decode_corpus(c: &mut Criterion) {
    let path = corpus_path().join("watchmaker.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!(
                "skipping bench_iw44_decode_corpus: watchmaker.djvu not found in tests/corpus/"
            );
            return;
        }
    };
    let bg44 = match first_bg44_chunk(&data) {
        Some(c) => c,
        None => {
            eprintln!("skipping bench_iw44_decode_corpus: no BG44 chunk found");
            return;
        }
    };
    c.bench_function("iw44_decode_corpus_color", |b| {
        b.iter(|| {
            let mut img = djvu_rs::iw44_new::Iw44Image::new();
            let _ = img.decode_chunk(black_box(&bg44));
        });
    });
}

criterion_group!(
    benches,
    bench_bzz_decode,
    bench_jb2_decode,
    bench_iw44_decode,
    bench_jb2_decode_corpus,
    bench_iw44_decode_corpus
);
criterion_main!(benches);
