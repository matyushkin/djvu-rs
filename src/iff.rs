//! IFF (Interchange File Format) container parser for DjVu files.
//!
//! This module provides two APIs:
//!
//! 1. **New spec-based parser** (`parse_form`) — zero-copy, borrowing slices from
//!    the input byte buffer. Written from the sndjvu.org specification.
//!
//! 2. **Legacy API** (`parse`, `Chunk`, `DjvuFile`) — the original tree-based parser
//!    kept for internal backward compatibility while the rewrite is in progress.
//!
//! ## DjVu IFF layout
//!
//! ```text
//! [4] magic   = "AT&T"
//! [4] id      = "FORM"
//! [4] length  (big-endian u32, covers form_type + all chunks)
//! [4] form_type = "DJVU" | "DJVM" | "BM44" | "PM44"
//! ... chunks
//! ```
//!
//! Each inner chunk:
//! ```text
//! [4] id
//! [4] length  (big-endian u32)
//! [n] data    (padded to even number of bytes if length is odd)
//! ```

// ---- IFF chunk types --------------------------------------------------------

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::error::LegacyError as Error;

/// A 4-byte chunk identifier (e.g., b"FORM", b"INFO", b"Sjbz").
pub type ChunkId = [u8; 4];

/// A parsed IFF chunk — either a FORM container or a leaf data chunk.
#[derive(Debug, Clone)]
pub enum Chunk {
    /// A FORM container with a secondary ID and child chunks.
    Form {
        /// The secondary ID (e.g., b"DJVU", b"DJVM", b"DJVI", b"THUM").
        secondary_id: ChunkId,
        /// Total byte length of the FORM payload (from the IFF length field).
        /// Includes the 4-byte secondary ID and all child chunk bytes.
        length: u32,
        /// Child chunks within this FORM.
        children: Vec<Chunk>,
    },
    /// A leaf chunk with raw data.
    Leaf {
        /// The chunk ID (e.g., b"INFO", b"Sjbz", b"BG44").
        id: ChunkId,
        /// The raw chunk payload bytes.
        data: Vec<u8>,
    },
}

impl Chunk {
    /// For leaf chunks, return the data slice. For FORM chunks, returns empty slice.
    pub fn data(&self) -> &[u8] {
        match self {
            Chunk::Form { .. } => &[],
            Chunk::Leaf { data, .. } => data,
        }
    }

    /// For FORM chunks, return children. For leaf chunks, returns empty slice.
    pub fn children(&self) -> &[Chunk] {
        match self {
            Chunk::Form { children, .. } => children,
            Chunk::Leaf { .. } => &[],
        }
    }

    /// Return the declared payload length from the IFF length field.
    ///
    /// For `Form` chunks, this is the value read from the IFF header — it
    /// covers the secondary ID (4 bytes) and all children.  For `Leaf`
    /// chunks, this equals `data().len()`.
    pub fn payload_length(&self) -> u32 {
        match self {
            Chunk::Form { length, .. } => *length,
            Chunk::Leaf { data, .. } => data.len() as u32,
        }
    }

    /// Find the first leaf chunk with the given ID in direct children.
    pub fn find_first(&self, target_id: &[u8; 4]) -> Option<&Chunk> {
        self.children().iter().find(|c| match c {
            Chunk::Leaf { id, .. } => id == target_id,
            _ => false,
        })
    }

    /// Find all leaf chunks with the given ID in direct children.
    pub fn find_all(&self, target_id: &[u8; 4]) -> Vec<&Chunk> {
        self.children()
            .iter()
            .filter(|c| match c {
                Chunk::Leaf { id, .. } => id == target_id,
                _ => false,
            })
            .collect()
    }
}

/// A parsed DjVu document (the root FORM chunk).
#[derive(Debug, Clone)]
pub struct DjvuFile {
    pub root: Chunk,
}

