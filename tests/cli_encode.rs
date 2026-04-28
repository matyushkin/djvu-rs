//! TDD tests for `djvu encode <input>`.

use assert_cmd::Command;
use std::path::Path;

/// Write a minimal grayscale PNG with a checkerboard pattern, half black half white.
fn write_test_png(path: &Path, w: u32, h: u32) {
    let file = std::fs::File::create(path).unwrap();
    let writer = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, w, h);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    let mut data = Vec::with_capacity((w * h) as usize);
    for _y in 0..h {
        for x in 0..w {
            // Half black, half white split by x
            data.push(if x < w / 2 { 0 } else { 255 });
        }
    }
    writer.write_image_data(&data).unwrap();
}

#[test]
fn encode_creates_djvu_file() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.png");
    let output = dir.path().join("out.djvu");
    write_test_png(&input, 64, 48);

    Command::cargo_bin("djvu")
        .unwrap()
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--dpi",
            "150",
        ])
        .assert()
        .success();

    assert!(output.exists(), "djvu output not created");
    let bytes = std::fs::read(&output).unwrap();
    assert_eq!(&bytes[..4], b"AT&T", "output is not a valid DjVu IFF file");
}

#[test]
fn encode_default_dpi_is_300() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.png");
    let output = dir.path().join("out.djvu");
    write_test_png(&input, 32, 32);

    Command::cargo_bin("djvu")
        .unwrap()
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let bytes = std::fs::read(&output).unwrap();
    let doc = djvu_rs::Document::from_bytes(bytes).unwrap();
    let page = doc.page(0).unwrap();
    assert_eq!(page.dpi(), 300);
    assert_eq!(page.width(), 32);
    assert_eq!(page.height(), 32);
}

#[test]
fn encode_quality_profile_unsupported_until_segmentation() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.png");
    let output = dir.path().join("out.djvu");
    write_test_png(&input, 16, 16);

    Command::cargo_bin("djvu")
        .unwrap()
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--quality",
            "quality",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("Quality profile"));
}

#[test]
fn encode_missing_input_fails() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("nope.png");
    let output = dir.path().join("out.djvu");

    Command::cargo_bin("djvu")
        .unwrap()
        .args([
            "encode",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure();
}
