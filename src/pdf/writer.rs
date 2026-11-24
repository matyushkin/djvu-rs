//! Low-level PDF syntax: the object writer, stream builders and number/string formatting.

use super::*;

/// Streams PDF objects to a [`Write`](std::io::Write) sink as they are added,
/// retaining only `(id, byte offset)` per object for the final xref table
/// (#606). Object bodies are written in insertion order — the same order the
/// former buffer-everything writer serialized them in, so output bytes are
/// unchanged.
pub(super) struct PdfWriter<W: std::io::Write> {
    pub(super) sink: W,
    /// Bytes written so far (= next object's offset).
    pub(super) written: usize,
    /// `(object id, byte offset)` in insertion order.
    pub(super) offsets: Vec<(usize, usize)>,
    pub(super) next_id: usize,
}

impl<W: std::io::Write> PdfWriter<W> {
    /// Create the writer and emit the PDF header.
    pub(super) fn new(sink: W) -> Result<Self, PdfError> {
        let mut w = PdfWriter {
            sink,
            written: 0,
            offsets: Vec::new(),
            next_id: 1,
        };
        w.write_all(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")?;
        Ok(w)
    }

    pub(super) fn write_all(&mut self, bytes: &[u8]) -> Result<(), PdfError> {
        self.sink.write_all(bytes)?;
        self.written += bytes.len();
        Ok(())
    }

    /// Reserve the next object ID.
    pub(super) fn alloc_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Write an object with a pre-allocated ID; its body is not retained.
    pub(super) fn add_obj(&mut self, id: usize, body: Vec<u8>) -> Result<(), PdfError> {
        self.offsets.push((id, self.written));
        self.write_all(format!("{id} 0 obj\n").as_bytes())?;
        self.write_all(&body)?;
        self.write_all(b"\nendobj\n")
    }

    /// Allocate and write an object, returning its ID.
    pub(super) fn add(&mut self, body: Vec<u8>) -> Result<usize, PdfError> {
        let id = self.alloc_id();
        self.add_obj(id, body)?;
        Ok(id)
    }

    /// Write the cross-reference table and trailer, consuming the writer.
    pub(super) fn finish(mut self) -> Result<(), PdfError> {
        let xref_offset = self.written;
        let max_id = self.offsets.iter().map(|(id, _)| *id).max().unwrap_or(0);
        let mut tail = format!("xref\n0 {}\n", max_id + 1).into_bytes();
        tail.extend_from_slice(b"0000000000 65535 f \n");

        let mut offset_map = vec![None; max_id + 1];
        for (obj_id, off) in &self.offsets {
            if *obj_id <= max_id {
                offset_map[*obj_id] = Some(*off);
            }
        }
        for entry in offset_map.iter().skip(1) {
            match entry {
                Some(off) => {
                    tail.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
                }
                None => tail.extend_from_slice(b"0000000000 65535 f \n"),
            }
        }

        tail.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                max_id + 1,
                xref_offset
            )
            .as_bytes(),
        );
        self.write_all(&tail)?;
        self.sink.flush()?;
        Ok(())
    }
}

/// Helper: make a PDF stream object `<< ... /Length N >> stream\n...\nendstream`.
pub(super) fn make_stream(dict_extra: &str, data: &[u8]) -> Vec<u8> {
    let len = data.len();
    let mut body = format!("<< /Length {len}{dict_extra} >>\nstream\n").into_bytes();
    body.extend_from_slice(data);
    body.extend_from_slice(b"\nendstream");
    body
}

/// Compress bytes using zlib/deflate.
pub(super) fn deflate(data: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec_zlib(data, 6)
}

/// Helper: make a compressed stream object.
pub(super) fn make_deflate_stream(dict_extra: &str, data: &[u8]) -> Vec<u8> {
    let compressed = deflate(data);
    let extra = format!(" /Filter /FlateDecode{dict_extra}");
    make_stream(&extra, &compressed)
}

