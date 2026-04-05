#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(doc) = djvu_rs::DjVuDocument::parse(data) else {
        return;
    };
    for i in 0..doc.page_count() {
        let Ok(page) = doc.page(i) else { continue };
        let _ = page.thumbnail();
        let _ = page.text_layer();
        let _ = page.annotations();
        // Exercise the render pipeline — must never panic, only return Err
        let opts = djvu_rs::djvu_render::RenderOptions {
            dpi: 72.0,
            ..Default::default()
        };
        let _ = djvu_rs::djvu_render::render_pixmap(page, &opts);
    }
});
