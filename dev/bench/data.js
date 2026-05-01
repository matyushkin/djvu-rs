window.BENCHMARK_DATA = {
  "lastUpdate": 1777636419709,
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
      }
    ]
  }
}