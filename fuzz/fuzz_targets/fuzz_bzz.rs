#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = cos_djvu::bzz_new::bzz_decode(data);
});