/// Encode RGB bytes as JPEG and return the compressed bytes.
///
/// `quality` is in range 1–100. Values around 75–85 give excellent
/// perceptual quality for typical DjVu backgrounds at a fraction of the
/// FlateDecode+RGB size.
pub(super) fn encode_rgb_to_jpeg(rgb: &[u8], width: u32, height: u32, quality: u8) -> Vec<u8> {
    use jpeg_encoder::{ColorType, Encoder};
    let mut out = Vec::new();
    let enc = Encoder::new(&mut out, quality);
    // Ignore encoding errors — fallback to empty, which will be caught at
    // the caller and downgraded to FlateDecode.
    let _ = enc.encode(rgb, width as u16, height as u16, ColorType::Rgb);
    out
}

/// Helper: make a DCTDecode (JPEG) stream object.
pub(super) fn make_dct_stream(dict_extra: &str, jpeg_bytes: &[u8]) -> Vec<u8> {
    let extra = format!(" /Filter /DCTDecode{dict_extra}");
    make_stream(&extra, jpeg_bytes)
}

/// Helper: make a CCITTFaxDecode (Group 4 / T.6) stream object.
///
/// `K -1` selects pure two-dimensional (G4) decoding. `BlackIs1 true` matches
/// this crate's `Bitmap`/JB2 convention (bit `1` = black/marked pixel) so the
/// decoded samples are byte-identical to what the Deflate path already embeds
/// — only the filter changes, not the downstream `/Decode` array.
pub(super) fn make_ccitt_stream(
    dict_extra: &str,
    ncols: u32,
    nrows: u32,
    bitstream: &[u8],
) -> Vec<u8> {
    let extra = format!(
        " /Filter /CCITTFaxDecode /DecodeParms\
         << /K -1 /Columns {ncols} /Rows {nrows} /BlackIs1 true >>{dict_extra}"
    );
    make_stream(&extra, bitstream)
}

/// Default glyph advance of the text-layer font, in 1/1000 em.
const TEXT_FONT_DEFAULT_WIDTH: u32 = 556;

/// Write the font used by the invisible text layer under object `font_id`.
///
/// The font is a composite (Type0) font with the `Identity-H` encoding: every
/// code in a content stream is the 2-byte Unicode value of one character. A `/ToUnicode`
/// map sends each code back to the same Unicode value, so search and copy work
/// for any script (#1012). The text is never painted, so the embedded font
/// program is a tiny TrueType file with a single empty glyph.
pub(super) fn add_text_font<W: std::io::Write>(
    w: &mut PdfWriter<W>,
    font_id: usize,
) -> Result<(), PdfError> {
    let font_file = glyphless_ttf();
    let file_id = w.add(make_deflate_stream(
        &format!(" /Length1 {}", font_file.len()),
        &font_file,
    ))?;
    let descriptor_id = w.add(
        format!(
            "<< /Type /FontDescriptor /FontName /GlyphLessFont /Flags 4              /FontBBox [0 0 1000 1000] /ItalicAngle 0 /Ascent 1000 /Descent 0              /CapHeight 1000 /StemV 80 /FontFile2 {file_id} 0 R >>"
        )
        .into_bytes(),
    )?;
    // Every code maps to glyph 0, the only glyph of the font.
    let cid_to_gid_id = w.add(make_deflate_stream("", &vec![0u8; 0x2_0000]))?;
    let cid_font_id = w.add(
        format!(
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /GlyphLessFont              /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >>              /FontDescriptor {descriptor_id} 0 R /CIDToGIDMap {cid_to_gid_id} 0 R              /DW {TEXT_FONT_DEFAULT_WIDTH} /W [{}] >>",
            text_font_widths()
        )
        .into_bytes(),
    )?;
    let to_unicode_id = w.add(make_deflate_stream("", to_unicode_cmap().as_bytes()))?;
    w.add_obj(
        font_id,
        format!(
            "<< /Type /Font /Subtype /Type0 /BaseFont /GlyphLessFont /Encoding /Identity-H              /DescendantFonts [{cid_font_id} 0 R] /ToUnicode {to_unicode_id} 0 R >>"
        )
        .into_bytes(),
    )
}

