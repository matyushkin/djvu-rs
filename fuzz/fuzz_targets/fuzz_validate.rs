#![no_main]
use libfuzzer_sys::fuzz_target;

// #696: the validator and semantic diff are diagnostic surfaces that must
// accept arbitrary hostile bytes without panicking, hanging, or overflowing —
// they are exactly what a server would run on untrusted uploads. The validator
// runs the cheap-probe, decode_pages, and resource-limit configurations; the
// semantic diff self-compares, which exercises every plane extractor.
fuzz_target!(|data: &[u8]| {
    let cheap = djvu_rs::validate::validate(data, &djvu_rs::validate::ValidateOptions::default());
    let _ = cheap.is_valid();
    let deep = djvu_rs::validate::validate(
        data,
        &djvu_rs::validate::ValidateOptions {
            strict: true,
            decode_pages: true,
            limits: None,
        },
    );
    let _ = deep.summary();
    let _ = deep.resources;

    // Tight limits drive the resource layer over hostile headers and force the
    // pre-decode skip path (decode_pages requested but a decode-cost limit is
    // exceeded), which must stay panic-free and estimate without overflow.
    let limited = djvu_rs::validate::validate(
        data,
        &djvu_rs::validate::ValidateOptions {
            strict: true,
            decode_pages: true,
            limits: Some(djvu_rs::validate::ResourceLimits {
                max_file_bytes: Some(0),
                max_pages: Some(0),
                max_components: Some(0),
                max_page_pixels: Some(0),
                max_total_pixels: Some(0),
                max_decoded_bytes: Some(0),
                max_render_pixels: Some(0),
            }),
        },
    );
    let _ = limited.summary();

    let _ = djvu_rs::validate::validate_planned_output(data);

    if let Ok(diff) = djvu_rs::semantic_diff::semantic_diff(data, data, None) {
        // A document compared with itself must never diverge.
        assert!(diff.is_identical());
    }
});
