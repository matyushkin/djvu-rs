#![no_main]
//! Encoder fuzzing: the decoders have dedicated targets, but the *encoders*
//! (which this project actively tunes for size) were only covered by bounded
//! proptest. This target derives a small `Bitmap` / `Pixmap` from arbitrary
//! input and asserts the encoders never panic and that lossless codecs
//! round-trip bit-exactly; IW44 (lossy) must at least produce a stream that
//! decodes back to the same dimensions.

use djvu_rs::iw44_encode::{Iw44EncodeOptions, encode_iw44_color};
use djvu_rs::iw44::Iw44Image;
use djvu_rs::smmr::{decode_smmr, encode_smmr};
use djvu_rs::{Bitmap, Pixmap, jb2, jb2_encode};
use libfuzzer_sys::fuzz_target;

fn assert_bitmap_eq(a: &Bitmap, b: &Bitmap) {
    assert_eq!((a.width, a.height), (b.width, b.height), "dimension mismatch");
    for y in 0..a.height {
        for x in 0..a.width {
            assert_eq!(a.get(x, y), b.get(x, y), "pixel mismatch at ({x},{y})");
        }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    // Bounded dimensions (1..=100) keep each iteration cheap; the proptest suite
    // covers the larger tile path. Area is capped as a second guard.
    let w = u16::from_le_bytes([data[0], data[1]]) as u32 % 100 + 1;
    let h = u16::from_le_bytes([data[2], data[3]]) as u32 % 100 + 1;
    let area = (w * h) as usize;
    let body = &data[4..];
    if body.is_empty() {
        return;
    }

    // ---- bilevel encoders: lossless → must round-trip bit-exactly ----
    let mut bm = Bitmap::new(w, h);
    for i in 0..area {
        let byte = body[(i / 8) % body.len()];
        if (byte >> (i % 8)) & 1 == 1 {
            bm.set_black((i as u32) % w, (i as u32) / w);
        }
    }

    let enc = jb2_encode::encode_jb2(&bm);
    let dec = jb2::decode(&enc, None).expect("jb2 direct: undecodable encoder output");
    assert_bitmap_eq(&bm, &dec);

    let enc = jb2_encode::encode_jb2_dict(&bm);
    let dec = jb2::decode(&enc, None).expect("jb2 dict: undecodable encoder output");
    assert_bitmap_eq(&bm, &dec);

    let enc = encode_smmr(&bm);
    let dec = decode_smmr(&enc).expect("smmr: undecodable encoder output");
    assert_bitmap_eq(&bm, &dec);

    // ---- IW44 colour encoder: lossy → dimensional round-trip only ----
    let mut pm = Pixmap::white(w, h);
    for i in 0..area {
        let r = body[(i * 3) % body.len()];
        let g = body[(i * 3 + 1) % body.len()];
        let b = body[(i * 3 + 2) % body.len()];
        pm.set_rgb((i as u32) % w, (i as u32) / w, r, g, b);
    }
    let chunks = encode_iw44_color(&pm, &Iw44EncodeOptions::default());
    let mut img = Iw44Image::new();
    for c in &chunks {
        img.decode_chunk(c).expect("iw44: undecodable encoder chunk");
    }
    let out = img.to_rgb().expect("iw44: to_rgb failed");
    assert_eq!((out.width, out.height), (w, h), "iw44 dimension mismatch");
});
