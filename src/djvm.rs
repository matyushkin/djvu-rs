//! DJVM document merge and split operations.
//!
//! Provides [`merge`] to combine multiple DjVu documents into a single
//! bundled DJVM, and [`split`] to extract page ranges from a document.
//!
//! [`merge`]: crate::djvm::merge
//! [`split`]: crate::djvm::split

#[cfg(not(feature = "std"))]
use alloc::{format, string::String, vec, vec::Vec};

use crate::dirm::{BUNDLED_FLAG, DirmComponentKind, DirmPayload};
use crate::error::IffError;
use crate::iff;
use crate::{ComponentGraph, ComponentNodeKind};

#[cfg(test)]
use crate::djvu_document::DjVuDocument;

/// Error type for DJVM merge, split, and conversion operations.
#[derive(Debug, thiserror::Error)]
pub enum DjvmError {
    /// IFF container parse error.
    #[error("IFF parse error: {0}")]
    Iff(#[from] IffError),

    /// Document model error.
    #[error("document error: {0}")]
    Doc(#[from] crate::djvu_document::DocError),

    /// No pages to merge.
    #[error("no pages to merge")]
    EmptyMerge,

    /// Page range is out of bounds.
    #[error("page range {start}..{end} is out of bounds (document has {count} pages)")]
    PageRangeOutOfBounds {
        start: usize,
        end: usize,
        count: usize,
    },

    /// The assembled document's FORM payload would exceed `u32::MAX` (4 GiB).
    #[error("merged document exceeds the 4 GiB IFF FORM limit")]
    OutputTooLarge,

    /// The input is not a bundled `FORM:DJVM` document.
    #[error("to_indirect requires a bundled FORM:DJVM document")]
    NotBundledDjvm,

    /// The `DIRM` payload is malformed or missing a required field.
    #[error("DIRM chunk is malformed: {0}")]
    DirmMalformed(&'static str),

    /// The bundled `DIRM` and embedded component count disagree.
    #[error("DIRM component count {dirm} does not match bundle child count {children}")]
    DirmComponentCountMismatch {
        /// Component count declared by `DIRM`.
        dirm: usize,
        /// Direct `FORM` children in the bundle.
        children: usize,
    },
}

/// An indirect `FORM:DJVM` index and the external component files it resolves.
pub struct IndirectDocument {
    /// The indirect `FORM:DJVM` index file bytes (`DIRM` has no bundled bit or
    /// offset table; document-level chunks such as `NAVM` are retained).
    pub index: Vec<u8>,
    /// One resolver-keyed standalone component file per `DIRM` entry, in
    /// directory order.
    pub components: Vec<(String, Vec<u8>)>,
}

/// The result of [`dedup_shared_components`].
pub struct ComponentDedup {
    /// The deduplicated bundled document.
    pub document: Vec<u8>,
    /// `(dropped_id, surviving_id)` for every merged duplicate, in DIRM order.
    pub merged: Vec<(String, String)>,
}

/// Merge byte-identical shared `FORM:DJVI` components in a bundled document,
/// redirecting `INCL` references to the surviving component. Pages and
/// thumbnails are never merged; only exact byte-for-byte duplicate shared
/// components are.
pub fn dedup_shared_components(bundled: &[u8]) -> Result<ComponentDedup, DjvmError> {
    let form = iff::parse_form(bundled)?;
    if form.form_type != *b"DJVM" {
        return Err(DjvmError::NotBundledDjvm);
    }

    let dirm_data = form
        .chunks
        .iter()
        .find(|chunk| chunk.id == *b"DIRM")
        .ok_or(DjvmError::DirmMalformed("bundled DJVM has no DIRM chunk"))?
        .data;
    let dirm = DirmPayload::decode(dirm_data).map_err(DjvmError::DirmMalformed)?;
    if !dirm.is_bundled() {
        return Err(DjvmError::NotBundledDjvm);
    }

    let directory = dirm.components();
    let component_forms = form
        .chunks
        .iter()
        .filter(|chunk| chunk.id == *b"FORM")
        .collect::<Vec<_>>();
    if component_forms.len() != directory.len() {
        return Err(DjvmError::DirmComponentCountMismatch {
            dirm: directory.len(),
            children: component_forms.len(),
        });
    }

    // A BTreeMap makes this grouping deterministic, while the first entry seen
    // for each byte payload is necessarily its lowest DIRM index.
    let mut survivor_by_payload = std::collections::BTreeMap::<Vec<u8>, usize>::new();
    let mut keep = vec![true; directory.len()];
    let mut merged = Vec::new();
    let mut dropped_to_survivor = std::collections::BTreeMap::new();

    for (index, (entry, component)) in directory.iter().zip(&component_forms).enumerate() {
        // Do not infer shareability from the FORM type alone: a malformed DIRM
        // could label a page or thumbnail as DJVI. Only a directory-declared
        // shared component with a DJVI body is eligible.
        if entry.kind != DirmComponentKind::Shared || !component.data.starts_with(b"DJVI") {
            continue;
        }

        if let Some(&survivor) = survivor_by_payload.get(component.data) {
            keep[index] = false;
            let surviving_id = directory[survivor].id.clone();
            merged.push((entry.id.clone(), surviving_id.clone()));
            dropped_to_survivor.insert(entry.id.clone(), surviving_id);
        } else {
            survivor_by_payload.insert(component.data.to_vec(), index);
        }
    }

    // Besides avoiding unnecessary DIRM metadata rewrites, this preserves the
    // source byte-for-byte when no duplicate is found.
    if merged.is_empty() {
        return Ok(ComponentDedup {
            document: bundled.to_vec(),
            merged,
        });
    }

    let mut components = Vec::new();
    let mut ids = Vec::new();
    let mut flags = Vec::new();
    for (index, (entry, component)) in directory.iter().zip(component_forms).enumerate() {
        if !keep[index] {
            continue;
        }

        let body = if component.data.starts_with(b"DJVU") || component.data.starts_with(b"DJVI") {
            rewrite_component_incls(component.data, &dropped_to_survivor)?
        } else {
            component.data.to_vec()
        };
        components.push(wrap_sub_form(&body));
        ids.push(entry.id.clone());
        flags.push(dirm_kind_flag(entry.kind));
    }

    let document_chunks = form
        .chunks
        .iter()
        .filter(|chunk| chunk.id != *b"DIRM" && chunk.id != *b"FORM")
        .map(|chunk| iff::Chunk::Leaf {
            id: chunk.id,
            data: chunk.data.to_vec(),
        })
        .collect::<Vec<_>>();
    let document = build_djvm_with_document_chunks(&components, &ids, &flags, &document_chunks)?;

    Ok(ComponentDedup { document, merged })
}

/// Rewrite INCL leaf payloads that name dropped components and return the
/// component FORM body. Unchanged forms retain their original body verbatim.
fn rewrite_component_incls(
    form_data: &[u8],
    dropped_to_survivor: &std::collections::BTreeMap<String, String>,
) -> Result<Vec<u8>, DjvmError> {
    let form_type = form_data
        .get(..4)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(DjvmError::DirmMalformed("component FORM body is too short"))?;
    let body = &form_data[4..];
    let chunks = iff::parse_form_body(body)?;
    let mut changed = false;
    let mut emitted_chunks = Vec::with_capacity(chunks.len());

    for chunk in chunks {
        let mut data = chunk.data.to_vec();
        if chunk.id == *b"INCL" {
            let id_end = data
                .iter()
                .rposition(|byte| *byte != 0 && !byte.is_ascii_whitespace())
                .map_or(0, |index| index + 1);
            if let Ok(id) = core::str::from_utf8(&data[..id_end])
                && let Some(survivor) = dropped_to_survivor.get(id)
            {
                let mut rewritten = survivor.as_bytes().to_vec();
                rewritten.extend_from_slice(&data[id_end..]);
                data = rewritten;
                changed = true;
            }
        }
        emitted_chunks.push(iff::Chunk::Leaf { id: chunk.id, data });
    }

    if !changed {
        return Ok(form_data.to_vec());
    }

    let parts = emitted_chunks
        .iter()
        .map(iff::EmitPart::Chunk)
        .collect::<Vec<_>>();
    let emitted = iff::partial_emit(form_type, &parts).ok_or(DjvmError::OutputTooLarge)?;
    let length = u32::from_be_bytes(
        emitted[8..12]
            .try_into()
            .expect("IFF emitter always writes a FORM length"),
    ) as usize;
    Ok(emitted[12..12 + length].to_vec())
}

fn dirm_kind_flag(kind: DirmComponentKind) -> u8 {
    match kind {
        DirmComponentKind::Shared => 0,
        DirmComponentKind::Page => 1,
        DirmComponentKind::Thumbnail => 2,
    }
}

/// Re-serialize a sub-FORM child — the raw `data` of a `FORM` chunk, which
/// begins with its 4-byte form type — back into a standalone `AT&T`-prefixed
/// FORM document. Inverse of [`strip_att`].
fn wrap_sub_form(form_data: &[u8]) -> Vec<u8> {
    // `form_data` is a FORM body: it begins with the 4-byte secondary id
    // (DJVU/DJVI/…) followed by the chunks. Route the AT&T/FORM/length framing
    // through the emission seam rather than hand-assembling it. A well-formed
    // FORM body is even-length (every inner chunk is word-aligned), so the seam
    // reproduces the original bytes exactly; a malformed odd body merely gains a
    // trailing pad, which re-parses identically.
    let split = form_data.len().min(4);
    let (id_bytes, body) = form_data.split_at(split);
    let mut secondary_id = *b"    ";
    secondary_id[..id_bytes.len()].copy_from_slice(id_bytes);
    iff::partial_emit(secondary_id, &[iff::EmitPart::Verbatim(body)])
        .expect("sub-FORM fits within the 4 GiB IFF FORM limit")
}

/// Strip a leading `AT&T` magic from a standalone FORM document, yielding the
/// `FORM`-chunk bytes to embed inside a DJVM bundle. Inverse of [`wrap_sub_form`].
fn strip_att(form: &[u8]) -> &[u8] {
    if form.len() >= 4 && &form[..4] == b"AT&T" {
        &form[4..]
    } else {
        form
    }
}

/// Convert a bundled `FORM:DJVM` into its indirect index and standalone
/// component files.
///
/// The returned index retains each document-level non-`FORM` chunk (including
/// `NAVM`). Its `DIRM` is the source directory with only the bundled bit and
/// offset table removed: the BZZ-compressed metadata tail is retained verbatim,
/// so component ids, names, titles, and flags remain stable. Each returned
/// component is a complete `AT&T`-prefixed `FORM:DJVU`, `FORM:DJVI`, or
/// `FORM:THUM` file suitable for [`crate::djvu_document::ComponentResolver`].
pub fn to_indirect(bundled: &[u8]) -> Result<IndirectDocument, DjvmError> {
    let form = iff::parse_form(bundled)?;
    if form.form_type != *b"DJVM" {
        return Err(DjvmError::NotBundledDjvm);
    }

    let dirm_data = form
        .chunks
        .iter()
        .find(|chunk| chunk.id == *b"DIRM")
        .ok_or(DjvmError::DirmMalformed("bundled DJVM has no DIRM chunk"))?
        .data;
    let mut dirm = DirmPayload::decode(dirm_data).map_err(DjvmError::DirmMalformed)?;
    if !dirm.is_bundled() {
        return Err(DjvmError::NotBundledDjvm);
    }

    let component_forms = form
        .chunks
        .iter()
        .filter(|chunk| chunk.id == *b"FORM")
        .collect::<Vec<_>>();
    let expected_count = dirm.nfiles as usize;
    if component_forms.len() != expected_count {
        return Err(DjvmError::DirmComponentCountMismatch {
            dirm: expected_count,
            children: component_forms.len(),
        });
    }

    // The BZZ metadata tail is opaque here. Decoding it only supplies the
    // resolver keys; the re-emitted DIRM carries the original metadata bytes.
    let components = dirm
        .components()
        .into_iter()
        .zip(component_forms)
        .map(|(component, form)| (component.id, wrap_sub_form(form.data)))
        .collect();

    // Bundled DIRM layout is [flags][nfiles][offset table][BZZ metadata].
    // Clear only the bit that selects that layout; `encode` then omits the
    // table while preserving the metadata blob byte-for-byte.
    dirm.flags &= !BUNDLED_FLAG;
    dirm.offsets.clear();
    let indirect_dirm = dirm.encode();

    // Preserve document-level chunks (NAVM and any extensions) in their
    // original order while removing all embedded component FORMs. Re-frame
    // leaves through the IFF emission seam so length and padding are correct.
    let mut index_chunks = Vec::with_capacity(form.chunks.len() - expected_count);
    let mut replaced_dirm = false;
    for chunk in &form.chunks {
        match chunk.id {
            id if id == *b"FORM" => {}
            id if id == *b"DIRM" && !replaced_dirm => {
                index_chunks.push(iff::Chunk::Leaf {
                    id: *b"DIRM",
                    data: indirect_dirm.clone(),
                });
                replaced_dirm = true;
            }
            id if id == *b"DIRM" => {}
            id => index_chunks.push(iff::Chunk::Leaf {
                id,
                data: chunk.data.to_vec(),
            }),
        }
    }
    debug_assert!(replaced_dirm, "the DIRM was found above");
    let index_parts = index_chunks
        .iter()
        .map(iff::EmitPart::Chunk)
        .collect::<Vec<_>>();
    let index = iff::partial_emit(*b"DJVM", &index_parts).ok_or(DjvmError::OutputTooLarge)?;

    Ok(IndirectDocument { index, components })
}

/// Merge multiple DjVu documents (raw bytes) into a single bundled DJVM.
///
/// Each input document contributes all its pages to the output.
/// Shared dictionaries (DJVI components) are included and INCL
/// references are preserved within each source document's pages.
pub fn merge(documents: &[&[u8]]) -> Result<Vec<u8>, DjvmError> {
    if documents.is_empty() {
        return Err(DjvmError::EmptyMerge);
    }

    let mut components: Vec<Vec<u8>> = Vec::new();
    let mut component_ids: Vec<String> = Vec::new();
    let mut component_flags: Vec<u8> = Vec::new();

    for (doc_idx, &doc_data) in documents.iter().enumerate() {
        let form = iff::parse_form(doc_data)?;

        if &form.form_type == b"DJVU" {
            // Single-page document — the whole file is one page
            components.push(doc_data.to_vec());
            component_ids.push(format!("p{:04}.djvu", components.len()));
            component_flags.push(1); // page
        } else if &form.form_type == b"DJVM" {
            // Multi-page bundled document — extract each FORM child
            for chunk in &form.chunks {
                if &chunk.id == b"FORM" && chunk.data.len() >= 4 {
                    let child_form_type = &chunk.data[..4];

                    let flag = if child_form_type == b"DJVI" { 0 } else { 1 }; // 0 = shared, 1 = page

                    components.push(wrap_sub_form(chunk.data));
                    component_ids.push(format!("d{}p{:04}.djvu", doc_idx, components.len()));
                    component_flags.push(flag);
                }
            }
        }
    }

    if components.is_empty() {
        return Err(DjvmError::EmptyMerge);
    }

    build_djvm(&components, &component_ids, &component_flags)
}

/// Split a document, extracting pages in the given range (0-based, exclusive end).
///
/// Returns raw DjVu bytes for a new document containing only the requested pages.
pub fn split(doc_data: &[u8], start: usize, end: usize) -> Result<Vec<u8>, DjvmError> {
    let form = iff::parse_form(doc_data)?;

    // Page count derived from the same FORM walk used for extraction below, so
    // the bounds check can never disagree with what is actually present (a
    // DIRM-based page count and the FORM:DJVU children can diverge).
    let count = match &form.form_type {
        b"DJVU" => 1,
        b"DJVM" => form
            .chunks
            .iter()
            .filter(|c| &c.id == b"FORM" && c.data.len() >= 4 && &c.data[..4] == b"DJVU")
            .count(),
        _ => 0,
    };

    if start >= count || end > count || start >= end {
        return Err(DjvmError::PageRangeOutOfBounds { start, end, count });
    }

    // Single-page document: just return the whole thing
    if &form.form_type == b"DJVU" && start == 0 && end == 1 {
        return Ok(doc_data.to_vec());
    }

    // For a single page extraction from a multi-page document with no shared
    // dependencies, return the standalone `FORM:DJVU`. If the page INCLs shared
    // components, fall through to the graph closure path below so it is bundled
    // with its dependencies — a bare page would carry dangling INCL references.
    if end - start == 1 && &form.form_type == b"DJVM" {
        let standalone = ComponentGraph::parse(doc_data)
            .ok()
            .and_then(|graph| {
                let pages = graph
                    .nodes()
                    .iter()
                    .filter(|node| node.kind == ComponentNodeKind::Page)
                    .collect::<Vec<_>>();
                // Only trust the graph when its page count agrees with the FORM
                // walk; otherwise keep the historical standalone behaviour.
                (pages.len() == count)
                    .then(|| pages.get(start).map(|page| page.includes.is_empty()))
                    .flatten()
            })
            .unwrap_or(true);

        if standalone {
            let mut page_idx = 0;
            for chunk in &form.chunks {
                if &chunk.id == b"FORM" && chunk.data.len() >= 4 && &chunk.data[..4] == b"DJVU" {
                    if page_idx == start {
                        return Ok(wrap_sub_form(chunk.data));
                    }
                    page_idx += 1;
                }
            }
        }
    }

    // Multiple pages: when the bundled component graph is available, retain
    // just the selected pages and their transitive INCL dependencies.  DIRM
    // identities must survive this rewrite: INCL chunks name those ids.
    if let Ok(graph) = ComponentGraph::parse(doc_data) {
        let pages = graph
            .nodes()
            .iter()
            .filter(|node| node.kind == ComponentNodeKind::Page)
            .collect::<Vec<_>>();

        // `count` is intentionally derived from the FORM walk above for
        // compatibility.  If a graph that otherwise parses has a different
        // page count, keep the established extraction path below.
        if pages.len() == count {
            let roots = pages[start..end]
                .iter()
                .map(|node| node.id.as_str())
                .collect::<Vec<_>>();
            let closure = graph.transitive_closure(&roots);
            let mut selected = vec![false; graph.nodes().len()];
            for node_index in closure {
                let node = &graph.nodes()[node_index];
                // Thumbnails are deliberately excluded from split output.
                if node.kind != ComponentNodeKind::Thumbnail {
                    selected[node_index] = true;
                }
            }

            let component_forms = form
                .chunks
                .iter()
                .filter(|chunk| chunk.id == *b"FORM")
                .collect::<Vec<_>>();
            let mut components = Vec::new();
            let mut component_ids = Vec::new();
            let mut component_flags = Vec::new();

            // The graph and reader both correlate DIRM entry i with embedded
            // FORM child i.  Iterating nodes keeps the output in DIRM order.
            for node in graph.nodes() {
                if selected[node.dirm_index] {
                    let component = component_forms[node.dirm_index];
                    components.push(wrap_sub_form(component.data));
                    component_ids.push(node.id.clone());
                    component_flags.push(u8::from(node.kind == ComponentNodeKind::Page));
                }
            }

            return build_djvm(&components, &component_ids, &component_flags);
        }
    }

    // Fallback for indirect, malformed, and otherwise non-graph DJVMs: keep
    // the historical FORM-based extraction behaviour.
    let mut components: Vec<Vec<u8>> = Vec::new();
    let mut component_ids: Vec<String> = Vec::new();
    let mut component_flags: Vec<u8> = Vec::new();

    // First pass: collect shared components (DJVI) that might be needed
    for chunk in &form.chunks {
        if &chunk.id == b"FORM" && chunk.data.len() >= 4 && &chunk.data[..4] == b"DJVI" {
            components.push(wrap_sub_form(chunk.data));
            component_ids.push(format!("shared{}.djvi", components.len()));
            component_flags.push(0); // shared
        }
    }

    // Second pass: collect pages in the requested range
    let mut page_idx = 0;
    for chunk in &form.chunks {
        if &chunk.id == b"FORM" && chunk.data.len() >= 4 && &chunk.data[..4] == b"DJVU" {
            if page_idx >= start && page_idx < end {
                components.push(wrap_sub_form(chunk.data));
                component_ids.push(format!("p{:04}.djvu", page_idx + 1));
                component_flags.push(1); // page
            }
            page_idx += 1;
        }
    }

    build_djvm(&components, &component_ids, &component_flags)
}

/// Build a bundled DJVM file from components.
///
/// The IFF framing — `FORM:DJVM` header, the `DIRM` chunk header, and the
/// even-byte padding between components — is delegated to [`iff::partial_emit`]
/// so this writer shares the one emission seam (#367). The DIRM goes through as
/// a re-framed [`iff::Chunk`]; each component is copied verbatim (its AT&T magic
/// stripped, since it is embedded, not a standalone file).
fn build_djvm(components: &[Vec<u8>], ids: &[String], flags: &[u8]) -> Result<Vec<u8>, DjvmError> {
    build_djvm_with_document_chunks(components, ids, flags, &[])
}

/// Build a bundled DJVM, retaining the supplied document-level chunks between
/// the rebuilt DIRM and embedded component FORMs.
fn build_djvm_with_document_chunks(
    components: &[Vec<u8>],
    ids: &[String],
    flags: &[u8],
    document_chunks: &[iff::Chunk],
) -> Result<Vec<u8>, DjvmError> {
    let n = components.len();

    // Each component includes the AT&T prefix — strip it for embedding. Its
    // remaining length (FORM header + payload) is the DIRM size-table entry.
    let stripped: Vec<&[u8]> = components.iter().map(|c| strip_att(c)).collect();
    let sizes: Vec<u32> = stripped
        .iter()
        .map(|s| u32::try_from(s.len()).unwrap_or(0))
        .collect();

    // Two-pass emission (the `partial_emit_with_offsets` contract): pass 1
    // learns where each component lands, pass 2 re-emits with the DIRM offset
    // table filled in. The offset table is fixed-width and the size table is
    // final from pass 1, so the layout cannot shift between passes. A zeroed
    // offset table is rejected by DjVuLibre's DjVmDir ("no indirect entries
    // allowed in bundled document", #657).
    let mut payload = DirmPayload::build_bundled(n, flags, ids, &sizes);
    let emit = |payload: &DirmPayload| -> Result<(Vec<u8>, Vec<usize>), DjvmError> {
        let dirm = iff::Chunk::Leaf {
            id: *b"DIRM",
            data: payload.encode(),
        };
        let mut parts: Vec<iff::EmitPart> = Vec::with_capacity(1 + document_chunks.len() + n);
        parts.push(iff::EmitPart::Chunk(&dirm));
        parts.extend(document_chunks.iter().map(iff::EmitPart::Chunk));
        parts.extend(stripped.iter().map(|s| iff::EmitPart::Verbatim(s)));
        iff::partial_emit_with_offsets(*b"DJVM", &parts).ok_or(DjvmError::OutputTooLarge)
    };

    let (_, offsets) = emit(&payload)?;
    payload.offsets = offsets[1 + document_chunks.len()..] // preceding parts are DIRM + document chunks
        .iter()
        .map(|&o| u32::try_from(o).map_err(|_| DjvmError::OutputTooLarge))
        .collect::<Result<_, _>>()?;
    let (bytes, second_offsets) = emit(&payload)?;
    debug_assert_eq!(offsets, second_offsets, "two-pass layout must be stable");
    Ok(bytes)
}

/// Create an indirect (non-bundled) DJVM index file that references pages as
/// separate files.
///
/// The returned bytes are a valid `FORM:DJVM` with a DIRM directory chunk whose
/// `is_bundled` flag is **not** set.  Each entry in `page_names` becomes one
/// `Page` component; there are no embedded `FORM:DJVU` sub-forms — the component
/// data lives in separate files that must be passed to a resolver when parsing.
///
/// Shared-dictionary (DJVI) components are not supported by this helper; use
/// [`merge`] to build a bundled document that includes them.
///
/// # Errors
///
/// Returns [`DjvmError::EmptyMerge`] if `page_names` is empty.
pub fn create_indirect(page_names: &[&str]) -> Result<Vec<u8>, DjvmError> {
    if page_names.is_empty() {
        return Err(DjvmError::EmptyMerge);
    }

    let count = page_names.len();
    let ids: Vec<String> = page_names.iter().map(|s| s.to_string()).collect();
    // All entries are pages (flag = 1)
    let flags: Vec<u8> = vec![1u8; count];

    // Indirect: a single DIRM chunk, no embedded component FORMs. Route the
    // DJVM framing through the emission seam (same path as the bundled build).
    let dirm = iff::Chunk::Leaf {
        id: *b"DIRM",
        data: DirmPayload::build_indirect(count, &flags, &ids).encode(),
    };
    iff::partial_emit(*b"DJVM", &[iff::EmitPart::Chunk(&dirm)]).ok_or(DjvmError::OutputTooLarge)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    struct SplitFixtureComponent {
        id: &'static str,
        dirm_flag: u8,
        form: [u8; 4],
        chunks: Vec<([u8; 4], Vec<u8>)>,
    }

    fn split_component(
        id: &'static str,
        dirm_flag: u8,
        form: [u8; 4],
        chunks: Vec<([u8; 4], Vec<u8>)>,
    ) -> SplitFixtureComponent {
        SplitFixtureComponent {
            id,
            dirm_flag,
            form,
            chunks,
        }
    }

    fn split_incl(id: &[u8]) -> ([u8; 4], Vec<u8>) {
        (*b"INCL", id.to_vec())
    }

    fn split_component_body(component: &SplitFixtureComponent) -> Vec<u8> {
        let chunks = component
            .chunks
            .iter()
            .map(|(id, data)| iff::Chunk::Leaf {
                id: *id,
                data: data.clone(),
            })
            .collect::<Vec<_>>();
        let parts = chunks.iter().map(iff::EmitPart::Chunk).collect::<Vec<_>>();
        let bytes = iff::partial_emit(component.form, &parts).expect("small fixture FORM");
        let length = u32::from_be_bytes(bytes[8..12].try_into().unwrap()) as usize;
        bytes[12..12 + length].to_vec()
    }

    fn split_bundled_fixture(components: Vec<SplitFixtureComponent>) -> Vec<u8> {
        split_bundled_fixture_with_document_chunks(components, vec![])
    }

    fn split_bundled_fixture_with_document_chunks(
        components: Vec<SplitFixtureComponent>,
        document_chunks: Vec<([u8; 4], Vec<u8>)>,
    ) -> Vec<u8> {
        let bodies = components
            .iter()
            .map(split_component_body)
            .collect::<Vec<_>>();
        let ids = components
            .iter()
            .map(|component| component.id.to_string())
            .collect::<Vec<_>>();
        let flags = components
            .iter()
            .map(|component| component.dirm_flag)
            .collect::<Vec<_>>();
        let sizes = bodies
            .iter()
            .map(|body| u32::try_from(8 + body.len()).unwrap())
            .collect::<Vec<_>>();
        let mut dirm = DirmPayload::build_bundled(components.len(), &flags, &ids, &sizes);
        let document_chunks = document_chunks
            .into_iter()
            .map(|(id, data)| iff::Chunk::Leaf { id, data })
            .collect::<Vec<_>>();

        let emit = |dirm: &DirmPayload| {
            let dirm_chunk = iff::Chunk::Leaf {
                id: *b"DIRM",
                data: dirm.encode(),
            };
            let mut parts = vec![iff::EmitPart::Chunk(&dirm_chunk)];
            parts.extend(document_chunks.iter().map(iff::EmitPart::Chunk));
            parts.extend(bodies.iter().map(|body| iff::EmitPart::Form(body)));
            iff::partial_emit_with_offsets(*b"DJVM", &parts).expect("small bundled fixture")
        };

        let (_, offsets) = emit(&dirm);
        dirm.offsets = offsets[1 + document_chunks.len()..]
            .iter()
            .map(|&offset| u32::try_from(offset).unwrap())
            .collect();
        emit(&dirm).0
    }

    fn split_dependency_fixture() -> Vec<u8> {
        split_bundled_fixture(vec![
            split_component("page0.djvu", 1, *b"DJVU", vec![split_incl(b"dictA.djvi")]),
            split_component("dictA.djvi", 0, *b"DJVI", vec![(*b"Djbz", vec![1])]),
            split_component("page1.djvu", 1, *b"DJVU", vec![split_incl(b"dictB.djvi")]),
            split_component("dictB.djvi", 0, *b"DJVI", vec![(*b"Djbz", vec![2])]),
            split_component("dictC.djvi", 0, *b"DJVI", vec![(*b"Djbz", vec![3])]),
        ])
    }

    #[test]
    fn dedup_shared_components_merges_identical_dicts_and_redirects_incls() {
        let bundled = split_bundled_fixture_with_document_chunks(
            vec![
                split_component("page0.djvu", 1, *b"DJVU", vec![split_incl(b"dictA.djvi")]),
                split_component("dictA.djvi", 0, *b"DJVI", vec![(*b"Djbz", vec![1])]),
                split_component("page1.djvu", 1, *b"DJVU", vec![split_incl(b"dictB.djvi")]),
                split_component("dictB.djvi", 0, *b"DJVI", vec![(*b"Djbz", vec![1])]),
                split_component("dictC.djvi", 0, *b"DJVI", vec![(*b"Djbz", vec![2])]),
            ],
            vec![(*b"NAVM", vec![1, 2, 3])],
        );
        let original_graph = ComponentGraph::parse(&bundled).expect("parse source graph");

        let result = dedup_shared_components(&bundled).expect("deduplicate bundled fixture");
        assert_eq!(
            result.merged,
            vec![("dictB.djvi".to_string(), "dictA.djvi".to_string())],
            "the first matching DIRM component survives"
        );

        let graph = ComponentGraph::parse(&result.document).expect("parse deduplicated graph");
        assert!(graph.node("dictA.djvi").is_some());
        assert!(graph.node("dictB.djvi").is_none());
        assert!(graph.node("dictC.djvi").is_some());
        for page in ["page0.djvu", "page1.djvu"] {
            assert_eq!(
                graph
                    .includes(page)
                    .into_iter()
                    .map(|node| node.id.as_str())
                    .collect::<Vec<_>>(),
                vec!["dictA.djvi"],
                "{page} now includes the surviving dictionary"
            );
        }
        assert!(
            graph
                .validate()
                .iter()
                .all(|error| !matches!(error, crate::GraphError::MissingTarget { .. })),
            "redirected INCL edges have no missing targets"
        );
        assert_eq!(
            graph
                .nodes()
                .iter()
                .filter(|node| node.kind == ComponentNodeKind::Page)
                .count(),
            original_graph
                .nodes()
                .iter()
                .filter(|node| node.kind == ComponentNodeKind::Page)
                .count(),
            "deduplication does not change the page count"
        );

        let document_chunks = iff::parse_form(&result.document)
            .expect("parse deduplicated document")
            .chunks
            .into_iter()
            .filter(|chunk| chunk.id != *b"DIRM" && chunk.id != *b"FORM")
            .map(|chunk| (chunk.id, chunk.data.to_vec()))
            .collect::<Vec<_>>();
        assert_eq!(document_chunks, vec![(*b"NAVM", vec![1, 2, 3])]);
    }

    #[test]
    fn dedup_shared_components_never_merges_different_dicts() {
        let bundled = split_bundled_fixture(vec![
            split_component("page0.djvu", 1, *b"DJVU", vec![split_incl(b"dictA.djvi")]),
            split_component("dictA.djvi", 0, *b"DJVI", vec![(*b"Djbz", vec![1])]),
            split_component("page1.djvu", 1, *b"DJVU", vec![split_incl(b"dictB.djvi")]),
            split_component("dictB.djvi", 0, *b"DJVI", vec![(*b"Djbz", vec![2])]),
        ]);

        let result = dedup_shared_components(&bundled).expect("deduplicate bundled fixture");
        assert!(result.merged.is_empty());
        let graph = ComponentGraph::parse(&result.document).expect("parse result graph");
        assert!(graph.node("dictA.djvi").is_some());
        assert!(graph.node("dictB.djvi").is_some());
        assert_eq!(
            graph
                .includes("page0.djvu")
                .into_iter()
                .map(|node| node.id.as_str())
                .collect::<Vec<_>>(),
            vec!["dictA.djvi"]
        );
        assert_eq!(
            graph
                .includes("page1.djvu")
                .into_iter()
                .map(|node| node.id.as_str())
                .collect::<Vec<_>>(),
            vec!["dictB.djvi"]
        );
    }

    #[test]
    fn dedup_shared_components_is_a_byte_preserving_no_op_without_duplicates() {
        let bundled = split_bundled_fixture(vec![
            split_component("page0.djvu", 1, *b"DJVU", vec![split_incl(b"dictA.djvi")]),
            split_component("dictA.djvi", 0, *b"DJVI", vec![(*b"Djbz", vec![1])]),
            split_component("dictB.djvi", 0, *b"DJVI", vec![(*b"Djbz", vec![2])]),
        ]);

        let result = dedup_shared_components(&bundled).expect("deduplicate bundled fixture");
        assert!(result.merged.is_empty());
        assert_eq!(
            result.document, bundled,
            "duplicate-free bundles are unchanged"
        );
        let graph = ComponentGraph::parse(&result.document).expect("parse result graph");
        assert!(
            graph
                .validate()
                .iter()
                .all(|error| !matches!(error, crate::GraphError::MissingTarget { .. }))
        );
    }

    #[test]
    fn dedup_shared_components_round_trips_bundled_fixture() {
        let bundled =
            std::fs::read(fixture_path("DjVu3Spec_bundled.djvu")).expect("bundled fixture exists");
        let original = ComponentGraph::parse(&bundled).expect("parse source graph");

        let result = dedup_shared_components(&bundled).expect("deduplicate fixture");
        let rewritten = ComponentGraph::parse(&result.document).expect("parse result graph");
        assert_eq!(
            rewritten
                .nodes()
                .iter()
                .filter(|node| node.kind == ComponentNodeKind::Page)
                .count(),
            original
                .nodes()
                .iter()
                .filter(|node| node.kind == ComponentNodeKind::Page)
                .count(),
            "deduplication preserves fixture page count"
        );
        assert!(
            rewritten
                .validate()
                .iter()
                .all(|error| !matches!(error, crate::GraphError::MissingTarget { .. })),
            "deduplicated fixture has no dangling INCL edges"
        );
    }

    #[test]
    fn to_indirect_round_trips_graph_dirm_metadata_and_shared_dictionaries() {
        use crate::djvu_document::{ComponentId, ComponentResolveError};

        let bundled = std::fs::read(fixture_path("DjVu3Spec_bundled.djvu"))
            .expect("DjVu3Spec_bundled fixture exists");
        let original_form = iff::parse_form(&bundled).expect("parse bundled fixture");
        let original_dirm = DirmPayload::decode(
            original_form
                .chunks
                .iter()
                .find(|chunk| chunk.id == *b"DIRM")
                .expect("bundled fixture has DIRM")
                .data,
        )
        .expect("decode bundled DIRM");
        let original_ids = original_dirm
            .components()
            .into_iter()
            .map(|component| component.id)
            .collect::<Vec<_>>();
        let original_graph =
            ComponentGraph::parse(&bundled).expect("build bundled component graph");
        assert_eq!(
            original_graph
                .nodes()
                .iter()
                .map(|node| node.id.as_str())
                .collect::<Vec<_>>(),
            original_ids.iter().map(String::as_str).collect::<Vec<_>>(),
            "the graph follows DIRM order"
        );
        assert!(
            original_graph
                .validate()
                .iter()
                .all(|error| !matches!(error, crate::GraphError::MissingTarget { .. })),
            "the source bundle has no dangling INCL edges"
        );

        let original_document = DjVuDocument::parse(&bundled).expect("parse bundled fixture");
        let pages_with_shared_dict = (0..original_document.page_count())
            .filter(|&index| {
                original_document
                    .page(index)
                    .expect("valid source page")
                    .decoded_shared_dict()
                    .is_some()
            })
            .collect::<Vec<_>>();
        assert!(
            !pages_with_shared_dict.is_empty(),
            "fixture must exercise shared DJVI resolution"
        );

        let indirect = to_indirect(&bundled).expect("convert bundled fixture");
        let index_form = iff::parse_form(&indirect.index).expect("parse indirect index");
        assert_eq!(&index_form.form_type, b"DJVM");
        assert!(
            index_form.chunks.iter().all(|chunk| chunk.id != *b"FORM"),
            "indirect index contains no embedded component forms"
        );
        let index_dirm = DirmPayload::decode(
            index_form
                .chunks
                .iter()
                .find(|chunk| chunk.id == *b"DIRM")
                .expect("indirect index has DIRM")
                .data,
        )
        .expect("decode indirect DIRM");
        assert!(!index_dirm.is_bundled(), "bundled bit is cleared");
        assert!(index_dirm.offsets.is_empty(), "offset table is removed");
        assert_eq!(index_dirm.nfiles, original_dirm.nfiles);
        assert_eq!(
            index_dirm.flags,
            original_dirm.flags & !BUNDLED_FLAG,
            "only the bundled bit changes"
        );
        assert_eq!(
            index_dirm.metadata, original_dirm.metadata,
            "the BZZ metadata blob, including ids/names/titles/component flags, is verbatim"
        );
        assert_eq!(
            indirect
                .components
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>(),
            original_ids.iter().map(String::as_str).collect::<Vec<_>>(),
            "one resolver-keyed file per DIRM entry, in DIRM order"
        );
        assert_eq!(indirect.components.len(), original_dirm.nfiles as usize);

        let original_document_chunks = original_form
            .chunks
            .iter()
            .filter(|chunk| chunk.id != *b"DIRM" && chunk.id != *b"FORM")
            .map(|chunk| (chunk.id, chunk.data))
            .collect::<Vec<_>>();
        let index_document_chunks = index_form
            .chunks
            .iter()
            .filter(|chunk| chunk.id != *b"DIRM" && chunk.id != *b"FORM")
            .map(|chunk| (chunk.id, chunk.data))
            .collect::<Vec<_>>();
        assert_eq!(
            index_document_chunks, original_document_chunks,
            "NAVM and every other document-level chunk survive in the index"
        );

        let component_map = indirect
            .components
            .iter()
            .cloned()
            .collect::<std::collections::BTreeMap<_, _>>();
        for node in original_graph.nodes() {
            let component = component_map
                .get(&node.id)
                .expect("every graph node has an extracted component");
            assert!(component.starts_with(b"AT&T"));
            let component_form =
                iff::parse_form(component).expect("component is a standalone FORM");
            let includes = component_form
                .chunks
                .iter()
                .filter(|chunk| chunk.id == *b"INCL")
                .map(|chunk| {
                    core::str::from_utf8(chunk.data.trim_ascii_end())
                        .expect("fixture INCL ids are UTF-8")
                })
                .collect::<Vec<_>>();
            let expected_includes = node
                .includes
                .iter()
                .map(|&target| original_graph.nodes()[target].id.as_str())
                .collect::<Vec<_>>();
            assert_eq!(
                includes, expected_includes,
                "INCL edges survive for {}",
                node.id
            );
        }

        let resolver = |component: &ComponentId| {
            component_map.get(&component.name).cloned().ok_or_else(|| {
                ComponentResolveError::Missing {
                    component: component.clone(),
                }
            })
        };
        let resolved = DjVuDocument::parse_with_component_resolver(&indirect.index, &resolver)
            .expect("parse converted indirect document");
        assert_eq!(resolved.page_count(), original_document.page_count());
        for index in pages_with_shared_dict {
            assert!(
                resolved
                    .page(index)
                    .expect("valid resolved page")
                    .decoded_shared_dict()
                    .is_some(),
                "page {index}'s INCL still resolves its shared dictionary"
            );
        }
    }

    #[test]
    fn to_indirect_rejects_an_indirect_djvm() {
        let indirect = create_indirect(&["page.djvu"]).expect("build indirect index");
        assert!(matches!(
            to_indirect(&indirect),
            Err(DjvmError::NotBundledDjvm)
        ));
    }

    #[test]
    fn merge_empty_returns_error() {
        let result = merge(&[]);
        assert!(result.is_err());
    }

    /// #657: merged bundles must carry a DjVuLibre-acceptable DIRM — version
    /// byte 0x81 (bundled, directory version 1), every offset non-zero and
    /// pointing at a component `FORM` tag, and the 24-bit size table matching
    /// each component's actual byte span. A zeroed offset table or version 0
    /// is rejected by DjVmDir ("no indirect entries allowed in bundled
    /// document").
    #[test]
    fn merge_dirm_offsets_sizes_and_version_are_djvulibre_clean() {
        let a = std::fs::read(fixture_path("navm_fgbz.djvu")).unwrap();
        let bytes = merge(&[&a, &a]).unwrap();

        assert_eq!(&bytes[16..20], b"DIRM");
        let dirm_len = u32::from_be_bytes(bytes[20..24].try_into().unwrap()) as usize;
        let payload = &bytes[24..24 + dirm_len];
        assert_eq!(payload[0], 0x81, "bundled bit + directory version 1");

        let nfiles = u16::from_be_bytes(payload[1..3].try_into().unwrap()) as usize;
        assert!(nfiles > 0);
        let dirm = DirmPayload::decode(payload).unwrap();
        let components = dirm.components();
        assert_eq!(components.len(), nfiles);
        for (c, &off) in components.iter().zip(&dirm.offsets) {
            assert_ne!(off, 0, "component {} has a zeroed offset", c.id);
            let off = off as usize;
            assert_eq!(&bytes[off..off + 4], b"FORM", "offset must hit a FORM tag");
            let form_len = u32::from_be_bytes(bytes[off + 4..off + 8].try_into().unwrap()) as u64;
            assert_eq!(
                c.size as u64,
                form_len + 8,
                "size table must match component {}'s FORM span",
                c.id
            );
        }
    }

    #[test]
    fn split_single_page_from_multipage() {
        let path = fixture_path("DjVu3Spec_bundled.djvu");
        if !path.exists() {
            // Skip if fixture not available
            return;
        }
        let data = std::fs::read(&path).expect("read fixture");
        let doc = DjVuDocument::parse(&data).expect("parse");
        let count = doc.page_count();
        assert!(count > 1, "need multipage fixture");

        // Split out page 0
        let page0 = split(&data, 0, 1).expect("split page 0");
        // Verify the result is parseable
        let form = iff::parse_form(&page0).expect("parse split page");
        assert_eq!(&form.form_type, b"DJVU");
    }

    #[test]
    fn merge_two_single_page_files() {
        let path = fixture_path("irish.djvu");
        if !path.exists() {
            return;
        }
        let irish = std::fs::read(&path).expect("read fixture");
        let data = merge(&[&irish, &irish]).expect("merge");
        // Verify the result has the right FORM type
        let form = iff::parse_form(&data).expect("parse merged");
        assert_eq!(&form.form_type, b"DJVM");
    }

    #[test]
    fn split_out_of_bounds() {
        let path = fixture_path("irish.djvu");
        if !path.exists() {
            return;
        }
        let data = std::fs::read(&path).expect("read fixture");
        let result = split(&data, 0, 5);
        assert!(result.is_err());
    }

    #[test]
    fn create_indirect_empty_returns_error() {
        let result = create_indirect(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn create_indirect_parses_with_resolver() {
        // Build an indirect DJVM that references "chicken.djvu"
        let indirect_bytes = create_indirect(&["chicken.djvu"]).expect("create_indirect");

        // Verify it parses as FORM:DJVM
        let form = iff::parse_form(&indirect_bytes).expect("parse form");
        assert_eq!(&form.form_type, b"DJVM");

        // Verify DIRM chunk has is_bundled = 0
        let dirm = form.chunks.iter().find(|c| &c.id == b"DIRM").expect("DIRM");
        let payload = crate::dirm::DirmPayload::decode(dirm.data).expect("decode DIRM");
        assert!(
            !payload.is_bundled(),
            "indirect DIRM must not have bundled bit set"
        );

        // Parse with a resolver that supplies chicken.djvu
        let chicken_path = fixture_path("chicken.djvu");
        if !chicken_path.exists() {
            return;
        }
        let chicken_data = std::fs::read(&chicken_path).expect("read chicken.djvu");
        let doc = DjVuDocument::parse_with_resolver(
            &indirect_bytes,
            Some(
                move |name: &str| -> Result<Vec<u8>, crate::djvu_document::DocError> {
                    if name == "chicken.djvu" {
                        Ok(chicken_data.clone())
                    } else {
                        Err(crate::djvu_document::DocError::IndirectResolve(
                            name.to_string(),
                        ))
                    }
                },
            ),
        )
        .expect("parse indirect with resolver");

        assert_eq!(doc.page_count(), 1);
        let page = doc.page(0).unwrap();
        assert_eq!(page.width(), 181);
        assert_eq!(page.height(), 240);
    }

    #[test]
    fn create_indirect_multipage() {
        // 3-page indirect document
        let indirect_bytes =
            create_indirect(&["page1.djvu", "page2.djvu", "page3.djvu"]).expect("create_indirect");
        let form = iff::parse_form(&indirect_bytes).expect("parse");
        assert_eq!(&form.form_type, b"DJVM");

        // Component count = 3 in DIRM
        let dirm = form.chunks.iter().find(|c| &c.id == b"DIRM").expect("DIRM");
        let payload = crate::dirm::DirmPayload::decode(dirm.data).expect("decode DIRM");
        assert_eq!(payload.nfiles, 3);
    }

    #[test]
    fn merge_with_djvm_input_extracts_all_pages() {
        let path = fixture_path("DjVu3Spec_bundled.djvu");
        if !path.exists() {
            return;
        }
        let data = std::fs::read(&path).expect("read");
        let doc = DjVuDocument::parse(&data).expect("parse");
        let expected_pages = doc.page_count();

        // merge(&[djvm]) should expand the DJVM into its component pages
        let merged = merge(&[&data]).expect("merge DJVM");
        let form = iff::parse_form(&merged).expect("parse merged DJVM");
        assert_eq!(&form.form_type, b"DJVM");
        let page_count = form
            .chunks
            .iter()
            .filter(|c| &c.id == b"FORM" && c.data.len() >= 4 && &c.data[..4] == b"DJVU")
            .count();
        assert_eq!(page_count, expected_pages);
    }

    #[test]
    fn split_single_page_djvu_returns_original_bytes() {
        let path = fixture_path("chicken.djvu");
        if !path.exists() {
            return;
        }
        let data = std::fs::read(&path).expect("read");
        let result = split(&data, 0, 1).expect("split single-page");
        assert_eq!(
            result, data,
            "splitting a single-page doc must return original bytes"
        );
    }

    #[test]
    fn split_unknown_form_type_is_out_of_bounds() {
        // A valid AT&T FORM with an unknown form type has 0 pages → always OOB
        let fake = iff::partial_emit(*b"UNKN", &[]).unwrap();
        let result = split(&fake, 0, 1);
        assert!(
            result.is_err(),
            "unknown form type must yield PageRangeOutOfBounds"
        );
    }

    #[test]
    fn split_range_from_multipage_djvm_builds_new_djvm() {
        let path = fixture_path("DjVu3Spec_bundled.djvu");
        if !path.exists() {
            return;
        }
        let data = std::fs::read(&path).expect("read");
        let doc = DjVuDocument::parse(&data).expect("parse");
        let count = doc.page_count();
        if count < 3 {
            return;
        }
        // Extract pages 1..3 — a multi-page range → hits build_djvm path
        let extracted = split(&data, 1, 3).expect("split range");
        let form = iff::parse_form(&extracted).expect("parse extracted");
        assert_eq!(&form.form_type, b"DJVM");
        let page_count = form
            .chunks
            .iter()
            .filter(|c| &c.id == b"FORM" && c.data.len() >= 4 && &c.data[..4] == b"DJVU")
            .count();
        assert_eq!(page_count, 2);
    }

    #[test]
    fn split_bundled_djvm_keeps_transitive_dependencies_and_dirm_ids() {
        let extracted = split(&split_dependency_fixture(), 0, 2).expect("split range");
        let graph = ComponentGraph::parse(&extracted).expect("parse extracted graph");
        let ids = graph
            .nodes()
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec!["page0.djvu", "dictA.djvi", "page1.djvu", "dictB.djvi"]
        );
        assert!(
            graph.node("dictA.djvi").is_some(),
            "original id is retained"
        );
        assert!(
            graph.node("dictC.djvi").is_none(),
            "unreferenced shared component is omitted"
        );
        assert!(
            graph
                .validate()
                .iter()
                .all(|error| !matches!(error, crate::GraphError::MissingTarget { .. })),
            "the retained page INCLs resolve within the extracted bundle"
        );
    }

    #[test]
    fn split_single_page_with_dependencies_bundles_its_closure() {
        // page0 INCLs dictA, so extracting it alone must produce a self-contained
        // bundle (page0 + dictA) rather than a bare page with a dangling INCL.
        let extracted = split(&split_dependency_fixture(), 0, 1).expect("split page");
        let form = iff::parse_form(&extracted).expect("parse extracted bundle");
        assert_eq!(&form.form_type, b"DJVM");

        let graph = ComponentGraph::parse(&extracted).expect("parse extracted graph");
        let ids = graph
            .nodes()
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["page0.djvu", "dictA.djvi"]);
        assert!(
            graph
                .validate()
                .iter()
                .all(|error| !matches!(error, crate::GraphError::MissingTarget { .. })),
            "the retained page INCL resolves within the extracted bundle"
        );
    }

    #[test]
    fn split_single_page_without_dependencies_returns_standalone_form_djvu() {
        // A page that references no shared component keeps the standalone fast
        // path. Extracting index 1 also covers the `page_idx += 1` skip.
        let doc = split_bundled_fixture(vec![
            split_component("page0.djvu", 1, *b"DJVU", vec![]),
            split_component("page1.djvu", 1, *b"DJVU", vec![]),
        ]);
        let extracted = split(&doc, 1, 2).expect("split page");
        let form = iff::parse_form(&extracted).expect("parse extracted page");
        assert_eq!(&form.form_type, b"DJVU");
    }

    #[test]
    fn split_djvm_without_a_component_graph_uses_legacy_fallback() {
        let page0 = split_component_body(&split_component("page0.djvu", 1, *b"DJVU", vec![]));
        let page1 = split_component_body(&split_component("page1.djvu", 1, *b"DJVU", vec![]));
        let doc = iff::partial_emit(
            *b"DJVM",
            &[iff::EmitPart::Form(&page0), iff::EmitPart::Form(&page1)],
        )
        .expect("small DIRM-less fixture");

        let extracted = split(&doc, 0, 2).expect("split through fallback");
        let form = iff::parse_form(&extracted).expect("parse fallback output");
        assert_eq!(&form.form_type, b"DJVM");
        assert_eq!(
            form.chunks
                .iter()
                .filter(|chunk| chunk.id == *b"FORM" && chunk.data.starts_with(b"DJVU"))
                .count(),
            2
        );
    }

    #[test]
    fn merge_unknown_form_type_returns_empty_merge_error() {
        // All docs are unknown type → components stays empty → EmptyMerge
        let fake = iff::partial_emit(*b"UNKN", &[]).unwrap();
        let result = merge(&[&fake]);
        assert!(matches!(result, Err(DjvmError::EmptyMerge)));
    }

    #[test]
    fn split_second_page_from_djvm_skips_first() {
        // Extracting page at index 1 forces page_idx to increment past index 0,
        // covering the page_idx += 1 path in the single-page DJVM loop.
        let path = fixture_path("DjVu3Spec_bundled.djvu");
        if !path.exists() {
            return;
        }
        let data = std::fs::read(&path).expect("read");
        let doc = DjVuDocument::parse(&data).expect("parse");
        if doc.page_count() < 2 {
            return;
        }
        let result = split(&data, 1, 2).expect("split page 1");
        let form = iff::parse_form(&result).expect("parse split page");
        // Page index 1 (p0002) INCLs the shared dict0020.iff, so its standalone
        // extraction is now a self-contained bundle rather than a bare page with
        // a dangling INCL. Its INCL must resolve within the extracted bundle.
        assert_eq!(&form.form_type, b"DJVM");
        let graph = ComponentGraph::parse(&result).expect("parse extracted graph");
        assert!(
            graph
                .validate()
                .iter()
                .all(|error| !matches!(error, crate::GraphError::MissingTarget { .. })),
            "the extracted page's INCL resolves within its bundle"
        );
    }

    #[test]
    fn parse_from_dir_indirect() {
        // Write an indirect DJVM index and chicken.djvu to a temp directory,
        // then open it via parse_from_dir.
        let chicken_path = fixture_path("chicken.djvu");
        if !chicken_path.exists() {
            return;
        }
        let tmp = std::env::temp_dir().join("djvu_indirect_test");
        std::fs::create_dir_all(&tmp).unwrap();

        // Copy chicken.djvu as the component
        let component_name = "p0001.djvu";
        std::fs::copy(&chicken_path, tmp.join(component_name)).unwrap();

        // Build indirect index
        let index_bytes = create_indirect(&[component_name]).expect("create_indirect");
        let index_path = tmp.join("index.djvu");
        std::fs::write(&index_path, &index_bytes).unwrap();

        // Open via parse_from_dir
        let index_data = std::fs::read(&index_path).unwrap();
        let doc = DjVuDocument::parse_from_dir(&index_data, &tmp).expect("parse_from_dir");
        assert_eq!(doc.page_count(), 1);
        assert_eq!(doc.page(0).unwrap().width(), 181);
    }
}
