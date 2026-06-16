//! Shared document-to-export traversal primitives (#345).
//!
//! The PDF, EPUB, TIFF, and OCR exporters share one domain operation — walk a
//! document, render each page at a target size, and project the text layer —
//! that previously existed only as copy-paste across four modules. This module
//! is the single home for those pieces so each exporter is a thin map over a
//! common traversal:
//!
//!  * [`page_indices`] — the per-page loop, with optional single-page
//!    selection (the hOCR/ALTO/EPUB/TIFF loops; the PDF loop keeps its own
//!    rayon parallel form because it needs an indexed range).
//!  * [`scaled_size`] — scale → pixel size with the shared `.max(1)` clamp.
//!  * [`word_spans`] — the leaf Word/Character descent the PDF and EPUB text
//!    layers both walk; callers map over it in their own coordinate units.
//!  * [`flip_y_bottom`] — the top-left-origin → bottom-origin vertical flip
//!    that PDF and EPUB both apply (OCR/hOCR/ALTO emit top-left as-is).
//!
//! Streaming-eligibility is *not* here: it lives on
//! [`RenderOptions::can_stream`](crate::djvu_render::RenderOptions::can_stream)
//! in the render module, where callers ask instead of re-deriving the rule.

use crate::djvu_document::{DjVuDocument, DjVuPage};
use crate::text::{Rect, TextLayer, TextZone, TextZoneKind};

/// Iterate the page indices to export: just page `only` when `Some`, otherwise
/// every page `0..page_count`.
///
/// Centralizes the `Box<dyn Iterator>` range the hOCR and ALTO writers
/// duplicated verbatim; the EPUB and TIFF whole-document loops use it too.
pub(crate) fn page_indices(
    doc: &DjVuDocument,
    only: Option<usize>,
) -> Box<dyn Iterator<Item = usize>> {
    match only {
        Some(i) => Box::new(core::iter::once(i)),
        None => Box::new(0..doc.page_count()),
    }
}

/// Scale `(w, h)` by `scale`, rounding to the nearest pixel and clamping each
/// dimension to at least 1.
///
/// This is the size kernel every raster exporter shares: PDF derives `scale`
/// from a capped DPI ratio, EPUB from an uncapped DPI ratio, and TIFF passes a
/// direct multiplier — but all three then round-and-clamp identically.
pub(crate) fn scaled_size(w: u32, h: u32, scale: f32) -> (u32, u32) {
    let sw = ((w as f32 * scale).round() as u32).max(1);
    let sh = ((h as f32 * scale).round() as u32).max(1);
    (sw, sh)
}

/// Pixel scale factor to render `page` at `target_dpi`, relative to the page's
/// native DPI.
///
/// This is the first half of the export sizing idiom that every raster exporter
/// and binding re-derived: `target_dpi / page.dpi()`. The page DPI is clamped to
/// at least 1 so a degenerate `0`-DPI INFO chunk can never produce a non-finite
/// scale.
pub(crate) fn scale_at_dpi(page: &DjVuPage, target_dpi: f32) -> f32 {
    target_dpi / page.dpi().max(1) as f32
}

/// Pixel `(width, height)` of `page` rendered at `target_dpi`.
///
/// The whole sizing idiom in one place: derive the scale from the two DPIs, then
/// round-and-clamp via [`scaled_size`]. Exporters that also need the render-time
/// `scale` (to populate [`RenderOptions`](crate::djvu_render::RenderOptions))
/// pair this with [`scale_at_dpi`].
pub(crate) fn size_at_dpi(page: &DjVuPage, target_dpi: f32) -> (u32, u32) {
    scaled_size(
        page.width() as u32,
        page.height() as u32,
        scale_at_dpi(page, target_dpi),
    )
}

/// Append the RGB bytes of one RGBA scanline to `dst`, dropping the alpha.
///
/// The per-row strip both the PDF and TIFF streaming encoders build from their
/// [`render_streaming`](crate::djvu_render::render_streaming) callback.
pub(crate) fn rgba_row_to_rgb(dst: &mut Vec<u8>, rgba_row: &[u8]) {
    for rgba in rgba_row.chunks_exact(4) {
        dst.extend_from_slice(&rgba[..3]);
    }
}

