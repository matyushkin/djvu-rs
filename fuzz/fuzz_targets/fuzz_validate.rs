#![no_main]
use libfuzzer_sys::fuzz_target;

// #696: the validator and semantic diff are diagnostic surfaces that must
// accept arbitrary hostile bytes without panicking, hanging, or overflowing —
// they are exactly what a server would run on untrusted uploads. The validator
// runs both the cheap-probe and the decode_pages configurations; the semantic
// diff self-compares, which exercises every plane extractor.
fuzz_target!(|data: &[u8]| {
    let cheap = djvu_rs::validate::validate(data, &djvu_rs::validate::ValidateOptions::default());
    let _ = cheap.is_valid();
    let deep = djvu_rs::validate::validate(
        data,
        &djvu_rs::validate::ValidateOptions {
            strict: true,
            decode_pages: true,
        },
    );
    let _ = deep.summary();
    let _ = djvu_rs::validate::validate_planned_output(data);

    if let Ok(diff) = djvu_rs::semantic_diff::semantic_diff(data, data, None) {
        // A document compared with itself must never diverge.
        assert!(diff.is_identical());
    }
});
