//! In-place DjVu document mutation — byte-preserving rewrite of the IFF tree.
//!
//! PR1 of [#222](https://github.com/matyushkin/djvu-rs/issues/222). This is the
//! foundation layer: parse a document into an editable tree, walk to a leaf
//! chunk by path, replace its data, and serialise back. When no mutations have
//! happened, [`DjVuDocumentMut::into_bytes`] returns the original bytes
//! verbatim (byte-identical round-trip).
//!
//! Future PRs in the [#222](https://github.com/matyushkin/djvu-rs/issues/222)
//! sequence add high-level setters (`set_metadata`, `set_bookmarks`,
//! `page_mut(i).set_text_layer`, `…set_annotations`) plus indirect-DJVM
//! support, which all build on the chunk-replacement primitive defined here.
//!
//! ## Example
//!
//! ```no_run
//! use djvu_rs::djvu_mut::DjVuDocumentMut;
//!
//! let original = std::fs::read("doc.djvu").unwrap();
//! let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();
//!
//! // Round-trip byte-identical without edits:
//! assert_eq!(doc.clone().into_bytes(), original);
//!
//! // Replace a leaf chunk's payload by path:
//! doc.replace_leaf(&[0], b"new payload".to_vec()).unwrap();
//! let edited = doc.into_bytes();
//! ```
//!
//! ## Path format
//!
//! A `path: &[usize]` is a sequence of child indices to walk from the root
//! `FORM` chunk. The root itself is never indexed — `[0]` selects the first
//! child of the root.
//!
//! For a single-page `FORM:DJVU`: `[i]` selects the i-th leaf chunk
//! (e.g. `INFO`, `Sjbz`, `BG44`). For a bundled `FORM:DJVM`:
//! `[0]` selects the `DIRM` chunk, `[1]` selects the `NAVM` chunk (if
//! present), `[i]` thereafter selects the i-th component `FORM:DJVU`. To
//! reach a leaf inside that component: `[i, j]`.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::annotation::{Annotation, MapArea, encode_annotations_bzz};
use crate::djvu_document::DjVuBookmark;
use crate::error::{IffError, LegacyError};
use crate::iff::{self, Chunk, DjvuFile};
use crate::info::PageInfo;
use crate::metadata::{DjVuMetadata, encode_metadata_bzz};
use crate::navm_encode::encode_navm;
use crate::text::TextLayer;
use crate::text_encode::encode_text_layer;

