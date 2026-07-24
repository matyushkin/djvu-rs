#!/usr/bin/env node
/**
 * Browser smoke test for the dual wasm npm package using Playwright Chromium.
 *
 * Serves the package directory over HTTP, loads a tiny page that imports the
 * package entry, and asserts init + WasmDocument.from_bytes succeed.
 *
 * Expects `playwright` to be importable (CI installs it before invoking this
 * script). Does not write into the repository working tree.
 */

import { createServer } from "node:http";
import { readFileSync, existsSync } from "node:fs";
import { extname, join, resolve } from "node:path";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";

function usage() {
  console.error(
    "Usage: node scripts/smoke_npm_browser.mjs --package <pkg-dir> --fixture <file.djvu>",
  );
  process.exit(2);
}

function parseArgs(argv) {
  const out = { package: null, fixture: null };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--package") out.package = argv[++i];
    else if (arg === "--fixture") out.fixture = argv[++i];
    else usage();
  }
  if (!out.package || !out.fixture) usage();
  return out;
}

function contentType(filePath) {
  switch (extname(filePath)) {
    case ".html":
      return "text/html; charset=utf-8";
    case ".js":
    case ".mjs":
      return "text/javascript; charset=utf-8";
    case ".wasm":
      return "application/wasm";
    default:
      return "application/octet-stream";
  }
}

async function loadPlaywright() {
  try {
    return await import("playwright");
  } catch {
    // Fall back to a require from a sibling install dir if NODE_PATH is set.
    const require = createRequire(pathToFileURL(resolve("package.json")));
    try {
      return require("playwright");
    } catch {
      throw new Error(
        "playwright is not installed; run: npm install playwright && npx playwright install chromium",
      );
    }
  }
}

const args = parseArgs(process.argv.slice(2));
const pkgDir = resolve(args.package);
const fixturePath = resolve(args.fixture);
if (!existsSync(join(pkgDir, "djvu_rs.js"))) {
  throw new Error(`missing djvu_rs.js in ${pkgDir}`);
}
if (!existsSync(fixturePath)) {
  throw new Error(`fixture not found: ${fixturePath}`);
}

const fixtureBytes = readFileSync(fixturePath);
const html = `<!doctype html>
<html>
  <body>
    <pre id="out">boot</pre>
    <script type="module">
      const out = document.getElementById("out");
      try {
        const mod = await import("/pkg/djvu_rs.js");
        await mod.default();
        const resp = await fetch("/fixture.djvu");
        const buf = new Uint8Array(await resp.arrayBuffer());
        const doc = mod.WasmDocument.from_bytes(buf);
        const page = doc.page(0);
        const pixels = page.render(72);
        out.textContent = JSON.stringify({
          ok: true,
          variant: mod.selectedWasmVariant(),
          pages: doc.page_count(),
          bytes: pixels.length,
        });
      } catch (err) {
        out.textContent = JSON.stringify({
          ok: false,
          error: String(err && err.stack ? err.stack : err),
        });
      }
    </script>
  </body>
</html>`;

const server = createServer((req, res) => {
  try {
    const url = new URL(req.url, "http://127.0.0.1");
    if (url.pathname === "/" || url.pathname === "/index.html") {
      res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
      res.end(html);
      return;
    }
    if (url.pathname === "/fixture.djvu") {
      res.writeHead(200, { "Content-Type": "application/octet-stream" });
      res.end(fixtureBytes);
      return;
    }
    if (url.pathname.startsWith("/pkg/")) {
      const rel = url.pathname.slice("/pkg/".length);
      const filePath = join(pkgDir, rel);
      const body = readFileSync(filePath);
      res.writeHead(200, { "Content-Type": contentType(filePath) });
      res.end(body);
      return;
    }
    res.writeHead(404);
    res.end("not found");
  } catch (err) {
    res.writeHead(500);
    res.end(String(err));
  }
});

await new Promise((resolveListen) => server.listen(0, "127.0.0.1", resolveListen));
const { port } = server.address();
const base = `http://127.0.0.1:${port}/`;

try {
  const { chromium } = await loadPlaywright();
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage();
    await page.goto(base, { waitUntil: "networkidle" });
    await page.waitForFunction(
      () => {
        const text = document.getElementById("out")?.textContent ?? "";
        return text.startsWith("{");
      },
      null,
      { timeout: 60000 },
    );
    const payload = JSON.parse(await page.textContent("#out"));
    if (!payload.ok) {
      throw new Error(`browser smoke failed: ${payload.error}`);
    }
    console.log(
      `browser smoke OK: pages=${payload.pages} bytes=${payload.bytes} variant=${payload.variant}`,
    );
  } finally {
    await browser.close();
  }
} finally {
  server.close();
}
