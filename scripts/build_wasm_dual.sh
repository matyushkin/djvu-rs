#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${OUT:-$ROOT/examples/wasm/pkg}"
FEATURES="${FEATURES:-wasm}"

SCALAR_OUT="$OUT/scalar"
SIMD_OUT="$OUT/simd128"

rm -rf "$OUT"
mkdir -p "$OUT"

echo "==> Building scalar wasm package"
RUSTFLAGS="" wasm-pack build "$ROOT" \
  --target web \
  --out-dir "$SCALAR_OUT" \
  --release \
  -- \
  --features "$FEATURES"

echo "==> Building simd128 wasm package"
RUSTFLAGS="-C target-feature=+simd128" wasm-pack build "$ROOT" \
  --target web \
  --out-dir "$SIMD_OUT" \
  --release \
  -- \
  --features "$FEATURES"

echo "==> Writing runtime-detecting loader"
cat > "$OUT/djvu_rs.js" <<'EOF_JS'
import * as scalarModule from "./scalar/djvu_rs.js";
import * as simd128Module from "./simd128/djvu_rs.js";

const SIMD128_PROBE = new Uint8Array([
  0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
  0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7b,
  0x03, 0x02, 0x01, 0x00,
  0x0a, 0x16, 0x01, 0x14, 0x00, 0xfd, 0x0c,
  0x00, 0x00, 0x00, 0x00,
  0x00, 0x00, 0x00, 0x00,
  0x00, 0x00, 0x00, 0x00,
  0x00, 0x00, 0x00, 0x00,
  0x0b,
]);

let selectedModule;
let selectedVariant;

export let WasmDocument;
export let WasmPage;
export let WasmPixmap;
export let WasmLazyDocument;
export let initThreadPool;

export function wasmSimd128Supported() {
  return typeof WebAssembly === "object" && WebAssembly.validate(SIMD128_PROBE);
}

export function selectedWasmVariant() {
  return selectedVariant;
}

export default async function init(input) {
  if (selectedModule !== undefined) {
    return selectedModule;
  }

  const useSimd128 = wasmSimd128Supported();
  selectedVariant = useSimd128 ? "simd128" : "scalar";
  selectedModule = useSimd128 ? simd128Module : scalarModule;

  const wasmInput = input ?? new URL(`./${selectedVariant}/djvu_rs_bg.wasm`, import.meta.url);
  await selectedModule.default({ module_or_path: wasmInput });

  WasmDocument = selectedModule.WasmDocument;
  WasmPage = selectedModule.WasmPage;
  WasmPixmap = selectedModule.WasmPixmap;
  WasmLazyDocument = selectedModule.WasmLazyDocument;
  initThreadPool = selectedModule.initThreadPool;

  return selectedModule;
}

export function initSync() {
  throw new Error("The dual wasm loader requires async init() for runtime variant selection.");
}
EOF_JS

cp "$SCALAR_OUT/djvu_rs.d.ts" "$OUT/djvu_rs.d.ts"
cp "$SCALAR_OUT/README.md" "$OUT/README.md"
cp "$SCALAR_OUT/LICENSE" "$OUT/LICENSE"
cat >> "$OUT/djvu_rs.d.ts" <<'EOF_DTS'

export function wasmSimd128Supported(): boolean;
export function selectedWasmVariant(): "scalar" | "simd128" | undefined;
EOF_DTS
SCALAR_PACKAGE="$SCALAR_OUT/package.json" node --input-type=module > "$OUT/package.json" <<'EOF_NODE'
import { readFileSync } from "node:fs";

const pkg = JSON.parse(readFileSync(process.env.SCALAR_PACKAGE, "utf8"));
pkg.files = [
  "djvu_rs.js",
  "djvu_rs.d.ts",
  "README.md",
  "LICENSE",
  "scalar/djvu_rs.js",
  "scalar/djvu_rs.d.ts",
  "scalar/djvu_rs_bg.wasm",
  "scalar/djvu_rs_bg.wasm.d.ts",
  "simd128/djvu_rs.js",
  "simd128/djvu_rs.d.ts",
  "simd128/djvu_rs_bg.wasm",
  "simd128/djvu_rs_bg.wasm.d.ts",
];
pkg.type = "module";
pkg.module = "djvu_rs.js";
pkg.main = "djvu_rs.js";
pkg.types = "djvu_rs.d.ts";
pkg.exports = {
  ".": {
    types: "./djvu_rs.d.ts",
    import: "./djvu_rs.js",
    default: "./djvu_rs.js",
  },
};
pkg.sideEffects = false;
console.log(`${JSON.stringify(pkg, null, 2)}\n`);
EOF_NODE

echo "==> Wrote dual wasm package to $OUT"