/// Errors produced by [`DjVuDocumentMut`] operations.
#[derive(Debug, thiserror::Error)]
pub enum MutError {
    /// IFF parse error during [`DjVuDocumentMut::from_bytes`].
    #[error("IFF parse error: {0}")]
    Parse(#[from] LegacyError),

    /// The path indexed past the end of a FORM's children.
    #[error("chunk path out of range: index {index} at depth {depth} (form has {len} children)")]
    PathOutOfRange {
        index: usize,
        depth: usize,
        len: usize,
    },

    /// The path traversed into a leaf chunk and tried to keep going.
    #[error("chunk path enters a leaf at depth {depth} but is {len} levels long")]
    PathTraversesLeaf { depth: usize, len: usize },

    /// `replace_leaf` was called with a path that ends on a `FORM` chunk
    /// rather than a leaf.
    #[error("path ends on a FORM, not a leaf chunk")]
    NotALeaf,

    /// The path is empty — must contain at least one index.
    #[error("path must not be empty")]
    EmptyPath,

    /// `page_mut` was called with an index past the document's page count.
    #[error("page index {index} out of range (document has {count} pages)")]
    PageOutOfRange {
        /// Requested page index.
        index: usize,
        /// Number of pages in the document.
        count: usize,
    },

    /// The page has no INFO chunk, which is required to encode chunks whose
    /// payload depends on page height (currently `set_text_layer`).
    #[error("page has no INFO chunk; cannot encode height-dependent chunk")]
    MissingPageInfo,

    /// The page's INFO chunk failed to parse.
    #[error("INFO chunk parse error: {0}")]
    InfoParse(#[from] IffError),

    /// The operation requires DIRM offset recomputation, which is not
    /// implemented for indirect (non-bundled) `FORM:DJVM` documents — those
    /// reference page bytes in external files via a resolver, so editing them
    /// in place would also need the external files rewritten. Tracked as a
    /// follow-up PR (PR5) in the
    /// [#222](https://github.com/matyushkin/djvu-rs/issues/222) sequence.
    #[error("mutation of indirect DJVM documents is not supported")]
    IndirectDjvmUnsupported,

    /// The DIRM chunk was malformed in a way that prevents offset
    /// recomputation. Should not occur after a successful
    /// [`DjVuDocumentMut::from_bytes`] on a well-formed DJVM document.
    #[error("DIRM chunk is malformed: {0}")]
    DirmMalformed(&'static str),

    /// The number of `FORM:DJVU`/`FORM:DJVI` components in the bundle does
    /// not match the count recorded in DIRM. Indicates a structurally
    /// inconsistent document.
    #[error("DIRM component count {dirm} does not match bundle child count {children}")]
    DirmComponentCountMismatch {
        /// Component count read from DIRM (`nfiles`).
        dirm: usize,
        /// Actual count of `FORM:DJVU`/`FORM:DJVI` children in the root.
        children: usize,
    },

    /// `set_bookmarks` was called on a `FORM:DJVU` (single-page) document.
    /// NAVM bookmarks live in `FORM:DJVM` bundles only.
    #[error("set_bookmarks requires a FORM:DJVM bundle (this document is FORM:DJVU)")]
    BookmarksRequireDjvm,
}

/// A DjVu document opened for in-place mutation.
///
/// Holds a parsed [`DjvuFile`] tree plus the original byte buffer, so that
/// [`Self::into_bytes`] returns a byte-identical copy when no edits have been
/// made. After any mutation the dirty flag is set and serialisation falls
/// through to [`iff::emit`], which reconstructs the IFF stream from the tree
/// (see the parser/emitter contract in `src/iff.rs`).
#[derive(Debug, Clone)]
pub struct DjVuDocumentMut {
    file: DjvuFile,
    /// Original bytes of the document.  Held so an unedited round-trip is
    /// byte-identical without re-emitting through `iff::emit` (which
    /// recomputes FORM lengths and would not necessarily match the original
    /// byte layout for documents with inconsistent headers).
    original_bytes: Vec<u8>,
    dirty: bool,
}

impl DjVuDocumentMut {
    /// Parse a DjVu document for mutation. Validates the IFF tree.
    ///
    /// The original bytes are retained so that a no-edit round-trip via
    /// [`Self::into_bytes`] is byte-identical to the input.
    pub fn from_bytes(data: &[u8]) -> Result<Self, MutError> {
        let file = iff::parse(data)?;
        Ok(Self {
            file,
            original_bytes: data.to_vec(),
            dirty: false,
        })
    }

    /// Number of direct children of the root FORM chunk.
    ///
    /// For a single-page `FORM:DJVU` this is the number of leaf chunks
    /// (`INFO`, `Sjbz`, …). For a bundled `FORM:DJVM` it is `DIRM` + optional
    /// `NAVM` + per-page component `FORM`s.
    pub fn root_child_count(&self) -> usize {
        self.file.root.children().len()
    }

    /// Return the 4-byte FORM type of the root (e.g. `b"DJVU"`, `b"DJVM"`).
    /// Returns `None` if the root is somehow a leaf — should never happen on
    /// a well-formed input that survived `from_bytes`.
    pub fn root_form_type(&self) -> Option<&[u8; 4]> {
        match &self.file.root {
            Chunk::Form { secondary_id, .. } => Some(secondary_id),
            Chunk::Leaf { .. } => None,
        }
    }

    /// Replace the data of the leaf chunk reached by `path`.
    ///
    /// `path` is a sequence of child indices walked from the root FORM's
    /// children. The walk descends into any FORM it encounters at an
    /// intermediate index; the final index must address a leaf.
    ///
    /// # Errors
    ///
    /// - [`MutError::EmptyPath`] if `path.is_empty()`.
    /// - [`MutError::PathOutOfRange`] if any index exceeds a FORM's child count.
    /// - [`MutError::PathTraversesLeaf`] if the path tries to descend past a leaf.
    /// - [`MutError::NotALeaf`] if the final chunk is a FORM rather than a leaf.
    pub fn replace_leaf(&mut self, path: &[usize], new_data: Vec<u8>) -> Result<(), MutError> {
        let chunk = self.chunk_at_path_mut(path)?;
        match chunk {
            Chunk::Leaf { data, .. } => {
                *data = new_data;
                self.dirty = true;
                Ok(())
            }
            Chunk::Form { .. } => Err(MutError::NotALeaf),
        }
    }

    /// Return the chunk at `path` for inspection (without mutation).
    pub fn chunk_at_path(&self, path: &[usize]) -> Result<&Chunk, MutError> {
        if path.is_empty() {
            return Err(MutError::EmptyPath);
        }
        let mut current = &self.file.root;
        for (depth, &idx) in path.iter().enumerate() {
            let children = current.children();
            if children.is_empty() && depth < path.len() - 1 {
                // We're inside a leaf but the path keeps going.
                return Err(MutError::PathTraversesLeaf {
                    depth,
                    len: path.len(),
                });
            }
            if let Chunk::Leaf { .. } = current {
                return Err(MutError::PathTraversesLeaf {
                    depth,
                    len: path.len(),
                });
            }
            if idx >= children.len() {
                return Err(MutError::PathOutOfRange {
                    index: idx,
                    depth,
                    len: children.len(),
                });
            }
            current = &children[idx];
        }
        Ok(current)
    }

    fn chunk_at_path_mut(&mut self, path: &[usize]) -> Result<&mut Chunk, MutError> {
        if path.is_empty() {
            return Err(MutError::EmptyPath);
        }
        // Validate path first using the immutable walk.  This avoids the
        // borrow-checker dance of validating during a mutable walk.
        let _ = self.chunk_at_path(path)?;
        // Now walk for real with `&mut`.
        let mut current = &mut self.file.root;
        for &idx in path {
            // Validation above guarantees the indices are in range and that
            // we never index into a leaf, so this match is total.
            match current {
                Chunk::Form { children, .. } => {
                    current = &mut children[idx];
                }
                Chunk::Leaf { .. } => unreachable!("validated by chunk_at_path"),
            }
        }
        Ok(current)
    }

    /// Whether any mutation has been applied since `from_bytes`.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Serialise the document back to bytes.
    ///
    /// When [`Self::is_dirty`] is `false`, this returns the bytes passed to
    /// [`Self::from_bytes`] verbatim. After any mutation it falls through to
    /// [`iff::emit`] which reconstructs the IFF stream from the parsed tree;
    /// for `FORM:DJVM` bundles the `DIRM` offsets are recomputed first so
    /// they point at the correct component positions in the new output.
    ///
    /// # Panics
    ///
    /// Panics if `DIRM` offset recomputation fails — this only happens on a
    /// structurally inconsistent document (DIRM `nfiles` not matching the
    /// bundle's child count, etc.) which a successful [`Self::from_bytes`]
    /// would already have rejected. Use [`Self::try_into_bytes`] to recover
    /// the error without panicking.
    pub fn into_bytes(self) -> Vec<u8> {
        self.try_into_bytes()
            .expect("DIRM recomputation failed — inconsistent document")
    }

    /// Like [`Self::into_bytes`] but returns the [`MutError`] from `DIRM`
    /// offset recomputation rather than panicking.
    pub fn try_into_bytes(mut self) -> Result<Vec<u8>, MutError> {
        if !self.dirty {
            return Ok(self.original_bytes);
        }
        recompute_dirm_offsets(&mut self.file.root)?;
        Ok(iff::emit(&self.file))
    }

    // ---- High-level setters (PR2 of #222) ----------------------------------

    /// Number of editable pages in the document.
    ///
    /// `1` for `FORM:DJVU`, the count of `FORM:DJVU` children for `FORM:DJVM`
    /// (shared-dictionary `FORM:DJVI` components are not counted as pages).
    pub fn page_count(&self) -> usize {
        match self.root_form_type() {
            Some(b"DJVM") => self
                .file
                .root
                .children()
                .iter()
                .filter(
                    |c| matches!(c, Chunk::Form { secondary_id, .. } if secondary_id == b"DJVU"),
                )
                .count(),
            _ => 1,
        }
    }

    /// Borrow the i-th page's `FORM:DJVU` for high-level mutation.
    ///
    /// For single-page `FORM:DJVU` only `index == 0` is valid. For bundled
    /// `FORM:DJVM` the index walks `FORM:DJVU` direct children in order
    /// (shared-dictionary `FORM:DJVI` components are skipped).
    ///
    /// On serialisation, [`Self::into_bytes`] rewrites DIRM offsets to
    /// reflect any size changes from page mutations.
    ///
    /// # Errors
    ///
    /// - [`MutError::PageOutOfRange`] if `index >= self.page_count()`.
    /// - [`MutError::IndirectDjvmUnsupported`] if the document is an
    ///   indirect (non-bundled) `FORM:DJVM` — page bytes live in external
    ///   files, so editing in place is not supported by this primitive.
    pub fn page_mut(&mut self, index: usize) -> Result<PageMut<'_>, MutError> {
        let count = self.page_count();
        if index >= count {
            return Err(MutError::PageOutOfRange { index, count });
        }
        let root_form_type = *self.root_form_type().expect("from_bytes validated FORM");
        if &root_form_type == b"DJVU" {
            debug_assert_eq!(index, 0);
            return Ok(PageMut {
                form: &mut self.file.root,
                dirty: &mut self.dirty,
            });
        }
        debug_assert_eq!(&root_form_type, b"DJVM");
        if !is_bundled_djvm(&self.file.root) {
            return Err(MutError::IndirectDjvmUnsupported);
        }
        // Walk the root's children, returning the index-th FORM:DJVU.
        let children = match &mut self.file.root {
            Chunk::Form { children, .. } => children,
            Chunk::Leaf { .. } => unreachable!("validated FORM root"),
        };
        let mut seen = 0usize;
        for child in children.iter_mut() {
            if let Chunk::Form { secondary_id, .. } = child
                && secondary_id == b"DJVU"
            {
                if seen == index {
                    return Ok(PageMut {
                        form: child,
                        dirty: &mut self.dirty,
                    });
                }
                seen += 1;
            }
        }
        unreachable!("page_count agreed with bundle but iteration disagreed")
    }

    /// Replace, insert, or remove the document's `NAVM` bookmark chunk.
    ///
    /// Empty `bookmarks` removes any existing NAVM. The chunk lives at the
    /// `FORM:DJVM` bundle root, between `DIRM` and the per-page components,
    /// and the encoder uses [`encode_navm`].
    ///
    /// # Errors
    ///
    /// - [`MutError::BookmarksRequireDjvm`] if the document is a single-page
    ///   `FORM:DJVU` (no NAVM in non-bundled documents per the DjVu spec).
    pub fn set_bookmarks(&mut self, bookmarks: &[DjVuBookmark]) -> Result<(), MutError> {
        let root_form_type = *self.root_form_type().expect("from_bytes validated FORM");
        if &root_form_type != b"DJVM" {
            return Err(MutError::BookmarksRequireDjvm);
        }
        let children = match &mut self.file.root {
            Chunk::Form { children, .. } => children,
            Chunk::Leaf { .. } => unreachable!("validated FORM root"),
        };
        let pos = children
            .iter()
            .position(|c| matches!(c, Chunk::Leaf { id, .. } if id == b"NAVM"));
        match (pos, bookmarks.is_empty()) {
            (Some(i), true) => {
                children.remove(i);
            }
            (Some(i), false) => {
                children[i] = Chunk::Leaf {
                    id: *b"NAVM",
                    data: encode_navm(bookmarks),
                };
            }
            (None, true) => { /* nothing to remove and nothing to insert */ }
            (None, false) => {
                // Insert NAVM right after DIRM if present, else right after
                // the secondary id (i.e. as the first child). DIRM is the
                // first chunk in a well-formed bundle.
                let dirm_pos = children
                    .iter()
                    .position(|c| matches!(c, Chunk::Leaf { id, .. } if id == b"DIRM"));
                let insert_at = dirm_pos.map(|i| i + 1).unwrap_or(0);
                children.insert(
                    insert_at,
                    Chunk::Leaf {
                        id: *b"NAVM",
                        data: encode_navm(bookmarks),
                    },
                );
            }
        }
        self.dirty = true;
        Ok(())
    }
}

/// Whether `chunk` is a bundled (rather than indirect) `FORM:DJVM`.
///
/// Returns `false` for any non-DJVM chunk.
fn is_bundled_djvm(chunk: &Chunk) -> bool {
    let Chunk::Form {
        secondary_id,
        children,
        ..
    } = chunk
    else {
        return false;
    };
    if secondary_id != b"DJVM" {
        return false;
    }
    children.iter().any(|c| {
        matches!(c, Chunk::Leaf { id, data } if id == b"DIRM" && !data.is_empty() && (data[0] & 0x80) != 0)
    })
}

/// Compute the byte length the chunk will occupy when emitted by [`iff::emit`]:
/// 8-byte chunk header + payload + word-alignment padding.
///
/// For `FORM` chunks the payload is recomputed recursively (4 bytes for
/// secondary_id + sum of children's emitted sizes), to mirror what
/// [`iff::emit`] writes after a tree mutation.
fn emitted_chunk_size(chunk: &Chunk) -> usize {
    match chunk {
        Chunk::Form {
            secondary_id: _,
            children,
            ..
        } => {
            let payload: usize = 4 + children.iter().map(emitted_chunk_size).sum::<usize>();
            let total = 8 + payload;
            total + (total & 1)
        }
        Chunk::Leaf { data, .. } => {
            let total = 8 + data.len();
            total + (total & 1)
        }
    }
}

/// Recompute the absolute byte offsets stored in the `DIRM` chunk so they
/// point at each `FORM:DJVU`/`FORM:DJVI` component in the about-to-be-emitted
/// document.
///
/// Offsets in DIRM are absolute file-byte positions (from the leading
/// `b"AT&T"` magic) of each component's outer `b"FORM"` chunk header. After a
/// page-chunk mutation those positions shift, and viewers that use DIRM for
/// page navigation see the wrong bytes if the table is not refreshed.
///
/// No-op for non-DJVM roots and for indirect DIRM (no offset table).
fn recompute_dirm_offsets(root: &mut Chunk) -> Result<(), MutError> {
    let Chunk::Form {
        secondary_id,
        children,
        ..
    } = root
    else {
        return Ok(());
    };
    if secondary_id != b"DJVM" {
        return Ok(());
    }

    // Absolute byte position of the next chunk inside the FORM:DJVM body:
    // AT&T(4) + FORM(4) + length(4) + secondary_id "DJVM"(4) = 16.
    let mut pos: usize = 16;
    let mut new_offsets: Vec<u32> = Vec::new();
    let mut dirm_idx: Option<usize> = None;

    // The `id == b"DIRM"` guard form is needed: `id` is `[u8; 4]` reached
    // through a `&` reference, so a by-value pattern would require `*b"DIRM"`
    // which clippy's redundant-guards autofix doesn't propose.
    #[allow(clippy::redundant_guards)]
    for (i, child) in children.iter().enumerate() {
        match child {
            Chunk::Leaf { id, .. } if id == b"DIRM" => {
                dirm_idx = Some(i);
            }
            Chunk::Form {
                secondary_id: sid, ..
            } if sid == b"DJVU" || sid == b"DJVI" || sid == b"THUM" => {
                new_offsets.push(u32::try_from(pos).map_err(|_| {
                    MutError::DirmMalformed("component offset exceeds u32 (file > 4 GiB)")
                })?);
            }
            _ => {}
        }
        pos += emitted_chunk_size(child);
    }

    let Some(dirm_idx) = dirm_idx else {
        // Bundled DJVM with no DIRM is malformed by spec, but tolerate it
        // (parse_dirm would have failed during from_bytes if it mattered).
        return Ok(());
    };

    let dirm = &mut children[dirm_idx];
    let Chunk::Leaf { data, .. } = dirm else {
        return Err(MutError::DirmMalformed("DIRM is not a leaf chunk"));
    };

    if data.len() < 3 {
        return Err(MutError::DirmMalformed("DIRM payload < 3 bytes"));
    }
    let bundled = (data[0] & 0x80) != 0;
    if !bundled {
        // Indirect DIRM has no offset table to update.
        return Ok(());
    }
    let nfiles = u16::from_be_bytes([data[1], data[2]]) as usize;
    if nfiles != new_offsets.len() {
        return Err(MutError::DirmComponentCountMismatch {
            dirm: nfiles,
            children: new_offsets.len(),
        });
    }
    let needed = 3usize
        .checked_add(4 * nfiles)
        .ok_or(MutError::DirmMalformed("DIRM offset table size overflow"))?;
    if data.len() < needed {
        return Err(MutError::DirmMalformed("DIRM offset table truncated"));
    }
    for (i, &off) in new_offsets.iter().enumerate() {
        let base = 3 + i * 4;
        data[base..base + 4].copy_from_slice(&off.to_be_bytes());
    }
    Ok(())
}

/// A mutable handle to one page's `FORM:DJVU` chunk inside a
/// [`DjVuDocumentMut`]. Returned by [`DjVuDocumentMut::page_mut`].
///
/// Each setter replaces the corresponding chunk in place, or appends a new
/// chunk if the page does not have one yet. The compressed `*z` chunk variant
/// is preferred on insert (TXTz / ANTz / METz) for size; if an existing
/// uncompressed `*a` chunk is present, the setter replaces *that* chunk and
/// upgrades its identifier to the `*z` form.
pub struct PageMut<'doc> {
    form: &'doc mut Chunk,
    dirty: &'doc mut bool,
}

impl PageMut<'_> {
    /// Replace (or insert) the page's text layer with the BZZ-compressed
    /// `TXTz` form of `layer`. Page height is read from the page's `INFO`
    /// chunk; missing INFO yields [`MutError::MissingPageInfo`].
    pub fn set_text_layer(&mut self, layer: &TextLayer) -> Result<(), MutError> {
        let info_data = self
            .find_leaf_data(b"INFO")
            .ok_or(MutError::MissingPageInfo)?;
        let info = PageInfo::parse(info_data)?;
        let plain = encode_text_layer(layer, info.height as u32);
        let compressed = crate::bzz_encode::bzz_encode(&plain);
        self.replace_or_insert_text(compressed);
        *self.dirty = true;
        Ok(())
    }