/// Parse a DjVu file from raw bytes (legacy tree-based parser).
///
/// Expects the file to begin with "AT&T" magic followed by a root FORM chunk.
pub fn parse(data: &[u8]) -> Result<DjvuFile, Error> {
    if data.len() < 4 {
        return Err(Error::UnexpectedEof);
    }
    // Check for "AT&T" magic
    let (magic, rest) = if &data[..4] == b"AT&T" {
        (&data[..4], &data[4..])
    } else {
        // Some files may not have AT&T prefix (bare FORM)
        (&data[..0], data)
    };
    let _ = magic;

    let (root, _) = parse_chunk(rest, 0)?;
    Ok(DjvuFile { root })
}

/// Parse a single chunk starting at `offset` within `data`.
/// Returns the parsed chunk and the number of bytes consumed (including padding).
fn parse_chunk(data: &[u8], offset: usize) -> Result<(Chunk, usize), Error> {
    if offset + 8 > data.len() {
        return Err(Error::UnexpectedEof);
    }

    let id: ChunkId = [
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ];
    let length = u32::from_be_bytes([
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ]);

    let payload_start = offset + 8;
    let payload_end = payload_start + length as usize;

    if payload_end > data.len() {
        return Err(Error::UnexpectedEof);
    }

    // Word-align: next chunk starts at even offset
    let total = 8 + length as usize;
    let padded_total = total + (total % 2);

    if &id == b"FORM" {
        if length < 4 {
            return Err(Error::InvalidLength);
        }
        let secondary_id: ChunkId = [
            data[payload_start],
            data[payload_start + 1],
            data[payload_start + 2],
            data[payload_start + 3],
        ];

        let children_start = payload_start + 4;
        let children = parse_children(data, children_start, payload_end)?;

        Ok((
            Chunk::Form {
                secondary_id,
                length,
                children,
            },
            padded_total,
        ))
    } else {
        let chunk_data = data[payload_start..payload_end].to_vec();
        Ok((
            Chunk::Leaf {
                id,
                data: chunk_data,
            },
            padded_total,
        ))
    }
}

/// Parse sequential chunks within a range of bytes.
fn parse_children(data: &[u8], start: usize, end: usize) -> Result<Vec<Chunk>, Error> {
    let mut chunks = Vec::new();
    let mut pos = start;

    while pos < end {
        if pos + 8 > end {
            // Trailing bytes — some files have junk at end; tolerate it
            break;
        }
        let (chunk, consumed) = parse_chunk(data, pos)?;
        chunks.push(chunk);
        pos += consumed;
    }

    Ok(chunks)
}

// ---- Legacy emitter (round-trip support, #195) ------------------------------

/// Serialise a `DjvuFile` (legacy parser) back into the on-disk IFF byte
/// stream, including the leading "AT&T" magic.
///
/// Parser/emitter contract: `parse(emit(file)) == file` for any tree
/// previously produced by `parse(...)`. This is used by property-based
/// round-trip tests under `tests/proptest_codecs.rs` (#195) and is small
/// enough to keep alongside the parser; not intended as a general-purpose
/// DjVu writer.
pub fn emit(file: &DjvuFile) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    out.extend_from_slice(b"AT&T");
    emit_chunk(&file.root, &mut out);
    out
}

fn emit_chunk(chunk: &Chunk, out: &mut Vec<u8>) {
    emit_chunk_inner(chunk, out, false);
}

