// <djvu-viewer>: a custom element that shows a DjVu document.
//
//   <script type="module" src=".../viewer/djvu-viewer.js"></script>
//   <djvu-viewer src="book.djvu" controls></djvu-viewer>
//
// The element only lays pages out and paints pixels. A Web Worker
// (djvu-viewer-worker.js) owns the wasm module and decodes.

const CSS_DPI = 96; // CSS pixels per inch: zoom 100% shows a page at its paper size.
const TILE_SIZE = 512; // Device pixels.
const PAGE_GAP = 12; // CSS pixels between pages and around them.
const MAX_CANVAS_PIXELS = 12_000_000; // Per page; a deeper zoom stretches the canvas.
const MIN_SCALE = 0.1;
const MAX_SCALE = 8;
const ZOOM_STEPS = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 2, 3, 4];

const STYLE = `
:host {
  display: block;
  position: relative;
  height: 600px;
  color-scheme: light dark;
  --djvu-background: light-dark(#e9e9ec, #26262a);
  --djvu-page-background: #fff;
  --djvu-toolbar-background: light-dark(#f7f7f8, #36363b);
  --djvu-toolbar-color: light-dark(#1d1d1f, #f2f2f4);
  --djvu-selection: rgba(0, 100, 255, 0.3);
  font: 13px/1.3 system-ui, sans-serif;
}
:host([hidden]) { display: none; }
.root { position: absolute; inset: 0; display: flex; flex-direction: column; }
.toolbar {
  display: none; align-items: center; gap: 6px; padding: 4px 8px; flex: none;
  background: var(--djvu-toolbar-background); color: var(--djvu-toolbar-color);
  border-bottom: 1px solid rgba(128, 128, 128, 0.35);
}
:host([controls]) .toolbar { display: flex; }
.toolbar button, .toolbar input, .toolbar select { font: inherit; color: inherit; }
.toolbar button { min-width: 28px; height: 26px; cursor: pointer; }
.toolbar input { width: 4em; height: 20px; text-align: right; }
.toolbar select { height: 26px; }
.spacer { flex: 1; }
.scroll {
  position: relative; flex: 1; overflow: auto; outline: none;
  background: var(--djvu-background);
}
.sheet { position: relative; margin: 0 auto; }
.page {
  position: absolute; overflow: hidden;
  background: var(--djvu-page-background);
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.3);
}
.page canvas { position: absolute; left: 0; top: 0; width: 100%; height: 100%; }
.text {
  position: absolute; left: 0; top: 0; transform-origin: 0 0;
  color: transparent; line-height: 1; user-select: text; cursor: text;
}
.text span { position: absolute; white-space: pre; transform-origin: 0 0; }
.text ::selection { background: var(--djvu-selection); }
.message {
  position: absolute; inset: 0; display: none; align-items: center; justify-content: center;
  padding: 16px; text-align: center; color: var(--djvu-toolbar-color); pointer-events: none;
}
.message.shown { display: flex; }
`;

const TEMPLATE = `
<div class="root" part="root">
  <div class="toolbar" part="toolbar">
    <button class="prev" type="button" title="Previous page" aria-label="Previous page">‹</button>
    <input class="number" type="number" min="1" value="1" aria-label="Page">
    <span class="count">/ 0</span>
    <button class="next" type="button" title="Next page" aria-label="Next page">›</button>
    <span class="spacer"></span>
    <button class="out" type="button" title="Zoom out" aria-label="Zoom out">−</button>
    <select class="zoom" aria-label="Zoom">
      <option value="fit-width">Fit width</option>
      <option value="fit-page">Fit page</option>
      ${ZOOM_STEPS.map((s) => `<option value="${s * 100}">${s * 100}%</option>`).join("")}
      <option class="custom" value="" hidden></option>
    </select>
    <button class="in" type="button" title="Zoom in" aria-label="Zoom in">+</button>
  </div>
  <div class="scroll" part="viewport" tabindex="0"><div class="sheet"></div></div>
  <div class="message" part="message" role="status"></div>
</div>
`;

let measureContext;
/** Width of `text` at a 100px font, to stretch a span to its zone. */
function textWidth(text) {
  measureContext ??= document.createElement("canvas").getContext("2d");
  measureContext.font = "100px sans-serif";
  return measureContext.measureText(text).width;
}

