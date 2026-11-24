#!/usr/bin/env node
/**
 * Browser smoke test for the <djvu-viewer> web component (Playwright Chromium).
 *
 * Serves the npm package directory and a fixture over HTTP, opens a page with
 * the element, and checks that the first page paints non-blank pixels, that
 * the text layer is present, and that the page and zoom controls work.
 *
 * It runs twice: against a server that answers HTTP Range requests (the
 * document opens lazily) and against one that does not (full download).
 *
 * Expects `playwright` to be importable. Writes nothing into the working tree.
 */

import { createServer } from "node:http";
import { readFileSync, existsSync } from "node:fs";
import { extname, join, resolve } from "node:path";

function usage() {
  console.error(
    "Usage: node scripts/smoke_viewer_browser.mjs --package <pkg-dir> --fixture <multi-page.djvu>",
  );
  process.exit(2);
}

const args = { package: null, fixture: null };
for (let i = 2; i < process.argv.length; i++) {
  if (process.argv[i] === "--package") args.package = process.argv[++i];
  else if (process.argv[i] === "--fixture") args.fixture = process.argv[++i];
  else usage();
}
if (!args.package || !args.fixture) usage();

const pkgDir = resolve(args.package);
const fixturePath = resolve(args.fixture);
if (!existsSync(join(pkgDir, "viewer", "djvu-viewer.js"))) {
  throw new Error(`missing viewer/djvu-viewer.js in ${pkgDir}`);
}
const fixture = readFileSync(fixturePath);

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".wasm": "application/wasm",
};

const HTML = `<!doctype html>
<html>
  <body style="margin:0">
    <djvu-viewer id="viewer" src="/fixture.djvu" controls
                 style="width:900px;height:700px"></djvu-viewer>
    <script type="module">
      window.events = [];
      const viewer = document.getElementById("viewer");
      for (const type of ["documentload", "pagechange", "zoomchange", "documenterror"]) {
        viewer.addEventListener(type, (e) => window.events.push({ type, ...e.detail }));
      }
      import("/pkg/viewer/djvu-viewer.js");
    </script>
  </body>
</html>`;

/** Serve the page, the package and the fixture; count the fixture bytes sent. */
function startServer(ranges) {
  const stats = { fixtureBytes: 0, rangeRequests: 0 };
  const server = createServer((req, res) => {
    try {
      const url = new URL(req.url, "http://127.0.0.1");
      if (url.pathname === "/") {
        res.writeHead(200, { "Content-Type": TYPES[".html"] });
        res.end(HTML);
      } else if (url.pathname === "/fixture.djvu") {
        const match = /^bytes=(\d+)-(\d+)?$/.exec(req.headers.range ?? "");
        if (ranges && match) {
          const start = Number(match[1]);
          const end = Math.min(fixture.length - 1, match[2] ? Number(match[2]) : fixture.length - 1);
          stats.rangeRequests += 1;
          stats.fixtureBytes += end - start + 1;
          res.writeHead(206, {
            "Content-Type": "application/octet-stream",
            "Content-Range": `bytes ${start}-${end}/${fixture.length}`,
            "Accept-Ranges": "bytes",
          });
          res.end(fixture.subarray(start, end + 1));
        } else {
          stats.fixtureBytes += fixture.length;
          res.writeHead(200, { "Content-Type": "application/octet-stream" });
          res.end(fixture);
        }
      } else if (url.pathname.startsWith("/pkg/")) {
        const filePath = join(pkgDir, url.pathname.slice("/pkg/".length));
        res.writeHead(200, { "Content-Type": TYPES[extname(filePath)] ?? "application/octet-stream" });
        res.end(readFileSync(filePath));
      } else {
        res.writeHead(404);
        res.end("not found");
      }
    } catch (err) {
      res.writeHead(500);
      res.end(String(err));
    }
  });
  return new Promise((done) =>
    server.listen(0, "127.0.0.1", () => done({ server, stats, port: server.address().port })),
  );
}

function check(condition, message) {
  if (!condition) throw new Error(`check failed: ${message}`);
}

/** State of the viewer as the page sees it. */
function snapshot() {
  const viewer = document.getElementById("viewer");
  const root = viewer.shadowRoot;
  const first = root.querySelector('.page[data-page="1"]');
  const canvas = first?.querySelector("canvas");
  let inked = 0;
  let sharp = false;
  if (canvas) {
    const { data } = canvas.getContext("2d").getImageData(0, 0, canvas.width, canvas.height);
    for (let i = 0; i < data.length; i += 4) {
      // A painted pixel that is not white: text or picture.
      if (data[i + 3] === 255 && data[i] + data[i + 1] + data[i + 2] < 600) inked += 1;
    }
    // The top-left pixel is opaque once its tile has been painted.
    sharp = data[3] === 255;
  }
  const spans = first ? [...first.querySelectorAll(".text span")] : [];
  return {
    page: viewer.page,
    pageCount: viewer.pageCount,
    zoom: viewer.zoom,
    scale: viewer.scale,
    canvas: canvas ? [canvas.width, canvas.height] : null,
    inked,
    sharp,
    spans: spans.length,
    text: spans.slice(0, 40).map((s) => s.textContent).join(""),
    pages: [...root.querySelectorAll(".page")].map((p) => Number(p.dataset.page)),
    toolbar: root.querySelector(".count").textContent,
    message: root.querySelector(".message").textContent,
    events: window.events,
  };
}

