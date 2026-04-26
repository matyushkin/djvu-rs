#!/bin/bash -eu
# Build script invoked by OSS-Fuzz infrastructure.
#
# `compile_rust_fuzzer` is provided by gcr.io/oss-fuzz-base/base-builder-rust;
# it wraps `cargo fuzz build` with the OSS-Fuzz-mandated flags and copies the
# resulting binary into $OUT/.

cd $SRC/djvu-rs

# Each target lives at fuzz/fuzz_targets/<name>.rs and is registered in
# fuzz/Cargo.toml. compile_rust_fuzzer args:  src-dir  target-name  out-name.
# When a starter corpus exists alongside the target it should be passed too;
# we don't ship one yet (issue #193 follow-up: seed from tests/corpus/).
compile_rust_fuzzer fuzz fuzz_full  fuzz_full
compile_rust_fuzzer fuzz fuzz_iff   fuzz_iff
compile_rust_fuzzer fuzz fuzz_jb2   fuzz_jb2
compile_rust_fuzzer fuzz fuzz_bzz   fuzz_bzz
compile_rust_fuzzer fuzz fuzz_iw44  fuzz_iw44
