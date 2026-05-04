window.BENCHMARK_DATA = {
  "lastUpdate": 1777865017382,
  "repoUrl": "https://github.com/matyushkin/djvu-rs",
  "entries": {
    "djvu-rs benchmarks": [
      {
        "commit": {
          "author": {
            "name": "Leo Matyushkin",
            "username": "matyushkin",
            "email": "leva.matyushkin@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "3b875b409d1d14ca4738b426b191544e94c1876a",
          "message": "ci(fuzz): skip cargo install when cargo-fuzz binary cache hits (#264)\n\nThe Cache cargo-fuzz binary step restores ~/.cargo/bin/cargo-fuzz from\na previous run, then `cargo install cargo-fuzz --locked` fails with\n`error: binary cargo-fuzz already exists in destination`. Gate the\ninstall on a cache-miss via `cache-hit` output.\n\nCo-authored-by: Claude Opus 4.7 <noreply@anthropic.com>",
          "timestamp": "2026-04-30T12:24:41Z",
          "url": "https://github.com/matyushkin/djvu-rs/commit/3b875b409d1d14ca4738b426b191544e94c1876a"
        },
        "date": 1777552513181,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 117,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 158976,
            "range": "± 837",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 774261,
            "range": "± 4254",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 563669,
            "range": "± 1762",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1374650,
            "range": "± 15267",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2676,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9453213,
            "range": "± 144467",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 587589,
            "range": "± 8303",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2252110,
            "range": "± 23397",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2779019,
            "range": "± 10906",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 27568773,
            "range": "± 700508",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 227450,
            "range": "± 614",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 353912,
            "range": "± 2176",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1579961,
            "range": "± 24114",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6079203,
            "range": "± 20186",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 23614985,
            "range": "± 647887",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1688721,
            "range": "± 10626",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 13051691,
            "range": "± 365174",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13133353,
            "range": "± 39426",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/bg_only_warm",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/mask_decode",
            "value": 5303517,
            "range": "± 178260",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28430531,
            "range": "± 461171",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 135089521,
            "range": "± 533905",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 134074109,
            "range": "± 2201813",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 218927,
            "range": "± 853",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8463932,
            "range": "± 54186",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1441146511,
            "range": "± 5638221",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4079238,
            "range": "± 175423",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3324,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 21678091,
            "range": "± 193548",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 21619479,
            "range": "± 644697",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3154919,
            "range": "± 6267",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22925161,
            "range": "± 45170",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 197789,
            "range": "± 6156",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8218999,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "leva.matyushkin@gmail.com",
            "name": "Leo Matyushkin",
            "username": "matyushkin"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b3906813dbe4d4c9946c93b9b9e6884c1da62efc",
          "message": "fix(iw44): correct vext lane in prelim_flags_band0_neon horizontal-OR (#266)\n\nThe 8→4-byte fold in prelim_flags_band0_neon read undefined `v1`\nand used `vext_u8::<1>` instead of `vext_u8::<2>`, breaking the\naarch64 build. The sibling helper prelim_flags_bucket_neon\n(line 1064) shows the canonical pattern.\n\nCI only runs on ubuntu-latest x86_64 so the regression went\nunnoticed on PR #261.\n\nCo-authored-by: Claude Opus 4.7 <noreply@anthropic.com>",
          "timestamp": "2026-05-01T19:20:56+09:00",
          "tree_id": "3764df12292f8e3290b4170a8953df8c1e7d6705",
          "url": "https://github.com/matyushkin/djvu-rs/commit/b3906813dbe4d4c9946c93b9b9e6884c1da62efc"
        },
        "date": 1777631450002,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 82,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 129438,
            "range": "± 1628",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 649355,
            "range": "± 1389",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 368498,
            "range": "± 9617",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 970319,
            "range": "± 10072",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 1980,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 7794887,
            "range": "± 51341",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 482617,
            "range": "± 15096",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 1865263,
            "range": "± 17757",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2235290,
            "range": "± 9800",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 21980212,
            "range": "± 104557",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 174244,
            "range": "± 1518",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 274655,
            "range": "± 2337",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1330504,
            "range": "± 15164",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 5130088,
            "range": "± 12055",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 19876427,
            "range": "± 57113",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1405874,
            "range": "± 9620",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10121753,
            "range": "± 191390",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10133412,
            "range": "± 40300",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/bg_only_warm",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/mask_decode",
            "value": 4601237,
            "range": "± 97101",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 22917656,
            "range": "± 270849",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 109805898,
            "range": "± 124473",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 109012507,
            "range": "± 197269",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 172186,
            "range": "± 293",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 6734782,
            "range": "± 30923",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1143264039,
            "range": "± 3069445",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4076119,
            "range": "± 87706",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2705,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 18848236,
            "range": "± 101208",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 18823007,
            "range": "± 80276",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 2673181,
            "range": "± 3167",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 19026108,
            "range": "± 84926",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 100383,
            "range": "± 1280",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 6753000,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "leva.matyushkin@gmail.com",
            "name": "Leo Matyushkin",
            "username": "matyushkin"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "eec08153575052d81f71eb5382176816f1592aff",
          "message": "feat(api): high-level setters for DjVuDocumentMut (PR2 of #222) (#267)\n\n* feat(api): high-level setters for DjVuDocumentMut (PR2 of #222)\n\nPR2 of #222 builds on PR1's chunk-replacement primitive and exposes\nhigh-level setters that compose `replace_leaf` with the existing chunk\nencoders.\n\n## New surface\n\n- `DjVuDocumentMut::page_count() -> usize`\n- `DjVuDocumentMut::page_mut(i) -> Result<PageMut<'_>, MutError>`\n- `PageMut::set_text_layer(&TextLayer)` — emits TXTz (replaces TXTa/TXTz)\n- `PageMut::set_annotations(&Annotation, &[MapArea])` — emits ANTz\n- `PageMut::set_metadata(&DjVuMetadata)` — emits METz; empty input\n  removes the existing chunk\n- `metadata::encode_metadata` / `encode_metadata_bzz` — new public\n  encoders, round-trip tested against `parse_metadata`/`parse_metadata_bzz`\n- New `MutError` variants: `PageOutOfRange`, `MissingPageInfo`,\n  `InfoParse(IffError)`, `DjvmMutationUnsupported`\n\n## Scope\n\n`page_mut` errors with `DjvmMutationUnsupported` on multi-page\n`FORM:DJVM` bundles: changing a page's chunk size shifts the per-component\noffsets in DIRM, which needs its own recomputation pass. Deferred to\nPR3 of the #222 sequence (along with `set_bookmarks` for NAVM at the\nbundle root). Single-page `FORM:DJVU` works fully.\n\n## Test plan\n\n- [x] `cargo test --release --lib` — 410 passed (402 → 410: +9\n      djvu_mut, +5 metadata)\n- [x] Round-trip tests parse re-emitted bytes and decode each chunk\n      back to the input value\n- [x] Empty/replace/remove paths covered explicitly\n- [x] `page_mut` error paths (out-of-range, DJVM bundle)\n- [x] `cargo clippy --workspace --lib --tests --bins -- -D warnings` clean\n- [x] `cargo fmt --check` clean\n\nCLAUDE.md / PERF_EXPERIMENTS.md updated with `### #222 PR2 — Kept (2026-05-01)`.\n\nCo-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>\n\n* fix(metadata): gate encode_metadata_bzz on feature=std\n\nbzz_encode (the encoder) is std-only; the new encode_metadata_bzz\nhelper transitively required std but was unconditionally pub. CI's\nno_std and wasm32 builds failed with E0433 \"cannot find bzz_encode\nin crate\". Gate the function and its test on feature = \"std\", matching\nthe existing precedent in src/annotation.rs:532.\n\n---------\n\nCo-authored-by: Claude Opus 4.7 <noreply@anthropic.com>",
          "timestamp": "2026-05-01T19:34:10+09:00",
          "tree_id": "9adf51d0ccd44795eae0cce34da078e62914c4eb",
          "url": "https://github.com/matyushkin/djvu-rs/commit/eec08153575052d81f71eb5382176816f1592aff"
        },
        "date": 1777632239539,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 118,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 147263,
            "range": "± 257",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 839055,
            "range": "± 6360",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 580149,
            "range": "± 2802",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1534837,
            "range": "± 30695",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 3155,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9246133,
            "range": "± 140464",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 589081,
            "range": "± 5261",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2210216,
            "range": "± 14601",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2769256,
            "range": "± 3550",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 30062610,
            "range": "± 450745",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 212729,
            "range": "± 1676",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 343415,
            "range": "± 615",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1308694,
            "range": "± 35181",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 5016641,
            "range": "± 33494",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 19573804,
            "range": "± 160915",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1576770,
            "range": "± 7834",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 12498509,
            "range": "± 99523",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 12429913,
            "range": "± 87764",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/bg_only_warm",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/mask_decode",
            "value": 5127865,
            "range": "± 6914",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 29961307,
            "range": "± 1279356",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 122325199,
            "range": "± 844174",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 121061081,
            "range": "± 347923",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 208976,
            "range": "± 808",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 9232954,
            "range": "± 51471",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1300174260,
            "range": "± 5469584",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 6813147,
            "range": "± 154438",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2413,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 26171199,
            "range": "± 463801",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 26789616,
            "range": "± 314300",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3140372,
            "range": "± 3743",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22884194,
            "range": "± 135343",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 218656,
            "range": "± 2069",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 7331000,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "leva.matyushkin@gmail.com",
            "name": "Leo Matyushkin",
            "username": "matyushkin"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6672a936202f66b32e0e77dafdf467503547de6e",
          "message": "feat(api): bundled DJVM mutation + set_bookmarks (PR3 of #222) (#268)\n\nPR3 lifts the `DjvmMutationUnsupported` block from PR2: `page_mut` now\nworks on bundled `FORM:DJVM` documents, and `into_bytes` recomputes the\nDIRM offset table so per-component byte positions stay correct after\nany chunk-size change.\n\nAdds `DjVuDocumentMut::set_bookmarks(&[DjVuBookmark])` for inserting,\nreplacing, or removing the bundle's NAVM chunk. Empty input removes,\nnon-empty input emits a fresh BZZ-compressed NAVM via `encode_navm` and\nplaces it immediately after DIRM (the canonical location).\n\n`MutError::DjvmMutationUnsupported` is replaced by\n`IndirectDjvmUnsupported` (deferred to PR5) plus structural variants\n`DirmMalformed` and `DirmComponentCountMismatch` for the recomputation\npath. `into_bytes` stays infallible (panicking on inconsistencies that\na successful `from_bytes` would already have rejected); `try_into_bytes`\nis added for callers that want the error.\n\nSingle-page `FORM:DJVU` `set_bookmarks` calls return\n`BookmarksRequireDjvm` — NAVM lives in DJVM bundles only per spec.\n\nTests cover: DIRM offsets matching actual FORM positions before and\nafter page edits, mid-page edits leaving prior offsets unchanged,\nNAVM round-trip via `parse_navm_bookmarks`, NAVM removal/insertion\nordering, and end-to-end parse via `DjVuDocument::parse` of mutated\nbundles.\n\nCo-authored-by: Claude Opus 4.7 <noreply@anthropic.com>",
          "timestamp": "2026-05-01T19:55:12+09:00",
          "tree_id": "60d8a1987b7b71fb440172029d5ddd867939a8a8",
          "url": "https://github.com/matyushkin/djvu-rs/commit/6672a936202f66b32e0e77dafdf467503547de6e"
        },
        "date": 1777633538855,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 117,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 158513,
            "range": "± 844",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 773437,
            "range": "± 4564",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 573558,
            "range": "± 2060",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1325039,
            "range": "± 15023",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2808,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9538319,
            "range": "± 63550",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 589332,
            "range": "± 1328",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2246657,
            "range": "± 23256",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2756377,
            "range": "± 7307",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 28629971,
            "range": "± 572529",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 243286,
            "range": "± 1631",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 351457,
            "range": "± 1003",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1515055,
            "range": "± 12276",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 5791012,
            "range": "± 44851",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 22514830,
            "range": "± 94950",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1693123,
            "range": "± 10749",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 13246555,
            "range": "± 98544",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13304976,
            "range": "± 108242",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/bg_only_warm",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/mask_decode",
            "value": 5304842,
            "range": "± 24470",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 29117187,
            "range": "± 403960",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 136890884,
            "range": "± 171206",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 136251630,
            "range": "± 434145",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 215155,
            "range": "± 523",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8448275,
            "range": "± 132351",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1437922694,
            "range": "± 4927475",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4095737,
            "range": "± 60136",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3331,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 22091176,
            "range": "± 194563",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 22085640,
            "range": "± 244658",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3160847,
            "range": "± 10275",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 23062193,
            "range": "± 110651",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 201705,
            "range": "± 1279",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8166000,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "leva.matyushkin@gmail.com",
            "name": "Leo Matyushkin",
            "username": "matyushkin"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "df1e95e51c08ebe4e85cb1899352aae0cf5a5692",
          "message": "fix(iff): preserve FORM length parity for byte-identical mutation (PR4 of #222) (#269)\n\nTwo valid IFF layouts exist when a FORM's last child has odd payload\nlength: declare FORM length odd and let the parent loop write the pad\nbyte, or declare even and include the pad inside the FORM body. Real\nDjVu files mix both styles inconsistently — the bundled DjVu3 spec\nfixture has 78 pages of one style and 5 pages of the other.\n\nPreviously `iff::emit` always inlined the pad (even-style), which\nshifted the FORM length-LSB by 1 on those 5 pages after any mutation.\nThat broke the PR4 byte-identical guarantee for unmutated pages.\n\nSwitch the legacy emitter to honor the parser's stored length parity:\nsuppress the trailing internal pad on the last child iff the original\nFORM length was odd. The outer pad still fires unconditionally so the\nparent's child loop sees correct alignment.\n\nAlso adds `unmutated_pages_byte_identical_after_metadata_edit` which\ncatches future regressions on the bundled fixture.\n\nCo-authored-by: Claude Opus 4.7 <noreply@anthropic.com>",
          "timestamp": "2026-05-01T20:43:12+09:00",
          "tree_id": "dba66838a75e4e33a8f3af255a74b2175499a1ae",
          "url": "https://github.com/matyushkin/djvu-rs/commit/df1e95e51c08ebe4e85cb1899352aae0cf5a5692"
        },
        "date": 1777636418696,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 117,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 157749,
            "range": "± 1321",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 764696,
            "range": "± 2252",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 570762,
            "range": "± 19941",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1309986,
            "range": "± 45975",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2696,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9360158,
            "range": "± 194982",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 596199,
            "range": "± 8821",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2261917,
            "range": "± 50805",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2745090,
            "range": "± 5039",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 28472649,
            "range": "± 486771",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 243058,
            "range": "± 893",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 350135,
            "range": "± 14554",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1515615,
            "range": "± 7611",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 5783109,
            "range": "± 23052",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 22464963,
            "range": "± 116395",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1687964,
            "range": "± 25033",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 13205527,
            "range": "± 87216",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13271178,
            "range": "± 38112",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/bg_only_warm",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/mask_decode",
            "value": 5319785,
            "range": "± 17204",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28471496,
            "range": "± 177432",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 137331476,
            "range": "± 330534",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 136803529,
            "range": "± 1621450",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 220176,
            "range": "± 2335",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8448921,
            "range": "± 38460",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1443500882,
            "range": "± 5696809",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4198028,
            "range": "± 61479",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3307,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 22092520,
            "range": "± 148649",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 22439484,
            "range": "± 361690",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3199539,
            "range": "± 23569",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 23520902,
            "range": "± 582076",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 197632,
            "range": "± 1252",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8286000,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "leva.matyushkin@gmail.com",
            "name": "Leo Matyushkin",
            "username": "matyushkin"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "78e5c793d3d7a86512e42820d8e3cf0c9870c48e",
          "message": "fix(jb2-enc): cap cluster_shared_symbols pixel budget at decoder limit (#270) (#271)\n\n* fix(jb2-enc): cap cluster_shared_symbols pixel budget at decoder limit (#270)\n\n`encode_djvm_bundle_jb2(corpus_pages, 2)` on the 517-page\n`pathogenic_bacteria_1896.djvu` corpus produced an undecodable bundle:\nthe shared `Djbz` totalled ~78 MP of symbols, exceeding the decoder's\n`MAX_TOTAL_SYMBOL_PIXELS = 64 MP` per-stream budget. `decode_dictionary`\nreturned `Jb2Error::ImageTooLarge`, `decoded_shared_dict()` swallowed it\nto `None`, and downstream pages then surfaced `MissingSharedDict`.\n\nTrim the cluster output at promotion time so cumulative symbol pixels\nstay under the decoder's budget, prioritising reps seen on more pages\n(higher byte-savings yield) and breaking ties toward smaller pixel cost.\nOn the affected corpus this drops 63 149 → 59 141 symbols (~6 %) and\ngrows the bundle by ~0.7 % (29.79 → 30.02 MB) — a worthwhile cost for\nencoder output that always round-trips.\n\nAlso expose `MAX_TOTAL_SYMBOL_PIXELS` as `pub(crate)` so the encoder\nreferences the decoder's authoritative cap rather than duplicating the\nvalue.\n\nCo-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>\n\n* style(jb2-enc): cargo fmt fix on #270 regression test\n\nCo-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>\n\n---------\n\nCo-authored-by: Claude Opus 4.7 <noreply@anthropic.com>",
          "timestamp": "2026-05-03T22:13:04+09:00",
          "tree_id": "2992bd8579e9ec6e3cfc056d2cd19c1133d81f55",
          "url": "https://github.com/matyushkin/djvu-rs/commit/78e5c793d3d7a86512e42820d8e3cf0c9870c48e"
        },
        "date": 1777814601836,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 117,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 157705,
            "range": "± 722",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 771739,
            "range": "± 5941",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 572293,
            "range": "± 1887",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1387637,
            "range": "± 48402",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2644,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9627370,
            "range": "± 31772",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 592242,
            "range": "± 1830",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2278748,
            "range": "± 10189",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2768786,
            "range": "± 4245",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 28846035,
            "range": "± 196898",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 228609,
            "range": "± 429",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 341988,
            "range": "± 1228",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1603573,
            "range": "± 8575",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6153706,
            "range": "± 18598",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 24169426,
            "range": "± 212543",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1698178,
            "range": "± 18878",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 13958844,
            "range": "± 266026",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13955227,
            "range": "± 61112",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/bg_only_warm",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/mask_decode",
            "value": 5283764,
            "range": "± 40547",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 29611790,
            "range": "± 112817",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 133956265,
            "range": "± 3009171",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 132889501,
            "range": "± 1475462",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 208007,
            "range": "± 393",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8478094,
            "range": "± 10486",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1432167790,
            "range": "± 3342354",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4408065,
            "range": "± 58844",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3324,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 26420108,
            "range": "± 133503",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 26419378,
            "range": "± 165190",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3168590,
            "range": "± 12082",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 23148904,
            "range": "± 101392",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 199433,
            "range": "± 860",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8189000,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "fa331459e5995805389d3b98a47b8ffdf96d63d0",
          "message": "chore(main): release 0.15.0 (#234)\n\n* chore(main): release 0.15.0\n\n* docs(changelog): add missing iw44 perf entries (#252, #257)\n\nRelease-please skipped two perf(iw44) commits in the auto-generated\n0.15.0 changelog:\n- WASM simd128 inverse wavelet load/store (Phase 2 of #190, #257)\n- x86_64 AVX2 stride-1 load/store (Phase 2 of #189, #252)\n\nBoth landed on main since v0.14.0 with conventional-commit subjects.\n\n---------\n\nCo-authored-by: github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>\nCo-authored-by: leo <leva.matyushkin@gmail.com>",
          "timestamp": "2026-05-03T22:40:43+09:00",
          "tree_id": "5f445552c5bc7bab25069fa6602c96e72141753a",
          "url": "https://github.com/matyushkin/djvu-rs/commit/fa331459e5995805389d3b98a47b8ffdf96d63d0"
        },
        "date": 1777816253944,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 116,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 158319,
            "range": "± 1102",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 763926,
            "range": "± 4148",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 567583,
            "range": "± 2369",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1320839,
            "range": "± 13969",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2659,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9304208,
            "range": "± 24968",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 590274,
            "range": "± 1655",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2228720,
            "range": "± 56791",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2762234,
            "range": "± 7802",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 27715401,
            "range": "± 115006",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 227739,
            "range": "± 305",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 339593,
            "range": "± 2422",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1599129,
            "range": "± 36315",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6159949,
            "range": "± 153614",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 23868507,
            "range": "± 54199",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1698792,
            "range": "± 27496",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 13462486,
            "range": "± 36027",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13475216,
            "range": "± 52316",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/bg_only_warm",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/mask_decode",
            "value": 5283449,
            "range": "± 30415",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28674484,
            "range": "± 169866",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 134118191,
            "range": "± 2104611",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 133231301,
            "range": "± 1352732",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 208482,
            "range": "± 2832",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8478994,
            "range": "± 111329",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1428384099,
            "range": "± 3724793",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4115855,
            "range": "± 40359",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3322,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 26794433,
            "range": "± 339303",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 26800392,
            "range": "± 316332",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3156803,
            "range": "± 20447",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22899473,
            "range": "± 99148",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 199344,
            "range": "± 845",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8170000,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "leva.matyushkin@gmail.com",
            "name": "leo",
            "username": "matyushkin"
          },
          "committer": {
            "email": "leva.matyushkin@gmail.com",
            "name": "leo",
            "username": "matyushkin"
          },
          "distinct": true,
          "id": "f833675708961ddf2100d2c351aa93170808b8b7",
          "message": "Fix JB2 long-corpus roundtrip caps",
          "timestamp": "2026-05-04T01:27:42+09:00",
          "tree_id": "010bb9f95fb5e7a3f0d053f6d85ea8543ee43a42",
          "url": "https://github.com/matyushkin/djvu-rs/commit/f833675708961ddf2100d2c351aa93170808b8b7"
        },
        "date": 1777826271007,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 118,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 146397,
            "range": "± 506",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 820037,
            "range": "± 2354",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 575833,
            "range": "± 2086",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1540233,
            "range": "± 15329",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 3069,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 10022875,
            "range": "± 186415",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 590961,
            "range": "± 1571",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2276521,
            "range": "± 47475",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2761838,
            "range": "± 32903",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 32276203,
            "range": "± 587709",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 219895,
            "range": "± 1189",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 340973,
            "range": "± 10060",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1313961,
            "range": "± 8271",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 5018844,
            "range": "± 104326",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 19971749,
            "range": "± 101271",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1598978,
            "range": "± 17275",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 12854357,
            "range": "± 137820",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13003044,
            "range": "± 65436",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/bg_only_warm",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/mask_decode",
            "value": 5132543,
            "range": "± 17700",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 30326668,
            "range": "± 179894",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 122023345,
            "range": "± 300753",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 121121637,
            "range": "± 301996",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 206893,
            "range": "± 417",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 9244347,
            "range": "± 18359",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1301271364,
            "range": "± 3024413",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 7255066,
            "range": "± 92904",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2368,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 27097346,
            "range": "± 351693",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 27005510,
            "range": "± 274681",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3134444,
            "range": "± 4975",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 23043045,
            "range": "± 48140",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 216074,
            "range": "± 1918",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 7432000,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "leva.matyushkin@gmail.com",
            "name": "leo",
            "username": "matyushkin"
          },
          "committer": {
            "email": "leva.matyushkin@gmail.com",
            "name": "leo",
            "username": "matyushkin"
          },
          "distinct": true,
          "id": "940baf755691a99dcd2c86566216416e9a752592",
          "message": "Fix JB2 fuzz timeout on exhausted ZP input",
          "timestamp": "2026-05-04T10:58:38+09:00",
          "tree_id": "8d83f1c3d34e0d0b9da4f8cc271df5f645979351",
          "url": "https://github.com/matyushkin/djvu-rs/commit/940baf755691a99dcd2c86566216416e9a752592"
        },
        "date": 1777860545516,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 110,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 167199,
            "range": "± 313",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 846511,
            "range": "± 4379",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 379844,
            "range": "± 11493",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1268917,
            "range": "± 28876",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2587,
            "range": "± 108",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 10225913,
            "range": "± 120128",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 617622,
            "range": "± 8341",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2386873,
            "range": "± 8731",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2886384,
            "range": "± 47685",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 28377879,
            "range": "± 286586",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 223822,
            "range": "± 432",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 358598,
            "range": "± 2328",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1713882,
            "range": "± 7782",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6617608,
            "range": "± 45689",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 25777065,
            "range": "± 86049",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1827795,
            "range": "± 13803",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 13109396,
            "range": "± 129566",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13119183,
            "range": "± 197581",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/bg_only_warm",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/mask_decode",
            "value": 5849861,
            "range": "± 27414",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 29871935,
            "range": "± 231608",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 140744440,
            "range": "± 281940",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 139813123,
            "range": "± 1639310",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 226332,
            "range": "± 506",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8392389,
            "range": "± 20679",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1464417860,
            "range": "± 5414253",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4562012,
            "range": "± 224143",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3487,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 23676652,
            "range": "± 172482",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 23605180,
            "range": "± 227367",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3444426,
            "range": "± 5603",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 24685039,
            "range": "± 110354",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 130999,
            "range": "± 604",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8715000,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "leva.matyushkin@gmail.com",
            "name": "leo",
            "username": "matyushkin"
          },
          "committer": {
            "email": "leva.matyushkin@gmail.com",
            "name": "leo",
            "username": "matyushkin"
          },
          "distinct": true,
          "id": "4445288db51131287732eac8dfb22b804a7d55b6",
          "message": "Fix repeated JB2 fuzz timeout after EOF",
          "timestamp": "2026-05-04T11:56:52+09:00",
          "tree_id": "b654a8c0eb51de45ac0e506c0b35243cbfb5f2ff",
          "url": "https://github.com/matyushkin/djvu-rs/commit/4445288db51131287732eac8dfb22b804a7d55b6"
        },
        "date": 1777864022616,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 117,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 157441,
            "range": "± 1958",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 776938,
            "range": "± 14088",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 563466,
            "range": "± 6201",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1311870,
            "range": "± 16559",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2600,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9266920,
            "range": "± 107197",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 586936,
            "range": "± 16889",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2232898,
            "range": "± 21887",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2748443,
            "range": "± 9414",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 27386518,
            "range": "± 225867",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 228462,
            "range": "± 923",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 353633,
            "range": "± 1743",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1578415,
            "range": "± 8455",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6092690,
            "range": "± 15669",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 23621608,
            "range": "± 32575",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1688214,
            "range": "± 19518",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 13047083,
            "range": "± 160284",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13031494,
            "range": "± 364829",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/bg_only_warm",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/mask_decode",
            "value": 5278711,
            "range": "± 39409",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 27904963,
            "range": "± 175717",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 133562302,
            "range": "± 1255835",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 132497932,
            "range": "± 233211",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 219573,
            "range": "± 405",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8463904,
            "range": "± 242828",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1435515020,
            "range": "± 4144050",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4036865,
            "range": "± 38330",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3305,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 25089773,
            "range": "± 290227",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 25139962,
            "range": "± 336942",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3160584,
            "range": "± 10305",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22991064,
            "range": "± 103990",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 199240,
            "range": "± 1032",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8182000,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "leva.matyushkin@gmail.com",
            "name": "leo",
            "username": "matyushkin"
          },
          "committer": {
            "email": "leva.matyushkin@gmail.com",
            "name": "leo",
            "username": "matyushkin"
          },
          "distinct": true,
          "id": "c939ba35952c4935ab69d971b62a0e417ed42e8c",
          "message": "chore(crates): extract djvu-bzz crate",
          "timestamp": "2026-05-04T12:12:43+09:00",
          "tree_id": "ac8aa3bddaeb8cc889c97afe5dd6c9bb32f26c16",
          "url": "https://github.com/matyushkin/djvu-rs/commit/c939ba35952c4935ab69d971b62a0e417ed42e8c"
        },
        "date": 1777865015928,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 117,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 158687,
            "range": "± 1208",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 775356,
            "range": "± 12999",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 564049,
            "range": "± 3129",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1298412,
            "range": "± 13385",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2595,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9603214,
            "range": "± 61623",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 591037,
            "range": "± 13291",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2264602,
            "range": "± 16088",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2761555,
            "range": "± 5261",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 28203354,
            "range": "± 151738",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 230447,
            "range": "± 384",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 348826,
            "range": "± 1555",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1515365,
            "range": "± 11545",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 5780472,
            "range": "± 22872",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 22410186,
            "range": "± 86913",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1686322,
            "range": "± 30709",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 13208508,
            "range": "± 106267",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13194907,
            "range": "± 65087",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/bg_only_warm",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/mask_decode",
            "value": 5325212,
            "range": "± 19979",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28434366,
            "range": "± 337087",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 136989889,
            "range": "± 225577",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 135900292,
            "range": "± 248096",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 214576,
            "range": "± 551",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8447483,
            "range": "± 25930",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1438861917,
            "range": "± 1750301",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4232828,
            "range": "± 52352",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3323,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 22768785,
            "range": "± 163611",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 22714313,
            "range": "± 279696",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3160784,
            "range": "± 18459",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22966675,
            "range": "± 56595",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 197939,
            "range": "± 2737",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8156000,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}