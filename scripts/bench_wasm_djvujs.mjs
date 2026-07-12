#!/usr/bin/env node
import { createServer } from "node:http";
import { mkdtempSync, readFileSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, extname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn, spawnSync } from "node:child_process";
import { once } from "node:events";

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const DEFAULT_FIXTURES = [
  "tests/corpus/big_scanned_page.djvu:0:photo",
  "tests/corpus/map_atlas_sample.djvu:0:line-art",
  "tests/corpus/goody_twoshoes.djvu:0:mixed",
  "tests/corpus/chinese_cookbook_sample.djvu:0:cjk",
  "tests/corpus/cyrillic_simonovich_co2.djvu:0:cyrillic",
  "tests/corpus/war_1812.djvu:0:newspaper",
  "references/djvujs/library/assets/colorbook.djvu:0:iw44-color",
  "references/djvujs/library/assets/carte.djvu:0:carte",
  "references/djvujs/library/assets/boy_jb2.djvu:0:jb2-small",
];

function usage() {
  console.error(`Usage:
  node scripts/bench_wasm_djvujs.mjs [--build-wasm] [--json]
    [--scalar target/wasm-djvujs/scalar]
    [--simd target/wasm-djvujs/simd128]
    [--djvujs target/bench-djvujs/package/library/dist/djvu.js]
    [--chrome "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"]
    [--fixtures path:page:class,...] [--iterations 7] [--warmup 2] [--dpi 100]
`);
}

function parseArgs(argv) {
  const args = {
    scalar: "target/wasm-djvujs/scalar",
    simd: "target/wasm-djvujs/simd128",
    djvujs: firstExisting([
      "references/djvujs/library/dist/djvu.js",
      "target/bench-djvujs/package/library/dist/djvu.js",
      "node_modules/djvujs-dist/library/dist/djvu.js",
    ]),
    chrome: firstExisting([
      "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
      "/Applications/Chromium.app/Contents/MacOS/Chromium",
    ]),
    fixtures: DEFAULT_FIXTURES,
    iterations: 7,
    warmup: 2,
    dpi: 100,
    buildWasm: false,
    json: false,
  };

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--build-wasm") {
      args.buildWasm = true;
      continue;
    }
    if (arg === "--json") {
      args.json = true;
      continue;
    }
    const next = argv[i + 1];
    if (next === undefined) {
      throw new Error(`missing value for ${arg}`);
    }
    i += 1;
    switch (arg) {
      case "--scalar":
        args.scalar = next;
        break;
      case "--simd":
        args.simd = next;
        break;
      case "--djvujs":
        args.djvujs = next;
        break;
      case "--chrome":
        args.chrome = next;
        break;
      case "--fixtures":
        args.fixtures = next.split(",").filter(Boolean);
        break;
      case "--iterations":
        args.iterations = Number.parseInt(next, 10);
        break;
      case "--warmup":
        args.warmup = Number.parseInt(next, 10);
        break;
      case "--dpi":
        args.dpi = Number.parseInt(next, 10);
        break;
      default:
        throw new Error(`unknown argument: ${arg}`);
    }
  }

  for (const key of ["iterations", "warmup", "dpi"]) {
    if (!Number.isInteger(args[key]) || args[key] <= 0) {
      throw new Error(`--${key} must be a positive integer`);
    }
  }
  if (!args.chrome) {
    throw new Error("Chrome not found; pass --chrome");
  }
  return args;
}

function firstExisting(paths) {
  for (const path of paths) {
    if (path && exists(path)) {
      return path;
    }
  }
  return undefined;
}

function exists(path) {
  try {
    statSync(resolve(REPO_ROOT, path));
    return true;
  } catch {
    return false;
  }
}

