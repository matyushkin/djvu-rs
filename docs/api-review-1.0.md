# Public API review before 1.0.0

Item 4 of [#993](https://github.com/matyushkin/djvu-rs/issues/993): review the
public surface of the root crate and the seven codec crates once, item by
item, and hide, rename or gate anything that should not be frozen.

**Input.** `cargo public-api` on `1.0.0-rc.1` (commit `4c9c70a`), about 6 200
lines: `djvu-rs` with `cli,tiff,async,serde,image,epub,mmap,parallel,jpeg,wasm-lazy`,
each codec crate with its default features. `ocr-tesseract` (needs a system
leptonica) and the experimental features were read from source. Each finding
was checked against the source. The downstream usage recorded in
[#992](https://github.com/matyushkin/djvu-rs/issues/992) was the list of items
that must keep working.

**Result.** No change touches an item a known dependent uses, except where a
row says so. Rows are grouped by the kind of change, in the order the work
should land. *Status* is filled in as pull requests merge.

## A. The gate and the tiers (first: everything below relies on it)

| # | Finding | Change | Status |
|---|---------|--------|--------|
| A1 | `cargo-semver-checks` checks only `djvu-rs` with default features. The seven codec crates (`djvu-zp` is not even re-exported), the writers (`pdf`, `epub`, `cbz`, `tiff`) and `async` have no gate after the tag. | Add the codec packages and a second run with the stable writer features. While the version is `1.0.0-rc.1`, cargo-semver-checks reads every change as major and skips its checks; the gate bites from `1.0.0`. Checked locally as a minor release: the three runs report exactly the intended breaks of this PR. | #1022 |
| A2 | `ffi` (C ABI, 15 `extern "C"` fns) is gated only on `std`, so every default build carries it as Rust surface. No tier, no shipped header. | Gate behind a new non-default `ffi` feature; declare its tier in §1 (or experimental). | #1022 |
| A3 | `wasm` Rust items are semver surface and carry pre-1.0 `wasm_bindgen`/`js_sys` types; the real contract is the npm `.d.ts`. | Make the module private / `#[doc(hidden)]`; §1: "binding tier, follows the npm package". Diff the `.d.ts` before and after. | #1022 |
| A4 | Encoder modules (`djvu_encode`, `djvm`, `segment`, `ingest`, `chunk_encode`, `smmr`, `*_encode`) have no stated tier, and nothing says whether output bytes, heuristics and default tuning are behaviour under SemVer. | §1: name the tier of each module; output bytes, heuristics and default knob values may change in a minor release while the output stays valid DjVu. | #1022 |
| A5 | No serde/JSON rule: `EditRequest` JSON is documented as persistable, `Optimization*::to_json` as "stable JSON". | Decide (see G1) and write it into §1. | #1022 |
| A6 | `#[doc(hidden)] pub` has no stated meaning; the codec crates need it for cross-crate internals that `pub(crate)` cannot cover. | §1: `#[doc(hidden)]` items are workspace-internal and not covered (cargo-semver-checks already skips them). | #1022 |
| A7 | No experimental item opens its rustdoc with `**Experimental:**` as §1 requires (`ocr_onnx`, jb2 `experimental`, `iw44-probe`). | Add the note. | #1022 |
| A8 | Cargo feature `jpeg` gates nothing (`zune-jpeg` comes with `std`); the README promises a no-`std` JPEG decode. | Remove the feature or document it as a no-op. | #1022 |

## B. Errors

| # | Finding | Change | Status |
|---|---------|--------|--------|
| B1 | `djvu_rs::Error` is `djvu_iff::LegacyError` (three names, "Legacy" in a 1.0 type). `Document`/`Page` turn every typed error, `ResourceLimitExceeded` included, into `FormatError(String)`, against §7. `Clone + Eq` stop it from ever wrapping a source. `MutError::Parse` carries it too. | A djvu-rs-owned `#[non_exhaustive] enum Error { Doc, Render, Io, … }` with `source()`; keep the name `djvu_rs::Error`. Remove the `error::{LegacyError, Error}` re-exports; rename or drop `LegacyError` in `djvu-iff`; `MutError::Parse` follows. | this PR: `LegacyError` removed, `iff::parse` returns `IffError`; `Error::resource_limit()` |
| B2 | `DjVuError` is called "top-level" but only `fgbz_encode::decode_fgbz` returns it. | Hide `decode_fgbz` (E8) and drop `DjVuError` from the root, or document what it is. | this PR: `DjVuError` removed |
| B3 | String payloads that lose the source: `DocError::{Smmr, IndirectResolve}`, `ComponentResolveError::Failed`, `RenderError::Jpeg`, `MutError::RewriteIo`, `OptimizeError::Verification`, `GraphError::Malformed`, `DjvmError::ComponentGraph` (a `{:?}`), `TiffError::Encode` (sink I/O too), `CbzError::Png`, `AsyncPdfError/AsyncDjvmError/AsyncRenderError/AsyncTileError::Join`. | Typed payload or `#[source]`; `TiffError::Io(io::Error)`; an opaque join error with `is_panic()`/`is_cancelled()`. | this PR: `JpegError`, `TiffEncodeError` + `TiffError::Io`, `PngEncodeError`, `JoinFailure` |
| B4 | `EncodeError::Unsupported(&'static str)` carries five unrelated failures. | Typed variants: `InvalidProfile`, `DimensionsTooLarge`, `MaskMismatch`, `Djvm(DjvmError)`. | this PR |
| B5 | Derives that stop errors from growing: `GraphError: Eq`, `ComponentResolveError: Clone`, `PixmapError: Copy + Hash`. | Drop them now. | this PR: findings split into `GraphIssue` |
| B6 | Misleading blanket `From`s: `MutError::InfoParse` gets every `IffError`, `SaveIo` every `io::Error`; `OptimizeError::Parse` gets rewrite failures. | Rename to `Iff`/`Io`, or map explicitly. | this PR: `MutError::{Iff, Io}`, `OptimizeError::Rewrite` |
| B7 | Dead variants: `DocError::InvalidUtf8`, `BzzError::{ZpError, InvalidBwtIndex}`, `IffError::UnknownFormType`, `LegacyError::InvalidMagic`, `TextSerializeError::Text`. | Remove. | this PR, also `TextSerializeError::Fmt` |
| B8 | `OcrError::{ModelVerificationFailed, ManifestInvalid}` exist only for experimental `ocr-onnx`. | Fold into a backend-neutral variant or `cfg(feature = "ocr-onnx")`. | this PR: `ManifestInvalid` removed; `ModelVerificationFailed` kept (backend-neutral) |
| B9 | Two async error enums that differ in one variant. | Merge `AsyncRenderError` and `AsyncTileError`. | this PR |
| B10 | `chunk_encode::EncodeError` and `djvu_encode::EncodeError` share a name. | Rename the first to `ChunkEncodeError`. | this PR |
| B11 | Found while fixing B1: `MutError::ComponentResolve` and `MutError::ComponentMalformed` still drop their source (`map_err(\|_\| …)`). | Carry the source. | |

## C. Third-party types in stable signatures

| # | Finding | Change | Status |
|---|---------|--------|--------|
| C1 | `zip` 9: `cbz::write_pages*` take `&mut ZipWriter`; `CbzError::Zip`, `EpubError::Zip` and their `From<ZipError>`. | `pub(crate)` the writers, add `djvu_to_cbz_writer_with_observer`; opaque error or `Io`. | |
| C2 | `tiff` 0.11: `impl From<tiff::TiffError> for TiffError`. | Remove; convert in a private helper. | |
| C3 | `image` 0.25: `image_compat` implements `image::ImageDecoder`. | Version the feature (`image-0_25`) or make it experimental; write the third-party rule into §1. | |
| C4 | Encoders that cannot fail: `encode_iw44_*`, `encode_jb2*`, `encode_djvm_bundle_jb2*` (`expect`), `smmr::encode_smmr`, `text_encode::encode_text_layer` truncate sizes with `as`. | Return `Result`, or make them `pub(crate)` behind a checked wrapper. | |

## D. Internals to hide

| # | Item | Change | Status |
|---|------|--------|--------|
| D1 | `djvu_zp::ZpDecoder` public registers; `djvu_zp::tables`; `ZpEncoder::{encode_bit_inline, encode_run, bytes_written}`. | Private fields + `#[doc(hidden)]` state save/restore for jb2/iw44 loops; hide the rest. | |
| D2 | `DjVuPage::{find_chunk, find_chunks}` (hidden deprecated aliases), `decoded_bg44/_partial/_mask/_fg44`, `extract_mask_indexed`; `djvu_jb2::decode_indexed`. | `pub(crate)` / `#[doc(hidden)]`. | |
| D3 | `validate::{check_document_limits, validate_planned_output}`, `text::MAX_ZONE_DEPTH`, `optimizer::{ProgressHook, CancelHook}`. | `pub(crate)`. | |
| D4 | `djvu_mut::DjVuDocumentMut::{replace_leaf, remove_leaf, chunk_at_path, root_child_count}`; `IndirectRewritePlan`, `RewriteItem`, `from_indirect_resolved`. | Behind `experimental`. | |
| D5 | Codec plumbing: `djvu_iff::{emit, emitted_size, framed_size, partial_emit*, EmitPart}`; `djvu_jb2::encode::{encode_jb2_dict_with_blits*, encode_jb2_dict_with_symbols*, symbol_boxes_in_emission_order, SymbolBox, EncodedBlit}`. | `#[doc(hidden)]` (A6). | |
| D6 | Research and diagnostics: `analyze_jb2_cc_stats`/`CcStats`, `cluster_shared_symbols_tunable` (dead parameter), `quality` (panics on a size mismatch), `thumbnail`, `render_pages_parallel`, `ProgressiveDecoder`, cache tuning (`downgrade_*`, `enforce_cache_budget_with`, `CacheBudgetOptions`). | Behind `experimental` or `pub(crate)`. | |
| D7 | `png_io`: CLI-shaped, `Box<dyn Error>` without `Send + Sync`, CLI flags in messages; `ingest` pixel helpers. | Behind `cli` or `#[doc(hidden)]`; `pub(crate)` the helpers. | |

## E. Duplicates and one-off names

| # | Finding | Change | Status |
|---|---------|--------|--------|
| E1 | `djvu_tile::TileRect` = `RenderRect` since 1.0; `TileCancelToken` = `CancelToken`. | Type alias or remove; fix the docs. | |
| E2 | `render_tile`, `render_tile_cached`, `render_tile_with`, `TileRenderControls`, `prefetch_tiles[_cancellable]`, async tile/progressive functions take `RenderOptions` and repeat `RenderRequest`. | One request-based tile entry point (or experimental). | |
| E3 | `Page::render_with_limits` repeats `Page::render_request`. | Remove. | |
| E4 | `bzz::decode` aliases `bzz_decode`. | Remove. | |
| E5 | Encoders reachable twice: `djvu_rs::{jb2_encode, iw44_encode}` and `djvu_rs::{jb2, iw44}::encode`. | Keep one path. | |
| E6 | Nine multi-page encode functions named by suffix, none taking the per-codec options. | One `#[non_exhaustive]` multi-page builder; hide the nine. | |
| E7 | Two resolver APIs on `DjVuDocument` (string-keyed `parse_with_resolver*` and typed `parse_with_component_resolver`, which ignores limits). | Keep the typed one, add its `_and_options` form. | |
| E8 | `fgbz_encode::decode_fgbz` is a decoder in an encoder module. | `pub(crate)`. | |
| E9 | `text_encode` is the only parse/encode pair split across modules. | Move into `text`. | |
| E10 | `validate::*` re-exports `resource_limits` (third path). | Remove the re-export. | |
| E11 | `semantic_diff::PLANES: [&str; 6]`, `PlaneDiff::plane: &str`, `planes: Option<&[String]>`. | `#[non_exhaustive] enum Plane` and an options struct. | |
| E12 | `LazyDocument` has three constructors; `load_document_async_streaming` does not stream. | One constructor; rename. | |

## F. Types that will need to grow

| # | Item | Change | Status |
|---|------|--------|--------|
| F1 | `annotation::{Annotation, MapArea}`, `text::Paragraph`, `ocr_tesseract::TesseractBackend` (public `String` field, constructor `with_tessdata`), `djvu_jb2::encode::AlignedRefine`, `djvu_iff::{Form, IffChunk}`. | `#[non_exhaustive]` + `with_*` methods. | |
| F2 | `semantic_diff::PlaneStatus`, `ComponentKind` are listed as closed but are not (`ComponentKind` has no shared-annotation kind). | `#[non_exhaustive]`; update the closed list in §1. | |
| F3 | `Copy` on growing options: `ParseOptions`, `CacheBudgetOptions`, `Iw44EncodeOptions`, `Jb2EncodeOptions`, `IngestPolicy`. | Drop `Copy`. | |
| F4 | `Iw44EncodeOptions::chroma_half` is a documented no-op; `ValidateOptions::strict` is a no-op. | Remove. | |
| F5 | `Iw44Image::{width, height}` are writable public fields that `to_rgb` relies on. | Private + accessors. | |
| F6 | `PageMut::{set_annotations, remove_text_layer, remove_annotations}` return `()`, the other setters `Result`. | Return `Result`. | |
| F7 | `OptimizationRequest::max_ssim_loss: f32`, reported values `f64`. | `f64`. | |
| F8 | `PageInfo` drops the INFO version. | Add `version` now or never. | |

## G. Decisions for the maintainer

| # | Question | Decision |
|---|----------|----------|
| G1 | Is the serde/JSON form of `EditRequest`, the optimizer reports and the types they embed covered by SemVer? | **Decided (2026-11-25):** not covered by SemVer. Versioned by `EDIT_SCHEMA_VERSION` only; new fields carry `#[serde(default)]`; a request with an older `version` stays accepted. |
| G2 | PDF entry points are named unlike EPUB/CBZ/TIFF (`djvu_to_pdf_to_writer`, no-options base function). | **Decided:** rename to `djvu_to_pdf(doc, &opts)`, `djvu_to_pdf_writer[_with_observer]`, `djvu_to_pdf_async_writer`. |
| G3 | DPI options differ across writers (`output_dpi`, `dpi`, TIFF `scale: f32`; `adaptive_raster` vs `adaptive`). | **Decided:** one name and one unit in all four writers. |
| G4 | `pub trait OcrBackend` has no `Send + Sync` supertraits; adding them later breaks implementors. | **Decided:** `OcrBackend: Send + Sync`. |
| G5 | `AsyncComponentResolver::resolve` returns a future with no `Send` bound. | **Decided:** `+ Send`. |
| G6 | `annotation::Rect` (bottom-left origin) and `text::Rect` (top-left) share a name; coordinates are `u32`, DjVuLibre uses signed. | Rename `annotation::Rect`; `i32`. |
| G7 | Annotation fields as strings (`Border::style`, `mode`), page annotations as a `(Annotation, Vec<MapArea>)` tuple. | Keep strings, document them as the contract; leave the tuple. |
| G8 | `OptimizationPlan` and `OptimizationReport` have identical fields. | Merge. |
| G9 | Legacy `Document`/`Page` next to `DjVuDocument`/`DjVuPage`. | Keep both: dependents use `Document::open`. Document `Document` as the facade. Fixing B1 is enough. |

## Accepted as they are

- `Bitmap`, `Pixmap`, `GrayPixmap` keep public `width`/`height`/`data` (plain data, §1); `dilate`/`dilate_n` receivers to be aligned.
- `DjVuMetadata` stays plain data; new keys stay in `extra` for all of 1.x.
- `tokio` 1.x and `futures_core::Stream` in async signatures.
- `Copy` on geometry and report values (`TileLayout`, `RenderRect`, report structs); `Copy` on `ResourceLimits`.
- `RenderOptions::new(w, h)` is pre-rotation; say so in its docs.
- `TextZoneKind`, `Rotation`: closed by the format.
- `export_control::ExportObserver`, `djvm::{merge, split, remove_pages, …}`, `PageEncoder`, `chunk_encode` as the low-level seam.
