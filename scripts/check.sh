#!/usr/bin/env bash
# Local mirror of the deterministic CI gates (.github/workflows/ci.yml).
# Run before pushing to catch fmt / clippy / no_std / wasm / test breaks locally
# instead of via a red CI run. Wired as the pre-push hook (see `make hooks`).
# Bypass once (not recommended): git push --no-verify
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

run() { printf '\n==> %s\n' "$*"; "$@"; }

run cargo fmt --check
run cargo clippy --all-targets -- -D warnings
run cargo clippy --all-targets --features cli,epub -- -D warnings  # pdf/epub writers (#509)
run cargo clippy --all-targets --features tiff -- -D warnings      # tiff ingest/export tree, which djvu-py now compiles
run cargo check --all-targets --features ocr-onnx           # ocr_onnx test tree; its CI job runs only on main push, so PRs never gate it
run scripts/check_feature_hygiene.sh                        # decode-only default tree (#509)
run scripts/check_package_versions.sh                       # py/npm versions track crate (#692)
run cargo build --no-default-features                       # no_std (host)
# Broken or private intra-doc links (mirrors the "Rustdoc" Lint step).
run env RUSTDOCFLAGS='-D warnings' cargo doc --workspace --exclude djvu-py --no-deps \
  --features cli,epub,tiff,async,parallel,mmap,serde,image,wasm-lazy,ocr-onnx,ocr-neural,experimental,iw44-probe

# wasm32 — the gate that catches no_std `vec!` / leaked `std::*` (#448 class).
if rustup target list --installed 2>/dev/null | grep -q '^wasm32-unknown-unknown'; then
  run cargo check --target wasm32-unknown-unknown --features wasm
  # Lazy Range open (#588). Clippy, not just check: wasm32 std stubs (e.g. a
  # `File` without `Drop`) trigger lints the host build never sees (#863).
  run cargo clippy --target wasm32-unknown-unknown --features wasm-lazy -- -D warnings
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
  run cargo nextest run --workspace --exclude djvu-py --features cli,tiff,async
else
  run cargo test --workspace --exclude djvu-py --features cli,tiff,async
fi

# README doctests — nextest skips doctests; this compiles every rust block in
# README.md and docs/guide.md via the ReadmeDoctests / GuideDoctests includes
# in src/lib.rs (doc-sync gate,
# mirrors the "README doctests" CI step).
run cargo test --doc --features cli,tiff,async,serde,image,epub

printf '\n\xe2\x9c\x93 local CI gates passed\n'
