use crate::bitmap::Bitmap;
use crate::error::Error;
use crate::iff::{Chunk, DjvuFile};
use crate::iw44::IW44Image;
use crate::jb2::JB2Dict;
use crate::pixmap::Pixmap;

#[cfg(test)]
pub use crate::iw44::NormalizedPlanes;

/// A bookmark entry from the NAVM chunk (table of contents).
#[derive(Debug, Clone)]
pub struct Bookmark {
    pub title: String,
    pub url: String,
    pub children: Vec<Bookmark>,
}

/// Rotation values from INFO chunk flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    None,
    Cw90,
    Cw180,
    Cw270,
}

/// Page metadata from the INFO chunk.
#[derive(Debug, Clone)]
pub struct PageInfo {
    pub width: u16,
    pub height: u16,
    pub dpi: u16,
    /// Display gamma (e.g. 2.2). Defaults to 2.2 when the INFO byte is 0.
    pub gamma: f32,
    pub rotation: Rotation,
}

/// FGbz palette: per-blit color indices into an RGB palette.
#[derive(Debug, Clone)]
pub struct Palette {
    pub colors: Vec<(u8, u8, u8)>,
    pub indices: Vec<i16>,
}

/// Text zone type in the DjVu text layer hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextZoneKind {
    Page = 1,
    Column = 2,
    Region = 3,
    Paragraph = 4,
    Line = 5,
    Word = 6,
    Character = 7,
}

/// A text zone with bounding box and text span within the page text.
///
/// Coordinates are in the DjVu coordinate system (origin at bottom-left, y increases upward).
/// Use `text_start` and `text_len` to index into `TextLayer::text`.
#[derive(Debug, Clone)]
pub struct TextZone {
    pub kind: TextZoneKind,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub text_start: usize,
    pub text_len: usize,
    pub children: Vec<TextZone>,
}

/// The text layer of a DjVu page (from TXTz or TXTa chunks).
#[derive(Debug, Clone)]
pub struct TextLayer {
    /// The full UTF-8 text content of the page.
    pub text: String,
    /// The zone hierarchy (None if the text has no zone structure).
    pub root: Option<TextZone>,
}

impl TextLayer {
    /// Get the text content of a specific zone.
    pub fn zone_text(&self, zone: &TextZone) -> &str {
        let end = (zone.text_start + zone.text_len).min(self.text.len());
        let start = zone.text_start.min(end);
        // Ensure we don't split multi-byte UTF-8 characters
        if self.text.is_char_boundary(start) && self.text.is_char_boundary(end) {
            &self.text[start..end]
        } else {
            ""
        }
    }
}

/// Component type in DIRM directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComponentType {
    Shared,    // 0 — DJVI
    Page,      // 1 — DJVU
    Thumbnail, // 2 — THUM
}

/// A component entry from the DIRM directory.
#[derive(Debug, Clone)]
struct DirmEntry {
    comp_type: ComponentType,
    id: String,
}

/// A parsed DjVu document (single-page or multi-page bundled).
pub struct Document<'a> {
    file: DjvuFile<'a>,
    /// For DJVM: DIRM entries and FORM children (indexed by order in DIRM).
    dirm_entries: Vec<DirmEntry>,
    /// Indices into dirm_entries for page-type components only.
    page_indices: Vec<usize>,
    /// For single-page DJVU: true.
    is_single_page: bool,
}

