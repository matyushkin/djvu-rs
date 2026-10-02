use super::*;

/// Regression test: a fuzzer-discovered 11-byte input triggered two DoS
/// paths simultaneously: the ZP-exhausted record loop spinning up to
/// MAX_RECORDS times, and a near-4MP symbol decode.
#[test]
fn huge_symbol_from_small_input_does_not_hang() {
    let data = &[
        0x7f, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    let start = std::time::Instant::now();
    let _ = decode(data, None);
    // In release the full decode is <100 ms; in debug the unoptimised loop
    // is ~10× slower, so we allow 8 s (still well under the 10 s fuzz
    // CI timeout that motivated this fix).
    let limit_secs = if cfg!(debug_assertions) { 8 } else { 2 };
    assert!(
        start.elapsed().as_secs() < limit_secs,
        "took {:?}",
        start.elapsed()
    );
}

/// Regression test for the 2026-05-03 `fuzz_jb2` timeout on main. The
/// 6-byte stream exhausts ZP input, then asks for a large refinement symbol.
#[test]
fn exhausted_refinement_symbol_from_small_input_does_not_hang() {
    let data = &[0x2a, 0xce, 0x7d, 0x24, 0x01, 0x00];
    let start = std::time::Instant::now();
    assert!(matches!(decode(data, None), Err(Jb2Error::Truncated)));
    let limit_secs = if cfg!(debug_assertions) { 2 } else { 1 };
    assert!(
        start.elapsed().as_secs() < limit_secs,
        "took {:?}",
        start.elapsed()
    );
}

/// Regression test for the follow-up `fuzz_jb2` timeout where the
/// post-EOF stream repeatedly emits small-but-expensive refinement symbols.
#[test]
fn exhausted_repeated_refinement_symbols_do_not_hang() {
    let data = &[0x2a, 0xce, 0xf1, 0xce, 0xf1, 0x88, 0x52, 0x82, 0xf7, 0xf7];
    let start = std::time::Instant::now();
    assert!(matches!(decode(data, None), Err(Jb2Error::Truncated)));
    let limit_secs = if cfg!(debug_assertions) { 2 } else { 1 };
    assert!(
        start.elapsed().as_secs() < limit_secs,
        "took {:?}",
        start.elapsed()
    );
}