    /// Replace (or insert) the page's annotation chunk with the
    /// BZZ-compressed `ANTz` form of `(annotation, areas)`.
    pub fn set_annotations(&mut self, annotation: &Annotation, areas: &[MapArea]) {
        let bytes = encode_annotations_bzz(annotation, areas);
        self.replace_or_insert(b"ANTa", b"ANTz", bytes);
        *self.dirty = true;
    }

    /// Replace (or insert) the page's metadata chunk with the
    /// BZZ-compressed `METz` form of `meta`. An empty `meta` value removes
    /// any existing METa/METz chunk.
    pub fn set_metadata(&mut self, meta: &DjVuMetadata) {
        let bytes = encode_metadata_bzz(meta);
        self.replace_or_insert(b"METa", b"METz", bytes);
        *self.dirty = true;
    }

    fn find_leaf_data(&self, id: &[u8; 4]) -> Option<&[u8]> {
        for child in self.form.children() {
            if let Chunk::Leaf { id: cid, data } = child
                && cid == id
            {
                return Some(data);
            }
        }
        None
    }

    /// Replace either the `*a` or `*z` variant of a chunk pair, picking `*z`
    /// (compressed) for any newly inserted chunk. If `data` is empty, removes
    /// the existing chunk (whichever variant is present) and does not insert.
    fn replace_or_insert(&mut self, id_a: &[u8; 4], id_z: &[u8; 4], data: Vec<u8>) {
        let children = match self.form {
            Chunk::Form { children, .. } => children,
            Chunk::Leaf { .. } => unreachable!("PageMut wraps a FORM"),
        };
        let pos = children
            .iter()
            .position(|c| matches!(c, Chunk::Leaf { id, .. } if id == id_a || id == id_z));
        match (pos, data.is_empty()) {
            (Some(i), true) => {
                children.remove(i);
            }
            (Some(i), false) => {
                children[i] = Chunk::Leaf { id: *id_z, data };
            }
            (None, true) => { /* nothing to remove and nothing to insert */ }
            (None, false) => {
                children.push(Chunk::Leaf { id: *id_z, data });
            }
        }
    }

