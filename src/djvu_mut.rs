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

use crate::error::LegacyError;
use crate::iff::{self, Chunk, DjvuFile};

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
    /// [`iff::emit`] which reconstructs the IFF stream from the parsed tree.
    pub fn into_bytes(self) -> Vec<u8> {
        if self.dirty {
            iff::emit(&self.file)
        } else {
            self.original_bytes
        }
    }
}

#[cfg(test)]
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
}
