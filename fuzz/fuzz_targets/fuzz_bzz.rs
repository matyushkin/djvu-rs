#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = djvu_rs::bzz_new::bzz_decode(data);
});
