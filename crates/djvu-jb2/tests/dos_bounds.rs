//! Regression: a sub-1 KB Sjbz of matched-refinement records used to decode
//! ~48 MP of symbols (~0.6 s native, a libFuzzer `fuzz_jb2` timeout under ASAN)
//! before the per-page symbol-pixel cap. It must now be rejected by that budget
//! (`ImageTooLarge`) rather than spinning to the looser truncation guard.
//! See PERF_EXPERIMENTS.md (JB2 page symbol-pixel cap).

/// The exact libFuzzer `fuzz_jb2` timeout artifact.
const PAGE_CAP_TIMEOUT: &[u8] = include_bytes!("fixtures/page_cap_timeout.bin");

#[test]
fn page_cap_bounds_refinement_amplification() {
    // The per-page cap fires at 32 MP, well before the ~48 MP this input would
    // otherwise reach — so the error is `ImageTooLarge`, not the later `Truncated`.
    // Asserting the specific kind proves the cap is the active bound on the work.
    match djvu_jb2::decode(PAGE_CAP_TIMEOUT, None) {
        Err(djvu_jb2::Jb2Error::ImageTooLarge) => {}
        other => panic!("expected Err(ImageTooLarge) from the page cap, got {other:?}"),
    }
}