/// Advance of one UTF-16 code unit in the text-layer font, in 1/1000 em.
fn code_unit_width(unit: u32) -> u32 {
    match char::from_u32(unit) {
        Some(c) => (super::text::helvetica_advance(c) * 1000.0).round() as u32,
        // A surrogate value: never written to a content stream.
        None => TEXT_FONT_DEFAULT_WIDTH,
    }
}

/// Body of the `/W` array: `first last width` for each run of code units whose
/// width differs from the default. Built from the same table that sizes the
/// text runs, so the viewer and the `Tz` scaling agree.
pub(super) fn text_font_widths() -> String {
    let mut out = String::new();
    let mut unit = 0u32;
    while unit <= 0xFFFF {
        let width = code_unit_width(unit);
        let mut last = unit;
        while last < 0xFFFF && code_unit_width(last + 1) == width {
            last += 1;
        }
        if width != TEXT_FONT_DEFAULT_WIDTH {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(&format!("{unit} {last} {width}"));
        }
        unit = last + 1;
    }
    out
}

/// `/ToUnicode` map: every 2-byte code is its own UTF-16 code unit.
///
/// A range may not cross a change of the high byte, hence 256 ranges, and a
/// block holds at most 100 ranges.
pub(super) fn to_unicode_cmap() -> String {
    let mut out = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
         1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
    );
    let mut high = 0u32;
    while high < 256 {
        let count = (256 - high).min(100);
        out.push_str(&format!("{count} beginbfrange\n"));
        for h in high..high + count {
            out.push_str(&format!("<{h:02X}00> <{h:02X}FF> <{h:02X}00>\n"));
        }
        out.push_str("endbfrange\n");
        high += count;
    }
    out.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    out
}

/// A minimal TrueType font: one empty glyph and the six tables PDF requires.
pub(super) fn glyphless_ttf() -> Vec<u8> {
    fn be16(values: &[u16]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_be_bytes()).collect()
    }
    fn checksum(data: &[u8]) -> u32 {
        data.chunks(4).fold(0u32, |sum, chunk| {
            let mut word = [0u8; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            sum.wrapping_add(u32::from_be_bytes(word))
        })
    }

    let width = TEXT_FONT_DEFAULT_WIDTH as u16;
    let mut head = Vec::new();
    head.extend_from_slice(&0x0001_0000u32.to_be_bytes()); // version
    head.extend_from_slice(&0x0001_0000u32.to_be_bytes()); // fontRevision
    head.extend_from_slice(&0u32.to_be_bytes()); // checkSumAdjustment, patched below
    head.extend_from_slice(&0x5F0F_3CF5u32.to_be_bytes()); // magicNumber
    head.extend_from_slice(&be16(&[0, 1000])); // flags, unitsPerEm
    head.extend_from_slice(&[0u8; 16]); // created, modified
    head.extend_from_slice(&be16(&[0, 0, 0, 0])); // bounding box
    head.extend_from_slice(&be16(&[0, 8, 2, 0, 0])); // macStyle, lowestRecPPEM, direction, loca format, glyph format

    // version, ascender, descender, lineGap, advanceWidthMax, three extents,
    // caret rise/run/offset, four reserved, metricDataFormat, numberOfHMetrics
    let mut hhea = 0x0001_0000u32.to_be_bytes().to_vec();
    hhea.extend_from_slice(&be16(&[
        1000, 0, 0, width, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1,
    ]));

    // version, numGlyphs, then the limits; maxZones is 2 as the format asks.
    let mut maxp = 0x0001_0000u32.to_be_bytes().to_vec();
    maxp.extend_from_slice(&be16(&[1, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0]));

    let tables: [(&[u8; 4], Vec<u8>); 6] = [
        // One glyph: no contours, empty bounding box, no instructions.
        (b"glyf", vec![0u8; 12]),
        (b"head", head),
        (b"hhea", hhea),
        (b"hmtx", be16(&[width, 0])),
        // Short offsets, stored halved: the glyph spans bytes 0..12.
        (b"loca", be16(&[0, 6])),
        (b"maxp", maxp),
    ];

    let mut font = 0x0001_0000u32.to_be_bytes().to_vec();
    // numTables, searchRange, entrySelector, rangeShift
    font.extend_from_slice(&be16(&[6, 64, 2, 32]));
    let mut offset = 12 + 16 * tables.len();
    let mut head_offset = 0;
    for (tag, data) in &tables {
        if *tag == b"head" {
            head_offset = offset;
        }
        font.extend_from_slice(*tag);
        font.extend_from_slice(&checksum(data).to_be_bytes());
        font.extend_from_slice(&(offset as u32).to_be_bytes());
        font.extend_from_slice(&(data.len() as u32).to_be_bytes());
        offset += data.len().next_multiple_of(4);
    }
    for (_, data) in &tables {
        font.extend_from_slice(data);
        font.resize(font.len().next_multiple_of(4), 0);
    }
    let adjustment = 0xB1B0_AFBAu32.wrapping_sub(checksum(&font));
    font[head_offset + 8..head_offset + 12].copy_from_slice(&adjustment.to_be_bytes());
    font
}

