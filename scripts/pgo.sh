#!/usr/bin/env bash
# Profile-Guided Optimization (PGO) build.
#
# Layers PGO on top of the existing fat-LTO release profile (LTO_FAT). A cold
# end-to-end render measured ~15% faster with PGO (see PERF_EXPERIMENTS.md,
# "PGO"); the isolated SIMD codec kernels are unaffected (already LTO-inlined).
#
# PGO is intentionally opt-in, NOT the default build: it needs a two-phase build
# plus the training corpus, and the resulting `.profdata` is corpus/host-specific
# (so it cannot ship with a crates.io release). Use it for local/self-hosted
# release builds whose workload resembles document rendering.
#
# Flow:
#   1. Build the `pgo_train` driver instrumented (-Cprofile-generate).
#   2. Run it (it decodes/renders a spread of the corpus) to emit raw profiles.
#   3. Merge them with llvm-profdata into target/pgo.profdata.
#   4. Rebuild the requested target with -Cprofile-use.
#
# Usage:
#   scripts/pgo.sh                 # PGO-build the `djvu` CLI (features: cli)
#   scripts/pgo.sh --features std --bench codecs   # PGO-build something else
#   RUSTFLAGS from step 4 are printed so you can reuse the profile for any build.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

PROFDIR="$PWD/target/pgo-data"
PROFDATA="$PWD/target/pgo.profdata"

# Locate llvm-profdata from the active toolchain (llvm-tools-preview component).
SYSROOT="$(rustc --print sysroot)"
LLVM_PROFDATA="$(find "$SYSROOT" -name 'llvm-profdata' -type f 2>/dev/null | head -n1 || true)"
if [[ -z "$LLVM_PROFDATA" ]]; then
  echo "!! llvm-profdata not found — run: rustup component add llvm-tools-preview" >&2
  exit 1
fi

run() { printf '\n==> %s\n' "$*"; "$@"; }

echo "==> PGO step 1/4: instrumented build of the training driver"
rm -rf "$PROFDIR" "$PROFDATA"
RUSTFLAGS="-Cprofile-generate=$PROFDIR" \
  cargo build --release --example pgo_train --features std

echo "==> PGO step 2/4: run the training driver (3x) to collect profiles"
for i in 1 2 3; do
  LLVM_PROFILE_FILE="$PROFDIR/prof-%p-%m.profraw" ./target/release/examples/pgo_train
done

echo "==> PGO step 3/4: merge raw profiles"
"$LLVM_PROFDATA" merge -o "$PROFDATA" "$PROFDIR"/*.profraw
ls -lh "$PROFDATA"

# Everything after this script's own flags is forwarded to the final cargo build.
# Default target: the `djvu` CLI binary.
FINAL_ARGS=("$@")
if [[ ${#FINAL_ARGS[@]} -eq 0 ]]; then
  FINAL_ARGS=(build --release --bin djvu --features cli)
else
  FINAL_ARGS=(build --release "${FINAL_ARGS[@]}")
fi

FINAL_RUSTFLAGS="-Cprofile-use=$PROFDATA -Cllvm-args=-pgo-warn-missing-function"
echo "==> PGO step 4/4: profile-guided rebuild"
echo "    RUSTFLAGS=\"$FINAL_RUSTFLAGS\""
echo "    cargo ${FINAL_ARGS[*]}"
RUSTFLAGS="$FINAL_RUSTFLAGS" cargo "${FINAL_ARGS[@]}"

echo
echo "==> PGO build done. Reuse the profile for any build with:"
echo "    RUSTFLAGS=\"$FINAL_RUSTFLAGS\" cargo build --release ..."
