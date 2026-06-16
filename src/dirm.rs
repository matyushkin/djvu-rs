//! DjVu `DIRM` directory-chunk model — the single owner of the DIRM byte layout.
//!
//! The `DIRM` chunk inside a `FORM:DJVM` is one of the trickiest, most
//! bug-prone parts of the container format: a version/flags byte (bit 7 marks a
//! *bundled* document), a big-endian `u16` component count, an optional table
//! of 4-byte big-endian `FORM`-header offsets, and a trailing BZZ-compressed
//! metadata blob (per-component sizes, flags, ids, names, titles).
//!
//! Historically that layout was decoded in [`crate::djvu_document`] and
//! [`crate::djvu_async`], rewritten in place in [`crate::djvu_mut`], and encoded
//! from scratch in [`crate::djvm`] — four copies enforced only by comments.
//! [`DirmPayload`] is now the one place those bytes are parsed and formatted;
//! every consumer routes through it.
//!
//! [`DirmPayload::decode`] keeps the BZZ metadata tail verbatim, so
//! `encode(decode(x)) == x` for any well-formed chunk — the property the
//! byte-preserving mutator relies on.

#[cfg(not(feature = "std"))]
use alloc::{format, string::String, vec::Vec};

/// Component classification from a DIRM per-component flags byte (low 6 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirmComponentKind {
    /// A renderable page (`FORM:DJVU`).
    Page,
    /// A shared dictionary (`FORM:DJVI`).
    Shared,
    /// A thumbnail (`FORM:THUM`).
    Thumbnail,
}

/// One DIRM component descriptor: its kind and resolver id.
#[derive(Debug, Clone)]
pub(crate) struct DirmComponent {
    /// Component classification.
    pub kind: DirmComponentKind,
    /// Directory entry id (resolver key / first metadata string field).
    pub id: String,
}

/// Owned, round-trippable model of a `DIRM` chunk payload.
#[derive(Debug, Clone)]
pub(crate) struct DirmPayload {
    /// DIRM version/flags byte; bit 7 set ⇒ bundled (carries an offset table).
    pub flags: u8,
    /// Declared component count (`nfiles`).
    pub nfiles: u16,
    /// Absolute `FORM`-header offsets, one per component. Empty when indirect.
    pub offsets: Vec<u32>,
    /// Trailing BZZ-compressed metadata, stored verbatim so that
    /// `encode(decode(x)) == x`.
    pub metadata: Vec<u8>,
}

impl DirmPayload {
    /// Whether this is a bundled document (offset table present).
    pub fn is_bundled(&self) -> bool {
        self.flags & 0x80 != 0
    }

    /// Decode the `DIRM` chunk payload (the bytes after the `DIRM` id+length).
    ///
    /// The BZZ metadata tail is captured verbatim; see [`DirmPayload::components`]
    /// to decode it into typed descriptors.
    pub fn decode(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 3 {
            return Err("DIRM chunk too short");
        }
        let flags = data[0];
        let nfiles = u16::from_be_bytes([data[1], data[2]]);
        let bundled = flags & 0x80 != 0;

        let mut pos = 3usize;
        let mut offsets = Vec::new();
        if bundled {
            let n = nfiles as usize;
            let end = pos
                .checked_add(n * 4)
                .ok_or("DIRM offset table size overflow")?;
            if end > data.len() {
                return Err("DIRM offset table truncated");
            }
            offsets.reserve(n);
            for i in 0..n {
                let b = pos + i * 4;
                offsets.push(u32::from_be_bytes([
                    data[b],
                    data[b + 1],
                    data[b + 2],
                    data[b + 3],
                ]));
            }
            pos = end;
        }

        Ok(Self {
            flags,
            nfiles,
            offsets,
            metadata: data[pos..].to_vec(),
        })
    }

