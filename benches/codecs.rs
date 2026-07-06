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
            let _ = djvu_rs::bzz::bzz_decode(black_box(&bzz_payload));
        });
    });
}

/// Extract the first TXTz-encoded text layer payload from a DjVu file (any
/// page, recursive search), if present.
fn first_txtz_payload(data: &[u8]) -> Option<Vec<u8>> {
    let file = djvu_rs::iff::parse(data).ok()?;
    find_chunk_legacy(file.root.children(), b"TXTz")
}

/// BZZ *encoder* benchmark — the counterpart to `bench_bzz_decode`. Encodes
/// the plaintext of a real TXTz text layer (decoded from the corpus, not
/// synthetic) so the block size and content statistics match what the BZZ
/// encoder actually sees in DjVu documents (see BZZ_ENC_DIAG in
/// PERF_EXPERIMENTS.md: real per-page TXTz plaintext is a few KB, not the
/// MAXBLOCK-scale (4 MB) inputs the suffix-sort's O(n log n) complexity
/// class matters most for).
fn bench_bzz_encode(c: &mut Criterion) {
    let path = corpus_path().join("cable_1973_100133.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_bzz_encode: cable_1973_100133.djvu not found");
            return;
        }
    };
    let txtz_payload = match first_txtz_payload(&data) {
        Some(p) => p,
        None => {
            eprintln!("skipping bench_bzz_encode: no TXTz payload found");
            return;
        }
    };
    let plaintext = match djvu_rs::bzz::bzz_decode(&txtz_payload) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("skipping bench_bzz_encode: TXTz decode failed");
            return;
        }
    };
    c.bench_function("bzz_encode", |b| {
        b.iter(|| {
            let _ = djvu_rs::bzz_encode::bzz_encode(black_box(&plaintext));
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
            let _ = djvu_rs::jb2::decode(black_box(&sjbz), None);
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
            let mut img = djvu_rs::iw44::Iw44Image::new();
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
            let _ = djvu_rs::jb2::decode(black_box(&sjbz), None);
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
            let mut img = djvu_rs::iw44::Iw44Image::new();
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
            let _ = djvu_rs::jb2::decode(black_box(&sjbz), None);
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
            let mut img = djvu_rs::iw44::Iw44Image::new();
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
    let mut img = djvu_rs::iw44::Iw44Image::new();
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

/// Benchmark: grayscale decode of a large colour page.
///
/// Compares the current gray path (`to_rgb().to_gray8()` — full YCbCr
/// reconstruct + colour conversion, then Rec.601 luma) against the direct
/// `to_gray8()` (Y-plane only, chroma planes never reconstructed). This is the
/// GRAY_DIRECT experiment: how much of gray-render decode is chroma work.
fn bench_iw44_gray_decode_large(c: &mut Criterion) {
    let path = assets_path().join("colorbook.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_iw44_gray_decode_large: colorbook.djvu missing");
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
    let chunks: Vec<Vec<u8>> = page.bg44_chunks().iter().map(|s| s.to_vec()).collect();
    if chunks.is_empty() {
        return;
    }
    let mut img = djvu_rs::iw44::Iw44Image::new();
    for chunk in &chunks {
        if img.decode_chunk(chunk).is_err() {
            return;
        }
    }

    let mut group = c.benchmark_group("iw44_gray_decode_large");
    group.bench_function("rgb_then_gray", |b| {
        b.iter(|| {
            let _ = black_box(img.to_rgb().map(|p| p.to_gray8()));
        });
    });
    group.bench_function("gray_direct", |b| {
        b.iter(|| {
            let _ = black_box(img.to_gray8());
        });
    });
    // Downscaled gray preview (thumbnail-grid case): compare the current
    // `to_rgb_subsample(s).to_gray8()` against direct `to_gray8_subsample(s)`.
    group.bench_function("rgb_then_gray_sub2", |b| {
        b.iter(|| {
            let _ = black_box(img.to_rgb_subsample(2).map(|p| p.to_gray8()));
        });
    });
    group.bench_function("gray_direct_sub2", |b| {
        b.iter(|| {
            let _ = black_box(img.to_gray8_subsample(2));
        });
    });
    group.bench_function("rgb_then_gray_sub4", |b| {
        b.iter(|| {
            let _ = black_box(img.to_rgb_subsample(4).map(|p| p.to_gray8()));
        });
    });
    group.bench_function("gray_direct_sub4", |b| {
        b.iter(|| {
            let _ = black_box(img.to_gray8_subsample(4));
        });
    });
    group.finish();
}

/// Benchmark: `to_rgb_subsample(sub)` on a pre-decoded colorbook page.
///
/// Isolates wavelet reconstruction + YCbCr→RGBA from ZP decode.
/// Uses the partial-decode path (first chunk only) to match the production path
/// for sub=4 renders.
fn bench_iw44_to_rgb_colorbook_sub(c: &mut Criterion) {
    let path = assets_path().join("colorbook.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_iw44_to_rgb_colorbook_sub: colorbook.djvu not found");
            return;
        }
    };
    let doc = match djvu_rs::DjVuDocument::parse(&data) {
        Ok(d) => d,
        Err(_) => {
            return;
        }
    };
    let page = match doc.page(0) {
        Ok(p) => p,
        Err(_) => {
            return;
        }
    };
    let chunks: Vec<Vec<u8>> = page.bg44_chunks().iter().map(|s| s.to_vec()).collect();
    if chunks.is_empty() {
        return;
    }

    // Decode only first chunk to match the partial-decode production path.
    let mut img_partial = djvu_rs::iw44::Iw44Image::new();
    if img_partial.decode_chunk(&chunks[0]).is_err() {
        return;
    }

    // Decode all chunks for the full-resolution reference.
    let mut img_full = djvu_rs::iw44::Iw44Image::new();
    for chunk in &chunks {
        if img_full.decode_chunk(chunk).is_err() {
            break;
        }
    }

    let mut group = c.benchmark_group("iw44_to_rgb_colorbook");
    group.bench_function("sub1_full_decode", |b| {
        b.iter(|| {
            let _ = black_box(img_full.to_rgb());
        });
    });
    group.bench_function("sub4_partial_decode", |b| {
        b.iter(|| {
            let _ = black_box(img_partial.to_rgb_subsample(4));
        });
    });
    group.bench_function("sub2_partial_decode", |b| {
        b.iter(|| {
            let _ = black_box(img_partial.to_rgb_subsample(2));
        });
    });
    group.finish();
}

/// Benchmark IW44 color encode: decode boy.djvu page to Pixmap, then encode back.
fn bench_iw44_encode_color(c: &mut Criterion) {
    let path = assets_path().join("boy.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_iw44_encode_color: boy.djvu not found");
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
    let opts = djvu_rs::djvu_render::RenderOptions {
        width: page.width() as u32,
        height: page.height() as u32,
        ..Default::default()
    };
    let pixmap = match djvu_rs::djvu_render::render_pixmap(page, &opts) {
        Ok(p) => p,
        Err(_) => return,
    };
    let enc_opts = djvu_rs::iw44_encode::Iw44EncodeOptions::default();
    c.bench_function("iw44_encode_color", |b| {
        b.iter(|| {
            let _ = djvu_rs::iw44_encode::encode_iw44_color(black_box(&pixmap), &enc_opts);
        });
    });
}

/// Benchmark IW44 color encode on a large synthetic 1024×1024 gradient pixmap.
///
/// The synthetic pixmap guarantees:
///   - w × h = 1 048 576 > 512 × 512, so the `parallel` feature parallel path fires.
///   - Non-trivial gradient content exercises realistic wavelet coefficients.
///
/// Compare sequential vs parallel:
///   cargo bench --bench codecs iw44_encode_large
///   cargo bench --bench codecs --features parallel iw44_encode_large
fn bench_iw44_encode_large(c: &mut Criterion) {
    const W: u32 = 1024;
    const H: u32 = 1024;
    let mut pixmap = djvu_rs::Pixmap::new(W, H, 0, 0, 0, 255);
    for y in 0..H {
        for x in 0..W {
            let r = ((x * 255) / W) as u8;
            let g = ((y * 255) / H) as u8;
            let b = (((x + y) * 127) / (W + H)) as u8;
            pixmap.set_rgb(x, y, r, g, b);
        }
    }
    let enc_opts = djvu_rs::iw44_encode::Iw44EncodeOptions::default();
    c.bench_function("iw44_encode_large_1024x1024", |b| {
        b.iter(|| {
            let _ = djvu_rs::iw44_encode::encode_iw44_color(black_box(&pixmap), &enc_opts);
        });
    });
}

/// Benchmark JB2 encode: decode a bilevel page, then encode the mask.
fn bench_jb2_encode(c: &mut Criterion) {
    let path = assets_path().join("boy_jb2.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_jb2_encode: boy_jb2.djvu not found");
            return;
        }
    };
    let sjbz = match first_sjbz_chunk(&data) {
        Some(c) => c,
        None => return,
    };
    let bitmap = match djvu_rs::jb2::decode(&sjbz, None) {
        Ok(b) => b,
        Err(_) => return,
    };
    c.bench_function("jb2_encode", |b| {
        b.iter(|| {
            let _ = djvu_rs::jb2_encode::encode_jb2(black_box(&bitmap));
        });
    });
}

