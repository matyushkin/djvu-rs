//! Deterministic instruction-count baselines for codec hot paths.
//!
//! Run on Linux with `cargo bench --bench instructions`; iai-callgrind invokes
//! Valgrind/Callgrind and reports instruction/cache events rather than elapsed
//! wall time.

use std::hint::black_box;
use std::path::PathBuf;
use std::sync::OnceLock;

use iai_callgrind::{library_benchmark, library_benchmark_group, main};

fn assets_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/djvujs/library/assets")
}

fn find_chunk(chunks: &[djvu_rs::iff::Chunk], target: &[u8; 4]) -> Option<Vec<u8>> {
    for chunk in chunks {
        match chunk {
            djvu_rs::iff::Chunk::Leaf { id, data } if id == target => return Some(data.clone()),
            djvu_rs::iff::Chunk::Form { children, .. } => {
                if let Some(data) = find_chunk(children, target) {
                    return Some(data);
                }
            }
            _ => {}
        }
    }
    None
}

fn chunk(file: &str, id: &[u8; 4]) -> Vec<u8> {
    let data = std::fs::read(assets_path().join(file)).expect("instruction fixture must exist");
    let parsed = djvu_rs::iff::parse(&data).expect("instruction fixture must parse");
    find_chunk(parsed.root.children(), id).expect("instruction fixture must contain chunk")
}

fn bzz_payload() -> &'static Vec<u8> {
    static DATA: OnceLock<Vec<u8>> = OnceLock::new();
    DATA.get_or_init(|| {
        let bytes = std::fs::read(assets_path().join("navm_fgbz.djvu")).unwrap();
        let form = djvu_rs::iff::parse_form(&bytes).unwrap();
        form.chunks.iter().find(|c| &c.id == b"DIRM").unwrap().data[1..].to_vec()
    })
}

fn sjbz_payload() -> &'static Vec<u8> {
    static DATA: OnceLock<Vec<u8>> = OnceLock::new();
    DATA.get_or_init(|| chunk("boy_jb2.djvu", b"Sjbz"))
}

fn bg44_payload() -> &'static Vec<u8> {
    static DATA: OnceLock<Vec<u8>> = OnceLock::new();
    DATA.get_or_init(|| chunk("boy.djvu", b"BG44"))
}

fn colorbook_iw44() -> &'static djvu_rs::iw44::Iw44Image {
    static IMAGE: OnceLock<djvu_rs::iw44::Iw44Image> = OnceLock::new();
    IMAGE.get_or_init(|| {
        let bytes = std::fs::read(assets_path().join("colorbook.djvu")).unwrap();
        let doc = djvu_rs::DjVuDocument::parse(&bytes).unwrap();
        let page = doc.page(0).unwrap();
        let mut image = djvu_rs::iw44::Iw44Image::new();
        for chunk in page.bg44_chunks() {
            image.decode_chunk(chunk).unwrap();
        }
        image
    })
}

fn colorbook_document() -> &'static djvu_rs::DjVuDocument {
    static DOCUMENT: OnceLock<djvu_rs::DjVuDocument> = OnceLock::new();
    DOCUMENT.get_or_init(|| {
        let bytes = std::fs::read(assets_path().join("colorbook.djvu")).unwrap();
        djvu_rs::DjVuDocument::parse(&bytes).unwrap()
    })
}

#[library_benchmark]
fn bzz_decode() {
    black_box(djvu_rs::bzz::bzz_decode(black_box(bzz_payload())).unwrap());
}

#[library_benchmark]
fn jb2_decode() {
    black_box(djvu_rs::jb2::decode(black_box(sjbz_payload()), None).unwrap());
}

#[library_benchmark]
fn iw44_decode_first_chunk() {
    let mut image = djvu_rs::iw44::Iw44Image::new();
    black_box(image.decode_chunk(black_box(bg44_payload())).unwrap());
}

#[library_benchmark]
fn iw44_to_rgb_colorbook() {
    black_box(colorbook_iw44().to_rgb().unwrap());
}

#[library_benchmark]
fn render_compositor_native() {
    let page = colorbook_document().page(0).unwrap();
    let options = djvu_rs::djvu_render::RenderOptions::fit_to_width(page, u32::from(page.width()));
    black_box(djvu_rs::djvu_render::render_pixmap(page, &options).unwrap());
}

#[library_benchmark]
fn render_lanczos_downscale() {
    let page = colorbook_document().page(0).unwrap();
    let mut options =
        djvu_rs::djvu_render::RenderOptions::fit_to_width(page, u32::from(page.width() / 2));
    options.resampling = djvu_rs::djvu_render::Resampling::Lanczos3;
    black_box(djvu_rs::djvu_render::render_pixmap(page, &options).unwrap());
}

library_benchmark_group!(name = instruction_counts; benchmarks = bzz_decode, jb2_decode, iw44_decode_first_chunk, iw44_to_rgb_colorbook, render_compositor_native, render_lanczos_downscale);
main!(library_benchmark_groups = instruction_counts);
