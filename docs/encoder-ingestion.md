# Encoder ingestion and color-management policy

Issue [#694](https://github.com/matyushkin/djvu-rs/issues/694) tracks
normalizing archival image inputs into the internal RGBA [`Pixmap`] consumed by
segmentation and encoding. This document is the supported-input matrix; behaviour
is deterministic and tested.

## Internal representation

All raster ingest paths target **8-bit RGBA** (`djvu_rs::Pixmap`: width × height ×
4 bytes, row-major). Alpha is preserved unless an explicit compositing policy is
selected (future CLI flag).

## PNG (slice 1)

| Source | Bit depth | Internal RGBA | Notes |
|--------|-----------|---------------|-------|
| Grayscale | 1/2/4/8 | R=G=B=sample, A=255 | Low bit depths expanded by the PNG decoder |
| Grayscale + alpha | 8 | R=G=B=gray, A=alpha | |
| RGB | 8 | R,G,B, A=255 | |
| RGBA | 8 | passthrough | |
| Palette (PLTE) | 1/2/4/8 index | palette RGB + alpha | `tRNS` per-entry alpha when present, else 255 |
| Grayscale | 16 | R=G=B=`sample>>8`, A=255 | Truncate high byte, no dithering |
| Grayscale + alpha | 16 | R=G=B=`sample>>8`, A=`sample>>8` | |
| RGB / RGBA | 16 | channels `>>8` | Big-endian samples |

### 16-bit down-conversion

Policy: [`DepthDownconversion::TruncateHighByte`](../src/ingest.rs) — each
16-bit channel keeps its **high byte** (equivalent to `u16 >> 8` for PNG's
big-endian layout). No dithering; identical results on all targets.

### Alpha at ingest

Default: [`AlphaCompositing::Preserve`](../src/ingest.rs). Partial transparency
is **not** flattened onto an undocumented background during file decode.

### ICC / color profiles

Embedded PNG `iCCP` and similar metadata are **ignored** at ingest in slice 1.
No silent color-space conversion is applied; sRGB-ish bytes are passed through.
Future slices may add explicit ICC handling or rejection.

## JPEG (current)

| Source | Internal RGBA | Notes |
|--------|---------------|-------|
| RGB (baseline/progressive) | R,G,B, A=255 | via `zune-jpeg` |
| CMYK | — | **not yet supported** (#694) |

## TIFF (current, `tiff` feature)

| Source | Internal RGBA | Notes |
|--------|---------------|-------|
| Gray8, GrayA8, RGB8, RGBA8 | see PNG-like expansion | first page only |
| Bilevel / palette / 16-bit / CMYK / multipage | — | **not yet supported** (#694) |

## Orientation

EXIF/TIFF orientation tags are **not** applied at ingest in slice 1. Pages are
decoded in stored raster order; rotation belongs to a later slice.

## Planned follow-ups (#694)

- Multipage TIFF page selection and metadata mapping
- Bilevel TIFF fast path without RGBA expansion
- CMYK → RGB transform with documented profile policy
- EXIF/TIFF orientation applied exactly once
- Configurable alpha compositing CLI flag
- Explicit ICC preserve/reject/transform modes