/// Direct JB2 encode of a full-page (>1024 px) bilevel mask. Unlike the small
/// `jb2_encode` fixture (192×256, single tile), this exercises the multi-tile
/// path in `encode_jb2` where `crop_bitmap` splits the page into 1024×1024 tiles.
fn bench_jb2_encode_multitile(c: &mut Criterion) {
    let path = corpus_path().join("cable_1973_100133.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_jb2_encode_multitile: cable_1973_100133.djvu not found");
            return;
        }
    };
    let sjbz = match first_sjbz_chunk(&data) {
        Some(c) => c,
        None => return,
    };
    let bitmap = match djvu_rs::jb2::decode(&sjbz, None) {
        Ok(b) => b,
        Err(_) => return,
    };
    c.bench_function("jb2_encode_multitile", |b| {
        b.iter(|| {
            let _ = djvu_rs::jb2_encode::encode_jb2(black_box(&bitmap));
        });
    });
}

/// Dictionary (connected-component) JB2 encode — the colour-page mask path.
/// Unlike `jb2_encode`/`jb2_encode_multitile` (which call `encode_jb2`, the
/// direct/tiled encoder), this exercises `encode_jb2_dict` → `extract_ccs` +
/// symbol dedup + refinement matching, which had no benchmark. Uses the dense
/// text mask from cable_1973_100133 (2550×3300, many CCs).
fn bench_jb2_encode_dict(c: &mut Criterion) {
    let path = corpus_path().join("cable_1973_100133.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_jb2_encode_dict: cable_1973_100133.djvu not found");
            return;
        }
    };
    let sjbz = match first_sjbz_chunk(&data) {
        Some(c) => c,
        None => return,
    };
    let bitmap = match djvu_rs::jb2::decode(&sjbz, None) {
        Ok(b) => b,
        Err(_) => return,
    };
    c.bench_function("jb2_encode_dict", |b| {
        b.iter(|| {
            let _ = djvu_rs::jb2_encode::encode_jb2_dict(black_box(&bitmap));
        });
    });
}