function buildWasm(args) {
  run("wasm-pack", [
    "build",
    ".",
    "--target",
    "web",
    "--release",
    "--out-dir",
    args.scalar,
    "--",
    "--features",
    "wasm",
  ]);
  run(
    "wasm-pack",
    [
      "build",
      ".",
      "--target",
      "web",
      "--release",
      "--out-dir",
      args.simd,
      "--",
      "--features",
      "wasm",
    ],
    { RUSTFLAGS: "-C target-feature=+simd128" },
  );
}

function run(command, argv, extraEnv = {}) {
  const result = spawnSync(command, argv, {
    cwd: REPO_ROOT,
    env: { ...process.env, ...extraEnv },
    stdio: "inherit",
  });
  if (result.status !== 0) {
    throw new Error(`${command} ${argv.join(" ")} failed`);
  }
}

function normalizeServedPath(path) {
  const abs = resolve(REPO_ROOT, path);
  if (!abs.startsWith(REPO_ROOT + sep) && abs !== REPO_ROOT) {
    throw new Error(`path escapes repo root: ${path}`);
  }
  return `/${abs.slice(REPO_ROOT.length + 1).split(sep).join("/")}`;
}

function parseFixture(spec) {
  const [path, page = "0", klass = "corpus"] = spec.split(":");
  return {
    path,
    url: normalizeServedPath(path),
    page: Number.parseInt(page, 10),
    class: klass,
  };
}

function startServer(djvujsPath) {
  const djvujsUrl = djvujsPath ? normalizeServedPath(djvujsPath) : null;
  const server = createServer((request, response) => {
    const url = new URL(request.url ?? "/", "http://127.0.0.1");
    if (url.pathname === "/__bench") {
      response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
      response.end(benchPage(djvujsUrl));
      return;
    }
    serveStatic(url.pathname, response);
  });
  return new Promise((resolvePromise) => {
    server.listen(0, "127.0.0.1", () => {
      resolvePromise({ server, port: server.address().port });
    });
  });
}

function serveStatic(pathname, response) {
  const decoded = decodeURIComponent(pathname);
  const abs = resolve(REPO_ROOT, "." + decoded);
  if (!abs.startsWith(REPO_ROOT + sep)) {
    response.writeHead(403).end("forbidden");
    return;
  }
  try {
    const type = mimeType(extname(abs));
    const body = readFileSync(abs);
    response.writeHead(200, { "content-type": type });
    response.end(body);
  } catch {
    response.writeHead(404).end("not found");
  }
}

function mimeType(ext) {
  switch (ext) {
    case ".js":
      return "text/javascript";
    case ".wasm":
      return "application/wasm";
    case ".djvu":
    case ".djv":
      return "image/vnd.djvu";
    default:
      return "application/octet-stream";
  }
}