impl<'a> Document<'a> {
    /// Parse a DjVu document from raw bytes.
    pub fn parse(data: &'a [u8]) -> Result<Self, Error> {
        let file = crate::iff::parse(data)?;
        match &file.root {
            Chunk::Form {
                secondary_id: [b'D', b'J', b'V', b'U'],
                ..
            } => {
                Ok(Document {
                    file,
                    dirm_entries: vec![],
                    page_indices: vec![0], // single page at index 0
                    is_single_page: true,
                })
            }
            Chunk::Form {
                secondary_id: [b'D', b'J', b'V', b'M'],
                children,
                ..
            } => {
                // Find and parse DIRM chunk
                let dirm_chunk = children
                    .iter()
                    .find_map(|c| match c {
                        Chunk::Leaf {
                            id: [b'D', b'I', b'R', b'M'],
                            data,
                        } => Some(*data),
                        _ => None,
                    })
                    .ok_or(Error::MissingChunk("DIRM"))?;

                let (dirm_entries, is_bundled) = parse_dirm(dirm_chunk)?;
                if !is_bundled {
                    return Err(Error::Unsupported("indirect DJVM not supported"));
                }

                let page_indices: Vec<usize> = dirm_entries
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.comp_type == ComponentType::Page)
                    .map(|(i, _)| i)
                    .collect();

                Ok(Document {
                    file,
                    dirm_entries,
                    page_indices,
                    is_single_page: false,
                })
            }
            _ => Err(Error::Unsupported("not a DJVU or DJVM document")),
        }
    }

    /// Number of pages (excluding thumbnails and shared components).
    pub fn page_count(&self) -> usize {
        self.page_indices.len()
    }

    /// Access a page by 0-based index.
    pub fn page(&self, index: usize) -> Result<Page<'_>, Error> {
        if index >= self.page_count() {
            return Err(Error::FormatError(format!(
                "page index {} out of range ({})",
                index,
                self.page_count()
            )));
        }

        if self.is_single_page {
            return Page::from_form(&self.file.root, self);
        }

        // Multi-page: find the FORM child corresponding to this page
        let dirm_index = self.page_indices[index];
        let form = self.get_component_form(dirm_index)?;
        Page::from_form(form, self)
    }

    /// Get the FORM chunk for a DIRM component by its dirm index.
    /// In bundled documents, FORM children after DIRM/NAVM correspond to DIRM entries in order.
    fn get_component_form(&self, dirm_index: usize) -> Result<&Chunk<'a>, Error> {
        let forms: Vec<&Chunk<'a>> = self
            .file
            .root
            .children()
            .iter()
            .filter(|c| matches!(c, Chunk::Form { .. }))
            .collect();

        forms
            .get(dirm_index)
            .copied()
            .ok_or(Error::FormatError(format!(
                "component {} not found",
                dirm_index
            )))
    }

    /// Parse the NAVM bookmarks (table of contents).
    ///
    /// Returns an empty Vec if there is no NAVM chunk.
    pub fn bookmarks(&self) -> Result<Vec<Bookmark>, Error> {
        let navm_data = match self.file.root.find_first(b"NAVM") {
            Some(c) => c.data(),
            None => return Ok(vec![]),
        };

        let decoded = crate::bzz_new::bzz_decode(navm_data)
            .map_err(|e| Error::FormatError(format!("NAVM BZZ decode: {}", e)))?;

        if decoded.len() < 2 {
            return Ok(vec![]);
        }

        let total_count = u16::from_be_bytes([decoded[0], decoded[1]]) as usize;
        let mut pos = 2usize;
        let mut bookmarks = Vec::new();
        let mut decoded_count = 0usize;

        while decoded_count < total_count {
            let bm = parse_bookmark(&decoded, &mut pos, &mut decoded_count)?;
            bookmarks.push(bm);
        }

        Ok(bookmarks)
    }

    /// Decode a thumbnail for the given page (0-based index).
    ///
    /// Thumbnails are stored in FORM:THUM components with TH44 (IW44) chunks.
    /// Returns `Ok(None)` if no thumbnail exists for this page.
    pub fn thumbnail(&self, page_index: usize) -> Result<Option<Pixmap>, Error> {
        if self.is_single_page {
            return Ok(None);
        }

        let mut thumb_idx: usize = 0;
        for (i, entry) in self.dirm_entries.iter().enumerate() {
            if entry.comp_type != ComponentType::Thumbnail {
                continue;
            }
            let form = self.get_component_form(i)?;
            let th44_chunks: Vec<&[u8]> = form
                .find_all(b"TH44")
                .into_iter()
                .map(|c| c.data())
                .collect();

            let mut img = IW44Image::new();
            for chunk_data in &th44_chunks {
                if chunk_data.is_empty() {
                    continue;
                }
                let serial = chunk_data[0];
                if serial == 0 && img.width() > 0 {
                    // Previous thumbnail is complete
                    if thumb_idx == page_index {
                        let pm = img
                            .to_pixmap()
                            .map_err(|e| Error::FormatError(e.to_string()))?;
                        return Ok(Some(pm));
                    }
                    thumb_idx += 1;
                    img = IW44Image::new();
                }
                img.decode_chunk(chunk_data)
                    .map_err(|e| Error::FormatError(e.to_string()))?;
            }
            // Handle last thumbnail in this THUM
            if img.width() > 0 {
                if thumb_idx == page_index {
                    let pm = img
                        .to_pixmap()
                        .map_err(|e| Error::FormatError(e.to_string()))?;
                    return Ok(Some(pm));
                }
                thumb_idx += 1;
            }
        }

        Ok(None)
    }

    /// Resolve an INCL reference to a shared DJVI component's children.
    fn resolve_incl(&self, ref_id: &str) -> Result<&Chunk<'a>, Error> {
        if self.is_single_page {
            return Err(Error::FormatError("INCL in single-page document".into()));
        }

        for (i, entry) in self.dirm_entries.iter().enumerate() {
            if entry.id == ref_id {
                return self.get_component_form(i);
            }
        }

        Err(Error::FormatError(format!(
            "INCL target '{}' not found",
            ref_id
        )))
    }
}

/// A single page within a DjVu document.
pub struct Page<'a> {
    pub info: PageInfo,
    form: &'a Chunk<'a>,
    doc: &'a Document<'a>,
}

impl<'a> Page<'a> {
    fn from_form(form: &'a Chunk<'a>, doc: &'a Document<'a>) -> Result<Self, Error> {
        let info_chunk = form
            .find_first(b"INFO")
            .ok_or(Error::MissingChunk("INFO"))?;
        let info = parse_info(info_chunk.data())?;
        Ok(Page { info, form, doc })
    }

    #[cfg(test)]
    pub fn has_mask(&self) -> bool {
        self.form.find_first(b"Sjbz").is_some()
    }

    #[cfg(test)]
    pub fn has_background(&self) -> bool {
        self.form.find_first(b"BG44").is_some()
    }

    #[cfg(test)]
    pub fn has_foreground(&self) -> bool {
        self.form.find_first(b"FG44").is_some()
    }

    pub fn has_palette(&self) -> bool {
        self.form.find_first(b"FGbz").is_some()
    }

    /// Decode the JB2 mask layer, resolving shared dictionaries via INCL.
    pub fn decode_mask(&self) -> Result<Option<Bitmap>, Error> {
        let sjbz = match self.form.find_first(b"Sjbz") {
            Some(c) => c.data(),
            None => return Ok(None),
        };

        let shared_dict = self.resolve_shared_dict()?;

        let bitmap = crate::jb2::decode(sjbz, shared_dict.as_ref())
            .map_err(|e| Error::FormatError(e.to_string()))?;
        Ok(Some(bitmap))
    }

    /// Decode the JB2 mask with per-pixel blit index map (for FGbz palette compositing).
    pub fn decode_mask_indexed(&self) -> Result<Option<(Bitmap, Vec<i32>)>, Error> {
        let sjbz = match self.form.find_first(b"Sjbz") {
            Some(c) => c.data(),
            None => return Ok(None),
        };

        let shared_dict = self.resolve_shared_dict()?;

        let result = crate::jb2::decode_indexed(sjbz, shared_dict.as_ref())
            .map_err(|e| Error::FormatError(e.to_string()))?;
        Ok(Some(result))
    }

    /// Decode the IW44 background layer.
    pub fn decode_background(&self) -> Result<Option<Pixmap>, Error> {
        let img = match self.decode_iw44_layer(b"BG44")? {
            Some(img) => img,
            None => return Ok(None),
        };
        let pm = img
            .to_pixmap()
            .map_err(|e| Error::FormatError(e.to_string()))?;
        Ok(Some(pm))
    }

    /// Number of BG44 chunks in this page (0 = no background layer).
    pub fn bg44_chunk_count(&self) -> usize {
        self.form.find_all(b"BG44").len()
    }