/// Photometric segmentation of a colour page (`segment_page`) — runs on every
/// colour `Quality`/`Archival` encode (fixed-threshold mask + sub-sampled BG
/// block means). Had no benchmark. Decodes the colorbook BG44 to a full-res
/// Pixmap, then times `segment_page`.
fn bench_segment_page_color(c: &mut Criterion) {
    let path = assets_path().join("colorbook.djvu");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_segment_page_color: colorbook.djvu not found");
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
    let chunks: Vec<Vec<u8>> = page.bg44_chunks().iter().map(|s| s.to_vec()).collect();
    if chunks.is_empty() {
        return;
    }
    let mut img = djvu_rs::iw44::Iw44Image::new();
    for chunk in &chunks {
        if img.decode_chunk(chunk).is_err() {
            return;
        }
    }
    let pm = match img.to_rgb() {
        Ok(p) => p,
        Err(_) => return,
    };
    let opts = djvu_rs::segment::SegmentOptions::default();
    c.bench_function("segment_page_color", |b| {
        b.iter(|| {
            let _ = black_box(djvu_rs::segment::segment_page(
                black_box(&pm),
                black_box(&opts),
            ));
        });
    });
}

/// Decode up to `max` colour pages of a document to full-resolution Pixmaps
/// (via each page's BG44 → IW44 → to_rgb). Used by the colour encode workloads.
fn load_color_pixmaps(path: &std::path::Path, max: usize) -> Vec<djvu_rs::Pixmap> {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let doc = match djvu_rs::DjVuDocument::parse(&data) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for i in 0..doc.page_count().min(max) {
        let Ok(page) = doc.page(i) else { continue };
        let chunks: Vec<Vec<u8>> = page.bg44_chunks().iter().map(|s| s.to_vec()).collect();
        if chunks.is_empty() {
            continue;
        }
        let mut img = djvu_rs::iw44::Iw44Image::new();
        // Decode progressively; stop at the first chunk that fails (some corpus
        // pages have a truncated trailing refinement chunk). A partial decode is
        // a perfectly representative colour source for an *encoder* benchmark.
        let mut any = false;
        for ch in &chunks {
            if img.decode_chunk(ch).is_err() {
                break;
            }
            any = true;
        }
        if !any {
            continue;
        }
        if let Ok(pm) = img.to_rgb() {
            out.push(pm);
        }
    }
    out
}