    /// TXTa / TXTz variant of `replace_or_insert` (kept separate for clarity).
    fn replace_or_insert_text(&mut self, data: Vec<u8>) {
        self.replace_or_insert(b"TXTa", b"TXTz", data);
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn corpus_path(name: &str) -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("tests/fixtures");
        p.push(name);
        p
    }

    fn read_corpus(name: &str) -> Vec<u8> {
        std::fs::read(corpus_path(name)).expect("corpus fixture missing")
    }

    /// Round-trip without edits is byte-identical on a single-page document.
    #[test]
    fn roundtrip_byte_identical_chicken() {
        let original = read_corpus("chicken.djvu");
        let doc = DjVuDocumentMut::from_bytes(&original).unwrap();
        assert!(!doc.is_dirty());
        assert_eq!(doc.into_bytes(), original);
    }

    /// Round-trip without edits is byte-identical on a bilevel JB2 document.
    #[test]
    fn roundtrip_byte_identical_boy_jb2() {
        let original = read_corpus("boy_jb2.djvu");
        let doc = DjVuDocumentMut::from_bytes(&original).unwrap();
        assert_eq!(doc.into_bytes(), original);
    }

    /// Round-trip without edits is byte-identical on a multi-page DJVM bundle.
    #[test]
    fn roundtrip_byte_identical_djvm_bundle() {
        let original = read_corpus("DjVu3Spec_bundled.djvu");
        let doc = DjVuDocumentMut::from_bytes(&original).unwrap();
        assert_eq!(doc.root_form_type(), Some(b"DJVM"));
        assert_eq!(doc.into_bytes(), original);
    }