function benchPage(djvujsUrl) {
  const djvuScript = djvujsUrl ? `<script src="${djvujsUrl}"></script>` : "";
  return `<!doctype html>
<meta charset="utf-8">
${djvuScript}
<script type="module">
const percentile = (values, p) => {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.min(sorted.length - 1, Math.floor((sorted.length - 1) * p))];
};
const summarize = (samples) => ({
  min_ms: Math.min(...samples),
  median_ms: percentile(samples, 0.5),
  p90_ms: percentile(samples, 0.9),
  mean_ms: samples.reduce((sum, value) => sum + value, 0) / samples.length,
});
const checksum = (pixels) => {
  let hash = 0x811c9dc5;
  for (let i = 0; i < pixels.length; i += 1) {
    hash ^= pixels[i];
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  hash ^= pixels.length;
  return hash >>> 0;
};
const heapMb = () => performance.memory?.usedJSHeapSize ? performance.memory.usedJSHeapSize / 1048576 : null;
const fetchBytes = async (url) => new Uint8Array(await (await fetch(url)).arrayBuffer());
const loadWasm = async (url) => {
  const pkg = await import(url);
  const wasmExports = await pkg.default();
  return { ...pkg, __wasmExports: wasmExports };
};
const wasmMemoryMb = (pkg) => pkg.__wasmExports?.memory?.buffer?.byteLength
  ? pkg.__wasmExports.memory.buffer.byteLength / 1048576
  : null;
const targetSize = (width, height, nativeDpi, targetDpi) => ({
  width: Math.max(1, Math.round(width * targetDpi / nativeDpi)),
  height: Math.max(1, Math.round(height * targetDpi / nativeDpi)),
});
const scaleImageData = (imageData, target) => {
  if (imageData.width === target.width && imageData.height === target.height) {
    return imageData.data;
  }
  const src = document.createElement("canvas");
  src.width = imageData.width;
  src.height = imageData.height;
  src.getContext("2d").putImageData(imageData, 0, 0);
  const dst = document.createElement("canvas");
  dst.width = target.width;
  dst.height = target.height;
  const ctx = dst.getContext("2d");
  ctx.imageSmoothingEnabled = true;
  ctx.drawImage(src, 0, 0, target.width, target.height);
  return ctx.getImageData(0, 0, target.width, target.height).data;
};
const benchOne = async (label, fixture, args, fn) => {
  for (let i = 0; i < args.warmup; i += 1) {
    await fn();
  }
  const samples = [];
  let expectedChecksum;
  let bytes = 0;
  let width = 0;
  let height = 0;
  const heapBefore = heapMb();
  for (let i = 0; i < args.iterations; i += 1) {
    if (globalThis.gc) globalThis.gc();
    const t0 = performance.now();
    const out = await fn();
    samples.push(performance.now() - t0);
    const currentChecksum = checksum(out.pixels);
    expectedChecksum ??= currentChecksum;
    if (currentChecksum !== expectedChecksum) {
      throw new Error(label + " unstable checksum for " + fixture.path);
    }
    bytes = out.pixels.length;
    width = out.width;
    height = out.height;
  }
  const heapAfter = heapMb();
  return {
    engine: label,
    fixture: fixture.path,
    class: fixture.class,
    page: fixture.page,
    dpi: args.dpi,
    iterations: args.iterations,
    width,
    height,
    pixels_mb: bytes / 1048576,
    checksum: expectedChecksum,
    heap_delta_mb: heapBefore === null || heapAfter === null ? null : heapAfter - heapBefore,
    ...summarize(samples),
  };
};
const benchWasmFixture = async (label, pkg, fixture, bytes, args, firstPage) => {
  const renderSelected = () => {
    const doc = pkg.WasmDocument.from_bytes(bytes);
    const page = doc.page(fixture.page);
    const pixels = page.render(args.dpi);
    return {
      pixels,
      width: page.width_at(args.dpi),
      height: page.height_at(args.dpi),
    };
  };
  const renderFirst = () => {
    const doc = pkg.WasmDocument.from_bytes(bytes);
    const page = doc.page(0);
    const pixels = page.render(args.dpi);
    return {
      pixels,
      width: page.width_at(args.dpi),
      height: page.height_at(args.dpi),
    };
  };
  const rows = [
    await benchOne(label + " first-page", { ...fixture, page: 0 }, args, renderFirst),
    await benchOne(label + " selected-page", fixture, args, renderSelected),
  ];
  for (const row of rows) {
    row.wasm_memory_mb = wasmMemoryMb(pkg);
  }
  return rows;
};
const benchDjvujsFixture = async (fixture, bytes, args) => {
  if (!globalThis.DjVu?.Document) return [];
  const renderPage = (pageIndex) => {
    const doc = new globalThis.DjVu.Document(bytes.buffer.slice(0));
    const page = doc.getPageUnsafe(pageIndex + 1);
    const nativeDpi = page.getDpi();
    const imageData = page.getImageData(true);
    const target = targetSize(imageData.width, imageData.height, nativeDpi, args.dpi);
    const pixels = scaleImageData(imageData, target);
    page.reset();
    return { pixels, width: target.width, height: target.height };
  };
  return [
    await benchOne("djvu.js first-page", { ...fixture, page: 0 }, args, () => renderPage(0)),
    await benchOne("djvu.js selected-page", fixture, args, () => renderPage(fixture.page)),
  ].map((row) => ({ ...row, wasm_memory_mb: null }));
};
window.runBench = async (args) => {
  const scalar = await loadWasm(args.scalarUrl);
  const simd = args.simdUrl ? await loadWasm(args.simdUrl) : null;
  const results = [];
  for (const fixture of args.fixtures) {
    const bytes = await fetchBytes(fixture.url);
    results.push(...await benchWasmFixture("wasm scalar", scalar, fixture, bytes, args));
    if (simd) {
      results.push(...await benchWasmFixture("wasm simd128", simd, fixture, bytes, args));
    }
    results.push(...await benchDjvujsFixture(fixture, bytes, args));
  }
  return {
    userAgent: navigator.userAgent,
    djvujsVersion: globalThis.DjVu?.VERSION ?? null,
    simdSupported: WebAssembly.validate(new Uint8Array([
      0,97,115,109,1,0,0,0,1,5,1,96,0,1,123,3,2,1,0,10,10,1,8,0,65,0,253,15,253,98,11
    ])),
    results,
  };
};
</script>`;
}