    /// Decode the IW44 background progressively: return a pixmap after each
    /// BG44 chunk.  The first entry is a coarse (blurry) preview decoded from
    /// just the first chunk; each subsequent entry is a more refined image.
    /// The last entry is identical to `decode_background()`.
    ///
    /// Returns `Ok(None)` if there are no BG44 chunks.
    pub fn decode_background_progressive(&self) -> Result<Option<Vec<Pixmap>>, Error> {
        let chunks: Vec<&[u8]> = self
            .form
            .find_all(b"BG44")
            .into_iter()
            .map(|c| c.data())
            .collect();

        if chunks.is_empty() {
            return Ok(None);
        }

        let mut img = IW44Image::new();
        let mut frames = Vec::with_capacity(chunks.len());

        for chunk_data in &chunks {
            img.decode_chunk(chunk_data)
                .map_err(|e| Error::FormatError(e.to_string()))?;
            let pm = img
                .to_pixmap()
                .map_err(|e| Error::FormatError(e.to_string()))?;
            frames.push(pm);
        }

        Ok(Some(frames))
    }

    /// Decode only the first BG44 chunk — a coarse (blurry) preview.
    ///
    /// Much faster than `decode_background()` because it skips refinement
    /// chunks and only does one inverse wavelet transform. Returns `None`
    /// if there are no BG44 chunks, or if there is only one chunk (in
    /// which case `decode_background()` is already fast enough).
    pub fn decode_background_coarse(&self) -> Result<Option<Pixmap>, Error> {
        let chunks: Vec<&[u8]> = self
            .form
            .find_all(b"BG44")
            .into_iter()
            .map(|c| c.data())
            .collect();

        // Only worth it for multi-chunk backgrounds.
        if chunks.len() <= 1 {
            return Ok(None);
        }

        let mut img = IW44Image::new();
        img.decode_chunk(chunks[0])
            .map_err(|e| Error::FormatError(e.to_string()))?;
        let pm = img
            .to_pixmap()
            .map_err(|e| Error::FormatError(e.to_string()))?;
        Ok(Some(pm))
    }

    /// Decode the IW44 foreground layer.
    pub fn decode_foreground(&self) -> Result<Option<Pixmap>, Error> {
        let img = match self.decode_iw44_layer(b"FG44")? {
            Some(img) => img,
            None => return Ok(None),
        };
        let pm = img
            .to_pixmap()
            .map_err(|e| Error::FormatError(e.to_string()))?;
        Ok(Some(pm))
    }

    #[cfg(test)]
    pub fn decode_background_planes(&self) -> Result<Option<NormalizedPlanes>, Error> {
        let img = match self.decode_iw44_layer(b"BG44")? {
            Some(img) => img,
            None => return Ok(None),
        };
        let planes = img
            .to_normalized_planes_subsample(1)
            .map_err(|e| Error::FormatError(e.to_string()))?;
        Ok(Some(planes))
    }

    /// Parse the FGbz palette chunk.
    pub fn decode_palette(&self) -> Result<Option<Palette>, Error> {
        let fgbz = match self.form.find_first(b"FGbz") {
            Some(c) => c.data(),
            None => return Ok(None),
        };
        let palette = parse_fgbz(fgbz)?;
        Ok(Some(palette))
    }

    /// Decode the text layer (TXTz or TXTa chunk).
    ///
    /// Returns `Ok(None)` if the page has no text layer.
    pub fn text_layer(&self) -> Result<Option<TextLayer>, Error> {
        // Try TXTz (BZZ-compressed) first, then TXTa (uncompressed)
        let data = if let Some(txtz) = self.form.find_first(b"TXTz") {
            let compressed = txtz.data();
            if compressed.is_empty() {
                return Ok(None);
            }
            crate::bzz_new::bzz_decode(compressed)
                .map_err(|e| Error::FormatError(format!("TXTz BZZ decode: {}", e)))?
        } else if let Some(txta) = self.form.find_first(b"TXTa") {
            txta.data().to_vec()
        } else {
            return Ok(None);
        };

        parse_text_layer(&data)
    }

    fn resolve_shared_dict(&self) -> Result<Option<JB2Dict>, Error> {
        // Check all INCL chunks for an external DJVI component with Djbz
        for incl in self.form.find_all(b"INCL") {
            let ref_id = std::str::from_utf8(incl.data())
                .map_err(|_| Error::FormatError("invalid INCL UTF-8".into()))?
                .trim_end_matches('\0')
                .trim();

            let shared_form = self.doc.resolve_incl(ref_id)?;
            if let Some(djbz) = shared_form.find_first(b"Djbz") {
                let dict = crate::jb2::decode_dict(djbz.data(), None)
                    .map_err(|e| Error::FormatError(e.to_string()))?;
                return Ok(Some(dict));
            }
        }

        // Then check for inline Djbz in the same FORM as Sjbz
        if let Some(djbz) = self.form.find_first(b"Djbz") {
            let dict = crate::jb2::decode_dict(djbz.data(), None)
                .map_err(|e| Error::FormatError(e.to_string()))?;
            return Ok(Some(dict));
        }

        Ok(None)
    }

    fn decode_iw44_layer(&self, chunk_id: &[u8; 4]) -> Result<Option<IW44Image>, Error> {
        let chunks: Vec<&[u8]> = self
            .form
            .find_all(chunk_id)
            .into_iter()
            .map(|c| c.data())
            .collect();

        if chunks.is_empty() {
            return Ok(None);
        }

        let mut img = IW44Image::new();
        for chunk_data in &chunks {
            img.decode_chunk(chunk_data)
                .map_err(|e| Error::FormatError(e.to_string()))?;
        }
        Ok(Some(img))
    }
}

// ============================================================
// INFO chunk parser
// ============================================================

fn parse_info(data: &[u8]) -> Result<PageInfo, Error> {
    if data.len() < 5 {
        return Err(Error::InvalidLength);
    }

    let width = u16::from_be_bytes([data[0], data[1]]);
    let height = u16::from_be_bytes([data[2], data[3]]);
    let _minver = data[4];
    let _majver = if data.len() > 5 { data[5] } else { 0 };

    // DPI is little-endian (unusual for IFF)
    let raw_dpi = if data.len() >= 8 {
        u16::from_le_bytes([data[6], data[7]])
    } else {
        300
    };
    let dpi = if (25..=6000).contains(&raw_dpi) {
        raw_dpi
    } else {
        300
    };

    let gamma_byte = if data.len() >= 9 { data[8] } else { 0 };
    let gamma = if gamma_byte == 0 {
        2.2_f32
    } else {
        gamma_byte as f32 / 10.0
    };

    let flags = if data.len() >= 10 { data[9] } else { 0 };
    let rotation = match flags & 0x07 {
        5 => Rotation::Cw90,
        2 => Rotation::Cw180,
        6 => Rotation::Cw270,
        _ => Rotation::None,
    };

    Ok(PageInfo {
        width,
        height,
        dpi,
        gamma,
        rotation,
    })
}

