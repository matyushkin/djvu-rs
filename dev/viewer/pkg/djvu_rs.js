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
export let WasmLazyPage;
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
  WasmLazyPage = selectedModule.WasmLazyPage;
  initThreadPool = selectedModule.initThreadPool;

  return selectedModule;
}

export function initSync() {
  throw new Error("The dual wasm loader requires async init() for runtime variant selection.");
}
