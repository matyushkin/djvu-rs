#![no_main]
//! `smmr::encode_g4` (full T.6: pass/vertical/horizontal modes) round-trip
//! fuzzing (#567).
//!
//! `fuzz_encode` exercises only `encode_smmr` (H-mode). `encode_g4` is newer
//! (PDF_G4, round 53) and has a cautionary history: its predecessor carried a
//! symmetric encoder/decoder `find_b1` bug that survived every internal
//! round-trip and was only caught by external decoders. This target derives a
//! structured bitmap from fuzz input, G4-encodes it, and decodes it back
//! through the crate's own T.6 decoder (`decode_smmr`, header prepended),
//! asserting pixel-exact reconstruction.

use djvu_rs::smmr::{decode_smmr, encode_g4};
use djvu_rs::Bitmap;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    // Bounded dimensions keep each iteration cheap; G4 is row-oriented so
    // wide-and-short and narrow-and-tall shapes both matter.
    let w = u16::from_le_bytes([data[0], data[1]]) as u32 % 200 + 1;
    let h = u16::from_le_bytes([data[2], data[3]]) as u32 % 200 + 1;
    let body = &data[4..];
    if body.is_empty() {
        return;
    }

    let mut bm = Bitmap::new(w, h);
    let area = (w * h) as usize;
    for i in 0..area {
        let byte = body[(i / 8) % body.len()];
        if (byte >> (i % 8)) & 1 == 1 {
            bm.set_black((i as u32) % w, (i as u32) / w);
        }
    }

    let g4 = encode_g4(&bm);

    // `encode_g4` emits the header-less CCITTFaxDecode payload; the crate's
    // own decoder consumes an Smmr chunk = 4-byte ncols/nrows header + G4 bits.
    let mut chunk = Vec::with_capacity(4 + g4.len());
    chunk.extend_from_slice(&(w as u16).to_be_bytes());
    chunk.extend_from_slice(&(h as u16).to_be_bytes());
    chunk.extend_from_slice(&g4);

    let dec = decode_smmr(&chunk).expect("g4: undecodable encoder output");
    assert_eq!((dec.width, dec.height), (w, h), "g4 dimension mismatch");
    for y in 0..h {
        for x in 0..w {
            assert_eq!(bm.get(x, y), dec.get(x, y), "g4 pixel mismatch at ({x},{y})");
        }
    }
});