/// Format one colour component for a PDF `rg` operator (0..255 → 0..1).
///
/// `0` and `255` format as the exact literals `0` / `1` so the all-black layer
/// emits the historical `0 0 0 rg` operator byte-for-byte.
pub(super) fn fmt_rg_component(v: u8) -> String {
    match v {
        0 => "0".to_string(),
        255 => "1".to_string(),
        _ => format!("{:.4}", f32::from(v) / 255.0),
    }
}

/// Format a point offset for a `cm` operator: exact `0` for zero (matching the
/// historical full-page operator), 4 decimals otherwise.
pub(super) fn fmt_pt(v: f32) -> String {
    if v == 0.0 {
        "0".to_string()
    } else {
        format!("{v:.4}")
    }
}

/// Escape an ASCII string for PDF literal string syntax.
///
/// Callers go through [`pdf_text_string`], [`pdf_uri_string`] or
/// [`pdf_cid_string`], which never pass a non-ASCII character here.
pub(super) fn pdf_escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            c if c.is_ascii() => out.push(c),
            _ => out.push('?'),
        }
    }
    out
}

/// Hex digits of `s` as big-endian UTF-16 code units.
fn utf16be_hex(s: &str) -> String {
    s.encode_utf16().map(|unit| format!("{unit:04X}")).collect()
}

/// A complete PDF text string object (outline titles and the like).
///
/// ASCII stays a literal string. Anything else becomes a UTF-16BE hex string
/// with a byte-order mark, which keeps every Unicode character.
pub(super) fn pdf_text_string(s: &str) -> String {
    if s.is_ascii() {
        format!("({})", pdf_escape_string(s))
    } else {
        format!("<FEFF{}>", utf16be_hex(s))
    }
}

/// A complete PDF string object holding a URI.
///
/// A URI is 7-bit ASCII, so each non-ASCII character is percent-encoded as
/// its UTF-8 bytes.
pub(super) fn pdf_uri_string(s: &str) -> String {
    let mut ascii = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii() {
            ascii.push(c);
        } else {
            let mut buf = [0u8; 4];
            for byte in c.encode_utf8(&mut buf).bytes() {
                ascii.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    format!("({})", pdf_escape_string(&ascii))
}

/// A complete string operand for `Tj` in the text-layer font: one 2-byte code
/// per character.
///
/// A character outside the Basic Multilingual Plane has no single 2-byte code,
/// so it is written as U+FFFD (the replacement character).
pub(super) fn pdf_cid_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 4 + 2);
    out.push('<');
    for c in s.chars() {
        let code = if c.len_utf16() == 1 { c as u32 } else { 0xFFFD };
        out.push_str(&format!("{code:04X}"));
    }
    out.push('>');
    out
}
