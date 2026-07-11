#![no_main]
//! BZZ *encoder* round-trip fuzzing (#567).
//!
//! `bzz_decode` has its own target (`fuzz_bzz`, arbitrary bytes → decoder must
//! not panic), but the encoder — whose output feeds every TXTz/ANTz/NAVM/DIRM
//! chunk this project writes — had no fuzz coverage at all. Arbitrary input
//! must compress and decompress back bit-exactly.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let enc = djvu_rs::bzz_encode::bzz_encode(data);
    let dec = djvu_rs::bzz::bzz_decode(&enc).expect("bzz: undecodable encoder output");
    assert_eq!(dec, data, "bzz round-trip mismatch");
});
