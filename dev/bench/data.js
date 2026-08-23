window.BENCHMARK_DATA = {
  "lastUpdate": 1787496413128,
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
          "id": "0d0fc92ac4ec35068155195be445e3d9af1b9e79",
          "message": "Merge pull request #717 from matyushkin/issue-689-component-dedup\n\nfeat(djvm): safe byte-exact shared-component deduplication (#689)",
          "timestamp": "2026-07-17T01:20:17+08:00",
          "tree_id": "7edad7937099ee566bed6d69e7b7a8f53056bbb2",
          "url": "https://github.com/matyushkin/djvu-rs/commit/0d0fc92ac4ec35068155195be445e3d9af1b9e79"
        },
        "date": 1784223909252,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 104,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "bzz_encode",
            "value": 169310,
            "range": "± 704",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 163137,
            "range": "± 900",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 780479,
            "range": "± 12749",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 585586,
            "range": "± 4838",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1381249,
            "range": "± 20837",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2451,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 10629946,
            "range": "± 94672",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3877571,
            "range": "± 68384",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2577576,
            "range": "± 16724",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 950666,
            "range": "± 1618",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 660637,
            "range": "± 2151",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 244516,
            "range": "± 1129",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 8957825,
            "range": "± 104476",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 572134,
            "range": "± 24869",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2188940,
            "range": "± 15094",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3421303,
            "range": "± 16079",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 50360472,
            "range": "± 1275225",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 160488,
            "range": "± 2007",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24175219,
            "range": "± 904646",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 8558736,
            "range": "± 43029",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 2992126,
            "range": "± 8620",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 13482732,
            "range": "± 71872",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6603363,
            "range": "± 26062",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 53436448,
            "range": "± 640398",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 934473259,
            "range": "± 1040806",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 18777839,
            "range": "± 28700",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 385598731,
            "range": "± 907435",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 17210092158,
            "range": "± 39855168",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 16385026,
            "range": "± 94178",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 17004829,
            "range": "± 70755",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 186155,
            "range": "± 4843",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 974319,
            "range": "± 3332",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4220155,
            "range": "± 26963",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 16865412,
            "range": "± 82193",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1005189,
            "range": "± 11214",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10705019,
            "range": "± 27925",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10712765,
            "range": "± 167836",
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
            "value": 5454689,
            "range": "± 5461",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 26662256,
            "range": "± 386214",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 90138944,
            "range": "± 750595",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 84342819,
            "range": "± 2734222",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4350797,
            "range": "± 13782",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 17162090,
            "range": "± 49624",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 84277457,
            "range": "± 544077",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 90190597,
            "range": "± 470661",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 86658036,
            "range": "± 45524",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 85745735,
            "range": "± 110689",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3474057,
            "range": "± 4113",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4341292,
            "range": "± 7795",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 84292616,
            "range": "± 136541",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 80764179,
            "range": "± 159750",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 80224150,
            "range": "± 57278",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 576812,
            "range": "± 1312",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4346574,
            "range": "± 74435",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 86632131,
            "range": "± 179468",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 80805185,
            "range": "± 546327",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10627447,
            "range": "± 220596",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 27533338,
            "range": "± 20369",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 90535,
            "range": "± 648",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 83865120,
            "range": "± 99568",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 185034,
            "range": "± 212",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 173581,
            "range": "± 152",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 90135,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 84465,
            "range": "± 93",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10620693,
            "range": "± 7920",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 10430438,
            "range": "± 167138",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 86648415,
            "range": "± 930914",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 87764710,
            "range": "± 75250",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 80748363,
            "range": "± 50823",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 81998718,
            "range": "± 150452",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 90842,
            "range": "± 126",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1170857,
            "range": "± 6675",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 22030558,
            "range": "± 22474",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 281566352,
            "range": "± 482064",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1178715945,
            "range": "± 4941517",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1665904633,
            "range": "± 5968241",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 2006687,
            "range": "± 22995",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2114,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 21008104,
            "range": "± 317595",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 7799877,
            "range": "± 130993",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 7600225,
            "range": "± 141869",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3252404,
            "range": "± 123577",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 23686236,
            "range": "± 72957",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6594,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 224188,
            "range": "± 7196",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 110861015,
            "range": "± 796816",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 12957732,
            "range": "± 34262",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 293042146,
            "range": "± 3169159",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2634421,
            "range": "± 7478",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 33109257,
            "range": "± 173697",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 6028202,
            "range": "± 40948",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 83136614,
            "range": "± 260769",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 70691942,
            "range": "± 144770",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 72342653,
            "range": "± 1104279",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 171794,
            "range": "± 1416",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 26412550,
            "range": "± 113899",
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
            "value": 8212000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49616000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 47686000,
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
          "id": "fee8e0689a30fbace1b847cc55677191c7e65ba4",
          "message": "Merge pull request #718 from matyushkin/issue-689-page-delete-gc\n\nfeat(djvm): page deletion with unreachable-component GC policy (#689)",
          "timestamp": "2026-07-17T02:07:11+08:00",
          "tree_id": "4f23aab34c9faa373f9a5ac84462da7cd0e502b0",
          "url": "https://github.com/matyushkin/djvu-rs/commit/fee8e0689a30fbace1b847cc55677191c7e65ba4"
        },
        "date": 1784226825484,
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
            "value": 127029,
            "range": "± 616",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 168895,
            "range": "± 775",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 835626,
            "range": "± 8190",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 415459,
            "range": "± 16976",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1206147,
            "range": "± 7447",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2428,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 11026720,
            "range": "± 96086",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 4126334,
            "range": "± 16122",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2946362,
            "range": "± 64804",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 1004763,
            "range": "± 23168",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 666665,
            "range": "± 1734",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 255085,
            "range": "± 1107",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9855537,
            "range": "± 94167",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 610150,
            "range": "± 3694",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2364440,
            "range": "± 11130",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3505712,
            "range": "± 9838",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 67711712,
            "range": "± 2406799",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 150296,
            "range": "± 271",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24241814,
            "range": "± 43645",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 9597779,
            "range": "± 46376",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 3195113,
            "range": "± 7876",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 13192064,
            "range": "± 62853",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6962501,
            "range": "± 32985",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 56223040,
            "range": "± 142455",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 968844230,
            "range": "± 1233515",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 19716901,
            "range": "± 275335",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 426666281,
            "range": "± 657994",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 19993899684,
            "range": "± 243694511",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 16890826,
            "range": "± 90137",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 18629801,
            "range": "± 133634",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 186677,
            "range": "± 1050",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1067486,
            "range": "± 4971",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4617153,
            "range": "± 6713",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 18432468,
            "range": "± 46204",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1094834,
            "range": "± 5100",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10934309,
            "range": "± 177780",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10930231,
            "range": "± 94504",
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
            "value": 6016953,
            "range": "± 8428",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28195204,
            "range": "± 125128",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 97699122,
            "range": "± 1699321",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 91714450,
            "range": "± 656201",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4433814,
            "range": "± 6846",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 17533894,
            "range": "± 40249",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 91517897,
            "range": "± 171796",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 97370390,
            "range": "± 190777",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 94233707,
            "range": "± 45182",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 92473867,
            "range": "± 102048",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3780722,
            "range": "± 3255",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4729200,
            "range": "± 115109",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 91504351,
            "range": "± 107090",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 88311821,
            "range": "± 49332",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 86433480,
            "range": "± 156367",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 393750,
            "range": "± 6725",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4720766,
            "range": "± 9316",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 94426392,
            "range": "± 58869",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 88411653,
            "range": "± 78186",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10853113,
            "range": "± 8114",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 28694050,
            "range": "± 12861",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 90156,
            "range": "± 145",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 91477735,
            "range": "± 103397",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 185368,
            "range": "± 311",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 187866,
            "range": "± 248",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 90425,
            "range": "± 265",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 91429,
            "range": "± 2505",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10850409,
            "range": "± 18707",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 10951580,
            "range": "± 56550",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 94466391,
            "range": "± 228215",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 94712710,
            "range": "± 427432",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 88449416,
            "range": "± 92088",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 88628358,
            "range": "± 212304",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 90861,
            "range": "± 196",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1285711,
            "range": "± 14796",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 22705628,
            "range": "± 31600",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 309401410,
            "range": "± 354566",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1247478188,
            "range": "± 3037868",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1739141312,
            "range": "± 1361430",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 1854912,
            "range": "± 96983",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2210,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 16679040,
            "range": "± 204919",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 6042219,
            "range": "± 77282",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 6057117,
            "range": "± 85436",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3558484,
            "range": "± 66435",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 25846546,
            "range": "± 161487",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6934,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 149800,
            "range": "± 4748",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 121260250,
            "range": "± 2184925",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 13887370,
            "range": "± 84210",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 326017015,
            "range": "± 1876728",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2795140,
            "range": "± 10861",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 34795523,
            "range": "± 175735",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 6297560,
            "range": "± 24653",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 90967116,
            "range": "± 831824",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 76936111,
            "range": "± 184883",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 79151302,
            "range": "± 235870",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 150801,
            "range": "± 342",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 28805965,
            "range": "± 124123",
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
            "value": 8783000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 53584000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 51491000,
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
          "id": "e6e7250f51157b19b0d9af4f5e7d872610a47188",
          "message": "Merge pull request #720 from matyushkin/issue-690-zip-streaming\n\nfeat(export): bound CBZ/EPUB writer memory to active page state (#690)",
          "timestamp": "2026-07-19T01:47:49+08:00",
          "tree_id": "8614e2cb0ad319dcf1b15226c81dc3417dfb2b6e",
          "url": "https://github.com/matyushkin/djvu-rs/commit/e6e7250f51157b19b0d9af4f5e7d872610a47188"
        },
        "date": 1784398362531,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 104,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "bzz_encode",
            "value": 175547,
            "range": "± 430",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 162171,
            "range": "± 423",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 779948,
            "range": "± 2012",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 589033,
            "range": "± 7129",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1402230,
            "range": "± 19724",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2434,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 10496449,
            "range": "± 87337",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3840978,
            "range": "± 35306",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2585502,
            "range": "± 13977",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 952652,
            "range": "± 5840",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 662094,
            "range": "± 7880",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 244365,
            "range": "± 748",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 8987137,
            "range": "± 17429",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 572534,
            "range": "± 1338",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2175347,
            "range": "± 57091",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3429754,
            "range": "± 11923",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 50742578,
            "range": "± 1476445",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 160695,
            "range": "± 2402",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24152394,
            "range": "± 149676",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 8595408,
            "range": "± 47603",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 2994337,
            "range": "± 4431",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 13471748,
            "range": "± 69074",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6629807,
            "range": "± 18864",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 53317767,
            "range": "± 243678",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 933297866,
            "range": "± 4365629",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 18744132,
            "range": "± 39322",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 381802029,
            "range": "± 961898",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 17058645974,
            "range": "± 10166424",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 16569811,
            "range": "± 69775",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 16980493,
            "range": "± 83725",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 172109,
            "range": "± 1187",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 972784,
            "range": "± 2486",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4205661,
            "range": "± 13593",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 16793407,
            "range": "± 60928",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1024491,
            "range": "± 16786",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10183894,
            "range": "± 14665",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10183765,
            "range": "± 42883",
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
            "value": 5444748,
            "range": "± 37371",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 25667282,
            "range": "± 133943",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 91911262,
            "range": "± 231301",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 84143029,
            "range": "± 444547",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4108373,
            "range": "± 7668",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 16390866,
            "range": "± 70186",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 84119814,
            "range": "± 235761",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 92014588,
            "range": "± 469820",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 88567706,
            "range": "± 168934",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 85969964,
            "range": "± 66787",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3457458,
            "range": "± 6535",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4534204,
            "range": "± 5090",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 84308164,
            "range": "± 118125",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 80873201,
            "range": "± 49895",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 80220410,
            "range": "± 199664",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 574308,
            "range": "± 532",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4526630,
            "range": "± 4442",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 88599935,
            "range": "± 40450",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 80887137,
            "range": "± 22122",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10049599,
            "range": "± 12307",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 26507527,
            "range": "± 77675",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 83405,
            "range": "± 250",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 85962513,
            "range": "± 218503",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 170887,
            "range": "± 114",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 176862,
            "range": "± 474",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 83311,
            "range": "± 58",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 86185,
            "range": "± 190",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10049962,
            "range": "± 11097",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 10149713,
            "range": "± 11556",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 88610883,
            "range": "± 72658",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 87691001,
            "range": "± 222199",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 80889978,
            "range": "± 40675",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 81923632,
            "range": "± 101321",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 84141,
            "range": "± 1726",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1153830,
            "range": "± 12762",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 21217594,
            "range": "± 33944",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 281023775,
            "range": "± 677702",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1158958078,
            "range": "± 3311467",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1647359568,
            "range": "± 749957",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 2037588,
            "range": "± 44973",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2112,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 20505304,
            "range": "± 399667",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 6678576,
            "range": "± 101643",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 6813914,
            "range": "± 263035",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3258686,
            "range": "± 28619",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 24202277,
            "range": "± 160048",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6618,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 220741,
            "range": "± 12759",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 110999485,
            "range": "± 364195",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 12835713,
            "range": "± 92331",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 304342898,
            "range": "± 1563818",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2602715,
            "range": "± 16335",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 33396929,
            "range": "± 200210",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 5852598,
            "range": "± 108342",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 89272542,
            "range": "± 873571",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 74171722,
            "range": "± 240493",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 76843557,
            "range": "± 343743",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 159431,
            "range": "± 1006",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 26943454,
            "range": "± 170977",
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
            "value": 8249000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49488000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 47631000,
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
          "id": "1c6835c42c2f59757838bbc6b0d00e99c7644393",
          "message": "Merge pull request #722 from matyushkin/issue-690-progress-cancel\n\nfeat(export): uniform progress/cancellation observer and atomic CLI output (#690)",
          "timestamp": "2026-07-19T02:52:27+08:00",
          "tree_id": "17e6c0565c5155eb93a323160399588ded67ce1e",
          "url": "https://github.com/matyushkin/djvu-rs/commit/1c6835c42c2f59757838bbc6b0d00e99c7644393"
        },
        "date": 1784402258862,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 104,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "bzz_encode",
            "value": 167599,
            "range": "± 1115",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 172222,
            "range": "± 1655",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 777951,
            "range": "± 6320",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 586355,
            "range": "± 6584",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1419574,
            "range": "± 23336",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2476,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 10546622,
            "range": "± 62367",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3866646,
            "range": "± 63016",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2604116,
            "range": "± 74952",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 958653,
            "range": "± 3096",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 668816,
            "range": "± 6525",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 244229,
            "range": "± 1716",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 8940293,
            "range": "± 42370",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 570019,
            "range": "± 2051",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2177146,
            "range": "± 37640",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3442525,
            "range": "± 8770",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 51343702,
            "range": "± 1413229",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 160610,
            "range": "± 6376",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24214299,
            "range": "± 379658",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 8565114,
            "range": "± 35089",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 2992299,
            "range": "± 5055",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 13506407,
            "range": "± 332157",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6606051,
            "range": "± 30478",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 53447838,
            "range": "± 381973",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 932401843,
            "range": "± 793029",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 18737438,
            "range": "± 25399",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 383758120,
            "range": "± 846643",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 17092599577,
            "range": "± 31765550",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 16543436,
            "range": "± 50145",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 16979003,
            "range": "± 170805",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 172043,
            "range": "± 557",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 973127,
            "range": "± 3665",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4208207,
            "range": "± 62388",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 16793515,
            "range": "± 26677",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1024526,
            "range": "± 30996",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10181019,
            "range": "± 28129",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10204738,
            "range": "± 8717",
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
            "value": 5439030,
            "range": "± 70191",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 25816117,
            "range": "± 110486",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 91921634,
            "range": "± 316749",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 84168409,
            "range": "± 256180",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4101576,
            "range": "± 7214",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 16404421,
            "range": "± 40206",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 84374174,
            "range": "± 1140901",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 92187382,
            "range": "± 349653",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 88597588,
            "range": "± 91826",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 85810171,
            "range": "± 35308",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3456152,
            "range": "± 3459",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4442616,
            "range": "± 5327",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 84324400,
            "range": "± 274747",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 80829032,
            "range": "± 233984",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 80462052,
            "range": "± 34354",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 572308,
            "range": "± 374",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4438993,
            "range": "± 13644",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 88502483,
            "range": "± 1150810",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 80862447,
            "range": "± 69601",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10061508,
            "range": "± 13351",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 26575869,
            "range": "± 20171",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 83331,
            "range": "± 69",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 85908508,
            "range": "± 60820",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 171500,
            "range": "± 88",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 172695,
            "range": "± 135",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 83263,
            "range": "± 121",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 84273,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10039444,
            "range": "± 23182",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 10143135,
            "range": "± 9954",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 88561441,
            "range": "± 104943",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 87750945,
            "range": "± 1279066",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 80913549,
            "range": "± 175635",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 82201918,
            "range": "± 93342",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 84085,
            "range": "± 146",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1154673,
            "range": "± 11397",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 21238633,
            "range": "± 40080",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 280852157,
            "range": "± 571212",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1157311011,
            "range": "± 2078732",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1651992778,
            "range": "± 1030275",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 1888288,
            "range": "± 16442",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2112,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 19717344,
            "range": "± 191348",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 6771735,
            "range": "± 95098",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 6617385,
            "range": "± 96905",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3266351,
            "range": "± 4705",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 24199933,
            "range": "± 68538",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6547,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 221848,
            "range": "± 8774",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 110722979,
            "range": "± 529634",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 12808341,
            "range": "± 39634",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 303566508,
            "range": "± 2012598",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2594707,
            "range": "± 22117",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 33143478,
            "range": "± 101546",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 5817204,
            "range": "± 24454",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 89222109,
            "range": "± 1253053",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 74170470,
            "range": "± 204266",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 76869476,
            "range": "± 128698",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 172768,
            "range": "± 1140",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 26381964,
            "range": "± 432452",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_72",
            "value": 165000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8228999,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49322000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49935000,
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
          "id": "2db57c6373fda92218aefaf7e3d755d07ec42abb",
          "message": "Merge pull request #723 from matyushkin/issue-690-tiff-g4-twopass\n\nfeat(tiff): two-pass bilevel G4 export bounds memory to one page (#690)",
          "timestamp": "2026-07-19T03:43:40+08:00",
          "tree_id": "c70e6ddd8cd55dab86fdfcf65dac7fb63a1fef5c",
          "url": "https://github.com/matyushkin/djvu-rs/commit/2db57c6373fda92218aefaf7e3d755d07ec42abb"
        },
        "date": 1784405238137,
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
            "value": 152729,
            "range": "± 1076",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 153608,
            "range": "± 1523",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 737593,
            "range": "± 8310",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 559063,
            "range": "± 4106",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 976946,
            "range": "± 7738",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2570,
            "range": "± 86",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 7923616,
            "range": "± 31749",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 2999090,
            "range": "± 12292",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 1966073,
            "range": "± 9180",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 736655,
            "range": "± 2250",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 552191,
            "range": "± 2749",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 192410,
            "range": "± 1249",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 7149538,
            "range": "± 25462",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 503220,
            "range": "± 2139",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 1763502,
            "range": "± 25978",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2941177,
            "range": "± 15841",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 43784065,
            "range": "± 168837",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 121066,
            "range": "± 785",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 18280791,
            "range": "± 115814",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 7289499,
            "range": "± 162433",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 2486434,
            "range": "± 51000",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 9100249,
            "range": "± 56828",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 5465748,
            "range": "± 84262",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 43173826,
            "range": "± 655227",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 798668375,
            "range": "± 4289541",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 14833302,
            "range": "± 172563",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 311652567,
            "range": "± 336821",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 15168534051,
            "range": "± 26427324",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 15197797,
            "range": "± 52007",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 11943101,
            "range": "± 197071",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 125059,
            "range": "± 797",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 666736,
            "range": "± 3830",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 2905185,
            "range": "± 49588",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 11752217,
            "range": "± 69002",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 950064,
            "range": "± 4228",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 7544575,
            "range": "± 246300",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 7567960,
            "range": "± 44139",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/bg_only_warm",
            "value": 7,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/mask_decode",
            "value": 5239476,
            "range": "± 98830",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 22408898,
            "range": "± 172908",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 62893625,
            "range": "± 459979",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 58806265,
            "range": "± 1684109",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 3045652,
            "range": "± 71074",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 12006389,
            "range": "± 210486",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 58916900,
            "range": "± 1158323",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 62543801,
            "range": "± 323661",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 59708618,
            "range": "± 121690",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 59803165,
            "range": "± 65127",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3293719,
            "range": "± 4640",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 3963305,
            "range": "± 59552",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 58553989,
            "range": "± 135557",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 55994875,
            "range": "± 186772",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 55527416,
            "range": "± 312632",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 557196,
            "range": "± 773",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 3935320,
            "range": "± 6751",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 59522624,
            "range": "± 69299",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 55753800,
            "range": "± 227992",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 7389787,
            "range": "± 35646",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 19244974,
            "range": "± 14639",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 60079,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 56994547,
            "range": "± 937516",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 123131,
            "range": "± 2142",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 124830,
            "range": "± 498",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 60128,
            "range": "± 847",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 60562,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 7390115,
            "range": "± 8203",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 7506152,
            "range": "± 10156",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 59588644,
            "range": "± 148193",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 59880410,
            "range": "± 486958",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 55743556,
            "range": "± 92818",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 55000478,
            "range": "± 943043",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 60984,
            "range": "± 373",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 851518,
            "range": "± 6619",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 15355228,
            "range": "± 43349",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 200661260,
            "range": "± 434582",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 870383888,
            "range": "± 7464979",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1278496909,
            "range": "± 5706413",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 2365521,
            "range": "± 17970",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 1252,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 14454093,
            "range": "± 105892",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 5208358,
            "range": "± 25284",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 5993527,
            "range": "± 24377",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3140850,
            "range": "± 40026",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 21847977,
            "range": "± 115337",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6039,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 238843,
            "range": "± 6524",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 108413243,
            "range": "± 2069682",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 12368198,
            "range": "± 102655",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 275845325,
            "range": "± 2840531",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2401130,
            "range": "± 18059",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 30518112,
            "range": "± 956956",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 4357570,
            "range": "± 36321",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 57683555,
            "range": "± 409955",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 51031928,
            "range": "± 272582",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 51149440,
            "range": "± 745316",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 238730,
            "range": "± 1232",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 19284912,
            "range": "± 390493",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_72",
            "value": 125000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 6136000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 36123000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 33981000,
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
          "id": "51a099ba2ac6c4ef7bfbe31a729998a570cef577",
          "message": "Merge pull request #724 from matyushkin/issue-690-djvm-spooled\n\nfeat(djvm): spooled streaming bundle writer with bounded memory (#690)",
          "timestamp": "2026-07-19T04:24:38+08:00",
          "tree_id": "d1bc17acbb4cb5d48529f88773f59c4b2c95c23d",
          "url": "https://github.com/matyushkin/djvu-rs/commit/51a099ba2ac6c4ef7bfbe31a729998a570cef577"
        },
        "date": 1784407841477,
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
            "value": 126297,
            "range": "± 1445",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 169458,
            "range": "± 1010",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 832979,
            "range": "± 2713",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 417731,
            "range": "± 28646",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1254189,
            "range": "± 39429",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2367,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 10966808,
            "range": "± 579294",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 4111206,
            "range": "± 7746",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2687584,
            "range": "± 115137",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 1021790,
            "range": "± 9514",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 675522,
            "range": "± 2341",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 255864,
            "range": "± 6550",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9720134,
            "range": "± 29693",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 606702,
            "range": "± 13846",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2415858,
            "range": "± 20150",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3506521,
            "range": "± 12651",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 56382228,
            "range": "± 1571849",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 150393,
            "range": "± 6664",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24212480,
            "range": "± 73863",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 9544140,
            "range": "± 32060",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 3195518,
            "range": "± 186556",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 13110357,
            "range": "± 42025",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6954889,
            "range": "± 252026",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 56197438,
            "range": "± 356280",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 967461736,
            "range": "± 2243497",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 19638698,
            "range": "± 163755",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 425950798,
            "range": "± 5695754",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 19753360133,
            "range": "± 41156636",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 16850305,
            "range": "± 492892",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 18515272,
            "range": "± 140282",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 186651,
            "range": "± 769",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1067660,
            "range": "± 11301",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4623863,
            "range": "± 7416",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 18518992,
            "range": "± 22372",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1097328,
            "range": "± 19873",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10888168,
            "range": "± 39554",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10906143,
            "range": "± 187130",
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
            "value": 5997674,
            "range": "± 4686",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28270475,
            "range": "± 1087190",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 97242844,
            "range": "± 2228016",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 91426129,
            "range": "± 1664943",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4412801,
            "range": "± 17666",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 17835738,
            "range": "± 121800",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 91464492,
            "range": "± 219593",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 97223204,
            "range": "± 189017",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 93981201,
            "range": "± 48059",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 92798445,
            "range": "± 281157",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3762101,
            "range": "± 24289",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4736408,
            "range": "± 7044",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 91491002,
            "range": "± 70621",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 88240833,
            "range": "± 49249",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 86341001,
            "range": "± 2624734",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 403837,
            "range": "± 11983",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4729663,
            "range": "± 3966",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 94107465,
            "range": "± 57814",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 88251392,
            "range": "± 226672",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10804282,
            "range": "± 176555",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 29127647,
            "range": "± 260030",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 90193,
            "range": "± 1206",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 91382189,
            "range": "± 416327",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 185260,
            "range": "± 220",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 202694,
            "range": "± 4830",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 90237,
            "range": "± 97",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 98867,
            "range": "± 88",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10801427,
            "range": "± 11242",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 11692212,
            "range": "± 29020",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 94123586,
            "range": "± 124882",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 94602856,
            "range": "± 49755",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 88284600,
            "range": "± 55906",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 88448706,
            "range": "± 531874",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 92021,
            "range": "± 708",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1286194,
            "range": "± 13730",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 23319737,
            "range": "± 185919",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 310099511,
            "range": "± 159320",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1255144471,
            "range": "± 7011369",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1741379957,
            "range": "± 6997780",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 2190393,
            "range": "± 29898",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2208,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 17074164,
            "range": "± 1274238",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 6252299,
            "range": "± 71366",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 6339866,
            "range": "± 443097",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3585363,
            "range": "± 11633",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 26256239,
            "range": "± 106565",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6995,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 151211,
            "range": "± 8444",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 121757611,
            "range": "± 207284",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 13895098,
            "range": "± 67859",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 329903347,
            "range": "± 2652579",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2780598,
            "range": "± 12002",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 35919945,
            "range": "± 237191",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 6303353,
            "range": "± 28317",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 90907748,
            "range": "± 3166674",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 76886916,
            "range": "± 172333",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 79075065,
            "range": "± 337681",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 145415,
            "range": "± 1979",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 28759816,
            "range": "± 1191404",
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
            "value": 8787000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 53663000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 51396000,
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
          "id": "120649205511268555697aba475e24714d24012c",
          "message": "Merge pull request #725 from matyushkin/issue-690-async-failure-tests\n\nfeat(export): async writer adapters, injected-failure tests, memory smoke (#690)",
          "timestamp": "2026-07-19T05:42:04+08:00",
          "tree_id": "e69689a8ace8d51286e0dce9b3316843b523d21d",
          "url": "https://github.com/matyushkin/djvu-rs/commit/120649205511268555697aba475e24714d24012c"
        },
        "date": 1784412465252,
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
            "value": 171467,
            "range": "± 677",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 162441,
            "range": "± 917",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 778678,
            "range": "± 1371",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 595627,
            "range": "± 5919",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1342206,
            "range": "± 38010",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2404,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 11345145,
            "range": "± 63866",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3943268,
            "range": "± 22914",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2999855,
            "range": "± 45046",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 962043,
            "range": "± 16219",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 671317,
            "range": "± 14669",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 246605,
            "range": "± 867",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9848149,
            "range": "± 101706",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 666432,
            "range": "± 30188",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2571878,
            "range": "± 89173",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3449341,
            "range": "± 96651",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 73989685,
            "range": "± 3137324",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 160207,
            "range": "± 1492",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24450329,
            "range": "± 686748",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 8851014,
            "range": "± 295973",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 3001105,
            "range": "± 71562",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 14202843,
            "range": "± 283732",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6784478,
            "range": "± 38603",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 54216453,
            "range": "± 162053",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 937484041,
            "range": "± 1246213",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 19133070,
            "range": "± 642902",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 395901294,
            "range": "± 4208021",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 18704682829,
            "range": "± 48918853",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 18901220,
            "range": "± 394819",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 17362451,
            "range": "± 468217",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 174150,
            "range": "± 620",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 976876,
            "range": "± 28680",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4233732,
            "range": "± 32416",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 17026533,
            "range": "± 39647",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1030413,
            "range": "± 8293",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10366878,
            "range": "± 29606",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10377148,
            "range": "± 54058",
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
            "value": 5546413,
            "range": "± 17026",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 27198648,
            "range": "± 244386",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 92458285,
            "range": "± 392859",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 84709544,
            "range": "± 406271",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4132501,
            "range": "± 14082",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 16679815,
            "range": "± 324227",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 84828497,
            "range": "± 302056",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 92673738,
            "range": "± 1132481",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 88692217,
            "range": "± 262524",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 85981675,
            "range": "± 304350",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3486297,
            "range": "± 14329",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4602572,
            "range": "± 57195",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 84957122,
            "range": "± 128642",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 80813259,
            "range": "± 50254",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 80103549,
            "range": "± 39071",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 579210,
            "range": "± 1471",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4609146,
            "range": "± 11208",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 88621735,
            "range": "± 130451",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 80904854,
            "range": "± 841920",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10072946,
            "range": "± 275984",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 26613596,
            "range": "± 46015",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 83328,
            "range": "± 134",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 86272673,
            "range": "± 200796",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 170748,
            "range": "± 238",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 173168,
            "range": "± 4090",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 83343,
            "range": "± 119",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 84351,
            "range": "± 139",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10062111,
            "range": "± 7868",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 10342091,
            "range": "± 19326",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 88570141,
            "range": "± 64979",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 87831076,
            "range": "± 83540",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 80856442,
            "range": "± 308357",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 82131124,
            "range": "± 101164",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 83985,
            "range": "± 2585",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1160548,
            "range": "± 22191",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 21623822,
            "range": "± 15744",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 281763955,
            "range": "± 2464537",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1163412209,
            "range": "± 3891274",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1655308342,
            "range": "± 5583211",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 2293175,
            "range": "± 101739",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2111,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 21551281,
            "range": "± 719375",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 8642771,
            "range": "± 299457",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 8472148,
            "range": "± 365914",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3309784,
            "range": "± 10286",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 25577749,
            "range": "± 841921",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6591,
            "range": "± 74",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 229042,
            "range": "± 19790",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 112588375,
            "range": "± 1993555",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 12900679,
            "range": "± 58114",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 313324103,
            "range": "± 3365227",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2638325,
            "range": "± 16621",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 36052962,
            "range": "± 371208",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 5982512,
            "range": "± 262908",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 89797664,
            "range": "± 536861",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 74754465,
            "range": "± 678091",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 77505911,
            "range": "± 929815",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 159858,
            "range": "± 2973",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 26938737,
            "range": "± 434260",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_72",
            "value": 167000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8813000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 50151000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 48097000,
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
          "id": "779865c7b098de48ffc798a620256a90e61b22b6",
          "message": "Merge pull request #726 from matyushkin/issue-696-inspect\n\nfeat(cli): add `djvu inspect` with offset-aware structural JSON (#696)",
          "timestamp": "2026-07-19T06:50:31+08:00",
          "tree_id": "8495048dc6235d3045fa4520041851959c21ff37",
          "url": "https://github.com/matyushkin/djvu-rs/commit/779865c7b098de48ffc798a620256a90e61b22b6"
        },
        "date": 1784416545482,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 105,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "bzz_encode",
            "value": 174923,
            "range": "± 1044",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 162539,
            "range": "± 2488",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 760064,
            "range": "± 4074",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 585950,
            "range": "± 2008",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1380378,
            "range": "± 14077",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2390,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 10944152,
            "range": "± 93997",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3926071,
            "range": "± 18528",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2646382,
            "range": "± 27180",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 965134,
            "range": "± 4707",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 674373,
            "range": "± 2176",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 247821,
            "range": "± 2810",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9107643,
            "range": "± 86753",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 577202,
            "range": "± 7818",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2219138,
            "range": "± 28806",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3455434,
            "range": "± 41242",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 55312197,
            "range": "± 1855780",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 164407,
            "range": "± 15324",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24205000,
            "range": "± 58381",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 9821823,
            "range": "± 28014",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 3142748,
            "range": "± 9930",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 13644426,
            "range": "± 85791",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6733826,
            "range": "± 35792",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 55069043,
            "range": "± 548256",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 938690306,
            "range": "± 1309469",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 19771864,
            "range": "± 32493",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 438916425,
            "range": "± 1369755",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 17948426896,
            "range": "± 25327037",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 16714954,
            "range": "± 58360",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 17170145,
            "range": "± 206394",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 186172,
            "range": "± 314",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 976727,
            "range": "± 1592",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4225646,
            "range": "± 7275",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 16911947,
            "range": "± 75269",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1006992,
            "range": "± 19118",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10737902,
            "range": "± 56033",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10738769,
            "range": "± 10056",
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
            "value": 5458607,
            "range": "± 7257",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 26995053,
            "range": "± 190441",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 90097333,
            "range": "± 214711",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 84246489,
            "range": "± 529750",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4350716,
            "range": "± 17944",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 17336627,
            "range": "± 157890",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 84324146,
            "range": "± 166060",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 90192722,
            "range": "± 75178",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 86614805,
            "range": "± 67226",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 85615858,
            "range": "± 203318",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3461383,
            "range": "± 4038",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4612973,
            "range": "± 4981",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 84475168,
            "range": "± 97572",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 80823811,
            "range": "± 191072",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 80170892,
            "range": "± 1172488",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 574438,
            "range": "± 1666",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4368575,
            "range": "± 7112",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 86612339,
            "range": "± 218381",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 80754562,
            "range": "± 59991",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10625543,
            "range": "± 17693",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 27578951,
            "range": "± 17621",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 90198,
            "range": "± 354",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 83841856,
            "range": "± 187781",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 185954,
            "range": "± 952",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 173289,
            "range": "± 189",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 90262,
            "range": "± 463",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 84622,
            "range": "± 89",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10638215,
            "range": "± 68318",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 10634246,
            "range": "± 26284",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 86586263,
            "range": "± 76922",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 87686748,
            "range": "± 75359",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 80784550,
            "range": "± 175164",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 81892087,
            "range": "± 30574",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 90836,
            "range": "± 218",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1169221,
            "range": "± 8002",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 22221791,
            "range": "± 46799",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 281221238,
            "range": "± 366133",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1180149770,
            "range": "± 1038550",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1682235545,
            "range": "± 838340",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 1980955,
            "range": "± 12098",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2112,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 21104872,
            "range": "± 192805",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 7920925,
            "range": "± 122507",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 7608088,
            "range": "± 91866",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3253903,
            "range": "± 13687",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 23822737,
            "range": "± 156433",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6704,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 224066,
            "range": "± 10461",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 111010683,
            "range": "± 422291",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 12955247,
            "range": "± 33786",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 294956705,
            "range": "± 665226",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2644207,
            "range": "± 26157",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 33452942,
            "range": "± 184905",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 6004748,
            "range": "± 11570",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 84715665,
            "range": "± 351574",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 71827353,
            "range": "± 979297",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 73716255,
            "range": "± 123004",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 179957,
            "range": "± 1172",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 27524109,
            "range": "± 84356",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_72",
            "value": 165000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8177000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49584000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
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
            "name": "Leo Matyushkin",
            "username": "matyushkin"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "583f66ab0fe4edce7068f7b42d566e51f73c4629",
          "message": "Merge pull request #727 from matyushkin/issue-696-validate\n\nfeat(cli): layered `djvu validate` with structural/dependency/codec findings (#696)",
          "timestamp": "2026-07-19T08:04:34+08:00",
          "tree_id": "3247295060cb6ec46abe9994f4cbde1b1277d3c7",
          "url": "https://github.com/matyushkin/djvu-rs/commit/583f66ab0fe4edce7068f7b42d566e51f73c4629"
        },
        "date": 1784420971169,
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
            "value": 169363,
            "range": "± 4310",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 161469,
            "range": "± 884",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 761074,
            "range": "± 1860",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 583278,
            "range": "± 1431",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1410156,
            "range": "± 12065",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2432,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 10768572,
            "range": "± 37328",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3875701,
            "range": "± 20712",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2607185,
            "range": "± 10816",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 955640,
            "range": "± 2663",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 665576,
            "range": "± 2025",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 243257,
            "range": "± 655",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9015907,
            "range": "± 39459",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 571988,
            "range": "± 1571",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2190492,
            "range": "± 6153",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3442439,
            "range": "± 12573",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 50364328,
            "range": "± 1077931",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 161048,
            "range": "± 782",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24114204,
            "range": "± 354673",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 9784788,
            "range": "± 44121",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 3138470,
            "range": "± 59606",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 13401778,
            "range": "± 53577",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6712191,
            "range": "± 28611",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 54505355,
            "range": "± 280032",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 940134763,
            "range": "± 871915",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 19550162,
            "range": "± 92890",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 431615790,
            "range": "± 428108",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 17315120775,
            "range": "± 28808532",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 16589302,
            "range": "± 95593",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 16941305,
            "range": "± 40802",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 178599,
            "range": "± 265",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 972730,
            "range": "± 2323",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4208333,
            "range": "± 5556",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 16800660,
            "range": "± 22520",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1003489,
            "range": "± 20234",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10706018,
            "range": "± 20403",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10700128,
            "range": "± 17995",
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
            "value": 5471359,
            "range": "± 5124",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 26536413,
            "range": "± 250264",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 89103164,
            "range": "± 1497990",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 84181558,
            "range": "± 142578",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4342987,
            "range": "± 20369",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 16588927,
            "range": "± 129164",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 84173987,
            "range": "± 229035",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 89306690,
            "range": "± 116378",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 85695863,
            "range": "± 101565",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 86697797,
            "range": "± 157631",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3476681,
            "range": "± 2993",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4352674,
            "range": "± 4391",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 84012695,
            "range": "± 86929",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 80507716,
            "range": "± 23100",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 81840974,
            "range": "± 71078",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 576805,
            "range": "± 814",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4359205,
            "range": "± 15612",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 85666728,
            "range": "± 122824",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 80577000,
            "range": "± 61927",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10597464,
            "range": "± 8315",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 27031981,
            "range": "± 39289",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 86468,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 82989068,
            "range": "± 29748",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 177262,
            "range": "± 113",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 179317,
            "range": "± 1097",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 86463,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 87377,
            "range": "± 60",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10589751,
            "range": "± 19200",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 10772109,
            "range": "± 10441",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 85668105,
            "range": "± 43612",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 88315956,
            "range": "± 29426",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 80489121,
            "range": "± 27347",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 83416514,
            "range": "± 43468",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 87196,
            "range": "± 1468",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1217614,
            "range": "± 9861",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 21514570,
            "range": "± 22021",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 289550261,
            "range": "± 643527",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1180421984,
            "range": "± 1003268",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1699881513,
            "range": "± 1550726",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 1983626,
            "range": "± 36776",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2110,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 20518253,
            "range": "± 131537",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 7468209,
            "range": "± 154526",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 7476896,
            "range": "± 90274",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3249947,
            "range": "± 4945",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 23703329,
            "range": "± 56884",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6920,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 227247,
            "range": "± 9120",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 110665322,
            "range": "± 311732",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 12903004,
            "range": "± 19318",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 295142983,
            "range": "± 291565",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2633897,
            "range": "± 5739",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 32795251,
            "range": "± 123890",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 5965277,
            "range": "± 14547",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 83978501,
            "range": "± 161524",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 71926350,
            "range": "± 150576",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 73392932,
            "range": "± 130846",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 173199,
            "range": "± 1218",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 27203781,
            "range": "± 72339",
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
            "value": 8212000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49336000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 47174000,
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
          "id": "616f2fe00ef3aef74cb7c81702cdd3faacea49c0",
          "message": "Merge pull request #728 from matyushkin/issue-696-semantic-diff\n\nfeat(cli): add `djvu diff` semantic document comparison (#696)",
          "timestamp": "2026-07-19T08:38:03+08:00",
          "tree_id": "023d7a4ddbe85a5730425088f3926857ba4bb16b",
          "url": "https://github.com/matyushkin/djvu-rs/commit/616f2fe00ef3aef74cb7c81702cdd3faacea49c0"
        },
        "date": 1784422830842,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 80,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "bzz_encode",
            "value": 124922,
            "range": "± 292",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 137272,
            "range": "± 9009",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 616988,
            "range": "± 1586",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 461386,
            "range": "± 2472",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1113230,
            "range": "± 46890",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2086,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 7922449,
            "range": "± 546636",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 2623505,
            "range": "± 102531",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 1737941,
            "range": "± 7739",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 639846,
            "range": "± 18910",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 496243,
            "range": "± 1705",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 167545,
            "range": "± 489",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 6144442,
            "range": "± 441430",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 448779,
            "range": "± 868",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 1546274,
            "range": "± 49165",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2444155,
            "range": "± 3760",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 38316165,
            "range": "± 357691",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 104926,
            "range": "± 303",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 16251379,
            "range": "± 1243142",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 6222534,
            "range": "± 25556",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 1878473,
            "range": "± 94019",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 7279304,
            "range": "± 31331",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 4377140,
            "range": "± 24972",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 35216426,
            "range": "± 1243055",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 762695071,
            "range": "± 16248011",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 12281161,
            "range": "± 73360",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 306632234,
            "range": "± 22381586",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 13542449018,
            "range": "± 133308109",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 12936302,
            "range": "± 83219",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 10588347,
            "range": "± 623035",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 105884,
            "range": "± 1120",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 585013,
            "range": "± 24924",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 2527880,
            "range": "± 208104",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 9814846,
            "range": "± 26390",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 776004,
            "range": "± 53438",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 6698246,
            "range": "± 218813",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 6672190,
            "range": "± 32560",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/bg_only_warm",
            "value": 7,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/mask_decode",
            "value": 4489425,
            "range": "± 5101",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 19997499,
            "range": "± 1021031",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 53036770,
            "range": "± 2814478",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 48834925,
            "range": "± 2610110",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 2682218,
            "range": "± 83712",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 9918815,
            "range": "± 476972",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 48711877,
            "range": "± 494020",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 53300899,
            "range": "± 2512178",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 49718966,
            "range": "± 60866",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 49457309,
            "range": "± 83894",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 2806080,
            "range": "± 54283",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 2852411,
            "range": "± 4781",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 49673877,
            "range": "± 133845",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 46681241,
            "range": "± 116852",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 46415966,
            "range": "± 58594",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 463049,
            "range": "± 1394",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 2851851,
            "range": "± 6366",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 50022480,
            "range": "± 340686",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 52569795,
            "range": "± 4020197",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 6517085,
            "range": "± 34564",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 15911792,
            "range": "± 21630",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 50503,
            "range": "± 240",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 52263462,
            "range": "± 4098457",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 103637,
            "range": "± 609",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 103910,
            "range": "± 338",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 58719,
            "range": "± 3161",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 53603,
            "range": "± 4122",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 6534413,
            "range": "± 26043",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 6697404,
            "range": "± 47232",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 49731303,
            "range": "± 153355",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 60312154,
            "range": "± 2191623",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 47385164,
            "range": "± 5488108",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 48217728,
            "range": "± 462913",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 51760,
            "range": "± 2827",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 840960,
            "range": "± 58739",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 12738285,
            "range": "± 47609",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 200892828,
            "range": "± 1737930",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 744527319,
            "range": "± 32158180",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1076208561,
            "range": "± 2985179",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 2400200,
            "range": "± 9145",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 979,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 15067241,
            "range": "± 826963",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 4859534,
            "range": "± 156303",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 4995131,
            "range": "± 87520",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 2639189,
            "range": "± 21832",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 18269650,
            "range": "± 57709",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 5014,
            "range": "± 399",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 197024,
            "range": "± 9961",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 91453982,
            "range": "± 7723843",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 10463575,
            "range": "± 28082",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 228399374,
            "range": "± 18778779",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 1996418,
            "range": "± 90227",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 30523795,
            "range": "± 1215173",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 3856486,
            "range": "± 28571",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 47738210,
            "range": "± 485355",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 42277851,
            "range": "± 116230",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 42499738,
            "range": "± 3019250",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 229707,
            "range": "± 9730",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 15990575,
            "range": "± 69449",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_72",
            "value": 106000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 5191000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 30604000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 28912000,
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
          "id": "01d2852b22508e76bf68817994ec3c75a88d2efd",
          "message": "Merge pull request #729 from matyushkin/issue-696-writer-validate-fuzz\n\nfeat(validate): writer pre-commit validation and validator fuzzing (#696)",
          "timestamp": "2026-07-19T09:06:04+08:00",
          "tree_id": "e30d2da3dcd45f94a891fcb9560a857d2a886202",
          "url": "https://github.com/matyushkin/djvu-rs/commit/01d2852b22508e76bf68817994ec3c75a88d2efd"
        },
        "date": 1784424689036,
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
            "value": 169971,
            "range": "± 721",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 160164,
            "range": "± 964",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 765872,
            "range": "± 24883",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 588992,
            "range": "± 5462",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1289567,
            "range": "± 9865",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2401,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 10632564,
            "range": "± 112562",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3896478,
            "range": "± 41431",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2574629,
            "range": "± 35220",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 954152,
            "range": "± 3543",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 669087,
            "range": "± 3396",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 246292,
            "range": "± 2373",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9410463,
            "range": "± 35189",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 575050,
            "range": "± 11131",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2257763,
            "range": "± 60585",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3440274,
            "range": "± 26080",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 50129727,
            "range": "± 1675489",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 160547,
            "range": "± 466",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24149870,
            "range": "± 736700",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 10136286,
            "range": "± 38036",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 3006062,
            "range": "± 77648",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 13574929,
            "range": "± 144700",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6636930,
            "range": "± 44584",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 53807405,
            "range": "± 297115",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 941314142,
            "range": "± 8520049",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 19418875,
            "range": "± 32900",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 446381130,
            "range": "± 1608539",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 17736832800,
            "range": "± 51416309",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 16730445,
            "range": "± 62630",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 17002640,
            "range": "± 118732",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 174154,
            "range": "± 1378",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 973193,
            "range": "± 8200",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4209423,
            "range": "± 16245",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 16815162,
            "range": "± 93798",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1041638,
            "range": "± 6771",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10139364,
            "range": "± 93984",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10145578,
            "range": "± 8624",
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
            "value": 5452711,
            "range": "± 8884",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 25841073,
            "range": "± 250687",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 89945762,
            "range": "± 3005651",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 83964814,
            "range": "± 1302470",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4121093,
            "range": "± 6605",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 16144431,
            "range": "± 285198",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 83793204,
            "range": "± 253929",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 89871560,
            "range": "± 294164",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 86341902,
            "range": "± 834649",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 86993245,
            "range": "± 127314",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3481501,
            "range": "± 9533",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4368085,
            "range": "± 75133",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 83890058,
            "range": "± 149195",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 80529865,
            "range": "± 73524",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 80581669,
            "range": "± 69625",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 575619,
            "range": "± 1364",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4498142,
            "range": "± 101912",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 86327646,
            "range": "± 170164",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 80579799,
            "range": "± 678169",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10096353,
            "range": "± 4726",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 26485759,
            "range": "± 30680",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 83772,
            "range": "± 248",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 83439754,
            "range": "± 97124",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 171500,
            "range": "± 430",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 173410,
            "range": "± 219",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 83789,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 84796,
            "range": "± 123",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10093390,
            "range": "± 21147",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 10211367,
            "range": "± 26651",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 86370068,
            "range": "± 345161",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 88897772,
            "range": "± 64516",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 80605243,
            "range": "± 275774",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 82574186,
            "range": "± 43908",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 84651,
            "range": "± 415",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1163067,
            "range": "± 35641",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 20981831,
            "range": "± 51789",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 280342968,
            "range": "± 1879333",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1166759934,
            "range": "± 3613666",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1653615984,
            "range": "± 6531218",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 1934471,
            "range": "± 37025",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2273,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 20058594,
            "range": "± 371869",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 6656254,
            "range": "± 440586",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 7004940,
            "range": "± 251448",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3242894,
            "range": "± 9984",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 23602366,
            "range": "± 119979",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6542,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 224729,
            "range": "± 9510",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 110150103,
            "range": "± 623657",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 12809443,
            "range": "± 92403",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 303166307,
            "range": "± 700037",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2595368,
            "range": "± 16991",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 33414615,
            "range": "± 787803",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 5829625,
            "range": "± 30917",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 89593728,
            "range": "± 488085",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 74446478,
            "range": "± 1040307",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 76939800,
            "range": "± 1109099",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 173851,
            "range": "± 1237",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 26764471,
            "range": "± 89862",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_72",
            "value": 165000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8212999,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49382000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 47759000,
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
          "id": "49952687d9942e0023fab319c6ca2329adf8357d",
          "message": "Merge pull request #731 from matyushkin/issue-696-sanitizer-fixes\n\nfix: two 32-bit overflow bugs found by the new sanitizer CI (#696)",
          "timestamp": "2026-07-19T18:36:48+08:00",
          "tree_id": "035cf748b55b599fb61fbb3ddeead3b2c4275e97",
          "url": "https://github.com/matyushkin/djvu-rs/commit/49952687d9942e0023fab319c6ca2329adf8357d"
        },
        "date": 1784458925184,
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
            "value": 173488,
            "range": "± 1301",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 163702,
            "range": "± 761",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 765061,
            "range": "± 12332",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 588507,
            "range": "± 3463",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1364449,
            "range": "± 30171",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2406,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 10550752,
            "range": "± 40982",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3877148,
            "range": "± 132066",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2585694,
            "range": "± 14299",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 950978,
            "range": "± 15886",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 666339,
            "range": "± 1995",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 244443,
            "range": "± 5633",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9048389,
            "range": "± 22848",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 576259,
            "range": "± 2240",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2192980,
            "range": "± 6546",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3433030,
            "range": "± 102732",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 49855541,
            "range": "± 810008",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 160449,
            "range": "± 6053",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24437787,
            "range": "± 44721",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 10114870,
            "range": "± 266872",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 3004231,
            "range": "± 8795",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 13382979,
            "range": "± 374988",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6624845,
            "range": "± 43943",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 53660382,
            "range": "± 450609",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 931187916,
            "range": "± 1372614",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 19417855,
            "range": "± 177144",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 443341960,
            "range": "± 1172809",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 17650425921,
            "range": "± 27006083",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 16548313,
            "range": "± 53733",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 16929810,
            "range": "± 63668",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 172844,
            "range": "± 944",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 972747,
            "range": "± 12476",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4208665,
            "range": "± 13860",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 16792136,
            "range": "± 23331",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1010557,
            "range": "± 13054",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10182976,
            "range": "± 28093",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10178679,
            "range": "± 42036",
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
            "value": 5485663,
            "range": "± 11473",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 25806774,
            "range": "± 506203",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 89494644,
            "range": "± 360305",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 83756404,
            "range": "± 1727667",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4102393,
            "range": "± 12749",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 16037353,
            "range": "± 90947",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 83674587,
            "range": "± 266495",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 89548300,
            "range": "± 125695",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 86435660,
            "range": "± 179872",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 85878325,
            "range": "± 2678423",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3477793,
            "range": "± 4509",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4348316,
            "range": "± 4450",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 83775289,
            "range": "± 166567",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 80546217,
            "range": "± 25862",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 80076221,
            "range": "± 907824",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 578532,
            "range": "± 750",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4350458,
            "range": "± 6338",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 86413098,
            "range": "± 78498",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 80646673,
            "range": "± 61654",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10044246,
            "range": "± 11482",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 26521195,
            "range": "± 53913",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 83610,
            "range": "± 108",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 83166292,
            "range": "± 309003",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 171601,
            "range": "± 207",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 173621,
            "range": "± 1061",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 83616,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 84698,
            "range": "± 693",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10051651,
            "range": "± 11308",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 10176035,
            "range": "± 32150",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 86318763,
            "range": "± 81339",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 87370631,
            "range": "± 77649",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 80564086,
            "range": "± 39288",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 81864386,
            "range": "± 78143",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 84448,
            "range": "± 2363",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1161771,
            "range": "± 32832",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 21047452,
            "range": "± 30472",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 279948758,
            "range": "± 547364",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1164835665,
            "range": "± 3521413",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1649410779,
            "range": "± 1717526",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 1842439,
            "range": "± 37755",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2272,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 19134936,
            "range": "± 222199",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 6491340,
            "range": "± 65955",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 6467976,
            "range": "± 94754",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3234674,
            "range": "± 12197",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 23581087,
            "range": "± 126372",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6592,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 223641,
            "range": "± 24462",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 109775699,
            "range": "± 398282",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 12759374,
            "range": "± 40526",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 302050240,
            "range": "± 3181068",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2580026,
            "range": "± 77720",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 32812826,
            "range": "± 73461",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 5851855,
            "range": "± 8601",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 88972399,
            "range": "± 1928795",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 74099678,
            "range": "± 952911",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 76796645,
            "range": "± 338344",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 175610,
            "range": "± 2146",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 26431994,
            "range": "± 101856",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_72",
            "value": 165000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8240000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 52349000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 47341000,
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
          "id": "ec8ea74ff084118cf5ddafd618dbee946db51638",
          "message": "Merge pull request #732 from matyushkin/issue-696-miri-scope\n\nci(miri): scope Miri to fast container crates so it completes (#696)",
          "timestamp": "2026-07-19T20:20:54+08:00",
          "tree_id": "e71a561123a103c1b129c946dcb5b2286926b5b6",
          "url": "https://github.com/matyushkin/djvu-rs/commit/ec8ea74ff084118cf5ddafd618dbee946db51638"
        },
        "date": 1784465211679,
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
            "value": 166647,
            "range": "± 1893",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 164020,
            "range": "± 1267",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 806911,
            "range": "± 3809",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 595443,
            "range": "± 2648",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1373653,
            "range": "± 33958",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2407,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 11260251,
            "range": "± 182204",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3914240,
            "range": "± 15271",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2603772,
            "range": "± 47242",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 957864,
            "range": "± 6730",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 676576,
            "range": "± 4908",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 245881,
            "range": "± 1590",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9631151,
            "range": "± 87730",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 585452,
            "range": "± 3028",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2323350,
            "range": "± 82130",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3482298,
            "range": "± 20033",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 60119494,
            "range": "± 3717526",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 160555,
            "range": "± 297",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24224579,
            "range": "± 76078",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 10246899,
            "range": "± 99588",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 3015089,
            "range": "± 23082",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 13869992,
            "range": "± 89922",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6661146,
            "range": "± 46111",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 54627736,
            "range": "± 185677",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 941781587,
            "range": "± 4384868",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 19743411,
            "range": "± 69953",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 457408393,
            "range": "± 845333",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 19146841658,
            "range": "± 190125324",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 18333211,
            "range": "± 278086",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 17313154,
            "range": "± 144396",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 173050,
            "range": "± 2536",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 974701,
            "range": "± 2549",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4222887,
            "range": "± 7084",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 16996784,
            "range": "± 59913",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1010907,
            "range": "± 7738",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10311851,
            "range": "± 82773",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10271807,
            "range": "± 24247",
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
            "value": 5487568,
            "range": "± 10216",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 26600893,
            "range": "± 465857",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 90242767,
            "range": "± 314681",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 84374812,
            "range": "± 238800",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4114980,
            "range": "± 21134",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 16399854,
            "range": "± 62512",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 84418125,
            "range": "± 333337",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 90237128,
            "range": "± 189606",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 86519078,
            "range": "± 346947",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 85823660,
            "range": "± 139393",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3564560,
            "range": "± 14966",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4396654,
            "range": "± 13865",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 84370444,
            "range": "± 404307",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 80682951,
            "range": "± 72902",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 80099629,
            "range": "± 184675",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 578718,
            "range": "± 3380",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4396840,
            "range": "± 16310",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 86333024,
            "range": "± 72238",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 80660952,
            "range": "± 327620",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10060973,
            "range": "± 775404",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 26559178,
            "range": "± 71408",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 83712,
            "range": "± 102",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 83464737,
            "range": "± 61054",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 171534,
            "range": "± 143",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 173392,
            "range": "± 133",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 83570,
            "range": "± 147",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 84590,
            "range": "± 121",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10065110,
            "range": "± 8699",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 10284327,
            "range": "± 19922",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 86350094,
            "range": "± 56820",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 87775751,
            "range": "± 65392",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 80613083,
            "range": "± 340325",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 82050321,
            "range": "± 92550",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 84457,
            "range": "± 224",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1165231,
            "range": "± 15034",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 21251285,
            "range": "± 105568",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 281900733,
            "range": "± 458270",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1172554208,
            "range": "± 5378624",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1662338682,
            "range": "± 792062",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 2091323,
            "range": "± 18641",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2272,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 20907329,
            "range": "± 187456",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 8437753,
            "range": "± 130165",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 8067800,
            "range": "± 161971",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3260047,
            "range": "± 19064",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 24218448,
            "range": "± 827545",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6551,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 226336,
            "range": "± 11835",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 110539885,
            "range": "± 362210",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 12847470,
            "range": "± 295590",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 305293427,
            "range": "± 1770210",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2613509,
            "range": "± 9720",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 34532238,
            "range": "± 250324",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 5847341,
            "range": "± 257988",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 89439411,
            "range": "± 337261",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 74256209,
            "range": "± 218760",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 77039362,
            "range": "± 1100211",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 164217,
            "range": "± 643",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 26747956,
            "range": "± 113823",
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
            "value": 8242000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49585000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 47555000,
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
          "id": "ee6dc02f3f0839a9178a1ce2c88ee1a3b9d10341",
          "message": "Merge pull request #733 from matyushkin/issue-696-render-recovery\n\nfeat(render): structured recovery report from permissive renders (#696)",
          "timestamp": "2026-07-19T22:21:27+08:00",
          "tree_id": "8c3a81290840b791b79b82c3c340c851e709c824",
          "url": "https://github.com/matyushkin/djvu-rs/commit/ee6dc02f3f0839a9178a1ce2c88ee1a3b9d10341"
        },
        "date": 1784472458259,
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
            "value": 127506,
            "range": "± 1571",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 169714,
            "range": "± 814",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 826192,
            "range": "± 4589",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 402800,
            "range": "± 12844",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1205891,
            "range": "± 62122",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2370,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 11369501,
            "range": "± 350154",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 4124168,
            "range": "± 128002",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2692969,
            "range": "± 60954",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 998730,
            "range": "± 14845",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 668989,
            "range": "± 1695",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 257078,
            "range": "± 2790",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9965638,
            "range": "± 115408",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 609926,
            "range": "± 8355",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2402703,
            "range": "± 97399",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3540305,
            "range": "± 14049",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 72305015,
            "range": "± 3729577",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 150220,
            "range": "± 4351",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24239016,
            "range": "± 343343",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 9481921,
            "range": "± 47692",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 3207619,
            "range": "± 9691",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 12653900,
            "range": "± 103372",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6998900,
            "range": "± 198202",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 56241819,
            "range": "± 148708",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 970658579,
            "range": "± 1092081",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 19736264,
            "range": "± 134383",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 413491015,
            "range": "± 1225247",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 18507862708,
            "range": "± 27748507",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 17064367,
            "range": "± 199820",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 18838291,
            "range": "± 93549",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 186668,
            "range": "± 1051",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 1066016,
            "range": "± 15111",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4624381,
            "range": "± 9432",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 18549398,
            "range": "± 502665",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1090449,
            "range": "± 5359",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10951762,
            "range": "± 184098",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10961787,
            "range": "± 28973",
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
            "value": 6030097,
            "range": "± 16003",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 28740911,
            "range": "± 203763",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 97682548,
            "range": "± 1384077",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 91696858,
            "range": "± 350086",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4458922,
            "range": "± 49617",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 17543661,
            "range": "± 392926",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 91907827,
            "range": "± 1994308",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 97791028,
            "range": "± 105528",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 94536049,
            "range": "± 1155824",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 92801872,
            "range": "± 94527",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3783601,
            "range": "± 9317",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4797150,
            "range": "± 10916",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 91875672,
            "range": "± 108290",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 88391240,
            "range": "± 67296",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 86940574,
            "range": "± 1620432",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 406239,
            "range": "± 8162",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4747153,
            "range": "± 6439",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 94421399,
            "range": "± 173755",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 88500037,
            "range": "± 163911",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10865041,
            "range": "± 19107",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 28680595,
            "range": "± 30499",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 90161,
            "range": "± 211",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 91504508,
            "range": "± 51620",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 185873,
            "range": "± 229",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 187867,
            "range": "± 200",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 90272,
            "range": "± 1451",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 90883,
            "range": "± 2038",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10869817,
            "range": "± 12861",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 11134990,
            "range": "± 58991",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 94376287,
            "range": "± 78373",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 95102269,
            "range": "± 88810",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 88494478,
            "range": "± 52155",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 88661770,
            "range": "± 153482",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 91221,
            "range": "± 306",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1279941,
            "range": "± 26258",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 22773336,
            "range": "± 67006",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 310035631,
            "range": "± 246209",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1258558438,
            "range": "± 4518455",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1744828582,
            "range": "± 4358425",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 2458717,
            "range": "± 21654",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2206,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 17491070,
            "range": "± 112730",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 6824401,
            "range": "± 124086",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 6857154,
            "range": "± 64041",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3587494,
            "range": "± 21074",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 26265236,
            "range": "± 207208",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 7015,
            "range": "± 72",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 153730,
            "range": "± 6295",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 121582841,
            "range": "± 1502708",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 13951711,
            "range": "± 75952",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 329667978,
            "range": "± 2609141",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2792069,
            "range": "± 5940",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 36851010,
            "range": "± 228263",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 6296345,
            "range": "± 184457",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 91034646,
            "range": "± 210763",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 77551817,
            "range": "± 187196",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 79337050,
            "range": "± 884870",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 145937,
            "range": "± 461",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 28815053,
            "range": "± 101538",
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
            "value": 8797000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 54063000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 51819000,
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
          "id": "8cfdff145f3139b09f208843aea3b8f4a4c89de9",
          "message": "Merge pull request #735 from matyushkin/issue-696-resource-limits\n\nfeat(validate): resource estimates and --limits before expensive work (#696)",
          "timestamp": "2026-07-25T00:34:19+08:00",
          "tree_id": "bb8ec53fc21ed033c71f22fbdac80c11c448a386",
          "url": "https://github.com/matyushkin/djvu-rs/commit/8cfdff145f3139b09f208843aea3b8f4a4c89de9"
        },
        "date": 1784912280625,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 72,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "bzz_encode",
            "value": 99784,
            "range": "± 1853",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 132016,
            "range": "± 321",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 643982,
            "range": "± 9891",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 388149,
            "range": "± 6767",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 990970,
            "range": "± 5725",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 1851,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 8236245,
            "range": "± 13325",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3315543,
            "range": "± 5543",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2259651,
            "range": "± 3578",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 836796,
            "range": "± 1467",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 519496,
            "range": "± 1374",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 199264,
            "range": "± 504",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 7507970,
            "range": "± 10254",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 473131,
            "range": "± 578",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 1820416,
            "range": "± 4927",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2735412,
            "range": "± 37096",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 39290945,
            "range": "± 1119685",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 116343,
            "range": "± 1065",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 18719846,
            "range": "± 51122",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 8380485,
            "range": "± 200491",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 2475199,
            "range": "± 3246",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 9955487,
            "range": "± 44026",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 5447064,
            "range": "± 6574",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 43863360,
            "range": "± 633016",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 757035848,
            "range": "± 735755",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 15924001,
            "range": "± 7077",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 361122784,
            "range": "± 391461",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 14276127526,
            "range": "± 12548456",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 12970541,
            "range": "± 49933",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 14212874,
            "range": "± 27660",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 146257,
            "range": "± 424",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 827370,
            "range": "± 1136",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 3583064,
            "range": "± 5263",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 14315772,
            "range": "± 18075",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 851813,
            "range": "± 1173",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 8440257,
            "range": "± 16719",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 8424380,
            "range": "± 151510",
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
            "value": 4686565,
            "range": "± 6464",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 21654334,
            "range": "± 32831",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 74381052,
            "range": "± 90937",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 69863821,
            "range": "± 76221",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 3422782,
            "range": "± 51591",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 13753075,
            "range": "± 20033",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 69864946,
            "range": "± 62750",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 74374138,
            "range": "± 115779",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 72908936,
            "range": "± 44006",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 72014047,
            "range": "± 89118",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 2939193,
            "range": "± 62194",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 3656729,
            "range": "± 1883",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 69886453,
            "range": "± 105891",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 68442191,
            "range": "± 41840",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 67647503,
            "range": "± 804881",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 341270,
            "range": "± 6334",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 3656894,
            "range": "± 5398",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 72967753,
            "range": "± 104037",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 68435053,
            "range": "± 55599",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 8365069,
            "range": "± 11281",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 22533475,
            "range": "± 37455",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 69959,
            "range": "± 62",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 70685491,
            "range": "± 92011",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 144074,
            "range": "± 210",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 146351,
            "range": "± 195",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 69918,
            "range": "± 69",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 70763,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 8376233,
            "range": "± 15476",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 8512073,
            "range": "± 5383",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 72945873,
            "range": "± 60716",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 73472872,
            "range": "± 24206",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 68369468,
            "range": "± 51987",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 68823356,
            "range": "± 52268",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 70490,
            "range": "± 947",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1037342,
            "range": "± 11410",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 18064438,
            "range": "± 13929",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 247818811,
            "range": "± 185575",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 971742539,
            "range": "± 1399555",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1346662691,
            "range": "± 1478737",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 1624378,
            "range": "± 20178",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 1714,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 13512494,
            "range": "± 34859",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 5136161,
            "range": "± 37669",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 5000803,
            "range": "± 15870",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 2742601,
            "range": "± 5112",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 19828593,
            "range": "± 43580",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 5497,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 117341,
            "range": "± 16956",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 93788259,
            "range": "± 366128",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 10736443,
            "range": "± 19758",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 251115229,
            "range": "± 249391",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2149674,
            "range": "± 2885",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 26907757,
            "range": "± 54559",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 4906555,
            "range": "± 13470",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 70430624,
            "range": "± 1114720",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 60177979,
            "range": "± 132452",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 61561161,
            "range": "± 154862",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 130737,
            "range": "± 404",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 22118293,
            "range": "± 66846",
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
            "value": 6801000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 41441000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 39960000,
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
          "id": "b5f76a228c2362fedacbff99f8db361a34fa303f",
          "message": "Merge pull request #736 from matyushkin/issue-694-encoder-ingestion\n\nExpand PNG encoder ingestion with explicit policy (#694)",
          "timestamp": "2026-07-25T02:33:59+08:00",
          "tree_id": "cff2bdf195e177f46e7456669ed06c26917f14be",
          "url": "https://github.com/matyushkin/djvu-rs/commit/b5f76a228c2362fedacbff99f8db361a34fa303f"
        },
        "date": 1784919577615,
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
            "value": 169388,
            "range": "± 12430",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 162598,
            "range": "± 1718",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 779219,
            "range": "± 1959",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 586726,
            "range": "± 23806",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1311093,
            "range": "± 12147",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2371,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 10475039,
            "range": "± 54646",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3858280,
            "range": "± 15055",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2579564,
            "range": "± 13093",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 945121,
            "range": "± 7713",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 661680,
            "range": "± 8198",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 244322,
            "range": "± 1444",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9275559,
            "range": "± 83938",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 572612,
            "range": "± 1062",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2305847,
            "range": "± 19039",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3410097,
            "range": "± 35669",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 49116066,
            "range": "± 613490",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 160605,
            "range": "± 694",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24144386,
            "range": "± 66086",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 8551910,
            "range": "± 71971",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 3000071,
            "range": "± 14505",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 13320885,
            "range": "± 489640",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6489114,
            "range": "± 27354",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 52281585,
            "range": "± 420073",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 926755113,
            "range": "± 1533801",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 18361453,
            "range": "± 38196",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 378186285,
            "range": "± 405927",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 16990901312,
            "range": "± 39436184",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 16472218,
            "range": "± 122849",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 16974216,
            "range": "± 36193",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 186048,
            "range": "± 561",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 973658,
            "range": "± 2632",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4215819,
            "range": "± 31158",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 16815420,
            "range": "± 34012",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1005027,
            "range": "± 9695",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10707152,
            "range": "± 14891",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10702577,
            "range": "± 6624",
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
            "value": 5458817,
            "range": "± 8531",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 26442867,
            "range": "± 378566",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 89887909,
            "range": "± 1409060",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 84171459,
            "range": "± 349002",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4351858,
            "range": "± 89333",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 17168610,
            "range": "± 99281",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 84357102,
            "range": "± 146252",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 90244593,
            "range": "± 435443",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 86622522,
            "range": "± 2699621",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 86689191,
            "range": "± 554260",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3466892,
            "range": "± 81176",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4335698,
            "range": "± 12049",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 83805636,
            "range": "± 196419",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 80859558,
            "range": "± 496875",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 82078790,
            "range": "± 2404095",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 576104,
            "range": "± 4790",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4339004,
            "range": "± 131118",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 86613998,
            "range": "± 104917",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 80772789,
            "range": "± 183261",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10618077,
            "range": "± 6804",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 27228015,
            "range": "± 49312",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 90095,
            "range": "± 89",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 83557808,
            "range": "± 82460",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 184874,
            "range": "± 274",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 173077,
            "range": "± 309",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 90086,
            "range": "± 105",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 84468,
            "range": "± 57",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10643548,
            "range": "± 27780",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 10463174,
            "range": "± 45324",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 86558239,
            "range": "± 103285",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 87841678,
            "range": "± 41920",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 80808864,
            "range": "± 46714",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 83528946,
            "range": "± 71691",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 90739,
            "range": "± 160",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1167817,
            "range": "± 14312",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 21841342,
            "range": "± 123488",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 281104750,
            "range": "± 291860",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1182481144,
            "range": "± 1530492",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1698686936,
            "range": "± 1388772",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 2058185,
            "range": "± 40328",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2088,
            "range": "± 61",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 20498558,
            "range": "± 164106",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 7318400,
            "range": "± 373702",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 6820542,
            "range": "± 156322",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3279271,
            "range": "± 10670",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 24408704,
            "range": "± 360026",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6550,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 225710,
            "range": "± 11929",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 111568882,
            "range": "± 564916",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 12821469,
            "range": "± 59788",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 295967471,
            "range": "± 1805335",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2609526,
            "range": "± 14689",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 34032249,
            "range": "± 810518",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 6007064,
            "range": "± 40741",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 84148405,
            "range": "± 369498",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 72121607,
            "range": "± 1589185",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 73746745,
            "range": "± 1238051",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 176330,
            "range": "± 854",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 27330954,
            "range": "± 414734",
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
            "value": 8234999,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49378000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 47418000,
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
          "id": "fc6162738ad7a19aa43b1452e4da3c1daf270765",
          "message": "Merge pull request #740 from matyushkin/issue-695-resource-limits\n\nfeat(limits): configurable ResourceLimits for parse/render (#695)",
          "timestamp": "2026-07-25T04:11:51+08:00",
          "tree_id": "059d966c1459642c085917cf37c35f0081c004c7",
          "url": "https://github.com/matyushkin/djvu-rs/commit/fc6162738ad7a19aa43b1452e4da3c1daf270765"
        },
        "date": 1784925465776,
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
            "value": 166679,
            "range": "± 916",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 160435,
            "range": "± 1235",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 765657,
            "range": "± 2481",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 585313,
            "range": "± 2776",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1405840,
            "range": "± 19493",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2356,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 10949283,
            "range": "± 204363",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3905846,
            "range": "± 33764",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2601447,
            "range": "± 47680",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 950274,
            "range": "± 3922",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 668241,
            "range": "± 2451",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 244625,
            "range": "± 1150",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 9461847,
            "range": "± 123943",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 575140,
            "range": "± 3280",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2425851,
            "range": "± 63436",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3458847,
            "range": "± 41727",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 55352636,
            "range": "± 4304199",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 160420,
            "range": "± 808",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24227960,
            "range": "± 65968",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 9893915,
            "range": "± 72270",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 2992035,
            "range": "± 6243",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 13838362,
            "range": "± 139468",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6761597,
            "range": "± 85208",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 55160797,
            "range": "± 297736",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 942993584,
            "range": "± 1826860",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 19687939,
            "range": "± 18043",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 441320830,
            "range": "± 684252",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 18130347092,
            "range": "± 23138100",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 16996875,
            "range": "± 465685",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 17212006,
            "range": "± 110911",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 178680,
            "range": "± 784",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 973912,
            "range": "± 2792",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4220842,
            "range": "± 8824",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 16896579,
            "range": "± 87362",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1009972,
            "range": "± 11391",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10465689,
            "range": "± 78783",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10518863,
            "range": "± 26493",
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
            "value": 5465035,
            "range": "± 12590",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 26918015,
            "range": "± 405403",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 90242211,
            "range": "± 368701",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 84578698,
            "range": "± 307665",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4225579,
            "range": "± 20426",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 16737085,
            "range": "± 100529",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 84430602,
            "range": "± 359130",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 90328357,
            "range": "± 174291",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 86599767,
            "range": "± 215064",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 85719337,
            "range": "± 143052",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3469798,
            "range": "± 5813",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4476628,
            "range": "± 13048",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 84461568,
            "range": "± 177612",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 80731934,
            "range": "± 60177",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 80020907,
            "range": "± 65623",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 577187,
            "range": "± 1215",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4507805,
            "range": "± 13062",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 86585929,
            "range": "± 67612",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 80747443,
            "range": "± 37018",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10366605,
            "range": "± 9826",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 27078145,
            "range": "± 32154",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 86598,
            "range": "± 263",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 83975087,
            "range": "± 323251",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 178402,
            "range": "± 404",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 173576,
            "range": "± 169",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 86613,
            "range": "± 297",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 84668,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10335556,
            "range": "± 8587",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 10442052,
            "range": "± 41405",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 86644669,
            "range": "± 94063",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 87887679,
            "range": "± 75964",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 80703869,
            "range": "± 354324",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 82081610,
            "range": "± 49312",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 87094,
            "range": "± 250",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1221767,
            "range": "± 8893",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 21657446,
            "range": "± 47264",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 291154246,
            "range": "± 675551",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1181104517,
            "range": "± 1758893",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1693387703,
            "range": "± 1631682",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 2120896,
            "range": "± 66114",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 2096,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 20994497,
            "range": "± 525875",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 7313338,
            "range": "± 380327",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 7212333,
            "range": "± 367067",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3286763,
            "range": "± 22522",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 24602776,
            "range": "± 210897",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6564,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 225097,
            "range": "± 14050",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 111218427,
            "range": "± 263096",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 12916674,
            "range": "± 37226",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 295875550,
            "range": "± 834311",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2596300,
            "range": "± 11423",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 34325319,
            "range": "± 436504",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 6023524,
            "range": "± 47270",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 89300641,
            "range": "± 358222",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 74725150,
            "range": "± 146946",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 77425497,
            "range": "± 148177",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 178433,
            "range": "± 3310",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 26694736,
            "range": "± 107504",
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
            "value": 8254000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49660000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 47769000,
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
          "id": "e9c4d377c96ce8ecd68302d3b52d0a2898d63e63",
          "message": "fix(deps): require tract-onnx >=0.22.2 (RUSTSEC-2026-0217) (#743)\n\nRUSTSEC-2026-0217 reports an integer overflow in the tract-nnef NNEF\ntensor parser (out-of-bounds read on model load) affecting tract-nnef\n0.21.10 in the freshly resolved dependency tree, failing the required\nDependencies (deny + audit) check on every PR.\n\nThe patched 0.21.x line (>=0.21.16) is unusable: tract-linalg 0.21.17\npins time <0.3.42, and all such time versions carry RUSTSEC-2026-0009\n(stack-exhaustion DoS, fixed in 0.3.47). The 0.23.x line requires\nRust 1.91, above our MSRV 1.88. The 0.22.x line (MSRV 1.85) fixes the\ntract advisory as of 0.22.2 and leaves time unpinned, so a fresh\nresolve picks a patched time.\n\ncargo audit on a from-scratch lockfile: 0 vulnerabilities.\ncargo build --features ocr-onnx passes against tract-onnx 0.22.3.\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs",
          "timestamp": "2026-08-11T16:19:53+05:00",
          "tree_id": "34235a0ed12b6b25d48fcf905634eb0694ed48ec",
          "url": "https://github.com/matyushkin/djvu-rs/commit/e9c4d377c96ce8ecd68302d3b52d0a2898d63e63"
        },
        "date": 1786448726878,
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
            "value": 168290,
            "range": "± 1376",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 162413,
            "range": "± 2350",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 769801,
            "range": "± 15896",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 587380,
            "range": "± 5819",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 1392892,
            "range": "± 24012",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 2390,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 10747226,
            "range": "± 56995",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3841682,
            "range": "± 7176",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2547776,
            "range": "± 74059",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 942040,
            "range": "± 2570",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 662666,
            "range": "± 1384",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 243707,
            "range": "± 2420",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 8861311,
            "range": "± 21586",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 570723,
            "range": "± 1009",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2166455,
            "range": "± 5327",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 3439751,
            "range": "± 8814",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 48936586,
            "range": "± 449076",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 160836,
            "range": "± 2099",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 24155681,
            "range": "± 30934",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 8494474,
            "range": "± 43671",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 2989761,
            "range": "± 5979",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 13317391,
            "range": "± 32072",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 6396555,
            "range": "± 29694",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 51645601,
            "range": "± 360384",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 920344991,
            "range": "± 1097784",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 18186629,
            "range": "± 26305",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 375439480,
            "range": "± 1264163",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 16410426880,
            "range": "± 36834449",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 16697877,
            "range": "± 185944",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 16965041,
            "range": "± 222629",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 178397,
            "range": "± 572",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 974438,
            "range": "± 2272",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 4218495,
            "range": "± 6455",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 16837875,
            "range": "± 27612",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 1042177,
            "range": "± 18139",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 10422583,
            "range": "± 88128",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 10421272,
            "range": "± 6469",
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
            "value": 5477245,
            "range": "± 12723",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 25879585,
            "range": "± 160036",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 89821234,
            "range": "± 208048",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 84107431,
            "range": "± 479506",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 4213096,
            "range": "± 13132",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 16538867,
            "range": "± 34398",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 84093715,
            "range": "± 245543",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 89854791,
            "range": "± 151315",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 86634621,
            "range": "± 1599258",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 86738103,
            "range": "± 49436",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 3471946,
            "range": "± 3751",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 4347066,
            "range": "± 4060",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 84200324,
            "range": "± 208943",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 80825330,
            "range": "± 91799",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 81940376,
            "range": "± 63644",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 575830,
            "range": "± 799",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 4353855,
            "range": "± 22035",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 86593910,
            "range": "± 60530",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 80801286,
            "range": "± 71652",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 10313950,
            "range": "± 8153",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 27183000,
            "range": "± 14414",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 86361,
            "range": "± 62",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 83754958,
            "range": "± 67985",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 177139,
            "range": "± 492",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 173305,
            "range": "± 143",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 86362,
            "range": "± 212",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 84498,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 10311313,
            "range": "± 7672",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 10379449,
            "range": "± 4506",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 86594131,
            "range": "± 39242",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 88333558,
            "range": "± 74414",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 80812867,
            "range": "± 966192",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 83507489,
            "range": "± 50469",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 87015,
            "range": "± 126",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1215885,
            "range": "± 12824",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 22386787,
            "range": "± 75480",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 291312931,
            "range": "± 463721",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 1167060975,
            "range": "± 10905135",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1652488750,
            "range": "± 1955566",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 2040738,
            "range": "± 30835",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 1989,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 20008453,
            "range": "± 282383",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 7249095,
            "range": "± 110846",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 6604596,
            "range": "± 106458",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 3245121,
            "range": "± 7805",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 23689645,
            "range": "± 73037",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 6656,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 226210,
            "range": "± 9198",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 110922624,
            "range": "± 577818",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 12952397,
            "range": "± 33411",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 295008179,
            "range": "± 2591899",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2631872,
            "range": "± 35443",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 32631321,
            "range": "± 142079",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 6179913,
            "range": "± 23594",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 84624426,
            "range": "± 344149",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 71672065,
            "range": "± 210581",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 73735250,
            "range": "± 403335",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 178126,
            "range": "± 1771",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 28465038,
            "range": "± 125497",
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
            "value": 8173999,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49610000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 47595000,
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
          "id": "73e1197530209afcf14f989f8624cf32609bc8e8",
          "message": "feat(ingest): expand TIFF encoder ingestion slice 2 (#694) (#742)\n\nDecode 16-bit gray/RGB/RGBA/CMYK TIFF through the IngestPolicy\ndown-conversion, convert CMYK via a documented profile-free transform,\nand add a raw strip reader for the layouts tiff 0.9 mishandles:\nbilevel, 2/4-bit gray, and palette images (uncompressed strips).\nMultipage TIFFs decode page-per-IFD; the CLI maps them to a\nmulti-page bundle under the same rules as directory input.\nUnsupported layouts (CCITT G4, tiled/planar bilevel, FillOrder 2)\nget targeted errors. Documents the matrix in encoder-ingestion.md.\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs",
          "timestamp": "2026-08-11T16:59:11+05:00",
          "tree_id": "4dcc16ec809c2c099986f1cc5079859905736d75",
          "url": "https://github.com/matyushkin/djvu-rs/commit/73e1197530209afcf14f989f8624cf32609bc8e8"
        },
        "date": 1786450953562,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 119000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 6192000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 37889000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 36511000,
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
          "id": "2cb629f9fdc6eb7dcdd3637bfe425c9499cff2ea",
          "message": "feat(render): tile-first rendering API slice 1 (#691) (#744)\n\nAdd djvu_tile module: TileLayout/TileRect/TileError plus render_tile and\nrender_tile_cached. Tiles live in display space (post-rotation); the\nlayout pulls each tile back to the region renderer's pre-rotation\nRenderRect for all four combined rotations. Assembly of all tiles is\nbyte-identical to render_pixmap; tile pixels are order-independent,\ncached or not. Lanczos3 and aa are rejected up front (post-passes that\ncannot be tiled byte-identically yet). Contract: docs/tile-rendering.md.\n\nFix a renderer divergence the parity tests exposed: render_region and\nrender_region_tiled always composited against the full-resolution JB2\nmask, while render_into/render_rows switch to the 1/4-resolution mask at\nbackground subsample >= 4. Both region entry points now take the same\nresolve_sub4_mask decision (regression test\nrender_region_matches_full_render_crop_at_sub4).\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs",
          "timestamp": "2026-08-11T19:09:22+05:00",
          "tree_id": "11e93a4149e9624c1a1195ee9e26362fd2b37b56",
          "url": "https://github.com/matyushkin/djvu-rs/commit/2cb629f9fdc6eb7dcdd3637bfe425c9499cff2ea"
        },
        "date": 1786458899771,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 142000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 7221000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 45184000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 43127000,
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
          "id": "fe7914f466ab08e9420a50e29fe1fc3e94e86d26",
          "message": "feat(render): tile cache budget, invalidation, prefetch (#691 slice 2) (#746)\n\nPublic cache control at tile granularity in djvu_tile:\ntile_cache_usage (bytes/budget/internal-tile count),\nset_tile_cache_budget (per-page override, 0 disables caching, evicts\ndown immediately, survives downgrade), clear_tile_cache, and\ninvalidate_tile_region — a display-space rect pulled back through the\ncombined rotation and mapped proportionally (rounding outward) into\nevery cached render size. With the parallel feature, prefetch_tiles\nschedules a bounded (Chebyshev radius, grid-clipped) background warm of\nthe same cache render_tile_cached reads.\n\nInternally TileCacheState grows an Option<usize> budget override and a\nshared evict_to_budget helper; PageLayers::downgrade now preserves the\noverride (configuration, not cached data). Cache state never changes\nrendered bytes — only latency; determinism tests cover budget shrink,\nbudget 0, cross-scale and cross-rotation invalidation, and prefetch.\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs",
          "timestamp": "2026-08-11T19:39:38+05:00",
          "tree_id": "65a7a6acfd8389f51134f7b77ee0150af0ce519d",
          "url": "https://github.com/matyushkin/djvu-rs/commit/fe7914f466ab08e9420a50e29fe1fc3e94e86d26"
        },
        "date": 1786460715914,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 165000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8199999,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49245000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 47535000,
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
          "id": "1534ba766391ded66d999595f4949be307cd54e3",
          "message": "feat(tile): progressive quality steps and cooperative cancellation (#691 slice 3a) (#747)\n\nAdd render_tile_with(page, opts, tile_size, col, row, &TileRenderControls):\n- quality_step: Some(k) composites the tile from BG44 chunks 0..=k only,\n  byte-identical to the matching crop of render_progressive_step frame k\n  under every rotation; monotonic never-regress ladder by construction.\n  Partial-quality pixels never enter the composited-tile cache.\n- TileCancelToken: shared one-way flag with checkpoints between units of\n  work (per tile, per internal cache tile, between decode and composite);\n  cancelled calls return TileError::Cancelled and never corrupt caches.\n- prefetch_tiles_cancellable bounds how much of the prefetch schedule runs.\n\nInternally, render_region_tiled gains a cancellable variant returning\nOk(None) on cancellation, and a new render_region_progressive mirrors\nrender_progressive's compositing (full-resolution mask, no\nresolve_sub4_mask) so crop parity holds by construction.\n\nAlso fixes a pre-existing determinism bug: the #607 warm-mask fast path in\ndecode_layers returned a maskless layer set to progressive renders, so a\nprior full render at strong downscale silently dropped the text layer from\nlater progressive frames (colorbook/history/carte fixtures). The fast path\nis now gated to full-background decodes; regression test\nrender_progressive_ignores_mask_sub4_warmth.\n\nContract: docs/tile-rendering.md, slice 3 section.\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs",
          "timestamp": "2026-08-11T23:44:59+05:00",
          "tree_id": "b7025013707179e7079c77a4e2e099b91f7cafba",
          "url": "https://github.com/matyushkin/djvu-rs/commit/1534ba766391ded66d999595f4949be307cd54e3"
        },
        "date": 1786475452834,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 161000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8693000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 53799000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 51339000,
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
          "id": "2b131064e544cf3a7b5a59c07482a38a525457ef",
          "message": "feat(ocr): model manifest with SHA-256 verification + DBNet detection (#693) (#749)\n\nSlice 2 of the neural OCR pipeline:\n\n- docs/ocr-model-manifest.toml: pinned PP-OCRv4 mobile det/rec artifacts\n  (commit-pinned URL, size, SHA-256, opset, SPDX license); embedded at\n  compile time as the single source of truth\n- ocr_onnx::manifest: manifest parser + mandatory size/SHA-256\n  verification; unverified weights are never loaded (typed hard errors)\n- ocr_onnx::preprocess: deterministic detector preprocessing — input size\n  from the page's actual extent (long side <= 960, sides rounded to x32),\n  fixed-point 16.16 bilinear resize, ImageNet mean/std CHW tensor\n- ocr_onnx::detect: DBNet text detection with a shape-keyed LRU of tract\n  plans; pure prob-map postprocessing (binarize, 4-connected components,\n  score/size filters, unclip, page-coordinate mapping) unit-tested on\n  synthetic maps; real-model tests skip silently without weights\n- scripts/fetch_ocr_models.sh: explicit, verifying fetch (weights are\n  never committed and never downloaded implicitly); models/ gitignored\n  and excluded from the published crate\n- CI: main-only non-required \"OCR (onnx models)\" job fetches pinned\n  weights (cached by manifest hash) and runs the ocr_onnx test tree\n- sha2 added to the ocr-onnx feature and to the feature-hygiene\n  forbidden list; README/feature-matrix/design-doc updated\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs",
          "timestamp": "2026-08-11T23:45:06+05:00",
          "tree_id": "481b32e58e63386aa528c0e79cae2f2e7b4706a1",
          "url": "https://github.com/matyushkin/djvu-rs/commit/2b131064e544cf3a7b5a59c07482a38a525457ef"
        },
        "date": 1786476943424,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 164000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8228000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49554000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 47498000,
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
          "id": "2f934f11cd8494e728df256968e0c54f92e51600",
          "message": "feat(tile): progressive tile quality, cancellation, async+wasm surfaces (#748)\n\nAsync (feature \"async\"):\n- render_tile_async runs render_tile_with inside spawn_blocking; every\n  byte guarantee of the sync entry point carries over unchanged, and a\n  TileCancelToken clone cancels from any task.\n- render_tile_progressive_stream is the tile-granular counterpart of\n  render_progressive_stream: one frame per quality step, coarsest first,\n  each byte-identical to the matching crop of the full-page progressive\n  frame. On cancellation the stream yields one Cancelled error and ends.\n- New AsyncTileError mirrors AsyncRenderError (Tile + Join).\n\nWASM (feature \"wasm\"):\n- WasmPage::tile_cols/tile_rows expose the display-space grid;\n  render_tile (full quality, composited-tile cache, returns WasmPixmap\n  with the clipped tile dimensions), render_tile_progressive (quality\n  step chunk_n, never cached), render_tile_into_pixmap (#611\n  buffer-reuse pattern). Cancellation tokens are not exposed: wasm\n  renders are synchronous calls JS cannot interrupt mid-call.\n\nBoth surfaces are thin adapters over render_tile_with — no rendering\nlogic of their own. Contract: docs/tile-rendering.md, slice 3b section.\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs",
          "timestamp": "2026-08-12T00:42:27+05:00",
          "tree_id": "829a049599ff01032cedc950b870de4c9496197f",
          "url": "https://github.com/matyushkin/djvu-rs/commit/2f934f11cd8494e728df256968e0c54f92e51600"
        },
        "date": 1786478871959,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 160000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8682000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 52836000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 51259000,
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
          "id": "bd6e1e644ef58cba05d2b8b2ce1c3f1ac6b7024c",
          "message": "feat(ocr): NeuralOcrBackend pipeline wired to CLI as --backend onnx (#751)\n\n* feat(ocr): Cyrillic CTC line recognition with pinned dictionary (#693)\n\nSlice 3a of the neural OCR pipeline — the recognition core:\n\n- manifest: add the official PaddlePaddle Cyrillic PP-OCRv5 mobile\n  recognizer (ONNX, opset 7, Apache-2.0) plus its companion inference.yml\n  pinned as a sibling artifact from the same upstream commit; the yml\n  embeds the CTC character dictionary (850 chars). Non-ONNX artifacts\n  use opset = 0; the manifest test now checks per-kind opsets and that\n  model and dictionary share one pinned commit\n- ocr_onnx::recognize: Vocabulary (dict parsed from the verified config;\n  class 0 = CTC blank, last class = appended space), pure greedy CTC\n  decoder with mean-confidence, line-crop preprocessing (height 48,\n  aspect-preserving fixed-point bilinear resize, width buckets of 32\n  capped at 3200, BGR, (v/255-0.5)/0.5, zero padding), TextRecognizer\n  with an LRU of tract plans keyed by padded width; a class-count\n  mismatch between model and dictionary is a hard error\n- 10 new unit tests run without weights; 2 model-gated tests verify the\n  real pinned dict/model agree (852 classes) and blank lines decode to\n  empty, and exercise plan-cache reuse; they skip silently when weights\n  are absent (the ocr-onnx CI job fetches them)\n\nSpike (local): tract-onnx 0.22.3 runs the model, output [1, W/8, 852].\nTextLayer assembly, word split, and CLI wiring follow in slice 3b.\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs\n\n* feat(ocr): neural OCR pipeline backend + CLI --backend onnx (#693)\n\nSlice 3b: assemble DBNet detection and Cyrillic CTC recognition into a\nCLI-live OCR backend.\n\n- ocr_onnx::pipeline::NeuralOcrBackend: detect -> recognize per line ->\n  TextLayer; engines behind one Mutex (plan caches need &mut, the trait\n  takes &self); always emits the guaranteed page-level zone.\n- assemble_text_layer / word_zones pure helpers: blank lines dropped,\n  detector order kept, proportional word-split heuristic (>= 1 px, full\n  line height).\n- CLI: --backend onnx now builds NeuralOcrBackend from the pinned\n  manifest; --model is rejected for this backend (it would bypass\n  SHA-256 verification); un-featured builds get a clear enable hint.\n- Docs: README (backend now CLI-live), seam decision update, design-doc\n  slice 3 marked done, OcrBackend granularity note.\n\nVerified end-to-end: tests/fixtures/big-scanned-page.djvu -> 627 chars\nembedded and re-extracted via `djvu text`. 10 new unit tests (8 pure,\n2 model-gated).\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs",
          "timestamp": "2026-08-12T00:57:49+05:00",
          "tree_id": "5216d73491b5804bc2a5d47ffc2cccabe00239c7",
          "url": "https://github.com/matyushkin/djvu-rs/commit/bd6e1e644ef58cba05d2b8b2ce1c3f1ac6b7024c"
        },
        "date": 1786480191938,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 106000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 5211000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 31206000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 29204000,
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
          "id": "667003e2c53e7eab07392bbfc412d77ed26975dc",
          "message": "feat(ocr): synthetic metrics corpus with CER/WER/IoU baseline (#752)\n\n* feat(ocr): synthetic metrics corpus with CER/WER/IoU baseline (#693)\n\nSlice 4: deterministic quality gate for the pinned neural OCR models.\n\n- Manifest: pin PT Sans regular (google/fonts, OFL-1.1) as the corpus\n  font — fetched and SHA-256-verified like every other artifact; the\n  fetch script handles it unchanged.\n- ocr_onnx::metrics: pure CER / WER / rect IoU / greedy mean line IoU\n  with unit tests; no new runtime dependencies.\n- ocr_onnx::corpus (test-only, ab_glyph as dev-dependency): renders\n  Cyrillic and Latin+digits pages from versioned layout parameters with\n  ink-tight ground-truth line boxes; determinism test plus a model-gated\n  baseline test (skips when artifacts are absent).\n- Recorded baseline in docs/ocr-model-metrics.md: CER = 0.0 and\n  WER = 0.0 on both samples; line IoU 0.937 (Cyrillic) / 0.827 (Latin).\n  Thresholds gate at 0.05 / 0.10 / 0.80-0.70 with margins.\n- Design doc slice 4 marked done; README points at the baseline.\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs\n\n* chore(deny): ignore RUSTSEC-2026-0192 (ttf-parser unmaintained, dev-only via ab_glyph)\n\nThe metrics corpus rasterizer (ab_glyph, dev-dependency) pulls\nowned_ttf_parser -> ttf-parser, now flagged unmaintained. Not a\nvulnerability and never part of the shipped library; revisit when\nab_glyph migrates (skrifa is the suggested successor).\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs",
          "timestamp": "2026-08-12T01:57:37+05:00",
          "tree_id": "b396d14cd7e3942c85986940ff67decd97c412c4",
          "url": "https://github.com/matyushkin/djvu-rs/commit/667003e2c53e7eab07392bbfc412d77ed26975dc"
        },
        "date": 1786483271807,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 124000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 6891000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 42366000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 40422000,
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
          "id": "0c0dd39fb98e59a661581c0035c79217aa156b41",
          "message": "chore(main): release 0.28.0 (#701)\n\nCo-authored-by: github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>",
          "timestamp": "2026-08-12T02:32:18+05:00",
          "tree_id": "d767717c30923168aad618aaaced18902051e862",
          "url": "https://github.com/matyushkin/djvu-rs/commit/0c0dd39fb98e59a661581c0035c79217aa156b41"
        },
        "date": 1786485449404,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 163000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8249000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49350000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 47379000,
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
          "id": "fdd606445944a7b33d97d0e799f4f654ef250d98",
          "message": "feat(ingest): decode CCITT G4 and PackBits TIFF strips (#694) (#756)\n\nAccept compression 4 (CCITT G4, bilevel only) and 32773 (PackBits) in\nthe raw TIFF strip reader. G4 reuses the existing T.6 decoder in\nsrc/smmr.rs via a new crate-internal decode_g4_rows helper; each strip\nis an independent stream with a fresh all-white reference line.\nBlackIsZero G4 renders inverted, matching libtiff. T6Options\nuncompressed mode, palette+G4, and remaining compressions (CCITT\nRLE/G3, LZW, Deflate, JPEG) keep targeted errors.\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs",
          "timestamp": "2026-08-18T10:45:06+05:00",
          "tree_id": "172303d7f4df66efcb254fb2fb4d6e4a04af0405",
          "url": "https://github.com/matyushkin/djvu-rs/commit/fdd606445944a7b33d97d0e799f4f654ef250d98"
        },
        "date": 1787033496218,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 161000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 9425000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 53906000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 51783000,
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
          "id": "5ec6a12db4251e567f948a6c9767a5c7704ab8cc",
          "message": "feat(ingest): map TIFF X/YResolution tags to default DPI (#694) (#758)\n\n--dpi is now optional. Without it, a TIFF input's XResolution\n(YResolution as fallback) + ResolutionUnit tags set the INFO dpi:\ninch directly, cm converted (x2.54), rationals honored, sane range\n25..=6000. ResolutionUnit 1 (no absolute unit) and missing/unusable\ntags keep the historical 300 default. An explicit --dpi always wins.\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs",
          "timestamp": "2026-08-18T16:29:50+05:00",
          "tree_id": "5d77fd9c406df1fdb012be362d5c3d896232acfb",
          "url": "https://github.com/matyushkin/djvu-rs/commit/5ec6a12db4251e567f948a6c9767a5c7704ab8cc"
        },
        "date": 1787054113069,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 164000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8288000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49403000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 47813000,
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
          "id": "afa2c67b43155b803ec4fce5bea606622a924054",
          "message": "perf(ingest): bilevel TIFF fast path straight to JB2 masks (#694) (#759)\n\n* perf(ingest): bilevel TIFF fast path straight to JB2 masks (#694)\n\n1-bit single-sample TIFF pages now decode directly to packed Bitmap\nmasks via png_io::decode_tiff_file_to_bitmaps, skipping the 32x RGBA\nexpansion and the segmentation pass. TIFF packed rows and Bitmap share\nthe MSB-first byte-padded layout, so WhiteIsZero rows copy through and\nBlackIsZero rows invert (padding bits cleared).\n\nThe CLI takes this path for --quality lossless and --quality auto;\nmasks are identical to the fixed-threshold segmentation of the RGBA\nroute, so output bytes do not change. Under auto, 1-bit pages are\nbilevel by construction and resolve to Lossless without pixel\nstatistics (blank 1-bit pages now go lossless too). Any non-1-bit\npage falls the file back to the RGBA route.\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs\n\n* docs(perf): record bilevel TIFF fast-path measurements (#694)\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs",
          "timestamp": "2026-08-18T17:05:38+05:00",
          "tree_id": "550925f852827d1b93ae277c5892ac03815e56ee",
          "url": "https://github.com/matyushkin/djvu-rs/commit/afa2c67b43155b803ec4fce5bea606622a924054"
        },
        "date": 1787056152565,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 123000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 6023000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 35561000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 33962000,
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
          "id": "4b600d01fd86b9dd660b8302b83f94d8e40eb2c1",
          "message": "chore(main): release 0.29.0 (#757)\n\nCo-authored-by: github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>",
          "timestamp": "2026-08-18T20:53:40+05:00",
          "tree_id": "bb5daf3360f4098a81cbce54a3b3966b5eb7d698",
          "url": "https://github.com/matyushkin/djvu-rs/commit/4b600d01fd86b9dd660b8302b83f94d8e40eb2c1"
        },
        "date": 1787070029722,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 163000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8356000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49748000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 47587000,
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
          "id": "bf20e3b5e28f861a2f772cebc47ce558812cfdf0",
          "message": "feat(ingest): apply TIFF Orientation tag exactly once at ingest (#694) (#760)\n\nThe tiff crate never applies tag 274, so ingest now does it once per\npage: mirrors (2, 4), 180-degree rotation (3), transpose variants\n(5, 7), and 90-degree rotations (6, 8) with swapped dimensions. Both\nthe RGBA route and the bilevel fast path orient identically; for\norientations 5-8 the DPI mapping prefers the stored YResolution,\nwhich becomes the visual horizontal density. Out-of-range values are\ntreated as upright (libtiff-compatible). JPEG EXIF orientation stays\non the #694 follow-up list.\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs",
          "timestamp": "2026-08-18T22:40:53+05:00",
          "tree_id": "9b18a2e396b7a15497371b963c4d99b547ca972d",
          "url": "https://github.com/matyushkin/djvu-rs/commit/bf20e3b5e28f861a2f772cebc47ce558812cfdf0"
        },
        "date": 1787076493051,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 161000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8810000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 53460000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 51509000,
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
          "id": "238c21b9e4d642e5b00f81db335bc5cb9871f483",
          "message": "test(ingest): pin CMYK/YCCK JPEG decode behaviour (#694) (#762)\n\nzune-jpeg 0.5 already decodes Adobe CMYK and YCCK JPEGs (baseline and\nprogressive) to RGB with the same profile-free (255-ink)*(255-K)/255 mix\nour CMYK TIFF ingest documents; verified pixel-exact against Pillow on\nfive sample files. The \"not yet supported\" matrix entry was stale.\n\nAdd four regression tests (CMYK, YCCK, progressive, CLI encode round\ntrip), update the supported-input matrix, and drop the follow-up item.\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs",
          "timestamp": "2026-08-18T23:19:01+05:00",
          "tree_id": "e92233ba21faacd45013b92bcc702dc10e9f2f31",
          "url": "https://github.com/matyushkin/djvu-rs/commit/238c21b9e4d642e5b00f81db335bc5cb9871f483"
        },
        "date": 1787078718093,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 165000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8234000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 50283000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 48202000,
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
          "id": "70908a499184de90537b8151cf9d7e602c70ebeb",
          "message": "feat(ingest): apply JPEG EXIF orientation exactly once at ingest (#694) (#763)\n\nEXIF inherited TIFF's Orientation tag (274, values 1-8), so the pixmap\nreorientation helpers move out of the tiff_ingest module to the shared\ntop level of png_io. decode_jpeg_file_to_pixmap now reads the raw EXIF\nblock zune-jpeg exposes (it never applies orientation itself), parses\ntag 274 from IFD0 in either byte order, and reorients the decoded page\nonce. Malformed EXIF, a wrong entry type, or an out-of-range value\nfalls back to upright, matching the TIFF ingest behaviour.\n\nFour new tests: all eight orientations against hand-written index\npermutations, big-endian EXIF, malformed/out-of-range payloads, and a\nCLI round trip checking the swapped page dimensions.\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs",
          "timestamp": "2026-08-18T23:41:37+05:00",
          "tree_id": "0c3b13d6da509c4e434e544c6e824db63b9cc094",
          "url": "https://github.com/matyushkin/djvu-rs/commit/70908a499184de90537b8151cf9d7e602c70ebeb"
        },
        "date": 1787080251960,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 166000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8250999,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49723000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 47768000,
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
          "id": "0abae18d5b0ea32f3e3855369ab07b6db2694159",
          "message": "feat(ingest): configurable alpha compositing via encode --background (#694) (#764)\n\nImplement AlphaCompositing::CompositeOnBackground (declared but dead until\nnow): blend every non-opaque RGBA pixel onto a solid background at decode\ntime with deterministic integer rounding, out = (c*a + bg*(255-a) + 127)/255,\nand set alpha to 255. Preserve stays the default and is a no-op.\n\nWire it through the CLI as `djvu encode --background <COLOR>` (RRGGBB hex\nwith optional '#', or white/black), threading an IngestPolicy into all\nencode decode call sites via decode_image_to_pixmap_with_policy. The policy\napplies uniformly to PNG and TIFF (after orientation); JPEG never carries\nalpha and the bilevel TIFF fast path never expands to RGBA.\n\nTests: unit coverage of apply(); integration coverage for PNG and RGBA8\nTIFF compositing (GrayA8 is untestable — tiff 0.9 rejects BlackIsZero with\ntwo samples), the preserve default, CLI colour parsing (white/hex/#hex),\nand rejection of invalid colours. Docs and README updated.\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs",
          "timestamp": "2026-08-19T00:14:03+05:00",
          "tree_id": "30722e72bff4624b42ba72560795d48d889260a1",
          "url": "https://github.com/matyushkin/djvu-rs/commit/0abae18d5b0ea32f3e3855369ab07b6db2694159"
        },
        "date": 1787082024975,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 164000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8228000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 49558000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 47647000,
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
          "id": "0e5625060b6ef4a9348870ae414dadc49cfe1e5e",
          "message": "feat(ingest): explicit ICC profile policy via encode --icc (#694) (#765)\n\nAdd IccHandling to IngestPolicy: Ignore (default) decodes pixel bytes as-is\nand drops the embedded profile — the long-standing behaviour, now explicit;\nReject fails with an error naming the source and profile size. DjVu has no\ncontainer for ICC profiles and ingest applies no colour management, so a\nprofile can never survive into the output; the policy makes that explicit\ninstead of silent. A Transform mode would need a colour-management engine\nand stays out of scope.\n\nDetection covers every ingest route: PNG iCCP chunk, JPEG APP2 ICC_PROFILE\nsegment (new decode_jpeg_file_to_pixmap_with_policy, dispatcher now routes\nJPEG through it), TIFF InterColorProfile tag 34675 on every page of the\nRGBA route, and the bilevel fast path (decode_tiff_file_to_bitmaps now\ntakes an IngestPolicy). An unreadable TIFF ICC tag under Reject is an\nerror, not a silent pass.\n\nCLI: `djvu encode --icc <ignore|reject>` (default ignore).\n\nTests: default-ignores + rejects for PNG, JPEG, TIFF, and the bilevel fast\npath, plus CLI end-to-end for both modes. Docs and README updated; the\ninitial #694 target coverage is now complete.\n\nClaude-Session: https://claude.ai/code/session_019AMBnPFkbYDEcPRRwTShTs",
          "timestamp": "2026-08-19T00:42:23+05:00",
          "tree_id": "84bbd7513c9afaf2088017e7b41e564002f285ce",
          "url": "https://github.com/matyushkin/djvu-rs/commit/0e5625060b6ef4a9348870ae414dadc49cfe1e5e"
        },
        "date": 1787083717618,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 160000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8778000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 53759000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 54615000,
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
          "id": "f94a7874434e401db7cee72221bdce2b08db8bcb",
          "message": "chore(main): release 0.30.0 (#761)\n\nCo-authored-by: github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>",
          "timestamp": "2026-08-19T01:01:56+05:00",
          "tree_id": "277021b7bae16ed141e7d99fdaa7e59d387d779c",
          "url": "https://github.com/matyushkin/djvu-rs/commit/f94a7874434e401db7cee72221bdce2b08db8bcb"
        },
        "date": 1787085238336,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 125000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 6104000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 37969000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 35127000,
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
          "id": "544d930f97c37f74ff9c658f89350ab385571e8c",
          "message": "ci(lint): allow clippy 1.98 chunks_exact_to_as_chunks pending #768 (#769)\n\n* ci(lint): allow clippy 1.98 chunks_exact_to_as_chunks pending #768\n\nStable clippy 1.98 added chunks_exact_to_as_chunks, which flags every\nconstant-size chunks_exact(N) iteration — ~50 sites across the\nworkspace, mostly hot RGBA decode/encode loops — turning the Lint gate\nred on every PR. Allow the lint in both canonical clippy invocations\n(ci.yml + scripts/check.sh) and track the deliberate as_chunks\nmigration, with benchmarks, in #768. -A unknown_lints keeps pre-1.98\nlocal toolchains green on the same command line.\n\nClaude-Session: https://claude.ai/code/session_019M9Rcr2Un4bMX4917n9XZU\n\n* fix(iw44): initialize payload_start from the branch expression\n\nClippy 1.98 extends needless_late_init across intervening statements and\nflags the declare-then-assign payload_start in decode_chunk. Same\nbehaviour, expression form.\n\nClaude-Session: https://claude.ai/code/session_019M9Rcr2Un4bMX4917n9XZU\n\n* fix(lint): initialize late-declared locals from branch expressions\n\nClippy 1.98 extends needless_late_init to declare-then-assign patterns\nwith intervening statements. Convert the three convertible sites\n(smmr v_offset, diff_fuzz their_level, jbig2 probe inputs) to\nexpression form; borrow-extension holders (pdf fg_owned,\ndjvu_encode owned_pixmap) stay as-is — the lint permits them.\n\nClaude-Session: https://claude.ai/code/session_019M9Rcr2Un4bMX4917n9XZU",
          "timestamp": "2026-08-22T15:27:04Z",
          "tree_id": "ca162586061fd17ae043030ac7c64bae166ccf47",
          "url": "https://github.com/matyushkin/djvu-rs/commit/544d930f97c37f74ff9c658f89350ab385571e8c"
        },
        "date": 1787413821598,
        "tool": "cargo",
        "benches": [
          {
            "name": "bzz_decode",
            "value": 73,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "bzz_encode",
            "value": 99257,
            "range": "± 398",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode",
            "value": 131372,
            "range": "± 161",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_first_chunk",
            "value": 648019,
            "range": "± 989",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_corpus_bilevel",
            "value": 367996,
            "range": "± 21334",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_decode_corpus_color",
            "value": 951642,
            "range": "± 4551",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_decode_large_600dpi",
            "value": 1869,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray",
            "value": 8861755,
            "range": "± 171233",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct",
            "value": 3234009,
            "range": "± 22785",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub2",
            "value": 2076409,
            "range": "± 72150",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub2",
            "value": 783550,
            "range": "± 1110",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/rgb_then_gray_sub4",
            "value": 523063,
            "range": "± 1845",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_gray_decode_large/gray_direct_sub4",
            "value": 198585,
            "range": "± 405",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub1_full_decode",
            "value": 7970649,
            "range": "± 97983",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub4_partial_decode",
            "value": 476776,
            "range": "± 12224",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_to_rgb_colorbook/sub2_partial_decode",
            "value": 2076719,
            "range": "± 89148",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_color",
            "value": 2743721,
            "range": "± 4354",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_large_1024x1024",
            "value": 56868039,
            "range": "± 4464655",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode",
            "value": 116745,
            "range": "± 296",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_multitile",
            "value": 18977121,
            "range": "± 362152",
            "unit": "ns/iter"
          },
          {
            "name": "jb2_encode_dict",
            "value": 7365796,
            "range": "± 33601",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color",
            "value": 2435195,
            "range": "± 4707",
            "unit": "ns/iter"
          },
          {
            "name": "segment_page_color_sauvola",
            "value": 10232258,
            "range": "± 48018",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality",
            "value": 5369096,
            "range": "± 26166",
            "unit": "ns/iter"
          },
          {
            "name": "encode_color_page_quality_bgheavy",
            "value": 43345477,
            "range": "± 439422",
            "unit": "ns/iter"
          },
          {
            "name": "encode_large/encode_color_page_quality_large",
            "value": 762259678,
            "range": "± 14889106",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_layered_shared",
            "value": 15057687,
            "range": "± 41234",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/encode_djvm_bundle_jb2",
            "value": 317673801,
            "range": "± 470046",
            "unit": "ns/iter"
          },
          {
            "name": "encode_multipage/cluster_shared_symbols_517p",
            "value": 13567282967,
            "range": "± 7047530",
            "unit": "ns/iter"
          },
          {
            "name": "iw44_encode_gray_1024x1024",
            "value": 13004948,
            "range": "± 52978",
            "unit": "ns/iter"
          },
          {
            "name": "render_region_bilevel",
            "value": 14409101,
            "range": "± 119726",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/72",
            "value": 148077,
            "range": "± 255",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/144",
            "value": 828855,
            "range": "± 1413",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/300",
            "value": 3591257,
            "range": "± 104840",
            "unit": "ns/iter"
          },
          {
            "name": "render_page/dpi/600",
            "value": 14343012,
            "range": "± 64566",
            "unit": "ns/iter"
          },
          {
            "name": "render_coarse",
            "value": 855092,
            "range": "± 1281",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook",
            "value": 8470597,
            "range": "± 17745",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_stages/full_render",
            "value": 8467551,
            "range": "± 4854",
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
            "value": 4629248,
            "range": "± 3110",
            "unit": "ns/iter"
          },
          {
            "name": "render_colorbook_cold",
            "value": 21640910,
            "range": "± 62567",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_color",
            "value": 75754369,
            "range": "± 308628",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel",
            "value": 71096802,
            "range": "± 170190",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/72",
            "value": 3433989,
            "range": "± 7333",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/150",
            "value": 13476617,
            "range": "± 21205",
            "unit": "ns/iter"
          },
          {
            "name": "render_corpus_bilevel_dpi/dpi/300",
            "value": 71390898,
            "range": "± 306461",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/watchmaker_color",
            "value": 76095186,
            "range": "± 79705",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/watchmaker_color",
            "value": 73244685,
            "range": "± 70584",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/watchmaker_color",
            "value": 71986376,
            "range": "± 45440",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/watchmaker_color",
            "value": 2889094,
            "range": "± 1878",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/watchmaker_color",
            "value": 3692916,
            "range": "± 1924",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_pixmap/cable_bilevel",
            "value": 71477936,
            "range": "± 222611",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_into_reuse_buffer/cable_bilevel",
            "value": 68599272,
            "range": "± 52372",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/render_streaming_discard/cable_bilevel",
            "value": 67308301,
            "range": "± 120820",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/mask_decode/cable_bilevel",
            "value": 305661,
            "range": "± 6076",
            "unit": "ns/iter"
          },
          {
            "name": "render_native_stages/bg_to_rgb_warm/cable_bilevel",
            "value": 3696169,
            "range": "± 22048",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_native_cached",
            "value": 73242849,
            "range": "± 108970",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/bilevel_native_cached",
            "value": 68642933,
            "range": "± 45343",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_cached",
            "value": 8417790,
            "range": "± 11548",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/color_downscale_mixed_cached",
            "value": 22401676,
            "range": "± 11897",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/small_color_downscale_cached",
            "value": 70060,
            "range": "± 104",
            "unit": "ns/iter"
          },
          {
            "name": "render_compositor_only/palette_native_cached",
            "value": 70898549,
            "range": "± 74575",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_dpi72",
            "value": 143824,
            "range": "± 125",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_dpi72",
            "value": 146259,
            "range": "± 127",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/thumbnail_half_bilinear",
            "value": 70048,
            "range": "± 70",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/thumbnail_half_bilinear",
            "value": 71712,
            "range": "± 94",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/colorbook_downscale",
            "value": 8407959,
            "range": "± 40017",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/colorbook_downscale",
            "value": 8484020,
            "range": "± 8015",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_color_native",
            "value": 73183256,
            "range": "± 68039",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_color_native",
            "value": 73527698,
            "range": "± 265225",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/direct_render_into/corpus_bilevel_native",
            "value": 68643625,
            "range": "± 40531",
            "unit": "ns/iter"
          },
          {
            "name": "render_row_scratch_ab/row_scratch_copy/corpus_bilevel_native",
            "value": 69061579,
            "range": "± 35111",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/bilinear",
            "value": 76054,
            "range": "± 1235",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_0.5x/lanczos3",
            "value": 1058347,
            "range": "± 4823",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/bilinear",
            "value": 17550820,
            "range": "± 16958",
            "unit": "ns/iter"
          },
          {
            "name": "render_scaled_large_colorbook/lanczos3",
            "value": 250163779,
            "range": "± 212685",
            "unit": "ns/iter"
          },
          {
            "name": "pdf_export_sequential",
            "value": 970716198,
            "range": "± 496903",
            "unit": "ns/iter"
          },
          {
            "name": "export/pdf_flatdecode",
            "value": 1348403944,
            "range": "± 759474",
            "unit": "ns/iter"
          },
          {
            "name": "parse_multipage_520p",
            "value": 1822140,
            "range": "± 43656",
            "unit": "ns/iter"
          },
          {
            "name": "iterate_pages_520p",
            "value": 432,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "open_and_render_first_page_520p",
            "value": 13850082,
            "range": "± 62202",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_first_page",
            "value": 5227219,
            "range": "± 30004",
            "unit": "ns/iter"
          },
          {
            "name": "render_large_doc_mid_page",
            "value": 5223727,
            "range": "± 29390",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_large_600dpi",
            "value": 2768189,
            "range": "± 3265",
            "unit": "ns/iter"
          },
          {
            "name": "decode_mask_mid_600dpi",
            "value": 20051681,
            "range": "± 61693",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_single_page",
            "value": 5529,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "text_extraction_cold",
            "value": 118531,
            "range": "± 10579",
            "unit": "ns/iter"
          },
          {
            "name": "shared_dict_mask_decode_30p",
            "value": 93572157,
            "range": "± 218953",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_bilevel_20p_128px_cold",
            "value": 10778661,
            "range": "± 84012",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_bilevel_20p_128px_cold",
            "value": 253123795,
            "range": "± 920589",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_th44_only_grid_color_6p_128px_cold",
            "value": 2163139,
            "range": "± 3891",
            "unit": "ns/iter"
          },
          {
            "name": "thumbnails_render_only_grid_color_6p_128px_cold",
            "value": 28199337,
            "range": "± 96294",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_newspaper",
            "value": 4882729,
            "range": "± 14075",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_mixed_layout",
            "value": 70612079,
            "range": "± 1244293",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_map",
            "value": 59965054,
            "range": "± 140052",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cjk",
            "value": 61636095,
            "range": "± 114630",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_cyrillic",
            "value": 116216,
            "range": "± 186",
            "unit": "ns/iter"
          },
          {
            "name": "tier2_corpus_render/tier2_photo",
            "value": 22294411,
            "range": "± 48313",
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
            "value": 6744000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 42109000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 40278000,
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
          "id": "41f3b1a83c610b8e655c8d78f266ab20c2b1d4a9",
          "message": "perf: migrate constant-size chunks_exact(N) to as_chunks (#768) (#770)\n\nClippy 1.98's chunks_exact_to_as_chunks flagged ~82 constant-size\nchunks_exact(N)/chunks_exact_mut(N) sites across 28 files. Migrate them\nall to as_chunks::<N>()/as_chunks_mut::<N>() (stable since 1.88 = MSRV):\nthe compiler sees the chunk length statically, so per-chunk bounds\nchecks disappear. Iterator-adapter heads get an explicit .iter()/\n.iter_mut(); the JB2 bit-unpack sites move from chunks_exact_mut(8) +\ninto_remainder() to the (chunks, tail) tuple. Variable-size\nchunks_exact(stride) and rayon par_chunks_exact_mut stay.\n\nDrop the temporary -A unknown_lints -A clippy::chunks_exact_to_as_chunks\nflags from scripts/check.sh and .github/workflows/ci.yml.\n\nBenchmarks (PERF_EXPERIMENTS.md round 113): render_page/dpi/72 -26.6%,\nrender_corpus_bilevel_dpi 150 -35.0%, render_colorbook -16.3%,\niw44_gray_decode_large/gray_direct -24.2%, jb2_encode_dict -7.6%;\napparent encode regressions are in untouched code paths and flip sign\non re-run (machine noise).\n\nCloses #768\n\nClaude-Session: https://claude.ai/code/session_019M9Rcr2Un4bMX4917n9XZU",
          "timestamp": "2026-08-22T17:57:00Z",
          "tree_id": "7a63b0a632f7779cae3ae094df932af4ef1b9da0",
          "url": "https://github.com/matyushkin/djvu-rs/commit/41f3b1a83c610b8e655c8d78f266ab20c2b1d4a9"
        },
        "date": 1787422963774,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 160000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8768000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 53802000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 51658000,
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
          "id": "a096d88c80ce330b61e72afa2368194b75bb2cdb",
          "message": "chore(main): release 0.30.1 (#771)\n\nCo-authored-by: github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>",
          "timestamp": "2026-08-23T20:19:47+06:00",
          "tree_id": "5ffcdd5d514036cb2ce22e2228c476adfa25be90",
          "url": "https://github.com/matyushkin/djvu-rs/commit/a096d88c80ce330b61e72afa2368194b75bb2cdb"
        },
        "date": 1787496412443,
        "tool": "cargo",
        "benches": [
          {
            "name": "djvulibre_render_dpi_72",
            "value": 161000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_150",
            "value": 8688000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 53556000,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "djvulibre_render_dpi_300",
            "value": 51737000,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}