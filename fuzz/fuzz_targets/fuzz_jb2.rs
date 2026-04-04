#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = djvu_rs::jb2_new::decode(data, None);
});
