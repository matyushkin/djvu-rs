window.BENCHMARK_DATA = {
  "lastUpdate": 1783956901798,
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
          "id": "d0666fdea2e6cc9cc99950c12412495996d8d5b2",
          "message": "Merge pull request #706 from matyushkin/codex/682-conformance-dashboard\n\nfeat: publish DjVu conformance dashboard (#682)",
          "timestamp": "2026-07-13T22:58:25+08:00",
          "tree_id": "040a5384a330f8f2cc0d3d4f82ed79c1545900c9",
          "url": "https://github.com/matyushkin/djvu-rs/commit/d0666fdea2e6cc9cc99950c12412495996d8d5b2"
        },
        "date": 1783956900503,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 73,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "bzz_encode",
            "value": 103199,
            "range": "± 505",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 131646,
            "range": "± 803",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 643206,
            "range": "± 1885",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 377708,
            "range": "± 7657",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1002698,
            "range": "± 19994",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 1887,
            "range": "± 105",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 8271461,
            "range": "± 227752",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3185263,
            "range": "± 15856",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2012772,
            "range": "± 55261",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 780422,
            "range": "± 13050",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 524640,
            "range": "± 3360",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 199109,
            "range": "± 11956",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 7501736,
            "range": "± 18085",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 470584,
            "range": "± 16380",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 1827015,
            "range": "± 42899",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2688032,
            "range": "± 6879",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 37424426,
            "range": "± 1340100",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 116433,
            "range": "± 3528",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 18752042,
            "range": "± 382369",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 8392884,
            "range": "± 230403",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 2478011,
            "range": "± 62727",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 10032153,
            "range": "± 87156",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 5456011,
            "range": "± 94920",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 44631078,
            "range": "± 368095",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 762802306,
            "range": "± 1009317",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 15897722,
            "range": "± 203266",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 370020282,
            "range": "± 4879446",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 15082716914,
            "range": "± 131979440",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 13363417,
            "range": "± 316440",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 14410188,
            "range": "± 261383",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 144690,
            "range": "± 2786",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 826807,
            "range": "± 19828",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 3590099,
            "range": "± 10450",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 14377791,
            "range": "± 430917",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 848486,
            "range": "± 19142",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 8433979,
            "range": "± 254799",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 8432515,
            "range": "± 90791",
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
            "value": 4674245,
            "range": "± 7264",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 21700549,
            "range": "± 118736",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 75565739,
            "range": "± 377265",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 71070753,
            "range": "± 1785236",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 3418964,
            "range": "± 67971",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 13782842,
            "range": "± 79232",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 71289693,
            "range": "± 2264799",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 75791492,
            "range": "± 118604",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 73079174,
            "range": "± 1325499",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 71756793,
            "range": "± 226343",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 2927701,
            "range": "± 14233",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 3663434,
            "range": "± 4632",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 71270894,
            "range": "± 165700",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 68471551,
            "range": "± 532957",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 66969717,
            "range": "± 77077",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 302309,
            "range": "± 4067",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 3664966,
            "range": "± 3006",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 72902803,
            "range": "± 55931",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 68302652,
            "range": "± 216387",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 8375820,
            "range": "± 9307",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 22529841,
            "range": "± 16875",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 69870,
            "range": "± 80",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 70839085,
            "range": "± 1421479",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 143899,
            "range": "± 134",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 146319,
            "range": "± 589",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 69968,
            "range": "± 1164",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 70606,
            "range": "± 1483",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 8380009,
            "range": "± 35335",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 8521796,
            "range": "± 25112",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 72908246,
            "range": "± 38131",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 73506642,
            "range": "± 98168",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 68312056,
            "range": "± 509932",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 68760070,
            "range": "± 490142",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 70455,
            "range": "± 72",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1037659,
            "range": "± 8577",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 17900066,
            "range": "± 133209",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 247859633,
            "range": "± 851491",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 970129638,
            "range": "± 5567843",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1348291950,
            "range": "± 5586142",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 1849085,
            "range": "± 150827",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 1713,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 14109156,
            "range": "± 176138",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 5270469,
            "range": "± 40002",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 5180862,
            "range": "± 470502",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 2770612,
            "range": "± 89111",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 20348200,
            "range": "± 418192",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 5527,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 120366,
            "range": "± 16044",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 94389671,
            "range": "± 1609364",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 11046523,
            "range": "± 240516",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 257949242,
            "range": "± 4470708",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2496017,
            "range": "± 7830",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 29415763,
            "range": "± 278443",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 4889811,
            "range": "± 8644",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 70789027,
            "range": "± 219964",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 60196907,
            "range": "± 86027",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 61810702,
            "range": "± 187024",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 118102,
            "range": "± 634",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 22237283,
            "range": "± 65781",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_72",
            "value": 124000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 6877000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 42425000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 40585000,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}