/// Render `page` at `opts` and hand each RGBA scanline to `row_cb`, streaming
/// straight from the compositor when [`RenderOptions::can_stream`] allows it and
/// falling back to a fully-allocated pixmap (walked row by row) otherwise.
///
/// This is the single home for the "stream when streamable, else render a pixmap
/// and walk its rows" decision the raster exporters share. PDF and EPUB both
/// build a full buffer from the rows; TIFF keeps its own strip-incremental
/// encoder loop (it streams into the TIFF writer, not into a `Vec`), so it does
/// not route through here. Centralizing it means a streamable page can never be
/// silently full-allocated in one exporter but streamed in another.
///
/// `#[allow(dead_code)]`: only the `pdf`/`epub`-gated exporters call it, so
/// builds without those features compile it unused.
///
/// [`RenderOptions::can_stream`](crate::djvu_render::RenderOptions::can_stream)
#[allow(dead_code)]
pub(crate) fn render_rows_or_pixmap(
    page: &DjVuPage,
    opts: &crate::djvu_render::RenderOptions,
    mut row_cb: impl FnMut(&[u8]),
) -> Result<(), crate::djvu_render::RenderError> {
    if opts.can_stream(page) {
        crate::djvu_render::render_streaming(page, opts, |_, rgba_row| row_cb(rgba_row))
    } else {
        let pixmap = crate::djvu_render::render_pixmap(page, opts)?;
        let stride = pixmap.width as usize * 4;
        if stride > 0 {
            for row in pixmap.data.chunks_exact(stride) {
                row_cb(row);
            }
        }
        Ok(())
    }
}

/// Parse a DjVu bookmark URL to a 0-based target page index, if it points
/// inside the document.
///
/// Handles the union of internal-link spellings the PDF and EPUB outline writers
/// each parsed on their own — and disagreed on: `#page=N`, `#page_N`, `#pageN`
/// (all 1-based), a bare `#N`, and the `#+N` / `#-N` relative forms (resolved as
/// 1-based absolute, matching the long-standing PDF behavior). Returns `None`
/// for external URLs, bare anchors, and anything unparseable, leaving the caller
/// to decide how those render.
///
/// `#[allow(dead_code)]`: only the `pdf`/`epub`-gated exporters call it.
#[allow(dead_code)]
pub(crate) fn bookmark_page_index(url: &str) -> Option<usize> {
    let frag = url.strip_prefix('#')?;
    if let Some(rest) = frag.strip_prefix("page") {
        let digits = rest
            .strip_prefix('=')
            .or_else(|| rest.strip_prefix('_'))
            .unwrap_or(rest)
            .trim();
        if let Ok(n) = digits.parse::<usize>() {
            return Some(n.saturating_sub(1));
        }
    }
    if let Ok(n) = frag.trim().parse::<i64>() {
        return Some((n.max(1) - 1) as usize);
    }
    None
}

/// A leaf text span: a `Word` or `Character` zone with non-empty text.
///
/// Borrows from the source [`TextLayer`]; `rect` is in top-left-origin pixels
/// (see [`Rect`]). Callers apply their own unit conversion and vertical flip.
pub(crate) struct WordSpan<'a> {
    pub rect: &'a Rect,
    pub text: &'a str,
}

/// Collect a text layer's leaf word/character spans in document order, skipping
/// zones with empty text.
///
/// Mirrors the descent the PDF (`build_text_content`) and EPUB
/// (`build_text_overlay`) paths each open-coded: recurse through structural
/// zones (Page/Column/Region/Para/Line) and stop at the first Word/Character
/// zone, emitting it without descending into its children. An empty-text Word
/// therefore drops any Character children — exactly as both exporters did.
///
/// The hOCR/ALTO writers deliberately do *not* use this: they emit the full
/// zone hierarchy (block/par/line/word), a structurally different traversal.
pub(crate) fn word_spans(layer: &TextLayer) -> Vec<WordSpan<'_>> {
    fn walk<'a>(zones: &'a [TextZone], out: &mut Vec<WordSpan<'a>>) {
        for zone in zones {
            match zone.kind {
                TextZoneKind::Word | TextZoneKind::Character => {
                    if !zone.text.is_empty() {
                        out.push(WordSpan {
                            rect: &zone.rect,
                            text: &zone.text,
                        });
                    }
                }
                _ => walk(&zone.children, out),
            }
        }
    }
    let mut out = Vec::new();
    walk(&layer.zones, &mut out);
    out
}

