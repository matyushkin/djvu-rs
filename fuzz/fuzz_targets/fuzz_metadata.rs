#![no_main]
//! Metadata-plane round-trip fuzzing (#597): structured `TextLayer` → TXTz,
//! bookmarks → NAVM (through the full document path), and annotation/maparea
//! sets → ANT s-expressions must survive encode → decode with content intact,
//! modulo documented lenient normalization. The committed deterministic
//! counterpart (same assertion bodies, LCG seeds) is
//! `tests/metadata_roundtrip.rs`; this target lets coverage guidance explore
//! the string/nesting space beyond those seeds.

use djvu_rs::annotation::{
    encode_annotations_bzz, parse_annotations, Annotation, Border, Color, Highlight, MapArea, Shape,
};
use djvu_rs::djvu_document::{DjVuBookmark, DjVuDocument};
use djvu_rs::djvu_mut::DjVuDocumentMut;
use djvu_rs::text::{parse_text_layer, Rect, TextLayer, TextZone, TextZoneKind};
use djvu_rs::text_encode::encode_text_layer;
use libfuzzer_sys::fuzz_target;

/// Base document for the NAVM path — small fixture with existing NAVM+FGbz.
const NAVM_BASE: &[u8] = include_bytes!("../../tests/fixtures/navm_fgbz.djvu");

/// Byte-driven decision source: consumes fuzz input; returns 0 when drained
/// so every input terminates.
struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn byte(&mut self) -> u8 {
        let b = self.data.get(self.pos).copied().unwrap_or(0);
        self.pos = self.pos.saturating_add(1);
        b
    }

    fn next(&mut self, m: u32) -> u32 {
        u32::from(self.byte()) % m.max(1)
    }

    fn drained(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Fuzz-driven text: raw UTF-8 runs from the input when valid, plus the
    /// multi-byte edge-case pool from #524/#551 (CP1252 favourites, Cyrillic,
    /// CJK, combining marks).
    fn text(&mut self, max_len: usize) -> String {
        const POOL: &[&str] = &[
            "a", "B", "z", "0", " ", "-", "'", "\u{2019}", "\u{201C}", "\u{00e9}", "\u{00df}", "ж",
            "Щ", "ы", "汉", "字", "\u{0301}", "\u{00a0}", "…", "€",
        ];
        let len = self.next(max_len.max(1) as u32) as usize;
        let mut s = String::new();
        for _ in 0..len {
            let b = self.byte();
            if b < 0x80 && b >= 0x20 {
                s.push(b as char);
            } else {
                s.push_str(POOL[b as usize % POOL.len()]);
            }
        }
        s
    }
}

fn zone(cur: &mut Cursor<'_>, text: &str, depth: u32) -> TextZone {
    let bounds: Vec<usize> = text
        .char_indices()
        .map(|(i, _)| i)
        .chain(core::iter::once(text.len()))
        .collect();
    let a = bounds[cur.next(bounds.len() as u32) as usize];
    let b = bounds[cur.next(bounds.len() as u32) as usize];
    let (a, b) = (a.min(b), a.max(b));
    const KINDS: [TextZoneKind; 6] = [
        TextZoneKind::Column,
        TextZoneKind::Region,
        TextZoneKind::Para,
        TextZoneKind::Line,
        TextZoneKind::Word,
        TextZoneKind::Character,
    ];
    let mut z = TextZone {
        kind: KINDS[cur.next(6) as usize],
        rect: Rect {
            x: cur.next(2000),
            y: cur.next(2000),
            width: 1 + cur.next(1000),
            height: 1 + cur.next(1000),
        },
        text: text[a..b].to_string(),
        children: Vec::new(),
    };
    // Depth 5 exceeds the soak's 3 to press on the #368 depth-guard class,
    // while the drained() check keeps termination input-bounded.
    if depth < 5 && !cur.drained() {
        for _ in 0..cur.next(3) {
            let t = z.text.clone();
            z.children.push(zone(cur, &t, depth + 1));
        }
    }
    z
}

fn assert_zones_eq(a: &TextZone, b: &TextZone) {
    assert_eq!(a.kind, b.kind, "zone kind");
    assert_eq!(a.rect, b.rect, "zone rect");
    assert_eq!(a.text, b.text, "zone text");
    assert_eq!(a.children.len(), b.children.len(), "child count");
    for (ca, cb) in a.children.iter().zip(&b.children) {
        assert_zones_eq(ca, cb);
    }
}

fn text_layer_case(cur: &mut Cursor<'_>) {
    let text = cur.text(120);
    if text.is_empty() {
        return;
    }
    let page_height = 1 + cur.next(4000);
    let root = TextZone {
        kind: TextZoneKind::Page,
        rect: Rect {
            x: 0,
            y: 0,
            width: 1 + cur.next(3000),
            height: page_height,
        },
        text: text.clone(),
        children: {
            let n = cur.next(4);
            (0..n).map(|_| zone(cur, &text, 0)).collect()
        },
    };
    let layer = TextLayer {
        text: text.clone(),
        zones: vec![root],
    };
    let encoded = encode_text_layer(&layer, page_height);
    let decoded = parse_text_layer(&encoded, page_height).expect("txt: undecodable encoder output");
    assert_eq!(decoded.text, layer.text, "txt: text");
    assert_eq!(decoded.zones.len(), layer.zones.len(), "txt: roots");
    assert_zones_eq(&layer.zones[0], &decoded.zones[0]);
}

