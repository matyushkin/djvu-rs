# OSS-Fuzz integration files

This directory contains the three files OSS-Fuzz expects under
`projects/<name>/` in [google/oss-fuzz](https://github.com/google/oss-fuzz):

* `project.yaml` — project metadata, contact e-mails, sanitizers, engines
* `Dockerfile`   — build environment (Rust nightly via `base-builder-rust`)
* `build.sh`     — invokes `compile_rust_fuzzer` for each target in `fuzz/`

## Status

**Not yet live.** djvu-rs is not fuzzed on OSS-Fuzz infrastructure yet.

* Submitted as [google/oss-fuzz#15331](https://github.com/google/oss-fuzz/pull/15331)
  on 2026-04-10. The PR is open and waits for a maintainer review.
* Since 2026-11-15 the PR checks pass: `cla/google`, `header-check` and
  `check-changes`. The trial build has not run yet.
* The PR is one commit. Its `build.sh` builds all eleven targets.
* The local container build of that commit passed on 2026-11-15:
  `build_image`, `build_fuzzers --sanitizer address` and `check_build`.
* Tracking issue: [#988](https://github.com/matyushkin/djvu-rs/issues/988).
  The triage steps for OSS-Fuzz reports are already in
  [`CONTRIBUTING.md`](../CONTRIBUTING.md#triaging-an-oss-fuzz-finding).

Remaining steps:

1. Wait for the maintainer review and the merge of #15331.
2. After the first green build: add the OSS-Fuzz badge to `README.md`, update
   the "Fuzzing" section of `SECURITY.md`, and close #988.

## Differences from the upstream PR

The copy in `projects/djvu-rs/` of the PR is what OSS-Fuzz builds. This
directory differs from it in these points:

* The upstream `build.sh` and `Dockerfile` start with the Apache-2.0 license
  header. The OSS-Fuzz `header-check` requires it. The commands are the same.
* `project.yaml` here also sets `auto_ccs` and `file_github_issue: true`. The
  PR does not set them.

Rules for a change to the PR branch (`add-djvu-rs` in the fork):

* Do not add a `Co-Authored-By` trailer for an address without a signed Google
  CLA. The `cla/google` check counts each co-author as a contributor and fails.
* Keep `fuzzing_engines` at `libfuzzer`. With the current `build.sh`, a build
  with `--engine afl` fails at link time (`undefined reference to
  __afl_manual_init`, checked 2026-11-16).

## Building locally

Build from the PR branch until the PR merges:

```sh
git clone --branch add-djvu-rs https://github.com/matyushkin/oss-fuzz.git
cd oss-fuzz
python infra/helper.py build_image djvu-rs
python infra/helper.py build_fuzzers --sanitizer address djvu-rs
python infra/helper.py check_build djvu-rs
python infra/helper.py run_fuzzer djvu-rs fuzz_full
```

The `Dockerfile` clones `main` of djvu-rs from GitHub. The build tests the
pushed `main`, not the local working tree.

## Targets

Mirrors `fuzz/fuzz_targets/` (`build.sh` copies every target listed here):

| Binary            | Coverage                                    |
| ----------------- | ------------------------------------------- |
| `fuzz_full`       | `DjVuDocument::parse` + render              |
| `fuzz_iff`        | IFF/AT&T container parser                   |
| `fuzz_jb2`        | JB2 bilevel decoder                         |
| `fuzz_bzz`        | BZZ decompressor                            |
| `fuzz_iw44`       | IW44 wavelet decoder                        |
| `fuzz_encode`     | JB2 / MMR / IW44 encoders, round-trip       |
| `fuzz_bzz_encode` | BZZ compressor, round-trip                  |
| `fuzz_g4`         | G4 (T.6) encoder, round-trip                |
| `fuzz_metadata`   | TXTz / NAVM / ANTz encode → decode          |
| `fuzz_graph`      | component graph (DIRM / INCL) traversal     |
| `fuzz_validate`   | validator and semantic diff                 |

## Seed corpora

`build.sh` zips `fuzz/corpus/<target>/` from the source tree into
`$OUT/<target>_seed_corpus.zip` for every target that has committed seeds.
`fuzz_graph` and `fuzz_validate` have none yet. OSS-Fuzz unpacks the zips on
the first run so coverage-guided fuzzing starts from realistic inputs
(malformed headers, truncated chunks, codec edge cases).

To extend a corpus, drop minimised reproducers into the relevant
`fuzz/corpus/<target>/` directory and commit them. Keep individual files
small (≤ 200 KB) and prefer synthetic / CC0-licensed inputs. `fuzz/corpus/`
is in `.gitignore`: add a new seed with `git add -f`.
