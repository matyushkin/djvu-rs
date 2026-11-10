# OSS-Fuzz integration files

This directory contains the three files OSS-Fuzz expects under
`projects/<name>/` in [google/oss-fuzz](https://github.com/google/oss-fuzz):

* `project.yaml` — project metadata, contact e-mails, sanitizers, engines
* `Dockerfile`   — build environment (Rust nightly via `base-builder-rust`)
* `build.sh`     — invokes `compile_rust_fuzzer` for each target in `fuzz/`

## Status

**Not yet live.** djvu-rs is not fuzzed on OSS-Fuzz infrastructure yet.

* Submitted as [google/oss-fuzz#15331](https://github.com/google/oss-fuzz/pull/15331)
  on 2026-04-10. The PR is still open and has no maintainer review.
* Its `cla/google` check fails: the Google Contributor License Agreement is
  not signed for the commit author. OSS-Fuzz maintainers do not review a PR
  until the CLA check passes, and the trial build has not run.
* Tracking issue: [#193](https://github.com/matyushkin/djvu-rs/issues/193).
  The triage steps for OSS-Fuzz reports are already in
  [`CONTRIBUTING.md`](../CONTRIBUTING.md#triaging-an-oss-fuzz-finding).

Remaining steps:

1. Sign the CLA at <https://cla.developers.google.com/> with the commit
   author's e-mail, then re-run the check on #15331.
2. Sync `projects/djvu-rs/` in the PR with this directory. The submitted
   `build.sh` copies only the first five targets; the other six were added
   later.
3. After the PR merges and the first build is green: add the OSS-Fuzz badge
   to `README.md`, update the "Fuzzing" section of `SECURITY.md`, and close #193.

## Submitting

```sh
git clone https://github.com/google/oss-fuzz.git
cp -r oss-fuzz oss-fuzz-upstream/projects/djvu-rs
cd oss-fuzz-upstream
python infra/helper.py build_image djvu-rs
python infra/helper.py build_fuzzers --sanitizer address djvu-rs
python infra/helper.py run_fuzzer djvu-rs fuzz_full
# When the local checks above pass:
git checkout -b add-djvu-rs
git add projects/djvu-rs
git commit -m "Project djvu-rs: initial integration"
git push -u <fork> add-djvu-rs
gh pr create --repo google/oss-fuzz ...
```

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
`$OUT/<target>_seed_corpus.zip` for every target. OSS-Fuzz unpacks these on
the first run so coverage-guided fuzzing starts from realistic inputs
(malformed headers, truncated chunks, codec edge cases).

To extend a corpus, drop minimised reproducers into the relevant
`fuzz/corpus/<target>/` directory and commit them. Keep individual files
small (≤ 200 KB) and prefer synthetic / CC0-licensed inputs.
