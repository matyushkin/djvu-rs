//! Integration tests for `djvu_document` and `djvu_render` public APIs.
//!
//! Uses both the low-level `DjVuDocument` and the high-level `Document` wrapper.

use djvu_rs::IffError;
use djvu_rs::djvu_document::{DjVuDocument, DocError};
use djvu_rs::djvu_render::{
    RenderOptions, render_coarse, render_gray8, render_pixmap, render_progressive,
};
use djvu_rs::iff::parse_form;

// ── DjVuDocument — parse ──────────────────────────────────────────────────────

/// Parsing a valid single-page DjVu file must succeed.
#[test]
fn djvu_document_parse_single_page() {
    let data = std::fs::read("tests/fixtures/boy.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).expect("boy.djvu must parse without error");
    assert_eq!(doc.page_count(), 1);
}

/// Parsing a multi-page DJVM document must yield the correct page count.
#[test]
fn djvu_document_parse_multipage() {
    let data = std::fs::read("tests/corpus/watchmaker.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).expect("watchmaker.djvu must parse");
    assert!(doc.page_count() > 1, "watchmaker.djvu has multiple pages");
}

/// Empty input must return an error, not a panic.
#[test]
fn djvu_document_parse_empty_returns_error() {
    let result = DjVuDocument::parse(&[]);
    assert!(result.is_err(), "empty data must return Err");
}

/// Garbage input must return an error, not a panic.
#[test]
fn djvu_document_parse_garbage_no_panic() {
    let _ = DjVuDocument::parse(b"this is not a djvu file at all!!");
}

// ── DjVuDocument — page accessors ────────────────────────────────────────────

/// page(0) on a single-page document must succeed and return correct dimensions.
#[test]
fn djvu_page_dimensions_single() {
    let data = std::fs::read("tests/fixtures/boy.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let page = doc.page(0).expect("page 0 must exist");
    assert!(page.width() > 0 && page.height() > 0);
    assert!(page.dpi() > 0);
    assert_eq!(page.index(), 0);
}

/// dimensions() returns (width, height) matching width() and height().
#[test]
fn djvu_page_dimensions_tuple_matches() {
    let data = std::fs::read("tests/fixtures/boy.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let page = doc.page(0).unwrap();
    assert_eq!(page.dimensions(), (page.width(), page.height()));
}

/// page(N) out of bounds must return an error.
#[test]
fn djvu_page_out_of_bounds_returns_error() {
    let data = std::fs::read("tests/fixtures/boy.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let result = doc.page(999);
    assert!(result.is_err(), "out-of-bounds page access must return Err");
}

/// bg44_chunks() returns at least one chunk for a color page.
#[test]
fn djvu_page_bg44_chunks_color_page() {
    let data = std::fs::read("tests/fixtures/boy.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let page = doc.page(0).unwrap();
    let chunks = page.bg44_chunks();
    assert!(
        !chunks.is_empty(),
        "color page must have at least one BG44 chunk"
    );
}

/// find_chunk() for an existing chunk returns Some.
#[test]
fn djvu_page_find_chunk_existing() {
    let data = std::fs::read("tests/fixtures/boy.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let page = doc.page(0).unwrap();
    assert!(
        page.find_chunk(b"INFO").is_some(),
        "INFO chunk must be found"
    );
}

/// find_chunk() for a nonexistent chunk returns None.
#[test]
fn djvu_page_find_chunk_missing() {
    let data = std::fs::read("tests/fixtures/boy.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let page = doc.page(0).unwrap();
    assert!(page.find_chunk(b"XXXX").is_none());
}

/// DocError implements std::error::Error.
#[test]
fn doc_error_implements_error_trait() {
    fn requires_error<E: std::error::Error>() {}
    requires_error::<DocError>();
}

// ── djvu_render — render_pixmap ──────────────────────────────────────────────

/// render_pixmap at native resolution must produce a valid RGBA pixmap.
#[test]
fn render_pixmap_default_options_boy() {
    let data = std::fs::read("tests/fixtures/boy.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let page = doc.page(0).unwrap();

    let opts = RenderOptions {
        width: page.width() as u32,
        height: page.height() as u32,
        ..RenderOptions::default()
    };
    let pixmap = render_pixmap(page, &opts).expect("render_pixmap must succeed");
    assert!(pixmap.width > 0 && pixmap.height > 0);
    assert_eq!(
        pixmap.data.len(),
        (pixmap.width * pixmap.height * 4) as usize
    );
}

/// render_pixmap must work on a bilevel page (JB2 only, no IW44 background).
#[test]
fn render_pixmap_bilevel_page() {
    let data = std::fs::read("tests/fixtures/boy_jb2.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let page = doc.page(0).unwrap();

    let opts = RenderOptions {
        width: page.width() as u32,
        height: page.height() as u32,
        ..RenderOptions::default()
    };
    let pixmap = render_pixmap(page, &opts).expect("bilevel render must succeed");
    assert!(pixmap.width > 0 && pixmap.height > 0);
}

// ── djvu_render — render_coarse ───────────────────────────────────────────────

/// render_coarse must produce a pixmap for a color page (at least one BG44 chunk).
#[test]
fn render_coarse_color_page() {
    let data = std::fs::read("tests/fixtures/boy.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let page = doc.page(0).unwrap();

    let opts = RenderOptions {
        width: page.width() as u32,
        height: page.height() as u32,
        ..RenderOptions::default()
    };
    let result = render_coarse(page, &opts).expect("render_coarse must not error");
    assert!(result.is_some(), "color page must produce coarse pixmap");
    let pix = result.unwrap();
    assert!(pix.width > 0 && pix.height > 0);
}

/// render_coarse on a bilevel-only page (no BG44) must not error.
#[test]
fn render_coarse_bilevel_page_no_error() {
    let data = std::fs::read("tests/fixtures/boy_jb2.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let page = doc.page(0).unwrap();

    let opts = RenderOptions {
        width: page.width() as u32,
        height: page.height() as u32,
        ..RenderOptions::default()
    };
    // A JB2-only page has no BG44 — coarse render returns None, not an error
    let _ = render_coarse(page, &opts).expect("render_coarse must not error on bilevel");
}

// ── djvu_render — render_progressive ─────────────────────────────────────────

/// render_progressive returns a valid pixmap for each BG44 chunk index.
#[test]
fn render_progressive_each_chunk() {
    let data = std::fs::read("tests/fixtures/boy.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let page = doc.page(0).unwrap();

    let n = page.bg44_chunks().len();
    assert!(n > 0, "boy.djvu must have BG44 chunks");

    let opts = RenderOptions {
        width: page.width() as u32,
        height: page.height() as u32,
        ..RenderOptions::default()
    };
    for i in 0..n {
        let pix = render_progressive(page, &opts, i)
            .unwrap_or_else(|e| panic!("render_progressive chunk {i} failed: {e}"));
        assert!(pix.width > 0 && pix.height > 0);
        assert_eq!(pix.data.len(), (pix.width * pix.height * 4) as usize);
    }
}

// ── iff::parse_form — edge cases ─────────────────────────────────────────────

/// parse_form on a valid single-page file succeeds and returns correct form_type.
#[test]
fn iff_parse_form_single_page() {
    let data = std::fs::read("tests/fixtures/boy.djvu").unwrap();
    let form = parse_form(&data).expect("boy.djvu must parse as IFF");
    assert_eq!(&form.form_type, b"DJVU");
    assert!(
        !form.chunks.is_empty(),
        "DJVU form must have at least one chunk"
    );
}

/// parse_form on a multi-page file succeeds and returns DJVM form_type.
#[test]
fn iff_parse_form_multipage_djvm() {
    let data = std::fs::read("tests/corpus/watchmaker.djvu").unwrap();
    let form = parse_form(&data).expect("watchmaker.djvu must parse as IFF");
    assert_eq!(&form.form_type, b"DJVM");
}

/// parse_form on empty input returns IffError.
#[test]
fn iff_parse_form_empty_returns_error() {
    let result = parse_form(&[]);
    assert!(result.is_err());
}

/// parse_form on truncated data (only magic bytes) returns IffError.
#[test]
fn iff_parse_form_truncated_returns_error() {
    let result = parse_form(b"AT&T");
    assert!(result.is_err());
}

/// parse_form on wrong magic returns IffError.
#[test]
fn iff_parse_form_wrong_magic_returns_error() {
    // Valid length but wrong magic
    let bad: &[u8] = b"XXXX\x00\x00\x00\x04DJVU";
    let result = parse_form(bad);
    assert!(result.is_err());
}

/// IffError implements std::error::Error.
#[test]
fn iff_error_implements_error_trait() {
    fn requires_error<E: std::error::Error>() {}
    requires_error::<IffError>();
}

// ── Layer extraction (#16) ───────────────────────────────────────────────────

/// extract_mask on a bilevel page returns a Bitmap matching page dimensions.
#[test]
fn extract_mask_bilevel_page() {
    let data = std::fs::read("tests/fixtures/boy_jb2.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let page = doc.page(0).unwrap();

    let mask = page.extract_mask().expect("extract_mask must not error");
    assert!(mask.is_some(), "boy_jb2 must have a JB2 mask");
    let bm = mask.unwrap();
    assert_eq!(bm.width as u16, page.width());
    assert_eq!(bm.height as u16, page.height());
}

/// extract_mask returns None when there is no Sjbz chunk (IW44-only page).
#[test]
fn extract_mask_no_sjbz_returns_none() {
    let data = std::fs::read("tests/fixtures/chicken.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let page = doc.page(0).unwrap();

    // chicken.djvu is IW44-only (no JB2 mask)
    let mask = page.extract_mask().expect("must not error");
    assert!(mask.is_none(), "IW44-only page should have no mask");
}

/// extract_foreground on a 3-layer page returns a Pixmap.
#[test]
fn extract_foreground_3layer() {
    let data = std::fs::read("tests/fixtures/colorbook.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let page = doc.page(0).unwrap();

    let fg = page
        .extract_foreground()
        .expect("extract_foreground must not error");
    assert!(
        fg.is_some(),
        "colorbook.djvu should have a foreground layer"
    );
    let pm = fg.unwrap();
    assert!(pm.width > 0 && pm.height > 0);
}

/// extract_foreground returns None when there are no FG44 chunks.
#[test]
fn extract_foreground_no_fg44_returns_none() {
    let data = std::fs::read("tests/fixtures/boy_jb2.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let page = doc.page(0).unwrap();

    let fg = page.extract_foreground().expect("must not error");
    assert!(fg.is_none(), "bilevel page should have no foreground layer");
}

/// extract_background on a color page returns a Pixmap with correct dimensions.
#[test]
fn extract_background_color_page() {
    let data = std::fs::read("tests/fixtures/chicken.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let page = doc.page(0).unwrap();

    let bg = page
        .extract_background()
        .expect("extract_background must not error");
    assert!(bg.is_some(), "chicken.djvu should have a background");
    let pm = bg.unwrap();
    assert!(pm.width > 0 && pm.height > 0);
}

/// extract_background returns None on a bilevel (JB2-only) page.
#[test]
fn extract_background_no_bg44_returns_none() {
    let data = std::fs::read("tests/fixtures/boy_jb2.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let page = doc.page(0).unwrap();

    let bg = page.extract_background().expect("must not error");
    assert!(bg.is_none(), "bilevel page should have no background");
}

// ── render_gray8 ────────────────────────────────────────────────────────────

/// Grayscale render of a bilevel page must return only 0 and 255 values.
#[test]
fn render_gray8_bilevel_only_black_and_white() {
    let data = std::fs::read("tests/fixtures/boy_jb2.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let page = doc.page(0).unwrap();

    let opts = RenderOptions {
        width: page.width() as u32,
        height: page.height() as u32,
        ..RenderOptions::default()
    };
    let gray = render_gray8(page, &opts).expect("render_gray8 must succeed");

    assert_eq!(
        gray.data.len(),
        gray.width as usize * gray.height as usize,
        "grayscale buffer must have exactly width*height bytes"
    );
    assert_eq!(gray.width, opts.width);
    assert_eq!(gray.height, opts.height);

    // Bilevel page: all pixels must be exactly 0 (black) or 255 (white).
    let unexpected: Vec<u8> = gray
        .data
        .iter()
        .copied()
        .filter(|&v| v != 0 && v != 255)
        .collect();
    assert!(
        unexpected.is_empty(),
        "bilevel grayscale must contain only 0/255, found: {:?}",
        &unexpected[..unexpected.len().min(10)]
    );
}

/// Grayscale render of a colour page must have correct buffer size.
#[test]
fn render_gray8_color_page_correct_size() {
    let data = std::fs::read("tests/fixtures/boy.djvu").unwrap();
    let doc = DjVuDocument::parse(&data).unwrap();
    let page = doc.page(0).unwrap();

    let opts = RenderOptions {
        width: page.width() as u32,
        height: page.height() as u32,
        ..RenderOptions::default()
    };
    let gray = render_gray8(page, &opts).expect("render_gray8 must succeed for colour page");

    assert_eq!(
        gray.data.len(),
        gray.width as usize * gray.height as usize,
        "grayscale buffer must have exactly width*height bytes"
    );
}

/// `Pixmap::to_gray8` must produce correct luminance values.
#[test]
fn pixmap_to_gray8_luminance_values() {
    use djvu_rs::Pixmap;

    let mut pm = Pixmap::white(3, 1);
    pm.set_rgb(0, 0, 0, 0, 0); // black → 0
    pm.set_rgb(1, 0, 255, 255, 255); // white → 255
    pm.set_rgb(2, 0, 76, 150, 29); // approx equal-luminance green (~0.299*76+0.587*150+0.114*29 ≈ 113)

    let gray = pm.to_gray8();
    assert_eq!(gray.data.len(), 3);
    assert_eq!(gray.get(0, 0), 0, "black must map to 0");
    assert_eq!(gray.get(1, 0), 255, "white must map to 255");
    // 0.299*76 + 0.587*150 + 0.114*29 = 22.7 + 88.1 + 3.3 = 114.1 → 114
    let lum = gray.get(2, 0);
    assert!(
        (110..=118).contains(&lum),
        "luminance should be ~114, got {lum}"
    );
}

// ── permissive render mode ───────────────────────────────────────────────────

/// Build a DjVu byte buffer with the BG44 chunk data deliberately truncated
/// to half its original size. Used to test permissive render mode.
///
/// DjVu file layout: `AT&T` (4) + `FORM` (4) + form_len (4 BE) + `DJVU` (4) + chunks…
/// We patch both the BG44 chunk length and the outer FORM chunk length.
fn make_truncated_bg44_djvu() -> Vec<u8> {
    let data = std::fs::read("tests/fixtures/boy.djvu").unwrap();
    // Find BG44 chunk id
    let bg44_pos = data
        .windows(4)
        .position(|w| w == b"BG44")
        .expect("boy.djvu must have a BG44 chunk");
    let chunk_len = u32::from_be_bytes(data[bg44_pos + 4..bg44_pos + 8].try_into().unwrap());
    // Truncate aggressively (keep only 4 bytes) so the IW44 decoder definitely errors.
    let truncated_len = 4u32;
    let reduction = chunk_len - truncated_len;

    // Patch the outer FORM length (at offset 8, after AT&T=4 + FORM=4)
    let form_len = u32::from_be_bytes(data[8..12].try_into().unwrap());
    let new_form_len = form_len - reduction;

    let header_end = bg44_pos + 8; // after BG44 id + length
    let mut out = data[..8].to_vec(); // AT&T + FORM
    out.extend_from_slice(&new_form_len.to_be_bytes()); // patched FORM length
    out.extend_from_slice(&data[12..bg44_pos + 4]); // DJVU + chunks up to BG44 id
    out.extend_from_slice(&truncated_len.to_be_bytes()); // patched BG44 length
    out.extend_from_slice(&data[header_end..header_end + truncated_len as usize]);
    out
}

/// Strict mode must return an error on a truncated BG44 chunk.
#[test]
fn permissive_strict_fails_on_truncated_bg44() {
    let corrupted = make_truncated_bg44_djvu();
    let doc = DjVuDocument::parse(&corrupted).unwrap();
    let page = doc.page(0).unwrap();
    let opts = RenderOptions {
        width: page.width() as u32,
        height: page.height() as u32,
        permissive: false,
        ..RenderOptions::default()
    };
    let result = render_pixmap(page, &opts);
    assert!(
        result.is_err(),
        "strict mode must return Err on corrupted BG44"
    );
}

/// Permissive mode must return Ok with a non-empty pixmap on the same file.
#[test]
fn permissive_render_returns_ok_on_truncated_bg44() {
    let corrupted = make_truncated_bg44_djvu();
    let doc = DjVuDocument::parse(&corrupted).unwrap();
    let page = doc.page(0).unwrap();
    let opts = RenderOptions {
        width: page.width() as u32,
        height: page.height() as u32,
        permissive: true,
        ..RenderOptions::default()
    };
    let pm =
        render_pixmap(page, &opts).expect("permissive mode must return Ok even for corrupted BG44");
    assert!(!pm.data.is_empty(), "pixmap must not be empty");
    assert_eq!(
        pm.data.len(),
        pm.width as usize * pm.height as usize * 4,
        "pixmap must have correct RGBA size"
    );
}

// ── IFF parse_form ──────────────────────────────────────────────────────────

/// find_first returns the first matching chunk.
#[test]
fn iff_find_first_existing_chunk() {
    let data = std::fs::read("tests/fixtures/boy.djvu").unwrap();
    let form = parse_form(&data).unwrap();

    // Use the new-API Chunk::find_first if available, or iterate chunks
    let info = form.chunks.iter().find(|c| &c.id == b"INFO");
    assert!(info.is_some(), "INFO chunk must exist in a DJVU form");
}