fn emit_chunk_inner(chunk: &Chunk, out: &mut Vec<u8>, suppress_inner_pad: bool) {
    match chunk {
        Chunk::Form {
            secondary_id,
            length: stored_length,
            children,
        } => {
            // Two valid IFF layouts exist for a FORM whose last child has odd
            // payload length:
            //   (A) FORM declared length is odd, no pad after last child;
            //       the outer/parent loop writes the alignment byte.
            //   (B) FORM declared length is even, includes a pad byte after
            //       the last child inside the FORM body.
            // Real DjVu files mix both styles. Preserve the parser's stored
            // length parity so unmutated subtrees round-trip byte-identical.
            let suppress_last_pad = (*stored_length & 1) == 1;
            let mut payload: Vec<u8> = Vec::new();
            payload.extend_from_slice(secondary_id);
            let n = children.len();
            for (i, child) in children.iter().enumerate() {
                let last = i + 1 == n;
                emit_chunk_inner(child, &mut payload, last && suppress_last_pad);
            }
            let len = payload.len() as u32;
            out.extend_from_slice(b"FORM");
            out.extend_from_slice(&len.to_be_bytes());
            out.extend_from_slice(&payload);
            // Outer pad to align the next sibling in our parent. Skip when
            // our parent told us they'll provide alignment for us.
            let total = 8 + payload.len();
            if !suppress_inner_pad && total % 2 == 1 {
                out.push(0);
            }
        }
        Chunk::Leaf { id, data } => {
            let len = data.len() as u32;
            out.extend_from_slice(id);
            out.extend_from_slice(&len.to_be_bytes());
            out.extend_from_slice(data);
            let total = 8 + data.len();
            if !suppress_inner_pad && total % 2 == 1 {
                out.push(0);
            }
        }
    }
}

// ---- New spec-based IFF parser (phase 1) ------------------------------------
//
// `parse_form` is a new zero-copy parser written from the sndjvu.org spec.
// It returns `Form` and `IffChunk` types (distinct from the legacy `Chunk`).

use crate::error::IffError;

/// A parsed IFF chunk from the new spec-based parser: a 4-byte identifier
/// plus a zero-copy slice into the original byte buffer.
#[derive(Debug, Clone, Copy)]
pub struct IffChunk<'a> {
    /// The 4-byte ASCII chunk identifier.
    pub id: [u8; 4],
    /// The raw data bytes of this chunk (not including id or length header).
    pub data: &'a [u8],
}

/// The top-level FORM structure parsed by the spec-based parser.
#[derive(Debug)]
pub struct Form<'a> {
    /// The 4-byte FORM type (e.g. `DJVU`, `DJVM`, `BM44`, `PM44`).
    pub form_type: [u8; 4],
    /// All chunks contained within the FORM, in order.
    pub chunks: Vec<IffChunk<'a>>,
}

/// Parse a DjVu IFF byte stream into a [`Form`].
///
/// This is the new spec-based zero-copy parser. It returns borrowed data
/// from the input slice.
///
/// # Errors
///
/// Returns [`IffError`] if:
/// - The data does not begin with the `AT&T` magic bytes
/// - The FORM chunk header is missing or malformed
/// - Any chunk extends beyond the available data
pub fn parse_form(data: &[u8]) -> Result<Form<'_>, IffError> {
    // Need at least: magic(4) + FORM id(4) + length(4) + form_type(4) = 16 bytes
    if data.len() < 16 {
        return Err(IffError::TooShort);
    }

    // Verify AT&T magic prefix
    let magic = read_4(data, 0)?;
    if &magic != b"AT&T" {
        return Err(IffError::BadMagic { got: magic });
    }

    // Read FORM chunk id
    let form_id = read_4(data, 4)?;
    if &form_id != b"FORM" {
        return Err(IffError::Truncated);
    }

    // Read FORM length (big-endian u32)
    let form_len = read_u32_be(data, 8)? as usize;

    // FORM data starts at byte 12 and must fit within the buffer
    let form_data_end = 12_usize.checked_add(form_len).ok_or(IffError::Truncated)?;
    if form_data_end > data.len() {
        return Err(IffError::ChunkTooLong {
            id: *b"FORM",
            claimed: form_len as u32,
            available: data.len().saturating_sub(12),
        });
    }

    // Read form_type (first 4 bytes of FORM data)
    if form_len < 4 {
        return Err(IffError::Truncated);
    }
    let form_type = read_4(data, 12)?;

    // Parse chunks from the FORM body (after form_type)
    let body = data.get(16..form_data_end).ok_or(IffError::Truncated)?;

    let chunks = parse_iff_chunks(body)?;

    Ok(Form { form_type, chunks })
}

