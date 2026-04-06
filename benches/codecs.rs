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
fn find_chunk_legacy(chunks: &[djvu_rs::iff::Chunk], target: &[u8; 4]) -> Option<Vec<u8>> {
    for chunk in chunks {
        match chunk {
            djvu_rs::iff::Chunk::Leaf { id, data } if id == target => {
                return Some(data.clone());
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

/// Benchmark: JB2 decode for pathogenic_bacteria_1896.djvu page 0 (large 600 dpi bilevel scan).
/// This isolates the ZP arithmetic + symbol decode cost for the large page.
fn bench_jb2_decode_large(c: &mut Criterion) {
    let path = corpus_path().join("pathogenic_bacteria_1896.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_jb2_decode_large: pathogenic_bacteria_1896.djvu not found");
            return;
        }
    };
    let sjbz = match first_sjbz_chunk(&data) {
        Some(c) => c,
        None => {
            eprintln!("skipping bench_jb2_decode_large: no Sjbz chunk found");
            return;
        }
    };
    eprintln!("bench_jb2_decode_large: Sjbz chunk = {} bytes", sjbz.len());
    c.bench_function("jb2_decode_large_600dpi", |b| {
        b.iter(|| {
            let _ = djvu_rs::jb2_new::decode(black_box(&sjbz), None);
        });
    });
}

/// Benchmark: decode ALL BG44 chunks for pathogenic_bacteria_1896.djvu page 0
/// (large mixed-content page, 600 dpi).  This isolates the ZP arithmetic decode
/// cost without any wavelet reconstruction or colour conversion.
fn bench_iw44_decode_large_all_chunks(c: &mut Criterion) {
    let path = corpus_path().join("pathogenic_bacteria_1896.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!(
                "skipping bench_iw44_decode_large_all_chunks: pathogenic_bacteria_1896.djvu not found"
            );
            return;
        }
    };
    let doc = match djvu_rs::DjVuDocument::parse(&data) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_iw44_decode_large_all_chunks: parse failed");
            return;
        }
    };
    let page = match doc.page(0) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("skipping bench_iw44_decode_large_all_chunks: page 0 not found");
            return;
        }
    };
    let chunks: Vec<Vec<u8>> = page.bg44_chunks().iter().map(|s| s.to_vec()).collect();
    if chunks.is_empty() {
        eprintln!("skipping bench_iw44_decode_large_all_chunks: no BG44 chunks");
        return;
    }
    eprintln!(
        "bench_iw44_decode_large_all_chunks: {} BG44 chunks, total {} bytes",
        chunks.len(),
        chunks.iter().map(|c| c.len()).sum::<usize>()
    );

    c.bench_function("iw44_decode_large_all_chunks", |b| {
        b.iter(|| {
            let mut img = djvu_rs::iw44_new::Iw44Image::new();
            for chunk in &chunks {
                let _ = img.decode_chunk(black_box(chunk));
            }
        });
    });
}

/// Benchmark: `to_rgb()` on a pre-decoded large page — isolates wavelet
/// reconstruction + colour conversion from ZP arithmetic decode.
fn bench_iw44_to_rgb_large(c: &mut Criterion) {
    let path = corpus_path().join("pathogenic_bacteria_1896.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_iw44_to_rgb_large: pathogenic_bacteria_1896.djvu not found");
            return;
        }
    };
    let doc = match djvu_rs::DjVuDocument::parse(&data) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_iw44_to_rgb_large: parse failed");
            return;
        }
    };
    let page = match doc.page(0) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("skipping bench_iw44_to_rgb_large: page 0 not found");
            return;
        }
    };
    let chunks: Vec<Vec<u8>> = page.bg44_chunks().iter().map(|s| s.to_vec()).collect();
    if chunks.is_empty() {
        eprintln!("skipping bench_iw44_to_rgb_large: no BG44 chunks");
        return;
    }

    // Pre-decode once; benchmark only to_rgb().
    let mut img = djvu_rs::iw44_new::Iw44Image::new();
    for chunk in &chunks {
        if img.decode_chunk(chunk).is_err() {
            eprintln!("skipping bench_iw44_to_rgb_large: decode_chunk failed");
            return;
        }
    }

    c.bench_function("iw44_to_rgb_large_page", |b| {
        b.iter(|| {
            let _ = black_box(img.to_rgb());
        });
    });
}

criterion_group!(
    benches,
    bench_bzz_decode,
    bench_jb2_decode,
    bench_iw44_decode,
    bench_jb2_decode_corpus,
    bench_iw44_decode_corpus,
    bench_jb2_decode_large,
    bench_iw44_decode_large_all_chunks,
    bench_iw44_to_rgb_large,
);
criterion_main!(benches);