function parseZoom(value) {
  if (value === "fit-page" || value === "fit-width") return value;
  const percent = Number.parseFloat(value);
  if (Number.isFinite(percent) && percent > 0) {
    return Math.min(MAX_SCALE, Math.max(MIN_SCALE, percent / 100));
  }
  return "fit-width";
}

export class DjVuViewer extends HTMLElement {
  static observedAttributes = ["src", "page", "zoom"];

  /** URL of the worker script. Set it before the first element opens a file
   *  when the worker is served from another place than this module. */
  static workerUrl = new URL("./djvu-viewer-worker.js", import.meta.url);
  /** URL of the `.wasm` file; by default the one next to the package entry. */
  static wasmUrl = undefined;

  #shadow = this.attachShadow({ mode: "open" });
  #scroll;
  #sheet;
  #message;
  #ui = {};

  #worker = null;
  #calls = new Map();
  #nextId = 1;

  #pages = []; // { width, height, dpi, chunks, known, top, cssWidth, cssHeight, zones, view }
  #zoom = "fit-width";
  #scale = 1;
  #page = 1; // 1-based current page
  #wantedPage = null; // page to show once the document is open
  #busy = false; // one tile request at a time
  #frame = 0;
  #connected = false;
  #kept = ""; // page indices the worker was last told to keep
  #resizeObserver = new ResizeObserver(() => this.#relayout());

  constructor() {
    super();
    this.#shadow.innerHTML = `<style>${STYLE}</style>${TEMPLATE}`;
    const $ = (selector) => this.#shadow.querySelector(selector);
    this.#scroll = $(".scroll");
    this.#sheet = $(".sheet");
    this.#message = $(".message");
    this.#ui = {
      prev: $(".prev"), next: $(".next"), number: $(".number"), count: $(".count"),
      out: $(".out"), in: $(".in"), zoom: $(".zoom"), custom: $(".custom"),
    };