fn bookmarks(cur: &mut Cursor<'_>, depth: u32, budget: &mut u32) -> Vec<DjVuBookmark> {
    let n = cur.next(4);
    let mut out = Vec::new();
    for _ in 0..n {
        if *budget == 0 {
            break;
        }
        *budget -= 1;
        out.push(DjVuBookmark {
            title: cur.text(24),
            url: format!("#{}", 1 + cur.next(500)),
            // Depth 8 exceeds the soak's 6 for the same #368 reason as zones.
            children: if depth < 8 {
                bookmarks(cur, depth + 1, budget)
            } else {
                Vec::new()
            },
        });
    }
    out
}

fn assert_bookmarks_eq(a: &[DjVuBookmark], b: &[DjVuBookmark]) {
    assert_eq!(a.len(), b.len(), "bookmark count");
    for (x, y) in a.iter().zip(b) {
        assert_eq!(x.title, y.title, "title");
        assert_eq!(x.url, y.url, "url");
        assert_bookmarks_eq(&x.children, &y.children);
    }
}

fn navm_case(cur: &mut Cursor<'_>) {
    let mut budget = 40u32;
    let marks = bookmarks(cur, 0, &mut budget);
    let mut doc = DjVuDocumentMut::from_bytes(NAVM_BASE).expect("fixture parses");
    doc.set_bookmarks(&marks).expect("set_bookmarks");
    let bytes = doc.try_into_bytes().expect("serialize");
    let parsed = DjVuDocument::parse(&bytes).expect("navm: unparseable document output");
    assert_bookmarks_eq(&marks, parsed.bookmarks());
}

fn color(cur: &mut Cursor<'_>) -> Color {
    Color {
        r: cur.byte(),
        g: cur.byte(),
        b: cur.byte(),
    }
}

fn shape(cur: &mut Cursor<'_>) -> Shape {
    let rect = djvu_rs::annotation::Rect {
        x: cur.next(3000),
        y: cur.next(3000),
        width: 1 + cur.next(1000),
        height: 1 + cur.next(1000),
    };
    match cur.next(5) {
        0 => Shape::Rect(rect),
        1 => Shape::Oval(rect),
        2 => Shape::Text(rect),
        3 => Shape::Line(
            cur.next(3000),
            cur.next(3000),
            cur.next(3000),
            cur.next(3000),
        ),
        _ => Shape::Poly(
            (0..3 + cur.next(6))
                .map(|_| (cur.next(3000), cur.next(3000)))
                .collect(),
        ),
    }
}

fn annotation_case(cur: &mut Cursor<'_>) {
    let ann = Annotation {
        background: (cur.next(2) == 0).then(|| color(cur)),
        zoom: (cur.next(2) == 0).then(|| 10 + cur.next(400)),
        mode: (cur.next(2) == 0)
            .then(|| ["color", "bw", "fore", "back"][cur.next(4) as usize].to_string()),
    };
    let areas: Vec<MapArea> = (0..cur.next(5))
        .map(|_| MapArea {
            // Strings pass through s-expression quoting — press on quotes,
            // backslashes, and non-ASCII.
            url: format!("http://e.x/{}?q=\"{}\"", cur.next(100), cur.text(8)),
            description: cur.text(20),
            shape: shape(cur),
            border: (cur.next(2) == 0).then(|| Border {
                style: ["(xor)", "(border #0000FF)", "(shadow_in 4)"][cur.next(3) as usize]
                    .to_string(),
            }),
            highlight: (cur.next(2) == 0).then(|| Highlight { color: color(cur) }),
        })
        .collect();

    let encoded = encode_annotations_bzz(&ann, &areas);
    if encoded.is_empty() {
        assert!(ann.background.is_none() && ann.zoom.is_none() && ann.mode.is_none());
        assert!(areas.is_empty());
        return;
    }
    let decoded = djvu_rs::bzz::bzz_decode(&encoded).expect("ant: undecodable BZZ");
    let (ann2, areas2) = parse_annotations(&decoded).expect("ant: unparseable encoder output");
    assert_eq!(ann.background, ann2.background, "ant: background");
    assert_eq!(ann.zoom, ann2.zoom, "ant: zoom");
    assert_eq!(ann.mode, ann2.mode, "ant: mode");
    assert_eq!(areas.len(), areas2.len(), "ant: area count");
    for (a, b) in areas.iter().zip(&areas2) {
        assert_eq!(a.url, b.url, "ant: url");
        assert_eq!(a.description, b.description, "ant: description");
        assert_eq!(a.shape, b.shape, "ant: shape");
    }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    let mut cur = Cursor { data, pos: 0 };
    match cur.next(3) {
        0 => text_layer_case(&mut cur),
        1 => navm_case(&mut cur),
        _ => annotation_case(&mut cur),
    }
});
