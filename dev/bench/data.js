window.BENCHMARK_DATA = {
  "lastUpdate": 1777552514553,
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
      }
    ]
  }
}