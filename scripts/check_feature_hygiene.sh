#!/usr/bin/env bash
# Feature-hygiene gate (#509): the default (decode-only) dependency tree must
# stay lean. Writer-side and feature-owned crates may only enter the build via
# the feature that uses them (`pdf`, `epub`, `cli`, `tiff`, ...), never via
# `std`/default. This is the regression guard for the #509 class of issue:
# "std hardwires writer deps that decode-only users can never reach".
#
# Run from anywhere inside the repo; part of scripts/check.sh and the CI Lint
# job. If this fails, a Cargo.toml change routed a feature-owned dependency
# back into the default tree — move it under the feature that uses it.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

# Crates that must NEVER appear in the default-features dependency tree of
# djvu-rs. Each is owned by an opt-in feature (right column).
forbidden=(
  zip           # epub
  zopfli        # (zip's `deflate` umbrella — must stay out entirely, #509)
  bumpalo       # (zopfli transitive)
  jpeg-encoder  # pdf
  clap          # cli
  tiff          # tiff
  tokio         # async
  rayon         # parallel
  memmap2       # mmap
  image         # image
  wasm-bindgen  # wasm
  tesseract     # ocr-tesseract
  tract-onnx    # ocr-onnx
  sha2          # ocr-onnx (model-weight verification, #693)
)

# `-e normal` excludes dev/build deps; `--prefix none` gives "name version"
# lines. Default features = the plain `djvu-rs = "…"` consumer's tree.
tree="$(cargo tree -p djvu-rs -e normal --prefix none | awk '{print $1}' | sort -u)"

fail=0
for crate in "${forbidden[@]}"; do
  if grep -qx "$crate" <<<"$tree"; then
    echo "!! feature hygiene: '$crate' leaked into the default (decode-only) dependency tree" >&2
    echo "   cargo tree -p djvu-rs -e normal -i $crate   # shows the path that pulls it in" >&2
    fail=1
  fi
done

if [ "$fail" -ne 0 ]; then
  echo "!! see #509 — writer/feature deps must be owned by their feature, not by std/default" >&2
  exit 1
fi

echo "feature hygiene OK: default tree is decode-only ($(wc -l <<<"$tree" | tr -d ' ') crates)"
