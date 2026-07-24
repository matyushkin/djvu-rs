//! Integration tests for encoder ingestion slice 1 (#694).

use assert_cmd::Command;
use djvu_rs::djvu_render::RenderOptions;
use djvu_rs::png_io::decode_png_to_pixmap;
use djvu_rs::quality;
use std::path::Path;

const ICCP_CHUNK: &[u8] = &[
    0x00, 0x00, 0x00, 0x3c, 0x69, 0x43, 0x43, 0x50, 0x69, 0x67, 0x6e, 0x6f, 0x72, 0x65, 0x64, 0x00,
    0x00, 0x78, 0x9c, 0x0d, 0xc8, 0xc1, 0x0d, 0x00, 0x20, 0x08, 0x04, 0xb0, 0x55, 0x6e, 0x35, 0x25,
    0x60, 0x88, 0x44, 0x08, 0xf2, 0x61, 0x7b, 0xed, 0xb3, 0x32, 0x36, 0x43, 0x89, 0x10, 0xe9, 0xa2,
    0xc6, 0x98, 0x5d, 0x7c, 0x21, 0x9e, 0xd0, 0xb3, 0xf8, 0x16, 0xc2, 0x4d, 0xa9, 0xf1, 0xb7, 0x1e,
    0x7d, 0x9a, 0x10, 0xe0, 0xc6, 0x08, 0xf6, 0x1c,
];

fn write_png(
    path: &Path,
    width: u32,
    height: u32,
    color: png::ColorType,
    depth: png::BitDepth,
    pixels: &[u8],
) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(color);
    encoder.set_depth(depth);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(pixels).unwrap();
}

fn write_indexed_png(path: &Path, width: u32, height: u32, indices: &[u8]) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Indexed);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_palette(vec![255, 255, 255, 0, 0, 0, 20, 40, 180]);
    encoder.set_trns(vec![255, 128, 255]);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(indices).unwrap();
}

fn write_sixteen_bit_gray_checker(path: &Path, width: u32, height: u32) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Sixteen);
    let mut writer = encoder.write_header().unwrap();
    let mut pixels = Vec::with_capacity((width * height * 2) as usize);
    for y in 0..height {
        for x in 0..width {
            let v = if (x + y) % 2 == 0 {
                0xFFFFu16
            } else {
                0x0000u16
            };
            pixels.extend_from_slice(&[(v >> 8) as u8, v as u8]);
        }
    }
    writer.write_image_data(&pixels).unwrap();
}

fn inject_iccp_after_ihdr(png: &[u8]) -> Vec<u8> {
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let mut pos = 8usize;
    while pos + 12 <= png.len() {
        let len = u32::from_be_bytes(png[pos..pos + 4].try_into().unwrap()) as usize;
        let typ = &png[pos + 4..pos + 8];
        let chunk_end = pos + 12 + len;
        if typ == b"IHDR" {
            let mut out = Vec::new();
            out.extend_from_slice(&png[..chunk_end]);
            out.extend_from_slice(ICCP_CHUNK);
            out.extend_from_slice(&png[chunk_end..]);
            return out;
        }
        pos = chunk_end;
    }
    panic!("IHDR chunk not found");
}

fn encode_lossless(input: &Path, output: &Path) {
    Command::cargo_bin("djvu")
        .unwrap()
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--quality",
            "lossless",
            "--dpi",
            "100",
        ])
        .assert()
        .success();
}

fn render_first_page(path: &Path) -> djvu_rs::Pixmap {
    let bytes = std::fs::read(path).unwrap();
    let doc = djvu_rs::Document::from_bytes(bytes).unwrap();
    let page = doc.page(0).unwrap();
    let opts = RenderOptions {
        width: page.width(),
        height: page.height(),
        ..Default::default()
    };
    page.render_with(&opts).unwrap()
}

#[test]
fn icc_profile_does_not_change_decoded_pixels() {
    let dir = tempfile::tempdir().unwrap();
    let plain = dir.path().join("plain.png");
    write_png(
        &plain,
        2,
        1,
        png::ColorType::Rgb,
        png::BitDepth::Eight,
        &[255, 0, 0, 0, 255, 0],
    );
    let with_icc = dir.path().join("with_icc.png");
    let bytes = std::fs::read(&plain).unwrap();
    std::fs::write(&with_icc, inject_iccp_after_ihdr(&bytes)).unwrap();

    let plain_pm = decode_png_to_pixmap(&plain).unwrap();
    let icc_pm = decode_png_to_pixmap(&with_icc).unwrap();
    assert_eq!(plain_pm.data, icc_pm.data);
}

#[test]
fn encode_one_bit_png_creates_valid_djvu() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("1bit.png");
    write_png(
        &input,
        8,
        8,
        png::ColorType::Grayscale,
        png::BitDepth::One,
        &[0b10101010; 8],
    );
    let output = dir.path().join("out.djvu");
    encode_lossless(&input, &output);
    assert_eq!(&std::fs::read(&output).unwrap()[..4], b"AT&T");
}

#[test]
fn encode_two_bit_png_creates_valid_djvu() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("2bit.png");
    write_png(
        &input,
        4,
        1,
        png::ColorType::Grayscale,
        png::BitDepth::Two,
        &[0b11100100],
    );
    let output = dir.path().join("out.djvu");
    encode_lossless(&input, &output);
    assert_eq!(&std::fs::read(&output).unwrap()[..4], b"AT&T");
}

#[test]
fn encode_sixteen_bit_png_creates_valid_djvu() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("16bit.png");
    write_sixteen_bit_gray_checker(&input, 2, 1);
    let output = dir.path().join("out.djvu");
    encode_lossless(&input, &output);
    assert_eq!(&std::fs::read(&output).unwrap()[..4], b"AT&T");
}

#[test]
fn encode_indexed_png_renders_with_acceptable_quality() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("indexed.png");
    write_indexed_png(&input, 32, 32, &vec![0u8; 32 * 32]);
    let source = decode_png_to_pixmap(&input).unwrap();
    let output = dir.path().join("out.djvu");
    encode_lossless(&input, &output);
    let rendered = render_first_page(&output);
    let report = quality::compare(&source, &rendered);
    assert!(
        report.ssim >= 0.95,
        "indexed PNG round-trip SSIM too low: {}",
        report.ssim
    );
}

#[test]
fn encode_sixteen_bit_png_renders_with_acceptable_quality() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("16bit.png");
    write_sixteen_bit_gray_checker(&input, 32, 32);
    let source = decode_png_to_pixmap(&input).unwrap();
    let output = dir.path().join("out.djvu");
    encode_lossless(&input, &output);
    let rendered = render_first_page(&output);
    let report = quality::compare(&source, &rendered);
    assert!(
        report.ssim >= 0.95,
        "16-bit PNG round-trip SSIM too low: {}",
        report.ssim
    );
}

#[test]
fn alpha_channel_preserved_through_png_ingest() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("alpha.png");
    write_png(
        &input,
        1,
        1,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        &[10, 20, 30, 128],
    );
    let pm = decode_png_to_pixmap(&input).unwrap();
    assert_eq!(pm.data, vec![10, 20, 30, 128]);
}
