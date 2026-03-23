use crate::error::Error;

/// A 4-byte chunk identifier (e.g., b"FORM", b"INFO", b"Sjbz").
pub type ChunkId = [u8; 4];

/// A parsed IFF chunk — either a FORM container or a leaf data chunk.
#[derive(Debug, Clone)]
pub enum Chunk<'a> {
    /// A FORM container with a secondary ID and child chunks.
    Form {
        /// The secondary ID (e.g., b"DJVU", b"DJVM", b"DJVI", b"THUM").
        secondary_id: ChunkId,
        /// Total byte length of the FORM payload (from the IFF length field).
        #[allow(dead_code)]
        length: u32,
        /// Child chunks within this FORM.
        children: Vec<Chunk<'a>>,
    },
    /// A leaf chunk with raw data.
    Leaf {
        /// The chunk ID (e.g., b"INFO", b"Sjbz", b"BG44").
        id: ChunkId,
        /// The raw chunk payload bytes (zero-copy slice of input).
        data: &'a [u8],
    },
}

impl<'a> Chunk<'a> {
    /// For leaf chunks, return the data slice. For FORM chunks, returns empty slice.
    pub fn data(&self) -> &'a [u8] {
        match self {
            Chunk::Form { .. } => &[],
            Chunk::Leaf { data, .. } => data,
        }
    }

    /// For FORM chunks, return children. For leaf chunks, returns empty slice.
    pub fn children(&self) -> &[Chunk<'a>] {
        match self {
            Chunk::Form { children, .. } => children,
            Chunk::Leaf { .. } => &[],
        }
    }

    /// Find the first leaf chunk with the given ID in direct children.
    pub fn find_first(&self, target_id: &[u8; 4]) -> Option<&Chunk<'a>> {
        self.children().iter().find(|c| match c {
            Chunk::Leaf { id, .. } => id == target_id,
            _ => false,
        })
    }

    /// Find all leaf chunks with the given ID in direct children.
    pub fn find_all(&self, target_id: &[u8; 4]) -> Vec<&Chunk<'a>> {
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
pub struct DjvuFile<'a> {
    pub root: Chunk<'a>,
}

/// Parse a DjVu file from raw bytes.
///
/// Expects the file to begin with "AT&T" magic followed by a root FORM chunk.
pub fn parse(data: &[u8]) -> Result<DjvuFile<'_>, Error> {
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
fn parse_chunk(data: &[u8], offset: usize) -> Result<(Chunk<'_>, usize), Error> {
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
        let chunk_data = &data[payload_start..payload_end];
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
fn parse_children(data: &[u8], start: usize, end: usize) -> Result<Vec<Chunk<'_>>, Error> {
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

/// Produce a structural dump of the chunk tree.
/// Format: each line has "  " indentation per depth, then "FORM:XXXX [len]" or "XXXX [len]".
/// This matches the structural parts of djvudump output (without semantic annotations).
#[cfg(test)]
pub fn dump(file: &DjvuFile<'_>) -> String {
    let mut out = String::new();
    dump_chunk(&file.root, 1, &mut out);
    out
}

#[cfg(test)]
fn dump_chunk(chunk: &Chunk<'_>, depth: usize, out: &mut String) {
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

    /// Parse our structural dump and djvudump output to comparable lines.
    /// We extract only: indentation + chunk_id + [length]
    /// from djvudump lines like:
    ///   "  FORM:DJVU [267] "
    ///   "    INFO [10]         DjVu 192x256, v24, 300 dpi, gamma=2.2"
    fn normalize_dump(input: &str) -> Vec<String> {
        input
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                // Find the pattern: optional leading spaces, then ID, then [number]
                // Keep only up to and including the "] " part
                let trimmed = line.trim_end();
                if let Some(bracket_end) = trimmed.find(']') {
                    let structural = &trimmed[..=bracket_end];
                    // Remove trailing spaces but keep the bracket
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
    fn parse_boy_jb2() {
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

                match &children[0] {
                    Chunk::Leaf { id, data } => {
                        assert_eq!(id, b"INFO");
                        assert_eq!(data.len(), 10);
                    }
                    _ => panic!("expected leaf INFO"),
                }

                match &children[1] {
                    Chunk::Leaf { id, data } => {
                        assert_eq!(id, b"Sjbz");
                        assert_eq!(data.len(), 237);
                    }
                    _ => panic!("expected leaf Sjbz"),
                }
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

    #[test]
    fn find_first_chunk() {
        let data = std::fs::read(assets_path().join("boy_jb2.djvu")).unwrap();
        let file = parse(&data).unwrap();
        let info = file.root.find_first(b"INFO").unwrap();
        assert_eq!(info.data().len(), 10);
        let sjbz = file.root.find_first(b"Sjbz").unwrap();
        assert_eq!(sjbz.data().len(), 237);
        assert!(file.root.find_first(b"BG44").is_none());
    }

    // --- Phase 6.2: Edge case tests ---

    #[test]
    fn empty_input() {
        assert!(parse(&[]).is_err());
    }

    #[test]
    fn too_short_for_magic() {
        assert!(parse(b"AT").is_err());
    }

    #[test]
    fn magic_only_no_form() {
        assert!(parse(b"AT&T").is_err());
    }

    #[test]
    fn truncated_form_header() {
        // AT&T + "FORM" but missing size
        assert!(parse(b"AT&TFORM").is_err());
    }

    #[test]
    fn truncated_chunk_data() {
        // Valid header but chunk size exceeds available data
        let mut data = b"AT&TFORM".to_vec();
        data.extend_from_slice(&100u32.to_be_bytes()); // size = 100
        data.extend_from_slice(b"DJVU"); // secondary ID
        data.extend_from_slice(b"INFO"); // leaf chunk ID
        data.extend_from_slice(&50u32.to_be_bytes()); // leaf size = 50
        data.extend_from_slice(&[0u8; 10]); // only 10 bytes of data
        assert!(parse(&data).is_err());
    }

    #[test]
    fn zero_size_leaf_chunk() {
        // A valid FORM with a zero-length leaf chunk
        let mut data = b"AT&TFORM".to_vec();
        let form_size = 4 + 4 + 4; // secondary_id + leaf_id + leaf_size
        data.extend_from_slice(&(form_size as u32).to_be_bytes());
        data.extend_from_slice(b"DJVU");
        data.extend_from_slice(b"INFO");
        data.extend_from_slice(&0u32.to_be_bytes()); // zero-length INFO
        let file = parse(&data).unwrap();
        let info = file.root.find_first(b"INFO").unwrap();
        assert_eq!(info.data().len(), 0);
    }

    #[test]
    fn unknown_chunk_id_passthrough() {
        // A FORM containing an unknown chunk type — should parse without error
        let mut data = b"AT&TFORM".to_vec();
        let leaf_data = b"hello";
        let leaf_size = leaf_data.len() as u32;
        // pad to even
        let padded = if leaf_size % 2 == 1 {
            leaf_size + 1
        } else {
            leaf_size
        };
        let form_size = 4 + 4 + 4 + padded; // secondary_id + id + size + data
        data.extend_from_slice(&form_size.to_be_bytes());
        data.extend_from_slice(b"DJVU");
        data.extend_from_slice(b"ZZZZ"); // unknown chunk
        data.extend_from_slice(&leaf_size.to_be_bytes());
        data.extend_from_slice(leaf_data);
        if leaf_size % 2 == 1 {
            data.push(0); // padding byte
        }
        let file = parse(&data).unwrap();
        let zzzz = file.root.find_first(b"ZZZZ").unwrap();
        assert_eq!(zzzz.data(), b"hello");
    }
}
