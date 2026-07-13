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
//! [4] magic   = "AT&T" (optional for standalone BM44/PM44 files)
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

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};
#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

// ---- Error types ------------------------------------------------------------

/// Errors that can occur while parsing the IFF container.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IffError {
    /// Input data is too short to contain a valid IFF file.
    #[error("input is too short to be a valid IFF file")]
    TooShort,

    /// The `AT&T` magic bytes were not found at the start of the file.
    #[error("bad magic bytes: expected AT&T, got {got:?}")]
    BadMagic { got: [u8; 4] },

    /// The FORM type identifier is not a recognised DjVu type.
    ///
    /// Note: this is *not* an error — callers may encounter unknown form types
    /// in bundled documents and should handle them gracefully.
    #[error("unknown FORM type: {id:?}")]
    UnknownFormType { id: [u8; 4] },

    /// A chunk header claims more bytes than are available in the buffer.
    #[error(
        "chunk {:?} claims {} bytes but only {} are available",
        id,
        claimed,
        available
    )]
    ChunkTooLong {
        id: [u8; 4],
        claimed: u32,
        available: usize,
    },

    /// The input ended unexpectedly in the middle of a chunk.
    #[error("unexpected end of input (truncated IFF data)")]
    Truncated,

    /// The INFO chunk declares a format version DjVuLibre itself refuses to
    /// decode. DjVuLibre's `DJVUVERSION_TOO_NEW` forward-compatibility
    /// ceiling (`libdjvu/DjVuInfo.h`) is 50: `if (info->version >= 50) throw
    /// "Cannot decode DjVu files with version>= 50"`. We match that ceiling
    /// rather than silently decoding files DjVuLibre itself refuses.
    #[error("unsupported DjVu format version {version} (DjVuLibre rejects version >= 50)")]
    UnsupportedVersion { version: u16 },
}

