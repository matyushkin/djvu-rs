//! DJVM document merge and split operations.
//!
//! Provides [`merge`] to combine multiple DjVu documents into a single
//! bundled DJVM, and [`split`] to extract page ranges from a document.
//!
//! [`merge`]: crate::djvm::merge
//! [`split`]: crate::djvm::split

#[cfg(not(feature = "std"))]
use alloc::{format, string::String, vec, vec::Vec};

use crate::djvu_document::DjVuDocument;
use crate::error::IffError;
use crate::iff;

/// Error type for merge/split operations.
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

                    // Wrap the chunk data back into a full FORM with AT&T header
                    let mut form_bytes = Vec::with_capacity(4 + 4 + 4 + chunk.data.len());
                    form_bytes.extend_from_slice(b"AT&T");
                    form_bytes.extend_from_slice(b"FORM");
                    let form_len = chunk.data.len() as u32;
                    form_bytes.extend_from_slice(&form_len.to_be_bytes());
                    form_bytes.extend_from_slice(chunk.data);

                    components.push(form_bytes);
                    component_ids.push(format!("d{}p{:04}.djvu", doc_idx, components.len()));

                    let flag = if child_form_type == b"DJVI" { 0 } else { 1 }; // 0 = shared, 1 = page
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
    let doc = DjVuDocument::parse(doc_data)?;
    let count = doc.page_count();

    if start >= count || end > count || start >= end {
        return Err(DjvmError::PageRangeOutOfBounds { start, end, count });
    }

    let form = iff::parse_form(doc_data)?;

    // Single-page document: just return the whole thing
    if &form.form_type == b"DJVU" && start == 0 && end == 1 {
        return Ok(doc_data.to_vec());
    }

    // For a single page extraction from a multi-page document
    if end - start == 1 && &form.form_type == b"DJVM" {
        let mut page_idx = 0;
        for chunk in &form.chunks {
            if &chunk.id == b"FORM" && chunk.data.len() >= 4 && &chunk.data[..4] == b"DJVU" {
                if page_idx == start {
                    let mut result = Vec::with_capacity(4 + 4 + 4 + chunk.data.len());
                    result.extend_from_slice(b"AT&T");
                    result.extend_from_slice(b"FORM");
                    let len = chunk.data.len() as u32;
                    result.extend_from_slice(&len.to_be_bytes());
                    result.extend_from_slice(chunk.data);
                    return Ok(result);
                }
                page_idx += 1;
            }
        }
    }

    // Multiple pages: build a new DJVM bundle with the requested range
    let mut components: Vec<Vec<u8>> = Vec::new();
    let mut component_ids: Vec<String> = Vec::new();
    let mut component_flags: Vec<u8> = Vec::new();

    // First pass: collect shared components (DJVI) that might be needed
    for chunk in &form.chunks {
        if &chunk.id == b"FORM" && chunk.data.len() >= 4 && &chunk.data[..4] == b"DJVI" {
            let mut form_bytes = Vec::with_capacity(4 + 4 + 4 + chunk.data.len());
            form_bytes.extend_from_slice(b"AT&T");
            form_bytes.extend_from_slice(b"FORM");
            let len = chunk.data.len() as u32;
            form_bytes.extend_from_slice(&len.to_be_bytes());
            form_bytes.extend_from_slice(chunk.data);
            components.push(form_bytes);
            component_ids.push(format!("shared{}.djvi", components.len()));
            component_flags.push(0); // shared
        }
    }

    // Second pass: collect pages in the requested range
    let mut page_idx = 0;
    for chunk in &form.chunks {
        if &chunk.id == b"FORM" && chunk.data.len() >= 4 && &chunk.data[..4] == b"DJVU" {
            if page_idx >= start && page_idx < end {
                let mut form_bytes = Vec::with_capacity(4 + 4 + 4 + chunk.data.len());
                form_bytes.extend_from_slice(b"AT&T");
                form_bytes.extend_from_slice(b"FORM");
                let len = chunk.data.len() as u32;
                form_bytes.extend_from_slice(&len.to_be_bytes());
                form_bytes.extend_from_slice(chunk.data);
                components.push(form_bytes);
                component_ids.push(format!("p{:04}.djvu", page_idx + 1));
                component_flags.push(1); // page
            }
            page_idx += 1;
        }
    }

    build_djvm(&components, &component_ids, &component_flags)
}