    /// Serialize back to `DIRM` chunk payload bytes.
    ///
    /// Inverse of [`DirmPayload::decode`]; byte-identical for any value produced
    /// by `decode`. Only the document writers (mutate / merge) format DIRM
    /// bytes, and those paths require `std`, so this is gated to keep the
    /// `no_std` build free of dead code.
    #[cfg(feature = "std")]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(3 + self.offsets.len() * 4 + self.metadata.len());
        out.push(self.flags);
        out.extend_from_slice(&self.nfiles.to_be_bytes());
        if self.is_bundled() {
            for &off in &self.offsets {
                out.extend_from_slice(&off.to_be_bytes());
            }
        }
        out.extend_from_slice(&self.metadata);
        out
    }

    /// Decode the per-component kind + id list from the BZZ metadata.
    ///
    /// Lenient by design, matching the historical readers: a BZZ-decode failure
    /// or short/truncated metadata yields `nfiles` synthetic `Page` entries
    /// (`p0000`, `p0001`, …) so callers can still reassign types from the actual
    /// `FORM` sub-type they walk.
    pub fn components(&self) -> Vec<DirmComponent> {
        let n = self.nfiles as usize;
        let meta = crate::bzz_new::bzz_decode(&self.metadata).unwrap_or_default();

        // Metadata layout: sizes(3b × N), flags(1b × N), then per-component
        // null-terminated id [+ name if flag&0x80] [+ title if flag&0x40].
        let flags_start = n * 3;
        if flags_start + n > meta.len() {
            return (0..n)
                .map(|i| DirmComponent {
                    kind: DirmComponentKind::Page,
                    id: format!("p{i:04}"),
                })
                .collect();
        }

        let mut out = Vec::with_capacity(n);
        let mut pos = flags_start + n;
        for &flag in &meta[flags_start..flags_start + n] {
            let kind = match flag & 0x3f {
                1 => DirmComponentKind::Page,
                2 => DirmComponentKind::Thumbnail,
                _ => DirmComponentKind::Shared,
            };
            let id = read_nt_string(&meta, &mut pos).unwrap_or_default();
            if flag & 0x80 != 0 {
                let _ = read_nt_string(&meta, &mut pos);
            }
            if flag & 0x40 != 0 {
                let _ = read_nt_string(&meta, &mut pos);
            }
            out.push(DirmComponent { kind, id });
        }
        out
    }

    /// Build a bundled `DIRM` (offset table present, zero-initialized) from
    /// component descriptors. Offsets are filled in by the caller once the final
    /// byte layout is known; readers fall back to `FORM` boundaries meanwhile.
    #[cfg(feature = "std")]
    pub fn build_bundled(count: usize, flags: &[u8], ids: &[String]) -> Self {
        Self {
            flags: 0x80,
            nfiles: count as u16,
            offsets: vec![0; count],
            metadata: build_metadata(count, flags, ids),
        }
    }

    /// Build an indirect `DIRM` (no offset table) from component descriptors.
    #[cfg(feature = "std")]
    pub fn build_indirect(count: usize, flags: &[u8], ids: &[String]) -> Self {
        Self {
            flags: 0x00,
            nfiles: count as u16,
            offsets: Vec::new(),
            metadata: build_metadata(count, flags, ids),
        }
    }
}

