# OSS-Fuzz integration files

This directory contains the three files OSS-Fuzz expects under
`projects/<name>/` in [google/oss-fuzz](https://github.com/google/oss-fuzz):

* `project.yaml` — project metadata, contact e-mails, sanitizers, engines
* `Dockerfile`   — build environment (Rust nightly via `base-builder-rust`)
* `build.sh`     — invokes `compile_rust_fuzzer` for each target in `fuzz/`

## Submitting

Tracking issue: [#193](https://github.com/matyushkin/djvu-rs/issues/193).

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

Mirrors `fuzz/fuzz_targets/`:

| Binary       | Coverage                                    |
| ------------ | ------------------------------------------- |
| `fuzz_full`  | `DjVuDocument::parse` + render            |
| `fuzz_iff`   | IFF/AT&T container parser                  |
| `fuzz_jb2`   | JB2 bilevel decoder                        |
| `fuzz_bzz`   | BZZ decompressor                           |
| `fuzz_iw44`  | IW44 wavelet decoder                       |

A seed corpus is **not** shipped here — it would re-pull `tests/corpus/*.djvu`
into every Docker build. Follow-up on #193: ship a small `<target>_seed_corpus.zip`
alongside each target so OSS-Fuzz starts with realistic inputs.
