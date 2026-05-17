window.BENCHMARK_DATA = {
  "lastUpdate": 1778995128012,
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
          "id": "1359764c39afe86cc2e258896dc353f032df4fc3",
          "message": "chore(crates): extract djvu-pixmap crate",
          "timestamp": "2026-05-04T12:22:35+09:00",
          "tree_id": "4519a1ca090e7c292a4406af3a2683b2a82eac61",
          "url": "https://github.com/matyushkin/djvu-rs/commit/1359764c39afe86cc2e258896dc353f032df4fc3"
        },
        "date": 1777865647169,
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
            "value": 159182,
            "range": "± 7784",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 769358,
            "range": "± 5665",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 567451,
            "range": "± 1854",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1427243,
            "range": "± 15715",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2696,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9554515,
            "range": "± 107227",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 595224,
            "range": "± 2143",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2277828,
            "range": "± 17784",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2769759,
            "range": "± 14669",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 30098535,
            "range": "± 1016887",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 229941,
            "range": "± 1140",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 353815,
            "range": "± 1261",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1583062,
            "range": "± 9147",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6101769,
            "range": "± 17021",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 24073489,
            "range": "± 77531",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1695202,
            "range": "± 8002",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 13645124,
            "range": "± 60169",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13621242,
            "range": "± 64929",
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
            "value": 5371932,
            "range": "± 107491",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 30365125,
            "range": "± 840939",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 135696805,
            "range": "± 175992",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 134962424,
            "range": "± 524683",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 215295,
            "range": "± 1922",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8525227,
            "range": "± 16448",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1444700046,
            "range": "± 5020114",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4531099,
            "range": "± 94614",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3098,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 23035417,
            "range": "± 313174",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 23066287,
            "range": "± 190025",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3175830,
            "range": "± 34271",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 23600089,
            "range": "± 152839",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 201039,
            "range": "± 3632",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8199999,
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
          "id": "1dc81b581da7e528ed7eab8ea8a77a319df5000f",
          "message": "chore(crates): extract djvu-iw44 crate",
          "timestamp": "2026-05-04T12:25:58+09:00",
          "tree_id": "fc8294c208ef9d9b6a37f1bd764106658d590d4a",
          "url": "https://github.com/matyushkin/djvu-rs/commit/1dc81b581da7e528ed7eab8ea8a77a319df5000f"
        },
        "date": 1777866268850,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 116,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 159260,
            "range": "± 2472",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 770232,
            "range": "± 18523",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 563409,
            "range": "± 2524",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1305408,
            "range": "± 13068",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2589,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9173851,
            "range": "± 37578",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 563046,
            "range": "± 1551",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2154366,
            "range": "± 11201",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2746451,
            "range": "± 61810",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 28127078,
            "range": "± 781107",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 227837,
            "range": "± 581",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 344689,
            "range": "± 2863",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1586885,
            "range": "± 16387",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6130063,
            "range": "± 29198",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 23743670,
            "range": "± 157376",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1697251,
            "range": "± 13344",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 13142142,
            "range": "± 54484",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13118824,
            "range": "± 35844",
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
            "value": 5366531,
            "range": "± 65800",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28219449,
            "range": "± 437261",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 133904293,
            "range": "± 2833441",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 133121695,
            "range": "± 421150",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 213432,
            "range": "± 894",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8549652,
            "range": "± 17349",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1432738268,
            "range": "± 6074578",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4044161,
            "range": "± 55373",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3303,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 27259852,
            "range": "± 862985",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 26138365,
            "range": "± 404895",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3148694,
            "range": "± 19067",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22794214,
            "range": "± 58417",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 200754,
            "range": "± 1142",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8211000,
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
          "id": "77fc6ffecd57cb5760f9a7e75333987b3530c872",
          "message": "feat(async): add wasm lazy reader entrypoint",
          "timestamp": "2026-05-04T12:42:55+09:00",
          "tree_id": "cc57a8455dfbb04b43f20e0425ec317b2ddac02b",
          "url": "https://github.com/matyushkin/djvu-rs/commit/77fc6ffecd57cb5760f9a7e75333987b3530c872"
        },
        "date": 1777866891590,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 117,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 159764,
            "range": "± 2244",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 765362,
            "range": "± 14639",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 564096,
            "range": "± 9813",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1307635,
            "range": "± 10234",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2581,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9197328,
            "range": "± 54207",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 566902,
            "range": "± 1920",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2295996,
            "range": "± 10080",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2750859,
            "range": "± 11905",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 28091162,
            "range": "± 394918",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 228056,
            "range": "± 369",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 344332,
            "range": "± 6609",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1585733,
            "range": "± 13298",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6119901,
            "range": "± 30593",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 23833303,
            "range": "± 141260",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1686935,
            "range": "± 10583",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 13126087,
            "range": "± 429894",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13132903,
            "range": "± 177899",
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
            "value": 5308241,
            "range": "± 25399",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28924802,
            "range": "± 268980",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 134406590,
            "range": "± 780900",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 133630081,
            "range": "± 271449",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 212942,
            "range": "± 1478",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8516138,
            "range": "± 48731",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1436926043,
            "range": "± 6171921",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4261582,
            "range": "± 86594",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3321,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 26881594,
            "range": "± 282190",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 26378218,
            "range": "± 323504",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3151190,
            "range": "± 7647",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22800930,
            "range": "± 49157",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 200561,
            "range": "± 1568",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8169000,
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
          "id": "b09fc413ebf002d02516a5cd5834a52c6bc578fe",
          "message": "Merge pull request #275 from matyushkin/release-please--branches--main--components--djvu-rs\n\nchore(main): release 0.16.0",
          "timestamp": "2026-05-04T13:52:34+09:00",
          "tree_id": "9029032b4e26e59e9abbcb30b2024239f76e4a28",
          "url": "https://github.com/matyushkin/djvu-rs/commit/b09fc413ebf002d02516a5cd5834a52c6bc578fe"
        },
        "date": 1777870945559,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 116,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 163838,
            "range": "± 864",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 765064,
            "range": "± 4341",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 564972,
            "range": "± 1932",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1287933,
            "range": "± 7111",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2598,
            "range": "± 84",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9269605,
            "range": "± 28049",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 563107,
            "range": "± 1625",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2163460,
            "range": "± 4690",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2745781,
            "range": "± 6862",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 27987152,
            "range": "± 460675",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 227969,
            "range": "± 938",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 351164,
            "range": "± 16650",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1587292,
            "range": "± 6921",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6086176,
            "range": "± 20218",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 23645527,
            "range": "± 424076",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1679911,
            "range": "± 17211",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 13353018,
            "range": "± 208472",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13240574,
            "range": "± 74536",
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
            "value": 5379097,
            "range": "± 39514",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28306330,
            "range": "± 421685",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 136364439,
            "range": "± 2256068",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 134395128,
            "range": "± 738007",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 212683,
            "range": "± 1775",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8499934,
            "range": "± 30788",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1431074633,
            "range": "± 6579099",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4837803,
            "range": "± 607219",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3322,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 25142027,
            "range": "± 233624",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 25232553,
            "range": "± 1029884",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3146102,
            "range": "± 13715",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22802468,
            "range": "± 113552",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 200001,
            "range": "± 1110",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 163000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8226000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 49516000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 47623000,
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
          "id": "d54982a8abdad2ed90c5fdf4f5a03cefb9d1057c",
          "message": "ci: fix release publishing for workspace crates",
          "timestamp": "2026-05-04T13:58:07+09:00",
          "tree_id": "2cfb523d205fe6905cc4575180f30d9dae79bb96",
          "url": "https://github.com/matyushkin/djvu-rs/commit/d54982a8abdad2ed90c5fdf4f5a03cefb9d1057c"
        },
        "date": 1777871535110,
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
            "value": 146666,
            "range": "± 288",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 827127,
            "range": "± 4935",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 573989,
            "range": "± 3101",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1514765,
            "range": "± 43487",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 3121,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 8890550,
            "range": "± 90282",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 579411,
            "range": "± 7355",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2145502,
            "range": "± 3936",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2758348,
            "range": "± 13897",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 29531322,
            "range": "± 537053",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 214492,
            "range": "± 1287",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 339693,
            "range": "± 6412",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1305323,
            "range": "± 12892",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 5023190,
            "range": "± 8185",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 19411032,
            "range": "± 31981",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1583358,
            "range": "± 26387",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 12370058,
            "range": "± 48061",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 12372809,
            "range": "± 73416",
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
            "value": 5132434,
            "range": "± 16667",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 29023711,
            "range": "± 326624",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 122333663,
            "range": "± 494862",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 121041968,
            "range": "± 322438",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 205765,
            "range": "± 311",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 9235582,
            "range": "± 11332",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1283541809,
            "range": "± 5061140",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 6168896,
            "range": "± 119163",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2320,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 23904849,
            "range": "± 323497",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 24209667,
            "range": "± 448200",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3136822,
            "range": "± 5340",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22864371,
            "range": "± 49899",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 217884,
            "range": "± 3705",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 141000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 7185000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 45089000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 43323000,
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
          "id": "b0f6ff44dc3ec05dd26dafae20ecea01ea19a45c",
          "message": "chore: release v0.16.1",
          "timestamp": "2026-05-04T14:01:37+09:00",
          "tree_id": "37bf48a5f742b2bba4910440def95cd69d6c4ae9",
          "url": "https://github.com/matyushkin/djvu-rs/commit/b0f6ff44dc3ec05dd26dafae20ecea01ea19a45c"
        },
        "date": 1777872155496,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 107,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 166973,
            "range": "± 875",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 834311,
            "range": "± 4794",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 447848,
            "range": "± 19638",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1210349,
            "range": "± 8124",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2612,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9852962,
            "range": "± 126430",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 594776,
            "range": "± 1924",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2406277,
            "range": "± 46938",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2863650,
            "range": "± 20043",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 28105056,
            "range": "± 927036",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 224520,
            "range": "± 1036",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 351986,
            "range": "± 5413",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1704490,
            "range": "± 25777",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6584185,
            "range": "± 30272",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 25656039,
            "range": "± 129065",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1817359,
            "range": "± 13160",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 13026084,
            "range": "± 36787",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13013388,
            "range": "± 357987",
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
            "value": 5830611,
            "range": "± 10346",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 29264305,
            "range": "± 117757",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 140515001,
            "range": "± 1555267",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 139587333,
            "range": "± 259876",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 221267,
            "range": "± 5268",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8286152,
            "range": "± 15077",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1454799007,
            "range": "± 4050595",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4335100,
            "range": "± 36328",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3484,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 23069771,
            "range": "± 185364",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 23038004,
            "range": "± 132013",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3425781,
            "range": "± 8080",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 24298210,
            "range": "± 59984",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 129959,
            "range": "± 765",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 161000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8714000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 53173000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 53102000,
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
          "id": "b9949ee25f1b5efae97dc1eb1c804aa57b2bc0bb",
          "message": "Merge pull request #276 from matyushkin/release-please--branches--main--components--djvu-rs\n\nchore(main): release 0.17.0",
          "timestamp": "2026-05-12T13:17:08+09:00",
          "tree_id": "3ecb4a614f8555724a5f0c60422601986158d95d",
          "url": "https://github.com/matyushkin/djvu-rs/commit/b9949ee25f1b5efae97dc1eb1c804aa57b2bc0bb"
        },
        "date": 1778560064198,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 116,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 158888,
            "range": "± 732",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 769366,
            "range": "± 3040",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 565208,
            "range": "± 2435",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1315787,
            "range": "± 23791",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2675,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9174101,
            "range": "± 58866",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 567729,
            "range": "± 3399",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2168207,
            "range": "± 5971",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2751086,
            "range": "± 17582",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 27910024,
            "range": "± 162576",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 228212,
            "range": "± 5877",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 344329,
            "range": "± 7273",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1586528,
            "range": "± 17238",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6141710,
            "range": "± 32786",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 23904936,
            "range": "± 95098",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1685955,
            "range": "± 18538",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 13198978,
            "range": "± 225394",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13174117,
            "range": "± 91208",
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
            "value": 5313041,
            "range": "± 96811",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28388977,
            "range": "± 177248",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 133237868,
            "range": "± 330004",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 132265482,
            "range": "± 388022",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 213597,
            "range": "± 4935",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8507625,
            "range": "± 26781",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1436468218,
            "range": "± 4448459",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4146298,
            "range": "± 107218",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3315,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 25591100,
            "range": "± 209251",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 25599010,
            "range": "± 325221",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3144092,
            "range": "± 9340",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22799115,
            "range": "± 298707",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 204863,
            "range": "± 719",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 165000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8196999,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 49360000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 47537000,
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
          "id": "a1805a990e71aa35a26f53c9ee87f1c932a9475d",
          "message": "perf: add JB2 cross-size refinement probe (#285)",
          "timestamp": "2026-05-12T14:47:41+09:00",
          "tree_id": "06fa8f3a0f7d296fa2533214a9c5ffa6690978d0",
          "url": "https://github.com/matyushkin/djvu-rs/commit/a1805a990e71aa35a26f53c9ee87f1c932a9475d"
        },
        "date": 1778565471247,
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
            "value": 157875,
            "range": "± 3099",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 783029,
            "range": "± 8308",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 566234,
            "range": "± 2865",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1292398,
            "range": "± 15152",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2658,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 8891192,
            "range": "± 433125",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 561599,
            "range": "± 27626",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2123465,
            "range": "± 41769",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2740948,
            "range": "± 9714",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 27533369,
            "range": "± 701308",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 243304,
            "range": "± 5140",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 345544,
            "range": "± 1992",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1594333,
            "range": "± 9575",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6121764,
            "range": "± 31570",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 23826236,
            "range": "± 406952",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1691768,
            "range": "± 6931",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 13153660,
            "range": "± 58019",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13162972,
            "range": "± 81523",
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
            "value": 5323533,
            "range": "± 52171",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28726390,
            "range": "± 397118",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 133450283,
            "range": "± 638979",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 132511319,
            "range": "± 1295549",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 213757,
            "range": "± 1618",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8518323,
            "range": "± 17076",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1436578342,
            "range": "± 4495579",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4153495,
            "range": "± 62366",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3322,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 21805851,
            "range": "± 526063",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 21884345,
            "range": "± 194732",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3163937,
            "range": "± 8699",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 23321066,
            "range": "± 69225",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 194666,
            "range": "± 3793",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 164000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8176000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 49497000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 47725000,
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
          "id": "6e0b8508689893e01124372ec6fac1dcfed423cd",
          "message": "feat: add archival FGbz scan profile (#287)",
          "timestamp": "2026-05-12T15:03:55+09:00",
          "tree_id": "b0f5bb9cbf2e6ade2a305bb7e6c887a080428470",
          "url": "https://github.com/matyushkin/djvu-rs/commit/6e0b8508689893e01124372ec6fac1dcfed423cd"
        },
        "date": 1778566408725,
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
            "value": 146384,
            "range": "± 9767",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 833721,
            "range": "± 8070",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 576446,
            "range": "± 2904",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1597615,
            "range": "± 12435",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 3071,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 8849796,
            "range": "± 87807",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 569806,
            "range": "± 1988",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2167310,
            "range": "± 14223",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2771433,
            "range": "± 13153",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 29750183,
            "range": "± 403123",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 215528,
            "range": "± 3461",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 338712,
            "range": "± 1503",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1303536,
            "range": "± 31600",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4985679,
            "range": "± 22103",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 19511002,
            "range": "± 333449",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1562680,
            "range": "± 32356",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 12516808,
            "range": "± 291352",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 12558783,
            "range": "± 117756",
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
            "value": 5132504,
            "range": "± 31744",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 29485906,
            "range": "± 443824",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 122492428,
            "range": "± 2441614",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 121280093,
            "range": "± 1075176",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 206534,
            "range": "± 722",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 9229434,
            "range": "± 35547",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1295448433,
            "range": "± 4380964",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 7101151,
            "range": "± 691228",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2425,
            "range": "± 74",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 25871458,
            "range": "± 504137",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 26146662,
            "range": "± 401485",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3128788,
            "range": "± 8168",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22897763,
            "range": "± 341614",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 216300,
            "range": "± 1519",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 147000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 7174000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 46516000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 43416000,
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
          "id": "f9120c8cb132675b5dc0a45e1f378de8a718f076",
          "message": "feat: emit per-blit FGbz color indices (#291)",
          "timestamp": "2026-05-12T15:39:25+09:00",
          "tree_id": "b7e026624086027823c807b717d956752e99d3fa",
          "url": "https://github.com/matyushkin/djvu-rs/commit/f9120c8cb132675b5dc0a45e1f378de8a718f076"
        },
        "date": 1778568575552,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 116,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 158873,
            "range": "± 2395",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 768834,
            "range": "± 6920",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 567563,
            "range": "± 5020",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1417734,
            "range": "± 9577",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2582,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9201481,
            "range": "± 38291",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 565752,
            "range": "± 3260",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2190072,
            "range": "± 25546",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2749203,
            "range": "± 10115",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 28361661,
            "range": "± 369913",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 228684,
            "range": "± 900",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 347383,
            "range": "± 8978",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1574648,
            "range": "± 14088",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6101429,
            "range": "± 22687",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 23802244,
            "range": "± 402740",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1691605,
            "range": "± 11212",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 13242249,
            "range": "± 892521",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13281149,
            "range": "± 454777",
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
            "value": 5326531,
            "range": "± 41784",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28947124,
            "range": "± 286255",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 134966642,
            "range": "± 1940532",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 134473694,
            "range": "± 1716714",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 212694,
            "range": "± 11702",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8508067,
            "range": "± 356686",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1468946334,
            "range": "± 6924328",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4283080,
            "range": "± 52519",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3306,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 22088418,
            "range": "± 356132",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 21853316,
            "range": "± 236230",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3157999,
            "range": "± 11453",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22893016,
            "range": "± 67835",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 198873,
            "range": "± 1792",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 164000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8246000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 49547000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 48643000,
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
          "id": "63fe3dc01064917d364647942d244d6773591c86",
          "message": "fix(render): reduce native colorbook diff",
          "timestamp": "2026-05-17T00:08:13+09:00",
          "tree_id": "0a013eaba2e3f5ab0d32235e898df2221529c5ba",
          "url": "https://github.com/matyushkin/djvu-rs/commit/63fe3dc01064917d364647942d244d6773591c86"
        },
        "date": 1778944690938,
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
            "value": 160887,
            "range": "± 2794",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 769676,
            "range": "± 6707",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 571775,
            "range": "± 1512",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1276793,
            "range": "± 10195",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2647,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9103005,
            "range": "± 108917",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 567894,
            "range": "± 2182",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2184395,
            "range": "± 12791",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2773318,
            "range": "± 11612",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 28222156,
            "range": "± 844762",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 229480,
            "range": "± 844",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 343610,
            "range": "± 2158",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1565034,
            "range": "± 11549",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 5962400,
            "range": "± 47088",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 23039825,
            "range": "± 105037",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1682108,
            "range": "± 10769",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 13256896,
            "range": "± 179211",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13208389,
            "range": "± 177742",
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
            "value": 5355737,
            "range": "± 33277",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28902580,
            "range": "± 781709",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 119097125,
            "range": "± 285487",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 120921662,
            "range": "± 177211",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 209528,
            "range": "± 845",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8502580,
            "range": "± 11554",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1434168314,
            "range": "± 1841003",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4371222,
            "range": "± 46352",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3321,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 23828958,
            "range": "± 282734",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 23324808,
            "range": "± 181329",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3182678,
            "range": "± 8986",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22953235,
            "range": "± 97531",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 194923,
            "range": "± 1194",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 163000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8215999,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 49391000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 47424000,
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
          "id": "4e3f9552010a13dd3aa4304a02367c10dda960cf",
          "message": "perf(export): stream TIFF color rendering",
          "timestamp": "2026-05-17T00:20:36+09:00",
          "tree_id": "03d5eeebe2811b74e935aa1e9bb73ce3ce05af0b",
          "url": "https://github.com/matyushkin/djvu-rs/commit/4e3f9552010a13dd3aa4304a02367c10dda960cf"
        },
        "date": 1778945453778,
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
            "value": 160318,
            "range": "± 4554",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 773848,
            "range": "± 6766",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 574632,
            "range": "± 3983",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1394976,
            "range": "± 14176",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2704,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9284653,
            "range": "± 122860",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 569955,
            "range": "± 1863",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2187870,
            "range": "± 15344",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2770465,
            "range": "± 26266",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 28494730,
            "range": "± 1018728",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 227978,
            "range": "± 2274",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 344988,
            "range": "± 3207",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1562452,
            "range": "± 16928",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 5972626,
            "range": "± 8400",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 23123493,
            "range": "± 351453",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1700431,
            "range": "± 14716",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 13130945,
            "range": "± 159186",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 13376308,
            "range": "± 189472",
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
            "value": 5376824,
            "range": "± 28694",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28839617,
            "range": "± 432851",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 119189667,
            "range": "± 399414",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 121334984,
            "range": "± 308161",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 209115,
            "range": "± 2346",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8499462,
            "range": "± 93081",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1446899685,
            "range": "± 3480395",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4426035,
            "range": "± 107770",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3325,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 23644318,
            "range": "± 426839",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 23718163,
            "range": "± 380993",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3185526,
            "range": "± 12925",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22953479,
            "range": "± 159681",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 199512,
            "range": "± 2701",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 163000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8182000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 49874000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 47657000,
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
          "id": "39fbe27cd46792b7b69d4bfa808406c37bed3619",
          "message": "perf(render): composite pixmap output directly",
          "timestamp": "2026-05-17T00:34:11+09:00",
          "tree_id": "551c4417125c15cce450a492c2759dcea30ca52f",
          "url": "https://github.com/matyushkin/djvu-rs/commit/39fbe27cd46792b7b69d4bfa808406c37bed3619"
        },
        "date": 1778946427017,
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
            "value": 167085,
            "range": "± 2332",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 834350,
            "range": "± 14647",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 379263,
            "range": "± 11259",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1292679,
            "range": "± 7126",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2542,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9787705,
            "range": "± 164644",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 598689,
            "range": "± 15463",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2497690,
            "range": "± 54834",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2890126,
            "range": "± 71249",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 29431943,
            "range": "± 1835847",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 223944,
            "range": "± 4523",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 391728,
            "range": "± 3596",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1788909,
            "range": "± 21188",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6923600,
            "range": "± 92424",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 26973067,
            "range": "± 207539",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1837281,
            "range": "± 9562",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 12409635,
            "range": "± 51058",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 12414936,
            "range": "± 340297",
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
            "value": 5888070,
            "range": "± 15133",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28880835,
            "range": "± 324324",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 141720451,
            "range": "± 2126407",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 144806570,
            "range": "± 1733624",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 141434396,
            "range": "± 3156618",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 139371127,
            "range": "± 542372",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 128474529,
            "range": "± 1558572",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3724731,
            "range": "± 6928",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4740519,
            "range": "± 11335",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 144452053,
            "range": "± 1488155",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 142512178,
            "range": "± 1797590",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 130003565,
            "range": "± 204781",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 389628,
            "range": "± 16769",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4743409,
            "range": "± 97797",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 257316,
            "range": "± 575",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8362755,
            "range": "± 10251",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1474817265,
            "range": "± 6960205",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4963406,
            "range": "± 53859",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3492,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 22686413,
            "range": "± 255177",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 22787184,
            "range": "± 491898",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3476574,
            "range": "± 16580",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 24728403,
            "range": "± 695467",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 129209,
            "range": "± 892",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 162000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8715000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 53832000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 51497000,
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
          "id": "913e1001ac75078a49ee41e06e3b73cc5217d9f6",
          "message": "feat(ocr): narrow experimental backends",
          "timestamp": "2026-05-17T00:47:18+09:00",
          "tree_id": "fe661cb816a19bd6ad897059ec572aa6113ebee9",
          "url": "https://github.com/matyushkin/djvu-rs/commit/913e1001ac75078a49ee41e06e3b73cc5217d9f6"
        },
        "date": 1778947189092,
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
            "value": 147210,
            "range": "± 2370",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 831898,
            "range": "± 3770",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 575548,
            "range": "± 2520",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1614299,
            "range": "± 5815",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 3116,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9252274,
            "range": "± 160029",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 581359,
            "range": "± 1888",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2136146,
            "range": "± 10664",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2779089,
            "range": "± 5712",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 29639608,
            "range": "± 1414834",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 222282,
            "range": "± 1715",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 369756,
            "range": "± 6121",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1432745,
            "range": "± 13039",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 5570225,
            "range": "± 7707",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 21848653,
            "range": "± 60500",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1656709,
            "range": "± 26923",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 11316106,
            "range": "± 104157",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 11467698,
            "range": "± 238769",
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
            "value": 5136741,
            "range": "± 15775",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28796233,
            "range": "± 272698",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 118429949,
            "range": "± 413700",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 121039905,
            "range": "± 389010",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 118646067,
            "range": "± 895020",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 113431767,
            "range": "± 586057",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 103089198,
            "range": "± 863730",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3397573,
            "range": "± 11206",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4256022,
            "range": "± 13944",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 120576372,
            "range": "± 302118",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 115783666,
            "range": "± 293620",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 103839458,
            "range": "± 128214",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 574197,
            "range": "± 1859",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4280127,
            "range": "± 35178",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 232342,
            "range": "± 383",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 9315733,
            "range": "± 13197",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1301830082,
            "range": "± 3423931",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 6625761,
            "range": "± 201127",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2472,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 22715692,
            "range": "± 280882",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 22476318,
            "range": "± 262643",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3132700,
            "range": "± 3906",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22859139,
            "range": "± 41410",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 218533,
            "range": "± 1574",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 141000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 7377000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 45902000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 43659000,
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
          "id": "7b71793bfa7c4378d22587da9fade166790979d4",
          "message": "feat(encode): add adaptive segmentation options",
          "timestamp": "2026-05-17T01:10:42+09:00",
          "tree_id": "697d4409843c9bd60212eacc6aaf881f2b30fe01",
          "url": "https://github.com/matyushkin/djvu-rs/commit/7b71793bfa7c4378d22587da9fade166790979d4"
        },
        "date": 1778948602326,
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
            "value": 158621,
            "range": "± 1326",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 765466,
            "range": "± 3546",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 569540,
            "range": "± 1632",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1328180,
            "range": "± 15071",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2571,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9245311,
            "range": "± 75444",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 572035,
            "range": "± 2369",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2172085,
            "range": "± 13091",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2750357,
            "range": "± 12178",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 28606987,
            "range": "± 306573",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 229013,
            "range": "± 689",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 374655,
            "range": "± 3534",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1608379,
            "range": "± 7450",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6236351,
            "range": "± 117660",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 24236868,
            "range": "± 45285",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1671086,
            "range": "± 10601",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 11345012,
            "range": "± 36491",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 11354732,
            "range": "± 29144",
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
            "value": 5330560,
            "range": "± 22875",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 26620624,
            "range": "± 393803",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 129872729,
            "range": "± 645800",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 132654893,
            "range": "± 501503",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 129630286,
            "range": "± 327056",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 126788861,
            "range": "± 177056",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 116258888,
            "range": "± 123381",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3399276,
            "range": "± 7663",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4497084,
            "range": "± 24734",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 132501138,
            "range": "± 1011534",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 129694382,
            "range": "± 219734",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 118858671,
            "range": "± 503239",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 571846,
            "range": "± 1757",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4487775,
            "range": "± 28401",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 239643,
            "range": "± 1329",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8549457,
            "range": "± 153154",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1436527955,
            "range": "± 2167445",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4545499,
            "range": "± 80715",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3322,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 24071366,
            "range": "± 358286",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 23949455,
            "range": "± 200026",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3168419,
            "range": "± 15413",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22915942,
            "range": "± 357413",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 193022,
            "range": "± 1543",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 164000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8241000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 49730000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 47907000,
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
          "id": "404054c4a848db1452f49d050492858da7b4be69",
          "message": "feat(encode): support layered directory bundles",
          "timestamp": "2026-05-17T01:17:41+09:00",
          "tree_id": "517e51009884232fad278edeb02cee6963006b5e",
          "url": "https://github.com/matyushkin/djvu-rs/commit/404054c4a848db1452f49d050492858da7b4be69"
        },
        "date": 1778949359584,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 116,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 163715,
            "range": "± 642",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 765300,
            "range": "± 9165",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 565014,
            "range": "± 2402",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1307753,
            "range": "± 18578",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2586,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9168966,
            "range": "± 257730",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 592589,
            "range": "± 3732",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2166418,
            "range": "± 11727",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2756268,
            "range": "± 29480",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 28009671,
            "range": "± 157579",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 229799,
            "range": "± 9189",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 373624,
            "range": "± 2191",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1603524,
            "range": "± 7889",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6197246,
            "range": "± 79133",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 24096869,
            "range": "± 54737",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1677477,
            "range": "± 34859",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 11660387,
            "range": "± 138936",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 11578105,
            "range": "± 149203",
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
            "value": 5330865,
            "range": "± 25144",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 26667977,
            "range": "± 780361",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 130317053,
            "range": "± 649271",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 133174908,
            "range": "± 217890",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 130249824,
            "range": "± 3418199",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 126709200,
            "range": "± 166731",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 116198041,
            "range": "± 139442",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3385381,
            "range": "± 10713",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4447871,
            "range": "± 123167",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 132923904,
            "range": "± 1428941",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 129688605,
            "range": "± 469937",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 118812644,
            "range": "± 122481",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 566087,
            "range": "± 8563",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4454153,
            "range": "± 157205",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 240394,
            "range": "± 560",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8562676,
            "range": "± 85381",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1436727779,
            "range": "± 7668344",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4305043,
            "range": "± 56959",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3321,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 23464722,
            "range": "± 158358",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 23627401,
            "range": "± 281907",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3155946,
            "range": "± 7513",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22839705,
            "range": "± 73938",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 196348,
            "range": "± 1042",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 164000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8250000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 49262000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 47647000,
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
          "id": "c3a362d92bcb34c0c4c549aab5271a4239a053c3",
          "message": "chore(main): release 0.18.0",
          "timestamp": "2026-05-17T09:54:00+09:00",
          "tree_id": "7cb779de980af7deb0e7b7b72bc55ce464578d38",
          "url": "https://github.com/matyushkin/djvu-rs/commit/c3a362d92bcb34c0c4c549aab5271a4239a053c3"
        },
        "date": 1778980003329,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 106,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 166585,
            "range": "± 419",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 834174,
            "range": "± 10341",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 389994,
            "range": "± 19110",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1238722,
            "range": "± 23787",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2607,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9740670,
            "range": "± 45293",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 612171,
            "range": "± 13192",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2347357,
            "range": "± 12909",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2870375,
            "range": "± 5124",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 28431718,
            "range": "± 193844",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 223967,
            "range": "± 3400",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 392161,
            "range": "± 1133",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1787110,
            "range": "± 7432",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6921797,
            "range": "± 14234",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 26919809,
            "range": "± 54692",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1828373,
            "range": "± 3859",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 12039277,
            "range": "± 63919",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 12034663,
            "range": "± 35496",
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
            "value": 5848107,
            "range": "± 15447",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28496850,
            "range": "± 78768",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 142829362,
            "range": "± 2242123",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 146109697,
            "range": "± 323958",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 142572720,
            "range": "± 3986121",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 139065353,
            "range": "± 565120",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 128110788,
            "range": "± 164001",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3682085,
            "range": "± 7195",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4745707,
            "range": "± 10044",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 145640721,
            "range": "± 1119622",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 142379246,
            "range": "± 363037",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 129657762,
            "range": "± 258796",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 401966,
            "range": "± 10483",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4746670,
            "range": "± 7875",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 257252,
            "range": "± 573",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8366082,
            "range": "± 14824",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1475791394,
            "range": "± 3017227",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4613340,
            "range": "± 65916",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3494,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 22349566,
            "range": "± 190015",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 22281850,
            "range": "± 63077",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3444420,
            "range": "± 20128",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 24428136,
            "range": "± 98078",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 130141,
            "range": "± 350",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 161000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8713000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 53323000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 51326000,
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
          "id": "8ac5645971b03bc0cb8a454c6433071f74f8635c",
          "message": "bench: add compositor-only render baselines (#312)",
          "timestamp": "2026-05-17T11:24:58+09:00",
          "tree_id": "0dfbdcad5a28521e517c0612b71869ea362519dd",
          "url": "https://github.com/matyushkin/djvu-rs/commit/8ac5645971b03bc0cb8a454c6433071f74f8635c"
        },
        "date": 1778985519952,
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
            "value": 158423,
            "range": "± 1346",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 766410,
            "range": "± 37659",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 567522,
            "range": "± 1939",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1304430,
            "range": "± 9736",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2609,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9511534,
            "range": "± 95217",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 568809,
            "range": "± 3405",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2166410,
            "range": "± 18958",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2760991,
            "range": "± 11924",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 29347877,
            "range": "± 779699",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 229454,
            "range": "± 712",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 373969,
            "range": "± 4090",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1631500,
            "range": "± 14532",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6220098,
            "range": "± 13708",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 24307085,
            "range": "± 66382",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1679500,
            "range": "± 27574",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 11458966,
            "range": "± 196284",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 11551703,
            "range": "± 141820",
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
            "value": 5294521,
            "range": "± 62304",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 27363015,
            "range": "± 452766",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 130994196,
            "range": "± 626442",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 133788005,
            "range": "± 168117",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 130938809,
            "range": "± 223532",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 127612091,
            "range": "± 114293",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 116429621,
            "range": "± 103300",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3377233,
            "range": "± 7552",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4443523,
            "range": "± 21975",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 133683365,
            "range": "± 218279",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 130342897,
            "range": "± 141420",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 118864255,
            "range": "± 1542061",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 569779,
            "range": "± 1814",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4460344,
            "range": "± 13774",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 127531150,
            "range": "± 133357",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 130382622,
            "range": "± 1167450",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 11486124,
            "range": "± 92797",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 243595,
            "range": "± 3622",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 241531,
            "range": "± 1025",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8563770,
            "range": "± 75618",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1446365639,
            "range": "± 1943487",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4200583,
            "range": "± 230415",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3322,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 23583015,
            "range": "± 303124",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 23706826,
            "range": "± 170686",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3157107,
            "range": "± 7538",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22904875,
            "range": "± 416506",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 198171,
            "range": "± 1120",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 167000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8275000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 49325000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 47618000,
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
          "id": "b4bf7bec9ef3bfff3f0540aa564817241f14fb12",
          "message": "bench: record row scratch render ab (#313)",
          "timestamp": "2026-05-17T11:59:54+09:00",
          "tree_id": "e29880bec8b32045711b162218417a4fed161ef1",
          "url": "https://github.com/matyushkin/djvu-rs/commit/b4bf7bec9ef3bfff3f0540aa564817241f14fb12"
        },
        "date": 1778987692480,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 108,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 167092,
            "range": "± 1148",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 836581,
            "range": "± 4114",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 391188,
            "range": "± 15758",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1220035,
            "range": "± 10640",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2613,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9598749,
            "range": "± 29986",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 594589,
            "range": "± 5303",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2302681,
            "range": "± 9396",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2868971,
            "range": "± 5980",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 28049255,
            "range": "± 118111",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 224105,
            "range": "± 415",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 391087,
            "range": "± 2436",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1783988,
            "range": "± 8301",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6912762,
            "range": "± 13248",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 26900394,
            "range": "± 39181",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1831311,
            "range": "± 8976",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 11933303,
            "range": "± 53998",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 11935465,
            "range": "± 67129",
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
            "value": 5839366,
            "range": "± 10847",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28159414,
            "range": "± 110056",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 141733623,
            "range": "± 475508",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 144739338,
            "range": "± 600179",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 141918506,
            "range": "± 516896",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 138940161,
            "range": "± 687454",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 140729582,
            "range": "± 328852",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3668601,
            "range": "± 5635",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4725250,
            "range": "± 13613",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 144990312,
            "range": "± 614925",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 141932579,
            "range": "± 395700",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 144081537,
            "range": "± 271191",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 390827,
            "range": "± 8381",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4886563,
            "range": "± 6088",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 139092563,
            "range": "± 253990",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 142169905,
            "range": "± 305008",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 11863968,
            "range": "± 37475",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 256338,
            "range": "± 423",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 390501,
            "range": "± 219",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 350817,
            "range": "± 516",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 256498,
            "range": "± 214",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 219422,
            "range": "± 343",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 11841976,
            "range": "± 23127",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 12328686,
            "range": "± 91612",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 138919744,
            "range": "± 410277",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 142336367,
            "range": "± 612305",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 142076736,
            "range": "± 494762",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 145674988,
            "range": "± 39002",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 257215,
            "range": "± 1144",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8368110,
            "range": "± 17474",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1468812897,
            "range": "± 958215",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4364352,
            "range": "± 63854",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3493,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 22244862,
            "range": "± 85747",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 22239195,
            "range": "± 95210",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3442678,
            "range": "± 7895",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 24354402,
            "range": "± 95901",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 130303,
            "range": "± 508",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 163000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8694000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 53155000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 51183000,
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
          "id": "bfb9257b31d4eb4c70ebef0976d30333ecbeb73a",
          "message": "feat: embed OCR text layers in DjVu output (#315)",
          "timestamp": "2026-05-17T12:51:07+09:00",
          "tree_id": "ccdf5b7ad42c188342a9cbfd887737dc64cbf165",
          "url": "https://github.com/matyushkin/djvu-rs/commit/bfb9257b31d4eb4c70ebef0976d30333ecbeb73a"
        },
        "date": 1778990760346,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 107,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 166828,
            "range": "± 507",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 826520,
            "range": "± 5709",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 418609,
            "range": "± 18994",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1316295,
            "range": "± 9773",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2550,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9783978,
            "range": "± 33344",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 599164,
            "range": "± 3395",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2417026,
            "range": "± 5398",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2866021,
            "range": "± 4150",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 28639100,
            "range": "± 751540",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 224130,
            "range": "± 1158",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 390920,
            "range": "± 1085",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1782430,
            "range": "± 8749",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6924986,
            "range": "± 49630",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 26956972,
            "range": "± 78713",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1836220,
            "range": "± 18926",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 12044212,
            "range": "± 117798",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 12021750,
            "range": "± 187313",
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
            "value": 5848285,
            "range": "± 17664",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28523983,
            "range": "± 2427716",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 141320714,
            "range": "± 757470",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 144396301,
            "range": "± 661566",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 141441099,
            "range": "± 938651",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 139224439,
            "range": "± 363398",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 140851606,
            "range": "± 214270",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3668855,
            "range": "± 11108",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4716879,
            "range": "± 7068",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 144687278,
            "range": "± 926407",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 142407647,
            "range": "± 3193997",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 144259548,
            "range": "± 452588",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 393438,
            "range": "± 17105",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4715970,
            "range": "± 6723",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 139512238,
            "range": "± 508674",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 142635039,
            "range": "± 2590596",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 11982680,
            "range": "± 89020",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 256607,
            "range": "± 568",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 391056,
            "range": "± 393",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 351012,
            "range": "± 470",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 257761,
            "range": "± 222",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 220391,
            "range": "± 381",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 11961729,
            "range": "± 78576",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 12351382,
            "range": "± 32611",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 139217336,
            "range": "± 188469",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 142465437,
            "range": "± 787654",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 142152837,
            "range": "± 303977",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 145872832,
            "range": "± 106045",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 257382,
            "range": "± 679",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8361801,
            "range": "± 25459",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1473544475,
            "range": "± 7284483",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4859648,
            "range": "± 41180",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3493,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 22988917,
            "range": "± 170041",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 22988911,
            "range": "± 300304",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3440987,
            "range": "± 5730",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 24464101,
            "range": "± 618427",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 130150,
            "range": "± 388",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 161000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8724000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 53638000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 51616000,
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
          "id": "aee5450690c224375f27d6376a5790e55c432890",
          "message": "feat(cli): expose adaptive segmentation flags (#317)\n\nRefs #297",
          "timestamp": "2026-05-17T13:12:35+09:00",
          "tree_id": "b4d8112d6a83c1f4e9a0ae7cff28053ba0594879",
          "url": "https://github.com/matyushkin/djvu-rs/commit/aee5450690c224375f27d6376a5790e55c432890"
        },
        "date": 1778992017117,
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
            "value": 158598,
            "range": "± 1653",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 765690,
            "range": "± 4304",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 562270,
            "range": "± 4719",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1297083,
            "range": "± 45230",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2649,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9150388,
            "range": "± 19358",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 565193,
            "range": "± 17409",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2174261,
            "range": "± 6717",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2748265,
            "range": "± 10516",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 28376954,
            "range": "± 913825",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 229099,
            "range": "± 1939",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 374133,
            "range": "± 1273",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1604188,
            "range": "± 20712",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6200038,
            "range": "± 140453",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 24142388,
            "range": "± 68904",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1670607,
            "range": "± 11361",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 11538048,
            "range": "± 49335",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 11349104,
            "range": "± 243525",
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
            "value": 5278937,
            "range": "± 34312",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 26498691,
            "range": "± 344902",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 130777808,
            "range": "± 587479",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 133589864,
            "range": "± 218591",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 130680209,
            "range": "± 593580",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 127232399,
            "range": "± 184525",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 123325410,
            "range": "± 114731",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3370356,
            "range": "± 8386",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4507360,
            "range": "± 10136",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 133650735,
            "range": "± 265856",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 130093431,
            "range": "± 142861",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 126184664,
            "range": "± 180727",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 563721,
            "range": "± 1923",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4420381,
            "range": "± 189890",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 127235636,
            "range": "± 115499",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 130069029,
            "range": "± 154550",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 11184110,
            "range": "± 27738",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 240695,
            "range": "± 437",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 373261,
            "range": "± 432",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 351628,
            "range": "± 793",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 240778,
            "range": "± 167",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 215772,
            "range": "± 266",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 11126250,
            "range": "± 54067",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 12157290,
            "range": "± 16857",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 127350225,
            "range": "± 97335",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 125475390,
            "range": "± 106789",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 130175680,
            "range": "± 56369",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 128599853,
            "range": "± 280674",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 241351,
            "range": "± 1104",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8572186,
            "range": "± 27341",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1438340738,
            "range": "± 5786213",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4031970,
            "range": "± 31118",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3303,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 24185365,
            "range": "± 87221",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 24202197,
            "range": "± 123527",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3153354,
            "range": "± 7696",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22810553,
            "range": "± 75317",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 198728,
            "range": "± 953",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 164000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8230000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 48980000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 47231000,
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
          "id": "445ed11c2457bd563c10fd579052c93729b2d273",
          "message": "perf(pdf): stream RGB staging for color pages (#319)\n\nRefs #299",
          "timestamp": "2026-05-17T14:04:25+09:00",
          "tree_id": "9809ebc01b759f6a2f76c02d730e3b40463b26c4",
          "url": "https://github.com/matyushkin/djvu-rs/commit/445ed11c2457bd563c10fd579052c93729b2d273"
        },
        "date": 1778995126855,
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
            "value": 160878,
            "range": "± 2412",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 773248,
            "range": "± 1828",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 569961,
            "range": "± 1436",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1407683,
            "range": "± 19169",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2709,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9214575,
            "range": "± 37112",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 569616,
            "range": "± 1288",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2186043,
            "range": "± 17344",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2753142,
            "range": "± 4962",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 28502907,
            "range": "± 463317",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 227806,
            "range": "± 1460",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 373200,
            "range": "± 4261",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1600576,
            "range": "± 17457",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 6204851,
            "range": "± 7672",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 24163730,
            "range": "± 40929",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1685764,
            "range": "± 11033",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 11317520,
            "range": "± 88493",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 11299569,
            "range": "± 206623",
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
            "value": 5361050,
            "range": "± 70562",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 26953898,
            "range": "± 190542",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 129688990,
            "range": "± 183259",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 132653161,
            "range": "± 182212",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 129731828,
            "range": "± 244576",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 127401201,
            "range": "± 225138",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 122582754,
            "range": "± 261144",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3397651,
            "range": "± 9973",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4344086,
            "range": "± 15855",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 132410360,
            "range": "± 696123",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 130298771,
            "range": "± 112602",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 125357484,
            "range": "± 389753",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 566576,
            "range": "± 6806",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4338245,
            "range": "± 12868",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 127423842,
            "range": "± 124715",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 130335733,
            "range": "± 143441",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 11147257,
            "range": "± 45483",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 239138,
            "range": "± 407",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 370942,
            "range": "± 583",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 355323,
            "range": "± 972",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 239348,
            "range": "± 336",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 214182,
            "range": "± 284",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 11180984,
            "range": "± 26852",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 12821058,
            "range": "± 81358",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 127339733,
            "range": "± 54215",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 125996266,
            "range": "± 281204",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 130178384,
            "range": "± 94765",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 128780472,
            "range": "± 250011",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 239597,
            "range": "± 2187",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 8546527,
            "range": "± 20351",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1356734765,
            "range": "± 1865817",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 4316996,
            "range": "± 74565",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 3325,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 21091385,
            "range": "± 98621",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 21062167,
            "range": "± 93160",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3180229,
            "range": "± 10977",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22976067,
            "range": "± 52147",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 199286,
            "range": "± 2605",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 165000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8199000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 49535000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 47549000,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}