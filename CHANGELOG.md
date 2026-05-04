# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.16.1](https://github.com/matyushkin/djvu-rs/compare/v0.16.0...v0.16.1) (2026-05-04)

### Fixed

* Gate the async lazy example behind the `async` feature so default CI and
  release validation do not build it without `tokio`.
* Fix no-default-features wasm builds after the workspace crate split.
* Publish all extracted workspace crates in dependency order before publishing
  the umbrella `djvu-rs` crate.

## [0.16.0](https://github.com/matyushkin/djvu-rs/compare/v0.15.0...v0.16.0) (2026-05-04)


### Features

* **async:** add native lazy page loader ([735a226](https://github.com/matyushkin/djvu-rs/commit/735a22615ad91c89b2a38c865407a46319c46815))
* **async:** add wasm lazy reader entrypoint ([77fc6ff](https://github.com/matyushkin/djvu-rs/commit/77fc6ffecd57cb5760f9a7e75333987b3530c872))
* **async:** resolve lazy shared dictionaries ([30d8ac9](https://github.com/matyushkin/djvu-rs/commit/30d8ac9f2a4be61c5e82b4bea5f1d8083d4bcd07))

## [Unreleased]

### Changed

* **crates:** split codec primitives into publishable workspace crates: `djvu-zp`,
  `djvu-bzz`, `djvu-iff`, `djvu-bitmap`, `djvu-jb2`, `djvu-pixmap`, and
  `djvu-iw44` ([#229](https://github.com/matyushkin/djvu-rs/issues/229)).
  The umbrella `djvu-rs` crate keeps the historical module paths as re-export
  shims, while consumers that only need `djvu_iff::parse_form` can depend on
  `djvu-iff` directly; a path-dependency cold `cargo check` for an iff-only
  consumer measured 3.67 s.

## [0.15.0](https://github.com/matyushkin/djvu-rs/compare/v0.14.0...v0.15.0) (2026-05-03)


### Features

* **api:** bundled DJVM mutation + set_bookmarks (PR3 of [#222](https://github.com/matyushkin/djvu-rs/issues/222)) ([#268](https://github.com/matyushkin/djvu-rs/issues/268)) ([6672a93](https://github.com/matyushkin/djvu-rs/commit/6672a936202f66b32e0e77dafdf467503547de6e))
* **api:** DjVuDocumentMut::from_bytes — chunk-replacement primitive (PR1 of [#222](https://github.com/matyushkin/djvu-rs/issues/222)) ([#263](https://github.com/matyushkin/djvu-rs/issues/263)) ([b6279ae](https://github.com/matyushkin/djvu-rs/commit/b6279aed03de0eae42b1df7ee22da0bb56efaf08))
* **api:** high-level setters for DjVuDocumentMut (PR2 of [#222](https://github.com/matyushkin/djvu-rs/issues/222)) ([#267](https://github.com/matyushkin/djvu-rs/issues/267)) ([eec0815](https://github.com/matyushkin/djvu-rs/commit/eec08153575052d81f71eb5382176816f1592aff))
* **async:** load_document_async — buffered AsyncRead constructor ([#196](https://github.com/matyushkin/djvu-rs/issues/196) Phase 1) ([#231](https://github.com/matyushkin/djvu-rs/issues/231)) ([2d85e65](https://github.com/matyushkin/djvu-rs/commit/2d85e65e1963efb88bb86fe4b1b7e706228e910d))
* **async:** page_byte_range API + streaming async loader ([#196](https://github.com/matyushkin/djvu-rs/issues/196) Phase 2) ([#237](https://github.com/matyushkin/djvu-rs/issues/237)) ([a365abb](https://github.com/matyushkin/djvu-rs/commit/a365abb64b06a1c86de87e230623e05f465909e1))
* **cli:** multi-page djvu encode from a directory of PNGs ([#223](https://github.com/matyushkin/djvu-rs/issues/223) follow-up) ([#245](https://github.com/matyushkin/djvu-rs/issues/245)) ([b8858bb](https://github.com/matyushkin/djvu-rs/commit/b8858bb36366be29441c17a06f0ea75daa4a210a))
* **djvu-enc:** high-level PageEncoder for bilevel Lossless ([#218](https://github.com/matyushkin/djvu-rs/issues/218)) ([#243](https://github.com/matyushkin/djvu-rs/issues/243)) ([afefcef](https://github.com/matyushkin/djvu-rs/commit/afefcef00cb2026772b4b22eb896c5533f87b844))
* **djvu-enc:** minimal layered Quality (segment → Sjbz + BG44) ([#246](https://github.com/matyushkin/djvu-rs/issues/246)) ([2febd04](https://github.com/matyushkin/djvu-rs/commit/2febd0486b839d800390e0abc1729965f02ad66c))
* **epub:** opt-in reflowable text section per page ([#228](https://github.com/matyushkin/djvu-rs/issues/228)) ([#240](https://github.com/matyushkin/djvu-rs/issues/240)) ([8207fb9](https://github.com/matyushkin/djvu-rs/commit/8207fb9e3046f594e5fb13150fa2d1220c5b35cf))
* **fgbz-enc:** FGbz foreground palette encoder ([#217](https://github.com/matyushkin/djvu-rs/issues/217)) ([#241](https://github.com/matyushkin/djvu-rs/issues/241)) ([e43bc9f](https://github.com/matyushkin/djvu-rs/commit/e43bc9f81ed3bc5e3cfaf5fa249c651749583a36))
* **jb2-enc:** expose tunable shared-Djbz clustering + corpus harness ([#194](https://github.com/matyushkin/djvu-rs/issues/194) Phase 2) ([#219](https://github.com/matyushkin/djvu-rs/issues/219)) ([9aafe42](https://github.com/matyushkin/djvu-rs/commit/9aafe42f04b6d474fefa0bbc73f06b4c666c9d1f))
* **render:** public render_streaming API (Phase 2 of [#225](https://github.com/matyushkin/djvu-rs/issues/225)) ([#260](https://github.com/matyushkin/djvu-rs/issues/260)) ([b92fac7](https://github.com/matyushkin/djvu-rs/commit/b92fac7c962c57115673eb31b6f84a5e7b36c085))
* **segment + cli:** FG/BG segmentation v1 + djvu encode subcommand ([#220](https://github.com/matyushkin/djvu-rs/issues/220), [#223](https://github.com/matyushkin/djvu-rs/issues/223)) ([#244](https://github.com/matyushkin/djvu-rs/issues/244)) ([4945b06](https://github.com/matyushkin/djvu-rs/commit/4945b0670938f4623e5368c14aa5ba41a0b107a5))
* **smmr-enc:** public Smmr (G4/MMR) encoder API ([#221](https://github.com/matyushkin/djvu-rs/issues/221)) ([#242](https://github.com/matyushkin/djvu-rs/issues/242)) ([009c706](https://github.com/matyushkin/djvu-rs/commit/009c70681eb9c34992ed8a438343638a694cd98b))
* **text:** add reflowable_text() for paragraph reading-order extraction ([#228](https://github.com/matyushkin/djvu-rs/issues/228)) ([#239](https://github.com/matyushkin/djvu-rs/issues/239)) ([221a49a](https://github.com/matyushkin/djvu-rs/commit/221a49a1c066bdef59d5dcabc26961e8d6283752))


### Bug Fixes

* **iff:** preserve FORM length parity for byte-identical mutation (PR4 of [#222](https://github.com/matyushkin/djvu-rs/issues/222)) ([#269](https://github.com/matyushkin/djvu-rs/issues/269)) ([df1e95e](https://github.com/matyushkin/djvu-rs/commit/df1e95e51c08ebe4e85cb1899352aae0cf5a5692))
* **iw44:** correct vext lane in prelim_flags_band0_neon horizontal-OR ([#266](https://github.com/matyushkin/djvu-rs/issues/266)) ([b390681](https://github.com/matyushkin/djvu-rs/commit/b3906813dbe4d4c9946c93b9b9e6884c1da62efc))
* **jb2-enc:** cap cluster_shared_symbols pixel budget at decoder limit ([#270](https://github.com/matyushkin/djvu-rs/issues/270)) ([#271](https://github.com/matyushkin/djvu-rs/issues/271)) ([78e5c79](https://github.com/matyushkin/djvu-rs/commit/78e5c793d3d7a86512e42820d8e3cf0c9870c48e))
* **render:** scale page-space coords into FG44 + BG plane space ([#199](https://github.com/matyushkin/djvu-rs/issues/199)) ([#248](https://github.com/matyushkin/djvu-rs/issues/248)) ([e2c07df](https://github.com/matyushkin/djvu-rs/commit/e2c07df45d5cfbc1edbef07553549e4fbf0fd2fc))


### Performance Improvements

* **iw44:** WASM simd128 inverse wavelet load/store (Phase 2 of [#190](https://github.com/matyushkin/djvu-rs/issues/190)) ([#257](https://github.com/matyushkin/djvu-rs/issues/257)) ([353813e](https://github.com/matyushkin/djvu-rs/commit/353813eac5460f1e1f4976fc639881ca8a98a306))
* **iw44:** WASM simd128 ycbcr_raw kernels (Phase 1 of [#190](https://github.com/matyushkin/djvu-rs/issues/190)) ([#253](https://github.com/matyushkin/djvu-rs/issues/253)) ([d59fcee](https://github.com/matyushkin/djvu-rs/commit/d59fcee1c770011a726ee41d77cf8964629dbc94))
* **iw44:** x86_64 AVX2 ports of prelim_flags kernels (Phase 3 of [#189](https://github.com/matyushkin/djvu-rs/issues/189)) ([#261](https://github.com/matyushkin/djvu-rs/issues/261)) ([0ed7a36](https://github.com/matyushkin/djvu-rs/commit/0ed7a36e3c117d83c9bf9dde6df5bd347a2652cf))
* **iw44:** x86_64 AVX2 stride-1 load/store (Phase 2 of [#189](https://github.com/matyushkin/djvu-rs/issues/189)) ([#252](https://github.com/matyushkin/djvu-rs/issues/252)) ([3edf027](https://github.com/matyushkin/djvu-rs/commit/3edf02784f68710f9eb1eeb7762a986a322b0ab3))
* **iw44:** x86_64 AVX2 ycbcr_raw kernels (Phase 1 of [#189](https://github.com/matyushkin/djvu-rs/issues/189)) ([#251](https://github.com/matyushkin/djvu-rs/issues/251)) ([7f3d867](https://github.com/matyushkin/djvu-rs/commit/7f3d8679e68f7c1daa8d7973e82d475ab3974c47))
* **jb2-enc:** opt-in lossy rec-7 near-duplicate substitution (Phase 4 of [#224](https://github.com/matyushkin/djvu-rs/issues/224)) ([#256](https://github.com/matyushkin/djvu-rs/issues/256)) ([98fb8c7](https://github.com/matyushkin/djvu-rs/commit/98fb8c76c22a3eb4a7306585f030605a0941673b))
* **jb2-enc:** per-CC accounting harness for shared-Djbz ([#194](https://github.com/matyushkin/djvu-rs/issues/194) Phase 2.5) ([#255](https://github.com/matyushkin/djvu-rs/issues/255)) ([f09bfdf](https://github.com/matyushkin/djvu-rs/commit/f09bfdf71ff0ce9959baa67aedb0c2fe73535f99))
* **render:** internal row-streaming refactor (Phase 1 of [#225](https://github.com/matyushkin/djvu-rs/issues/225)) ([#259](https://github.com/matyushkin/djvu-rs/issues/259)) ([3d22a59](https://github.com/matyushkin/djvu-rs/commit/3d22a59ae90ccff0cfcc0b4eb3fb21c7bb693dc6))

## [0.14.0](https://github.com/matyushkin/djvu-rs/compare/v0.13.0...v0.14.0) (2026-04-28)


### Features

* **jb2-enc:** multi-page shared Djbz dictionary in DJVM bundle, Phase 1 ([#194](https://github.com/matyushkin/djvu-rs/issues/194)) ([#216](https://github.com/matyushkin/djvu-rs/issues/216)) ([507b061](https://github.com/matyushkin/djvu-rs/commit/507b0618b27dc6619ce33736d01a9e920f2fdb32))
* **jb2-enc:** symbol-dictionary encoder Phases 1-3 ([#188](https://github.com/matyushkin/djvu-rs/issues/188)) ([#213](https://github.com/matyushkin/djvu-rs/issues/213)) ([8a2ed15](https://github.com/matyushkin/djvu-rs/commit/8a2ed150ad86bde2d0305c0b18e1297523590ce2))


### Bug Fixes

* **annotation:** add MAX_SEXPR_DEPTH=64 guard to prevent stack overflow on deeply nested S-expressions ([#200](https://github.com/matyushkin/djvu-rs/issues/200)) ([2f03789](https://github.com/matyushkin/djvu-rs/commit/2f03789638a65f9ac2e6c7cf5763f63ec9a3a0f6))
* **jb2-enc:** tile direct encoder to ≤1MP records ([#198](https://github.com/matyushkin/djvu-rs/issues/198)) ([#214](https://github.com/matyushkin/djvu-rs/issues/214)) ([24ac60f](https://github.com/matyushkin/djvu-rs/commit/24ac60fce1c139a8b55a3d30e71a7c36cac5d6d8))


### Performance Improvements

* **iw44-enc:** NEON forward wavelet + parallel Y/Cb/Cr + skip empty passes ([#206](https://github.com/matyushkin/djvu-rs/issues/206)) ([d1f2765](https://github.com/matyushkin/djvu-rs/commit/d1f27658d9eecfcd741a63788a56be7a891e8ed0))
* **iw44:** NEON decoder — column pass s=2/4, scatter, YCbCr, lifting (~25% to_rgb) ([#207](https://github.com/matyushkin/djvu-rs/issues/207)) ([f570297](https://github.com/matyushkin/djvu-rs/commit/f5702973fd0833ed195554335853325ece4e5f12))
* **jb2-enc:** eliminate bounds checks from JB2 encode hot loop ([#205](https://github.com/matyushkin/djvu-rs/issues/205)) ([5cbeaf0](https://github.com/matyushkin/djvu-rs/commit/5cbeaf0f64afbb2d39b86f568c73d04df9c9d35a))

## [0.13.0](https://github.com/matyushkin/djvu-rs/compare/v0.12.0...v0.13.0) (2026-04-18)


### Features

* **ci:** add ocr-tesseract integration test + CI job ([#178](https://github.com/matyushkin/djvu-rs/issues/178)) ([220671d](https://github.com/matyushkin/djvu-rs/commit/220671d0ae6d21f05a2a847945e062ae27eb931c))
* **epub:** DPI-aware rendering, language tag, hyperlinks, cover image ([191e386](https://github.com/matyushkin/djvu-rs/commit/191e3869623a091998d75761f9be30d75c9576a8))
* **mask:** wire Smmr (G4/MMR) decoder into both render pipelines ([07272b3](https://github.com/matyushkin/djvu-rs/commit/07272b32f44da9b4382bd9f1d4ff7828ce6223d3))
* **tiff:** embed DPI resolution tags in exported TIFF files ([87efb14](https://github.com/matyushkin/djvu-rs/commit/87efb140cc603a3ad4bd0f13b5e7a12c995ca516))


### Bug Fixes

* **ci:** install libleptonica-dev + libtesseract-dev for ocr-tesseract job ([0410f70](https://github.com/matyushkin/djvu-rs/commit/0410f70f0ed73be0425906685c3af1e0b6397374))
* **ci:** split test into test-stable + test-beta jobs (matrix.rust not allowed in job if) ([0d60c76](https://github.com/matyushkin/djvu-rs/commit/0d60c7662a78bff90dad81d9847e07d93dfb3b15))
* **docs:** resolve all remaining broken intra-doc links (0 warnings) ([4ac8bb1](https://github.com/matyushkin/djvu-rs/commit/4ac8bb162d922529ac4e58757155d556fbe3e7fc))
* **docs:** resolve broken intra-doc links in djvu_document ([8dbfda0](https://github.com/matyushkin/djvu-rs/commit/8dbfda050a72df34dbe3f149427aec4a6e4252a9))
* **fuzz:** revert to cargo install cargo-fuzz, add explicit binary cache ([84e6eef](https://github.com/matyushkin/djvu-rs/commit/84e6eef4b18c6f957f327d34fc2f87ab560a5f3c))
* **fuzz:** use correct public module djvu_rs::jb2 in fuzz_jb2 target ([9133de6](https://github.com/matyushkin/djvu-rs/commit/9133de69f3af8ee05f2c8f9239efcf87e2ed0a76))
* **pdf:** Unicode-aware glyph width for invisible text layer ([2398c38](https://github.com/matyushkin/djvu-rs/commit/2398c3851d8cef9c5a2b41dbcdaf5352baf9575a))
* **render:** add #[allow(unsafe_code)] + unsafe blocks for Rust 2024 SIMD ([#169](https://github.com/matyushkin/djvu-rs/issues/169)) ([179d171](https://github.com/matyushkin/djvu-rs/commit/179d17132c31b5ec31eaa99156acc4ac4cdda1f6))
* **render:** correct gamma LUT formula to match DjVuLibre ([#161](https://github.com/matyushkin/djvu-rs/issues/161)) ([cd3228d](https://github.com/matyushkin/djvu-rs/commit/cd3228d812e77f6d9bdcc48e1d9a92f8ff34077e))
* **render:** resolve unsafe_code / unsafe_op_in_unsafe_fn CI conflicts ([889f28f](https://github.com/matyushkin/djvu-rs/commit/889f28fb67c5b13c8f5b4a66ae32b1546a498988))
* **render:** restore bilevel composite fast path, recover 2× regression from [#165](https://github.com/matyushkin/djvu-rs/issues/165) ([46b6931](https://github.com/matyushkin/djvu-rs/commit/46b69318d0e1181d4eec5d9fc428cc5eee9723f6))
* **render:** use core::arch instead of std::arch for no_std compatibility ([122b989](https://github.com/matyushkin/djvu-rs/commit/122b989708e475658a764b2b145a887605590e37))
* resolve issues [#164](https://github.com/matyushkin/djvu-rs/issues/164) [#169](https://github.com/matyushkin/djvu-rs/issues/169) [#170](https://github.com/matyushkin/djvu-rs/issues/170) [#174](https://github.com/matyushkin/djvu-rs/issues/174) [#176](https://github.com/matyushkin/djvu-rs/issues/176) [#177](https://github.com/matyushkin/djvu-rs/issues/177) ([774cbdb](https://github.com/matyushkin/djvu-rs/commit/774cbdb834c703c27ca82ed9f8f3bb4a17504aeb))
* wasm CI job, open_dir API, DPI scaling in OCR export, jb2_new cleanup ([6cbe038](https://github.com/matyushkin/djvu-rs/commit/6cbe038909858f4c523f13d7204998a7109cd84d))
* **zp:** widen a/c/fence fields from u16 to u32 to match jb2 inline decoder ([fb0db12](https://github.com/matyushkin/djvu-rs/commit/fb0db122daa4dccd97f49a971103de0add5a2366))


### Performance Improvements

* **bzz:** inline ZP state locals in MTF decode hot loop ([bad5b21](https://github.com/matyushkin/djvu-rs/commit/bad5b215bfa5384589f2252b3da2ee0ecb9a3ac2))
* **ci:** single nextest pass on main avoids sequential overhead ([f17d901](https://github.com/matyushkin/djvu-rs/commit/f17d901b1055c3cc2fb8e6dbfa3b6d445b6aa357))
* **deps:** split ocr-neural into lightweight stub + ocr-neural-candle ([#175](https://github.com/matyushkin/djvu-rs/issues/175)) ([3a930b2](https://github.com/matyushkin/djvu-rs/commit/3a930b2e38ca939eb15af2eefbb55156a158224b))
* **jb2:** bit-pack Jbm to 1 bit/pixel — 8x memory, corpus −3.9% ([#187](https://github.com/matyushkin/djvu-rs/issues/187)) ([17f331f](https://github.com/matyushkin/djvu-rs/commit/17f331f3d44e9e8b6f2d179b77e7212ba5b47433))
* **render:** x86_64 SSE2/SSSE3 fast paths for alpha fill and RGB→RGBA ([#169](https://github.com/matyushkin/djvu-rs/issues/169)) ([0ea3f3e](https://github.com/matyushkin/djvu-rs/commit/0ea3f3e674ae1eb112fb73368038e3fd0b2e0dde))

## [0.12.0](https://github.com/matyushkin/djvu-rs/compare/v0.11.1...v0.12.0) (2026-04-14)


### Features

* **pdf:** parallel page rendering with rayon ([#148](https://github.com/matyushkin/djvu-rs/issues/148)) ([ae79a7c](https://github.com/matyushkin/djvu-rs/commit/ae79a7c6539fd2173885b18aa4e3beb7146b70b4))
* **wasm:** progressive IW44 render API ([#150](https://github.com/matyushkin/djvu-rs/issues/150)) ([35d3a30](https://github.com/matyushkin/djvu-rs/commit/35d3a30d285fd699781d9c4e5373b584b29f544d))


### Bug Fixes

* **ocr:** resolve all clippy errors in ocr_neural, ocr_onnx, ocr_tesseract ([65a0cee](https://github.com/matyushkin/djvu-rs/commit/65a0cee338411a608c0c8a92d02c3638cb18c153))
* update tesseract API and optimize JB2 inner loop ([f64d884](https://github.com/matyushkin/djvu-rs/commit/f64d88423aff0d4861a6dd50c1ec7a4c45e1d10b))


### Performance Improvements

* allow 1.5× upscale in IW44 subsample selection for faster downscaled renders ([8f0baa2](https://github.com/matyushkin/djvu-rs/commit/8f0baa2a06eadce23974d973b7ee635b5939a5f0))
* downsampled mask pyramid for composite — 8 ms vs 23 ms for 150 dpi renders ([1374f27](https://github.com/matyushkin/djvu-rs/commit/1374f2707ead48b1db2a0eea48c12c8872bf9527))
* eliminate bounds checks in JB2 hot loops and ZP renormalize ([7e94000](https://github.com/matyushkin/djvu-rs/commit/7e94000ace8ed67b5410586563e7168031fa595e))
* **jb2:** close performance gap vs DjVuLibre + CLI improvements ([#159](https://github.com/matyushkin/djvu-rs/issues/159)) ([3efc430](https://github.com/matyushkin/djvu-rs/commit/3efc430234f8848ecc597302c7f6ece8ae9ac887))
* **jb2:** local-copy ZP state for register-allocation + hardware CLZ ([0590d3c](https://github.com/matyushkin/djvu-rs/commit/0590d3cd00dce042132d8759204421a7b1750f33))
* partial BG44 chunk decode for sub=4 renders — skip high-frequency refinement ([b371e4e](https://github.com/matyushkin/djvu-rs/commit/b371e4eebe5cd766114d81dac3480430bc8712ed))
* **pdf:** output_dpi option + bilevel fast path — 2× faster export ([cfccbc6](https://github.com/matyushkin/djvu-rs/commit/cfccbc6deefbf288bcd2055ee27c3fed5d7c9b54)), closes [#147](https://github.com/matyushkin/djvu-rs/issues/147)
* replace bg_subsample division with shift in composite hot path ([0e0f2a3](https://github.com/matyushkin/djvu-rs/commit/0e0f2a3aa52da045126588ec4094fe436ce739c3))
* replace mask division with bit-shift in composite hot path ([e4b7982](https://github.com/matyushkin/djvu-rs/commit/e4b7982903dbaa040e3285ecb5ace4f24ca06b73))
* use chunks_exact_mut in composite loops — eliminate per-pixel bounds checks ([0176860](https://github.com/matyushkin/djvu-rs/commit/01768603ded218496c95678c40b5cc892a9dae9e))

## [0.11.1](https://github.com/matyushkin/djvu-rs/compare/v0.11.0...v0.11.1) (2026-04-13)


### Bug Fixes

* **fuzz:** add [workspace] to fuzz/Cargo.toml to fix cargo-fuzz build ([be7a5a8](https://github.com/matyushkin/djvu-rs/commit/be7a5a80d23ca77c6c326c1c9065e1498ab41b5d))
* **smmr:** replace manual div_ceil with .div_ceil() per clippy ([a8d24ca](https://github.com/matyushkin/djvu-rs/commit/a8d24ca5fa470e9f9c59bee7ef7a605786a8fe79))

## [0.11.0](https://github.com/matyushkin/djvu-rs/compare/v0.10.0...v0.11.0) (2026-04-13)


### Features

* add BZZ encoder with ZP arithmetic coding ([582432f](https://github.com/matyushkin/djvu-rs/commit/582432f601cf546c9404228af1541ec2067b503e))
* add NAVM bookmark encoder and ANTa/ANTz annotation encoder ([#133](https://github.com/matyushkin/djvu-rs/issues/133)) ([#136](https://github.com/matyushkin/djvu-rs/issues/136)) ([9e6bf66](https://github.com/matyushkin/djvu-rs/commit/9e6bf662125194fe57be02bed2ce42d87b8c16b7))
* indirect DJVM — create_indirect() and parse_from_dir() ([#135](https://github.com/matyushkin/djvu-rs/issues/135)) ([#137](https://github.com/matyushkin/djvu-rs/issues/137)) ([d7fbf74](https://github.com/matyushkin/djvu-rs/commit/d7fbf74e45b9f5a85d0f162796709126131ab8a0))
* IW44 wavelet encoder — BG44/FG44 chunk encoding (issue [#131](https://github.com/matyushkin/djvu-rs/issues/131)) ([#139](https://github.com/matyushkin/djvu-rs/issues/139)) ([10adc4f](https://github.com/matyushkin/djvu-rs/commit/10adc4f61bf8a79f9c4ba74559fb004b31d8968c))
* JB2 bilevel image encoder — Sjbz chunk encoding (issue [#132](https://github.com/matyushkin/djvu-rs/issues/132)) ([#140](https://github.com/matyushkin/djvu-rs/issues/140)) ([cf280ba](https://github.com/matyushkin/djvu-rs/commit/cf280ba6e1356f1d8f7498c84c57e2759f603b3c))
* **smmr:** add G4/MMR bilevel image decoder (issue [#134](https://github.com/matyushkin/djvu-rs/issues/134)) ([#138](https://github.com/matyushkin/djvu-rs/issues/138)) ([8bd6e41](https://github.com/matyushkin/djvu-rs/commit/8bd6e4157c885e7a03c0443657d294099fbe7619))


### Bug Fixes

* **ci:** enable cli feature for nextest to build djvu binary ([576f12f](https://github.com/matyushkin/djvu-rs/commit/576f12faf154662a3992ac3a20c3c126e32fc798))
* **ci:** exclude djvu-py from nextest, fix audit advisories ([788b0d3](https://github.com/matyushkin/djvu-rs/commit/788b0d39fc04d1ed28dc5052f2bc88004f106117))
* **ci:** exclude fuzz/ from workspace to fix cargo-fuzz builds ([330ff0a](https://github.com/matyushkin/djvu-rs/commit/330ff0a28a601629718cf0c12d4e90ab93524621))
* **jb2_encode:** return empty Vec for zero-dimension bitmaps; unreachable dead branch ([#142](https://github.com/matyushkin/djvu-rs/issues/142)) ([#143](https://github.com/matyushkin/djvu-rs/issues/143)) ([091b657](https://github.com/matyushkin/djvu-rs/commit/091b6573467c67c3c8d487ee82dba24b7c59f136))

## [0.10.0](https://github.com/matyushkin/djvu-rs/compare/v0.9.0...v0.10.0) (2026-04-10)


### Features

* **djvm:** add merge and split commands for DjVu documents ([#126](https://github.com/matyushkin/djvu-rs/issues/126)) ([37eefd8](https://github.com/matyushkin/djvu-rs/commit/37eefd89f969ade36e3547fce0222a9d23d9c215)), closes [#76](https://github.com/matyushkin/djvu-rs/issues/76)
* **ffi:** add C FFI bindings via extern "C" functions ([#127](https://github.com/matyushkin/djvu-rs/issues/127)) ([2226bdf](https://github.com/matyushkin/djvu-rs/commit/2226bdfad34ef39a08bb9a3b1b220cac2942b30a)), closes [#72](https://github.com/matyushkin/djvu-rs/issues/72)
* **ocr:** pluggable OCR backend trait with Tesseract, ONNX, and Candle backends ([#125](https://github.com/matyushkin/djvu-rs/issues/125)) ([bf26603](https://github.com/matyushkin/djvu-rs/commit/bf26603b714d1721f63beedcf2ad54a5f95e21a0)), closes [#77](https://github.com/matyushkin/djvu-rs/issues/77)
* **python:** add Python bindings via PyO3 ([#128](https://github.com/matyushkin/djvu-rs/issues/128)) ([e250fff](https://github.com/matyushkin/djvu-rs/commit/e250fffcef36531d8641e1ab8031198c30c7d372)), closes [#71](https://github.com/matyushkin/djvu-rs/issues/71)


### Bug Fixes

* eliminate memory leaks and add OOM protection ([25f041d](https://github.com/matyushkin/djvu-rs/commit/25f041d78c513f1e3dfbd87aa69dc4fc261488da))
* **jb2:** prevent infinite loop in decode_num on corrupt streams ([d7bee1e](https://github.com/matyushkin/djvu-rs/commit/d7bee1e8c8c612583a055d40c4aa7a8c81666e47)), closes [#122](https://github.com/matyushkin/djvu-rs/issues/122)

## [0.9.0](https://github.com/matyushkin/djvu-rs/compare/v0.8.0...v0.9.0) (2026-04-09)


### Features

* **epub:** EPUB 3 export — page images, text overlay, navigation (Issue [#74](https://github.com/matyushkin/djvu-rs/issues/74)) ([9a6d155](https://github.com/matyushkin/djvu-rs/commit/9a6d1555c43041b86a783c5967a55607e0090768))
* **wasm:** add WasmPage::text_zones_json() — text selection overlay API ([e513bbf](https://github.com/matyushkin/djvu-rs/commit/e513bbfc4052161f0b61f0c2cc429c58869d7170)), closes [#119](https://github.com/matyushkin/djvu-rs/issues/119)
* **wasm:** add WasmPage::text() — expose page text layer to JS ([35a776d](https://github.com/matyushkin/djvu-rs/commit/35a776d16804b39e1ae8cfddbf18073e4711204c))


### Bug Fixes

* **wasm:** correct render() pixel layout and Uint8ClampedArray allocation ([1a967d2](https://github.com/matyushkin/djvu-rs/commit/1a967d2003bb74f0e42bccc8f108b2f1303617c1))

## [0.8.0](https://github.com/matyushkin/djvu-rs/compare/v0.7.1...v0.8.0) (2026-04-09)


### Features

* **epub:** EPUB 3 export — page images, text overlay, navigation (Issue [#74](https://github.com/matyushkin/djvu-rs/issues/74)) ([9a6d155](https://github.com/matyushkin/djvu-rs/commit/9a6d1555c43041b86a783c5967a55607e0090768))
* **wasm:** add WasmPage::text_zones_json() — text selection overlay API ([e513bbf](https://github.com/matyushkin/djvu-rs/commit/e513bbfc4052161f0b61f0c2cc429c58869d7170)), closes [#119](https://github.com/matyushkin/djvu-rs/issues/119)
* **wasm:** add WasmPage::text() — expose page text layer to JS ([35a776d](https://github.com/matyushkin/djvu-rs/commit/35a776d16804b39e1ae8cfddbf18073e4711204c))
* **wasm:** WebAssembly bindings via wasm-bindgen (Issue [#73](https://github.com/matyushkin/djvu-rs/issues/73)) ([#118](https://github.com/matyushkin/djvu-rs/issues/118)) ([4300939](https://github.com/matyushkin/djvu-rs/commit/4300939b9085725005f7ce0a62d4e34f836367ff))


### Bug Fixes

* **wasm:** correct render() pixel layout and Uint8ClampedArray allocation ([1a967d2](https://github.com/matyushkin/djvu-rs/commit/1a967d2003bb74f0e42bccc8f108b2f1303617c1))

## [0.7.1](https://github.com/matyushkin/djvu-rs/compare/v0.7.0...v0.7.1) (2026-04-08)


### Performance Improvements

* **iw44:** compact-plane wavelet for sub≥2 + correct start_scale (Issue [#115](https://github.com/matyushkin/djvu-rs/issues/115)) ([#116](https://github.com/matyushkin/djvu-rs/issues/116)) ([4fc8921](https://github.com/matyushkin/djvu-rs/commit/4fc8921a7892834b3be97ed952d30000ea467b21))

## [0.7.0](https://github.com/matyushkin/djvu-rs/compare/v0.6.0...v0.7.0) (2026-04-08)


### Features

* **async:** progressive stream render API (Issue [#81](https://github.com/matyushkin/djvu-rs/issues/81)) ([#112](https://github.com/matyushkin/djvu-rs/issues/112)) ([eaff91d](https://github.com/matyushkin/djvu-rs/commit/eaff91d5f5b8ccfa3cf86429339e7c21bcde5f72))
* **ci:** continuous benchmark tracking — PR regression detection (Issue [#88](https://github.com/matyushkin/djvu-rs/issues/88)) ([#109](https://github.com/matyushkin/djvu-rs/issues/109)) ([0dfade5](https://github.com/matyushkin/djvu-rs/commit/0dfade5a8c4ec125bac50ff9deb5df37a159c56f))
* **render:** zero-copy region render — render_region API (Issue [#86](https://github.com/matyushkin/djvu-rs/issues/86)) ([#111](https://github.com/matyushkin/djvu-rs/issues/111)) ([b2aa2a8](https://github.com/matyushkin/djvu-rs/commit/b2aa2a860dc28a63409785af8ef2be55cca40a11))


### Performance Improvements

* **bzz:** parallel inverse-BWT via rayon (Issue [#89](https://github.com/matyushkin/djvu-rs/issues/89)) ([#110](https://github.com/matyushkin/djvu-rs/issues/110)) ([eb5bab0](https://github.com/matyushkin/djvu-rs/commit/eb5bab0024029e494a7ab069db46425e8e75b2f9))
* **iw44:** SIMD row pass — 8 rows at a time with i32x8 ([#107](https://github.com/matyushkin/djvu-rs/issues/107)) ([1418ff4](https://github.com/matyushkin/djvu-rs/commit/1418ff4f3c2cdfd1e0ee7a1210bc3852d0240239))

## [0.6.0](https://github.com/matyushkin/djvu-rs/compare/v0.5.0...v0.6.0) (2026-04-06)


### Features

* hOCR and ALTO XML export for text layer (Issue [#75](https://github.com/matyushkin/djvu-rs/issues/75)) ([#98](https://github.com/matyushkin/djvu-rs/issues/98)) ([263cf14](https://github.com/matyushkin/djvu-rs/commit/263cf1492b57caa2b1d986a7eec8fd4a6cc8305b))
* implement ImageDecoder trait for image-rs integration (Issue [#80](https://github.com/matyushkin/djvu-rs/issues/80)) ([#97](https://github.com/matyushkin/djvu-rs/issues/97)) ([d7e4a64](https://github.com/matyushkin/djvu-rs/commit/d7e4a64e427454f4353f5124d1d03ba4ae31fe8c))
* serde support for metadata, annotations, bookmarks, and text zones (Issue [#82](https://github.com/matyushkin/djvu-rs/issues/82)) ([#96](https://github.com/matyushkin/djvu-rs/issues/96)) ([e872ecd](https://github.com/matyushkin/djvu-rs/commit/e872ecdefbee38e0862c899da3955c2ee98ca233))


### Bug Fixes

* **ci:** use core::mem::take in no_std context; fix clippy redundant-Some in ocr_export test ([#101](https://github.com/matyushkin/djvu-rs/issues/101)) ([cc1cdf1](https://github.com/matyushkin/djvu-rs/commit/cc1cdf136e4eaaf9311170a20cd1f3b5ffd4ce54))
* **jb2:** correct regression test comment for fuzz2 fix ([ee380ae](https://github.com/matyushkin/djvu-rs/commit/ee380ae87d3f589627182b5a1350ae76072eb901))
* **jb2:** guard blit against negative symbol dimensions ([49a3792](https://github.com/matyushkin/djvu-rs/commit/49a3792c764e25594165a10b127612a446d7a732))
* **jb2:** reduce MAX_RECORDS and MAX_SYMBOL_PIXELS to prevent fuzz timeouts ([3292193](https://github.com/matyushkin/djvu-rs/commit/3292193237d733976ea24c879ade958b1021caaa))


### Performance Improvements

* **iw44:** allocate chroma planes at half resolution when chroma_half=true (Issue [#85](https://github.com/matyushkin/djvu-rs/issues/85)) ([#99](https://github.com/matyushkin/djvu-rs/issues/99)) ([927e7c0](https://github.com/matyushkin/djvu-rs/commit/927e7c01580412e45a1487ddb98d62b524eed059))
* **jb2:** reuse scratch buffer across symbol decodes to eliminate per-symbol heap allocations (Issue [#90](https://github.com/matyushkin/djvu-rs/issues/90)) ([#100](https://github.com/matyushkin/djvu-rs/issues/100)) ([12575d0](https://github.com/matyushkin/djvu-rs/commit/12575d095e73e650a5b04f4a5a56624b15c541f9))
* **jb2:** shared dict cache + split_at_mut inner loop (Issue [#87](https://github.com/matyushkin/djvu-rs/issues/87)) ([#106](https://github.com/matyushkin/djvu-rs/issues/106)) ([08ca0f4](https://github.com/matyushkin/djvu-rs/commit/08ca0f43f557024411e765afd9f419c2681f6275))
* **render:** 66% speedup on 600 dpi bilevel pages (Issue [#104](https://github.com/matyushkin/djvu-rs/issues/104)) ([#105](https://github.com/matyushkin/djvu-rs/issues/105)) ([8e5a2f4](https://github.com/matyushkin/djvu-rs/commit/8e5a2f42fdb0872b5a13d2560330346b0dc09989))
* **render:** NEON bilinear vertical pass + 4-byte RGBX stride ([#93](https://github.com/matyushkin/djvu-rs/issues/93)) ([b0dfdb8](https://github.com/matyushkin/djvu-rs/commit/b0dfdb8de78e34f5f7478bc06bf3c3dfad21d1df))
* **render:** precomputed coord tables, zero-copy BG path, remove PageMapper ([cf1a8e9](https://github.com/matyushkin/djvu-rs/commit/cf1a8e99eebf86d47c35fa403603354cbd23a5d7))

## [0.5.3] (unreleased)


### Performance Improvements

* **jb2:** cache shared symbol dictionary to avoid re-decoding Djbz on every `decode_mask()` call — `render_large_doc_first_page` 14.5 ms → 10.5 ms (−28%), `render_large_doc_mid_page` 43.9 ms → 36.2 ms (−18%) (closes [#87](https://github.com/matyushkin/djvu-rs/issues/87))
  - `Document::get_or_decode_dict`: `RwLock<HashMap<usize, Arc<JB2Dict>>>` keyed by Djbz data pointer — multi-page documents decode the shared dictionary once across all pages
  - `decode_bitmap_direct`: `split_at_mut` look-ahead row access eliminates per-pixel `row * width` multiply and 4-comparison bounds checks; `jb2_decode` small-page benchmark: 245 µs → 189 µs (−23%)


## [0.5.2](https://github.com/matyushkin/djvu-rs/compare/v0.5.1...v0.5.2) (2026-04-06)


### Performance Improvements

* **render:** 66% speedup on 600 dpi bilevel pages — `render_large_doc_first_page` 42.7 ms → 14.5 ms (closes [#104](https://github.com/matyushkin/djvu-rs/issues/104))
  - `Pixmap::new`: replaced per-pixel push loop with bulk fill — 18 ms → 0.8 ms (−95%) for a 2649×4530 buffer
  - `composite_bilevel`: row-slice writes + rayon `par_chunks_mut` under `--features parallel`
  - Skip `apply_gamma` for pure bilevel pages (0/255 values, gamma is a mathematical no-op)
  - Parallel Y/Cb/Cr wavelet reconstruction via `rayon::join` under `--features parallel`
  - Parallel bilinear scaler passes under `--features parallel`


## [0.5.0](https://github.com/matyushkin/djvu-rs/compare/v0.4.2...v0.5.0) (2026-04-05)


### Features

* **fuzz:** add render to fuzz_full, add CI fuzz workflow (60 s/target) ([7e3e4eb](https://github.com/matyushkin/djvu-rs/commit/7e3e4eb0d51404d0c460b71ac5a359d4eac6da8b))
* **mmap:** add memory-mapped I/O via MmapDocument ([387a2ea](https://github.com/matyushkin/djvu-rs/commit/387a2ea40696d88906dc5ead848154cf42189c6a)), closes [#70](https://github.com/matyushkin/djvu-rs/issues/70)
* **render:** add rayon-based parallel page rendering ([3dc06f9](https://github.com/matyushkin/djvu-rs/commit/3dc06f991d7e26099ea4623580ebded7022f8775)), closes [#69](https://github.com/matyushkin/djvu-rs/issues/69)


### Bug Fixes

* **clippy:** use `contains()` instead of `iter().any()` in tiff_export ([f9b4c2b](https://github.com/matyushkin/djvu-rs/commit/f9b4c2b7b9ad3aadbc9539aec09ef651f5545312))
* **jb2,iw44:** cap comment bytes and IW44 pixel limit to prevent fuzz timeouts ([49c7b1b](https://github.com/matyushkin/djvu-rs/commit/49c7b1bd29505528d53b70e7ab820e5a9c0eae2e))
* **jb2,iw44:** prevent DoS via refinement bitmaps and uncapped total pixel budget ([3e72cf6](https://github.com/matyushkin/djvu-rs/commit/3e72cf6c33d21661db1198013d789217d8978580))
* **jb2:** add blit-pixel budget to prevent type-7 dict-copy DoS ([1c03505](https://github.com/matyushkin/djvu-rs/commit/1c0350543b870c3b29fe87c1b59370410ba5e464))
* **jb2:** cap decode loop at 1 M records to prevent infinite spin on exhausted ZP input ([0b84f2d](https://github.com/matyushkin/djvu-rs/commit/0b84f2dcadaf4cbbae75bd2e53cb82f7f99d4f2d))
* **jb2:** guard blit fast path against i32 overflow and data buffer overread ([be72d29](https://github.com/matyushkin/djvu-rs/commit/be72d29584170afb0f12a951c10d3f24859ae02f))
* **jb2:** limit symbol bitmap size to 4 MP to prevent DoS via crafted input ([943f25e](https://github.com/matyushkin/djvu-rs/commit/943f25eaacc7c91234caea2e99ff87950f7d632f))


### Performance Improvements

* **iw44:** SIMD-accelerate inverse wavelet transform column pass ([2ac4318](https://github.com/matyushkin/djvu-rs/commit/2ac4318c2c6e56af93ad5fe52670e032a70d378f)), closes [#68](https://github.com/matyushkin/djvu-rs/issues/68)

## [0.4.2](https://github.com/matyushkin/djvu-rs/compare/v0.4.1...v0.4.2) (2026-04-05)

### Documentation

* Rewrite README — DjVuDocument API, CLI examples, PDF/TIFF/async sections

## [0.4.1](https://github.com/matyushkin/djvu-rs/compare/v0.4.0...v0.4.1) (2026-04-05)

### Documentation

* remove stale next-up block from roadmap section

## [0.4.0](https://github.com/matyushkin/djvu-rs/compare/djvu-rs-v0.3.0...djvu-rs-v0.4.0) (2026-04-05)


### Features

* add fit_to_width/height/box to RenderOptions ([#33](https://github.com/matyushkin/djvu-rs/issues/33)) ([b371a93](https://github.com/matyushkin/djvu-rs/commit/b371a93099276cab573244cc262dc6ba093276cf))
* **api:** raw_chunk / all_chunks / chunk_ids on DjVuPage and DjVuDocument (Issue [#43](https://github.com/matyushkin/djvu-rs/issues/43)) ([#54](https://github.com/matyushkin/djvu-rs/issues/54)) ([3135627](https://github.com/matyushkin/djvu-rs/commit/31356279ad2739ae13ab36a144fbaabe5b5f63ab))
* **async:** async render API via tokio::task::spawn_blocking (Issue [#51](https://github.com/matyushkin/djvu-rs/issues/51)) ([#61](https://github.com/matyushkin/djvu-rs/issues/61)) ([452636f](https://github.com/matyushkin/djvu-rs/commit/452636f143c56f4fc674968be5d5a4d8bd15d14a))
* **bench:** add render_scaled and pdf_export benchmarks + BENCHMARKS.md (Issue [#52](https://github.com/matyushkin/djvu-rs/issues/52)) ([#62](https://github.com/matyushkin/djvu-rs/issues/62)) ([a8523c7](https://github.com/matyushkin/djvu-rs/commit/a8523c711c2fc3fc12d1089c5becf1b68af25595))
* **cli:** implement djvu info/render/text — 24/24 tests green ([eb2e9d6](https://github.com/matyushkin/djvu-rs/commit/eb2e9d61f0a12285f9c6d7de4a4665b31f19a32a))
* **cos-djvu:** benchmark suite, corpus infrastructure, BENCHMARKS.md (closes [#282](https://github.com/matyushkin/djvu-rs/issues/282)) ([#332](https://github.com/matyushkin/djvu-rs/issues/332)) ([50b6933](https://github.com/matyushkin/djvu-rs/commit/50b69330c73e5883a2954fe646f2cc7b7ec4e654))
* **cos-djvu:** phase 1 — IFF parser, typed errors, MIT skeleton (closes [#267](https://github.com/matyushkin/djvu-rs/issues/267)) ([#277](https://github.com/matyushkin/djvu-rs/issues/277)) ([1943f3f](https://github.com/matyushkin/djvu-rs/commit/1943f3f1cbef880038065ea377ccdfdff4ee33d0))
* **cos-djvu:** phase 2a — ZP arithmetic coder + BZZ decompressor (closes [#268](https://github.com/matyushkin/djvu-rs/issues/268)) ([#279](https://github.com/matyushkin/djvu-rs/issues/279)) ([4983056](https://github.com/matyushkin/djvu-rs/commit/498305678a0d13947e6fe5473d2bc6250383595d))
* **cos-djvu:** phase-5 rendering pipeline — compositing, gamma, scaling, AA (closes [#273](https://github.com/matyushkin/djvu-rs/issues/273)) ([7f5e161](https://github.com/matyushkin/djvu-rs/commit/7f5e161dd7f13c94ed1fb680966416e339eb80d5))
* **cos-djvu:** phase-6 quality — fuzz targets, benchmarks, no_std, full docs (closes [#274](https://github.com/matyushkin/djvu-rs/issues/274)) ([#324](https://github.com/matyushkin/djvu-rs/issues/324)) ([67fab7f](https://github.com/matyushkin/djvu-rs/commit/67fab7fdce945d972c574f61f9e460c32998a189))
* **cos-djvu:** text layer + annotations extraction (closes [#272](https://github.com/matyushkin/djvu-rs/issues/272)) ([#316](https://github.com/matyushkin/djvu-rs/issues/316)) ([5beeba9](https://github.com/matyushkin/djvu-rs/commit/5beeba96034a55396769d044ff1c20f20dc379a6))
* djvu render --format pdf|cbz, roadmap v0.1 finalised ([7d823f6](https://github.com/matyushkin/djvu-rs/commit/7d823f634a1efbc75f743d5d5a8a4f6056e0b0a0))
* DjVu to PDF converter with text, bookmarks, and hyperlinks ([#2](https://github.com/matyushkin/djvu-rs/issues/2)-[#6](https://github.com/matyushkin/djvu-rs/issues/6)) ([#29](https://github.com/matyushkin/djvu-rs/issues/29)) ([a6f0a74](https://github.com/matyushkin/djvu-rs/commit/a6f0a7486d85146e677ae094396e96b44675e894))
* document model — DjVuDocument, Page, DIRM, NAVM (closes [#271](https://github.com/matyushkin/djvu-rs/issues/271)) ([#283](https://github.com/matyushkin/djvu-rs/issues/283)) ([e36fd41](https://github.com/matyushkin/djvu-rs/commit/e36fd4169b591cb5f4146fc58bd3aaacf464d82b))
* import cos-djvu history, remove GPL legacy code ([0f33110](https://github.com/matyushkin/djvu-rs/commit/0f33110d1846e7114c6a46726c9e088cbef25bea))
* IW44 wavelet decoder with planar YCbCr (closes [#270](https://github.com/matyushkin/djvu-rs/issues/270)) ([#281](https://github.com/matyushkin/djvu-rs/issues/281)) ([f799e70](https://github.com/matyushkin/djvu-rs/commit/f799e7098dfbffdabada8e0c19fea2e31cdac351))
* JB2 bilevel decoder (closes [#269](https://github.com/matyushkin/djvu-rs/issues/269)) ([#280](https://github.com/matyushkin/djvu-rs/issues/280)) ([e2a6898](https://github.com/matyushkin/djvu-rs/commit/e2a6898618765fe96a07ddaac51a40b665b64efb))
* **jb2:** DJVI shared dictionary support via INCL chunks (Issue [#45](https://github.com/matyushkin/djvu-rs/issues/45)) ([#56](https://github.com/matyushkin/djvu-rs/issues/56)) ([86a63cb](https://github.com/matyushkin/djvu-rs/commit/86a63cb975303c14159340da4718ed3e23182e3e))
* mask and foreground/background layer extraction API ([#36](https://github.com/matyushkin/djvu-rs/issues/36)) ([d4c6527](https://github.com/matyushkin/djvu-rs/commit/d4c6527eb03aa9397a681ccdf03a27cf9ed77b0b))
* **metadata:** METa/METz document metadata parsing (Issue [#44](https://github.com/matyushkin/djvu-rs/issues/44)) ([#55](https://github.com/matyushkin/djvu-rs/issues/55)) ([eb4515b](https://github.com/matyushkin/djvu-rs/commit/eb4515b49819c3b75824738989c07153a7c2c0d6))
* **pdf:** DCTDecode background encoding — smaller PDF output (Issue [#49](https://github.com/matyushkin/djvu-rs/issues/49)) ([#59](https://github.com/matyushkin/djvu-rs/issues/59)) ([de90a9f](https://github.com/matyushkin/djvu-rs/commit/de90a9fd94ef98fdeb8822aef174e945ebe5a3ea))
* progressive DjVu rendering, multi-book cache, cos-diagnostics crate ([32432d8](https://github.com/matyushkin/djvu-rs/commit/32432d8c240f4a4673310a70158cf12cb9643635))
* **render:** BGjp/FGjp JPEG background/foreground decoder (Issue [#47](https://github.com/matyushkin/djvu-rs/issues/47)) ([#57](https://github.com/matyushkin/djvu-rs/issues/57)) ([b65bd81](https://github.com/matyushkin/djvu-rs/commit/b65bd817270c21248aa3ec46b140b6fb97a9b683))
* **render:** grayscale output mode — GrayPixmap + render_gray8 ([c13ebb7](https://github.com/matyushkin/djvu-rs/commit/c13ebb75ffa0673e670530bdd7ebe53f311a5044))
* **render:** grayscale output mode — GrayPixmap + render_gray8 (Issue [#15](https://github.com/matyushkin/djvu-rs/issues/15)) ([75d7b37](https://github.com/matyushkin/djvu-rs/commit/75d7b37aed6ce0a6d4d118abc8050b731089a8bf))
* **render:** Lanczos-3 separable resampling (Issue [#50](https://github.com/matyushkin/djvu-rs/issues/50)) ([#60](https://github.com/matyushkin/djvu-rs/issues/60)) ([56817d1](https://github.com/matyushkin/djvu-rs/commit/56817d162a335fe9597d2e65f84e0778ee147c65))
* **render:** permissive render mode — skip corrupted chunks ([dc5734a](https://github.com/matyushkin/djvu-rs/commit/dc5734a088f82b6c5bf7b6cfc06a599a2f342a2b))
* **render:** permissive render mode — skip corrupted chunks (Issue [#19](https://github.com/matyushkin/djvu-rs/issues/19)) ([df5a8d7](https://github.com/matyushkin/djvu-rs/commit/df5a8d715ba8534856d50f570752b8722118c2a0))
* **text:** TextLayer::transform — rotate + scale zone rects for rendered pages (Issue [#46](https://github.com/matyushkin/djvu-rs/issues/46)) ([#53](https://github.com/matyushkin/djvu-rs/issues/53)) ([c4a514e](https://github.com/matyushkin/djvu-rs/commit/c4a514e8bbdd4bccdb0032130b949580ef5306cc))
* **tiff:** TIFF export — multi-page color and bilevel modes (Issue [#48](https://github.com/matyushkin/djvu-rs/issues/48)) ([#58](https://github.com/matyushkin/djvu-rs/issues/58)) ([dc90cc0](https://github.com/matyushkin/djvu-rs/commit/dc90cc049ee7bd299ce6156c9c390c8616bdabca))
* transfer from cos-djvu, remove legacy GPL code, add PD corpus, benchmarks ([33fd496](https://github.com/matyushkin/djvu-rs/commit/33fd4969a691b5445c708b6c5f1ae2f877304f73))
* **ui:** table of contents navigation panel (closes [#60](https://github.com/matyushkin/djvu-rs/issues/60)) ([#298](https://github.com/matyushkin/djvu-rs/issues/298)) ([3fb0b2a](https://github.com/matyushkin/djvu-rs/commit/3fb0b2a94ff19891d6469f95aa381dcf66b7b712))
* user-controllable rotation in RenderOptions ([#35](https://github.com/matyushkin/djvu-rs/issues/35)) ([e0f79a8](https://github.com/matyushkin/djvu-rs/commit/e0f79a80bd3d3b2cc43933debb9a0b290aaea285))


### Bug Fixes

* add missing chunk_data binding in iw44_new doctest ([d1a210b](https://github.com/matyushkin/djvu-rs/commit/d1a210b8b61fabc5c7634282f0bfb8062503dfa3))
* apply gamma correction in all legacy render paths ([#9](https://github.com/matyushkin/djvu-rs/issues/9)) ([#22](https://github.com/matyushkin/djvu-rs/issues/22)) ([dfba614](https://github.com/matyushkin/djvu-rs/commit/dfba614c510d8da150ee76cf62f629f89cff48bc))
* apply page rotation from INFO chunk in render_pixmap and render_coarse ([#10](https://github.com/matyushkin/djvu-rs/issues/10)) ([#24](https://github.com/matyushkin/djvu-rs/issues/24)) ([adec5ee](https://github.com/matyushkin/djvu-rs/commit/adec5eed27bf5f096bad4187a2d88a5349bd07a4))
* **ci:** IJG license allowlist, no_std BTreeMap, clippy errors ([6a2a391](https://github.com/matyushkin/djvu-rs/commit/6a2a39199dffd1aa60cbe916bd2b28a516185dcd))
* clippy errors and fmt — let-chain, ref on let, line wrapping ([b4ba2f8](https://github.com/matyushkin/djvu-rs/commit/b4ba2f87acc234e7b7467f394169775b846c03b5))
* exclude .cargo/config.toml from published package (fixes docs.rs build) ([b9dd0da](https://github.com/matyushkin/djvu-rs/commit/b9dd0dafac62f0d6e12064f431d6b962b6a93e08))
* FGbz multi-color foreground palette — use per-glyph blit index ([#12](https://github.com/matyushkin/djvu-rs/issues/12)) ([#26](https://github.com/matyushkin/djvu-rs/issues/26)) ([7897164](https://github.com/matyushkin/djvu-rs/commit/789716408799e39c0e78e3caeedb051f913d26ac))
* **hard-rule:** eliminate last 5 .expect()/.unwrap() in production code (Issue [#443](https://github.com/matyushkin/djvu-rs/issues/443)) ([#444](https://github.com/matyushkin/djvu-rs/issues/444)) ([e4247ea](https://github.com/matyushkin/djvu-rs/commit/e4247eafc1eabadfac42fb04c318252ac0ebcc71))
* remove deprecated [[licenses.deny]] syntax from deny.toml (cargo-deny v2) ([7971e44](https://github.com/matyushkin/djvu-rs/commit/7971e448cdeb82d8be78910e9f275ecc1b975d65))
* replace all internal cos-djvu/cos_djvu references with djvu-rs/djvu_rs ([03fb17a](https://github.com/matyushkin/djvu-rs/commit/03fb17ae4460ca6baff50235993c6f005ddc08e4))
* update MSRV to 1.88 (let-chains stabilized in 1.88) ([8d5b94f](https://github.com/matyushkin/djvu-rs/commit/8d5b94f9d84c9065662e2464003aa75a95613421))
* vendor djvu-rs into crates/cos-djvu and fix production panics ([5f6d7fe](https://github.com/matyushkin/djvu-rs/commit/5f6d7fecadb7ea3bdcfa5cd530215ff28ea5e133)), closes [#4](https://github.com/matyushkin/djvu-rs/issues/4)


### Performance Improvements

* area-averaging downscale for better quality when rendering at reduced size ([#13](https://github.com/matyushkin/djvu-rs/issues/13)) ([#28](https://github.com/matyushkin/djvu-rs/issues/28)) ([b822ded](https://github.com/matyushkin/djvu-rs/commit/b822ded71372b0f70071f8245768d09b89e62a17))
* **bitmap:** packed bitwise dilation with ping-pong buffers ([2887689](https://github.com/matyushkin/djvu-rs/commit/288768968e910eccfe313785aa316ef8eb0fbdac))
* **bitmap:** packed bitwise dilation with ping-pong buffers (Issue [#17](https://github.com/matyushkin/djvu-rs/issues/17)) ([0814f55](https://github.com/matyushkin/djvu-rs/commit/0814f5519475073c4b426ca91360491c8ad69830))
* eliminate redundant mask sampling in 3-layer composite ([#14](https://github.com/matyushkin/djvu-rs/issues/14)) ([#27](https://github.com/matyushkin/djvu-rs/issues/27)) ([f601036](https://github.com/matyushkin/djvu-rs/commit/f601036e8988c2ea0c4f2a5742cd4b1321bc8065))
* **iw44:** SIMD YCbCr→RGB using wide::i32x8 (Issue [#1](https://github.com/matyushkin/djvu-rs/issues/1)) ([#64](https://github.com/matyushkin/djvu-rs/issues/64)) ([abceef4](https://github.com/matyushkin/djvu-rs/commit/abceef47d2bca524a20d13f4926f4dbf84e3c79b))
* **render:** eliminate redundant mask sampling in 3-layer composite (Issue [#14](https://github.com/matyushkin/djvu-rs/issues/14)) ([#37](https://github.com/matyushkin/djvu-rs/issues/37)) ([585991d](https://github.com/matyushkin/djvu-rs/commit/585991d961978277b8fdaf2b87236f6d1b825ac0))

## [Unreleased]

## [0.3.0] — 2026-04-05

### Added

- **TIFF export** — `djvu_to_tiff` converts DjVu to multi-page TIFF in color (RGB8) or bilevel
  (Gray8) modes; CLI: `djvu render --format tiff`; feature-gated: `--features tiff`
- **BGjp/FGjp JPEG decoder** — DjVu pages with JPEG-encoded background or foreground now render
  correctly; uses `zune-jpeg` (pure Rust, no libjpeg)
- **Async render API** — `djvu_async::render_pixmap_async` / `render_gray8_async` delegate
  CPU-bound IW44/JB2 work to `tokio::task::spawn_blocking`; feature-gated: `--features async`
- **Document metadata** — `metadata::parse_metadata` / `parse_metadata_bzz` extract METa/METz
  chunks; `DjVuMetadata` struct with title, author, date, and arbitrary key-value fields
- **Chunk introspection API** — `DjVuPage::raw_chunk`, `all_chunks`, `chunk_ids`;
  `DjVuDocument::raw_chunk`, `all_chunks` for direct access to IFF chunk data
- **DJVI shared dictionary** — `Sjbz` pages that reference a shared JB2 dictionary via `INCL`
  chunks now decode correctly; fixes rendering of multi-page documents with shared symbol sets
- **TextLayer coordinate transform** — `TextLayer::transform(scale, rotation)` maps zone rectangles
  to the rendered page coordinate system; simplifies hit-testing in viewer applications
- **DCTDecode PDF export** — `pdf::djvu_to_pdf_with_options` encodes page images as JPEG
  (DCTDecode) instead of raw RGB (FlateDecode); typically 5–10× smaller output; `PdfOptions`
  controls JPEG quality (default 80)
- **Lanczos-3 resampling** — `RenderOptions { resampling: Resampling::Lanczos3, .. }` applies a
  two-pass separable 6-tap Lanczos kernel after rendering; sharper thumbnails at the cost of ~5×
  render time vs `Bilinear`
- **Grayscale output** — `djvu_render::render_gray8` returns a `GrayPixmap` (1 byte/pixel);
  CLI: `djvu render --gray`
- **Permissive render mode** — `RenderOptions::permissive = true` skips corrupted or unsupported
  chunks instead of returning an error; useful for broken files in the wild
- **Benchmark suite** — `benches/render.rs` gains `render_scaled` (Bilinear vs Lanczos-3) and
  `pdf_export` benchmarks; `BENCHMARKS.md` documents results on Apple M1 Max and comparison vs
  DjVuLibre 3.5.29
- **Benchmark CI** — `.github/workflows/bench.yml` runs `cargo bench` on `ubuntu-latest` and
  `macos-latest` on every release tag; Criterion HTML reports uploaded as artifacts

### Performance

- **SIMD YCbCr→RGB** — `Iw44Image::to_rgb` now processes 8 pixels per iteration using
  `wide::i32x8` (maps to AVX2 on x86_64, NEON on ARM64, scalar fallback elsewhere); eliminates
  per-pixel overhead in the full-resolution color conversion hot path
- **Packed bitwise mask dilation** — `Bitmap::dilate` uses bitwise OR on packed `u64` words
  instead of per-pixel loops; 2–4× faster for bold-text rendering
- **Composite optimisation** — eliminated redundant mask sampling in the 3-layer composite loop

### Fixed

- **Permissive mode robustness** — decode pipeline no longer panics on documents with missing or
  truncated BG44/FG44 chunks when `permissive = true`

## [0.2.1] — 2026-04-04

### Fixed

- Exclude `.cargo/config.toml` from published package — it contained `-D warnings` which caused
  docs.rs builds to fail silently

## [0.2.0] — 2026-04-04

### Added

- **Structural PDF export** — `djvu render --format pdf` now produces searchable PDFs with selectable
  text (from TXTz/TXTa), bookmarks (NAVM → PDF outline), and hyperlinks (ANTz → PDF link annotations)
- **Mask / layer extraction API** — `DjVuPage::extract_mask()`, `extract_foreground()`,
  `extract_background()`; CLI: `djvu render --layer mask|fg|bg`
- **`RenderOptions::fit_to_width` / `fit_to_height` / `fit_to_box`** — aspect-preserving smart scaling
  helpers that respect page rotation
- **User-controllable rotation** — `RenderOptions::rotation` overrides the INFO chunk value

### Fixed

- **Gamma correction** — gamma LUT now applied in all render paths (`render_pixmap`, `render_coarse`,
  `render_progressive`, rotation branches)
- **Page rotation** — `render_pixmap` and `render_coarse` now apply the rotation from the INFO chunk;
  output dimensions swap correctly for 90°/270° pages
- **FGbz multi-color foreground** — per-glyph blit index is now used when compositing; documents with
  multi-color foreground (stamps, colored annotations) render correctly

### Performance

- **Area-averaging downscale** — render at scale < 1.0 now uses box-filter averaging instead of
  bilinear; better anti-aliasing and ~2× faster for thumbnail/overview sizes
- **Composite optimisation** — eliminated redundant mask sampling in 3-layer composite loop

### Refactored

- **Removed `ouroboros` dependency** — `Document` is now a fully owned struct; no self-referential
  proc-macro required; `lib.rs` is now truly `unsafe`-free

## [0.1.0] — 2026-04-04

### Added

- **IFF container parser** — zero-copy, borrowing slices from input (`iff::parse_form`)
- **JB2 bilevel image decoder** — ZP adaptive arithmetic coding with symbol dictionary (`jb2_new`)
- **IW44 wavelet image decoder** — planar YCbCr storage, progressive multi-chunk refinement (`iw44_new`)
- **BZZ decompressor** — ZP + MTF + BWT for DIRM, NAVM, ANTz chunks (`bzz_new`)
- **Text layer extraction** — TXTz/TXTa chunk parsing with full zone hierarchy (`text`)
- **Annotation parsing** — ANTz/ANTa chunks: hyperlinks, map areas, background color (`annotation`)
- **Bookmarks** — NAVM table-of-contents parsing (`DjVuDocument::bookmarks`)
- **Multi-page documents** — DJVM bundle format with DIRM directory chunk
- **Page rendering** — composite foreground mask + background wavelet into RGBA output
- **Progressive rendering** — incremental BG44 wavelet refinement (`Page::render_scaled_progressive`)
- **Thumbnails** — TH44 embedded thumbnail extraction (`Page::thumbnail`)
- **High-level API** — `Document` / `Page` (requires `std` feature)
- **New document model** — `DjVuDocument` / `DjVuPage` built on clean-room codecs
- **CLI tool** — `djvu info`, `djvu render --format png|pdf|cbz`, `djvu text` subcommands
- **Rasterized PDF export** — `djvu render --format pdf` embeds each page as an RGB image (FlateDecode)
- **CBZ export** — `djvu render --format cbz` produces a comic-book ZIP with PNG pages
- **`no_std` support** — IFF/BZZ/JB2/IW44/ZP modules work with `alloc` only

[Unreleased]: https://github.com/matyushkin/djvu-rs/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/matyushkin/djvu-rs/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/matyushkin/djvu-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/matyushkin/djvu-rs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/matyushkin/djvu-rs/releases/tag/v0.1.0
