#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(doc) = djvu_rs::DjVuDocument::parse(data) {
        for i in 0..doc.page_count() {
            if let Ok(page) = doc.page(i) {
                let _ = page.thumbnail();
                let _ = page.text_layer();
                let _ = page.annotations();
            }
        }
    }
});