/// Build the BZZ-compressed DIRM metadata tail from component descriptors.
///
/// Layout: sizes(3b × N, zeroed — readers use `FORM` boundaries), flags(1b × N),
/// ids(null-terminated), names(null-terminated, mirrors ids), and titles
/// (N empty, null-terminated strings).
#[cfg(feature = "std")]
fn build_metadata(count: usize, flags: &[u8], ids: &[String]) -> Vec<u8> {
    let mut meta = Vec::new();
    for _ in 0..count {
        meta.extend_from_slice(&[0, 0, 0]);
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
    meta.extend(core::iter::repeat_n(0u8, count));
    crate::bzz_encode::bzz_encode(&meta)
}

/// Read a null-terminated UTF-8 string at `*pos`, advancing past the terminator.
///
/// Returns `None` (leaving `*pos` unchanged) if there is no terminator or the
/// bytes are not valid UTF-8.
fn read_nt_string(data: &[u8], pos: &mut usize) -> Option<String> {
    let start = *pos;
    let rest = data.get(start..)?;
    let nul = rest.iter().position(|&b| b == 0)?;
    *pos = start + nul + 1;
    core::str::from_utf8(&rest[..nul]).ok().map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_encode_roundtrip_bundled() {
        // flags=0x81 (bundled, v1), nfiles=2, two 4-byte offsets, arbitrary tail.
        let chunk = [
            0x81, 0x00, 0x02, // flags + nfiles
            0x00, 0x00, 0x00, 0x10, // offset 0
            0x00, 0x00, 0x12, 0x34, // offset 1
            0xde, 0xad, 0xbe, 0xef, // metadata tail (opaque here)
        ];
        let p = DirmPayload::decode(&chunk).expect("decode");
        assert!(p.is_bundled());
        assert_eq!(p.nfiles, 2);
        assert_eq!(p.offsets, vec![0x10, 0x1234]);
        assert_eq!(p.metadata, vec![0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(p.encode(), chunk, "encode∘decode must be identity");
    }

    #[test]
    fn decode_encode_roundtrip_indirect() {
        // flags=0x01 (indirect, v1), nfiles=3, no offset table, metadata tail.
        let chunk = [0x01, 0x00, 0x03, 0x01, 0x02, 0x03];
        let p = DirmPayload::decode(&chunk).expect("decode");
        assert!(!p.is_bundled());
        assert_eq!(p.nfiles, 3);
        assert!(p.offsets.is_empty());
        assert_eq!(p.metadata, vec![0x01, 0x02, 0x03]);
        assert_eq!(p.encode(), chunk, "encode∘decode must be identity");
    }

    #[test]
    fn decode_rejects_truncated_offset_table() {
        // Bundled, claims 4 files but only one offset's worth of bytes follow.
        let chunk = [0x80, 0x00, 0x04, 0x00, 0x00, 0x00, 0x10];
        assert!(DirmPayload::decode(&chunk).is_err());
    }

    #[test]
    fn decode_rejects_too_short() {
        assert!(DirmPayload::decode(&[0x80, 0x00]).is_err());
    }

    #[test]
    fn build_bundled_components_roundtrip() {
        let ids = vec!["page1".to_string(), "dict".to_string()];
        let flags = vec![1u8, 0u8]; // page, shared
        let p = DirmPayload::build_bundled(2, &flags, &ids);
        assert!(p.is_bundled());
        assert_eq!(p.nfiles, 2);
        assert_eq!(p.offsets, vec![0, 0]);

        let comps = p.components();
        assert_eq!(comps.len(), 2);
        assert_eq!(comps[0].kind, DirmComponentKind::Page);
        assert_eq!(comps[0].id, "page1");
        assert_eq!(comps[1].kind, DirmComponentKind::Shared);
        assert_eq!(comps[1].id, "dict");

        // Structural round-trip survives a decode/encode cycle.
        let bytes = p.encode();
        let p2 = DirmPayload::decode(&bytes).expect("decode built");
        assert_eq!(p2.encode(), bytes);
    }

    #[test]
    fn build_indirect_has_no_offset_table() {
        let ids = vec!["a.djvu".to_string(), "b.djvu".to_string()];
        let p = DirmPayload::build_indirect(2, &[1, 1], &ids);
        assert!(!p.is_bundled());
        assert!(p.offsets.is_empty());
        let bytes = p.encode();
        // Byte 0 is the indirect flags byte; no offset table follows nfiles.
        assert_eq!(bytes[0], 0x00);
        let comps = p.components();
        assert_eq!(
            comps.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
            ["a.djvu", "b.djvu"]
        );
    }

    #[test]
    fn components_short_metadata_yields_synthetic_pages() {
        // Bundled, nfiles=2, offsets present, but metadata too short to parse.
        let p = DirmPayload {
            flags: 0x80,
            nfiles: 2,
            offsets: vec![0, 0],
            metadata: Vec::new(),
        };
        let comps = p.components();
        assert_eq!(comps.len(), 2);
        assert!(comps.iter().all(|c| c.kind == DirmComponentKind::Page));
        assert_eq!(comps[0].id, "p0000");
        assert_eq!(comps[1].id, "p0001");
    }
}