/// Vertical flip for a top-left-origin `rect` on a `total_h`-tall page: the gap
/// from the page bottom to the rect's bottom edge,
/// `total_h - (rect.y + rect.height)` (saturating at 0), in **pixels**.
///
/// DjVu text rects are top-left origin; bottom-anchored exporters need this
/// flip before scaling into their own units. The EPUB overlay uses it directly
/// (then converts to a CSS percentage). PDF performs the *equivalent* flip in
/// point space (`pt_h − px_to_pt(top_edge)`) rather than calling this, because
/// reordering the `f32` operations would perturb its `{:.4}`-formatted output.
///
/// `#[allow(dead_code)]`: the only current caller is the `epub`-gated exporter,
/// so std-only builds compile it unused.
#[allow(dead_code)]
pub(crate) fn flip_y_bottom(total_h: u32, rect: &Rect) -> u32 {
    total_h.saturating_sub(rect.y + rect.height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{Rect, TextZone, TextZoneKind};

    fn zone(kind: TextZoneKind, text: &str, children: Vec<TextZone>) -> TextZone {
        TextZone {
            kind,
            rect: Rect {
                x: 1,
                y: 2,
                width: 3,
                height: 4,
            },
            text: text.to_string(),
            children,
        }
    }

    #[test]
    fn word_spans_collects_leaf_words_in_order_skipping_empty() {
        // Page → Line → [Word "a", Word "" (dropped), Word "b"]; plus a
        // Character leaf under a second line.
        let layer = TextLayer {
            text: String::new(),
            zones: vec![zone(
                TextZoneKind::Page,
                "",
                vec![
                    zone(
                        TextZoneKind::Line,
                        "",
                        vec![
                            zone(TextZoneKind::Word, "a", vec![]),
                            zone(TextZoneKind::Word, "", vec![]),
                            zone(TextZoneKind::Word, "b", vec![]),
                        ],
                    ),
                    zone(
                        TextZoneKind::Line,
                        "",
                        vec![zone(TextZoneKind::Character, "c", vec![])],
                    ),
                ],
            )],
        };
        let spans = word_spans(&layer);
        let texts: Vec<&str> = spans.iter().map(|s| s.text).collect();
        assert_eq!(texts, ["a", "b", "c"]);
    }

    #[test]
    fn word_spans_does_not_descend_into_an_emitted_word() {
        // A Word with Character children: the Word is emitted, its children are
        // not visited (matches PDF/EPUB behavior).
        let layer = TextLayer {
            text: String::new(),
            zones: vec![zone(
                TextZoneKind::Word,
                "word",
                vec![
                    zone(TextZoneKind::Character, "w", vec![]),
                    zone(TextZoneKind::Character, "o", vec![]),
                ],
            )],
        };
        let spans = word_spans(&layer);
        let texts: Vec<&str> = spans.iter().map(|s| s.text).collect();
        assert_eq!(texts, ["word"]);
    }

    #[test]
    fn word_spans_empty_word_drops_its_character_children() {
        let layer = TextLayer {
            text: String::new(),
            zones: vec![zone(
                TextZoneKind::Word,
                "",
                vec![zone(TextZoneKind::Character, "x", vec![])],
            )],
        };
        assert!(word_spans(&layer).is_empty());
    }

    #[test]
    fn scaled_size_rounds_and_clamps_to_one() {
        assert_eq!(scaled_size(100, 200, 0.5), (50, 100));
        // Round to nearest.
        assert_eq!(scaled_size(3, 3, 0.5), (2, 2)); // 1.5 → 2
        // Clamp to at least 1 even when scale collapses the dimension.
        assert_eq!(scaled_size(10, 10, 0.001), (1, 1));
        // scale == 0 still yields the 1×1 floor.
        assert_eq!(scaled_size(10, 10, 0.0), (1, 1));
    }

    #[test]
    fn flip_y_bottom_saturates_and_matches_formula() {
        let r = Rect {
            x: 0,
            y: 10,
            width: 5,
            height: 20,
        };
        // 100 - (10 + 20) = 70.
        assert_eq!(flip_y_bottom(100, &r), 70);
        // Rect taller than the page saturates to 0 instead of underflowing.
        assert_eq!(flip_y_bottom(15, &r), 0);
    }
}