/// Original error type used by the legacy implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyError {
    /// Input data is shorter than expected.
    UnexpectedEof,
    /// A required magic number or tag was not found.
    InvalidMagic,
    /// A chunk or field has an invalid length.
    InvalidLength,
    /// A required chunk is missing.
    MissingChunk(&'static str),
    /// An unsupported feature or version was encountered.
    Unsupported(&'static str),
    /// Generic format violation.
    FormatError(String),
}

impl core::fmt::Display for LegacyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LegacyError::UnexpectedEof => write!(f, "unexpected end of input"),
            LegacyError::InvalidMagic => write!(f, "invalid magic number"),
            LegacyError::InvalidLength => write!(f, "invalid length"),
            LegacyError::MissingChunk(id) => write!(f, "missing required chunk: {}", id),
            LegacyError::Unsupported(msg) => write!(f, "unsupported: {}", msg),
            LegacyError::FormatError(msg) => write!(f, "format error: {}", msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LegacyError {}

/// Alias for [`LegacyError`].
pub use LegacyError as Error;

// ---- IFF chunk types --------------------------------------------------------

/// The 4-byte magic that prefixes every on-disk DjVu IFF stream.
///
/// The single source of the literal: writers prepend `&MAGIC` rather than
/// re-spelling `b"AT&T"`, so the emission seam owns the framing bytes. (A
/// guard test rejects raw `b"AT&T"`/`b"FORM"` assembly outside this crate.)
pub const MAGIC: [u8; 4] = *b"AT&T";

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

    let (root, _) = parse_chunk(rest, 0, 0)?;
    Ok(DjvuFile { root })
}

/// Maximum FORM nesting depth. Real DjVu files nest a handful of levels
/// (DJVM → DJVU → chunks); a deeper chain is malformed and, without this bound,
/// drives `parse_chunk`/`parse_children` into unbounded recursion → stack
/// overflow on crafted input.
const MAX_IFF_DEPTH: u32 = 64;

/// Parse a single chunk starting at `offset` within `data`.
/// Returns the parsed chunk and the number of bytes consumed (including padding).
fn parse_chunk(data: &[u8], offset: usize, depth: u32) -> Result<(Chunk, usize), Error> {
    if depth > MAX_IFF_DEPTH {
        return Err(Error::InvalidLength);
    }
    if offset.checked_add(8).is_none_or(|end| end > data.len()) {
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
    // `length` is attacker-controlled (u32, up to 4 GiB). On 32-bit targets
    // (wasm32) `payload_start + length` can wrap, defeating the bounds check
    // below and causing a slice panic or a runaway loop. Use checked math.
    let payload_end = payload_start
        .checked_add(length as usize)
        .ok_or(Error::InvalidLength)?;

    if payload_end > data.len() {
        return Err(Error::UnexpectedEof);
    }

    // Word-align: next chunk starts at even offset
    let total = 8usize
        .checked_add(length as usize)
        .ok_or(Error::InvalidLength)?;
    let padded_total = total.checked_add(total % 2).ok_or(Error::InvalidLength)?;

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
        let children = parse_children(data, children_start, payload_end, depth + 1)?;

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
fn parse_children(data: &[u8], start: usize, end: usize, depth: u32) -> Result<Vec<Chunk>, Error> {
    let mut chunks = Vec::new();
    let mut pos = start;

    while pos < end {
        if pos + 8 > end {
            // Trailing bytes — some files have junk at end; tolerate it
            break;
        }
        let (chunk, consumed) = parse_chunk(data, pos, depth)?;
        chunks.push(chunk);
        // `consumed` is padded_total ≥ 8, so this always advances; checked to
        // stay sound under any future change and on 32-bit targets.
        pos = pos.checked_add(consumed).ok_or(Error::InvalidLength)?;
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
    out.extend_from_slice(&MAGIC);
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

/// Number of bytes [`emit`] writes for `chunk`: the 8-byte header, the payload,
/// and any word-alignment pad byte.
///
/// This is the single source of the framing/size arithmetic. It walks the same
/// `suppress_last_pad` parity rule as [`emit_chunk_inner`], so `emitted_size`
/// and `emit` can never disagree — a guarantee callers that pre-compute byte
/// offsets (e.g. DIRM offset recomputation in the document mutator) rely on for
/// correctness.
pub fn emitted_size(chunk: &Chunk) -> usize {
    emitted_size_inner(chunk, false)
}

fn emitted_size_inner(chunk: &Chunk, suppress_inner_pad: bool) -> usize {
    match chunk {
        Chunk::Form {
            length: stored_length,
            children,
            ..
        } => {
            let suppress_last_pad = (*stored_length & 1) == 1;
            let n = children.len();
            let mut payload = 4usize; // secondary_id
            for (i, child) in children.iter().enumerate() {
                let last = i + 1 == n;
                payload += emitted_size_inner(child, last && suppress_last_pad);
            }
            let total = 8 + payload;
            total + usize::from(!suppress_inner_pad && total % 2 == 1)
        }
        Chunk::Leaf { data, .. } => {
            let total = 8 + data.len();
            total + usize::from(!suppress_inner_pad && total % 2 == 1)
        }
    }
}

/// One child for [`partial_emit`]: a parsed [`Chunk`] to re-frame, a verbatim
/// byte slice copied as-is, or a nested `FORM` container framed from its body.
pub enum EmitPart<'a> {
    /// Re-frame this chunk through the canonical emitter (8-byte header,
    /// payload, word-alignment pad).
    Chunk(&'a Chunk),
    /// Copy these bytes into the FORM payload verbatim. Use this for children
    /// whose bytes must be preserved exactly (the byte-preserving path); any
    /// word-alignment pad is added by [`partial_emit`] if the slice has odd
    /// length, so callers may pass either padded or unpadded child blocks.
    Verbatim(&'a [u8]),
    /// Frame a nested `FORM` container whose *body* is given verbatim. `body`
    /// starts with the 4-byte secondary id (`DJVU`/`DJVI`/`THUM`/…); the seam
    /// writes the `FORM` tag, the big-endian length, the body, and the
    /// word-alignment pad. Use this for the component sub-FORMs of a bundle so
    /// the `FORM` framing is never hand-rolled at the call site (and so the
    /// component's start offset is reported by [`partial_emit_with_offsets`]).
    Form(&'a [u8]),
}

/// Emit a complete DjVu file (`AT&T` magic + one root `FORM`) whose children
/// are a mix of re-framed chunks and verbatim original slices.
///
/// This is the byte-preserving counterpart to [`emit`]: untouched children pass
/// through as [`EmitPart::Verbatim`] (their original bytes), while edited
/// children are re-framed as [`EmitPart::Chunk`]. Every child is word-aligned
/// inside the payload, and the FORM length is computed here — through the same
/// framing rules as [`emit`] / [`emitted_size`], so the three can't drift.
///
/// Returns `None` if the assembled FORM payload exceeds `u32::MAX`.
pub fn partial_emit(secondary_id: ChunkId, parts: &[EmitPart<'_>]) -> Option<Vec<u8>> {
    partial_emit_with_offsets(secondary_id, parts).map(|(bytes, _)| bytes)
}

/// Like [`partial_emit`], but also returns the absolute file-byte offset of
/// each part within the returned buffer: `offsets[i]` is the index at which
/// `parts[i]`'s framing begins, measured from the start of the leading `AT&T`
/// magic.
///
/// This is the seam for writers that must record an external index of where
/// each component landed — most notably a bundled `FORM:DJVM`, whose `DIRM`
/// offset table stores the file offset of every component `FORM`. Those
/// offsets live *inside* one part (the `DIRM`) yet describe the *others*, so
/// such a writer is inherently two-pass: emit once to learn the offsets, write
/// them into the `DIRM`, then emit again. The second pass yields identical
/// offsets — a part's position depends only on the sizes of the parts before
/// it, and a fixed-width offset table does not change size when its values
/// change — so the two passes cannot disagree.
///
/// Returns `None` if the assembled FORM payload (or any [`EmitPart::Form`]
/// body) exceeds `u32::MAX`.
pub fn partial_emit_with_offsets(
    secondary_id: ChunkId,
    parts: &[EmitPart<'_>],
) -> Option<(Vec<u8>, Vec<usize>)> {
    // The file prologue before the payload is AT&T(4) + FORM(4) + length(4) =
    // 12 bytes, so a part written while the payload already holds `k` bytes
    // begins at file offset 12 + k.
    const PROLOGUE: usize = 12;
    let mut payload = Vec::new();
    payload.extend_from_slice(&secondary_id); // even start (4 bytes)
    let mut offsets = Vec::with_capacity(parts.len());
    for part in parts {
        offsets.push(PROLOGUE + payload.len());
        match part {
            EmitPart::Chunk(chunk) => emit_chunk(chunk, &mut payload),
            EmitPart::Verbatim(bytes) => {
                payload.extend_from_slice(bytes);
                if payload.len() % 2 == 1 {
                    payload.push(0);
                }
            }
            EmitPart::Form(body) => {
                let len = u32::try_from(body.len()).ok()?;
                payload.extend_from_slice(b"FORM");
                payload.extend_from_slice(&len.to_be_bytes());
                payload.extend_from_slice(body);
                if payload.len() % 2 == 1 {
                    payload.push(0);
                }
            }
        }
    }
    let len = u32::try_from(payload.len()).ok()?;
    let mut out = Vec::with_capacity(8 + payload.len());
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(b"FORM");
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&payload);
    // Payload stays even (even start + self-aligned parts), so no outer pad is
    // ever needed; guard defensively to keep the invariant explicit.
    if (8 + payload.len()) % 2 == 1 {
        out.push(0);
    }
    Some((out, offsets))
}

// ---- New spec-based IFF parser (phase 1) ------------------------------------
//
// `parse_form` is a new zero-copy parser written from the sndjvu.org spec.
// It returns `Form` and `IffChunk` types (distinct from the legacy `Chunk`).

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
/// - The data does not begin with the `AT&T` magic bytes or a bare `FORM`
/// - The FORM chunk header is missing or malformed
/// - Any chunk extends beyond the available data
pub fn parse_form(data: &[u8]) -> Result<Form<'_>, IffError> {
    // Need at least: FORM id(4) + length(4) + form_type(4) = 12 bytes.
    // The optional AT&T prefix adds four bytes.
    if data.len() < 12 {
        return Err(IffError::TooShort);
    }

    // DjVuLibre's standalone BM44/PM44 extractor emits a bare FORM without
    // the optional AT&T prefix.  Keep accepting the canonical prefix while
    // also accepting that valid legacy IW44 representation.
    let prefix = read_4(data, 0)?;
    let form_offset = if &prefix == b"AT&T" {
        4
    } else if &prefix == b"FORM" {
        0
    } else {
        return Err(IffError::BadMagic { got: prefix });
    };

    // Read FORM chunk id
    let form_id = read_4(data, form_offset)?;
    if &form_id != b"FORM" {
        return Err(IffError::Truncated);
    }

    // Read FORM length (big-endian u32)
    let form_len = read_u32_be(data, form_offset + 4)? as usize;

    // FORM data starts after the FORM header and must fit within the buffer.
    let form_data_start = form_offset + 8;
    let form_data_end = form_data_start
        .checked_add(form_len)
        .ok_or(IffError::Truncated)?;
    if form_data_end > data.len() {
        return Err(IffError::ChunkTooLong {
            id: *b"FORM",
            claimed: form_len as u32,
            available: data.len().saturating_sub(form_data_start),
        });
    }

    // Read form_type (first 4 bytes of FORM data)
    if form_len < 4 {
        return Err(IffError::Truncated);
    }
    let form_type = read_4(data, form_data_start)?;
    if form_offset == 0 && !matches!(&form_type, b"BM44" | b"PM44") {
        return Err(IffError::BadMagic { got: prefix });
    }

    // Parse chunks from the FORM body (after form_type)
    let body = data
        .get(form_data_start + 4..form_data_end)
        .ok_or(IffError::Truncated)?;

    let chunks = parse_form_body(body)?;

    Ok(Form { form_type, chunks })
}

/// Parse a sequence of IFF chunks from a FORM body (the bytes *after* the
/// 4-byte form type), returning zero-copy [`IffChunk`] slices.
///
/// Each chunk is: `[4-byte id][4-byte big-endian length][length bytes data]`,
/// with data padded to an even byte boundary. This is the single chunk-walker
/// shared by the document reader, the mutator, and DJVM merge/split — callers
/// that already stripped the `AT&T`/`FORM`/length/form-type prologue (e.g. a
/// sub-FORM body, or a `FORM:DJVU` page extracted from a bundle) pass the
/// remaining bytes here instead of re-implementing the walk.
pub fn parse_form_body(mut buf: &[u8]) -> Result<Vec<IffChunk<'_>>, IffError> {
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

    /// A chain of FORMs nested far deeper than `MAX_IFF_DEPTH` must be rejected,
    /// not recurse until the stack overflows (security finding).
    #[test]
    fn deeply_nested_forms_are_rejected_not_overflow() {
        // innermost: FORM + len(4) + secondary "DJVU" (even-length, no padding)
        let mut buf = Vec::new();
        buf.extend_from_slice(b"FORM");
        buf.extend_from_slice(&4u32.to_be_bytes());
        buf.extend_from_slice(b"DJVU");
        for _ in 0..200 {
            let inner = buf;
            let len = 4 + inner.len();
            let mut outer = Vec::new();
            outer.extend_from_slice(b"FORM");
            outer.extend_from_slice(&(len as u32).to_be_bytes());
            outer.extend_from_slice(b"DJVU");
            outer.extend_from_slice(&inner);
            buf = outer;
        }
        let mut full = Vec::from(*b"AT&T");
        full.extend_from_slice(&buf);
        assert!(
            parse(&full).is_err(),
            "deep nesting must error, not overflow"
        );
    }

    /// A chunk length that overflows `usize` math must be rejected (32-bit / wasm32
    /// safety — `payload_start + length` must not wrap).
    #[test]
    fn overflowing_chunk_length_is_rejected() {
        let mut data = Vec::from(*b"AT&T");
        data.extend_from_slice(b"JUNK");
        data.extend_from_slice(&u32::MAX.to_be_bytes()); // 4 GiB claimed length
        data.extend_from_slice(b"\x00\x00");
        assert!(parse(&data).is_err());
    }

    fn assets_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../references/djvujs/library/assets")
    }

    fn golden_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/iff")
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

    // ---- emitted_size / partial_emit ----------------------------------------

    /// `emitted_size(root)` must equal the bytes `emit` writes for that root
    /// (the whole file minus the 4-byte `AT&T` magic) — the invariant DIRM
    /// offset recomputation relies on. Checked across the real-asset corpus,
    /// which mixes odd- and even-length FORM declarations.
    fn assert_emitted_size_matches_emit(name: &str) {
        let Ok(data) = std::fs::read(assets_path().join(name)) else {
            return; // asset not vendored in this checkout
        };
        let file = parse(&data).unwrap();
        let emitted = emit(&file);
        assert_eq!(
            emitted_size(&file.root),
            emitted.len() - 4,
            "emitted_size disagrees with emit() for {name}"
        );
    }

    #[test]
    fn emitted_size_matches_emit_corpus() {
        for name in [
            "boy_jb2.djvu",
            "boy.djvu",
            "chicken.djvu",
            "carte.djvu",
            "navm_fgbz.djvu",
            "colorbook.djvu",
            "DjVu3Spec_bundled.djvu",
            "big-scanned-page.djvu",
        ] {
            assert_emitted_size_matches_emit(name);
        }
    }

    #[test]
    fn partial_emit_verbatim_matches_chunk_framing() {
        // A child copied verbatim from a canonical emit must produce the same
        // bytes as re-framing that child through EmitPart::Chunk — i.e. the
        // byte-preserving path and the re-emit path agree. Build an even-parity
        // tree (root length 0) so emit word-aligns every child, the convention
        // partial_emit also uses.
        let tree = DjvuFile {
            root: Chunk::Form {
                secondary_id: *b"DJVU",
                length: 0,
                children: vec![
                    Chunk::Leaf {
                        id: *b"INFO",
                        data: vec![0xAA; 5], // odd → forces a pad
                    },
                    Chunk::Leaf {
                        id: *b"Sjbz",
                        data: vec![0xBB; 4], // even
                    },
                ],
            },
        };
        let canonical = emit(&tree); // AT&T + FORM + DJVU + framed children

        let Chunk::Form { children, .. } = &tree.root else {
            unreachable!()
        };
        // Re-emit each child into its own framed block to slice verbatim spans.
        let mut info_bytes = Vec::new();
        emit_chunk(&children[0], &mut info_bytes);
        let mut sjbz_bytes = Vec::new();
        emit_chunk(&children[1], &mut sjbz_bytes);

        let via_verbatim = partial_emit(
            *b"DJVU",
            &[
                EmitPart::Verbatim(&info_bytes),
                EmitPart::Verbatim(&sjbz_bytes),
            ],
        )
        .expect("fits in u32");
        let via_chunks = partial_emit(
            *b"DJVU",
            &[EmitPart::Chunk(&children[0]), EmitPart::Chunk(&children[1])],
        )
        .expect("fits in u32");

        assert_eq!(via_verbatim, canonical, "verbatim path must match emit");
        assert_eq!(via_chunks, canonical, "chunk path must match emit");
    }

    #[test]
    fn partial_emit_pads_odd_verbatim_child() {
        // A 3-byte verbatim child must be padded to an even boundary inside the
        // payload, exactly like an emitted odd-length chunk.
        let parts = [EmitPart::Verbatim(&[1u8, 2, 3])];
        let out = partial_emit(*b"DJVU", &parts).unwrap();
        // AT&T(4) FORM(4) len(4) DJVU(4) + 3 data + 1 pad = 20 bytes.
        assert_eq!(out.len(), 20);
        assert_eq!(&out[..8], b"AT&TFORM");
        // FORM length = DJVU(4) + 3 + 1 pad = 8.
        assert_eq!(u32::from_be_bytes(out[8..12].try_into().unwrap()), 8);
        assert_eq!(&out[12..16], b"DJVU");
        assert_eq!(&out[16..19], &[1, 2, 3]);
        assert_eq!(out[19], 0);
    }

    #[test]
    fn partial_emit_form_part_frames_nested_form() {
        // An `EmitPart::Form` body must be framed as `FORM` + len + body + pad,
        // identical to copying a pre-framed FORM chunk verbatim.
        let body: &[u8] = b"DJVUxyz"; // 7 bytes (odd) → forces a pad
        let via_form = partial_emit(*b"DJVM", &[EmitPart::Form(body)]).unwrap();

        // Hand-frame the same component to compare against the seam output.
        let mut framed = Vec::new();
        framed.extend_from_slice(b"FORM");
        framed.extend_from_slice(&(body.len() as u32).to_be_bytes());
        framed.extend_from_slice(body);
        framed.push(0); // odd body → pad
        let via_verbatim = partial_emit(*b"DJVM", &[EmitPart::Verbatim(&framed)]).unwrap();

        assert_eq!(via_form, via_verbatim, "Form part must match framed FORM");
        // Spot-check the literal bytes too.
        assert_eq!(&via_form[..8], b"AT&TFORM");
        assert_eq!(&via_form[12..16], b"DJVM");
        assert_eq!(&via_form[16..20], b"FORM");
        assert_eq!(u32::from_be_bytes(via_form[20..24].try_into().unwrap()), 7);
        assert_eq!(&via_form[24..31], body);
        assert_eq!(via_form[31], 0); // pad
    }

    #[test]
    fn partial_emit_with_offsets_reports_part_starts() {
        // Each reported offset must point at the byte where that part's framing
        // begins (the `FORM`/leaf-id tag), measured from the `AT&T` magic.
        let dirm = Chunk::Leaf {
            id: *b"DIRM",
            data: vec![0xAB; 5], // odd → the DIRM chunk gets a pad
        };
        let comp0: &[u8] = b"DJVU0000"; // 8 bytes (even)
        let comp1: &[u8] = b"DJVIaa"; // 6 bytes (even)
        let parts = [
            EmitPart::Chunk(&dirm),
            EmitPart::Form(comp0),
            EmitPart::Form(comp1),
        ];
        let (bytes, offsets) = partial_emit_with_offsets(*b"DJVM", &parts).unwrap();

        assert_eq!(offsets.len(), 3);
        // DIRM: AT&T(4)+FORM(4)+len(4)+DJVM(4) = 16.
        assert_eq!(offsets[0], 16);
        assert_eq!(&bytes[offsets[0]..offsets[0] + 4], b"DIRM");
        // Component FORM tags land exactly where the offset table says.
        for &off in &offsets[1..] {
            assert_eq!(&bytes[off..off + 4], b"FORM", "offset must point at FORM");
        }
        // comp1 sits after comp0's full framing: 8 (header) + 8 (even body).
        assert_eq!(offsets[2] - offsets[1], 16);
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
    fn non_form_root_chunk_is_truncated_error() {
        // Line 556: AT&T magic present but root chunk id is not FORM
        let mut data = Vec::new();
        data.extend_from_slice(b"AT&T");
        data.extend_from_slice(b"INFO"); // not FORM
        data.extend_from_slice(&10u32.to_be_bytes());
        data.extend_from_slice(&[0u8; 10]);
        assert_eq!(parse_form(&data).unwrap_err(), IffError::Truncated);
    }

    #[test]
    fn form_too_short_for_secondary_id() {
        // Line 574: FORM length < 4 (not enough bytes for the secondary_id).
        // parse_form requires >= 16 bytes total, so pad to 16 while keeping length=3.
        let mut data = Vec::new();
        data.extend_from_slice(b"AT&T");
        data.extend_from_slice(b"FORM");
        data.extend_from_slice(&3u32.to_be_bytes()); // length = 3 < 4
        data.extend_from_slice(b"XYZ\x00"); // 4 bytes to reach 16 total
        assert_eq!(parse_form(&data).unwrap_err(), IffError::Truncated);
    }

    #[test]
    fn sub_chunk_length_exceeds_body() {
        // Lines 608-610: a sub-chunk in parse_form_body claims more bytes than available
        // Build a minimal DJVU FORM: AT&T + FORM(length) + DJVU + INFO(claimed 100, actual 2)
        let mut body = Vec::new();
        body.extend_from_slice(b"DJVU"); // form_type
        body.extend_from_slice(b"INFO");
        body.extend_from_slice(&100u32.to_be_bytes()); // claimed length: 100
        body.extend_from_slice(&[0u8; 2]); // only 2 actual bytes
        let mut data = Vec::new();
        data.extend_from_slice(b"AT&T");
        data.extend_from_slice(b"FORM");
        data.extend_from_slice(&(body.len() as u32).to_be_bytes());
        data.extend_from_slice(&body);
        match parse_form(&data).unwrap_err() {
            IffError::ChunkTooLong { .. } => {}
            other => panic!("expected ChunkTooLong, got {other:?}"),
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
    fn bare_form_legacy_iw44_parses() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/legacy_pm44.iw4");
        let data = std::fs::read(path).expect("legacy_pm44.iw4 must exist");
        let form = parse_form(&data).expect("bare FORM:PM44 should parse");

        assert_eq!(&form.form_type, b"PM44");
        assert_eq!(form.chunks.len(), 3);
        assert!(form.chunks.iter().all(|chunk| &chunk.id == b"PM44"));
    }

    #[test]
    fn bare_form_normal_djvu_is_rejected() {
        let data = minimal_djvu_bytes();
        let error = parse_form(&data[4..]).expect_err("bare FORM:DJVU must stay rejected");
        assert_eq!(error, IffError::BadMagic { got: *b"FORM" });
    }

    #[test]
    fn real_multipage_djvu_parses() {
        let path = assets_path().join("navm_fgbz.djvu");
        let data = std::fs::read(&path).expect("navm_fgbz.djvu must exist");
        let form = parse_form(&data).expect("navm_fgbz.djvu should parse");

        assert_eq!(&form.form_type, b"DJVM");
        assert!(!form.chunks.is_empty());
    }

    // Lines 95-102: LegacyError Display variants
    #[test]
    fn legacy_error_display_variants() {
        assert_eq!(
            LegacyError::UnexpectedEof.to_string(),
            "unexpected end of input"
        );
        assert_eq!(
            LegacyError::InvalidMagic.to_string(),
            "invalid magic number"
        );
        assert_eq!(LegacyError::InvalidLength.to_string(), "invalid length");
        assert_eq!(
            LegacyError::MissingChunk("INFO").to_string(),
            "missing required chunk: INFO"
        );
        assert_eq!(LegacyError::Unsupported("x").to_string(), "unsupported: x");
        assert_eq!(
            LegacyError::FormatError("y".to_string()).to_string(),
            "format error: y"
        );
    }

    // Lines 151, 169-172, 180, 185-190: Chunk accessor methods on Form/Leaf
    #[test]
    fn chunk_accessors_form_and_leaf() {
        let leaf = Chunk::Leaf {
            id: *b"INFO",
            data: vec![1, 2, 3],
        };
        let form = Chunk::Form {
            secondary_id: *b"DJVU",
            length: 10,
            children: vec![leaf.clone()],
        };

        // data(): Form returns empty, Leaf returns data
        assert_eq!(form.data(), &[] as &[u8]);
        assert_eq!(leaf.data(), &[1u8, 2, 3]);

        // children(): Form returns children, Leaf returns empty
        assert_eq!(form.children().len(), 1);
        assert!(leaf.children().is_empty());

        // payload_length(): Form returns declared length, Leaf returns data.len()
        assert_eq!(form.payload_length(), 10);
        assert_eq!(leaf.payload_length(), 3);

        // find_first(): on Leaf returns None (no children)
        assert!(leaf.find_first(b"INFO").is_none());

        // find_first() on Form with no matching child returns None
        let form2 = Chunk::Form {
            secondary_id: *b"DJVU",
            length: 0,
            children: vec![],
        };
        assert!(form2.find_first(b"INFO").is_none());
    }

    #[test]
    fn find_all_returns_all_matching_leaves() {
        let leaf1 = Chunk::Leaf {
            id: *b"INFO",
            data: vec![1],
        };
        let leaf2 = Chunk::Leaf {
            id: *b"INFO",
            data: vec![2],
        };
        let leaf3 = Chunk::Leaf {
            id: *b"BG44",
            data: vec![3],
        };
        // A Form child — find_all should skip it (the _ => false branch)
        let child_form = Chunk::Form {
            secondary_id: *b"DJVU",
            length: 0,
            children: vec![],
        };
        let form = Chunk::Form {
            secondary_id: *b"DJVU",
            length: 0,
            children: vec![leaf1, leaf2, leaf3, child_form],
        };
        let all_info = form.find_all(b"INFO");
        assert_eq!(all_info.len(), 2);
        let all_bg44 = form.find_all(b"BG44");
        assert_eq!(all_bg44.len(), 1);
        let all_none = form.find_all(b"NONE");
        assert!(all_none.is_empty());
    }

    #[test]
    fn find_first_skips_form_children() {
        // A Form whose first child is itself a Form — the `_ => false` branch
        // in find_first skips it and finds the Leaf later.
        let child_form = Chunk::Form {
            secondary_id: *b"DJVU",
            length: 0,
            children: vec![],
        };
        let leaf = Chunk::Leaf {
            id: *b"INFO",
            data: vec![42],
        };
        let form = Chunk::Form {
            secondary_id: *b"DJVU",
            length: 0,
            children: vec![child_form, leaf],
        };
        let found = form.find_first(b"INFO").expect("should find INFO");
        assert!(matches!(found, Chunk::Leaf { id, .. } if id == b"INFO"));
    }

    #[test]
    fn parse_empty_input_returns_unexpected_eof() {
        // Line 207: data.len() < 4
        assert!(matches!(parse(b""), Err(Error::UnexpectedEof)));
        assert!(matches!(parse(b"AT"), Err(Error::UnexpectedEof)));
    }

    #[test]
    fn parse_form_length_too_small_returns_invalid_length() {
        // Line 255: FORM chunk with length field < 4
        // AT&T + FORM + length(3) + 3 bytes payload = 15 bytes total
        let mut data = vec![];
        data.extend_from_slice(b"AT&T");
        data.extend_from_slice(b"FORM");
        data.extend_from_slice(&3u32.to_be_bytes()); // length < 4
        data.extend_from_slice(b"XYZ");
        assert!(matches!(parse(&data), Err(Error::InvalidLength)));
    }

    #[test]
    fn parse_children_skips_trailing_bytes() {
        // Line 295: FORM with trailing bytes (pos + 8 > end but pos < end)
        // Construct a FORM with 4 bytes secondary_id + 5 bytes trailing junk
        // (5 < 8, so parse_children will break out of its loop)
        let mut data = vec![];
        data.extend_from_slice(b"AT&T");
        data.extend_from_slice(b"FORM");
        let secondary_plus_junk = b"DJVU\x01\x02\x03\x04\x05"; // 4 + 5 = 9 bytes
        data.extend_from_slice(&(secondary_plus_junk.len() as u32).to_be_bytes());
        data.extend_from_slice(secondary_plus_junk);
        let result = parse(&data);
        // Should succeed (not error) and produce a Form with 0 children
        let djvu = result.expect("trailing bytes must not cause an error");
        assert!(matches!(djvu.root, Chunk::Form { .. }));
        assert!(djvu.root.children().is_empty());
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
