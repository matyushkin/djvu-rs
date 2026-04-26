//! Property-based round-trip tests (issue #195).
//!
//! Generates randomised inputs and checks the encode/decode invariants:
//! * `decode(encode(B)) == B` for `Bitmap` (JB2, both encoders)
//! * `bzz_decode(bzz_encode(d)) == d` for arbitrary byte slices
//!
//! Default `proptest` budget is 256 cases per test, suitable for CI.

use djvu_rs::Bitmap;
use djvu_rs::{bzz_encode, bzz_new, jb2, jb2_encode};
use proptest::prelude::*;

/// Strategy for a bilevel `Bitmap` of arbitrary dimensions and content.
fn arb_bitmap(max_w: u32, max_h: u32) -> impl Strategy<Value = Bitmap> {
    (1u32..=max_w, 1u32..=max_h).prop_flat_map(|(w, h)| {
        let total_bits = (w * h) as usize;
        prop::collection::vec(any::<bool>(), total_bits).prop_map(move |bits| {
            let mut bm = Bitmap::new(w, h);
            for (i, &b) in bits.iter().enumerate() {
                let x = (i as u32) % w;
                let y = (i as u32) / w;
                if b {
                    bm.set_black(x, y);
                }
            }
            bm
        })
    })
}

fn bitmaps_eq(src: &Bitmap, decoded: &Bitmap) -> Result<(), TestCaseError> {
    prop_assert_eq!(src.width, decoded.width);
    prop_assert_eq!(src.height, decoded.height);
    for y in 0..src.height {
        for x in 0..src.width {
            prop_assert_eq!(
                src.get(x, y),
                decoded.get(x, y),
                "mismatch at ({}, {})",
                x,
                y
            );
        }
    }
    Ok(())
}

proptest! {
    // Cap: 64 cases × max 64×64 bitmap keeps total run time well under 1 s on
    // CI. The encoder is O(w·h) plus arithmetic coding overhead; this is plenty
    // to expose context-update / boundary bugs without dominating CI time.
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// JB2 direct encoder (single record-3 / tiled): bit-exact round-trip for
    /// any randomly-generated bitmap.
    #[test]
    fn jb2_direct_roundtrip(bm in arb_bitmap(64, 64)) {
        let bytes = jb2_encode::encode_jb2(&bm);
        let decoded = jb2::decode(&bytes, None).expect("decode failed");
        bitmaps_eq(&bm, &decoded)?;
    }

    /// JB2 dict encoder (CC + record types 1+7): bit-exact round-trip.
    #[test]
    fn jb2_dict_roundtrip(bm in arb_bitmap(64, 64)) {
        let bytes = jb2_encode::encode_jb2_dict(&bm);
        let decoded = jb2::decode(&bytes, None).expect("decode failed");
        bitmaps_eq(&bm, &decoded)?;
    }

    /// BZZ: bit-exact round-trip on arbitrary byte slices.
    #[test]
    fn bzz_roundtrip(data in prop::collection::vec(any::<u8>(), 0..4096)) {
        let encoded = bzz_encode::bzz_encode(&data);
        let decoded = bzz_new::decode(&encoded).expect("BZZ decode failed");
        prop_assert_eq!(data, decoded);
    }
}

proptest! {
    // Larger image → fewer cases; specifically aimed at the >1 MP tile path
    // (#198) and at edge-tile sizes that aren't multiples of 1024.
    #![proptest_config(ProptestConfig::with_cases(4))]

    #[test]
    fn jb2_direct_tiled_roundtrip(bm in arb_bitmap(1500, 1100)) {
        let bytes = jb2_encode::encode_jb2(&bm);
        let decoded = jb2::decode(&bytes, None).expect("decode failed");
        bitmaps_eq(&bm, &decoded)?;
    }
}