/// Parse a sequence of IFF chunks from a byte slice (new spec-based parser).
///
/// Each chunk is: `[4-byte id][4-byte big-endian length][length bytes data]`,
/// with data padded to an even byte boundary.
fn parse_iff_chunks(mut buf: &[u8]) -> Result<Vec<IffChunk<'_>>, IffError> {
    let mut chunks = Vec::new();

    while buf.len() >= 8 {
        let id = read_4(buf, 0)?;
        let data_len = read_u32_be(buf, 4)? as usize;

        let data_start = 8_usize;
        let data_end = data_start
            .checked_add(data_len)
            .ok_or(IffError::Truncated)?;

        if data_end > buf.len() {
            return Err(IffError::ChunkTooLong {
                id,
                claimed: data_len as u32,
                available: buf.len().saturating_sub(data_start),
            });
        }

        let chunk_data = buf.get(data_start..data_end).ok_or(IffError::Truncated)?;
        chunks.push(IffChunk {
            id,
            data: chunk_data,
        });

        // Advance past this chunk; pad to even boundary
        let padded_len = data_len + (data_len & 1);
        let next = data_start
            .checked_add(padded_len)
            .ok_or(IffError::Truncated)?;

        // Clamp to buf length to handle trailing padding gracefully
        buf = buf.get(next.min(buf.len())..).ok_or(IffError::Truncated)?;
    }

    Ok(chunks)
}

/// Read 4 bytes from `data` at `offset` as a `[u8; 4]`.
#[inline]
fn read_4(data: &[u8], offset: usize) -> Result<[u8; 4], IffError> {
    data.get(offset..offset + 4)
        .and_then(|s| s.try_into().ok())
        .ok_or(IffError::Truncated)
}

/// Read a big-endian `u32` from `data` at `offset`.
#[inline]
fn read_u32_be(data: &[u8], offset: usize) -> Result<u32, IffError> {
    let b = read_4(data, offset)?;
    Ok(u32::from_be_bytes(b))
}

// ---- Legacy dump helper (tests only) ----------------------------------------

/// Produce a structural dump of the chunk tree.
#[cfg(test)]
pub fn dump(file: &DjvuFile) -> String {
    let mut out = String::new();
    dump_chunk(&file.root, 1, &mut out);
    out
}