    this.#scroll.addEventListener("scroll", () => this.#schedule(), { passive: true });
    this.#scroll.addEventListener("keydown", (event) => this.#onKey(event));
    this.#scroll.addEventListener("wheel", (event) => this.#onWheel(event), { passive: false });
    this.#ui.prev.addEventListener("click", () => (this.page = this.#page - 1));
    this.#ui.next.addEventListener("click", () => (this.page = this.#page + 1));
    this.#ui.number.addEventListener("change", () => (this.page = Number(this.#ui.number.value)));
    this.#ui.out.addEventListener("click", () => this.zoomOut());
    this.#ui.in.addEventListener("click", () => this.zoomIn());
    this.#ui.zoom.addEventListener("change", () => (this.zoom = this.#ui.zoom.value));
  }

  // ---- Public API -----------------------------------------------------------

  /** URL of the document. */
  get src() {
    return this.getAttribute("src") ?? "";
  }
  set src(value) {
    if (value == null) this.removeAttribute("src");
    else this.setAttribute("src", value);
  }

  /** Current page, counted from 1. Setting it scrolls to that page. */
  get page() {
    return this.#page;
  }
  set page(value) {
    const number = Math.round(Number(value));
    if (!Number.isFinite(number)) return;
    if (this.#pages.length === 0) {
      this.#wantedPage = number;
      return;
    }
    this.#scrollToPage(Math.min(this.#pages.length, Math.max(1, number)));
  }

  /** `"fit-width"`, `"fit-page"`, or a percent such as `"150"`. */
  get zoom() {
    return typeof this.#zoom === "string" ? this.#zoom : String(Math.round(this.#zoom * 100));
  }
  set zoom(value) {
    const zoom = parseZoom(String(value));
    if (zoom === this.#zoom) return;
    this.#zoom = zoom;
    this.#relayout();
  }

  /** Number of pages; 0 until a document is open. */
  get pageCount() {
    return this.#pages.length;
  }

  /** Current scale: 1 shows a page at its paper size. */
  get scale() {
    return this.#scale;
  }

  zoomIn() {
    this.zoom = (ZOOM_STEPS.find((s) => s > this.#scale * 1.01) ?? MAX_SCALE) * 100;
  }

  zoomOut() {
    this.zoom = (ZOOM_STEPS.findLast((s) => s < this.#scale * 0.99) ?? MIN_SCALE) * 100;
  }

  // ---- Lifecycle ------------------------------------------------------------

  connectedCallback() {
    this.#connected = true;
    this.#resizeObserver.observe(this.#scroll);
    if (this.src && !this.#worker) this.#open(this.src);
  }

  disconnectedCallback() {
    this.#connected = false;
    this.#resizeObserver.disconnect();
    this.#close();
  }

  attributeChangedCallback(name, old, value) {
    if (old === value) return;
    if (name === "src") {
      if (this.#connected) value ? this.#open(value) : this.#close();
    } else if (name === "page") {
      if (value !== null) this.page = value;
    } else if (name === "zoom") {
      this.zoom = value ?? "fit-width";
    }
  }

  // ---- Worker ---------------------------------------------------------------

  #call(type, args = {}) {
    const worker = this.#worker;
    return new Promise((resolve, reject) => {
      const id = this.#nextId++;
      this.#calls.set(id, { resolve, reject });
      worker.postMessage({ id, type, ...args });
    });
  }

  #close() {
    this.#worker?.terminate();
    this.#worker = null;
    this.#calls.clear();
    this.#busy = false;
    this.#kept = "";
    this.#pages = [];
    this.#sheet.replaceChildren();
    this.#sheet.style.height = "0";
    this.#updateToolbar();
  }

  async #open(src) {
    this.#close();
    this.#showMessage("Loading…");
    const worker = new Worker(DjVuViewer.workerUrl, { type: "module" });
    this.#worker = worker;
    worker.onmessage = ({ data }) => {
      if (worker !== this.#worker) return;
      const call = this.#calls.get(data.id);
      if (!call) return;
      this.#calls.delete(data.id);
      if (data.ok) call.resolve(data);
      else call.reject(new Error(data.error));
    };
    worker.onerror = (event) => this.#fail(worker, new Error(event.message || "worker failed"));

    try {
      const url = new URL(src, document.baseURI).href;
      const wasmUrl = DjVuViewer.wasmUrl ? String(DjVuViewer.wasmUrl) : undefined;
      const { pageCount, infos } = await this.#call("open", { url, wasmUrl });
      if (worker !== this.#worker) return;
      this.#pages = Array.from({ length: pageCount }, (_, index) => ({
        // An unknown page takes the first page's size until its own arrives.
        ...(infos[index] ?? infos[0]),
        known: index < infos.length,
        loading: false,
        top: 0, cssWidth: 0, cssHeight: 0,
        zones: undefined,
        view: null,
      }));
      this.#showMessage("");
      this.#page = 1;
      const start = this.#wantedPage ?? Number(this.getAttribute("page") ?? 1);
      this.#wantedPage = null;
      // Set the scale here, so that `documentload` is the first event.
      this.#scale = this.#computeScale();
      this.#relayout();
      if (start > 1) this.#scrollToPage(Math.min(pageCount, Math.max(1, Math.round(start) || 1)));
      else this.#scroll.scrollTop = 0;
      this.#updateToolbar();
      this.#emit("documentload", { pageCount });
      this.#emit("pagechange", { page: this.#page, pageCount });
    } catch (error) {
      this.#fail(worker, error);
    }
  }

  #fail(worker, error) {
    if (worker !== this.#worker) return;
    this.#close();
    this.#showMessage(`Cannot show this document: ${error.message}`);
    this.#emit("documenterror", { message: error.message });
  }

  #emit(type, detail) {
    this.dispatchEvent(new CustomEvent(type, { detail, bubbles: true, composed: true }));
  }

  #showMessage(text) {
    this.#message.textContent = text;
    this.#message.classList.toggle("shown", text !== "");
  }

  // ---- Layout ---------------------------------------------------------------

  /** Scale for the current zoom mode and viewport. */
  #computeScale() {
    if (typeof this.#zoom === "number") return this.#zoom;
    const availableWidth = Math.max(50, this.#scroll.clientWidth - 2 * PAGE_GAP);
    const current = this.#pages[this.#page - 1];
    let widest = 0;
    for (const page of this.#pages) widest = Math.max(widest, page.width / page.dpi);
    let scale = availableWidth / (widest * CSS_DPI);
    if (this.#zoom === "fit-page") {
      const availableHeight = Math.max(50, this.#scroll.clientHeight - 2 * PAGE_GAP);
      scale = Math.min(scale, availableHeight / ((current.height / current.dpi) * CSS_DPI));
    }
    return Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale));
  }

  /** Place every page for the current scale; keep the reading position. */
  #relayout() {
    if (this.#pages.length === 0 || this.#scroll.clientWidth === 0) return;
    // Remember where the top of the viewport is inside its page.
    const anchor = this.#pageAt(this.#scroll.scrollTop);
    const anchorPage = this.#pages[anchor];
    const within = anchorPage.cssHeight
      ? (this.#scroll.scrollTop - anchorPage.top) / anchorPage.cssHeight
      : 0;

    // And where its middle is across the sheet; a sheet that fits is centred.
    const scroll = this.#scroll;
    const across = scroll.scrollWidth > scroll.clientWidth
      ? (scroll.scrollLeft + scroll.clientWidth / 2) / scroll.scrollWidth
      : 0.5;

    const oldScale = this.#scale;
    this.#scale = this.#computeScale();
    let top = PAGE_GAP;
    let widest = 0;
    for (const page of this.#pages) {
      page.cssWidth = (page.width / page.dpi) * CSS_DPI * this.#scale;
      page.cssHeight = (page.height / page.dpi) * CSS_DPI * this.#scale;
      page.top = top;
      top += page.cssHeight + PAGE_GAP;
      widest = Math.max(widest, page.cssWidth);
    }
    this.#sheet.style.height = `${top}px`;
    this.#sheet.style.width = `${widest + 2 * PAGE_GAP}px`;
    for (const page of this.#pages) if (page.view) this.#placeView(page);

    this.#scroll.scrollTop = anchorPage.top + within * anchorPage.cssHeight;
    scroll.scrollLeft = across * scroll.scrollWidth - scroll.clientWidth / 2;
    if (oldScale !== this.#scale) {
      this.#updateToolbar();
      this.#emit("zoomchange", { zoom: this.zoom, scale: this.#scale });
    }
    this.#update();
  }

  /** Index of the page at vertical position `y` of the sheet. */
  #pageAt(y) {
    let low = 0;
    let high = this.#pages.length - 1;
    while (low < high) {
      const middle = (low + high) >> 1;
      const page = this.#pages[middle];
      if (page.top + page.cssHeight + PAGE_GAP <= y) low = middle + 1;
      else high = middle;
    }
    return low;
  }

  #scrollToPage(number) {
    this.#scroll.scrollTop = this.#pages[number - 1].top - PAGE_GAP;
    this.#update();
  }

  #schedule() {
    if (this.#frame) return;
    this.#frame = requestAnimationFrame(() => {
      this.#frame = 0;
      this.#update();
    });
  }

  /** Bring the DOM in line with the scroll position, then ask for tiles. */
  #update() {
    if (this.#pages.length === 0) return;
    const top = this.#scroll.scrollTop;
    const height = this.#scroll.clientHeight;

    const current = this.#pageAt(top + height / 2) + 1;
    if (current !== this.#page) {
      this.#page = current;
      this.#updateToolbar();
      this.#emit("pagechange", { page: current, pageCount: this.#pages.length });
      // Fit-page depends on the current page's size.
      if (this.#zoom === "fit-page" && this.#computeScale() !== this.#scale) {
        this.#relayout();
        return;
      }
    }

    // Pages within one screen of the viewport have a DOM node.
    const first = this.#pageAt(top - height);
    const last = this.#pageAt(top + 2 * height);
    const keep = [];
    this.#pages.forEach((page, index) => {
      const near = index >= first && index <= last;
      if (near) {
        keep.push(index);
        if (!page.view) this.#createView(page, index);
        if (!page.known && !page.loading) this.#loadInfo(page, index);
      } else if (page.view) {
        page.view.element.remove();
        page.view = null;
      }
    });
    const kept = keep.join();
    if (kept !== this.#kept) {
      this.#kept = kept;
      this.#call("keep", { indices: keep }).catch(() => {});
    }
    this.#pump();
  }

  async #loadInfo(page, index) {
    const worker = this.#worker;
    page.loading = true;
    try {
      const info = await this.#call("info", { index });
      if (worker !== this.#worker) return;
      const changed = info.width !== page.width || info.height !== page.height || info.dpi !== page.dpi;
      Object.assign(page, { width: info.width, height: info.height, dpi: info.dpi, chunks: info.chunks, known: true });
      if (changed) this.#relayout();
      else this.#pump();
    } catch (error) {
      this.#fail(worker, error);
    }
  }

  // ---- Page views -----------------------------------------------------------

  #createView(page, index) {
    const element = document.createElement("div");
    element.className = "page";
    element.part = "page";
    element.dataset.page = String(index + 1);
    page.view = { element, layer: null, stale: [], text: null };
    this.#sheet.append(element);
    this.#placeView(page);
  }

  #placeView(page) {
    const { element, text } = page.view;
    const sheetWidth = Number.parseFloat(this.#sheet.style.width);
    element.style.left = `${(sheetWidth - page.cssWidth) / 2}px`;
    element.style.top = `${page.top}px`;
    element.style.width = `${page.cssWidth}px`;
    element.style.height = `${page.cssHeight}px`;
    if (text) text.style.transform = `scale(${page.cssWidth / page.width})`;
  }

  /** Render resolution for a page: device pixels, capped by canvas size. */
  #renderDpi(page) {
    const wanted = CSS_DPI * this.#scale * (window.devicePixelRatio || 1);
    const inches = (page.width / page.dpi) * (page.height / page.dpi);
    const cap = Math.sqrt(MAX_CANVAS_PIXELS / inches);
    return Math.max(8, Math.round(Math.min(wanted, cap)));
  }

  /** The next piece of work for the pages near the viewport, or null. */
  #nextJob() {
    const top = this.#scroll.scrollTop;
    const bottom = top + this.#scroll.clientHeight;
    const margin = this.#scroll.clientHeight / 2;
    let refine = null;

    for (let index = this.#pageAt(top - margin); index < this.#pages.length; index++) {
      const page = this.#pages[index];
      if (page.top > bottom + margin) break;
      if (!page.view || !page.known) continue;

      const dpi = this.#renderDpi(page);
      let layer = page.view.layer;
      if (!layer || layer.dpi !== dpi) return { kind: "grid", page, index, dpi };
      if (page.zones === undefined) return { kind: "text", page, index };

      // Rows and columns of the grid that the viewport, plus a margin, covers.
      const toDevice = layer.height / page.cssHeight;
      const tile = TILE_SIZE;
      const firstRow = Math.max(0, Math.floor(((top - margin - page.top) * toDevice) / tile));
      const lastRow = Math.min(layer.rows - 1, Math.floor(((bottom + margin - page.top) * toDevice) / tile));
      for (let row = firstRow; row <= lastRow; row++) {
        for (let col = 0; col < layer.cols; col++) {
          const state = layer.tiles[row * layer.cols + col];
          if (state === 0) {
            // A page with a background image gets a quick coarse tile first.
            const step = page.chunks > 1 ? 0 : null;
            return { kind: "tile", page, index, layer, col, row, step };
          }
          if (state === 1 && !refine) {
            refine = { kind: "tile", page, index, layer, col, row, step: null };
          }
        }
      }
    }
    return refine;
  }

  /** Run one job at a time: coarse tiles of the visible pages come first. */
  async #pump() {
    if (this.#busy || !this.#worker) return;
    const job = this.#nextJob();
    if (!job) {
      // Everything on screen is sharp: drop the canvases of the old zoom.
      for (const page of this.#pages) {
        if (page.view?.stale.length) {
          for (const canvas of page.view.stale) canvas.remove();
          page.view.stale = [];
        }
      }
      return;
    }
    const worker = this.#worker;
    this.#busy = true;
    try {
      await this.#run(job, worker);
    } catch (error) {
      this.#fail(worker, error);
      return;
    }
    if (worker !== this.#worker) return;
    this.#busy = false;
    this.#pump();
  }

  async #run(job, worker) {
    const { page, index } = job;
    if (job.kind === "grid") {
      const grid = await this.#call("grid", { index, dpi: job.dpi, tileSize: TILE_SIZE });
      if (worker !== this.#worker || !page.view) return;
      const canvas = document.createElement("canvas");
      canvas.width = grid.width;
      canvas.height = grid.height;
      // The canvas of the old zoom stays below, stretched, until this one fills.
      if (page.view.layer) page.view.stale.push(page.view.layer.canvas);
      page.view.element.insertBefore(canvas, page.view.text);
      page.view.layer = {
        dpi: job.dpi, canvas, context: canvas.getContext("2d"),
        width: grid.width, height: grid.height, cols: grid.cols, rows: grid.rows,
        tiles: new Uint8Array(grid.cols * grid.rows), // 0 empty, 1 coarse, 2 sharp
      };
    } else if (job.kind === "text") {
      const { zones } = await this.#call("text", { index, dpi: page.dpi });
      if (worker !== this.#worker) return;
      page.zones = zones ? JSON.parse(zones) : null;
      if (page.view) this.#buildText(page);
    } else {
      const { layer, col, row, step } = job;
      const tile = await this.#call("tile", {
        index, dpi: layer.dpi, tileSize: TILE_SIZE, col, row, step,
      });
      if (worker !== this.#worker) return;
      const pixels = new Uint8ClampedArray(tile.pixels);
      layer.context.putImageData(
        new ImageData(pixels, tile.width, tile.height), col * TILE_SIZE, row * TILE_SIZE,
      );
      layer.tiles[row * layer.cols + col] = step === null ? 2 : 1;
    }
  }

  /** Invisible, selectable text over the page image. */
  #buildText(page) {
    const view = page.view;
    view.text?.remove();
    view.text = null;
    if (!page.zones?.length) return;
    const layer = document.createElement("div");
    layer.className = "text";
    layer.part = "text-layer";
    layer.style.width = `${page.width}px`;
    layer.style.height = `${page.height}px`;
    layer.style.transform = `scale(${page.cssWidth / page.width})`;
    // Some files give every word the box of its whole line. Such words
    // become one piece of text that fills the box once.
    const zones = [];
    for (const zone of page.zones) {
      const last = zones.at(-1);
      if (last && last.x === zone.x && last.y === zone.y && last.w === zone.w && last.h === zone.h) {
        last.t += ` ${zone.t}`;
      } else {
        zones.push({ ...zone });
      }
    }
    let previous = null;
    for (const zone of zones) {
      const span = document.createElement("span");
      // A zone that starts above or left of the end of the previous one opens
      // a new line; a copied selection then keeps the line breaks.
      const sameLine = previous && Math.abs(zone.y - previous.y) < previous.h / 2;
      if (previous && !sameLine) layer.append("\n");
      span.textContent = `${zone.t} `;
      span.style.left = `${zone.x}px`;
      span.style.top = `${zone.y}px`;
      span.style.fontSize = `${zone.h}px`;
      span.style.fontFamily = "sans-serif";
      const natural = (textWidth(zone.t) * zone.h) / 100;
      if (natural > 0) span.style.transform = `scaleX(${zone.w / natural})`;
      layer.append(span);
      previous = zone;
    }
    view.element.append(layer);
    view.text = layer;
  }

  // ---- Input ----------------------------------------------------------------

  #onKey(event) {
    if (event.altKey || event.metaKey) return;
    const key = event.key;
    if (event.ctrlKey) {
      if (key === "+" || key === "=") this.zoomIn();
      else if (key === "-") this.zoomOut();
      else if (key === "0") this.zoom = "fit-width";
      else return;
    } else if (key === "ArrowRight" || key === "n") this.page = this.#page + 1;
    else if (key === "ArrowLeft" || key === "p") this.page = this.#page - 1;
    else if (key === "Home") this.page = 1;
    else if (key === "End") this.page = this.#pages.length;
    else if (key === "+" || key === "=") this.zoomIn();
    else if (key === "-") this.zoomOut();
    else return; // Arrows up/down, PageUp/PageDown and Space scroll natively.
    event.preventDefault();
  }

  #onWheel(event) {
    if (!event.ctrlKey) return; // Ctrl + wheel, and a trackpad pinch, zoom.
    event.preventDefault();
    const scale = this.#scale * Math.exp(-event.deltaY / 300);
    this.zoom = Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale)) * 100;
  }

  #updateToolbar() {
    const count = this.#pages.length;
    const ui = this.#ui;
    ui.number.value = String(count ? this.#page : 1);
    ui.number.max = String(Math.max(1, count));
    ui.count.textContent = `/ ${count}`;
    ui.prev.disabled = !count || this.#page <= 1;
    ui.next.disabled = !count || this.#page >= count;
    const zoom = this.zoom;
    if (![...ui.zoom.options].some((option) => option !== ui.custom && option.value === zoom)) {
      ui.custom.value = zoom;
      ui.custom.textContent = `${zoom}%`;
      ui.custom.hidden = false;
    }
    ui.zoom.value = zoom;
  }
}

if (!customElements.get("djvu-viewer")) {
  customElements.define("djvu-viewer", DjVuViewer);
}
