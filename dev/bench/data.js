window.BENCHMARK_DATA = {
  "lastUpdate": 1784220879212,
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
          "id": "5104fa35880a73d47a6b479811098695ab4a6f3c",
          "message": "Merge pull request #707 from matyushkin/codex/683-bm44-pm44\n\nfeat(document): decode legacy FORM:BM44 and FORM:PM44 (#683)",
          "timestamp": "2026-07-13T23:12:29+08:00",
          "tree_id": "e723e03bffe0091d020242c92a6e878f6647f50c",
          "url": "https://github.com/matyushkin/djvu-rs/commit/5104fa35880a73d47a6b479811098695ab4a6f3c"
        },
        "date": 1783958410226,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 105,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "bzz_encode",
            "value": 175305,
            "range": "± 838",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 163345,
            "range": "± 665",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 765630,
            "range": "± 1491",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 595402,
            "range": "± 6556",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1400251,
            "range": "± 43480",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2485,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 10741157,
            "range": "± 48094",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3884922,
            "range": "± 22481",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2611078,
            "range": "± 17193",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 952240,
            "range": "± 16909",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 675886,
            "range": "± 37023",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 245234,
            "range": "± 763",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9602638,
            "range": "± 52208",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 645742,
            "range": "± 1495",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2202195,
            "range": "± 14914",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3415813,
            "range": "± 17200",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 48474968,
            "range": "± 1613021",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 160564,
            "range": "± 2738",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24141763,
            "range": "± 56957",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 9825081,
            "range": "± 38212",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 3003326,
            "range": "± 61758",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 13441605,
            "range": "± 97309",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6618034,
            "range": "± 19741",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 53618504,
            "range": "± 339827",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 932323921,
            "range": "± 4817733",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 19177871,
            "range": "± 31423",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 434537038,
            "range": "± 2128892",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 17614453506,
            "range": "± 103938421",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 16729775,
            "range": "± 181995",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 16956043,
            "range": "± 45592",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 172708,
            "range": "± 1407",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 972739,
            "range": "± 3172",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4209935,
            "range": "± 28589",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 16805749,
            "range": "± 42413",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1005865,
            "range": "± 7642",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10215964,
            "range": "± 24990",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10181196,
            "range": "± 11387",
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
            "value": 5454247,
            "range": "± 7276",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 25910756,
            "range": "± 675921",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 89766122,
            "range": "± 225526",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 84047735,
            "range": "± 235539",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4106566,
            "range": "± 22407",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 16139495,
            "range": "± 62678",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 84074069,
            "range": "± 269273",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 89919271,
            "range": "± 180964",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 86432688,
            "range": "± 46655",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 86553190,
            "range": "± 319860",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3473690,
            "range": "± 4681",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4372293,
            "range": "± 36173",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 84186693,
            "range": "± 104253",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 80693505,
            "range": "± 2671889",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 81930782,
            "range": "± 71662",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 579679,
            "range": "± 3893",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4367377,
            "range": "± 13790",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 86461127,
            "range": "± 51662",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 80641326,
            "range": "± 151441",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10041405,
            "range": "± 11326",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 26541148,
            "range": "± 15925",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 83670,
            "range": "± 89",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 83374326,
            "range": "± 72986",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 171536,
            "range": "± 178",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 173344,
            "range": "± 230",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 83741,
            "range": "± 246",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 84635,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10051020,
            "range": "± 14004",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 10395593,
            "range": "± 10182",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 86358257,
            "range": "± 146586",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 88193939,
            "range": "± 73140",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 80598810,
            "range": "± 104306",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 83323004,
            "range": "± 59498",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 84325,
            "range": "± 570",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1163268,
            "range": "± 26692",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 21103179,
            "range": "± 73193",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 280086968,
            "range": "± 263336",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1171404354,
            "range": "± 3600975",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1657490091,
            "range": "± 7141296",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 1941116,
            "range": "± 33073",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2095,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 20017176,
            "range": "± 510844",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 6834584,
            "range": "± 237532",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 6697519,
            "range": "± 273642",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3280929,
            "range": "± 10936",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 24192494,
            "range": "± 215703",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6644,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 226699,
            "range": "± 10114",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 110781564,
            "range": "± 290378",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 13014125,
            "range": "± 289793",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 306507245,
            "range": "± 1547685",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2979612,
            "range": "± 22981",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 34107162,
            "range": "± 233808",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 5868668,
            "range": "± 134609",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 89198584,
            "range": "± 214996",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 74602724,
            "range": "± 137616",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 77299002,
            "range": "± 474444",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 176273,
            "range": "± 968",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 26718507,
            "range": "± 82298",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_72",
            "value": 166000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8205999,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49548000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 47514000,
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
          "id": "4fe8d25545c47c6a2cf7dacd912107a044bbc4c2",
          "message": "Merge pull request #713 from matyushkin/issue-689-component-graph\n\nfeat(document): add validated component dependency graph (#689)",
          "timestamp": "2026-07-16T22:49:21+08:00",
          "tree_id": "2c8b175a36c05cc7e25c08fc0776fecb38b7fa91",
          "url": "https://github.com/matyushkin/djvu-rs/commit/4fe8d25545c47c6a2cf7dacd912107a044bbc4c2"
        },
        "date": 1784214826757,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 96,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "bzz_encode",
            "value": 151860,
            "range": "± 5390",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 157897,
            "range": "± 7393",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 765785,
            "range": "± 35169",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 578230,
            "range": "± 27549",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1024767,
            "range": "± 35777",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2717,
            "range": "± 111",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 8266971,
            "range": "± 232523",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3111980,
            "range": "± 133743",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2040438,
            "range": "± 70159",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 767320,
            "range": "± 28772",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 559799,
            "range": "± 18294",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 199652,
            "range": "± 6709",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 7363869,
            "range": "± 398452",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 516975,
            "range": "± 12690",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 1827382,
            "range": "± 69010",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3023255,
            "range": "± 91403",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 45004774,
            "range": "± 1580563",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 126060,
            "range": "± 7572",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 18930906,
            "range": "± 420086",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 7552919,
            "range": "± 242588",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 2568500,
            "range": "± 117691",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 9380350,
            "range": "± 337990",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 5644946,
            "range": "± 205991",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 44812317,
            "range": "± 1009917",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 838044352,
            "range": "± 17052982",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 15227192,
            "range": "± 221424",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 321140112,
            "range": "± 2214253",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 15865005896,
            "range": "± 82518966",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 15432973,
            "range": "± 634790",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 12393497,
            "range": "± 320178",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 127946,
            "range": "± 6854",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 683743,
            "range": "± 23319",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 2982797,
            "range": "± 73098",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 12083737,
            "range": "± 548212",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 944583,
            "range": "± 36327",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 7811516,
            "range": "± 441798",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 7785929,
            "range": "± 106832",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/bg_only_warm",
            "value": 8,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/mask_decode",
            "value": 5463985,
            "range": "± 71192",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 23190656,
            "range": "± 773891",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 65394913,
            "range": "± 1825360",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 61112251,
            "range": "± 1453930",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 3106078,
            "range": "± 110875",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 12178805,
            "range": "± 329492",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 61351470,
            "range": "± 2281896",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 65542660,
            "range": "± 1751043",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 62298807,
            "range": "± 946193",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 61580545,
            "range": "± 2474707",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3436161,
            "range": "± 61389",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 3515403,
            "range": "± 122977",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 62285487,
            "range": "± 2858346",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 58927685,
            "range": "± 1460204",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 57570855,
            "range": "± 1893927",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 578644,
            "range": "± 23793",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 3479249,
            "range": "± 42520",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 63021290,
            "range": "± 677764",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 58676200,
            "range": "± 501521",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 7589422,
            "range": "± 57302",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 19845946,
            "range": "± 201023",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 62263,
            "range": "± 1060",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 59633815,
            "range": "± 808793",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 126531,
            "range": "± 1135",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 129165,
            "range": "± 852",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 62149,
            "range": "± 1833",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 63638,
            "range": "± 3244",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 7759876,
            "range": "± 88229",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 7781906,
            "range": "± 81500",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 62814112,
            "range": "± 829676",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 63841211,
            "range": "± 785025",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 59492002,
            "range": "± 2617941",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 60539891,
            "range": "± 742199",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 64760,
            "range": "± 2758",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 895307,
            "range": "± 39633",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 16062502,
            "range": "± 200380",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 214314458,
            "range": "± 3863462",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 913445896,
            "range": "± 13945546",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1344726728,
            "range": "± 20593757",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 2444535,
            "range": "± 58406",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 1246,
            "range": "± 84",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 15018051,
            "range": "± 793230",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 5338221,
            "range": "± 198150",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 6240232,
            "range": "± 193398",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3290334,
            "range": "± 177576",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 22550606,
            "range": "± 742590",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6199,
            "range": "± 251",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 249123,
            "range": "± 13283",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 110614190,
            "range": "± 2853806",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 12815747,
            "range": "± 417775",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 280022868,
            "range": "± 7748611",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2455750,
            "range": "± 59155",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 31597820,
            "range": "± 1497892",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 4437162,
            "range": "± 119610",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 60175689,
            "range": "± 1773499",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 53441844,
            "range": "± 3164457",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 53382775,
            "range": "± 1740619",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 253128,
            "range": "± 6523",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 20217727,
            "range": "± 1368604",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_72",
            "value": 147000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 6304000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 37147000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 35405000,
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
          "id": "33d24ad7362cf73fc0e9c652d06bd29b2ebb2b7d",
          "message": "Merge pull request #714 from matyushkin/issue-689-extract-closure\n\nfeat(djvm): extract-with-includes uses the component graph closure (#689)",
          "timestamp": "2026-07-16T23:21:18+08:00",
          "tree_id": "275bc7b773cf35a88c60c509ee4121a48243d731",
          "url": "https://github.com/matyushkin/djvu-rs/commit/33d24ad7362cf73fc0e9c652d06bd29b2ebb2b7d"
        },
        "date": 1784216819509,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 104,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "bzz_encode",
            "value": 170305,
            "range": "± 596",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 163154,
            "range": "± 498",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 779178,
            "range": "± 2305",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 590272,
            "range": "± 7305",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1420409,
            "range": "± 21019",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2402,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 10542826,
            "range": "± 93523",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3860405,
            "range": "± 16081",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2583655,
            "range": "± 49505",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 949760,
            "range": "± 3945",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 663697,
            "range": "± 9579",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 244723,
            "range": "± 2137",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 8952797,
            "range": "± 19788",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 571431,
            "range": "± 5715",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2166048,
            "range": "± 49292",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3427696,
            "range": "± 12111",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 50188310,
            "range": "± 1450086",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 160330,
            "range": "± 1753",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24149556,
            "range": "± 628337",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 8550540,
            "range": "± 25558",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 2994237,
            "range": "± 3729",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 13499900,
            "range": "± 65733",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6610774,
            "range": "± 13057",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 53223740,
            "range": "± 158918",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 929204625,
            "range": "± 715531",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 18767620,
            "range": "± 15658",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 382373392,
            "range": "± 1132781",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 17244656525,
            "range": "± 123927300",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 16446014,
            "range": "± 34383",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 17053302,
            "range": "± 78476",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 186132,
            "range": "± 506",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 975277,
            "range": "± 1630",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4227742,
            "range": "± 28388",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 16839233,
            "range": "± 40887",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1005933,
            "range": "± 16400",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10715104,
            "range": "± 18573",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10706078,
            "range": "± 10523",
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
            "value": 5454409,
            "range": "± 7857",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 26422883,
            "range": "± 262803",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 90087674,
            "range": "± 260516",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 84090584,
            "range": "± 303490",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4352504,
            "range": "± 9871",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 17217317,
            "range": "± 69759",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 84563286,
            "range": "± 400467",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 90332991,
            "range": "± 180668",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 86698691,
            "range": "± 86939",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 85632453,
            "range": "± 183766",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3473804,
            "range": "± 7886",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4460703,
            "range": "± 8280",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 84154899,
            "range": "± 135823",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 80858799,
            "range": "± 69120",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 80045520,
            "range": "± 137094",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 576623,
            "range": "± 1385",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4335246,
            "range": "± 10708",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 86687025,
            "range": "± 140519",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 80777917,
            "range": "± 92332",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10613350,
            "range": "± 13187",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 27531369,
            "range": "± 27571",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 90076,
            "range": "± 98",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 83950531,
            "range": "± 82906",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 184912,
            "range": "± 170",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 173366,
            "range": "± 233",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 90082,
            "range": "± 108",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 84443,
            "range": "± 67",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10623359,
            "range": "± 9497",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 10422576,
            "range": "± 14768",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 86735731,
            "range": "± 190243",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 87783546,
            "range": "± 179234",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 80891941,
            "range": "± 153680",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 81851674,
            "range": "± 68139",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 90881,
            "range": "± 142",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1170558,
            "range": "± 9773",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 22095482,
            "range": "± 46398",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 281750393,
            "range": "± 607614",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1178909645,
            "range": "± 1674944",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1665461540,
            "range": "± 2036482",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 1947579,
            "range": "± 59076",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2110,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 19771036,
            "range": "± 254814",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 6409510,
            "range": "± 68971",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 6633141,
            "range": "± 334787",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3250847,
            "range": "± 11665",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 23661324,
            "range": "± 84391",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6548,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 223154,
            "range": "± 15584",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 110753999,
            "range": "± 3720974",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 12902000,
            "range": "± 17471",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 292616201,
            "range": "± 350667",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2629897,
            "range": "± 4027",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 32601169,
            "range": "± 81994",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 6010736,
            "range": "± 14084",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 82631518,
            "range": "± 220343",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 70388260,
            "range": "± 136360",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 72089776,
            "range": "± 184542",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 169919,
            "range": "± 1954",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 26205253,
            "range": "± 60368",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_72",
            "value": 163000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8154000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49563000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 47277000,
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
          "id": "e28795cba705b4418edd82a98f34bbc2ad80754b",
          "message": "Merge pull request #715 from matyushkin/issue-689-single-page-incl\n\nfeat(djvm): bundle single-page extraction with its INCL closure (#689)",
          "timestamp": "2026-07-16T23:48:38+08:00",
          "tree_id": "bf740e0ef9e382f05d5273dca8b2b13acec8fb22",
          "url": "https://github.com/matyushkin/djvu-rs/commit/e28795cba705b4418edd82a98f34bbc2ad80754b"
        },
        "date": 1784218465590,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 93,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "bzz_encode",
            "value": 126060,
            "range": "± 664",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 169752,
            "range": "± 522",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 838796,
            "range": "± 2934",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 447183,
            "range": "± 22707",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1214479,
            "range": "± 18687",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2380,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 10867886,
            "range": "± 78068",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 4109821,
            "range": "± 40210",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2714861,
            "range": "± 32847",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 1040646,
            "range": "± 12487",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 670825,
            "range": "± 2770",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 254644,
            "range": "± 6581",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9642435,
            "range": "± 45373",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 611789,
            "range": "± 2087",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2359883,
            "range": "± 16137",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3533991,
            "range": "± 5577",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 59421889,
            "range": "± 3058732",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 150195,
            "range": "± 1641",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24181688,
            "range": "± 422424",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 9607409,
            "range": "± 80090",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 3200333,
            "range": "± 6991",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 13077490,
            "range": "± 82520",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6960594,
            "range": "± 18176",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 56184416,
            "range": "± 1663217",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 968593659,
            "range": "± 998285",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 19671253,
            "range": "± 21718",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 426698194,
            "range": "± 813645",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 19425377963,
            "range": "± 346316477",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 16742150,
            "range": "± 64339",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 18586665,
            "range": "± 37252",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 186559,
            "range": "± 516",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1065398,
            "range": "± 4739",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4613845,
            "range": "± 8385",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 18491005,
            "range": "± 40524",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1097118,
            "range": "± 16116",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10959676,
            "range": "± 23954",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10964500,
            "range": "± 520528",
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
            "value": 6040473,
            "range": "± 5658",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28293476,
            "range": "± 154725",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 97530973,
            "range": "± 2876421",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 91761108,
            "range": "± 141665",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4433683,
            "range": "± 20037",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 17431134,
            "range": "± 161650",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 91578913,
            "range": "± 2726949",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 97519635,
            "range": "± 90327",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 94291041,
            "range": "± 198154",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 92594542,
            "range": "± 183270",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3786124,
            "range": "± 11553",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4742279,
            "range": "± 14391",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 91555494,
            "range": "± 614028",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 88359428,
            "range": "± 50030",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 86413517,
            "range": "± 89819",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 406987,
            "range": "± 4753",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4737119,
            "range": "± 7169",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 94409500,
            "range": "± 168575",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 88558987,
            "range": "± 104758",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10862913,
            "range": "± 10339",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 28694973,
            "range": "± 22813",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 90281,
            "range": "± 1838",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 91437996,
            "range": "± 107805",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 185776,
            "range": "± 759",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 188076,
            "range": "± 196",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 90197,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 90928,
            "range": "± 74",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10848683,
            "range": "± 69388",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 10933586,
            "range": "± 24741",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 94387483,
            "range": "± 155319",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 94650146,
            "range": "± 196198",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 88492793,
            "range": "± 65128",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 88790152,
            "range": "± 348226",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 90919,
            "range": "± 1073",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1282291,
            "range": "± 5526",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 22675265,
            "range": "± 243416",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 309799035,
            "range": "± 6238759",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1248337229,
            "range": "± 4645300",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1740717598,
            "range": "± 8854211",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 1926027,
            "range": "± 51282",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2211,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 16726304,
            "range": "± 131698",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 6126029,
            "range": "± 69974",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 6238088,
            "range": "± 77822",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3565304,
            "range": "± 100560",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 25790186,
            "range": "± 187133",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6943,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 149252,
            "range": "± 13613",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 121192695,
            "range": "± 1717780",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 13910041,
            "range": "± 31960",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 325464600,
            "range": "± 2701965",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2792777,
            "range": "± 11404",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 35111347,
            "range": "± 970525",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 6390200,
            "range": "± 18053",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 90964426,
            "range": "± 657123",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 77098444,
            "range": "± 389641",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 79255318,
            "range": "± 154091",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 144637,
            "range": "± 518",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 28597509,
            "range": "± 80963",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_72",
            "value": 161000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8706000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 53186000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 51160000,
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
          "id": "d69d6800b4126e11b1e80662f62b2e1c4130ede1",
          "message": "Merge pull request #716 from matyushkin/issue-689-bundled-to-indirect\n\nfeat(djvm): add bundled -> indirect conversion preserving graph and names (#689)",
          "timestamp": "2026-07-17T00:28:56+08:00",
          "tree_id": "5093645c746086fe6d080ca4d2869e2e94ca6e32",
          "url": "https://github.com/matyushkin/djvu-rs/commit/d69d6800b4126e11b1e80662f62b2e1c4130ede1"
        },
        "date": 1784220878728,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 93,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "bzz_encode",
            "value": 127766,
            "range": "± 2710",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 169033,
            "range": "± 1284",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 839927,
            "range": "± 5085",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 432821,
            "range": "± 22979",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1192274,
            "range": "± 9123",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2400,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 10756558,
            "range": "± 358351",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 4101427,
            "range": "± 17581",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2603670,
            "range": "± 21394",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 1002851,
            "range": "± 5464",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 667012,
            "range": "± 2039",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 251983,
            "range": "± 788",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 10375994,
            "range": "± 178368",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 733220,
            "range": "± 2133",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2351795,
            "range": "± 13323",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3511886,
            "range": "± 10608",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 53166829,
            "range": "± 2702536",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 150219,
            "range": "± 557",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24214127,
            "range": "± 514734",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 9512964,
            "range": "± 23149",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 3197695,
            "range": "± 7114",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 12970126,
            "range": "± 99272",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6945557,
            "range": "± 20019",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 56226291,
            "range": "± 204231",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 967857546,
            "range": "± 981042",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 19614493,
            "range": "± 281988",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 422378695,
            "range": "± 513645",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 19008183081,
            "range": "± 29395282",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 16900842,
            "range": "± 301434",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 18609267,
            "range": "± 58028",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 186602,
            "range": "± 2510",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1066369,
            "range": "± 18009",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4621323,
            "range": "± 7194",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 18470184,
            "range": "± 55613",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1093655,
            "range": "± 18059",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10938866,
            "range": "± 24760",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 11151454,
            "range": "± 13994",
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
            "value": 6017760,
            "range": "± 14842",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 27918558,
            "range": "± 320913",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 97583439,
            "range": "± 3309059",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 91629762,
            "range": "± 100200",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4438579,
            "range": "± 13616",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 17430456,
            "range": "± 61230",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 91735910,
            "range": "± 211765",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 97708350,
            "range": "± 117077",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 94503672,
            "range": "± 510261",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 92891221,
            "range": "± 2299203",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3780541,
            "range": "± 4993",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4729955,
            "range": "± 5545",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 91588005,
            "range": "± 1990002",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 88500227,
            "range": "± 69552",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 86534079,
            "range": "± 131161",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 397801,
            "range": "± 4912",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4729437,
            "range": "± 4771",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 94283552,
            "range": "± 63989",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 88286349,
            "range": "± 80348",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10858244,
            "range": "± 11519",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 28678402,
            "range": "± 34436",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 90213,
            "range": "± 94",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 91163647,
            "range": "± 95818",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 185400,
            "range": "± 214",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 188055,
            "range": "± 617",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 90309,
            "range": "± 245",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 91125,
            "range": "± 129",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10849642,
            "range": "± 13519",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 10935691,
            "range": "± 19765",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 94303442,
            "range": "± 74304",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 94636320,
            "range": "± 2940592",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 88469744,
            "range": "± 721353",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 88518125,
            "range": "± 174538",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 90824,
            "range": "± 582",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1279735,
            "range": "± 6799",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 22713157,
            "range": "± 76242",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 309999282,
            "range": "± 2939483",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1247700419,
            "range": "± 5299143",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1739899850,
            "range": "± 1511776",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 2088244,
            "range": "± 139546",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2211,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 17637334,
            "range": "± 1162195",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 7084637,
            "range": "± 269109",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 7235400,
            "range": "± 373390",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3553862,
            "range": "± 9659",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 25752237,
            "range": "± 621928",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 7031,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 150370,
            "range": "± 12050",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 121154392,
            "range": "± 258035",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 13885978,
            "range": "± 34301",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 325344088,
            "range": "± 4593216",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2807504,
            "range": "± 61643",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 34818984,
            "range": "± 194694",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 6295810,
            "range": "± 114346",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 90905380,
            "range": "± 157926",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 77006538,
            "range": "± 156052",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 79225458,
            "range": "± 1653456",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 144842,
            "range": "± 509",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 28653657,
            "range": "± 423713",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_72",
            "value": 160000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8763000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 53621000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 51588000,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}