#[cfg(test)]
fn dump_chunk(chunk: &Chunk, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    match chunk {
        Chunk::Form {
            secondary_id,
            length,
            children,
        } => {
            let sec = std::str::from_utf8(secondary_id).unwrap_or("????");
            out.push_str(&format!("{}FORM:{} [{}] \n", indent, sec, length));
            for child in children {
                dump_chunk(child, depth + 1, out);
            }
        }
        Chunk::Leaf { id, data } => {
            let id_str = std::str::from_utf8(id).unwrap_or("????");
            out.push_str(&format!("{}{} [{}] \n", indent, id_str, data.len()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assets_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/djvujs/library/assets")
    }

    fn golden_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/iff")
    }

    // ---- Legacy parser tests ------------------------------------------------

    /// Parse our structural dump and djvudump output to comparable lines.
    fn normalize_dump(input: &str) -> Vec<String> {
        input
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                let trimmed = line.trim_end();
                if let Some(bracket_end) = trimmed.find(']') {
                    let structural = &trimmed[..=bracket_end];
                    structural.trim_end().to_string()
                } else {
                    trimmed.to_string()
                }
            })
            .collect()
    }

    fn assert_structure_matches(djvu_file: &str, golden_file: &str) {
        let data = std::fs::read(assets_path().join(djvu_file)).unwrap();
        let file = parse(&data).unwrap();
        let actual = dump(&file);
        let expected = std::fs::read_to_string(golden_path().join(golden_file)).unwrap();

        let actual_lines = normalize_dump(&actual);
        let expected_lines = normalize_dump(&expected);

        assert_eq!(
            actual_lines.len(),
            expected_lines.len(),
            "Line count mismatch for {} ({} vs {})",
            djvu_file,
            actual_lines.len(),
            expected_lines.len()
        );

        for (i, (a, e)) in actual_lines.iter().zip(expected_lines.iter()).enumerate() {
            assert_eq!(
                a,
                e,
                "Line {} mismatch for {}\n  actual:   {:?}\n  expected: {:?}",
                i + 1,
                djvu_file,
                a,
                e
            );
        }
    }

    #[test]
    fn parse_boy_jb2_legacy() {
        let data = std::fs::read(assets_path().join("boy_jb2.djvu")).unwrap();
        let file = parse(&data).unwrap();

        match &file.root {
            Chunk::Form {
                secondary_id,
                children,
                ..
            } => {
                assert_eq!(secondary_id, b"DJVU");
                assert_eq!(children.len(), 2);
            }
            _ => panic!("expected FORM root"),
        }
    }

    #[test]
    fn structure_boy_jb2() {
        assert_structure_matches("boy_jb2.djvu", "boy_jb2.dump");
    }

    #[test]
    fn structure_boy() {
        assert_structure_matches("boy.djvu", "boy.dump");
    }

    #[test]
    fn structure_chicken() {
        assert_structure_matches("chicken.djvu", "chicken.dump");
    }

    #[test]
    fn structure_carte() {
        assert_structure_matches("carte.djvu", "carte.dump");
    }

    #[test]
    fn structure_navm_fgbz() {
        assert_structure_matches("navm_fgbz.djvu", "navm_fgbz.dump");
    }

    #[test]
    fn structure_colorbook() {
        assert_structure_matches("colorbook.djvu", "colorbook.dump");
    }

    #[test]
    fn structure_djvu3spec_bundled() {
        assert_structure_matches("DjVu3Spec_bundled.djvu", "djvu3spec_bundled.dump");
    }

    #[test]
    fn structure_big_scanned_page() {
        assert_structure_matches("big-scanned-page.djvu", "big_scanned_page.dump");
    }

    // ---- New spec-based parser tests ----------------------------------------

    /// Build a minimal valid single-page DjVu file in memory for testing.
    fn minimal_djvu_bytes() -> Vec<u8> {
        let info_data: &[u8] = &[
            0x00, 0xB5, // width = 181
            0x00, 0xF0, // height = 240
            0x18, // minor version
            0x00, // major version
            0x64, 0x00, // dpi = 100 (little-endian)
            0x16, // gamma byte = 22 → 2.2
            0x00, // flags: no rotation
        ];
        let info_len = info_data.len() as u32;

        let mut chunk = Vec::new();
        chunk.extend_from_slice(b"INFO");
        chunk.extend_from_slice(&info_len.to_be_bytes());
        chunk.extend_from_slice(info_data);

        let mut form_body = Vec::new();
        form_body.extend_from_slice(b"DJVU");
        form_body.extend_from_slice(&chunk);

        let form_len = form_body.len() as u32;

        let mut file = Vec::new();
        file.extend_from_slice(b"AT&T");
        file.extend_from_slice(b"FORM");
        file.extend_from_slice(&form_len.to_be_bytes());
        file.extend_from_slice(&form_body);

        file
    }

    #[test]
    fn empty_input_is_error() {
        let result = parse_form(&[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), IffError::TooShort);
    }

    #[test]
    fn short_input_is_error() {
        let result = parse_form(&[0u8; 10]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), IffError::TooShort);
    }

    #[test]
    fn bad_magic_is_error() {
        let mut data = minimal_djvu_bytes();
        data[0] = 0xFF;
        data[1] = 0xFF;
        data[2] = 0xFF;
        data[3] = 0xFF;

        let result = parse_form(&data);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            IffError::BadMagic {
                got: [0xFF, 0xFF, 0xFF, 0xFF]
            }
        );
    }

    #[test]
    fn valid_single_page_parses() {
        let data = minimal_djvu_bytes();
        let form = parse_form(&data).expect("should parse successfully");

        assert_eq!(&form.form_type, b"DJVU");
        assert_eq!(form.chunks.len(), 1);
        assert_eq!(&form.chunks[0].id, b"INFO");
        assert_eq!(form.chunks[0].data.len(), 10);
    }

    #[test]
    fn truncated_chunk_is_error() {
        let mut data = minimal_djvu_bytes();
        let new_len = data.len() - 4;
        data.truncate(new_len);

        let result = parse_form(&data);
        assert!(result.is_err());
        match result.unwrap_err() {
            IffError::ChunkTooLong { .. } | IffError::Truncated => {}
            other => panic!("expected ChunkTooLong or Truncated, got {:?}", other),
        }
    }

    #[test]
    fn unknown_form_type_allowed() {
        let mut data = minimal_djvu_bytes();
        data[12] = b'X';
        data[13] = b'X';
        data[14] = b'X';
        data[15] = b'X';

        let form = parse_form(&data).expect("unknown form type should still parse");
        assert_eq!(&form.form_type, b"XXXX");
    }

    #[test]
    fn real_chicken_djvu_parses() {
        let path = assets_path().join("chicken.djvu");
        let data = std::fs::read(&path).expect("chicken.djvu must exist");
        let form = parse_form(&data).expect("chicken.djvu should parse");

        assert_eq!(&form.form_type, b"DJVU");
        assert!(!form.chunks.is_empty(), "must have at least one chunk");
        assert_eq!(&form.chunks[0].id, b"INFO");
        assert!(form.chunks[0].data.len() >= 10);
    }

    #[test]
    fn real_multipage_djvu_parses() {
        let path = assets_path().join("navm_fgbz.djvu");
        let data = std::fs::read(&path).expect("navm_fgbz.djvu must exist");
        let form = parse_form(&data).expect("navm_fgbz.djvu should parse");

        assert_eq!(&form.form_type, b"DJVM");
        assert!(!form.chunks.is_empty());
    }

    #[test]
    fn odd_length_chunk_padding() {
        let chunk1_data: &[u8] = &[0xAA, 0xBB, 0xCC, 0xDD, 0xEE]; // 5 bytes → padded to 6
        let chunk2_data: &[u8] = &[0x01, 0x02]; // 2 bytes

        let mut form_body: Vec<u8> = Vec::new();
        form_body.extend_from_slice(b"DJVU");

        form_body.extend_from_slice(b"TST1");
        form_body.extend_from_slice(&5u32.to_be_bytes());
        form_body.extend_from_slice(chunk1_data);
        form_body.push(0x00); // padding byte

        form_body.extend_from_slice(b"TST2");
        form_body.extend_from_slice(&2u32.to_be_bytes());
        form_body.extend_from_slice(chunk2_data);

        let form_len = form_body.len() as u32;

        let mut file: Vec<u8> = Vec::new();
        file.extend_from_slice(b"AT&T");
        file.extend_from_slice(b"FORM");
        file.extend_from_slice(&form_len.to_be_bytes());
        file.extend_from_slice(&form_body);

        let form = parse_form(&file).expect("should parse padded chunk");
        assert_eq!(form.chunks.len(), 2);
        assert_eq!(&form.chunks[0].id, b"TST1");
        assert_eq!(form.chunks[0].data, chunk1_data);
        assert_eq!(&form.chunks[1].id, b"TST2");
        assert_eq!(form.chunks[1].data, chunk2_data);
    }
}
