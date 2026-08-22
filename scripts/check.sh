#!/usr/bin/env bash
# Local mirror of the deterministic CI gates (.github/workflows/ci.yml).
# Run before pushing to catch fmt / clippy / no_std / wasm / test breaks locally
# instead of via a red CI run. Wired as the pre-push hook (see `make hooks`).
# Bypass once (not recommended): git push --no-verify
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

run() { printf '\n==> %s\n' "$*"; "$@"; }

run cargo fmt --check
# chunks_exact_to_as_chunks (clippy 1.98): ~50 hot decode/encode loops use
# chunks_exact(N); migrating to as_chunks is a deliberate follow-up (#768),
# not a lint-chase. unknown_lints keeps pre-1.98 local toolchains green.
CLIPPY_FLAGS=(-D warnings -A unknown_lints -A clippy::chunks_exact_to_as_chunks)
run cargo clippy --all-targets -- "${CLIPPY_FLAGS[@]}"
run cargo clippy --all-targets --features cli,epub -- "${CLIPPY_FLAGS[@]}"  # pdf/epub writers (#509)
run scripts/check_feature_hygiene.sh                        # decode-only default tree (#509)
run scripts/check_package_versions.sh                       # py/npm versions track crate (#692)
run cargo build --no-default-features                       # no_std (host)

# wasm32 — the gate that catches no_std `vec!` / leaked `std::*` (#448 class).
if rustup target list --installed 2>/dev/null | grep -q '^wasm32-unknown-unknown'; then
  run cargo check --target wasm32-unknown-unknown --features wasm
  run cargo check --target wasm32-unknown-unknown --features wasm-lazy  # lazy Range open (#588)
  run cargo build --no-default-features --target wasm32-unknown-unknown
  run env RUSTFLAGS='-C target-feature=+simd128' \
    cargo check --target wasm32-unknown-unknown --features wasm
  run cargo build --manifest-path tests/no_std_smoke/Cargo.toml \
    --target wasm32-unknown-unknown
else
  echo "!! wasm32-unknown-unknown not installed — run: rustup target add wasm32-unknown-unknown" >&2
  exit 1
fi

# tests (nextest if present, else cargo test) — same scope as CI
if command -v cargo-nextest >/dev/null 2>&1; then
  run cargo nextest run --workspace --exclude djvu-py --features cli,tiff
else
  run cargo test --workspace --exclude djvu-py --features cli,tiff
fi

# README doctests — nextest skips doctests; this compiles every rust block in
# README.md via the ReadmeDoctests include in src/lib.rs (doc-sync gate,
# mirrors the "README doctests" CI step).
run cargo test --doc --features cli,tiff,async,serde,image,epub

printf '\n\xe2\x9c\x93 local CI gates passed\n'