/// Full single-page colour encode (`PageEncoder::from_pixmap().encode()`) —
/// the end-to-end `Quality` path: segment_page + encode_jb2_dict + encode_iw44_color
/// + foreground_fgbz. Previously only the individual stages were benched.
fn bench_encode_color_page_quality(c: &mut Criterion) {
    let pms = load_color_pixmaps(&assets_path().join("colorbook.djvu"), 1);
    let Some(pm) = pms.into_iter().next() else {
        eprintln!("skipping bench_encode_color_page_quality: no colour page");
        return;
    };
    c.bench_function("encode_color_page_quality", |b| {
        b.iter(|| {
            let _ = black_box(
                djvu_rs::djvu_encode::PageEncoder::from_pixmap(black_box(&pm))
                    .with_quality(djvu_rs::djvu_encode::EncodeQuality::Quality)
                    .encode(),
            );
        });
    });
}

/// Full single-page colour encode on the largest available page
/// (`big-scanned-page.djvu`, 6780×9148). Exercises the same Quality path as
/// `bench_encode_color_page_quality` but with a JB2/IW44 split large enough in
/// absolute terms for the PAR_PAGE_LAYERS `rayon::join` (Sjbz ∥ BG44) overlap to
/// clear the noise floor — the BG-heavier fixture the round-2 revert asked for.
fn bench_encode_color_page_quality_large(c: &mut Criterion) {
    let pms = load_color_pixmaps(&assets_path().join("big-scanned-page.djvu"), 1);
    let Some(pm) = pms.into_iter().next() else {
        eprintln!("skipping bench_encode_color_page_quality_large: no colour page");
        return;
    };
    let mut group = c.benchmark_group("encode_large");
    group.sample_size(10);
    group.bench_function("encode_color_page_quality_large", |b| {
        b.iter(|| {
            let _ = black_box(
                djvu_rs::djvu_encode::PageEncoder::from_pixmap(black_box(&pm))
                    .with_quality(djvu_rs::djvu_encode::EncodeQuality::Quality)
                    .encode(),
            );
        });
    });
    group.finish();
}

/// Multi-page layered colour bundle (`encode_djvm_layered_shared`) — segments +
/// JB2-dict + IW44 + FGbz for every page, then shared-Djbz clustering + DJVM
/// container assembly. First (≤3) colour pages of watchmaker. No prior bench.
fn bench_encode_djvm_layered_shared(c: &mut Criterion) {
    let pms = load_color_pixmaps(&corpus_path().join("watchmaker.djvu"), 3);
    if pms.len() < 2 {
        eprintln!("skipping bench_encode_djvm_layered_shared: need ≥2 colour pages");
        return;
    }
    // Heavy multi-page workload: reduce sample count to keep the run bounded.
    let mut group = c.benchmark_group("encode_multipage");
    group.sample_size(10);
    group.bench_function("encode_djvm_layered_shared", |b| {
        b.iter(|| {
            let _ = black_box(djvu_rs::djvu_encode::encode_djvm_layered_shared(
                black_box(&pms),
                djvu_rs::djvu_encode::EncodeQuality::Quality,
                300,
                None,
                2,
            ));
        });
    });
    group.finish();
}