    /// Round-trip without edits is byte-identical on a navm/fgbz document.
    #[test]
    fn roundtrip_byte_identical_navm() {
        let original = read_corpus("navm_fgbz.djvu");
        let doc = DjVuDocumentMut::from_bytes(&original).unwrap();
        assert_eq!(doc.into_bytes(), original);
    }

    /// `replace_leaf` mutates in place and the serialised output reflects it.
    #[test]
    fn replace_leaf_changes_emitted_bytes() {
        let original = read_corpus("chicken.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();

        // Walk to the first leaf — for chicken.djvu (FORM:DJVU) this is INFO.
        let first = doc.chunk_at_path(&[0]).unwrap();
        let original_first_data = first.data().to_vec();
        assert!(!original_first_data.is_empty());

        // Replace with a marker and serialise.
        let marker = b"PR1_TEST_MARKER".to_vec();
        doc.replace_leaf(&[0], marker.clone()).unwrap();
        assert!(doc.is_dirty());

        let edited = doc.into_bytes();

        // Re-parse the edited bytes and confirm the leaf payload changed.
        let reparsed = DjVuDocumentMut::from_bytes(&edited).unwrap();
        let new_first = reparsed.chunk_at_path(&[0]).unwrap();
        assert_eq!(new_first.data(), marker.as_slice());
    }

    #[test]
    fn replace_leaf_rejects_empty_path() {
        let original = read_corpus("chicken.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();
        let err = doc.replace_leaf(&[], vec![]).unwrap_err();
        assert!(matches!(err, MutError::EmptyPath));
    }

    #[test]
    fn replace_leaf_rejects_out_of_range() {
        let original = read_corpus("chicken.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();
        let err = doc.replace_leaf(&[9999], vec![]).unwrap_err();
        assert!(matches!(err, MutError::PathOutOfRange { .. }));
    }

    #[test]
    fn replace_leaf_rejects_traversing_leaf() {
        let original = read_corpus("chicken.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();
        // [0] is a leaf (INFO).  [0, 0] tries to descend past it.
        let err = doc.replace_leaf(&[0, 0], vec![]).unwrap_err();
        assert!(matches!(err, MutError::PathTraversesLeaf { .. }));
    }

    #[test]
    fn replace_leaf_rejects_form_target() {
        // For a DJVM bundle, [N] for some N points at a FORM:DJVU page,
        // not a leaf.  Picking the last child of DjVu3Spec_bundled (which
        // is a page FORM) demonstrates NotALeaf.
        let original = read_corpus("DjVu3Spec_bundled.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();
        let last_idx = doc.root_child_count() - 1;
        let err = doc.replace_leaf(&[last_idx], vec![]).unwrap_err();
        assert!(matches!(err, MutError::NotALeaf));
    }

    #[test]
    fn root_form_type_djvu_single_page() {
        let original = read_corpus("chicken.djvu");
        let doc = DjVuDocumentMut::from_bytes(&original).unwrap();
        assert_eq!(doc.root_form_type(), Some(b"DJVU"));
    }

    // ---- PR2 setters ------------------------------------------------------

    #[test]
    fn page_count_single_page_djvu_is_one() {
        let original = read_corpus("chicken.djvu");
        let doc = DjVuDocumentMut::from_bytes(&original).unwrap();
        assert_eq!(doc.page_count(), 1);
    }

    #[test]
    fn page_count_djvm_bundle_counts_djvu_components_only() {
        let original = read_corpus("DjVu3Spec_bundled.djvu");
        let doc = DjVuDocumentMut::from_bytes(&original).unwrap();
        // The bundle has multiple FORM:DJVU pages; assert it's > 1 and matches
        // the count of DJVU children at the root.
        let direct: usize = doc
            .file
            .root
            .children()
            .iter()
            .filter(|c| {
                matches!(c, crate::iff::Chunk::Form { secondary_id, .. } if secondary_id == b"DJVU")
            })
            .count();
        assert!(direct >= 2, "expected multi-page bundle, got {direct}");
        assert_eq!(doc.page_count(), direct);
    }

    #[test]
    fn page_mut_out_of_range_errors() {
        let original = read_corpus("chicken.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();
        let err = doc.page_mut(1).err().unwrap();
        assert!(matches!(
            err,
            MutError::PageOutOfRange { index: 1, count: 1 }
        ));
    }

    #[test]
    fn page_mut_djvm_bundle_succeeds_after_pr3() {
        // PR3 enables page_mut on bundled FORM:DJVM. Verify it returns a
        // valid handle for index 0 and rejects out-of-range indices.
        let original = read_corpus("DjVu3Spec_bundled.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();
        assert!(doc.page_mut(0).is_ok());
        let count = doc.page_count();
        let err = doc.page_mut(count).err().unwrap();
        assert!(matches!(err, MutError::PageOutOfRange { .. }));
    }

    #[test]
    fn set_text_layer_roundtrip_chicken() {
        use crate::text::{Rect, TextLayer, TextZone, TextZoneKind};

        let original = read_corpus("chicken.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();

        let layer = TextLayer {
            text: "hello world".to_string(),
            zones: vec![TextZone {
                kind: TextZoneKind::Page,
                rect: Rect {
                    x: 0,
                    y: 0,
                    width: 100,
                    height: 50,
                },
                text: "hello world".to_string(),
                children: vec![],
            }],
        };
        doc.page_mut(0).unwrap().set_text_layer(&layer).unwrap();
        assert!(doc.is_dirty());
        let edited = doc.into_bytes();

        // Re-parse and confirm a TXTz chunk now exists.
        let reparsed = DjVuDocumentMut::from_bytes(&edited).unwrap();
        let has_txtz = reparsed
            .file
            .root
            .children()
            .iter()
            .any(|c| matches!(c, Chunk::Leaf { id, .. } if id == b"TXTz"));
        assert!(
            has_txtz,
            "TXTz chunk should be present after set_text_layer"
        );
    }

    #[test]
    fn set_annotations_roundtrip_chicken() {
        use crate::annotation::{Annotation, Color};

        let original = read_corpus("chicken.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();

        let mut ann = Annotation::default();
        ann.background = Some(Color {
            r: 0xFF,
            g: 0xFF,
            b: 0xFF,
        });
        ann.mode = Some("color".to_string());
        doc.page_mut(0).unwrap().set_annotations(&ann, &[]);
        let edited = doc.into_bytes();

        let reparsed = DjVuDocumentMut::from_bytes(&edited).unwrap();
        let antz = reparsed
            .file
            .root
            .children()
            .iter()
            .find(|c| matches!(c, Chunk::Leaf { id, .. } if id == b"ANTz"));
        assert!(antz.is_some(), "ANTz should be inserted");
        let data = antz.unwrap().data();
        let (parsed_ann, _areas) =
            crate::annotation::parse_annotations_bzz(data).expect("ANTz must round-trip");
        assert_eq!(parsed_ann.mode.as_deref(), Some("color"));
        assert_eq!(
            parsed_ann.background,
            Some(Color {
                r: 0xFF,
                g: 0xFF,
                b: 0xFF
            })
        );
    }

    #[test]
    fn set_metadata_roundtrip_chicken() {
        let original = read_corpus("chicken.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();

        let mut meta = DjVuMetadata::default();
        meta.title = Some("Test Title".into());
        meta.author = Some("Tester".into());
        doc.page_mut(0).unwrap().set_metadata(&meta);
        let edited = doc.into_bytes();

        let reparsed = DjVuDocumentMut::from_bytes(&edited).unwrap();
        let metz = reparsed
            .file
            .root
            .children()
            .iter()
            .find(|c| matches!(c, Chunk::Leaf { id, .. } if id == b"METz"))
            .expect("METz should be inserted");
        let parsed = crate::metadata::parse_metadata_bzz(metz.data()).unwrap();
        assert_eq!(parsed, meta);
    }

    #[test]
    fn set_metadata_empty_removes_existing_chunk() {
        let original = read_corpus("chicken.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();

        // Insert one, then clear.
        let mut meta = DjVuMetadata::default();
        meta.title = Some("X".into());
        doc.page_mut(0).unwrap().set_metadata(&meta);
        doc.page_mut(0)
            .unwrap()
            .set_metadata(&DjVuMetadata::default());

        let edited = doc.into_bytes();
        let reparsed = DjVuDocumentMut::from_bytes(&edited).unwrap();
        let any_meta = reparsed
            .file
            .root
            .children()
            .iter()
            .any(|c| matches!(c, Chunk::Leaf { id, .. } if id == b"METa" || id == b"METz"));
        assert!(!any_meta, "set_metadata(empty) should remove any METa/METz");
    }

    #[test]
    fn set_metadata_replaces_existing_chunk_in_place() {
        let original = read_corpus("chicken.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();

        let mut m1 = DjVuMetadata::default();
        m1.title = Some("First".into());
        doc.page_mut(0).unwrap().set_metadata(&m1);

        let mut m2 = DjVuMetadata::default();
        m2.title = Some("Second".into());
        doc.page_mut(0).unwrap().set_metadata(&m2);

        let edited = doc.into_bytes();
        let reparsed = DjVuDocumentMut::from_bytes(&edited).unwrap();
        let metz_count = reparsed
            .file
            .root
            .children()
            .iter()
            .filter(|c| matches!(c, Chunk::Leaf { id, .. } if id == b"METa" || id == b"METz"))
            .count();
        assert_eq!(metz_count, 1, "should not duplicate METz on repeat set");
    }

    // ---- PR3: bundled DJVM mutation + set_bookmarks -----------------------

    /// Helper: parse the FORM:DJVM body, return the DIRM chunk's offset table
    /// and the actual file offsets where each component FORM header sits.
    fn dirm_offsets_and_actual(data: &[u8]) -> (Vec<u32>, Vec<u32>) {
        // Parse top-level FORM
        let form = crate::iff::parse_form(data).expect("parse_form");
        assert_eq!(&form.form_type, b"DJVM");

        let dirm = form
            .chunks
            .iter()
            .find(|c| &c.id == b"DIRM")
            .expect("DIRM present");
        let payload = dirm.data;
        let nfiles = u16::from_be_bytes([payload[1], payload[2]]) as usize;
        let mut declared = Vec::with_capacity(nfiles);
        for i in 0..nfiles {
            let base = 3 + i * 4;
            declared.push(u32::from_be_bytes([
                payload[base],
                payload[base + 1],
                payload[base + 2],
                payload[base + 3],
            ]));
        }

        // Walk the file to find each FORM child's absolute byte offset.
        // Layout: AT&T(4) FORM(4) length(4) DJVM(4) chunks…
        let mut actual = Vec::with_capacity(nfiles);
        let mut pos = 16usize;
        let body_end = 8 + u32::from_be_bytes([data[8], data[9], data[10], data[11]]) as usize;
        while pos < body_end {
            let id = &data[pos..pos + 4];
            let len =
                u32::from_be_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
                    as usize;
            if id == b"FORM" {
                actual.push(pos as u32);
            }
            let mut next = pos + 8 + len;
            if next & 1 == 1 {
                next += 1;
            }
            pos = next;
        }
        (declared, actual)
    }

    #[test]
    fn dirm_offsets_match_actual_after_no_edit() {
        // Sanity: even without edits, the recompute path agrees with the
        // original document layout on a real bundle.
        let original = read_corpus("DjVu3Spec_bundled.djvu");
        let (declared, actual) = dirm_offsets_and_actual(&original);
        assert_eq!(declared, actual);
    }

    #[test]
    fn dirm_offsets_recomputed_after_page_metadata_edit() {
        let original = read_corpus("DjVu3Spec_bundled.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();

        // Edit page 0's metadata so the page FORM grows.
        let mut meta = DjVuMetadata::default();
        meta.title = Some("PR3 DJVM bundled mutation".into());
        meta.author = Some("djvu-rs PR3 tests".into());
        doc.page_mut(0).unwrap().set_metadata(&meta);
        assert!(doc.is_dirty());

        let edited = doc.into_bytes();
        // Sizes must have changed (metadata chunk was inserted).
        assert_ne!(edited.len(), original.len());

        // DIRM offsets in the new bytes must match where the FORM headers
        // actually live.
        let (declared, actual) = dirm_offsets_and_actual(&edited);
        assert_eq!(
            declared, actual,
            "DIRM offsets must point at the new FORM positions after edit"
        );

        // The full document must still parse via DjVuDocument and expose the
        // expected page count.
        let reparsed =
            crate::djvu_document::DjVuDocument::parse(&edited).expect("edited bundle must parse");
        let original_doc =
            crate::djvu_document::DjVuDocument::parse(&original).expect("original bundle parses");
        assert_eq!(reparsed.page_count(), original_doc.page_count());
    }

    #[test]
    fn dirm_offsets_recomputed_after_middle_page_edit() {
        // Editing a non-first page must shift only the trailing offsets.
        let original = read_corpus("DjVu3Spec_bundled.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();
        let count = doc.page_count();
        assert!(count >= 3);

        let mid = count / 2;
        let mut meta = DjVuMetadata::default();
        meta.title = Some("PR3 mid-page edit".into());
        doc.page_mut(mid).unwrap().set_metadata(&meta);

        let edited = doc.into_bytes();
        let (declared, actual) = dirm_offsets_and_actual(&edited);
        assert_eq!(declared, actual);

        // Pages before `mid` should have unchanged offsets vs. the original.
        let (orig_declared, _) = dirm_offsets_and_actual(&original);
        for i in 0..mid {
            assert_eq!(
                declared[i], orig_declared[i],
                "offset for page {i} (before edit) must be unchanged"
            );
        }
    }

    #[test]
    fn set_bookmarks_replaces_navm_in_bundle() {
        use crate::djvu_document::DjVuBookmark;

        let original = read_corpus("DjVu3Spec_bundled.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();

        let bookmarks = vec![
            DjVuBookmark {
                title: "Front matter".into(),
                url: "#1".into(),
                children: vec![DjVuBookmark {
                    title: "Acknowledgments".into(),
                    url: "#3".into(),
                    children: vec![],
                }],
            },
            DjVuBookmark {
                title: "Body".into(),
                url: "#10".into(),
                children: vec![],
            },
        ];
        doc.set_bookmarks(&bookmarks).unwrap();
        assert!(doc.is_dirty());
        let edited = doc.into_bytes();

        // DIRM offsets must still be correct after the NAVM size change.
        let (declared, actual) = dirm_offsets_and_actual(&edited);
        assert_eq!(declared, actual);

        // Round-trip the bookmarks via the high-level DjVuDocument parser.
        let reparsed = crate::djvu_document::DjVuDocument::parse(&edited)
            .expect("bundle with new bookmarks parses");
        let parsed_bms = reparsed.bookmarks();
        assert_eq!(parsed_bms.len(), 2);
        assert_eq!(parsed_bms[0].title, "Front matter");
        assert_eq!(parsed_bms[0].children.len(), 1);
        assert_eq!(parsed_bms[0].children[0].title, "Acknowledgments");
        assert_eq!(parsed_bms[1].title, "Body");
    }

    #[test]
    fn set_bookmarks_empty_removes_navm() {
        let original = read_corpus("DjVu3Spec_bundled.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();
        // The fixture might or might not have NAVM; either way, calling with
        // an empty slice should result in no NAVM in the output.
        doc.set_bookmarks(&[]).unwrap();
        let edited = doc.into_bytes();

        let form = crate::iff::parse_form(&edited).unwrap();
        let has_navm = form.chunks.iter().any(|c| &c.id == b"NAVM");
        assert!(!has_navm, "set_bookmarks(&[]) must remove NAVM");

        // DIRM offsets still match.
        let (declared, actual) = dirm_offsets_and_actual(&edited);
        assert_eq!(declared, actual);
    }

    #[test]
    fn set_bookmarks_inserts_navm_when_absent() {
        use crate::djvu_document::DjVuBookmark;

        // Build a bundle that has no NAVM by first stripping it, then
        // re-add bookmarks via set_bookmarks.
        let original = read_corpus("DjVu3Spec_bundled.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();
        doc.set_bookmarks(&[]).unwrap();
        let stripped = doc.into_bytes();

        let mut doc = DjVuDocumentMut::from_bytes(&stripped).unwrap();
        let bms = vec![DjVuBookmark {
            title: "Re-added".into(),
            url: "#1".into(),
            children: vec![],
        }];
        doc.set_bookmarks(&bms).unwrap();
        let edited = doc.into_bytes();

        let form = crate::iff::parse_form(&edited).unwrap();
        let navm_pos = form
            .chunks
            .iter()
            .position(|c| &c.id == b"NAVM")
            .expect("NAVM should be inserted");
        let dirm_pos = form.chunks.iter().position(|c| &c.id == b"DIRM").unwrap();
        assert_eq!(
            navm_pos,
            dirm_pos + 1,
            "NAVM should be placed immediately after DIRM"
        );

        let (declared, actual) = dirm_offsets_and_actual(&edited);
        assert_eq!(declared, actual);
    }

    #[test]
    fn set_bookmarks_on_single_page_djvu_errors() {
        let original = read_corpus("chicken.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();
        let err = doc.set_bookmarks(&[]).err().unwrap();
        assert!(matches!(err, MutError::BookmarksRequireDjvm));
    }

    #[test]
    fn page_mut_djvm_text_layer_roundtrip() {
        use crate::text::{Rect, TextLayer, TextZone, TextZoneKind};

        let original = read_corpus("DjVu3Spec_bundled.djvu");
        let mut doc = DjVuDocumentMut::from_bytes(&original).unwrap();
        let layer = TextLayer {
            text: "djvm page-3 text".into(),
            zones: vec![TextZone {
                kind: TextZoneKind::Page,
                rect: Rect {
                    x: 0,
                    y: 0,
                    width: 100,
                    height: 50,
                },
                text: "djvm page-3 text".into(),
                children: vec![],
            }],
        };
        doc.page_mut(2).unwrap().set_text_layer(&layer).unwrap();
        let edited = doc.into_bytes();

        let (declared, actual) = dirm_offsets_and_actual(&edited);
        assert_eq!(declared, actual);

        // Re-open and confirm the targeted page now has a TXTz chunk.
        let reparsed = DjVuDocumentMut::from_bytes(&edited).unwrap();
        // The third FORM:DJVU child should have a TXTz leaf.
        let mut djvu_seen = 0usize;
        let mut found_txtz = false;
        for child in reparsed.file.root.children() {
            if let Chunk::Form {
                secondary_id,
                children,
                ..
            } = child
                && secondary_id == b"DJVU"
            {
                if djvu_seen == 2 {
                    found_txtz = children
                        .iter()
                        .any(|c| matches!(c, Chunk::Leaf { id, .. } if id == b"TXTz"));
                    break;
                }
                djvu_seen += 1;
            }
        }
        assert!(
            found_txtz,
            "TXTz chunk should be present on page 2 after set_text_layer"
        );
    }
}
