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

// ── TIFF ingestion slice 2 (#694) ────────────────────────────────────────────

/// Library-level TIFF normalization tests over hand-crafted fixtures. The
/// builder writes minimal classic little-endian TIFFs so it can produce
/// layouts the `tiff` crate encoder cannot (bilevel, palette, CMYK,
/// multipage, multi-strip).
#[cfg(feature = "tiff")]
mod tiff_slice2 {
    use djvu_rs::ingest::IngestPolicy;
    use djvu_rs::png_io::{
        decode_tiff_file_to_pixmap, decode_tiff_file_to_pixmap_with_policy,
        decode_tiff_file_to_pixmaps,
    };
    use std::path::Path;

    pub struct TiffPage {
        pub width: u32,
        pub height: u32,
        /// Per-sample bit depth (length = samples per pixel).
        pub bits: Vec<u16>,
        /// 0 WhiteIsZero, 1 BlackIsZero, 2 RGB, 3 Palette, 5 CMYK.
        pub photometric: u16,
        /// Packed strip data, one strip per `rows_per_strip` rows.
        pub data: Vec<u8>,
        pub rows_per_strip: Option<u32>,
        pub colormap: Option<Vec<u16>>,
        /// Compression tag value (1 = none). Data is written as-is.
        pub compression: u16,
    }

    impl TiffPage {
        pub fn gray(width: u32, height: u32, bits: u16, photometric: u16, data: Vec<u8>) -> Self {
            TiffPage {
                width,
                height,
                bits: vec![bits],
                photometric,
                data,
                rows_per_strip: None,
                colormap: None,
                compression: 1,
            }
        }
    }

