// Worker half of <djvu-viewer>: it owns the wasm module and the document, and
// answers one request at a time. The page never decodes anything itself.
//
// Request:  { id, type, ...args }
// Response: { id, ok: true, ...result } or { id, ok: false, error }

import init, * as djvu from "../djvu_rs.js";

let doc = null;
let lazy = false;
// Page handles, by page index. A lazy page is fetched on first use, so the map
// holds promises: two requests for one page share the fetch.
const pages = new Map();

function getPage(index) {
  let page = pages.get(index);
  if (!page) {
    page = Promise.resolve(doc.page(index));
    pages.set(index, page);
    page.catch(() => pages.delete(index));
  }
  return page;
}

function pageInfo(page) {
  const dpi = page.dpi();
  return {
    width: page.width_at(dpi),
    height: page.height_at(dpi),
    dpi,
    chunks: page.bg44_chunk_count(),
  };
}

async function rangeFetch(url, offset, length) {
  const response = await fetch(url, {
    headers: { Range: `bytes=${offset}-${offset + length - 1}` },
  });
  if (response.status !== 206) {
    throw new Error(`range request failed: HTTP ${response.status}`);
  }
  return new Uint8Array(await response.arrayBuffer());
}

async function open({ url, wasmUrl }) {
  await init(wasmUrl);

  // Ask for one byte. A 206 answer with the total length means that the
  // server supports ranges: open lazily and fetch each page on demand.
  // A 200 answer already carries the whole file.
  const probe = await fetch(url, { headers: { Range: "bytes=0-0" } });
  if (!probe.ok) {
    throw new Error(`cannot load ${url}: HTTP ${probe.status}`);
  }
  let total = null;
  if (probe.status === 206) {
    const match = /\/(\d+)\s*$/.exec(probe.headers.get("Content-Range") ?? "");
    if (match) total = Number(match[1]);
  }
  if (total !== null && djvu.WasmLazyDocument) {
    try {
      doc = await djvu.WasmLazyDocument.open(total, (offset, length) =>
        rangeFetch(url, offset, length),
      );
      lazy = true;
    } catch {
      doc = null; // Not a file the lazy reader takes: load it whole.
    }
  }
  if (!doc) {
    const response = probe.status === 200 ? probe : await fetch(url);
    if (!response.ok) {
      throw new Error(`cannot load ${url}: HTTP ${response.status}`);
    }
    doc = djvu.WasmDocument.from_bytes(new Uint8Array(await response.arrayBuffer()));
    lazy = false;
  }

  const pageCount = doc.page_count();
  if (pageCount === 0) throw new Error("the document has no pages");
  // A whole file gives every page size at once. A lazy one gives the first
  // page only; the viewer asks for the others as they come near the screen.
  const infos = [];
  const known = lazy ? 1 : pageCount;
  for (let index = 0; index < known; index++) {
    const page = await doc.page(index);
    infos.push(pageInfo(page));
    page.free();
  }
  return { pageCount, lazy, infos };
}

const handlers = {
  open,

  async info({ index }) {
    return pageInfo(await getPage(index));
  },

  // Canvas size and tile grid of a page at `dpi`.
  async grid({ index, dpi, tileSize }) {
    const page = await getPage(index);
    return {
      width: page.width_at(dpi),
      height: page.height_at(dpi),
      cols: page.tile_cols(dpi, tileSize),
      rows: page.tile_rows(dpi, tileSize),
    };
  },

  // One tile as RGBA bytes. `step` null is full quality; a number is a
  // coarse render from the first `step + 1` background chunks.
  async tile({ index, dpi, tileSize, col, row, step }) {
    const page = await getPage(index);
    const pixmap =
      step === null
        ? page.render_tile(dpi, tileSize, col, row)
        : page.render_tile_progressive(dpi, tileSize, col, row, step);
    try {
      const pixels = pixmap.to_bytes();
      return {
        width: pixmap.width(),
        height: pixmap.height(),
        pixels: pixels.buffer,
        transfer: [pixels.buffer],
      };
    } finally {
      pixmap.free();
    }
  },

  // Text zones in the page's own pixels, as a JSON string, or null.
  async text({ index, dpi }) {
    const page = await getPage(index);
    return { zones: page.text_zones_json(dpi) ?? null };
  },

  // Drop the handles of the pages that left the screen.
  async keep({ indices }) {
    const wanted = new Set(indices);
    for (const [index, page] of pages) {
      if (!wanted.has(index)) {
        pages.delete(index);
        page.then((handle) => handle.free()).catch(() => {});
      }
    }
    return {};
  },
};

self.onmessage = async ({ data }) => {
  const { id, type, ...args } = data;
  try {
    const { transfer, ...result } = await handlers[type](args);
    self.postMessage({ id, ok: true, ...result }, transfer ?? []);
  } catch (error) {
    self.postMessage({ id, ok: false, error: String(error?.message ?? error) });
  }
};