// ============================================================
// DIRM chunk parser
// ============================================================

fn parse_dirm(data: &[u8]) -> Result<(Vec<DirmEntry>, bool), Error> {
    if data.len() < 3 {
        return Err(Error::InvalidLength);
    }

    let dflags = data[0];
    let is_bundled = (dflags >> 7) != 0;
    let nfiles = u16::from_be_bytes([data[1], data[2]]) as usize;

    let mut pos = 3;

    // Skip offsets array for bundled documents
    if is_bundled {
        let offsets_size = nfiles * 4;
        if pos + offsets_size > data.len() {
            return Err(Error::UnexpectedEof);
        }
        pos += offsets_size;
    }

    // Remaining bytes are BZZ-compressed metadata
    let bzz_data = &data[pos..];
    let meta =
        crate::bzz_new::bzz_decode(bzz_data).map_err(|e| Error::FormatError(e.to_string()))?;

    // Parse metadata: for each component, read size(3), flags(1), id(strNT), name?(strNT), title?(strNT)
    let mut mpos = 0;
    // First: skip sizes (3 bytes each)
    mpos += nfiles * 3;

    // Read flags for all components
    if mpos + nfiles > meta.len() {
        return Err(Error::UnexpectedEof);
    }
    let flags: Vec<u8> = meta[mpos..mpos + nfiles].to_vec();
    mpos += nfiles;

    // Read IDs and optional name/title strings
    let mut entries = Vec::with_capacity(nfiles);
    for &flag in flags.iter().take(nfiles) {
        let id = read_str_nt(&meta, &mut mpos)?;
        let has_name = (flag & 0x80) != 0;
        let has_title = (flag & 0x40) != 0;
        if has_name {
            let _ = read_str_nt(&meta, &mut mpos)?;
        }
        if has_title {
            let _ = read_str_nt(&meta, &mut mpos)?;
        }

        let comp_type = match flag & 0x3f {
            1 => ComponentType::Page,
            2 => ComponentType::Thumbnail,
            _ => ComponentType::Shared,
        };

        entries.push(DirmEntry { comp_type, id });
    }

    Ok((entries, is_bundled))
}

fn read_str_nt(data: &[u8], pos: &mut usize) -> Result<String, Error> {
    let start = *pos;
    while *pos < data.len() && data[*pos] != 0 {
        *pos += 1;
    }
    if *pos >= data.len() {
        return Err(Error::UnexpectedEof);
    }
    let s = std::str::from_utf8(&data[start..*pos])
        .map_err(|_| Error::FormatError("invalid UTF-8 in DIRM".into()))?;
    *pos += 1; // skip null terminator
    Ok(s.to_string())
}

// ============================================================
// FGbz palette parser
// ============================================================

fn parse_fgbz(data: &[u8]) -> Result<Palette, Error> {
    if data.len() < 3 {
        return Err(Error::InvalidLength);
    }

    let version = data[0];
    if (version & 0x7f) != 0 {
        return Err(Error::Unsupported("unsupported FGbz version"));
    }

    let palette_size = u16::from_be_bytes([data[1], data[2]]) as usize;
    let color_bytes = palette_size * 3;
    if data.len() < 3 + color_bytes {
        return Err(Error::UnexpectedEof);
    }

    // Colors are stored as BGR triplets
    let mut colors = Vec::with_capacity(palette_size);
    for i in 0..palette_size {
        let base = 3 + i * 3;
        let b = data[base];
        let g = data[base + 1];
        let r = data[base + 2];
        colors.push((r, g, b));
    }

    let mut indices = Vec::new();
    if (version & 0x80) != 0 {
        let idx_start = 3 + color_bytes;
        if idx_start + 3 > data.len() {
            return Err(Error::UnexpectedEof);
        }
        let data_size = ((data[idx_start] as u32) << 16)
            | ((data[idx_start + 1] as u32) << 8)
            | (data[idx_start + 2] as u32);

        let bzz_data = &data[idx_start + 3..];
        let decoded =
            crate::bzz_new::bzz_decode(bzz_data).map_err(|e| Error::FormatError(e.to_string()))?;

        // Each index is i16be
        let num_indices = data_size as usize;
        if decoded.len() < num_indices * 2 {
            return Err(Error::UnexpectedEof);
        }
        indices.reserve(num_indices);
        for i in 0..num_indices {
            let idx = i16::from_be_bytes([decoded[i * 2], decoded[i * 2 + 1]]);
            indices.push(idx);
        }
    }

    Ok(Palette { colors, indices })
}

// ============================================================
// NAVM bookmark parser
// ============================================================

fn parse_bookmark(data: &[u8], pos: &mut usize, counter: &mut usize) -> Result<Bookmark, Error> {
    if *pos >= data.len() {
        return Err(Error::UnexpectedEof);
    }
    let children_count = data[*pos] as usize;
    *pos += 1;

    let title = read_navm_string(data, pos)?;
    let url = read_navm_string(data, pos)?;
    *counter += 1;

    let mut children = Vec::with_capacity(children_count);
    for _ in 0..children_count {
        children.push(parse_bookmark(data, pos, counter)?);
    }

    Ok(Bookmark {
        title,
        url,
        children,
    })
}

fn read_navm_string(data: &[u8], pos: &mut usize) -> Result<String, Error> {
    if *pos + 3 > data.len() {
        return Err(Error::UnexpectedEof);
    }
    let len = ((data[*pos] as usize) << 16)
        | ((data[*pos + 1] as usize) << 8)
        | (data[*pos + 2] as usize);
    *pos += 3;

    if *pos + len > data.len() {
        return Err(Error::UnexpectedEof);
    }
    let s = std::str::from_utf8(&data[*pos..*pos + len])
        .map_err(|_| Error::FormatError("invalid UTF-8 in NAVM bookmark".into()))?;
    *pos += len;
    Ok(s.to_string())
}

// ============================================================
// TXTz / TXTa text layer parser
// ============================================================