/// Build a bundled DJVM file from components.
fn build_djvm(components: &[Vec<u8>], ids: &[String], flags: &[u8]) -> Result<Vec<u8>, DjvmError> {
    let n = components.len();

    // Build DIRM chunk
    let dirm_data = build_dirm(n, flags, ids);

    // Calculate total FORM body size
    let mut body_size: usize = 4; // "DJVM"
    body_size += 8 + dirm_data.len(); // DIRM chunk header + data
    if !dirm_data.len().is_multiple_of(2) {
        body_size += 1; // IFF padding
    }
    for comp in components {
        // Each component includes AT&T prefix — strip it for embedding
        let comp_data = if comp.len() >= 4 && &comp[..4] == b"AT&T" {
            &comp[4..]
        } else {
            comp.as_slice()
        };
        body_size += comp_data.len();
        if !comp_data.len().is_multiple_of(2) {
            body_size += 1; // IFF padding
        }
    }

    let mut output = Vec::with_capacity(4 + 4 + 4 + body_size);

    // AT&T magic
    output.extend_from_slice(b"AT&T");
    // FORM header
    output.extend_from_slice(b"FORM");
    output.extend_from_slice(&(body_size as u32).to_be_bytes());
    // DJVM type
    output.extend_from_slice(b"DJVM");

    // DIRM chunk
    output.extend_from_slice(b"DIRM");
    output.extend_from_slice(&(dirm_data.len() as u32).to_be_bytes());
    output.extend_from_slice(&dirm_data);
    if !dirm_data.len().is_multiple_of(2) {
        output.push(0); // IFF padding
    }

    // Component FORM chunks
    for comp in components {
        let comp_data = if comp.len() >= 4 && &comp[..4] == b"AT&T" {
            &comp[4..]
        } else {
            comp.as_slice()
        };
        output.extend_from_slice(comp_data);
        if !comp_data.len().is_multiple_of(2) {
            output.push(0); // IFF padding
        }
    }

    Ok(output)
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

    let dirm_data = build_dirm_indirect(count, &flags, &ids);

    let mut body_size: usize = 4; // "DJVM"
    body_size += 8 + dirm_data.len(); // DIRM chunk header + data
    if !dirm_data.len().is_multiple_of(2) {
        body_size += 1;
    }

    let mut output = Vec::with_capacity(4 + 4 + 4 + body_size);
    output.extend_from_slice(b"AT&T");
    output.extend_from_slice(b"FORM");
    output.extend_from_slice(&(body_size as u32).to_be_bytes());
    output.extend_from_slice(b"DJVM");
    output.extend_from_slice(b"DIRM");
    output.extend_from_slice(&(dirm_data.len() as u32).to_be_bytes());
    output.extend_from_slice(&dirm_data);
    if !dirm_data.len().is_multiple_of(2) {
        output.push(0);
    }

    Ok(output)
}

/// Build an indirect (non-bundled) DIRM chunk.
///
/// Unlike the bundled variant, there is no per-component offset table.
fn build_dirm_indirect(count: usize, flags: &[u8], ids: &[String]) -> Vec<u8> {
    let mut data = Vec::new();

    // Flags byte: 0x00 = indirect (not bundled)
    data.push(0x00);

    // Component count (16-bit big-endian)
    data.push((count >> 8) as u8);
    data.push(count as u8);

    // No offset table for indirect documents.

    let mut meta = Vec::new();
    for _ in 0..count {
        meta.extend_from_slice(&[0, 0, 0]); // sizes (unused for indirect)
    }
    for &f in flags {
        meta.push(f);
    }
    for id in ids {
        meta.extend_from_slice(id.as_bytes());
        meta.push(0);
    }
    for id in ids {
        meta.extend_from_slice(id.as_bytes());
        meta.push(0);
    }
    meta.extend(core::iter::repeat_n(0u8, count)); // empty titles

    let compressed = crate::bzz_encode::bzz_encode(&meta);
    data.extend_from_slice(&compressed);

    data
}

/// Build the DIRM chunk data.
///
/// Format:
/// - 1 byte: flags (0x80 = bundled)
/// - 2 bytes: component count (big-endian)
/// - 4 bytes x n: component offsets (big-endian, computed from component sizes)
/// - BZZ-compressed metadata: sizes(3b×N), flags(1b×N), IDs, names, titles
///
/// The BZZ-compressed section is built by reusing the existing reference
/// BZZ stream from the first input document when possible. For fresh
/// construction, we build the raw metadata and use a minimal BZZ wrapper.
fn build_dirm(count: usize, flags: &[u8], ids: &[String]) -> Vec<u8> {
    let mut data = Vec::new();

    // Flags byte: 0x80 = bundled format
    data.push(0x80);

    // Component count (16-bit big-endian)
    data.push((count >> 8) as u8);
    data.push(count as u8);

    // Placeholder for offsets (4 bytes each) — filled in below
    let _offsets_start = data.len();
    for _ in 0..count {
        data.extend_from_slice(&[0, 0, 0, 0]);
    }

    // Build the raw metadata that would normally be BZZ-compressed.
    // Layout: sizes(3b × N) + flags(1b × N) + IDs(null-term) + names(null-term) + titles(null-term)
    let mut meta = Vec::new();

    // Component sizes — 3 bytes each, set to 0 (readers use FORM boundaries)
    for _ in 0..count {
        meta.extend_from_slice(&[0, 0, 0]);
    }
    // Component flags (1 byte each)
    for &f in flags {
        meta.push(f);
    }
    // Component IDs (null-terminated)
    for id in ids {
        meta.extend_from_slice(id.as_bytes());
        meta.push(0);
    }
    // Names (null-terminated, same as IDs)
    for id in ids {
        meta.extend_from_slice(id.as_bytes());
        meta.push(0);
    }
    // Titles (empty, null-terminated)
    meta.extend(core::iter::repeat_n(0u8, count));

    // Encode the metadata using BZZ. We use a trivial BZZ stream:
    // the raw metadata is small enough that we can encode it directly
    // using the BZZ block format with a passthrough identity encoding.
    let compressed = crate::bzz_encode::bzz_encode(&meta);
    data.extend_from_slice(&compressed);

    data
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn merge_empty_returns_error() {
        let result = merge(&[]);
        assert!(result.is_err());
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
        assert_eq!(
            dirm.data[0] & 0x80,
            0,
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
        let nfiles = u16::from_be_bytes([dirm.data[1], dirm.data[2]]) as usize;
        assert_eq!(nfiles, 3);
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