    /// Serialize pages as a classic little-endian TIFF.
    pub fn build_tiff(pages: &[TiffPage]) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"II");
        out.extend_from_slice(&42u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // first-IFD offset, patched below

        // Strip data first. Multi-strip pages split rows evenly by
        // rows_per_strip; `data` holds the concatenated strips.
        let mut data_offsets = Vec::new();
        for p in pages {
            data_offsets.push(out.len() as u32);
            out.extend_from_slice(&p.data);
            if out.len() % 2 == 1 {
                out.push(0);
            }
        }

        let mut ifd_offsets = Vec::new();
        for (i, p) in pages.iter().enumerate() {
            let samples = p.bits.len() as u32;
            let rows_per_strip = p.rows_per_strip.unwrap_or(p.height).max(1);
            let strip_count = p.height.div_ceil(rows_per_strip).max(1) as usize;
            let row_bytes = (p.width as usize * p.bits[0] as usize).div_ceil(8);

            // Per-strip offsets/byte counts (out-of-line when > 1 strip).
            let mut strip_offsets = Vec::with_capacity(strip_count);
            let mut strip_counts = Vec::with_capacity(strip_count);
            let mut cursor = data_offsets[i];
            for s in 0..strip_count {
                let rows = (p.height as usize)
                    .saturating_sub(s * rows_per_strip as usize)
                    .min(rows_per_strip as usize);
                let len = (rows * row_bytes) as u32;
                strip_offsets.push(cursor);
                strip_counts.push(len);
                cursor += len;
            }

            let write_u16s = |out: &mut Vec<u8>, vals: &[u16]| -> u32 {
                let off = out.len() as u32;
                for v in vals {
                    out.extend_from_slice(&v.to_le_bytes());
                }
                off
            };
            let bits_offset = if p.bits.len() > 2 {
                write_u16s(&mut out, &p.bits)
            } else {
                0
            };
            let colormap_offset = p.colormap.as_ref().map(|cm| write_u16s(&mut out, cm));
            let write_u32s = |out: &mut Vec<u8>, vals: &[u32]| -> u32 {
                let off = out.len() as u32;
                for v in vals {
                    out.extend_from_slice(&v.to_le_bytes());
                }
                off
            };
            let (offsets_value, counts_value) = if strip_count > 1 {
                (
                    write_u32s(&mut out, &strip_offsets),
                    write_u32s(&mut out, &strip_counts),
                )
            } else {
                (strip_offsets[0], strip_counts[0])
            };
            if out.len() % 2 == 1 {
                out.push(0);
            }

            ifd_offsets.push(out.len() as u32);
            // (tag, type, count, value) — type 3 = SHORT, 4 = LONG.
            let mut entries: Vec<(u16, u16, u32, u32)> = vec![
                (256, 4, 1, p.width),
                (257, 4, 1, p.height),
                (259, 3, 1, p.compression as u32),
                (262, 3, 1, p.photometric as u32),
                (273, 4, strip_count as u32, offsets_value),
                (277, 3, 1, samples),
                (278, 4, 1, rows_per_strip),
                (279, 4, strip_count as u32, counts_value),
            ];
            entries.push(match p.bits.len() {
                1 => (258, 3, 1, p.bits[0] as u32),
                2 => (258, 3, 2, (p.bits[0] as u32) | ((p.bits[1] as u32) << 16)),
                _ => (258, 3, samples, bits_offset),
            });
            if let (Some(cm), Some(off)) = (&p.colormap, colormap_offset) {
                entries.push((320, 3, cm.len() as u32, off));
            }
            entries.sort_by_key(|e| e.0);

            out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
            for (tag, typ, count, value) in &entries {
                out.extend_from_slice(&tag.to_le_bytes());
                out.extend_from_slice(&typ.to_le_bytes());
                out.extend_from_slice(&count.to_le_bytes());
                out.extend_from_slice(&value.to_le_bytes());
            }
            out.extend_from_slice(&0u32.to_le_bytes()); // next-IFD, patched below
        }

        out[4..8].copy_from_slice(&ifd_offsets[0].to_le_bytes());
        for w in ifd_offsets.windows(2) {
            let ifd = w[0] as usize;
            let entry_count = u16::from_le_bytes([out[ifd], out[ifd + 1]]) as usize;
            let next_ptr = ifd + 2 + entry_count * 12;
            out[next_ptr..next_ptr + 4].copy_from_slice(&w[1].to_le_bytes());
        }
        out
    }

    pub fn write_tiff(path: &Path, pages: &[TiffPage]) {
        std::fs::write(path, build_tiff(pages)).unwrap();
    }

    #[test]
    fn bilevel_black_is_zero_expands_to_full_range() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("g1.tif");
        // 8x2: rows 0b10101010 and 0b11001100.
        write_tiff(
            &path,
            &[TiffPage::gray(8, 2, 1, 1, vec![0b1010_1010, 0b1100_1100])],
        );
        let pm = decode_tiff_file_to_pixmap(&path).unwrap();
        assert_eq!((pm.width, pm.height), (8, 2));
        let grays: Vec<u8> = pm.data.chunks_exact(4).map(|px| px[0]).collect();
        assert_eq!(
            grays,
            vec![
                255, 0, 255, 0, 255, 0, 255, 0, 255, 255, 0, 0, 255, 255, 0, 0
            ]
        );
        assert!(pm.data.chunks_exact(4).all(|px| px[3] == 255));
    }

    #[test]
    fn bilevel_white_is_zero_inverts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("g1w.tif");
        write_tiff(&path, &[TiffPage::gray(8, 1, 1, 0, vec![0b1010_1010])]);
        let pm = decode_tiff_file_to_pixmap(&path).unwrap();
        let grays: Vec<u8> = pm.data.chunks_exact(4).map(|px| px[0]).collect();
        assert_eq!(grays, vec![0, 255, 0, 255, 0, 255, 0, 255]);
    }

    #[test]
    fn bilevel_multi_strip_rows_concatenate() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("strips.tif");
        let mut page = TiffPage::gray(8, 4, 1, 1, vec![0xFF, 0x00, 0xFF, 0x00]);
        page.rows_per_strip = Some(1);
        write_tiff(&path, &[page]);
        let pm = decode_tiff_file_to_pixmap(&path).unwrap();
        assert_eq!((pm.width, pm.height), (8, 4));
        let row = |r: usize| pm.data[r * 8 * 4];
        assert_eq!([row(0), row(1), row(2), row(3)], [255, 0, 255, 0]);
    }

    #[test]
    fn four_bit_gray_scales_linearly() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("g4.tif");
        // 2x1 nibbles: 0xF, 0x5 → 255, 85.
        write_tiff(&path, &[TiffPage::gray(2, 1, 4, 1, vec![0xF5])]);
        let pm = decode_tiff_file_to_pixmap(&path).unwrap();
        assert_eq!(pm.data[0], 255);
        assert_eq!(pm.data[4], 85);
    }

    #[test]
    fn sixteen_bit_gray_truncates_high_byte() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("g16.tif");
        // LE samples 0x1234, 0xABCD → high bytes 0x12, 0xAB.
        write_tiff(
            &path,
            &[TiffPage::gray(2, 1, 16, 1, vec![0x34, 0x12, 0xCD, 0xAB])],
        );
        let pm = decode_tiff_file_to_pixmap_with_policy(&path, IngestPolicy::default()).unwrap();
        assert_eq!(pm.data[0], 0x12);
        assert_eq!(pm.data[4], 0xAB);
    }

    #[test]
    fn sixteen_bit_rgb_truncates_each_channel() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rgb16.tif");
        let page = TiffPage {
            width: 1,
            height: 1,
            bits: vec![16, 16, 16],
            photometric: 2,
            data: vec![0x34, 0x12, 0x00, 0x00, 0xCD, 0xAB],
            rows_per_strip: None,
            colormap: None,
            compression: 1,
        };
        write_tiff(&path, &[page]);
        let pm = decode_tiff_file_to_pixmap(&path).unwrap();
        assert_eq!(pm.data, vec![0x12, 0x00, 0xAB, 255]);
    }

    #[test]
    fn cmyk_converts_through_documented_transform() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cmyk.tif");
        let page = TiffPage {
            width: 3,
            height: 1,
            bits: vec![8, 8, 8, 8],
            photometric: 5,
            // pure cyan, pure black, paper white
            data: vec![255, 0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 0],
            rows_per_strip: None,
            colormap: None,
            compression: 1,
        };
        write_tiff(&path, &[page]);
        let pm = decode_tiff_file_to_pixmap(&path).unwrap();
        assert_eq!(&pm.data[0..4], &[0, 255, 255, 255], "pure cyan");
        assert_eq!(&pm.data[4..8], &[0, 0, 0, 255], "pure black");
        assert_eq!(&pm.data[8..12], &[255, 255, 255, 255], "paper white");
    }

    #[test]
    fn palette_maps_through_colormap_high_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pal8.tif");
        let mut cm = vec![0u16; 3 * 256];
        cm[0] = 0xFF00; // R of entry 0
        cm[256 + 1] = 0x8000; // G of entry 1
        cm[2 * 256 + 1] = 0x1234; // B of entry 1
        let page = TiffPage {
            width: 2,
            height: 1,
            bits: vec![8],
            photometric: 3,
            data: vec![0, 1],
            rows_per_strip: None,
            colormap: Some(cm),
            compression: 1,
        };
        write_tiff(&path, &[page]);
        let pm = decode_tiff_file_to_pixmap(&path).unwrap();
        assert_eq!(&pm.data[0..4], &[0xFF, 0, 0, 255]);
        assert_eq!(&pm.data[4..8], &[0, 0x80, 0x12, 255]);
    }

    #[test]
    fn four_bit_palette_unpacks_indices() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pal4.tif");
        let mut cm = vec![0u16; 3 * 16];
        cm[1] = 0xAA00; // R of entry 1
        cm[16 + 2] = 0xBB00; // G of entry 2
        let page = TiffPage {
            width: 2,
            height: 1,
            bits: vec![4],
            photometric: 3,
            data: vec![0x12], // indices 1, 2
            rows_per_strip: None,
            colormap: Some(cm),
            compression: 1,
        };
        write_tiff(&path, &[page]);
        let pm = decode_tiff_file_to_pixmap(&path).unwrap();
        assert_eq!(&pm.data[0..4], &[0xAA, 0, 0, 255]);
        assert_eq!(&pm.data[4..8], &[0, 0xBB, 0, 255]);
    }

    #[test]
    fn multipage_returns_all_pages_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("book.tif");
        write_tiff(
            &path,
            &[
                TiffPage::gray(2, 1, 8, 1, vec![10, 20]),
                TiffPage::gray(1, 2, 8, 1, vec![30, 40]),
            ],
        );
        let pages = decode_tiff_file_to_pixmaps(&path, IngestPolicy::default()).unwrap();
        assert_eq!(pages.len(), 2);
        assert_eq!((pages[0].width, pages[0].height), (2, 1));
        assert_eq!((pages[1].width, pages[1].height), (1, 2));
        assert_eq!(pages[1].data[0], 30);
        // Single-page API keeps returning the first page only.
        let first = decode_tiff_file_to_pixmap(&path).unwrap();
        assert_eq!(first.data[0], 10);
    }

    #[test]
    fn compressed_bilevel_reports_targeted_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("g4comp.tif");
        let mut page = TiffPage::gray(8, 1, 1, 1, vec![0xAA]);
        page.compression = 4; // CCITT G4
        write_tiff(&path, &[page]);
        let err = decode_tiff_file_to_pixmap(&path).unwrap_err().to_string();
        assert!(err.contains("CCITT G4"), "unexpected error: {err}");
        assert!(err.contains("not supported"), "unexpected error: {err}");
    }
}