fn parse_text_layer(data: &[u8]) -> Result<Option<TextLayer>, Error> {
    if data.len() < 3 {
        return Ok(None);
    }

    let mut pos = 0;

    // Read text length (u24be)
    let text_len = read_text_u24(data, &mut pos)?;

    // Read UTF-8 text
    if pos + text_len > data.len() {
        return Err(Error::UnexpectedEof);
    }
    let text = std::str::from_utf8(&data[pos..pos + text_len])
        .map_err(|_| Error::FormatError("invalid UTF-8 in text layer".into()))?
        .to_string();
    pos += text_len;

    // Read version byte
    if pos >= data.len() {
        return Ok(Some(TextLayer { text, root: None }));
    }
    let _version = data[pos];
    pos += 1;

    // Parse zone tree if there's more data
    if pos >= data.len() {
        return Ok(Some(TextLayer { text, root: None }));
    }

    let root = parse_text_zone(data, &mut pos, None, None)?;
    Ok(Some(TextLayer {
        text,
        root: Some(root),
    }))
}

/// Internal context for delta-encoded zone coordinates.
struct ZoneCtx {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    text_start: i32,
    text_len: i32,
}

fn parse_text_zone(
    data: &[u8],
    pos: &mut usize,
    parent: Option<&ZoneCtx>,
    prev: Option<&ZoneCtx>,
) -> Result<TextZone, Error> {
    if *pos >= data.len() {
        return Err(Error::UnexpectedEof);
    }

    let type_byte = data[*pos];
    *pos += 1;

    let kind = match type_byte {
        1 => TextZoneKind::Page,
        2 => TextZoneKind::Column,
        3 => TextZoneKind::Region,
        4 => TextZoneKind::Paragraph,
        5 => TextZoneKind::Line,
        6 => TextZoneKind::Word,
        7 => TextZoneKind::Character,
        _ => {
            return Err(Error::FormatError(format!(
                "unknown text zone type {}",
                type_byte
            )));
        }
    };

    // Read raw delta-encoded values
    let mut x = read_text_i16_biased(data, pos)?;
    let mut y = read_text_i16_biased(data, pos)?;
    let width = read_text_i16_biased(data, pos)?;
    let height = read_text_i16_biased(data, pos)?;
    let mut text_start = read_text_i16_biased(data, pos)?;
    let text_len = read_text_i24(data, pos)?;

    // Apply delta encoding (matches djvujs DjVuText.js decodeZone)
    if let Some(prev) = prev {
        match type_byte {
            1 | 4 | 5 => {
                // PAGE, PARAGRAPH, LINE
                x += prev.x;
                y = prev.y - (y + height);
            }
            _ => {
                // COLUMN, REGION, WORD, CHARACTER
                x += prev.x + prev.width;
                y += prev.y;
            }
        }
        text_start += prev.text_start + prev.text_len;
    } else if let Some(parent) = parent {
        x += parent.x;
        y = parent.y + parent.height - (y + height);
        text_start += parent.text_start;
    }

    // Read children count (i24)
    let children_count = read_text_i24(data, pos)?.max(0) as usize;

    let ctx = ZoneCtx {
        x,
        y,
        width,
        height,
        text_start,
        text_len,
    };

    let mut children = Vec::with_capacity(children_count);
    let mut prev_child: Option<ZoneCtx> = None;

    for _ in 0..children_count {
        let child = parse_text_zone(data, pos, Some(&ctx), prev_child.as_ref())?;
        prev_child = Some(ZoneCtx {
            x: child.x,
            y: child.y,
            width: child.width,
            height: child.height,
            text_start: child.text_start as i32,
            text_len: child.text_len as i32,
        });
        children.push(child);
    }

    Ok(TextZone {
        kind,
        x,
        y,
        width,
        height,
        text_start: text_start.max(0) as usize,
        text_len: text_len.max(0) as usize,
        children,
    })
}

fn read_text_u24(data: &[u8], pos: &mut usize) -> Result<usize, Error> {
    if *pos + 3 > data.len() {
        return Err(Error::UnexpectedEof);
    }
    let val = ((data[*pos] as usize) << 16)
        | ((data[*pos + 1] as usize) << 8)
        | (data[*pos + 2] as usize);
    *pos += 3;
    Ok(val)
}

fn read_text_i16_biased(data: &[u8], pos: &mut usize) -> Result<i32, Error> {
    if *pos + 2 > data.len() {
        return Err(Error::UnexpectedEof);
    }
    let raw = u16::from_be_bytes([data[*pos], data[*pos + 1]]);
    *pos += 2;
    Ok(raw as i32 - 0x8000)
}