async function launchChrome(chromePath, url) {
  const profile = mkdtempSync(join(tmpdir(), "djvu-rs-chrome-"));
  const child = spawn(chromePath, [
    "--headless=new",
    "--disable-gpu",
    "--no-first-run",
    "--no-default-browser-check",
    "--remote-debugging-port=0",
    `--user-data-dir=${profile}`,
    "--js-flags=--expose-gc",
    "about:blank",
  ]);
  const endpoint = await new Promise((resolvePromise, rejectPromise) => {
    let stderr = "";
    const timer = setTimeout(() => rejectPromise(new Error("Chrome DevTools endpoint timeout")), 10000);
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
      const match = stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/);
      if (match) {
        clearTimeout(timer);
        resolvePromise(match[1]);
      }
    });
    child.on("exit", (code) => rejectPromise(new Error(`Chrome exited early: ${code}`)));
  });
  const base = endpoint.replace(/^ws:\/\/([^/]+).*/, "http://$1");
  const newPageResponse = await fetch(`${base}/json/new?${encodeURIComponent(url)}`, { method: "PUT" });
  const page = await newPageResponse.json();
  return { child, profile, pageWs: page.webSocketDebuggerUrl };
}

class CdpClient {
  constructor(wsUrl) {
    this.ws = new WebSocket(wsUrl);
    this.nextId = 1;
    this.pending = new Map();
    this.ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      if (message.id && this.pending.has(message.id)) {
        const { resolvePromise, rejectPromise } = this.pending.get(message.id);
        this.pending.delete(message.id);
        if (message.error) rejectPromise(new Error(message.error.message));
        else resolvePromise(message.result);
      }
    };
  }

  async open() {
    if (this.ws.readyState === WebSocket.OPEN) return;
    await new Promise((resolvePromise, rejectPromise) => {
      this.ws.onopen = resolvePromise;
      this.ws.onerror = rejectPromise;
    });
  }

  call(method, params = {}) {
    const id = this.nextId++;
    const payload = JSON.stringify({ id, method, params });
    return new Promise((resolvePromise, rejectPromise) => {
      this.pending.set(id, { resolvePromise, rejectPromise });
      this.ws.send(payload);
    });
  }

  close() {
    this.ws.onmessage = null;
    this.ws.onerror = null;
    this.ws.onopen = null;
    this.ws.close();
  }
}

