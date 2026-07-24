#!/usr/bin/env node
/**
 * Smoke-test an npm package directory or packed tarball.
 *
 * Covers Node init + document open/render, and optionally a bundler graph check.
 * Browser smoke lives in smoke_npm_browser.mjs (Playwright).
 */

import { createHash } from "node:crypto";
import {
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { spawnSync } from "node:child_process";

function usage() {
  console.error(`Usage:
  node scripts/smoke_npm_package.mjs --package <dir|tgz> --fixture <file.djvu> [--expect-version X]
  node scripts/smoke_npm_package.mjs --package <dir|tgz> --fixture <file.djvu> --bundler
`);
  process.exit(2);
}

function parseArgs(argv) {
  const out = { package: null, fixture: null, expectVersion: null, bundler: false };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--package") out.package = argv[++i];
    else if (arg === "--fixture") out.fixture = argv[++i];
    else if (arg === "--expect-version") out.expectVersion = argv[++i];
    else if (arg === "--bundler") out.bundler = true;
    else usage();
  }
  if (!out.package || !out.fixture) usage();
  return out;
}

function run(cmd, args, opts = {}) {
  const result = spawnSync(cmd, args, {
    stdio: "inherit",
    encoding: "utf8",
    ...opts,
  });
  if (result.status !== 0) {
    throw new Error(`${cmd} ${args.join(" ")} failed with status ${result.status}`);
  }
}

function sha256File(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

async function smokeNode(pkgDir, fixturePath, expectVersion) {
  const pkgJson = JSON.parse(readFileSync(join(pkgDir, "package.json"), "utf8"));
  console.log(`npm package version=${pkgJson.version}`);
  if (expectVersion && pkgJson.version !== expectVersion) {
    throw new Error(
      `version mismatch: package.json=${pkgJson.version} expected=${expectVersion}`,
    );
  }

  for (const rel of [
    "djvu_rs.d.ts",
    "scalar/djvu_rs_bg.wasm",
    "simd128/djvu_rs_bg.wasm",
  ]) {
    const abs = join(pkgDir, rel);
    readFileSync(abs);
    console.log(`artifact ok: ${rel} sha256=${sha256File(abs).slice(0, 12)}…`);
  }

  const entry = pathToFileURL(join(pkgDir, "djvu_rs.js")).href;
  const mod = await import(entry);
  // `--target web` loaders use fetch(URL); pass bytes so Node smoke works
  // from an extracted tarball without an HTTP server.
  const preferSimd =
    typeof WebAssembly === "object" &&
    WebAssembly.validate(
      new Uint8Array([
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60,
        0x00, 0x01, 0x7b, 0x03, 0x02, 0x01, 0x00, 0x0a, 0x16, 0x01, 0x14, 0x00,
        0xfd, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0b,
      ]),
    );
  const predicted = preferSimd ? "simd128" : "scalar";
  const wasmBytes = readFileSync(join(pkgDir, predicted, "djvu_rs_bg.wasm"));
  await mod.default(wasmBytes);

  const variant = mod.selectedWasmVariant?.();
  console.log(`selectedWasmVariant=${variant}`);
  if (variant !== "scalar" && variant !== "simd128") {
    throw new Error(`unexpected wasm variant: ${variant}`);
  }

  const bytes = new Uint8Array(readFileSync(fixturePath));
  const doc = mod.WasmDocument.from_bytes(bytes);
  const pageCount = doc.page_count();
  if (pageCount < 1) throw new Error("page_count < 1");
  const page = doc.page(0);
  const pixels = page.render(72);
  if (!(pixels instanceof Uint8ClampedArray) && !(pixels instanceof Uint8Array)) {
    throw new Error("render did not return a typed array");
  }
  if (pixels.length < 4) throw new Error("render produced empty buffer");
  console.log(
    `node smoke OK: pages=${pageCount} render_bytes=${pixels.length} variant=${variant}`,
  );
}

function ensureTarball(packagePath, pkgDir) {
  const abs = resolve(packagePath);
  if (abs.endsWith(".tgz") || abs.endsWith(".tar.gz")) {
    return { tarball: abs, cleanup: null };
  }
  const packDir = mkdtempSync(join(tmpdir(), "djvu-npm-repack-"));
  run("npm", ["pack", "--pack-destination", packDir], { cwd: pkgDir });
  const name = readdirSync(packDir).find((n) => n.endsWith(".tgz"));
  if (!name) {
    rmSync(packDir, { recursive: true, force: true });
    throw new Error("npm pack produced no tarball");
  }
  return { tarball: join(packDir, name), cleanup: packDir };
}

function smokeBundler(tarball) {
  const work = mkdtempSync(join(tmpdir(), "djvu-npm-bundler-"));
  try {
    writeFileSync(
      join(work, "package.json"),
      JSON.stringify({
        name: "djvu-bundler-smoke",
        private: true,
        type: "module",
      }),
    );
    run("npm", ["install", "--no-save", resolve(tarball)], { cwd: work });
    const entry = join(work, "entry.mjs");
    writeFileSync(
      entry,
      `import init, { selectedWasmVariant } from "djvu-rs";\nexport { init, selectedWasmVariant };\n`,
    );
    const outfile = join(work, "bundle.mjs");
    run(
      "npx",
      [
        "--yes",
        "esbuild@0.25.5",
        entry,
        "--bundle",
        "--format=esm",
        "--platform=browser",
        "--main-fields=module,browser,main",
        "--conditions=import,module,browser,default",
        `--outfile=${outfile}`,
        "--loader:.wasm=file",
      ],
      { cwd: work },
    );
    const bundled = readFileSync(outfile, "utf8");
    if (!bundled.includes("selectedWasmVariant")) {
      throw new Error("bundler output missing selectedWasmVariant");
    }
    console.log(`bundler smoke OK: ${bundled.length} bytes`);
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}

function materializePackage(packagePath) {
  const abs = resolve(packagePath);
  if (abs.endsWith(".tgz") || abs.endsWith(".tar.gz")) {
    const work = mkdtempSync(join(tmpdir(), "djvu-npm-pack-"));
    run("tar", ["-xzf", abs, "-C", work]);
    return { pkgDir: join(work, "package"), cleanup: work };
  }
  return { pkgDir: abs, cleanup: null };
}

const args = parseArgs(process.argv.slice(2));
const fixture = resolve(args.fixture);
const packagePath = resolve(args.package);
const { pkgDir, cleanup } = materializePackage(packagePath);

try {
  await smokeNode(pkgDir, fixture, args.expectVersion);
  if (args.bundler) {
    const packed = ensureTarball(packagePath, pkgDir);
    try {
      smokeBundler(packed.tarball);
    } finally {
      if (packed.cleanup) rmSync(packed.cleanup, { recursive: true, force: true });
    }
  }
} finally {
  if (cleanup) rmSync(cleanup, { recursive: true, force: true });
}