fn read_text_i24(data: &[u8], pos: &mut usize) -> Result<i32, Error> {
    if *pos + 3 > data.len() {
        return Err(Error::UnexpectedEof);
    }
    let val =
        ((data[*pos] as i32) << 16) | ((data[*pos + 1] as i32) << 8) | (data[*pos + 2] as i32);
    *pos += 3;
    Ok(val)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assets_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/djvujs/library/assets")
    }

    fn golden_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/document")
    }

    #[test]
    fn page_counts() {
        let cases: &[(&str, usize)] = &[
            ("boy_jb2.djvu", 1),
            ("boy.djvu", 1),
            ("chicken.djvu", 1),
            ("navm_fgbz.djvu", 6),
            ("DjVu3Spec_bundled.djvu", 71),
            ("colorbook.djvu", 62),
        ];
        for (file, expected) in cases {
            let data = std::fs::read(assets_path().join(file)).unwrap();
            let doc = Document::parse(&data).unwrap();
            assert_eq!(
                doc.page_count(),
                *expected,
                "page count mismatch for {}",
                file
            );
        }
    }

    #[test]
    fn page_dimensions_navm_fgbz() {
        let data = std::fs::read(assets_path().join("navm_fgbz.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();

        let golden = std::fs::read_to_string(golden_path().join("navm_fgbz_sizes.txt")).unwrap();
        for (i, line) in golden.lines().enumerate() {
            let page = doc.page(i).unwrap();
            let expected = format!("width={} height={}", page.info.width, page.info.height);
            assert_eq!(
                expected,
                line.trim(),
                "size mismatch for navm_fgbz page {}",
                i + 1
            );
        }
    }

    #[test]
    fn page_dimensions_djvu3spec() {
        let data = std::fs::read(assets_path().join("DjVu3Spec_bundled.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();

        let golden =
            std::fs::read_to_string(golden_path().join("djvu3spec_bundled_sizes.txt")).unwrap();
        for (i, line) in golden.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let page = doc.page(i).unwrap();
            let expected = format!("width={} height={}", page.info.width, page.info.height);
            assert_eq!(
                expected,
                line.trim(),
                "size mismatch for djvu3spec page {}",
                i + 1
            );
        }
    }

    #[test]
    fn layer_availability() {
        // boy_jb2: mask only
        let data = std::fs::read(assets_path().join("boy_jb2.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let p = doc.page(0).unwrap();
        assert!(p.has_mask());
        assert!(!p.has_background());
        assert!(!p.has_foreground());
        assert!(!p.has_palette());

        // chicken: background only
        let data = std::fs::read(assets_path().join("chicken.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let p = doc.page(0).unwrap();
        assert!(!p.has_mask());
        assert!(p.has_background());
        assert!(!p.has_foreground());
        assert!(!p.has_palette());

        // navm_fgbz p1: mask + palette + background
        let data = std::fs::read(assets_path().join("navm_fgbz.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let p = doc.page(0).unwrap();
        assert!(p.has_mask());
        assert!(p.has_background());
        assert!(!p.has_foreground());
        assert!(p.has_palette());
    }

    #[test]
    fn decode_mask_matches_direct_boy_jb2() {
        let data = std::fs::read(assets_path().join("boy_jb2.djvu")).unwrap();

        // Via document API
        let doc = Document::parse(&data).unwrap();
        let mask_via_doc = doc.page(0).unwrap().decode_mask().unwrap().unwrap();

        // Via direct JB2 decode
        let file = crate::iff::parse(&data).unwrap();
        let sjbz = file.root.find_first(b"Sjbz").unwrap();
        let mask_direct = crate::jb2::decode(sjbz.data(), None).unwrap();

        assert_eq!(mask_via_doc.data, mask_direct.data, "mask data mismatch");
    }

    #[test]
    fn decode_mask_with_shared_dict() {
        // navm_fgbz page 1 uses INCL → dict0006.iff
        let data = std::fs::read(assets_path().join("navm_fgbz.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let mask = doc.page(0).unwrap().decode_mask().unwrap();
        assert!(mask.is_some(), "expected mask for navm_fgbz p1");
        let bm = mask.unwrap();
        assert_eq!(bm.width, 2550);
        assert_eq!(bm.height, 3300);
    }

    #[test]
    fn decode_background_chicken() {
        let data = std::fs::read(assets_path().join("chicken.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let bg = doc.page(0).unwrap().decode_background().unwrap();
        assert!(bg.is_some());
        let pm = bg.unwrap();
        assert_eq!(pm.width, 181);
        assert_eq!(pm.height, 240);
    }

    #[test]
    fn decode_palette_navm_fgbz() {
        let data = std::fs::read(assets_path().join("navm_fgbz.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let pal = doc.page(0).unwrap().decode_palette().unwrap();
        assert!(pal.is_some());
        let p = pal.unwrap();
        assert_eq!(p.colors.len(), 2); // FGbz with 2 colors per dump
    }

    #[test]
    fn page_info_dpi() {
        let data = std::fs::read(assets_path().join("navm_fgbz.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let p = doc.page(0).unwrap();
        assert_eq!(p.info.dpi, 300);
    }

    #[test]
    #[ignore]
    fn debug_bg_lowres_vs_ddjvu() {
        let cases = [
            ("carte.djvu", 0usize, "/tmp/rdjvu_debug/carte_bg_sub3.ppm"),
            (
                "colorbook.djvu",
                0usize,
                "/tmp/rdjvu_debug/colorbook_p1_bg_sub3.ppm",
            ),
            (
                "navm_fgbz.djvu",
                3usize,
                "/tmp/rdjvu_debug/navm_p4_bg_sub3.ppm",
            ),
        ];
        for (file, page_idx, ref_file) in cases {
            let ref_path = std::path::Path::new(ref_file);
            if !ref_path.exists() {
                continue;
            }
            let data = std::fs::read(assets_path().join(file)).unwrap();
            let doc = Document::parse(&data).unwrap();
            let page = doc.page(page_idx).unwrap();
            let bg = page.decode_background().unwrap().unwrap();
            let actual = bg.to_ppm();
            let expected = std::fs::read(ref_path).unwrap();
            let header_end = actual.iter().position(|&b| b == b'\n').unwrap() + 1;
            let header_end = header_end
                + actual[header_end..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .unwrap()
                + 1;
            let header_end = header_end
                + actual[header_end..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .unwrap()
                + 1;
            let a = &actual[header_end..];
            let e = &expected[header_end..];
            let mut diff_px = 0usize;
            let mut abs = [0u64; 3];
            let px = (a.len().min(e.len())) / 3;
            for p in 0..px {
                let i = p * 3;
                if a[i] != e[i] || a[i + 1] != e[i + 1] || a[i + 2] != e[i + 2] {
                    diff_px += 1;
                }
                abs[0] += (a[i] as i32 - e[i] as i32).unsigned_abs() as u64;
                abs[1] += (a[i + 1] as i32 - e[i + 1] as i32).unsigned_abs() as u64;
                abs[2] += (a[i + 2] as i32 - e[i + 2] as i32).unsigned_abs() as u64;
            }
            eprintln!(
                "{} p{} bg-lowres mismatch_px={} mean_abs=({:.3},{:.3},{:.3}) dims_a={} dims_e={}",
                file,
                page_idx + 1,
                diff_px,
                abs[0] as f64 / px as f64,
                abs[1] as f64 / px as f64,
                abs[2] as f64 / px as f64,
                a.len() / 3,
                e.len() / 3
            );
        }
    }

    #[test]
    fn bookmarks_navm_fgbz() {
        let data = std::fs::read(assets_path().join("navm_fgbz.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let bm = doc.bookmarks().unwrap();

        // 4 top-level bookmarks
        assert_eq!(bm.len(), 4);

        assert_eq!(bm[0].title, "Links");
        assert_eq!(bm[0].url, "#1");
        assert!(bm[0].children.is_empty());

        assert_eq!(bm[1].title, "Ink, Rectangles, Ellipses, Lines");
        assert_eq!(bm[1].url, "#2");
        assert!(bm[1].children.is_empty());

        assert_eq!(bm[2].title, "Stamps");
        assert_eq!(bm[2].url, "#3");
        assert_eq!(bm[2].children.len(), 2);
        assert_eq!(bm[2].children[0].title, "Stamps - Faces");
        assert_eq!(bm[2].children[0].url, "#4");
        assert_eq!(bm[2].children[1].title, "Stamps - Pointers");
        assert_eq!(bm[2].children[1].url, "#5");

        assert_eq!(bm[3].title, "Last Page");
        assert_eq!(bm[3].url, "#6");
        assert!(bm[3].children.is_empty());
    }

    #[test]
    fn bookmarks_empty_for_single_page() {
        let data = std::fs::read(assets_path().join("boy_jb2.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let bm = doc.bookmarks().unwrap();
        assert!(bm.is_empty());
    }

    #[test]
    fn bookmarks_empty_for_no_navm() {
        let data = std::fs::read(assets_path().join("colorbook.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let bm = doc.bookmarks().unwrap();
        assert!(bm.is_empty());
    }

    // --- Phase 6.2: Edge case tests ---

    #[test]
    fn document_empty_input() {
        assert!(Document::parse(&[]).is_err());
    }

    #[test]
    fn document_truncated_file() {
        // Just the AT&T magic, not enough for a FORM
        assert!(Document::parse(b"AT&T").is_err());
    }

    #[test]
    fn document_missing_info_chunk() {
        // Valid IFF structure but no INFO chunk — page should have sensible error
        let mut data = b"AT&TFORM".to_vec();
        let form_size = 4 + 4 + 4 + 4; // secondary + chunk_id + size + data(4)
        data.extend_from_slice(&(form_size as u32).to_be_bytes());
        data.extend_from_slice(b"DJVU");
        data.extend_from_slice(b"Sjbz");
        data.extend_from_slice(&4u32.to_be_bytes());
        data.extend_from_slice(&[0u8; 4]);
        let result = Document::parse(&data);
        // Should either fail to parse or fail when accessing page info
        match result {
            Err(_) => {} // expected
            Ok(doc) => {
                assert!(doc.page(0).is_err());
            }
        }
    }

    #[test]
    fn document_page_out_of_bounds() {
        let data = std::fs::read(assets_path().join("boy_jb2.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        assert_eq!(doc.page_count(), 1);
        assert!(doc.page(1).is_err());
        assert!(doc.page(100).is_err());
    }

    #[test]
    fn document_missing_optional_chunks() {
        // boy_jb2.djvu has no BG44 or FG44 — decode should return None
        let data = std::fs::read(assets_path().join("boy_jb2.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        assert!(page.decode_background().unwrap().is_none());
        assert!(page.decode_foreground().unwrap().is_none());
        assert!(!page.has_palette());
    }

    // --- Text extraction tests ---

    fn text_golden_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/text")
    }

    /// Format a TextZone tree as djvused print-txt output for comparison.
    fn format_zone(layer: &TextLayer, zone: &TextZone, indent: usize) -> String {
        let mut out = String::new();
        let pad = " ".repeat(indent);
        let kind_str = match zone.kind {
            TextZoneKind::Page => "page",
            TextZoneKind::Column => "column",
            TextZoneKind::Region => "region",
            TextZoneKind::Paragraph => "para",
            TextZoneKind::Line => "line",
            TextZoneKind::Word => "word",
            TextZoneKind::Character => "char",
        };
        let x2 = zone.x + zone.width;
        let y2 = zone.y + zone.height;

        if zone.children.is_empty() {
            // Leaf zone: include text (strip trailing whitespace like djvused)
            let text = layer.zone_text(zone);
            let trimmed = text.trim_end();
            let escaped = djvused_escape(trimmed);
            out.push_str(&format!(
                "{}({} {} {} {} {} \"{}\")",
                pad, kind_str, zone.x, zone.y, x2, y2, escaped
            ));
        } else {
            out.push_str(&format!(
                "{}({} {} {} {} {}",
                pad, kind_str, zone.x, zone.y, x2, y2
            ));
            for child in &zone.children {
                out.push('\n');
                out.push_str(&format_zone(layer, child, indent + 1));
            }
            out.push(')');
        }
        out
    }

    /// Escape text like djvused: non-printable and non-ASCII bytes as 3-digit octal.
    fn djvused_escape(text: &str) -> String {
        let mut out = String::new();
        for b in text.bytes() {
            match b {
                b'\\' => out.push_str("\\\\"),
                b'"' => out.push_str("\\\""),
                0x20..=0x7e => out.push(b as char),
                _ => out.push_str(&format!("\\{:03o}", b)),
            }
        }
        out
    }

    fn format_text_layer(layer: &TextLayer) -> String {
        match &layer.root {
            Some(root) => format_zone(layer, root, 0),
            None => String::new(),
        }
    }

    #[test]
    fn text_layer_none_for_no_text() {
        // boy_jb2 has no text layer
        let data = std::fs::read(assets_path().join("boy_jb2.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let tl = doc.page(0).unwrap().text_layer().unwrap();
        assert!(tl.is_none());
    }

    #[test]
    fn text_layer_carte_p1() {
        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let tl = doc.page(0).unwrap().text_layer().unwrap().unwrap();

        // Verify text is non-empty
        assert!(!tl.text.is_empty(), "carte text should not be empty");

        // Verify root zone is PAGE type
        let root = tl.root.as_ref().unwrap();
        assert_eq!(root.kind, TextZoneKind::Page);

        // Compare against golden djvused output
        let golden = std::fs::read_to_string(text_golden_path().join("carte_p1.txt")).unwrap();
        let actual = format_text_layer(&tl);
        assert_eq!(actual.trim(), golden.trim(), "carte p1 text mismatch");
    }

    #[test]
    fn text_layer_djvu3spec_p1() {
        let data = std::fs::read(assets_path().join("DjVu3Spec_bundled.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let tl = doc.page(0).unwrap().text_layer().unwrap().unwrap();

        assert!(!tl.text.is_empty());

        let root = tl.root.as_ref().unwrap();
        assert_eq!(root.kind, TextZoneKind::Page);
        // DjVu3Spec has full hierarchy: page → column → region → para → line → word
        assert!(!root.children.is_empty());

        let golden = std::fs::read_to_string(text_golden_path().join("djvu3spec_p1.txt")).unwrap();
        let actual = format_text_layer(&tl);
        assert_eq!(actual.trim(), golden.trim(), "djvu3spec p1 text mismatch");
    }

    #[test]
    fn text_layer_colorbook_p1() {
        let data = std::fs::read(assets_path().join("colorbook.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let tl = doc.page(0).unwrap().text_layer().unwrap().unwrap();

        assert!(!tl.text.is_empty());

        let golden = std::fs::read_to_string(text_golden_path().join("colorbook_p1.txt")).unwrap();
        let actual = format_text_layer(&tl);
        assert_eq!(actual.trim(), golden.trim(), "colorbook p1 text mismatch");
    }

    #[test]
    fn text_layer_czech_p6_utf8() {
        // Czech text with non-ASCII characters
        let data = std::fs::read(assets_path().join("czech.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let tl = doc.page(5).unwrap().text_layer().unwrap().unwrap();

        assert!(!tl.text.is_empty());

        let golden = std::fs::read_to_string(text_golden_path().join("czech_p6.txt")).unwrap();
        let actual = format_text_layer(&tl);
        assert_eq!(actual.trim(), golden.trim(), "czech p6 text mismatch");
    }

    #[test]
    fn text_layer_zone_text_access() {
        // Test the zone_text helper
        let data = std::fs::read(assets_path().join("DjVu3Spec_bundled.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let tl = doc.page(0).unwrap().text_layer().unwrap().unwrap();

        // Find the first word zone
        fn find_first_word(zone: &TextZone) -> Option<&TextZone> {
            if zone.kind == TextZoneKind::Word {
                return Some(zone);
            }
            for child in &zone.children {
                if let Some(w) = find_first_word(child) {
                    return Some(w);
                }
            }
            None
        }

        let root = tl.root.as_ref().unwrap();
        let word = find_first_word(root).expect("should have at least one word");
        let text = tl.zone_text(word);
        assert!(!text.is_empty(), "first word text should not be empty");
    }

    #[test]
    fn text_layer_all_pages_djvu3spec() {
        // All 71 pages should parse without error
        let data = std::fs::read(assets_path().join("DjVu3Spec_bundled.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        for i in 0..doc.page_count() {
            let result = doc.page(i).unwrap().text_layer();
            assert!(result.is_ok(), "text_layer failed for djvu3spec page {}", i);
        }
    }

    // --- Thumbnail tests ---

    #[test]
    fn thumbnail_carte() {
        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let thumb = doc
            .thumbnail(0)
            .unwrap()
            .expect("carte should have a thumbnail");
        // Thumbnail should be much smaller than the page (4200x2556)
        assert!(
            thumb.width > 0 && thumb.width < 500,
            "thumb width: {}",
            thumb.width
        );
        assert!(
            thumb.height > 0 && thumb.height < 500,
            "thumb height: {}",
            thumb.height
        );
        assert_eq!(
            thumb.data.len(),
            thumb.width as usize * thumb.height as usize * 4
        );
    }

    #[test]
    fn thumbnail_djvu3spec_all_pages() {
        let data = std::fs::read(assets_path().join("DjVu3Spec_bundled.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let mut count = 0;
        for i in 0..doc.page_count() {
            if let Some(thumb) = doc.thumbnail(i).unwrap() {
                assert!(
                    thumb.width > 0 && thumb.height > 0,
                    "page {} thumb empty",
                    i
                );
                assert_eq!(
                    thumb.data.len(),
                    thumb.width as usize * thumb.height as usize * 4,
                    "page {} thumb data mismatch",
                    i
                );
                count += 1;
            }
        }
        assert_eq!(count, 71, "expected 71 thumbnails, got {}", count);
    }

    #[test]
    fn thumbnail_none_for_single_page() {
        let data = std::fs::read(assets_path().join("boy_jb2.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        assert!(doc.thumbnail(0).unwrap().is_none());
    }

    #[test]
    fn thumbnail_none_for_no_thum() {
        let data = std::fs::read(assets_path().join("colorbook.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        assert!(doc.thumbnail(0).unwrap().is_none());
    }

    // ── Progressive decode ─────────────────────────────────────────────

    #[test]
    fn progressive_bg_returns_frames_per_chunk() {
        // carte.djvu is a color page with multiple BG44 chunks.
        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        let chunk_count = page.bg44_chunk_count();
        assert!(
            chunk_count > 1,
            "need multi-chunk file for progressive test"
        );

        let frames = page.decode_background_progressive().unwrap().unwrap();
        assert_eq!(frames.len(), chunk_count, "one frame per BG44 chunk");

        // Each frame should have the same dimensions.
        let (w, h) = (frames[0].width, frames[0].height);
        for (i, f) in frames.iter().enumerate() {
            assert_eq!((f.width, f.height), (w, h), "frame {i} size mismatch");
        }
    }

    #[test]
    fn progressive_last_frame_matches_full_decode() {
        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();

        let full = page.decode_background().unwrap().unwrap();
        let frames = page.decode_background_progressive().unwrap().unwrap();
        let last = frames.last().unwrap();

        assert_eq!(full.width, last.width);
        assert_eq!(full.height, last.height);
        assert_eq!(
            full.data, last.data,
            "last progressive frame must match full decode"
        );
    }

    #[test]
    fn progressive_single_chunk_returns_one_frame() {
        // boy.djvu has a background but likely only 1 BG44 chunk.
        let data = std::fs::read(assets_path().join("boy.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        if page.bg44_chunk_count() <= 1 {
            let frames = page.decode_background_progressive().unwrap().unwrap();
            assert_eq!(frames.len(), 1);
        }
    }

    #[test]
    fn coarse_decode_returns_blurry_frame() {
        let data = std::fs::read(assets_path().join("carte.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        assert!(page.bg44_chunk_count() > 1);

        let coarse = page.decode_background_coarse().unwrap().unwrap();
        let full = page.decode_background().unwrap().unwrap();

        // Same dimensions, different pixel data (coarse is blurrier).
        assert_eq!(coarse.width, full.width);
        assert_eq!(coarse.height, full.height);
        assert_ne!(coarse.data, full.data, "coarse should differ from full");
    }

    #[test]
    fn coarse_decode_single_chunk_returns_none() {
        let data = std::fs::read(assets_path().join("boy.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        if page.bg44_chunk_count() <= 1 {
            assert!(page.decode_background_coarse().unwrap().is_none());
        }
    }

    #[test]
    fn progressive_no_bg_returns_none() {
        // boy_jb2.djvu has no BG44 chunks.
        let data = std::fs::read(assets_path().join("boy_jb2.djvu")).unwrap();
        let doc = Document::parse(&data).unwrap();
        let page = doc.page(0).unwrap();
        assert_eq!(page.bg44_chunk_count(), 0);
        assert!(page.decode_background_progressive().unwrap().is_none());
    }
}
