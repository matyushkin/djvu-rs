#!/usr/bin/env bash
# Assemble the static <djvu-viewer> demo site from a built npm package.
#
#   make wasm                                  # builds examples/wasm/pkg
#   scripts/build_viewer_demo.sh               # writes viewer_site/
#   python3 -m http.server -d viewer_site      # http://localhost:8000
#
# PKG: built npm package directory (default: examples/wasm/pkg).
# OUT: output directory (default: viewer_site).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PKG="${PKG:-$ROOT/examples/wasm/pkg}"
OUT="${OUT:-$ROOT/viewer_site}"

if [ ! -f "$PKG/viewer/djvu-viewer.js" ]; then
  echo "error: $PKG is not a built package; run 'make wasm' first" >&2
  exit 1
fi

rm -rf "$OUT"
mkdir -p "$OUT/pkg"
cp "$ROOT/viewer/demo/index.html" "$OUT/index.html"
cp "$ROOT/tests/fixtures/DjVu3Spec_bundled.djvu" "$OUT/sample.djvu"
# Only what the browser loads: the loader, both wasm builds, the viewer.
cp "$PKG/djvu_rs.js" "$OUT/pkg/"
for variant in scalar simd128; do
  mkdir -p "$OUT/pkg/$variant"
  cp "$PKG/$variant/djvu_rs.js" "$PKG/$variant/djvu_rs_bg.wasm" "$OUT/pkg/$variant/"
done
mkdir -p "$OUT/pkg/viewer"
cp "$PKG/viewer/djvu-viewer.js" "$PKG/viewer/djvu-viewer-worker.js" "$OUT/pkg/viewer/"
echo "==> Wrote the viewer demo to $OUT"