/// Multi-page bilevel bundle (`encode_djvm_bundle_jb2`) — per-page JB2-dict encode
/// plus shared-symbol clustering plus DJVM container assembly (DIRM + IFF emit).
/// Uses the first six page masks of conquete_paix; exercises the
/// `assemble_djvm_bundle` two-pass emit and `cluster_shared_symbols`.
fn bench_encode_djvm_bundle_jb2(c: &mut Criterion) {
    let data = match std::fs::read(corpus_path().join("conquete_paix.djvu")) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping bench_encode_djvm_bundle_jb2: conquete_paix.djvu not found");
            return;
        }
    };
    let doc = match djvu_rs::DjVuDocument::parse(&data) {
        Ok(d) => d,
        Err(_) => return,
    };
    let mut masks = Vec::new();
    for i in 0..doc.page_count().min(6) {
        let Ok(page) = doc.page(i) else { continue };
        if let Ok(Some(bm)) = page.extract_mask() {
            masks.push(bm);
        }
    }
    if masks.len() < 2 {
        eprintln!("skipping bench_encode_djvm_bundle_jb2: need ≥2 masks");
        return;
    }
    // Heavy multi-page workload: reduce sample count to keep the run bounded.
    let mut group = c.benchmark_group("encode_multipage");
    group.sample_size(10);
    group.bench_function("encode_djvm_bundle_jb2", |b| {
        b.iter(|| {
            let _ = black_box(djvu_rs::jb2_encode::encode_djvm_bundle_jb2(
                black_box(&masks),
                2,
                300,
            ));
        });
    });
    group.finish();
}

/// Shared-symbol clustering (`cluster_shared_symbols`) in isolation — the
/// per-document dictionary builder for layered multi-page bilevel encode. Loads
/// every page mask of the 517-page `pathogenic_bacteria_1896` (a dense text
/// scan with large same-size symbol buckets) and times only the clustering. The
/// bucketing scan had no dedicated bench; the surrounding `encode_djvm_bundle_jb2`
/// bench amortizes it into the full per-page encode.
fn bench_cluster_shared_symbols(c: &mut Criterion) {
    let data = match std::fs::read(corpus_path().join("pathogenic_bacteria_1896.djvu")) {
        Ok(d) => d,
        Err(_) => {
            eprintln!(
                "skipping bench_cluster_shared_symbols: pathogenic_bacteria_1896.djvu not found"
            );
            return;
        }
    };
    let doc = match djvu_rs::DjVuDocument::parse(&data) {
        Ok(d) => d,
        Err(_) => return,
    };
    let mut masks = Vec::new();
    for i in 0..doc.page_count() {
        let Ok(page) = doc.page(i) else { continue };
        if let Ok(Some(bm)) = page.extract_mask() {
            masks.push(bm);
        }
    }
    if masks.len() < 2 {
        eprintln!("skipping bench_cluster_shared_symbols: need ≥2 masks");
        return;
    }
    let mut group = c.benchmark_group("encode_multipage");
    group.sample_size(10);
    group.bench_function("cluster_shared_symbols_517p", |b| {
        b.iter(|| {
            let _ = black_box(djvu_rs::jb2_encode::cluster_shared_symbols(
                black_box(&masks),
                2,
            ));
        });
    });
    group.finish();
}

/// IW44 grayscale encode (`encode_iw44_gray`) — the TH44 thumbnail codec path.
/// Synthetic 1024×1024 gradient GrayPixmap. Previously no gray-path bench.
fn bench_iw44_encode_gray(c: &mut Criterion) {
    let (w, h) = (1024u32, 1024u32);
    let data: Vec<u8> = (0..w * h).map(|i| (i % 256) as u8).collect();
    let gray = djvu_rs::GrayPixmap {
        width: w,
        height: h,
        data,
    };
    let opts = djvu_rs::iw44_encode::Iw44EncodeOptions::default();
    c.bench_function("iw44_encode_gray_1024x1024", |b| {
        b.iter(|| {
            let _ = black_box(djvu_rs::iw44_encode::encode_iw44_gray(
                black_box(&gray),
                black_box(&opts),
            ));
        });
    });
}

criterion_group!(
    benches,
    bench_bzz_decode,
    bench_bzz_encode,
    bench_jb2_decode,
    bench_iw44_decode,
    bench_jb2_decode_corpus,
    bench_iw44_decode_corpus,
    bench_jb2_decode_large,
    bench_iw44_decode_large_all_chunks,
    bench_iw44_to_rgb_large,
    bench_iw44_gray_decode_large,
    bench_iw44_to_rgb_colorbook_sub,
    bench_iw44_encode_color,
    bench_iw44_encode_large,
    bench_jb2_encode,
    bench_jb2_encode_multitile,
    bench_jb2_encode_dict,
    bench_segment_page_color,
    bench_encode_color_page_quality,
    bench_encode_color_page_quality_large,
    bench_encode_djvm_layered_shared,
    bench_encode_djvm_bundle_jb2,
    bench_cluster_shared_symbols,
    bench_iw44_encode_gray,
);
criterion_main!(benches);
