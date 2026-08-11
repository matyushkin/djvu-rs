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

## TIFF (slice 2, `tiff` feature)

| Source | Internal RGBA | Notes |
|--------|---------------|-------|
| Gray8, GrayA8, RGB8, RGBA8 | PNG-like expansion, A=255 unless present | any compression the `tiff` crate decodes (LZW, Deflate, PackBits, JPEG-in-TIFF) |
| Gray16, GrayA16, RGB16, RGBA16 | channels `>>8` | same [`DepthDownconversion::TruncateHighByte`](../src/ingest.rs) policy as PNG |
| CMYK8 / CMYK16 | naive transform (below), A=255 | 16-bit downconverted first |
| Bilevel (1-bit) WhiteIsZero / BlackIsZero | 0/255 gray, A=255 | **uncompressed strips only**; raw strip reader (tiff 0.9 misdecodes sub-byte samples) |
| Gray 2/4-bit | linear scale to 0..255 | uncompressed strips only |
| Palette 1/2/4/8-bit | ColorMap 16-bit entries `>>8` | uncompressed strips only |
| Multipage | one `Pixmap` per IFD, stored order | [`decode_tiff_file_to_pixmaps`](../src/png_io.rs); CLI `encode` maps a multipage file to a multi-page bundle |
| YCbCr, planar, tiled bilevel/palette, CCITT G3/G4, FillOrder 2 | — | targeted error naming the limitation |

### CMYK → RGB transform

Deterministic, profile-free: `channel = (255 − ink) · (255 − K) / 255` per
channel, alpha 255. No ICC transform is applied (see below); archival CMYK
with embedded profiles renders approximately.

### Multipage TIFF in the CLI

`djvu encode book.tif -o book.djvu` with a multipage file produces a
multi-page bundle under the same rules as a directory input: `--quality auto`
classifies every page (all bilevel → lossless JB2 bundle, else layered),
`--shared-dict-pages` and `--thumbnails` apply, `--bilevel-codec smmr` is
rejected. Page order is stored IFD order. DPI comes from `--dpi`;
X/YResolution tags are ignored (follow-up below).

## Orientation

EXIF/TIFF orientation tags are **not** applied at ingest in slice 1. Pages are
decoded in stored raster order; rotation belongs to a later slice.

## Planned follow-ups (#694)

- Compressed bilevel TIFF (CCITT G4 / PackBits) — needs a G4 decoder or crate support
- Bilevel TIFF fast path without RGBA expansion
- TIFF X/YResolution → default DPI mapping
- CMYK JPEG input
- EXIF/TIFF orientation applied exactly once
- Configurable alpha compositing CLI flag
- Explicit ICC preserve/reject/transform modes