async function run(chromium, ranges) {
  const label = ranges ? "range server" : "plain server";
  const { server, stats, port } = await startServer(ranges);
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1000, height: 800 } });
    const errors = [];
    page.on("pageerror", (err) => errors.push(String(err)));
    page.on("console", (msg) => msg.type() === "error" && errors.push(msg.text()));
    await page.goto(`http://127.0.0.1:${port}/`);

    // Wait until the first page has paint and a text layer, or an error shows.
    await page.waitForFunction(
      () => {
        const root = document.getElementById("viewer").shadowRoot;
        return (
          window.events.some((e) => e.type === "documenterror") ||
          (root.querySelector('.page[data-page="1"] .text span') &&
            root.querySelector('.page[data-page="1"] canvas'))
        );
      },
      null,
      { timeout: 60_000 },
    );
    // Let the tile queue drain: the state stops changing.
    let state;
    let previous = "";
    let stable = 0;
    for (let i = 0; i < 100; i++) {
      await page.waitForTimeout(150);
      state = await page.evaluate(snapshot);
      const key = `${state.inked}/${state.spans}/${state.pages}`;
      stable = state.inked > 0 && key === previous ? stable + 1 : 0;
      if (stable === 4) break;
      previous = key;
    }

    check(!state.events.some((e) => e.type === "documenterror"), `no error event (${state.message})`);
    check(state.events[0]?.type === "documentload", "documentload fires first");
    check(state.pageCount > 2, `a multi-page fixture (${state.pageCount} pages)`);
    check(state.toolbar === `/ ${state.pageCount}`, "the toolbar shows the page count");
    check(state.page === 1, "the viewer starts on page 1");
    check(state.zoom === "fit-width", "the default zoom is fit-width");
    check(state.canvas && state.canvas[0] > 500, `the page canvas has a size (${state.canvas})`);
    check(state.sharp, "the first tile is painted");
    check(state.inked > 1000, `the first page has non-blank pixels (${state.inked})`);
    check(state.spans > 20, `the text layer has words (${state.spans})`);
    check(/[A-Za-z]{3}/.test(state.text), `the text layer has text (${JSON.stringify(state.text.slice(0, 60))})`);
    check(state.pages.length < state.pageCount, "only the pages near the viewport are in the DOM");

    const first = state;

    // Page property: jump, then read the event.
    await page.evaluate(() => (document.getElementById("viewer").page = 3));
    await page.waitForFunction(() => window.events.at(-1)?.page === 3, null, { timeout: 30_000 });
    await page.waitForFunction(
      () => {
        const canvas = document
          .getElementById("viewer")
          .shadowRoot.querySelector('.page[data-page="3"] canvas');
        return canvas && canvas.getContext("2d").getImageData(0, 0, 1, 1).data[3] === 255;
      },
      null,
      { timeout: 60_000 },
    );

    // Zoom attribute: a percent sets the scale.
    await page.evaluate(() => document.getElementById("viewer").setAttribute("zoom", "150"));
    state = await page.evaluate(snapshot);
    check(Math.abs(state.scale - 1.5) < 1e-9, `zoom="150" gives scale 1.5 (${state.scale})`);
    check(state.events.some((e) => e.type === "zoomchange" && e.zoom === "150"), "zoomchange fires");
    check(state.page === 3, `a zoom change keeps the page (${state.page})`);

    // Keyboard: End goes to the last page, Home to the first.
    await page.locator("djvu-viewer").locator(".scroll").focus();
    await page.keyboard.press("End");
    await page.waitForFunction(
      () => document.getElementById("viewer").page === document.getElementById("viewer").pageCount,
      null,
      { timeout: 30_000 },
    );
    await page.keyboard.press("Home");
    await page.waitForFunction(() => document.getElementById("viewer").page === 1, null, {
      timeout: 30_000,
    });

    if (process.env.SMOKE_SCREENSHOT) {
      await page.waitForTimeout(1500);
      await page.screenshot({ path: `${process.env.SMOKE_SCREENSHOT}-${ranges ? "range" : "plain"}.png` });
    }
    check(errors.length === 0, `no page errors (${errors.join("; ")})`);
    if (ranges) {
      check(stats.rangeRequests > 1, "the document opens through range requests");
      check(
        stats.fixtureBytes < fixture.length,
        `a lazy open fetches part of the file (${stats.fixtureBytes} of ${fixture.length} bytes)`,
      );
    } else {
      check(stats.rangeRequests === 0, "no range request is answered");
    }
    console.log(
      `OK (${label}): ${first.pageCount} pages, ${first.inked} inked pixels on page 1, ` +
        `${first.spans} text zones, ${stats.fixtureBytes} of ${fixture.length} fixture bytes fetched`,
    );
  } finally {
    await browser.close();
    server.close();
  }
}

let playwright;
try {
  playwright = await import("playwright");
} catch {
  throw new Error(
    "playwright is not installed; run: npm install playwright && npx playwright install chromium",
  );
}
const chromium = playwright.chromium ?? playwright.default.chromium;
await run(chromium, true);
await run(chromium, false);
