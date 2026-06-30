#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut img = djvu_rs::iw44::Iw44Image::new();
    let _ = img.decode_chunk(data);
});
