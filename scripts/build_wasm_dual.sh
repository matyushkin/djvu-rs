#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${OUT:-$ROOT/examples/wasm/pkg}"
# wasm-lazy (a superset of wasm) adds the HTTP Range loaders; the npm package ships it.
FEATURES="${FEATURES:-wasm-lazy}"

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
export let WasmRenderRequest;
export let WasmLazyDocument;
export let WasmLazyIndirectDocument;
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
  WasmRenderRequest = selectedModule.WasmRenderRequest;
  WasmLazyDocument = selectedModule.WasmLazyDocument;
  WasmLazyIndirectDocument = selectedModule.WasmLazyIndirectDocument;
  initThreadPool = selectedModule.initThreadPool;

  return selectedModule;
}

export function initSync() {
  throw new Error("The dual wasm loader requires async init() for runtime variant selection.");
}
EOF_JS

cp "$SCALAR_OUT/djvu_rs.d.ts" "$OUT/djvu_rs.d.ts"
# npm gets its own landing page; wasm-pack would copy the Rust crate README.
cp "$ROOT/examples/wasm/README.npm.md" "$OUT/README.md"
cp "$SCALAR_OUT/LICENSE" "$OUT/LICENSE"
# License texts for the third-party crates linked into the .wasm (#949). Needs
# cargo-about; CI (and so every published package) requires it, a local demo
# build without it just skips the file.
if command -v cargo-about >/dev/null 2>&1; then
  python3 "$ROOT/scripts/third_party_notices.py" npm --features "$FEATURES" --out "$OUT/THIRD_PARTY_NOTICES"
elif [[ -n "${CI:-}" ]]; then
  echo "error: cargo-about is required to build the npm package (cargo install cargo-about --locked --features cli)" >&2
  exit 1
else
  echo "warning: cargo-about not found; skipping THIRD_PARTY_NOTICES (cargo install cargo-about --locked --features cli)" >&2
fi
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
  "THIRD_PARTY_NOTICES",
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
// The crate's description and keywords cover the CLI/Python encoders and
// PDF export, which the JS package does not ship.
pkg.description =
  "Open and render DjVu files in the browser or Node: pure-Rust WebAssembly, lazy HTTP Range loading, text layer, TypeScript types. MIT, no GPL.";
pkg.keywords = ["djvu", "viewer", "render", "wasm", "webassembly", "document", "scan"];
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
