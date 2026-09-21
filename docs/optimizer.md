# Document optimizer

Issue: [#686](https://github.com/matyushkin/djvu-rs/issues/686)

The optimizer is intentionally conservative. The first vertical slice exposes
a typed request, a dry-run plan, an audit report, and an atomic CLI output path.
It removes only `FREE` IFF padding chunks. `FREE` has no decoded image or
document semantics, so the following content is retained byte-for-byte:

- image chunks and page order;
- text, annotations, metadata, bookmarks, and links;
- shared dictionaries, thumbnails, and unknown chunk IDs.

`OptimizationPreset::Archival` is accepted as a typed policy, but it currently
selects the same lossless cleanup. It does not silently invoke a lossy codec or
claim that a target size was reached. If `--target-size` cannot be met by
removing padding, the JSON plan/report sets `target_met` to `false` and names
the reason.

`--max-ssim-loss` is accepted for forward compatibility with archival
re-encode, but the current FREE-cleanup path is pixel-exact by construction and
does **not** measure SSIM. When the flag is set, the plan/report keeps
`quality_floor_met: true` and emits an explicit warning so callers cannot
mistake the threshold for an active gate.

The CLI always requires a separate `--output` path, rejects input/output path
aliasing, stages the result beside the destination, syncs it, and renames it
into place only after the optimizer succeeds. This keeps the input untouched
on a failed or interrupted write.

## Progress ([#814](https://github.com/matyushkin/djvu-rs/issues/814), slice 1)

`Optimizer::with_progress(hook)` installs a `Fn(&ProgressEvent) + Send + Sync`
hook. The optimizer calls it on the calling thread, once per component per
phase, after that component is handled:

| Phase | Reported by | Components | `bytes_so_far` |
|-------|-------------|------------|----------------|
| `plan` | `plan` and `optimize` | the root's children of a `DJVM`, or the single root `FORM` | encoded size of the components walked, headers included |
| `rewrite` | `optimize` | the components the plan rewrites; none when the plan is a pass-through | input payload of the components rewritten |
| `verify` | `optimize` | the components of the re-parsed output | as for `plan`, over the output |

Each `ProgressEvent` carries the phase, the zero-based `component_index`, the
`component_count` of its phase, the four-byte `component_id` (`DJVU`, `DJVI`,
`THUM`, `DIRM`, `NAVM`, `FREE`, …) and `bytes_so_far`. Within one phase the
index increases by one per event and `bytes_so_far` never decreases. The hook
lives on the `Optimizer`, not on `OptimizationRequest`, so the request stays a
plain comparable value.

The `verify` phase is new with the hook: `optimize` re-parses its own output
and fails with `OptimizeError::Verification` instead of returning bytes whose
page count differs from the input's.

`djvu optimize` shows a one-line progress indicator on stderr when stderr is a
terminal, and clears it before printing the JSON. A pipe or a redirect sees
neither the line nor its escape codes. Nothing in the JSON plan or report
changes for this slice.

## Cancellation ([#814](https://github.com/matyushkin/djvu-rs/issues/814), slice 2)

`Optimizer::with_cancel(hook)` installs a `Fn() -> bool + Send + Sync` hook,
the same cooperative contract as `ExportObserver::cancelled` for the export
writers. The optimizer polls it on the calling thread before parsing the input
and before each component of each phase. Once it returns `true`, `plan` and
`optimize` return `OptimizeError::Cancelled`. Work already begun on a
component completes first; no partial output is ever returned, and the
progress events reported before the stop are exactly those of the components
that were handled.

Rewrites are applied one component at a time in plan order, so a stop in the
`rewrite` phase lands on a component boundary. The `djvu` binary stages its
output only after `optimize` succeeds, so a cancelled run leaves no file
behind. The binary installs no signal handler of its own: an interrupt ends the
process before anything is staged.

The remaining roadmap is quality-aware archival re-encoding and target-size
search.
