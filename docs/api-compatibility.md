# API compatibility, stability, and resource-limit policy

This document is the checked-in, enforceable contract for consumers of the
`djvu-rs` crate and its workspace codec crates (`djvu-iff`, `djvu-bzz`,
`djvu-jb2`, `djvu-iw44`, `djvu-zp`, `djvu-bitmap`, `djvu-pixmap`). It defines:

- what "stable", "experimental", and "deprecated" mean for this project;
- the semantic-versioning and deprecation-window rules;
- the minimum supported Rust version (MSRV) policy;
- the supported feature combinations and build targets
  (see also [`feature-matrix.md`](feature-matrix.md));
- the `Send`/`Sync` and thread-safety expectations of the public types;
- the panic-free contract for untrusted input;
- the resource-limit (memory / work / decompression) contract and how limit
  failures surface as typed errors.

It is enforced in CI. See [Enforcement](#enforcement) for the exact gates.

> **Versions.** From 1.0.0 the crate follows Semantic Versioning as written:
> only a major release breaks the stable surface. The `0.x` releases used
> Cargo's rule for `0.MINOR.PATCH`, where the minor number was the breaking
> axis; [`docs/migration.md`](migration.md) lists what each of them broke.
> The Python and npm packages carry the crate's version and follow the same
> rule for their own API, deprecation window included.

## 1. Surface tiers

Every public item belongs to exactly one tier. The tier is declared in the
item's rustdoc and, where a whole area is involved, gated behind a Cargo
feature whose name signals the tier.

### Stable

The default surface. Stable items follow SemVer: a breaking change to their
signature, behavior, or error contract requires a major release. This
includes:

- `Document`, `Page`, and the crate-root re-exports of the document model
  (`DjVuDocument`, `DjVuPage`, `DjVuBookmark`, `PageInfo`, `Rotation`,
  `ComponentId`, `ComponentKind`, …).
- The error hierarchy (`DjVuError`, `IffError`, `Jb2Error`, `Iw44Error`,
  `BzzError`, `DocError`, `RenderError`) — see the error-stability rule below.
- The rendering entry points in [`djvu_render`] (`RenderOptions`,
  `RenderRequest`, `render_pixmap`, …). `RenderRequest` is the one way to
  ask for a region, a progressive step, a coarse preview, a row stream or a
  render into a caller's buffer; the older `render_*` variants were removed
  in 1.0.0 (see [`docs/migration.md`](migration.md#100)).
- The codec entry points of the workspace crates that the README advertises as
  `no_std`-callable: `iff::parse_form`, `bzz::bzz_decode`, `jb2::decode_dict`,
  `iw44::Iw44Image::decode_chunk`.
- The text / annotation / metadata parsers and their data models.
- The writer surfaces behind their feature gates (`pdf`, `epub`, `cbz`,
  `tiff`): once a `djvu_to_*` entry point ships, its signature is stable.

**Error stability rule.** Every public error *enum* carries
`#[non_exhaustive]`: consumers must not rely on the absence of variants, and
adding a new variant is a compatible change. Renaming or removing an existing
variant, or changing the data it carries, is breaking. Matching on a stable
variant and reading its documented fields is supported; a `match` on an error
enum must include a `_` arm.

This was a rule "in spirit" until 0.32.0, enforced only by review. It is now
literal, which is what lets the `Public API breakage` CI gate be a hard failure
instead of an advisory one: before, every intended new variant reported as
breakage, so the gate had to run with `continue-on-error` and could not block a
real, unintended break either. Any new public error enum must be marked the same
way.

**Extensible-type rule.** A type that is expected to gain members is marked
`#[non_exhaustive]`, so that adding a member is a compatible change:

- **Options structs** (`RenderOptions`, `PdfOptions`, `ResourceLimits`,
  `Jb2EncodeOptions`, `Iw44EncodeOptions`, …). Each implements `Default` and
  has one `with_<field>` method per public field; `RenderOptions::new(width,
  height)` sets the two fields every render needs. Consumers build them with
  those methods, not with a struct literal. Reading a public field and
  assigning to one stay supported. A new field arrives with a default that
  keeps the earlier behavior, and with its `with_*` method.
- **Report structs** — values the crate returns (`RenderReport`,
  `ValidationReport`, `Finding`, `QualityReport`, `ComponentNode`,
  `SegmentedPage`, `ChunkRecord`, …). Consumers read their fields; they do
  not construct them.
- **Options and classification enums** (`Resampling`, `EncodeQuality`,
  `Iw44Target`, `Severity`, `EditOperation`, …). A `match` on one of them
  must include a `_` arm.

Adding a field or a variant to such a type is compatible. Removing or
renaming one, or changing its type, is breaking.

Plain data types are **not** extensible and stay constructible by literal:
geometry (`Rect`, `RenderRect`, `TileRect`), pixels (`Pixmap`, `GrayPixmap`,
`Bitmap`, `Color`, `FgbzColor`), and the document data models (`PageInfo`,
`TextLayer`, `TextZone`, `DjVuBookmark`, `DjVuMetadata`, `EditRequest`,
`ComponentId`, …). Adding a public field to one of them is breaking. So is
adding a variant to an enum that the format or geometry closes
(`TextZoneKind`, `Rotation`, `UserRotation`, `annotation::Shape`,
`ComponentKind`, `PlaneStatus`, `iff::Chunk`).

[`docs/migration.md`](migration.md#100) lists every type in each group as
of 1.0.0. A new public options struct, report struct or options enum must be
marked the same way.

### Experimental

Opt-in, may change or be removed in any release **including a patch release**,
and is excluded from the API-breakage gate. Experimental status is signalled
two ways, both of which must be present:

- the rustdoc opens with a bolded `**Experimental:**` note explaining the
  instability, and
- the surface is reachable only behind an experimental feature flag.

Current experimental features: `experimental`, `iw44-probe`, `alloc-profile`,
`ocr-onnx`, `wasm-threads`. These are **out of scope** for API freezing per
issue #695 and remain free to change. The 1.0.0 release confirmed this set
(#993): the 1.0 stability promise does not cover them.

### Deprecated

Still stable and still shipped, but scheduled for removal. A deprecated item:

- carries `#[deprecated(since = "x.y.z", note = "use … instead")]` on code
  items, or an explicit "Deprecated" marker in its rustdoc / the feature table
  for feature-level aliases;
- keeps working, unchanged, for the **deprecation window** below;
- names its replacement.

**Deprecation window.** A deprecated item remains available for at least **two
minor releases** after the release that introduced the deprecation, and never
less than **90 days**, whichever is longer. Removal happens only in a
major release.

Current deprecated surfaces: none. 1.0.0 removed every deprecated item,
including the module aliases `bzz_new` and `iw44_new` (listed here since
0.28.0); see [`docs/migration.md`](migration.md#100).

## 2. Semantic versioning

- **Patch** (`x.y.Z`): bug fixes, performance work, documentation.
- **Minor** (`x.Y.z`): new **stable** APIs that are purely additive, new
  error variants, new fields and variants on the extensible types of §1, new
  features that default to off, new deprecation notes, MSRV raises (§3).
- **Major** (`X.y.z`): removals of deprecated items after their window,
  signature/behavior changes to stable items.
- Experimental surfaces are exempt: they may break in any release.

Unintended breakage of the **stable** surface is caught by
[`cargo-semver-checks`](#enforcement) in CI, which compares each change against
its parent (a PR against its base commit, a push to `main` against the previous
tip). A break declared by an earlier
PR is therefore not reported again before the next release. An
intended break is declared with a `!` in the PR title (`feat(scope)!: …`) or
a `BREAKING CHANGE:` footer, the same marker release-please reads; the gate
then checks the PR as the major release it will produce and
still fails on any break the PR does not declare. The version itself is never
bumped by hand.

A PR that declares a break also adds a section to
[`docs/migration.md`](migration.md), under the release it will ship in, with
the code before and after. `tests/migration_sync.rs` fails when
`CHANGELOG.md` lists breaking changes for a release that has no section
there, so the release PR stays red until the section exists.

### Intentional breaks, by release

#### 1.0.0 — the deprecated surface is removed and the growing types are marked

Two changes, both described with code in
[`docs/migration.md`](migration.md#100):

- Every item with a deprecation note is removed: the `render_*` variants
  that `RenderRequest` replaces, `RenderOptions::scale`, `Pixmap::new`,
  `OcrExportError`, the old render methods of the Python and npm packages,
  and the module aliases `bzz_new` and `iw44_new`.
- The options structs, the report structs and the options enums become
  `#[non_exhaustive]` (the extensible-type rule in §1). This is the same
  step 0.34.0 took for the optimizer types, made once for the whole stable
  surface, so that 1.x releases can add options and report fields without a
  major release. `Jb2EncodeOptions::with_lossy_threshold` and
  `with_despeckle` change from constructors to methods on a value.

#### 0.34.0 — the optimizer's request and report types became extensible

The archival preset now re-encodes page backgrounds and, on request, masks
under a measured SSIM floor (#814, slice 3; `docs/optimizer.md`). That needs
a new rewrite action, a measured-quality record on each rewritten component,
a `min_ssim` summary and a `lossy_text` knob. Adding any of those to an
exhaustive enum or a struct with all-public fields is a break, so the types
are marked `#[non_exhaustive]` once, now, and later slices (target-size
search) add to them without another one:

| Item | Was | Is |
|------|-----|-----|
| [`optimizer::OptimizationRequest`](../src/optimizer.rs) | constructible struct literal | `#[non_exhaustive]`; build with `new`/`lossless_cleanup`/`archival` and `with_*`; new field `lossy_text` |
| [`optimizer::RewriteAction`](../src/optimizer.rs) | exhaustive enum, one variant | `#[non_exhaustive]`; new variants `ReencodeBackground`, `ReencodeMask` |
| [`optimizer::RewrittenComponent`](../src/optimizer.rs) | constructible, `Eq` | `#[non_exhaustive]`, `PartialEq` only; new field `quality: Option<ComponentQuality>` |
| [`optimizer::OptimizationPlan`](../src/optimizer.rs) | constructible | `#[non_exhaustive]`; new field `min_ssim: Option<f64>` |
| [`optimizer::OptimizationReport`](../src/optimizer.rs) | constructible | `#[non_exhaustive]`; new field `min_ssim: Option<f64>` |

Reading fields is unchanged. A `match` on `RewriteAction` needs a wildcard
arm; code that built any of these structs by literal switches to the
constructors. The JSON plan and report gain `min_ssim` and a per-component
`quality` object; existing keys keep their meaning.

#### 0.33.0 — the render caches became self-bounding

Rendering memoises what it decoded. Before 0.33 nothing ever gave that memory
back on its own: every eviction entry point wanted `&mut`, and every render
entry point holds a shared `&DjVuPage`, so a program that only rendered grew
until it ran out. See `PERF_EXPERIMENTS.md` (`READ_CACHE_BOUNDED`).

Letting a cache drop a layer while a render is in flight means the render must
hold its own handle on that layer, not a borrow of the cache's copy. That is
the break:

| Item | Was | Is |
|------|-----|-----|
| [`DjVuPage::decoded_bg44`](../src/djvu_document/page.rs) | `Option<&Iw44Image>` | `Option<Arc<Iw44Image>>` |
| [`DjVuPage::decoded_bg44_partial`](../src/djvu_document/page.rs) | `Option<&Iw44Image>` | `Option<Arc<Iw44Image>>` |
| [`DjVuPage::decoded_mask`](../src/djvu_document/page.rs) | `Option<&Bitmap>` | `Option<Arc<Bitmap>>` |
| [`DjVuPage::decoded_fg44`](../src/djvu_document/page.rs) | `Option<&Iw44Image>` | `Option<Arc<Iw44Image>>` |

`Arc<T>` derefs to `T`, so most call sites need no change; a site that stored
the returned reference now stores an owned handle instead, which is what makes
it safe.

The eviction methods now take a shared borrow, which only ever admits more
callers — existing `&mut` call sites keep compiling:

| Item | Was | Is |
|------|-----|-----|
| `DjVuPage::evict_render_cache` | `&mut self` | `&self` |
| `DjVuPage::downgrade_render_cache` | `&mut self` | `&self` |
| `DjVuDocument::retain_render_caches` | `&mut self` | `&self` |
| `DjVuDocument::enforce_cache_budget` | `&mut self` | `&self` |
| `DjVuDocument::enforce_cache_budget_with` | `&mut self` | `&self` |
| `DjVuDocument::downgrade_render_caches` | `&mut self` | `&self` |

Behaviour also changed: the render caches are now bounded by default. See §7.

## 3. Minimum supported Rust version (MSRV)

- MSRV is **Rust 1.89** (edition 2024; the `wide` 1.x SIMD dependency needs
  1.89). It is declared in `rust-version` in every workspace `Cargo.toml`.
- Raising the MSRV is a **minor** release, never a patch release, and is
  called out in the changelog.
- MSRV is a **required** CI gate (the `MSRV` job builds the crate on the
  pinned toolchain). A PR that uses a newer-than-MSRV language or std feature
  fails that gate.
- One exception: the experimental `ocr-onnx` feature needs Rust **1.91**, the
  MSRV of its `tract-onnx` 0.23 dependency. The MSRV job builds the default
  features, so it does not cover `ocr-onnx`.

## 4. Feature combinations and targets

The supported combinations and build targets, and the CI jobs that keep them
green, are enumerated in [`feature-matrix.md`](feature-matrix.md). The load-
bearing invariants:

- **Decode-only default tree.** The default (`std`) build must not pull in any
  writer/encoder dependency (zip, zopfli, jpeg-encoder, clap, …). Enforced by
  `scripts/check_feature_hygiene.sh` in the required `Lint` gate (#509).
- **`no_std` + `alloc`.** `--no-default-features` must build on the host and on
  `wasm32-unknown-unknown`, and the four codec entry points must be callable
  from a `#![no_std]` consumer (`tests/no_std_smoke`). Required via the
  `Test (stable)` and `wasm32 build check` gates.
- **wasm.** `wasm`, `wasm-lazy`, and `+simd128` all build for
  `wasm32-unknown-unknown`. Required via the `wasm32 build check` gate.
- **Additivity.** Enabling any documented feature must not break another
  feature that already built. Any documented combination must compile.

`wasm-threads` is the one documented target deliberately **outside** the
required gate: it needs a nightly toolchain and `-Z build-std`, so it is checked
by the opt-in `make wasm-threads-check` only.

## 5. `Send` / `Sync` and thread-safety

The public types' thread-safety is part of the contract and asserted at compile
time in [`tests/send_sync_contract.rs`](../tests/send_sync_contract.rs); a
regression there is a breaking change and fails CI.

| Type | `Send` | `Sync` | Notes |
|------|:------:|:------:|-------|
| `Document` | ✓ | ✓ | Owns an `Arc<dyn AsRef<[u8]> + Send + Sync>` backing; safe to share across threads. |
| `DjVuDocument` | ✓ | ✓ | The document model; shared-dictionary caches use `Arc<SharedDict>` with interior sync. |
| `Page<'a>` / `DjVuPage` | ✓ | ✓ | Borrow of the parent document; carries no interior mutability observable to callers. |
| `DjVuDocumentMut` / `PageMut<'doc>` | ✓ | — by borrow | The mutable editor is `Send`; a `PageMut` borrows `&mut`, so exclusivity (not `Sync`) is how concurrent edits are prevented. |
| `Pixmap` / `GrayPixmap` / `Bitmap` | ✓ | ✓ | Plain owned pixel buffers. |
| `RenderOptions`, `TextLayer`, `Annotation`, metadata types | ✓ | ✓ | Plain data. |
| `LazyDocument<R>` (async) | ✓/✓ **iff** `R: Send`/`Sync` | conditional | Thread-safety is inherited from the caller-supplied reader `R`. |

**Async guidance.** CPU-bound IW44/JB2 decode and render is synchronous and
must run on a blocking pool (`tokio::task::spawn_blocking`), not on a runtime
thread. `Document`/`DjVuDocument` being `Send + Sync` means a single parsed
document can be shared (e.g. behind an `Arc`) across render tasks.

## 6. Panic-free contract for untrusted input

**Contract.** No public parse, decode, or render entry point may panic, abort,
or overflow on *any* byte input, trusted or not. Malformed input must surface as
a typed `Err`, never an unwind. This covers `DjVuDocument::parse` /
`parse_from_dir`, `Document::open` / `from_bytes` / `from_reader`, every
`DjVuPage` accessor (`thumbnail`, `text_layer`, `annotations`, `extract_mask`),
every `djvu_render::*` entry point, and the standalone codec entry points.

**Tests.** The contract is exercised by:

- [`tests/panic_free_corpus.rs`](../tests/panic_free_corpus.rs) — runs every
  public decode/render entry point across the whole corpus and a set of
  adversarial byte patterns on every PR; any unwind fails the run.
- `tests/proptest_codecs.rs` — property tests over the codec surface.
- `fuzz/fuzz_targets/*` (libFuzzer, run by the `fuzz.yml` workflow) —
  adversarial coverage of IFF/BZZ/JB2/IW44/G4, encode, metadata, validate,
  and the full parse→render pipeline (`fuzz_full`). djvu-rs is not on
  OSS-Fuzz: the submission was declined on 2026-11-16
  ([`oss-fuzz/README.md`](../oss-fuzz/README.md)).

The FFI/WASM boundaries additionally catch unwinds and convert them to error
codes, so a hostile file can never unwind across the language boundary.

## 7. Resource-limit contract

Public decode/render operations **inherit** documented, bounded resource
ceilings: every allocation or loop whose size is driven by untrusted bytes is
capped against an explicit constant or the input length, so a crafted file
costs only bounded memory and work before erroring out. The authoritative table
of ceilings lives in [`../SECURITY.md`](../SECURITY.md#decode-time-resource-ceilings)
(the #589 inventory) and is summarized here:

| Axis | Bound | Constant |
|------|-------|----------|
| JB2 symbol pixels (per symbol / per stream) | 16 MP / 256 MP | `MAX_SYMBOL_PIXELS` / `MAX_TOTAL_SYMBOL_PIXELS` |
| JB2 page / blit pixels | 16 MP / 256 MP | `MAX_PAGE_SYMBOL_PIXELS` / `MAX_TOTAL_BLIT_PIXELS` |
| JB2 record count | 65 536 | `MAX_RECORDS` |
| IW44 declared image | 64 MP | checked in `decode_chunk` |
| BZZ block / total output | 4 MiB / 256 MB | `MAX_BLOCK_SIZE` / `MAX_OUTPUT_SIZE` |
| IFF / FORM nesting depth | 64 | `MAX_IFF_DEPTH` |
| NAVM bookmark depth | 256 | `MAX_NAVM_DEPTH` |
| S-expression (ANTz) depth | 64 | `MAX_SEXPR_DEPTH` |
| Text-zone (TXTz) depth | 64 | `MAX_ZONE_DEPTH` |
| Pixmap pixels | 64 MP | `MAX_PIXELS` |
| Render output pixels | 512 MP | [`DEFAULT_MAX_RENDER_PIXELS`](../../src/validate.rs) |

**Configurable budgets.** Callers may supply a [`ResourceLimits`](../../src/validate.rs)
budget via [`ParseOptions::limits`](../../src/validate.rs) on
[`DjVuDocument::parse_with_options`](../../src/djvu_document/document.rs) /
[`Document::from_bytes_with_options`](../../src/lib.rs), or via
[`RenderRequest::limits`](../../src/djvu_render/request.rs) on a render.
Unset fields mean “no limit on this axis”. When render calls omit an explicit
override, the document inherits limits stored at parse time. When both are
unset, render output inherits [`DEFAULT_MAX_RENDER_PIXELS`]. Use
[`ResourceLimits::inherited`] for the documented default render ceiling only.
Per-render tightening uses [`RenderRequest::limits`](../../src/djvu_render/request.rs).
The validator and `djvu validate --limits` use the same type.

**Render caches are bounded by default (since 0.33).** The decode results a
render memoises are held against a process-wide ceiling,
[`render_cache::DEFAULT_BUDGET`](../src/render_cache.rs) = 256 MiB. When a
cache fill takes the total over the ceiling, the least-recently-used cached
*layers* — a decoded background, a mask, a converted pixmap, a page's tile
store — are dropped across all pages until it is under again (#813;
0.33 dropped whole page caches). Only the layer being filled is never dropped,
so the resident total can exceed the ceiling by at most one layer, and a
page's recently used layers survive while its stale ones go. This axis is a
**policy** ceiling, not a decode ceiling: crossing it frees memory, it never
fails a render.

| Axis | Bound | Constant |
|------|-------|----------|
| Resident page render caches (process-wide) | 256 MiB | [`render_cache::DEFAULT_BUDGET`](../src/render_cache.rs) |

Set your own ceiling with [`render_cache::set_budget`](../src/render_cache.rs),
read the current total with `render_cache::resident_bytes`, and sweep on demand
with `render_cache::enforce` or `render_cache::clear`. Pass `usize::MAX` to
`set_budget` to render without a ceiling — the behaviour of 0.32 and earlier.
Per-document control stays available through `DjVuDocument::enforce_cache_budget`.
The module is `std`-only; a `no_std` build memoises nothing across pages.

**Limit failures are typed and identify the operation.** When a ceiling is hit,
the failing entry point returns a typed error naming the axis and operation:

| Limit | Typed error | Wrapped as |
|-------|-------------|------------|
| IFF nesting too deep | `IffError::DepthLimitExceeded { max }` | `DjVuError::Iff` |
| IFF chunk longer than its container | `IffError::ChunkTooLong { id, claimed, available }` | `DjVuError::Iff` |
| JB2 image / symbol / dict too large | `Jb2Error::ImageTooLarge` / `InheritedDictTooLarge` | `DjVuError::Jb2` |
| JB2 too many records | `Jb2Error::TooManyRecords` | `DjVuError::Jb2` |
| IW44 image too large | `Iw44Error::ImageTooLarge` | `DjVuError::Iw44` |
| BZZ block / output too large | `BzzError::BlockSizeTooLarge(_)` / `OutputTooLarge` | `DjVuError::Bzz` |
| Configured document/render budget exceeded | [`ResourceLimitExceeded`](../../src/validate.rs) | [`DocError::ResourceLimit`](../../src/djvu_document/error.rs) / [`RenderError::ResourceLimit`](../../src/djvu_render/options.rs) |

The `DjVuError` variant (`Iff` / `Jb2` / `Iw44` / `Bzz`) identifies which
decode stage rejected the input; the inner variant identifies the axis.
[`ResourceLimitExceeded::operation`] names the public entry point (for example
`document.parse` or `render_pixmap`). `IffError::ChunkTooLong` additionally
reports the offending chunk `id`.

## 8. Experimental-surface consistency (README ↔ docs.rs)

The set of experimental/placeholder/deprecated surfaces is stated identically
in three places, and they must not drift:

- this document (§1),
- the **Feature flags** and **Status & limitations** tables in the README, and
- the rustdoc of each item (rendered on docs.rs, built with `--all-features` and
  `--cfg docsrs`).

## Enforcement

| Guarantee (AC) | Gate |
|----------------|------|
| Stable API breakage detection | `cargo-semver-checks` (`.github/workflows/api-stability.yml`) |
| Feature combinations / targets | feature-matrix job (`api-stability.yml`) + `scripts/check_feature_hygiene.sh` (`Lint`) |
| MSRV | `MSRV` job (required) |
| `no_std` builds | `Test (stable)` `Build (no_std check)` + `wasm32 build check` (both required) |
| Panic-free contract | `tests/panic_free_corpus.rs`, proptests, `fuzz.yml` (libFuzzer) |
| `Send`/`Sync` contract | `tests/send_sync_contract.rs` |
| Resource limits | codec unit tests + fuzz; ceilings documented here and in `SECURITY.md` |

Run the deterministic subset locally with `make check` before pushing.
