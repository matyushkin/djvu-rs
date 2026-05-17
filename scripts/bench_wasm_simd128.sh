#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${OUT:-$ROOT/target/wasm-bench}"
FIXTURE="${FIXTURE:-$ROOT/tests/fixtures/boy.djvu}"
ITERATIONS="${ITERATIONS:-50}"
WARMUP="${WARMUP:-10}"
DPI="${DPI:-150}"

mkdir -p "$OUT"

echo "==> Building scalar wasm bundle"
RUSTFLAGS="" wasm-pack build "$ROOT" \
  --target nodejs \
  --out-dir "$OUT/scalar" \
  --no-opt \
  -- \
  --features wasm

echo "==> Building simd128 wasm bundle"
RUSTFLAGS="-C target-feature=+simd128" wasm-pack build "$ROOT" \
  --target nodejs \
  --out-dir "$OUT/simd128" \
  --no-opt \
  -- \
  --features wasm

echo "==> Running scalar vs simd128 benchmark"
node "$ROOT/scripts/bench_wasm_simd128.mjs" \
  --scalar "$OUT/scalar" \
  --simd "$OUT/simd128" \
  --fixture "$FIXTURE" \
  --iterations "$ITERATIONS" \
  --warmup "$WARMUP" \
  --dpi "$DPI"
