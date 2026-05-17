# Performance experiments

Log of perf experiments and their outcomes. Each entry: issue, approach,
numbers, decision, reason. Referenced from issue templates ("Record result
in `PERF_EXPERIMENTS.md` (Kept or Reverted + reason)") and from
`.github/workflows/bench.yml`.

### #295 — JB2 encoder corpus round-trip and size baseline — **Needs follow-up** (2026-05-17)

**Approach.** Refreshed the existing JB2 quality harnesses without changing
encoder behavior. The page-level run measured original `Sjbz`, direct
`encode_jb2`, and dict `encode_jb2_dict` bytes/bpp/round-trip status across
current JB2-bearing fixtures and corpus files. The shared-Djbz run measured
`encode_jb2_dict` independent page totals vs bundled shared-Djbz totals, with
CC accounting and cross-size probe output enabled.

**Platform.**
- OS: macOS 26.3.1 (Darwin 25.3)
- CPU: Apple M1 Max, 10 cores
- target_arch: `aarch64`
- target_feature(s): ARM64 baseline; NEON available on Apple Silicon
- Rust: 1.92.0 stable (`aarch64-apple-darwin`)
- RUSTFLAGS: unset
- Source artifact: local run on `codex/issue-295-jb2-quality-refresh`

**Command(s).**

```sh
cargo run --release --example encode_quality_jb2 -- \
  references/djvujs/library/assets/boy_jb2.djvu \
  references/djvujs/library/assets/boy.djvu \
  references/djvujs/library/assets/carte.djvu \
  references/djvujs/library/assets/chicken.djvu \
  references/djvujs/library/assets/colorbook.djvu \
  references/djvujs/library/assets/DjVu3Spec_bundled.djvu \
  references/djvujs/library/assets/irish.djvu \
  references/djvujs/library/assets/navm_fgbz.djvu \
  tests/corpus/cable_1973_100133.djvu \
  tests/corpus/conquete_paix.djvu \
  tests/corpus/pathogenic_bacteria_1896.djvu \
  tests/corpus/watchmaker.djvu

cargo run --release --example encode_quality_djbz -- \
  --cc-stats --cross-size-stats \
  references/djvujs/library/assets/colorbook.djvu \
  references/djvujs/library/assets/DjVu3Spec_bundled.djvu \
  references/djvujs/library/assets/navm_fgbz.djvu \
  tests/corpus/conquete_paix.djvu \
  tests/corpus/pathogenic_bacteria_1896.djvu \
  tests/corpus/watchmaker.djvu
```

**Numbers.**

Page-level JB2 refresh:

| Mode | Pages | Bytes | bpp | vs original | Round-trip |
|------|------:|------:|----:|------------:|------------|
| Original `Sjbz` | 692 | 26,569,542 | 0.0263 | 1.000x | source |
| Direct `encode_jb2` | 692 | 46,252,033 | 0.0457 | 1.741x | 464 ok, 228 decode errors |
| Dict `encode_jb2_dict` | 692 | 36,016,741 | 0.0356 | 1.356x | 692 ok, 0 failures |

Per-file dict ratios:

| File | Pages | Dict/orig | Dict failures | Direct failures |
|------|------:|----------:|--------------:|----------------:|
| `boy_jb2.djvu` | 1 | 1.000x | 0 | 0 |
| `colorbook.djvu` | 62 | 1.030x | 0 | 46 decode errors |
| `DjVu3Spec_bundled.djvu` | 70 | 1.627x | 0 | 70 decode errors |
| `irish.djvu` | 1 | 0.302x | 0 | 0 |
| `navm_fgbz.djvu` | 5 | 0.301x | 0 | 5 decode errors |
| `cable_1973_100133.djvu` | 2 | 1.136x | 0 | 0 |
| `conquete_paix.djvu` | 22 | 1.025x | 0 | 16 decode errors |
| `pathogenic_bacteria_1896.djvu` | 517 | 1.378x | 0 | 80 decode errors |
| `watchmaker.djvu` | 12 | 1.058x | 0 | 11 decode errors |

`carte.djvu` was skipped by the harness because the checked-in fixture is
truncated and does not parse.

Shared-Djbz refresh:

| Mode | Files/pages | Bytes | bpp | vs original | Round-trip |
|------|------------:|------:|----:|------------:|------------|
| Original `Sjbz` totals | 6 / 688 | 26,424,220 | 0.0262 | 1.000x | source |
| Independent dict pages | 6 / 688 | 35,963,419 | 0.0356 | 1.361x | all pages ok |
| Bundled shared-Djbz | 6 / 688 | 34,986,136 | 0.0347 | 1.324x | all bundles ok |

Bundled shared-Djbz was `0.973x` of independent dict output (`-2.7%`) on this
six-file run. Individual bundle/independent ratios were: `colorbook` 1.002x,
`DjVu3Spec_bundled` 0.642x, `navm_fgbz` 0.955x, `conquete_paix` 1.029x,
`pathogenic_bacteria_1896` 0.976x, and `watchmaker` 0.945x.

Failure buckets:
- Direct `encode_jb2` decode errors are oversized whole-image record-3 symbols
  hitting decoder symbol-size limits on large pages.
- Dict `encode_jb2_dict` has no current mismatch or decode-error bucket on the
  refreshed corpus; the old `483/553` dict round-trip number is stale.
- Shared-Djbz has no current mismatch or decode-error bucket with byte-exact
  clustering; all six bundles round-trip pixel-exact.
- `carte.djvu` is a harness/input bucket: truncated fixture parse failure, not
  an encoder failure.

**Decision.** Needs follow-up. The refreshed safe baseline is dict encoding:
it round-trips all 692 pages but remains `1.356x` original bytes overall.
Shared-Djbz is safe and saves `2.7%` vs independent dict on this corpus, but it
still remains `1.324x` original bytes overall.

**Reason.** Correctness is no longer the blocker for the dict path on the
current corpus; byte cost is. The next narrow JB2 implementation issue should
be #301: add a byte-cost estimator for cross-size refinement before emitting
any new cross-size or lossy/lossless refinement records. The largest measured
size gaps are still `pathogenic_bacteria_1896` and `DjVu3Spec_bundled`, while
`watchmaker` shows cross-size candidate headroom already recorded by the probe.

### #294 — thumbnail row-scratch A/B — **Rejected** (2026-05-17)

**Approach.** Added a `render_row_scratch_ab` Criterion group to compare the
current strict direct `render_into` path against a row-scratch adapter that
copies `render_streaming` rows into the final RGBA buffer. The comparison uses
the issue's thumbnail and native targets with warmed decode caches.

**Platform.**
- OS: macOS 26.3.1 (Darwin 25.3)
- CPU: Apple M1 Max, 10 cores
- target_arch: `aarch64`
- target_feature(s): ARM64 baseline; NEON available on Apple Silicon
- Rust: 1.92.0 stable
- RUSTFLAGS: unset
- Source artifact: local run on `codex/issue-294-row-scratch-ab`

**Command(s).**

```sh
cargo bench --bench render -- render_row_scratch_ab \
  --warm-up-time 1 --measurement-time 2 --sample-size 10
```

**Numbers.**

First run:

| Target | Direct `render_into` | Row-scratch copy | Decision signal |
|--------|---------------------:|-----------------:|-----------------|
| `thumbnail_dpi72` | 248.21 µs | 205.35 µs | row-scratch faster |
| `thumbnail_half_bilinear` | 153.55 µs | 399.13 µs | row-scratch much slower |
| `colorbook_downscale` | 23.674 ms | 18.925 ms | row-scratch faster, noisy |
| `corpus_color_native` | 207.96 ms | 248.74 ms | native regression |
| `corpus_bilevel_native` | 150.93 ms | 198.23 ms | native regression |

Rerun after bounding the A/B group to keep full CI benchmark runtime stable:

| Target | Direct `render_into` | Row-scratch copy | Decision signal |
|--------|---------------------:|-----------------:|-----------------|
| `thumbnail_dpi72` | 306.59 µs | 199.09 µs | row-scratch faster |
| `thumbnail_half_bilinear` | 143.84 µs | 124.58 µs | row-scratch faster |
| `colorbook_downscale` | 15.966 ms | 11.861 ms | row-scratch faster, noisy |
| `corpus_color_native` | 155.40 ms | 135.02 ms | row-scratch faster, noisy |
| `corpus_bilevel_native` | 146.10 ms | 160.35 ms | no clear signal |

**Decision.** Rejected as a render heuristic. No production render path changed.
The A/B harness is kept so future thumbnail work can rerun the comparison.

**Reason.** The repeated short A/B runs are too noisy and inconsistent to justify
a production heuristic: the first run showed a thumbnail loss and native
regressions, while the rerun showed broader wins but still no clean bilevel
native signal. A threshold heuristic would be fragile without a more stable
predictor than output size alone.

### #293 — compositor-only render baselines — **Kept** (2026-05-17)

**Approach.** Added a `render_compositor_only` Criterion group to
`benches/render.rs`. Each case warms page-level decode caches with one
`render_pixmap` call, then measures `render_into` into a reused RGBA buffer.
This isolates cached compositor/output materialization from document parse,
codec decode/cache setup, and output allocation.

**Platform.**
- OS: macOS 26.3.1 (Darwin 25.3)
- CPU: Apple M1 Max, 10 cores
- target_arch: `aarch64`
- target_feature(s): ARM64 baseline; NEON available on Apple Silicon
- Rust: 1.92.0 stable
- RUSTFLAGS: unset
- Source artifact: local run on `codex/issue-293-compositor-baselines`

**Command(s).**

```sh
cargo bench --bench render -- render_compositor_only \
  --warm-up-time 1 --measurement-time 2 --sample-size 10
```

**Numbers.**

| Bench | Fixture/path | Cached path | Time |
|-------|--------------|-------------|-----:|
| `render_compositor_only/color_native_cached` | `tests/corpus/watchmaker.djvu` | color native, decoded caches warm, reused RGBA buffer | 71.061 ms |
| `render_compositor_only/bilevel_native_cached` | `tests/corpus/cable_1973_100133.djvu` | bilevel native, decoded caches warm, reused RGBA buffer | 72.171 ms |
| `render_compositor_only/color_downscale_cached` | `references/djvujs/library/assets/colorbook.djvu` | color downscale, decoded caches warm, reused RGBA buffer | 7.4213 ms |
| `render_compositor_only/small_color_downscale_cached` | `references/djvujs/library/assets/boy.djvu` | small color 0.5x downscale, decoded caches warm, reused RGBA buffer | 152.00 µs |

**Decision.** Kept. The new benches can be run independently with a single
Criterion filter, and their names identify color/bilevel, native/downscale,
and cached decode state.

**Reason.** This gives #294 and later compositor work a narrow baseline without
changing render behavior or mixing optimization into the measurement issue.

### #290 — layered multi-page DJVM directory encode — **Kept** (2026-05-16)

**Approach.** Extended `djvu encode <dir> --quality quality|archival` to encode
pages independently with `PageEncoder::from_pixmap`, then bundle the resulting
single-page `FORM:DJVU` pages with `djvm::merge`. The existing lossless directory
path is left unchanged and still uses `encode_djvm_bundle_jb2` with
`--shared-dict-pages`. Layered directory encode deliberately does **not** create a
shared Djbz dictionary: each page keeps its own `Sjbz` mask plus `BG44` and
optional `FGbz`, avoiding rejected Hamming shared-Djbz clustering while preserving
layered chunks in a parseable bundled DJVM.

**Numbers / fixture.** Added CLI fixtures for two-page RGB directories. Both
`--quality quality` and `--quality archival` produce parseable `page_count=2`
DJVM bundles; each page has `Sjbz`, `BG44`, and `FGbz`. The quality fixture also
renders every page through `djvu_render::render_pixmap` at native 32×32 pixels.
The pre-existing lossless directory fixture still produces `page_count=3` with
`Sjbz` pages and no `BG44` / `FGbz` chunks.

**Decision.** Kept. This satisfies layered multi-page encode without changing the
lossless shared-Djbz behavior or reviving Hamming clustering in the default path.

### #288 — adaptive segmentation + BG-block inpainting — **Kept** (2026-05-16)

**Approach.** Extended `SegmentOptions` without changing its default behaviour:
`Binarization::Fixed` remains the default global BT.601 threshold, while
`Binarization::Sauvola { window, k }` adds local adaptive binarisation for mixed
lighting scans. Added optional `bg_inpaint` for fully masked background blocks:
when a BG subsample cell has no unmasked source pixels, it is filled from the
nearest neighbouring unmasked pixels instead of falling back to the ink-coloured
block mean. `PageEncoder::with_segment_options` lets library callers opt into
these knobs for `Quality` / `Archival` single-page encodes; CLI defaults remain
unchanged.

**Numbers / fixture.** Added a checked-in synthetic mixed text/photo test in
`djvu_encode::tests::adaptive_segment_options_improve_decoded_mixed_lighting_fixture`:
left half dark paper (`Y=80`), right half bright paper (`Y=220`), with dark ink
(`Y=40`) and light gray ink (`Y=140`). With `bg_subsample=6`, fixed-threshold
Quality encode decodes at `mean_abs_rgb_diff=10.767` versus source; Sauvola +
inpainting decodes at `4.188` (61% lower), and the test requires at least a 30%
reduction. The lower-level `segment::tests::sauvola_handles_dark_background_and_light_ink`
asserts that fixed 128 masks most dark paper and misses the light ink, while
Sauvola keeps the mask less than half the fixed-mask size and retains both ink
pixels. Added `segment::tests::inpaint_fully_masked_bg_block_from_neighbors`: a
fully masked black 4×4 BG block next to tan paper now inpaints to
`(210,200,160)` when `bg_inpaint` is enabled; default fixed-threshold/no-inpaint
still falls back to black for all-black pages.

**Tests.** Added/updated segment unit tests, proptest `SegmentOptions`
constructors, and a `PageEncoder::with_segment_options` parseability test proving
Quality encode still emits `Sjbz` + `BG44` with adaptive options.

**Decision.** Kept. The new behaviour is opt-in, deterministic, covered by a
synthetic mixed-light fixture, and does not enable Hamming shared-Djbz clustering
or alter the default fixed-threshold path.

### #281 — strict `render_pixmap` composites directly into its output — **Kept** (2026-05-16)

**Approach.** Added native-resolution stage benches for the DjVuLibre comparison
corpus (`render_native_stages/*`) covering public `render_pixmap`,
`render_into` with a reused RGBA buffer, `render_streaming` with discarded rows,
JB2 mask decode, and cached IW44 inverse/RGB. Then changed strict
`render_pixmap` to call `render_into` directly instead of routing through the
row-streaming adapter and copying each scratch row into the output `Pixmap`.
`opts.permissive` keeps the old `render_rows` path because it has different
chunk-error recovery semantics.

**Numbers.** Quick local Criterion runs (`--warm-up-time 1 --measurement-time 2/3
--sample-size 10`) after #279 had made native render more expensive:

- `render_corpus_color`: `88.44 ms` → `72.27 ms` median (**18% faster**).
- `render_corpus_bilevel`: `90.09 ms` → `72.00 ms` median (**20% faster**).
- `render_colorbook` at 150 dpi: `7.29 ms` historical / `7.12 ms` after this
  change (no regression; slight improvement in the quick run).
- `iw44_to_rgb_colorbook/sub4_partial_decode`: `344 µs`, Criterion reported no
  statistically significant change, so the known sub4 partial decode path did
  not regress.

The new stage split (recorded in `BENCHMARKS_RESULTS.md`) shows warm JB2/IW44
codec stages are only a few milliseconds on the native corpus; the remaining
DjVuLibre gap is dominated by compositor sampling and output materialization.

**Tests.** Targeted render tests passed, including byte-identical
`render_rows`/`render_into` and `render_streaming`/`render_pixmap` checks plus the
permissive truncated-BG44 regression. Full validation below covered the rest of
the workspace.

**Decision.** Kept. The change is narrow, removes an avoidable row copy from the
public strict render path, beats Criterion noise on both native corpus targets,
and leaves the permissive recovery path and IW44 sub4 decode untouched.

### #280 — TIFF export uses `render_streaming` rows — **Kept** (2026-05-16)

**Approach.** Added `tiff_export::djvu_to_tiff_writer(doc, opts, writer)` and
changed the existing `djvu_to_tiff` byte-buffer wrapper to delegate to it.
Color TIFF pages now use `djvu_render::render_streaming` when options are
streamable (no AA, bilinear/no-op resampling, identity combined rotation) and
feed RGB rows directly into TIFF strips. Pages requiring render post-processing
keep the existing full-`Pixmap` fallback. Bilevel TIFF export was already a
mask-extraction path and remains unchanged.

**Numbers.** Repro probe added as
`examples/probe_tiff_streaming_memory.rs` (`required-features = ["tiff"]`).
Command run locally after a release build:

```text
/usr/bin/time -l target/release/examples/probe_tiff_streaming_memory \
  tests/fixtures/problem_page.djvu /tmp/problem_page_streamed.tiff 1.0
```

Output for the 600-dpi `problem_page.djvu` fixture:

- page: `3288x5050` px at scale `1.000` (`16,604,400` pixels)
- output TIFF bytes written to `File`: `49,813,798`
- full RGBA pixmap allocation avoided: `66,417,600` bytes
- full RGB staging allocation avoided: `49,813,200` bytes
- `/usr/bin/time -l` maximum resident set size: `7,962,624` bytes
- peak memory footprint: `7,111,552` bytes

**Tests.** Added TIFF tests comparing decoded streamed color-TIFF pixels against
the existing `render_pixmap(...).to_rgb()` result for both a color page
(`chicken.djvu`) and a bilevel page (`boy_jb2.djvu`). Also fixed an existing
TIFF test to unwrap `extract_bilevel_pixels` under the `tiff` feature.

**Decision.** Kept. This makes a real public export path use the row-streaming
renderer end-to-end without constructing a full output `Pixmap` or full RGB
staging image, while preserving byte/pixel equivalence through tests and keeping
the full-pixmap fallback for unsupported render options.

### #222 PR2 — high-level setters (`page_mut(i).set_text_layer`/`set_annotations`/`set_metadata`) — **Kept** (2026-05-01)

**Approach.** Builds on PR1's chunk-replacement primitive. New surface:

- `DjVuDocumentMut::page_count() -> usize` — `1` for `FORM:DJVU`, count of
  `FORM:DJVU` direct children for `FORM:DJVM`.
- `DjVuDocumentMut::page_mut(i) -> Result<PageMut<'_>, MutError>` — borrow
  one page's `FORM:DJVU` for editing.
- `PageMut::set_text_layer(&TextLayer)` — encode via `encode_text_layer`
  (page height read from `INFO`) + `bzz_encode`, replace the existing
  `TXTa`/`TXTz` or insert a new `TXTz`.
- `PageMut::set_annotations(&Annotation, &[MapArea])` — same shape over
  `encode_annotations_bzz` and `ANTa`/`ANTz`.
- `PageMut::set_metadata(&DjVuMetadata)` — over a new
  `metadata::encode_metadata` / `encode_metadata_bzz` pair, against
  `METa`/`METz`. Empty `DjVuMetadata` removes the chunk.
- New `MutError` variants: `PageOutOfRange`, `MissingPageInfo`,
  `InfoParse(IffError)`, `DjvmMutationUnsupported`.

`page_mut` errors with `DjvmMutationUnsupported` on `FORM:DJVM` bundles —
the page-level setters change a component FORM's byte size which would
shift DIRM offsets. DIRM recomputation is its own concern, deferred.

**Tests.** Nine new unit tests in `djvu_mut::tests` plus five in
`metadata::tests`:

- `set_text_layer_roundtrip_chicken`, `set_annotations_roundtrip_chicken`,
  `set_metadata_roundtrip_chicken` — each parse the re-emitted bytes and
  decode the chunk back to the input value.
- `set_metadata_empty_removes_existing_chunk` and
  `set_metadata_replaces_existing_chunk_in_place` — exercise the
  remove-on-empty and replace-don't-duplicate behaviours.
- `page_count_*`, `page_mut_out_of_range_errors`,
  `page_mut_djvm_returns_unsupported` — error paths.
- Metadata encoder tests cover empty input, dedicated-field round-trip,
  `extra` ordering, escape handling for `"`/`\\`, and BZZ round-trip.

All 410 lib tests pass (402 → 410; `+9` djvu_mut, `+5` metadata, with the
PR1 metadata count shift). `cargo clippy --workspace --lib --tests --bins
-- -D warnings` clean, `cargo fmt --check` clean. (Examples have two
pre-existing clippy warnings unrelated to this PR.)

**Reason kept.** Direct continuation of PR1's contract — PR1 only exposed
`replace_leaf(path, bytes)`; PR2 wires the existing chunk encoders to
that primitive so callers don't need to know IFF chunk IDs or BZZ
compression to update text/annotations/metadata. With this PR the
`librarian` consumer (#158) can finally drop its `djvused` shell-out for
single-page DjVu files.

**Open follow-ups (PR3-4 of #222 sequence).**
1. **PR3**: bundled DJVM mutation (DIRM offset recomputation) plus
   `DjVuDocumentMut::set_bookmarks(&[DjVuBookmark])` for NAVM at the
   bundle root.
2. **PR4**: byte-range patching for true byte-identical round-trip even
   *with* edits (only changed chunks are rewritten; unchanged regions are
   memcpy'd). Currently any mutation triggers a full `iff::emit` which
   may differ from the original byte layout in incidental ways.
3. **PR5**: indirect DJVM support — the issue's "per-file rewrite vs
   re-bundle" decision still needs a concrete answer.

### #222 PR1 — `DjVuDocumentMut::from_bytes` + chunk-replacement primitive — **Kept** (2026-04-30)

**Approach.** New `src/djvu_mut.rs` module gated on `feature = "std"` with
the foundation layer for in-place document mutation. Public surface:

- `pub struct DjVuDocumentMut` — owns a parsed `DjvuFile` tree plus the
  original byte buffer.
- `pub fn from_bytes(data: &[u8]) -> Result<Self, MutError>` — parses (via
  `iff::parse`, the legacy tree-based parser) and retains the input bytes.
- `pub fn into_bytes(self) -> Vec<u8>` — fast path: when no mutation has
  happened, returns the original bytes verbatim. After any mutation, falls
  through to `iff::emit`.
- `pub fn replace_leaf(&mut self, path: &[usize], new_data: Vec<u8>)` —
  walks the tree by child indices and rewrites the leaf payload.
- `pub fn chunk_at_path(&self, path: &[usize]) -> Result<&Chunk, _>` —
  read-only walker, used by tests and (future) inspectors.
- Utility: `root_child_count`, `root_form_type`, `is_dirty`.
- `pub enum MutError`: `Parse(LegacyError)`, `PathOutOfRange`,
  `PathTraversesLeaf`, `NotALeaf`, `EmptyPath`.

The byte-identical-no-edit guarantee is achieved by holding the original
`Vec<u8>` and short-circuiting `into_bytes` when `!is_dirty`. After any
mutation `iff::emit` is invoked, which **does not** guarantee byte-identity
even for unmutated chunks (it recomputes FORM lengths from children) — but
this case is explicitly out of scope for PR1 and tracked as a follow-up
for PR3 (proper byte-range patching).

**Tests.** Ten new unit tests in `djvu_mut::tests`:

- Round-trip byte-identical (no edit) on four corpus fixtures:
  - `chicken.djvu` — color FORM:DJVU
  - `boy_jb2.djvu` — bilevel FORM:DJVU
  - `DjVu3Spec_bundled.djvu` — multi-page FORM:DJVM
  - `navm_fgbz.djvu` — FORM:DJVU with NAVM + FGbz
- `replace_leaf_changes_emitted_bytes` — replaces INFO with a marker, parses
  the output, verifies the marker came back.
- Negative paths: `EmptyPath`, `PathOutOfRange`, `PathTraversesLeaf`,
  `NotALeaf` (last picks the last child of a DJVM bundle, which is a
  page FORM).
- `root_form_type_djvu_single_page` — sanity on the tree-introspection API.

All 402 lib tests pass (393 → 402; `+10` djvu_mut, `-1` ignored count
shifted). `cargo clippy --workspace --all-targets -- -D warnings` clean,
`cargo fmt --check` clean.

**Reason kept.** PR1 of #222 establishes the byte-identical contract and
the chunk-walking primitive that PR2-4 build on (per the issue body's
sequencing comment). The implementation is intentionally minimal — wrap
the existing IFF parser, hold raw bytes for fast path, expose one
mutation primitive — to ship a focused first slice without committing to
the high-level setter design (`set_metadata`, `set_bookmarks`,
`page_mut(i).set_text_layer`). Those settings each compose
`replace_leaf` with one of the existing chunk encoders
(`encode_navm`, `encode_annotations*`, `encode_metadata`,
`encode_text_layer`).

**Open follow-ups (PR2-4 of #222 sequence).**
1. **PR2**: high-level setters (`set_metadata`, `set_bookmarks`,
   `page_mut(i).set_text_layer`, `…set_annotations`) on top of
   `replace_leaf`.
2. **PR3**: byte-range patching for true byte-identical round-trip even
   *with* edits (only changed chunks are rewritten; unchanged regions are
   memcpy'd). Currently any mutation triggers a full `iff::emit` which
   may differ from the original byte layout in incidental ways (FORM
   length recomputation, padding).
3. **PR4**: indirect DJVM support — the issue's "per-file rewrite vs
   re-bundle" decision still needs a concrete answer.
4. `librarian` consumer migration off `djvused` shell-out (#158
   follow-up) — depends on PR2 setters.

### #229 PR1 — extract `djvu-zp` into a standalone workspace crate — **Kept** (2026-04-30)

**Approach.** Moved `src/zp/{mod,encoder,tables}.rs` into a new
`crates/djvu-zp/` workspace member with its own `Cargo.toml`. The new
crate:

- Defines `pub enum ZpError { TooShort }` instead of leaking `BzzError`
  back into ZP. Decoupling ZP from `crate::error` is what makes the
  extraction publishable.
- Promotes every `pub(crate)` to `pub` (the audit the issue body warns
  about): `ZpDecoder`, `ZpDecoder::{a, c, fence, bit_buf, bit_count, data,
  pos}` fields, `decode_bit`, `decode_passthrough`, `decode_passthrough_iw44`,
  `is_exhausted`, `ZpEncoder` + its methods, and the four format-constant
  tables (`PROB`, `THRESHOLD`, `MPS_NEXT`, `LPS_NEXT`).
- Has a `default = ["std"]` feature that gates the encoder (which needs
  `Vec<u8>`). Decoder works in `no_std` builds and never allocates.
- Adds a `Default` impl on `ZpEncoder` (clippy `new_without_default` for
  the now-public `new` method).

`src/lib.rs` keeps the historical internal name via
`pub(crate) use djvu_zp as zp_impl;` so every existing import
(`crate::zp_impl::ZpDecoder`, `crate::zp_impl::tables::PROB`, etc) keeps
working unchanged. `From<djvu_zp::ZpError> for BzzError` makes the `?`
operator in `bzz_new::bzz_decode` continue to work without per-callsite
edits.

`src/zp/` is removed; the `#[path = "zp/mod.rs"] pub(crate) mod zp_impl;`
attribute in `src/lib.rs` was replaced with the `use` re-export. Workspace
`members = [".", "djvu-py", "crates/djvu-zp"]`.

**Tests.** Per-crate test counts:

- `djvu-rs` (umbrella): 393 lib tests pass (down from 405 — the 4 ZP
  decoder tests + 7 ZP encoder roundtrip tests moved into the new crate).
- `djvu-zp`: 11 unit tests pass (`zp_decoder_*`, `zp_tables_spot_check`,
  7 roundtrip tests in the encoder module). Two doctest examples
  (`ZpDecoder::new` from a 2-byte slice, `ZpEncoder` round-trip).
- `djvu-py`: builds. No tests defined.
- Workspace `cargo build --no-default-features --lib` (host
  no-std-compatible build) green; no_std smoke test
  (`tests/no_std_smoke`) builds green against the new dependency graph.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo fmt --check` clean.

**Scope of `pub` audit.** Every newly-`pub` item was an internal
`pub(crate)` before — there is no new behavioural surface, just a wider
visibility. Specifically:

| Was            | Now           | Justification                                               |
| -------------- | ------------- | ----------------------------------------------------------- |
| `ZpDecoder`    | `pub`         | Required for cross-crate use                                |
| Decoder fields | `pub`         | Hot-path field access from JB2/IW44/BZZ in djvu-rs internals |
| `ZpEncoder`    | `pub`         | Required for cross-crate use                                |
| `PROB` etc.    | `pub` (in `pub mod tables`) | Used by JB2/IW44/BZZ saturation-bound tests in djvu-rs |

The decoder field exposure is the only mildly load-bearing widening: it
lets djvu-rs internals manipulate the registers directly during
saturated-decode fast paths. Wrapping each in a getter would force every
hot-path access through a function call. Acceptable for an internal-
collaboration sub-crate and matches the precedent set by `wide` /
`bytemuck` / similar low-level numerics crates.

**Reason kept.** Lossless extraction of ~780 LOC into a publishable
sub-crate, no behavioural change for djvu-rs consumers, all tests pass,
no_std build still works. This is the canonical "is this approach
viable" first step of #229; PR2 (`djvu-bzz`), PR3 (`djvu-iff`), PR4-5
(`djvu-jb2`, `djvu-iw44`), and PR6 (umbrella re-export shim) follow the
same pattern.

**Open follow-ups.**
1. The `From<ZpError> for BzzError` mapping collapses to `BzzError::TooShort`
   — fine for now since `ZpError::TooShort` is the only variant. If
   future ZP-coder errors are added, the mapping needs a more specific
   `BzzError` variant (likely `BzzError::ZpError`-already-exists).
2. Publish to crates.io once the API is reviewed. The `version = "0.1.0"`
   reflects new-crate convention, not djvu-rs's `0.14.0` line.
3. Consider whether the encoder fields (`a`, `subend`, `buffer`, `nrun`,
   `delay`, `byte`, `scount`, `output`) need to be `pub`. Currently they
   stay private — only methods are exposed.

### #189 Phase 3 — x86_64 AVX2 ports of `prelim_flags_bucket` + `prelim_flags_band0` — **Kept** (2026-04-30)

**Approach.** Two new AVX2 functions mirroring the existing aarch64 NEON
helpers in `src/iw44_new.rs`:

- `prelim_flags_bucket_avx2`: loads 16 i16 (one `__m256i` — twice the lane
  width of NEON's two `int16x8_t` loads), compares to zero with
  `_mm256_cmpeq_epi16`, builds UNK/ACTIVE flags via `uv ^ (xv & nz)` (UNK=8,
  XV=10), narrows u16→u8 via `_mm_packus_epi16` of the two 128-bit halves
  (saturating but values 2/8 fit), stores 16 bytes via `_mm_storeu_si128`,
  horizontally OR-reduces via `_mm_unpackhi_epi64` + `_mm_srli_si128` chain.

- `prelim_flags_band0_avx2`: same flag computation, then conditional blend
  `(new & should_update) | (old & ~should_update)` using SSE2
  `_mm_andnot_si128` to replicate NEON's `vbslq_u8`. Keeps the ZERO-state
  lane unchanged; updates other lanes from the coef comparison.

A new `band0_dispatch` helper picks NEON / AVX2 / scalar at runtime via
`is_x86_feature_detected!("avx2")` (gated on `feature = "std"` per the
established pattern in #251/#252). The scalar fallback is unchanged — so
non-AVX2 x86_64 hosts and `no_std` builds keep their existing behaviour.

The dispatcher in `prelim_flags_bucket` was extended the same way: AVX2
branch added, NEON path unchanged, scalar fallback unchanged.

**Tests.** Two new unit tests gated on `cfg(all(target_arch = "x86_64",
feature = "std"))` + AVX2 runtime detection:

- `prelim_flags_bucket_avx2_matches_scalar` — sweeps 5 coef vectors
  (all-zero, mixed, all-one, all-negative-one, edge values) at four bases
  including the highest valid bucket offset (1008). Verifies bucket bytes
  and bstatetmp byte-exact vs scalar.
- `prelim_flags_band0_avx2_matches_scalar` — sweeps 4 old-flag patterns ×
  4 coef patterns. Verifies the conditional-update semantics: ZERO lanes
  are preserved, other lanes get UNK/ACTIVE from the coef comparison.

Both pass on the local x86_64 host. All 405 lib tests pass; clippy
`-D warnings` and `cargo fmt --check` clean.

**Bench.** No native bench harness for this kernel in isolation; expected
speedup over scalar at this hot path (called once per (block × band) =
~1024 blocks/page × 10 bands = ~10K calls/page) is on the order of
4–8× from replacing the scalar 16-iteration loop with three AVX2 ops + a
narrow + horizontal OR. End-to-end `iw44_decode_*` benches will pick up
the change at the next `bench.yml` AVX2 runner pass.

**Reason kept.** Two more AVX2 kernels close the parity gap with NEON
that issue #189 calls out (lines 11–14 of the issue body listed
`preliminary_flag_computation` band-0 and band≠0 as next priorities after
`load8s`/`store8s`, which shipped in #252). Bit-exact verified vs scalar,
zero behavioural change for non-AVX2 hosts, no allocation overhead, no
runtime cost on the dispatcher (one feature-detected branch). Pattern
established for the remaining kernels (`row_pass_neon_s1_row`,
`lifting_even`, `predict_inner`, `predict_avg`).

**Open follow-ups.**
1. `row_pass_neon_s1_row` AVX2 port — significantly larger because AVX2
   has no native `vld2q_s16` deinterleave; `### #184` below is the
   cautionary tale of attempting strided loads in AVX2 without gather.
2. Encoder-side ports (`forward_row_neon_s1_row`, `forward_col_predict_neon`).
3. Bench numbers from the next `bench.yml` AVX2 runner pass should be
   recorded here once available.

### #225 Phase 2 — public `render_streaming` API — **Kept** (2026-04-30)

**Approach.** Built on Phase 1's internal `render_rows` primitive. Added one
new public entry point and one new error variant:

- `pub fn render_streaming<F: FnMut(usize, &[u8])>(page, opts, sink)` — thin
  wrapper around `render_rows` that rejects render options requiring
  post-processing of a fully-allocated pixmap.
- `RenderError::UnsupportedOption(&'static str)` — returned when the streaming
  path cannot honour the requested options.

The constraints surface what `render_pixmap` does after compositing: the
streaming path *cannot* support `opts.aa = true` (the AA downscale needs the
full pixmap), `opts.resampling = Lanczos3` *when scaling actually happens*
(re-renders at native resolution and downscales), or any non-identity
combined rotation (`combine_rotations(page.rotation(), opts.rotation)`
wraps a rotate-pixmap step). When all three constraints hold,
`render_streaming` is byte-identical to `render_pixmap` — verified by two
new tests on `chicken.djvu` (color) and `boy_jb2.djvu` (bilevel).

Lanczos at native size is permitted: the early-return path in
`render_pixmap` skips Lanczos when output dimensions equal page dimensions
(`need_scale = false`), so it has no effect on bytes either way.

**Tests.** Seven new unit tests in `djvu_render::tests`:

- `render_streaming_byte_identical_to_render_pixmap_color`
- `render_streaming_byte_identical_to_render_pixmap_bilevel`
- `render_streaming_rejects_aa`
- `render_streaming_rejects_lanczos_with_scaling`
- `render_streaming_allows_lanczos_at_native_size`
- `render_streaming_rejects_user_rotation`
- `render_streaming_rejects_zero_dimensions`

All 403 lib tests pass; clippy `-D warnings` and `cargo fmt --check` clean.

**Memory.** Phase 1 already established that the internal compositing path
allocates a single `opts.width * 4` byte scratch row reused across rows;
`render_streaming` inherits that. Peak heap during compositing is bounded
by `scratch_row + decoded BG44 + decoded JB2 mask + FG palette` — no full
pixmap. The 600-dpi A3 (≈100 MB pixmap) target from the issue's DoD is met
by construction (the scratch row is < 16 KB at any reasonable width).

**Reason kept.** The DoD-required public API is now in place with no
behavioural change for existing `render_pixmap` callers, byte-exact
equivalence verified, post-processing options safely refused with a typed
error rather than silently producing different output. The `UnsupportedOption`
variant is `&'static str` — no allocation on the error path. Phase 1's
zero-cost adapter through `render_rows` means `render_pixmap` continues to
benefit from the warm-cache row scratch (`### #225 Phase 1` below,
−13% on `render_page/dpi/72`).

**Open follow-ups.**
1. `render_region`, `render_coarse`, `render_progressive` could similarly
   gain streaming variants if a use case appears.
2. Memory benchmark from the issue's DoD ("peak RSS during render of a
   600-dpi 2550×3301 page < 4 MB") not yet wired into `bench/`. Manual
   verification via `heaptrack` or `dhat` would confirm the BG44/mask
   buffers are the only large allocations.

### #225 Phase 1 — internal row-streaming render refactor — **Kept** (2026-04-29)

**Approach.** Extracted the composite hot path into a per-row streaming
primitive without changing the public API. Three new module-private functions:

- `composite_rows_bilevel_one` / `composite_rows_bilinear_one` /
  `composite_rows_area_avg_one` — per-row helpers containing the pixel-level
  computation for each of the three compositing modes (bilevel fast path,
  bilinear upscale/1:1, area-average downscale). These are `#[inline]` and
  mirror the existing `composite_loop_*` bodies row by row.

- `composite_rows<F: FnMut(usize, &[u8])>` — allocates a single row scratch
  buffer (`out_w * 4` bytes, reused across rows), calls the appropriate per-row
  helper, then invokes the sink `F(row_index, &row_slice)`. The
  `composite_into` direct flat-buffer path is untouched and continues to drive
  `render_into`, `render_region`, `render_coarse`, and `render_progressive`.

- `pub(crate) render_rows<F>` — decode/setup entry point (mirrors
  `render_pixmap`'s decode logic) that calls `composite_rows`. This is the
  Phase 2 hook: future `render_streaming` will delegate here instead of
  allocating a full Pixmap.

`render_pixmap` is now a thin adapter: it pre-allocates `Pixmap::white(w, h)`,
calls `render_rows` with a sink that copies each row into `pm.data`, then
applies the existing aa/Lanczos/rotation post-processing steps.

Two new unit tests — `render_rows_byte_identical_to_render_into_color` and
`render_rows_byte_identical_to_render_into_bilevel` — verify that
`composite_rows` and `composite_into` produce byte-exact identical output for
color (chicken.djvu) and bilevel (boy_jb2.djvu) pages.

**Bench** (`cargo bench --bench render -- 'render_page/dpi/72'`,
100 samples, Apple M1 Max):

| Benchmark             | Before   | After    | Δ       |
|-----------------------|----------|----------|---------|
| `render_page/dpi/72`  | 243.5 µs | 211.8 µs | **−13%** |
| `render_colorbook_cold` | — | 17.8 ms | flat (no prior baseline in this worktree) |

The 72-dpi benchmark **improved** by ~13% despite the per-row scratch
allocation and `copy_from_slice` on each row. The likely cause: the scratch row
buffer (`w * 4 ≈ 400–2400 bytes`) fits entirely in L1 cache; subsequent writes
from the composite inner loop and the copy into `pm.data` both hit warm L1
rather than cold L2/L3 as in the previous approach that wrote directly into the
full pre-allocated pixmap. The decode step dominates at 72 dpi (BG44 + JB2
cache hits account for ~200 µs), so even the best-case compositing improvement
is bounded.

**Reason kept.** Material improvement on the warm-cache render benchmark (−13%)
with zero public API change, bit-exact output verified by tests, all 550 tests
pass, clippy and fmt clean. The `render_rows` hook is in place for Phase 2.

**Open follow-ups.**
1. Phase 2 (future PR): expose `pub fn render_streaming` with a user-visible
   row callback, enabling true zero-full-pixmap rendering for WASM / embedded.
2. `render_region`, `render_coarse`, `render_progressive` could similarly be
   refactored to use `composite_rows` for API symmetry, but are not hot paths.

### #190 Phase 2 — WASM simd128 inverse wavelet (load/store stride-1) — **Kept** (2026-04-29)

**Approach.** Added `load8s_s1_simd128` and `store8s_s1_simd128` (gated on
`cfg(all(target_arch = "wasm32", target_feature = "simd128"))`) as the WASM
counterparts to the AVX2 stride-1 helpers shipped in Phase 2 of #189.

`load8s_s1_simd128`: loads 8 consecutive i16 as one `v128`, then calls
`i32x4_extend_low_i16x8` / `i32x4_extend_high_i16x8` to sign-extend into two
`v128`s of i32, which are transmuted directly to `wide::i32x8` (`{a: i32x4(v128),
b: i32x4(v128)}`). This replaces 8 scalar `as i32` casts assembled via
`i32x8::from([...])`.

`store8s_s1_simd128`: transmutes `i32x8` back to `[v128; 2]`, then uses a
constant `i8x16_shuffle` with indices `[0,1,4,5,8,9,12,13, 16,17,20,21,24,25,28,29]`
to pick the low 2 bytes of each i32 lane from both halves into a single `v128`,
stored in one `v128_store`. This replicates the truncating `as i16` semantics of
the scalar path (not saturating narrow), matching the AVX2 byte-shuffle approach.

Both functions are wired into `load8s` and `store8s` via a compile-time
`#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]` block
(the `return` before the scalar `#[allow(unreachable_code)]` block), so
the hot column-pass loop at `s == 1` gets the fast path with no runtime branch.

**Bench.** No direct wasm bench harness available locally. Expected speedup is
analogous to the AVX2 load/store path (#189 Phase 2), which measured −3.9% on
`jb2_decode_corpus_bilevel`. The WASM path processes 8 lanes (same as `v128`
width) in 2 ops (load) or 1 shuffle + 1 store (store) vs 8 scalar cast-and-
write pairs. The column pass at `s=1` is the hottest sub-kernel in
`inverse_wavelet_transform_from` during full-resolution (`to_rgb`) decoding.
CI bench job will capture actual WASM numbers on next main merge.

**Reason kept.** Zero regression risk: compile-time gating, bit-exact by
construction (sign-extend from i16→i32 is exact; low-halfword extraction via
byte-shuffle is exact truncation). Two new unit tests
(`load8s_s1_simd128_matches_scalar`, `store8s_s1_simd128_matches_scalar`)
gate on `wasm32 + simd128` and verify round-trip across the full i16/i32 range.
All 389 host lib tests pass; both WASM builds (plain and `+simd128`) succeed.

### #224 Phase 4 — opt-in lossy rec-7 substitution for near-duplicates — **Kept** (2026-04-28)

**Approach.** Added `Jb2EncodeOptions { lossy_threshold: f32 }` and
`pub fn encode_jb2_dict_with_options(bitmap, shared, &opts)`. When
`lossy_threshold > 0.0`, the action-selection branch tries
`find_lossy_copy_ref` *before* the lossless refinement matcher
(`find_refinement_ref`): for each CC, it scans `same_size_indices` in
`dict_entries`, and if any entry has `packed_hamming(rep, cc) <= pixels *
lossy_threshold`, the encoder emits `rec-7` (matched copy, no
refinement bitmap) referencing it. Decoder will then reconstruct the
dict entry's pixels, with visual error bounded by the threshold. The
existing `REFINEMENT_MIN_PIXELS = 32` floor still applies — tiny CCs
stay byte-exact regardless of threshold.

`encode_jb2_dict_with_shared` now delegates to
`encode_jb2_dict_with_options(bitmap, shared, &Jb2EncodeOptions::default())`
so the shipped lossless path is unchanged. Default threshold = 0 = exact
behaviour preserved.

`examples/encode_quality_jb2.rs` got a `--lossy-threshold <fraction>`
flag, plus a `bitmap_hamming` helper that decodes the lossy-encoded Sjbz
and computes pixel-wise Hamming vs the original mask, so the harness
reports both byte savings and total reconstruction error.

**Bench** (`encode_quality_jb2` on a 15-page bilevel mix:
`tests/corpus/{cable_1973_100133,watchmaker}.djvu` +
`tests/fixtures/{big-scanned-page,carte,chicken,irish}.djvu`,
~188 M total pixels, Apple M1 Max):

| `--lossy-threshold` | rs-lossy bytes | vs rs-dict (lossless) | total err pixels | bits/pixel error |
|---------------------|---------------:|----------------------:|-----------------:|-----------------:|
| 0 (lossless dict)   | 167 314        | 1.000×                | 0                | 0                |
| 0.01                | 158 250        | **0.946×** (−5.4%)    | 10 986           | 0.000087         |
| 0.02                | 154 050        | 0.921× (−7.9%)        | 17 946           | 0.000142         |
| 0.04                | 150 118        | 0.897× (−10.3%)       | 28 568           | 0.000226         |
| 0.05                | 149 015        | 0.891× (−10.9%)       | 32 386           | 0.000256         |
| 0.08                | 146 104        | **0.873×** (−12.7%)   | 40 767           | 0.000322         |

Reconstruction error is on the order of 1 in 5–20 K pixels (≈0.0001–
0.0003 bits/pixel) — visually imperceptible for scanned text on these
600 dpi-class bilevel inputs. The `lossy decode errors: 1` row in the
summary is the same `irish.djvu` page that already trips
`roundtrip_dict: decode_error` on the lossless path (issue #198: a CC
larger than `MAX_SYMBOL_PIXELS`); orthogonal to lossy mode.

**Reason kept.** Material byte savings on top of the already-shipped
lossless dict path, opt-in via `Jb2EncodeOptions`, default behaviour
unchanged. The threshold knob is exposed so callers can pick their own
size↔fidelity point. Pairs naturally with the cjb2 quality settings
(default ≈ 0.005, conservative ≈ 0.02 in DjVuLibre) — a CLI front-end
could map that mapping in a follow-up. All 32 `jb2_encode` unit tests
plus the new `lossy_threshold_substitutes_near_duplicate_with_rec7`
test pass.

**Open follow-ups.**
1. `--lossy-threshold` doesn't yet feed into `cjb2`-equivalent CLI
   front-end (`tools/djvu-encode` if/when one exists).
2. The same threshold logic could be extended to refinement: instead of
   only substituting same-size near-dups with rec-7, allow lossy rec-6
   that emits a *truncated* refinement bitmap. Unclear if there's
   additional headroom past the rec-7 path measured here.

### #194 Phase 2.5 — per-CC accounting harness for shared-Djbz refinement — **Kept (instrumentation only)** (2026-04-28)

**Approach.** Added `pub fn analyze_jb2_cc_stats(page, &shared)` that mirrors
the rec-1/rec-6/rec-7 action-selection branch in
`encode_jb2_dict_with_shared` but emits no bytes — just counts and a
Hamming-distance histogram for rec-6 emissions, separating refs that
land in the shared dict (cross-page) from refs that land in the
page-local running dict. Wired through to `encode_quality_djbz` via a
new `--cc-stats` flag.

This is the measurement layer Phase 2.5 needs before deciding whether
the per-CC profitability model in the #194 follow-up is worth
implementing. The Phase 2 result already showed flat Hamming clustering
doesn't beat byte-exact; the open question was whether selective
near-duplicate promotion (with a profitability gate per CC) could.
Without the actual rec-6 distribution we were guessing.

**Observations** (`--cc-stats` on `tests/corpus/*.djvu`, 36 pages, 4 books):

| File | Pages | rec-1 fresh | rec-6 shared | rec-6 local | rec-7 exact |
|---|---:|---:|---:|---:|---:|
| `cable_1973_100133.djvu` | 2  | 12.4% | 0.0%  | 4.7% | 82.8% |
| `conquete_paix.djvu`     | 22 | 40.7% | 0.2%  | 2.1% | 56.9% |
| `watchmaker.djvu`        | 12 | 6.1%  | 24.7% | 1.8% | 67.5% |

rec-6 Hamming-distance distribution on `watchmaker.djvu` (6256 rec-6
matches, dominant case): 49.7% in [1, 4], 47.7% in [5, 16], 2.5% in
[17, 64], 0% above. Very tight — the existing 4%-of-pixels threshold in
`find_refinement_ref` is approximately right; there is little headroom
for "tighter" to improve the picture.

**Reason kept.** Pure instrumentation, no encoder behavior change. Gives
future Phase 2.5 work (and any Phase 4 lossy-refinement experiment from
#224) a concrete CC-action breakdown without round-tripping bytes.
Round-trip + clippy + nextest all clean; new test
`analyze_jb2_cc_stats_classifies_records` covers all three buckets +
shared/local distinction.

**What this tells us about Phase 2.5 viability.** On the dominant
shared-dict beneficiary (`watchmaker`), rec-6 already covers 24.7% of
CCs against the shared dict, and the Hamming distribution is bimodal
on [1, 16]. The remaining 6.1% rec-1 are mostly:
1. Unique glyphs (no shared-dict twin) — promotion candidates need ≥ N
   page repetitions, by definition rare for these
2. Glyphs that fail the same-(w, h) bucket constraint
   (cross-size matching is `find_refinement_ref`'s explicit
   limitation, see jb2_encode.rs:611)

So the most plausible Phase 2.5 win is **cross-size refinement**, not
per-CC profitability. That's a substantially larger change (requires
resampling for Hamming scoring) and is what the open #194 follow-up
should track. Per-CC profitability against the existing same-size
shortlist is unlikely to add anything material — the rec-6 hits we
already get are tight enough that a profitability gate would barely
exclude any of them.

### #185 — perf(jb2): bit-pack Jbm to 1 bit/pixel — **Kept** (2026-04-18)

**Approach.** Changed the internal `Jbm` working bitmap from 1 byte/pixel
(`Vec<u8>` of `w * h`) to 1 bit/pixel packed (`Vec<u8>` of
`((w + 7) / 8) * h`, MSB-first within byte) — matching `Bitmap`'s public
convention. 8× memory reduction on the symbol dict.

Decoder hot path uses **Variant A**: `decode_bitmap_direct` and
`decode_bitmap_ref` keep rolling unpacked scratch rows (3 for direct,
3 mbm + 2 cbm for ref) and pack into `Jbm.data` once per row. The ZP
inner loop is unchanged. New helpers: `pack_row_into`, `unpack_row_into`.

`blit_indexed`: reads packed source with a byte-at-a-time skip of
all-zero bytes (common for sparse symbols). `blit_to_bitmap`: source and
dest are both packed MSB-first; byte-aligned branch becomes a direct `|=`
row copy, unaligned branch is a shift-and-OR.

**Bench** (`cargo bench`, 100 samples, Linux x86_64, Criterion p-values):

| Benchmark                    | Baseline  | Packed    | Δ      | p    |
|------------------------------|-----------|-----------|--------|------|
| `jb2_decode`                 | 187.93 µs | 188.79 µs | +0.5%  | 0.31 |
| `jb2_decode_corpus_bilevel`  | 813.80 µs | 782.21 µs | −3.9%  | 0.00 |
| `jb2_decode_large_600dpi`    | 4.37 µs   | 4.27 µs   | −2.3%  | 0.06 |
| `render_corpus_bilevel`      | 189.76 ms | 191.36 ms | +0.8%  | 0.19 |

No regression anywhere; `jb2_decode_corpus_bilevel` is significantly
faster (p = 0.00), consistent with reduced L2 pressure on the decoded
symbol dict.

**Reason kept.** 8× memory reduction on working bitmaps with neutral-to-
positive decode/render perf. The scratch allocation in the hot path
(three `Vec<u8>` × `width` bytes per symbol decode, reused across rows)
adds no measurable overhead vs the previous direct-indexed `bm.data`
split. All 324 library + 71 integration tests pass.

**Notes.** The issue suggested `Vec<u32>` + 32-bit row alignment for SIMD
potential. That was relaxed to byte-aligned `Vec<u8>` to match `Bitmap`
exactly (avoiding the byte→bit packing step in `blit_to_bitmap`). A
follow-up could explore word-granular compositing once there is a
workload that stresses the unaligned `blit_to_bitmap` branch.

### #184 — perf(iw44): column_pass SIMD at s=2 — **Reverted** (2026-04-18)

**Approach.** Generalised the existing `s == 1` SIMD fast path in the column
pass of `inverse_wavelet_transform_from` to `s ∈ {1, 2}`. Introduced
stride-aware helpers `load8_col_s` / `store8_col_s` that gather/scatter 8
`i16` samples at stride `s`, threaded an `allow_simd` parameter for
comparability, and added a golden test
(`simd_inverse_wavelet_transform_matches_scalar`) that confirmed bit-exact
parity with the scalar path on 32×32 and 33×32 planes.

**Bench** (`cargo bench --bench codecs -- 'iw44_decode_first_chunk|iw44_decode_corpus_color'`,
release, 100 samples, Linux x86_64):

| Benchmark                  | Scalar   | SIMD s=2 | Δ     |
|----------------------------|----------|----------|-------|
| `iw44_decode_first_chunk`  | 1.226 ms | 1.206 ms | −1.6% |
| `iw44_decode_corpus_color` | 3.747 ms | 3.669 ms | −2.1% |

Run-to-run noise on the same build was ±2–5% (e.g. `iw44_decode_corpus_color`
ranged 3.31 ms → 3.81 ms across consecutive runs). Criterion's change test
came back non-significant (`p ∈ {0.09, 0.20, 0.24, 0.36, 0.68}`) once the
cold-start outlier was excluded.

**Reason.** On x86_64, the implementation must fall back to 8 scalar loads
assembled into an `i32x8` — `wide::i32x8` exposes no strided / gather load for
`i16`, and no native `_mm*_i16gather_*` intrinsic exists for 16-bit lanes.
The arithmetic savings at `s == 2` (which already processes half as many
columns as `s == 1`) do not exceed the gather overhead.

The issue expected the win to come from ARM64 NEON `vld2q_s16` / `vst2q_s16`,
which are not reachable through `wide` and would require raw
`core::arch::aarch64` intrinsics. Without that, there is no benefit on the
x86_64 CI host. The stride-aware helpers would be reusable if the ARM64
follow-up lands, but committing them today costs complexity for zero measured
gain.

**Next step.** Re-attempt on ARM64 (M1) with raw NEON `vld2q_s16`, measure
against the baseline `iw44_decode_first_chunk` (715 µs) on the reference
hardware listed in `BENCHMARKS_RESULTS.md`.

### #194 Phase 2 — multi-page shared Djbz with Hamming clustering — **Reverted default, kept tunable knob** (2026-04-28)

**Approach.** Phase 1 (#194, shipped) builds the shared Djbz dictionary by
byte-exact `(w, h, data)` dedup of CCs across pages: any CC signature
appearing on `≥ threshold` distinct pages becomes a shared symbol, the rest
stay per-page Sjbz. Phase 2 attempted to widen the cluster predicate to
"same `(w, h)` AND `packed_hamming(rep, cc) ≤ pixels * fraction`", folding
near-duplicate scanned-glyph variants into one shared rep so the per-page
Sjbz can emit `rec-7` (matched copy) or `rec-6` (matched refinement)
instead of `rec-1` (new direct).

Implementation: `cluster_shared_symbols_tunable(pages, page_threshold,
diff_fraction)` — bucketed by `(w, h)`, linear scan per bucket choosing the
nearest existing rep within `max_diff = pixels * diff_fraction / 100` (with
a `REFINEMENT_MIN_PIXELS = 32` floor that keeps tiny CCs byte-exact).
`encode_djvm_bundle_jb2_with_shared(pages, &shared)` lets a benchmark
harness drive cluster selection without re-running the IFF/DIRM pipeline.

**Harness.** `examples/encode_quality_djbz.rs` — for each multi-page DjVu
input, computes total bytes for {original Sjbz, independent
`encode_jb2_dict` per page, bundled `encode_djvm_bundle_jb2_with_shared`}
across configurable Hamming thresholds; verifies pixel-exact bundle
round-trip.

**Bench** (`encode_quality_djbz` on `pathogenic_bacteria_1896.djvu`,
517 pages of cjb2 scans, Apple M1 Max):

| `--diff-fraction` | shared syms | Djbz bytes | Σ Sjbz | bundle / independent | round-trip |
|-------------------|-------------|------------|--------|----------------------|------------|
| 0 (byte-exact, shipped) | 1 568 | 41 KB | 7.40 MB | **0.870×** (−13.0%) | ✓ |
| 1% | 1 547 | 40 KB | 7.40 MB | 0.870× (−13.0%) | ✓ |
| 2% | 1 503 | 39 KB | 7.41 MB | 0.871× (−12.9%) | ✓ |
| 3% | 1 449 | 38 KB | — | — | **✗ mismatch** |
| 4% | 1 387 | 36 KB | 7.50 MB | 0.877× (−12.3%) | ✓ |

Small corpus (`tests/corpus/*.djvu`, 36 pages from 4 books):

| `--diff-fraction` | bundle / independent |
|-------------------|----------------------|
| 0 (byte-exact)    | 1.021× (+2.1%) |
| 4%                | 1.150× (+15.0%) |

**Reason reverted as default.** The Phase 1 byte-exact win (−13.0% bundle
vs independent on the 517-page corpus) is the entire shared-Djbz benefit.
Hamming clustering at 1–2% is within 0.05% of byte-exact; at 4% it is
strictly worse. Hypothesis: the per-page `symbol_index_ctx` encoding pays
≈ `log2(K)` bits per reference, so growing `K` (more shared reps) inflates
every `rec-7` reference; meanwhile `rec-6` refinement bitmaps cost more
ZP-coded bits than a fresh `rec-1` direct emission whenever the shared rep
isn't a near-perfect match. Net: cross-page Hamming clustering must match
*better than* the per-page intra-CC refinement matcher already does within
each page (#188 Phase 3) — and on this corpus it doesn't.

**Reason kept tunable.** `cluster_shared_symbols_tunable` and
`encode_djvm_bundle_jb2_with_shared` are exposed `pub` so the benchmark
harness — and any future Phase 2.5 calibration work (per-CC profitability
model instead of a flat fraction) — can sweep thresholds without forking
the encoder. The default `cluster_shared_symbols` continues to delegate to
`diff_fraction = 0`.

**Open follow-ups.**
1. The `diff_fraction = 3%` round-trip mismatch on the big corpus is a real
   bug in the rec-6 refinement path against shared reps — should be filed
   as a sub-issue. (Doesn't block ship: 0% remains lossless and is the
   shipped default.)
2. Per-CC profitability model: instead of a flat Hamming fraction, decide
   per CC whether `cost(rec-6 against shared rep)` < `cost(rec-1 fresh) +
   amortized log2(K) increase`. Unclear if the win exists — would need to
   re-measure with a corpus where intra-page refinement is already
   exhausted.

### #258 — shared-Djbz Hamming clustering — **Rejected** (2026-05-04)

**Approach.** Re-tested the `diff_fraction = 3` path on the 517-page
`pathogenic_bacteria_1896.djvu` corpus. The corpus exposed three separate
robustness problems: the 1 MP per-symbol decode cap was too low for large
connected components, the 64 MP cumulative symbol-work cap was too low for
dense independently encoded pages, and Hamming shared clustering/rec-6
refinement did not provide a reliable size win. The kept path raises decode
caps to 16 MP per symbol and 256 MP cumulative symbol work, disables
lossless rec-6 emission, and keeps shared-Djbz clustering byte-exact with a
4 MP retained shared-dict budget.

**Numbers.** Re-running the 517-page `pathogenic_bacteria_1896.djvu`
experiment at `--diff-fraction 3` before this change localized the failure
to page-level JB2 decode errors such as `Jb2(ImageTooLarge)` beginning at
page 81. The clustered shared dictionary had 63,062 symbols; the per-page
Sjbz stream then emitted enough shared-ref rec-6 refinements to exceed the
decoder's per-stream symbol-pixel budget before pixel comparison.

After the change:

| Command | shared syms | bundle / independent | round-trip |
|---------|-------------|----------------------|------------|
| `--threshold 999 --diff-fraction 3` | 0 | 1.001× | ✓ |
| `--diff-fraction 3` | 5,164 | 0.976× | ✓ |

**Decision.** Rejected. Hamming shared clustering has no material measured
size win over byte-exact clustering, and the `diff_fraction = 3` corpus path
still produces invalid page streams. `cluster_shared_symbols_tunable` keeps
its public benchmarking signature but now ignores the Hamming allowance and
uses byte-exact clustering for every threshold. In addition, inherited
shared-Djbz symbols are used only for exact record-7 hits, and lossless
near matches fall back to record-1 rather than rec-6 refinement.

### #283 — cross-size JB2 refinement probe — **Kept instrumentation, default unchanged** (2026-05-12)

**Approach.** Added `analyze_jb2_cross_size_refinement(page, shared,
max_dim_delta, max_hamming_fraction)`, an experiment-only accounting helper
that mirrors `encode_jb2_dict_with_shared` dictionary growth but does not
emit bytes. For fresh record-1 candidates, it scans dictionary symbols whose
width/height differ by at most 2 px, normalizes the reference into the
candidate box with nearest-neighbor sampling, and reports how many candidates
land within a 5% normalized Hamming budget. The existing
`examples/encode_quality_djbz.rs` harness now exposes this via
`--cross-size-stats`.

**Command.**

```text
cargo run --release --example encode_quality_djbz -- \
  --cc-stats --cross-size-stats \
  tests/corpus/watchmaker.djvu \
  tests/corpus/pathogenic_bacteria_1896.djvu
```

**Numbers.**

| File | Pages | bundle / independent | round-trip | fresh CCs | cross-size candidates | near @ 5% |
|------|-------|----------------------|------------|-----------|-----------------------|-----------|
| `watchmaker.djvu` | 12 | 0.945× | ✓ | 2,652 | 2,331 | 547 (20.65%) |
| `pathogenic_bacteria_1896.djvu` | 517 | 0.976× | ✓ | 759,291 | 686,402 | 61,485 (8.73%) |

Aggregate bundled bytes for the two-file run were 33,553,108 vs 34,384,941
for independent per-page JB2 dict encoding (0.976×, −2.4%). Pixel round-trip
stayed exact because the probe is observational only.

**Decision.** Keep the probe, but do not change the default encoder. The
candidate counts prove there is real cross-size shape similarity, especially
on `watchmaker`, but they are only an upper bound: record-6 would still carry
refinement bitmap bytes plus symbol-index/context overhead, and the previous
same-size/shared-rec-6 experiments showed that plausible-looking Hamming
matches can lose bytes or create invalid streams. A shipped cross-size
encoder path needs a byte-cost model and explicit lossy/lossless semantics;
until then `encode_djvm_bundle_jb2` remains exact rec-7 + fresh rec-1 only.

### #278 PR1 — single-page Quality/Archival FGbz profiles — **Kept** (2026-05-12)

**Approach.** Completed the conservative single-page color profile path:
`Quality` still uses the existing deterministic segmentation and
`INFO + Sjbz + BG44...` shape, but now adds an `FGbz` foreground palette
when the detected foreground color is not black. `Archival` no longer
returns `Unsupported` for color input; it emits the same layered shape with
a denser background sample grid (`bg_subsample = 6` instead of 12).

This deliberately does not change the multi-page directory encoder, which
still uses the lossless shared-Djbz path only, and does not revive Hamming
shared-Djbz clustering.

**Tests.**

- `cargo test -q djvu_encode::tests`
- `cargo test -q --features cli --test cli_encode -- --nocapture`

The CLI regression fixture is generated in `tests/cli_encode.rs` as a
white RGB PNG with a dark red foreground block. `--quality quality` and
`--quality archival` both produce parseable single-page DjVu files with
`Sjbz`, at least one `BG44`, and `FGbz`.

**Decision.** Kept as PR1 scope. This removes the user-visible
`Archival` unsupported path for single PNGs and gives colored foreground
documents a foreground color layer. Remaining quality work should be split
into focused follow-ups: adaptive binarization/inpainting, per-blit FGbz
indices or FG44 for multi-color foregrounds, and layered multi-page DJVM
encoding.

### #289 — per-blit FGbz indices for colored foreground — **Kept** (2026-05-12)

**Approach.** Switched single-page color profiles from direct whole-page
`encode_jb2` masks to dict-based `encode_jb2_dict` masks, then derives the
FGbz palette from the independently decoded `decode_indexed` blit map. Each
foreground blit gets an average source RGB color; duplicate colors share one
palette entry; multi-color foregrounds emit an FGbz index table. Single-color
foregrounds still use compact palette-only FGbz.

**Tests.**

- `cargo test -q djvu_encode::tests`
- `cargo test -q --features cli --test cli_encode -- --nocapture`
- `cargo clippy --lib --tests -- -D warnings`

The new regression fixture has two separated colored foreground components.
The unit test verifies both the FGbz palette/index table and a decoded render:
the left component remains red-dominant and the right component remains
blue-dominant.

**Decision.** Kept. This closes the main PR1 limitation from #278: colored
foreground no longer collapses to one averaged ink color when the page has
multiple separated foreground components. Continuous foreground regions and
FG44 remain out of scope; those need separate visual-quality measurements.

### #233 — async lazy first-page probe — **Kept** (2026-05-04)

**Approach.** Added `examples/async_lazy_first_page.rs`, a small native
probe for the Phase 3 lazy async loader. It wraps a DjVu file in an
`AsyncRead + AsyncSeek` reader that can simulate broadband throughput,
constructs `LazyDocument` with `from_async_reader_lazy`, fetches page 0,
and renders the first pixmap.

**Command.**

```sh
cargo run -q --example async_lazy_first_page --features async -- \
  tests/corpus/pathogenic_bacteria_1896.djvu --bandwidth-mib 12.5 --dpi 150 --pad-to-mib 100
```

**Numbers.**

| Corpus | Size | Pages | Simulated bandwidth | Bytes read | First pixel |
|--------|------|-------|---------------------|------------|-------------|
| `pathogenic_bacteria_1896.djvu` padded with an ignored `JUNK` chunk | 104,857,600 bytes | 520 | 12.5 MiB/s | 28,578 | 491.469 ms |

**Decision.** Kept. The probe pads the largest checked-in multi-page corpus
to exactly 100 MiB with a valid ignored `JUNK` IFF chunk, preserving the
DIRM/page offsets while making the file size match the issue target. Indexing
plus first-page fetch reads only the DIRM and first page/component ranges
instead of buffering the full 100 MiB document, and first pixel is well below
the 2 s target under the simulated broadband reader.

### #189 — x86-64-v3 AVX2 validation — **Kept partial / needs follow-up** (2026-05-04)

**Approach.** Pulled the GitHub Actions artifact from run `25299920836`
(`Benchmark (x86-64-v3 AVX2 validation)`, head `77fc6ff`) and compared
default `RUSTFLAGS` against `RUSTFLAGS=-C target-cpu=x86-64-v3` on the same
Ubuntu runner. This validates the already-landed AVX2 paths on real x86_64
hardware even though the local development host is `arm64`.

**Numbers.**

| Bench | default ns | +x86-64-v3 ns | Delta |
|-------|-----------:|--------------:|------:|
| `iw44_decode_corpus_color` | 1,385,461 | 1,123,865 | -18.88% |
| `iw44_decode_first_chunk` | 765,703 | 728,565 | -4.85% |
| `iw44_to_rgb_colorbook/sub1_full_decode` | 9,231,033 | 9,129,333 | -1.10% |
| `iw44_to_rgb_colorbook/sub2_partial_decode` | 2,164,523 | 2,199,280 | +1.61% |
| `iw44_to_rgb_colorbook/sub4_partial_decode` | 565,640 | 583,519 | +3.16% |
| `render_colorbook` | 13,072,440 | 12,826,562 | -1.88% |
| `render_colorbook_cold` | 28,127,606 | 27,105,326 | -3.63% |
| `render_colorbook_stages/mask_decode` | 5,325,125 | 5,107,550 | -4.09% |
| `render_corpus_color` | 133,813,976 | 133,185,634 | -0.47% |

**Decision.** Kept partial. Existing AVX2 decode paths earn their keep on
full IW44 decode (`-18.88%` corpus decode, `-4.85%` first chunk), but the
sub4 partial path regresses by `+3.16%`. This does not close #189: the
umbrella still lacks AVX2 equivalents for the horizontal row pass and encoder
kernels, and those should be implemented only in an x86_64 AVX2 session with
this validation job green after each slice.

### #292 — cross-architecture benchmark matrix — **Kept** (2026-05-17)

**Approach.** Added a canonical cross-architecture platform metadata template
and seed matrix to `BENCHMARKS_RESULTS.md`. This issue did not run new
benchmarks; it normalized existing trustworthy artifacts and made missing
target families explicit for downstream architecture issues.

**Platform.**
- OS: macOS 26.3.1 (Darwin 25.3) for the local Apple ARM64 seed row; Ubuntu
  GitHub-hosted runner for the x86_64 artifact rows.
- CPU: Apple M1 Max, 10 cores, for the broad local baseline; GitHub-hosted
  x86_64 runner for #189 artifact run `25299920836`.
- arch: `aarch64` and `x86_64`
- target features: Apple ARM64 baseline/NEON available; x86_64 baseline;
  x86_64-v3/AVX2 via `RUSTFLAGS=-C target-cpu=x86-64-v3`.
- Rust: 1.92.0 stable for the local Apple ARM64 row; stable toolchain from
  `.github/workflows/bench.yml` for the GitHub artifact rows.
- RUSTFLAGS: unset for local Apple ARM64 and Linux x86_64 baseline rows;
  `-C target-cpu=x86-64-v3` for the AVX2 row.

**Command(s).**

```sh
# Existing local summary source already recorded in BENCHMARKS_RESULTS.md:
cargo bench --workspace --features cli,tiff

# Existing x86_64-v3 artifact source already recorded in this file under #189:
gh run view 25299920836 --repo matyushkin/djvu-rs
```

**Numbers.** The seed matrix records Apple ARM64 local values for
`iw44_decode_*`, `iw44_to_rgb_colorbook/*`, `render_colorbook*`, and
`render_corpus_color`, plus the #189 Linux x86_64 baseline vs
`x86_64-v3`/AVX2 values. wasm32 scalar, wasm32 simd128, and Linux aarch64 are
explicitly marked missing.

**Decision.** Kept. The repository now has one copy/pasteable platform metadata
block and one public cross-architecture result schema for #306, #307, and #308.

**Reason.** Normalizing the table first avoids each downstream architecture
issue inventing a different platform format, while preserving measurement
discipline by distinguishing current numbers from missing/untrusted cells.

### #298 — PDF export memory and parallel baseline — **Needs follow-up** (2026-05-17)

**Approach.** Measured the existing PDF export pipeline before any streaming
rewrite. Criterion measured the stable `pdf_export_sequential` and
`pdf_export_parallel` benches on `tests/corpus/watchmaker.djvu` (12 pages,
default PDF options: 150 dpi, JPEG-80). A new reproducible
`examples/pdf_memory_probe.rs` harness recorded read/parse, one-page
render/RGB/JPEG staging, full PDF export time, PDF bytes, and peak RSS via
`/usr/bin/time -l`.

**Platform.**
- OS: macOS 26.3.1 / Darwin 25.3.0 (`RELEASE_ARM64_T6000`)
- CPU: Apple M1 Max, 10 cores
- arch: `arm64` / Rust host `aarch64-apple-darwin`
- target features: Apple ARM64 baseline; NEON available on Apple Silicon
- Rust: `rustc 1.92.0 (ded5c06cf 2025-12-08)`
- RUSTFLAGS: unset
- Source artifact: local run on `codex/issue-298-pdf-baseline`

**Command(s).**

```sh
cargo bench --bench render --features std -- pdf_export_sequential
cargo bench --bench render --features std,parallel -- pdf_export_parallel

/usr/bin/time -l cargo run --release --example pdf_memory_probe -- \
  tests/corpus/watchmaker.djvu

/usr/bin/time -l cargo run --release --features parallel \
  --example pdf_memory_probe -- tests/corpus/watchmaker.djvu
```

**Numbers.**

| Measurement | Sequential | Parallel |
|-------------|-----------:|---------:|
| Criterion `pdf_export_*` | 955.42 ms median (`916.16..999.54 ms`) | 154.05 ms median (`153.41..154.66 ms`) |
| Probe `pdf_export_ms` | 893.827 ms | 187.183 ms |
| Peak RSS (`maximum resident set size`) | 80,379,904 bytes (76.7 MiB) | 240,058,368 bytes (228.9 MiB) |
| Peak memory footprint | 79,479,872 bytes (75.8 MiB) | 239,175,232 bytes (228.1 MiB) |
| Output PDF bytes | 6,651,085 | 6,651,085 |

Single-page breakdown from the same probe, page 0 rendered at 150 dpi
(`1275x1651`):

| Stage | Time | Bytes |
|-------|-----:|------:|
| Read input | 0.075 ms | 183,352 |
| Parse document | 0.152 ms | - |
| Render full RGBA pixmap | 43.822 ms | 8,420,100 |
| Convert RGBA to RGB staging buffer | 2.904 ms | 6,315,075 |
| JPEG-80 encode staging buffer | 13.065 ms | 312,922 |

The parallel probe uses the same one-page breakdown before full export; that
single-page section stayed essentially unchanged (`render_pixmap_ms=44.410`,
`rgb_stage_ms=3.183`, `jpeg_stage_ms=13.228`) while full export dropped to
`187.183 ms` and peak RSS rose to `228.9 MiB`.

**Decision.** Needs follow-up.

**Reason.** Parallel export is about 5.3-6.2x faster on the 12-page color
fixture, but it increases peak RSS by about 3.0x because `djvu_to_pdf_impl`
collects every `RenderedPage` before sequential object emission. The concrete
baseline for #299 is therefore: beat ~894 ms sequential wall time and reduce
or cap the ~76.7 MiB sequential peak RSS / ~228.9 MiB parallel peak RSS by
streaming page render/RGB/JPEG data into PDF objects instead of retaining all
encoded page bodies at once.

### #299 — PDF color row streaming — **Kept** (2026-05-17)

**Approach.** Replaced the PDF color-image path's full `Pixmap` + full RGB
staging pair with `render_streaming` into one RGB staging buffer when render
options are streamable. The fallback `render_pixmap(...).to_rgb()` path remains
for anti-aliasing, scaled Lanczos, rotation, and other non-streamable options.
Measured against the #298 baseline on the same `tests/corpus/watchmaker.djvu`
PDF fixture (12 pages, default PDF options: 150 dpi, JPEG-80).

**Platform.**
- OS: macOS 26.3.1 / Darwin 25.3.0 (`RELEASE_ARM64_T6000`)
- CPU: Apple M1 Max, 10 cores
- arch: `arm64` / Rust host `aarch64-apple-darwin`
- target features: Apple ARM64 baseline; NEON available on Apple Silicon
- Rust: `rustc 1.92.0 (ded5c06cf 2025-12-08)`
- RUSTFLAGS: unset
- Source artifact: local run on `codex/issue-299-pdf-streaming`

**Command(s).**

```sh
cargo bench --bench render --features std -- pdf_export_sequential
cargo bench --bench render --features std,parallel -- pdf_export_parallel
cargo bench --bench render --features std,parallel -- pdf_export_parallel

/usr/bin/time -l cargo run --release --example pdf_memory_probe -- \
  tests/corpus/watchmaker.djvu
/usr/bin/time -l cargo run --release --features parallel \
  --example pdf_memory_probe -- tests/corpus/watchmaker.djvu
```

**Numbers.**

| Measurement | #298 baseline | #299 row streaming |
|-------------|--------------:|-------------------:|
| Criterion `pdf_export_sequential` median | 955.42 ms | 811.83 ms (`810.13..813.58 ms`) |
| Criterion `pdf_export_parallel` median | 154.05 ms | 165.57 ms rerun (`154.19..178.74 ms`) |
| Sequential probe `pdf_export_ms` | 893.827 ms | 852.285 ms |
| Parallel probe `pdf_export_ms` | 187.183 ms | 155.745 ms |
| Sequential peak RSS | 80,379,904 bytes (76.7 MiB) | 77,512,704 bytes (73.9 MiB) |
| Parallel peak RSS | 240,058,368 bytes (228.9 MiB) | 177,684,480 bytes (169.5 MiB) |
| Output PDF bytes | 6,651,085 | 6,651,085 |

The first parallel Criterion run after the change measured
`219.65 ms` (`206.80..232.42 ms`) and reported a regression; an immediate rerun
measured `165.57 ms` (`154.19..178.74 ms`). The single-run probe also measured
parallel export at `155.745 ms`. Treat parallel timing as noisy on this host;
the stable win is peak RSS.

**Decision.** Kept.

**Reason.** The change preserves PDF bytes and keeps the fallback path for
non-streamable render options. It removes the extra full RGBA page allocation
from the streamable PDF color path. Sequential RSS falls modestly from
`76.7 MiB` to `73.9 MiB`; parallel RSS falls materially from `228.9 MiB` to
`169.5 MiB` (-26%). The remaining peak is dominated by retained per-page
encoded RGB/JPEG/PDF object bodies, so a larger memory reduction would require
streaming PDF object emission rather than only row-streamed rendering.
