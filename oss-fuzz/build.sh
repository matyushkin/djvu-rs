#!/bin/bash -eu
# Build script invoked by OSS-Fuzz infrastructure.
#
# `compile_rust_fuzzer` is provided by gcr.io/oss-fuzz-base/base-builder-rust;
# it wraps `cargo fuzz build` with the OSS-Fuzz-mandated flags and copies the
# resulting binary into $OUT/.

cd $SRC/djvu-rs

# Each target lives at fuzz/fuzz_targets/<name>.rs and is registered in
# fuzz/Cargo.toml. compile_rust_fuzzer args:  src-dir  target-name  out-name.
compile_rust_fuzzer fuzz fuzz_full  fuzz_full
compile_rust_fuzzer fuzz fuzz_iff   fuzz_iff
compile_rust_fuzzer fuzz fuzz_jb2   fuzz_jb2
compile_rust_fuzzer fuzz fuzz_bzz   fuzz_bzz
compile_rust_fuzzer fuzz fuzz_iw44  fuzz_iw44

# Seed corpora — OSS-Fuzz convention: $OUT/<target>_seed_corpus.zip is
# unpacked into the per-target corpus dir on the first run. We ship the
# in-tree fuzz/corpus/<target>/ contents which are all small (< 200 KB
# each, < 1 MB total) and CC0/synthetic.
for target in fuzz_full fuzz_iff fuzz_jb2 fuzz_bzz fuzz_iw44; do
    if [ -d "fuzz/corpus/$target" ] && [ -n "$(ls -A fuzz/corpus/$target 2>/dev/null)" ]; then
        (cd "fuzz/corpus/$target" && zip -qr "$OUT/${target}_seed_corpus.zip" .)
    fi
done
