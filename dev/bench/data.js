window.BENCHMARK_DATA = {
  "lastUpdate": 1777631451347,
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
      }
    ]
  }
}