#!/usr/bin/env bash
# Build check for the opt-in `wasm-threads` feature (rayon thread pool inside
# wasm32, via `wasm-bindgen-rayon`). NOT part of `scripts/check.sh` / the
# required CI gates: it needs a nightly toolchain + `-Z build-std`, which the
# stable `wasm32 build check` gate (and the default `wasm` feature it covers)
# must never depend on. See WASM_THREADS in PERF_EXPERIMENTS.md.
#
# Requirements (one-time):
#   rustup toolchain install nightly
#   rustup component add rust-src --toolchain nightly
#   rustup target add wasm32-unknown-unknown --toolchain nightly
#
# Usage:
#   scripts/wasm_threads_check.sh            # cargo check only (fast)
#   scripts/wasm_threads_check.sh --build     # full release build + link (slower)
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

# `+atomics,+bulk-memory` enable wasm threads codegen; the `--shared-memory` /
# `--import-memory` / `--max-memory` / `__tls_*` link-args are required so the
# linked module's memory is an actually-`shared` WebAssembly.Memory (otherwise
# `postMessage`-ing it to a Worker throws `DataCloneError` at runtime — see the
# WASM_THREADS journal entry for how that was diagnosed).
export RUSTFLAGS="-C target-feature=+atomics,+bulk-memory \
-C link-arg=--shared-memory -C link-arg=--max-memory=1073741824 -C link-arg=--import-memory \
-C link-arg=--export=__wasm_init_tls -C link-arg=--export=__tls_size \
-C link-arg=--export=__tls_align -C link-arg=--export=__tls_base"

cmd=(cargo)
[ -n "${RUSTUP_TOOLCHAIN:-}" ] || cmd=(rustup run nightly cargo)

if [ "${1:-}" = "--build" ]; then
  run() { printf '\n==> %s\n' "$*"; "$@"; }
  run "${cmd[@]}" build -Z build-std=panic_abort,std --target wasm32-unknown-unknown \
    --features wasm-threads --release
else
  run() { printf '\n==> %s\n' "$*"; "$@"; }
  run "${cmd[@]}" check -Z build-std=panic_abort,std --target wasm32-unknown-unknown \
    --features wasm-threads
fi

printf '\n\xe2\x9c\x93 wasm-threads build OK (nightly + build-std — NOT a required CI gate)\n'
