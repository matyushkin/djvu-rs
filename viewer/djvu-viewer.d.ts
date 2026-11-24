/** Detail of the `documentload` event. */
export interface DjVuDocumentLoadDetail {
  pageCount: number;
}
/** Detail of the `pagechange` event. `page` counts from 1. */
export interface DjVuPageChangeDetail {
  page: number;
  pageCount: number;
}
/** Detail of the `zoomchange` event. `scale` 1 shows a page at its paper size. */
export interface DjVuZoomChangeDetail {
  zoom: string;
  scale: number;
}
/** Detail of the `documenterror` event. */
export interface DjVuDocumentErrorDetail {
  message: string;
}

export interface DjVuViewerEventMap extends HTMLElementEventMap {
  documentload: CustomEvent<DjVuDocumentLoadDetail>;
  pagechange: CustomEvent<DjVuPageChangeDetail>;
  zoomchange: CustomEvent<DjVuZoomChangeDetail>;
  documenterror: CustomEvent<DjVuDocumentErrorDetail>;
}

/**
 * `<djvu-viewer src="book.djvu" controls>`: a scrolling DjVu viewer.
 *
 * Attributes: `src`, `page` (from 1), `zoom` (`fit-width`, `fit-page` or a
 * percent), `controls` (show the toolbar). Decoding runs in a Web Worker.
 */
export class DjVuViewer extends HTMLElement {
  /** URL of the worker script; the file next to this module by default. */
  static workerUrl: URL | string;
  /** URL of the `.wasm` file; the one inside the package by default. */
  static wasmUrl: URL | string | undefined;

  /** URL of the document. */
  src: string;
  /** Current page, counted from 1. Setting it scrolls to that page. */
  page: number;
  /** `"fit-width"`, `"fit-page"`, or a percent such as `"150"`. */
  zoom: string;
  /** Number of pages; 0 until a document is open. */
  readonly pageCount: number;
  /** Current scale: 1 shows a page at its paper size. */
  readonly scale: number;

  zoomIn(): void;
  zoomOut(): void;

  addEventListener<K extends keyof DjVuViewerEventMap>(
    type: K,
    listener: (this: DjVuViewer, event: DjVuViewerEventMap[K]) => unknown,
    options?: boolean | AddEventListenerOptions,
  ): void;
  addEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject,
    options?: boolean | AddEventListenerOptions,
  ): void;
  removeEventListener<K extends keyof DjVuViewerEventMap>(
    type: K,
    listener: (this: DjVuViewer, event: DjVuViewerEventMap[K]) => unknown,
    options?: boolean | EventListenerOptions,
  ): void;
  removeEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject,
    options?: boolean | EventListenerOptions,
  ): void;
}

declare global {
  interface HTMLElementTagNameMap {
    "djvu-viewer": DjVuViewer;
  }
}
