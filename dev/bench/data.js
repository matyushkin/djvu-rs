window.BENCHMARK_DATA = {
  "lastUpdate": 1783955494470,
  "repoUrl": "https://github.com/matyushkin/djvu-rs",
  "entries": {
    "djvu-rs benchmarks": [
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
          "id": "a5c2dd11e74e0323778e3400abb1b58eab504b80",
          "message": "Merge pull request #704 from matyushkin/issue-687-resolver\n\nfeat(document): add typed indirect component resolver",
          "timestamp": "2026-07-13T22:45:57+08:00",
          "tree_id": "9bcba3aa01620b4c9ba0f744dabf0b028fc6eee0",
          "url": "https://github.com/matyushkin/djvu-rs/commit/a5c2dd11e74e0323778e3400abb1b58eab504b80"
        },
        "date": 1783955493297,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 105,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "bzz_encode",
            "value": 167738,
            "range": "± 1449",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 162673,
            "range": "± 2437",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 776538,
            "range": "± 3426",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 592489,
            "range": "± 2497",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1310597,
            "range": "± 18839",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2446,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 10794139,
            "range": "± 135675",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3858483,
            "range": "± 23292",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2583651,
            "range": "± 57973",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 950805,
            "range": "± 4511",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 666933,
            "range": "± 1770",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 263250,
            "range": "± 954",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 8951852,
            "range": "± 149044",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 577432,
            "range": "± 1768",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2180143,
            "range": "± 3964",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3382356,
            "range": "± 10610",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 47165263,
            "range": "± 761075",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 160489,
            "range": "± 2095",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24181107,
            "range": "± 56186",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 8568684,
            "range": "± 33253",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 3001218,
            "range": "± 5287",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 13357055,
            "range": "± 101785",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6490306,
            "range": "± 16200",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 52403039,
            "range": "± 273803",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 936905498,
            "range": "± 2409771",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 18320492,
            "range": "± 15772",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 384427194,
            "range": "± 2879173",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 17695137439,
            "range": "± 431600810",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 16636880,
            "range": "± 184182",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 17347085,
            "range": "± 83087",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 172242,
            "range": "± 346",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 976063,
            "range": "± 3140",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4215722,
            "range": "± 30616",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 16901384,
            "range": "± 65441",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1010425,
            "range": "± 8326",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10226193,
            "range": "± 25384",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10197587,
            "range": "± 46971",
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
            "value": 5487236,
            "range": "± 4053",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 25914158,
            "range": "± 1014820",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 92288064,
            "range": "± 335256",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 84805526,
            "range": "± 215614",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4123600,
            "range": "± 21773",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 16477894,
            "range": "± 166432",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 84833168,
            "range": "± 209885",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 92259995,
            "range": "± 133613",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 88580079,
            "range": "± 104871",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 86554175,
            "range": "± 70728",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3489002,
            "range": "± 4693",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4457128,
            "range": "± 6828",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 84911506,
            "range": "± 182118",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 80857136,
            "range": "± 48976",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 81942847,
            "range": "± 240204",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 585811,
            "range": "± 1127",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4441063,
            "range": "± 9781",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 88536720,
            "range": "± 121170",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 80842755,
            "range": "± 154567",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10052977,
            "range": "± 17889",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 26615108,
            "range": "± 25549",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 83377,
            "range": "± 89",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 86231063,
            "range": "± 68968",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 170962,
            "range": "± 176",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 172876,
            "range": "± 141",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 83485,
            "range": "± 707",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 84498,
            "range": "± 193",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10056701,
            "range": "± 10203",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 10391263,
            "range": "± 49849",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 88547452,
            "range": "± 53960",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 88541829,
            "range": "± 51244",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 80822051,
            "range": "± 52415",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 83763565,
            "range": "± 85171",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 83994,
            "range": "± 239",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1158328,
            "range": "± 5981",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 21415111,
            "range": "± 55051",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 281665778,
            "range": "± 316805",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1164334531,
            "range": "± 2810567",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1655265949,
            "range": "± 2346549",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 2134446,
            "range": "± 56747",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2112,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 21261964,
            "range": "± 302802",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 7377037,
            "range": "± 186319",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 7586501,
            "range": "± 251323",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3282546,
            "range": "± 14429",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 24472758,
            "range": "± 474567",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6549,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 225510,
            "range": "± 8981",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 110725945,
            "range": "± 309933",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 13195036,
            "range": "± 89503",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 306243419,
            "range": "± 1498892",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 3014249,
            "range": "± 22730",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 33875271,
            "range": "± 299425",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 5887215,
            "range": "± 38635",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 89379601,
            "range": "± 329987",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 74551030,
            "range": "± 780174",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 77253003,
            "range": "± 234836",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 173877,
            "range": "± 1707",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 26834795,
            "range": "± 76917",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_72",
            "value": 164000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8598000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49997000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 48134000,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}