async function runBrowserBench(args) {
  const fixtures = args.fixtures.map(parseFixture);
  const { server, port } = await startServer(args.djvujs);
  const url = `http://127.0.0.1:${port}/__bench`;
  const chrome = await launchChrome(args.chrome, url);
  const cdp = new CdpClient(chrome.pageWs);
  try {
    await cdp.open();
    await cdp.call("Runtime.enable");
    await cdp.call("Page.enable");
    await waitForRunBench(cdp);
    const config = {
      scalarUrl: normalizeServedPath(join(args.scalar, "djvu_rs.js")),
      simdUrl: args.simd && exists(join(args.simd, "djvu_rs.js")) ? normalizeServedPath(join(args.simd, "djvu_rs.js")) : null,
      fixtures,
      iterations: args.iterations,
      warmup: args.warmup,
      dpi: args.dpi,
    };
    const result = await cdp.call("Runtime.evaluate", {
      expression: `window.runBench(${JSON.stringify(config)})`,
      awaitPromise: true,
      returnByValue: true,
    });
    if (result.exceptionDetails) {
      const details = result.exceptionDetails;
      const description = details.exception?.description ?? details.text;
      throw new Error(`${description}\n${JSON.stringify(details, null, 2)}`);
    }
    return result.result.value;
  } finally {
    cdp.close();
    chrome.child.kill("SIGKILL");
    await Promise.race([
      once(chrome.child, "exit").catch(() => undefined),
      new Promise((resolvePromise) => setTimeout(resolvePromise, 1000)),
    ]);
    try {
      rmSync(chrome.profile, { recursive: true, force: true });
    } catch {
      // Chrome can still be releasing its profile files after SIGKILL.
    }
    await new Promise((resolvePromise) => server.close(resolvePromise));
  }
}

async function waitForRunBench(cdp) {
  const deadline = Date.now() + 10000;
  while (Date.now() < deadline) {
    const result = await cdp.call("Runtime.evaluate", {
      expression: "window.runBench !== undefined",
      returnByValue: true,
    });
    if (result.result.value === true) {
      return;
    }
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 50));
  }
  throw new Error("browser benchmark page did not initialize");
}

function pct(base, value) {
  return ((value - base) / base) * 100;
}

function printMarkdown(report) {
  console.log(`Chrome: ${report.userAgent}`);
  console.log(`djvu.js: ${report.djvujsVersion ?? "unavailable"}`);
  console.log(`simd128 supported: ${report.simdSupported}`);
  console.log("");
  console.log("| Fixture | Class | Page | Engine | Median ms | p90 ms | Pixels MiB | Heap delta MiB | Wasm memory MiB | vs djvu.js |");
  console.log("|---|---|---:|---|---:|---:|---:|---:|---:|---:|");
  const byCase = new Map();
  for (const row of report.results) {
    const key = `${row.fixture}:${row.page}:${row.engine.replace(/^wasm (scalar|simd128) /, "")}`;
    if (!byCase.has(key)) byCase.set(key, {});
    byCase.get(key)[row.engine] = row;
  }
  for (const row of report.results) {
    const family = row.engine.replace(/^wasm (scalar|simd128) /, "");
    const baseline = byCase.get(`${row.fixture}:${row.page}:${family}`)?.[`djvu.js ${family}`]?.median_ms;
    const delta = baseline === undefined ? "n/a" : `${pct(baseline, row.median_ms).toFixed(1)}%`;
    const heap = row.heap_delta_mb === null ? "n/a" : row.heap_delta_mb.toFixed(1);
    const wasmMemory = row.wasm_memory_mb === null ? "n/a" : row.wasm_memory_mb.toFixed(1);
    console.log(
      `| \`${row.fixture}\` | ${row.class} | ${row.page} | ${row.engine} | ${row.median_ms.toFixed(2)} | ${row.p90_ms.toFixed(2)} | ${row.pixels_mb.toFixed(1)} | ${heap} | ${wasmMemory} | ${delta} |`,
    );
  }
}

try {
  const args = parseArgs(process.argv.slice(2));
  if (args.buildWasm) {
    buildWasm(args);
  }
  const report = await runBrowserBench(args);
  if (args.json) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    printMarkdown(report);
  }
  process.exit(0);
} catch (err) {
  usage();
  console.error(err instanceof Error ? err.message : err);
  process.exit(2);
}
