use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use djvu_rs::Document;

#[derive(Parser)]
#[command(name = "djvu", about = "DjVu file utility", version)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show document info: page count, dimensions, DPI.
    Info {
        /// Path to the DjVu file.
        file: PathBuf,
    },
    /// Render pages to PNG, PDF, or CBZ.
    Render {
        /// Path to the DjVu file.
        file: PathBuf,
        /// Page number to render (1-based). Default: 1.
        #[arg(short, long, default_value = "1")]
        page: usize,
        /// Render all pages.
        #[arg(long, conflicts_with = "page")]
        all: bool,
        /// Output DPI. Default: 150.
        #[arg(short, long, default_value = "150")]
        dpi: u32,
        /// Output format.
        #[arg(short, long, default_value = "png", value_enum)]
        format: Format,
        /// Output file (single page) or directory (--all, PNG only).
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Extract the text layer from a DjVu document.
    Text {
        /// Path to the DjVu file.
        file: PathBuf,
        /// Page number to extract (1-based). Default: 1.
        #[arg(short, long, default_value = "1")]
        page: usize,
        /// Extract text from all pages.
        #[arg(long, conflicts_with = "page")]
        all: bool,
    },
}

#[derive(Clone, ValueEnum)]
enum Format {
    Png,
    Pdf,
    Cbz,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Cmd::Info { file } => cmd_info(&file),
        Cmd::Render {
            file,
            page,
            all,
            dpi,
            format,
            output,
        } => cmd_render(&file, page, all, dpi, format, &output),
        Cmd::Text { file, page, all } => cmd_text(&file, page, all),
    }
}

// ── info ──────────────────────────────────────────────────────────────────────

fn cmd_info(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let doc = open(path)?;
    let count = doc.page_count();
    println!("Pages: {count}");
    for i in 0..count {
        let page = doc.page(i)?;
        println!(
            "  Page {:>4}: {} x {} px  {} dpi",
            i + 1,
            page.width(),
            page.height(),
            page.dpi(),
        );
    }
    Ok(())
}

// ── render ────────────────────────────────────────────────────────────────────

fn cmd_render(
    path: &Path,
    page: usize,
    all: bool,
    dpi: u32,
    format: Format,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let doc = open(path)?;
    let count = doc.page_count();

    match format {
        Format::Png => render_png(&doc, page, all, dpi, count, output),
        Format::Pdf => render_pdf(&doc, page, all, dpi, count, output),
        Format::Cbz => render_cbz(&doc, page, all, dpi, count, output),
    }
}

fn render_png(
    doc: &Document,
    page: usize,
    all: bool,
    dpi: u32,
    count: usize,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if all {
        std::fs::create_dir_all(output)?;
        for i in 0..count {
            let out = output.join(format!("page_{:04}.png", i + 1));
            render_page_png(doc, i, dpi, &out)?;
        }
    } else {
        let idx = page_idx(page, count)?;
        if let Some(parent) = output.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        render_page_png(doc, idx, dpi, output)?;
    }
    Ok(())
}

fn render_pdf(
    doc: &Document,
    page: usize,
    all: bool,
    dpi: u32,
    count: usize,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let pages: Vec<usize> = if all {
        (0..count).collect()
    } else {
        vec![page_idx(page, count)?]
    };

    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let mut pdf_pages = Vec::new();
    for idx in pages {
        let p = doc.page(idx)?;
        let native_dpi = p.dpi() as f32;
        let scale = dpi as f32 / native_dpi;
        let w = ((p.width() as f32 * scale).round() as u32).max(1);
        let h = ((p.height() as f32 * scale).round() as u32).max(1);
        let pixmap = p.render_to_size(w, h)?;
        // Convert RGBA → RGB
        let rgb: Vec<u8> = pixmap
            .data
            .chunks_exact(4)
            .flat_map(|px| [px[0], px[1], px[2]])
            .collect();
        pdf_pages.push((w, h, dpi, rgb));
    }

    let pdf = build_pdf(&pdf_pages);
    std::fs::write(output, pdf)?;
    Ok(())
}

fn render_cbz(
    doc: &Document,
    page: usize,
    all: bool,
    dpi: u32,
    count: usize,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let pages: Vec<usize> = if all {
        (0..count).collect()
    } else {
        vec![page_idx(page, count)?]
    };

    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let file = std::fs::File::create(output)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for (n, idx) in pages.iter().enumerate() {
        let p = doc.page(*idx)?;
        let native_dpi = p.dpi() as f32;
        let scale = dpi as f32 / native_dpi;
        let w = ((p.width() as f32 * scale).round() as u32).max(1);
        let h = ((p.height() as f32 * scale).round() as u32).max(1);
        let pixmap = p.render_to_size(w, h)?;

        let mut png_buf = Vec::new();
        encode_png(&mut png_buf, pixmap.width, pixmap.height, &pixmap.data)?;

        let name = format!("page_{:04}.png", n + 1);
        zip.start_file(name, opts)?;
        use std::io::Write;
        zip.write_all(&png_buf)?;
    }

    zip.finish()?;
    Ok(())
}

// ── PDF builder ───────────────────────────────────────────────────────────────

/// Build a minimal valid PDF with one rasterized RGB image per page.
///
/// Uses FlateDecode (zlib) for image streams — lossless, no JPEG artefacts.
/// The PDF structure follows ISO 32000-1 §7 (cross-reference table, object
/// streams) at the simplest level: one XObject image per page, no fonts.
fn build_pdf(pages: &[(u32, u32, u32, Vec<u8>)]) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut offsets: Vec<usize> = Vec::new();

    // Object numbering:
    //  1 — Catalog
    //  2 — Pages
    //  3, 5, 7, … — Page objects      (3 + 2*i)
    //  4, 6, 8, … — Image XObjects    (4 + 2*i)

    buf.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");

    let n = pages.len();
    let page_obj_ids: Vec<usize> = (0..n).map(|i| 3 + 2 * i).collect();
    let image_obj_ids: Vec<usize> = (0..n).map(|i| 4 + 2 * i).collect();

    // 1 — Catalog
    offsets.push(buf.len());
    let kids: String = page_obj_ids
        .iter()
        .map(|id| format!("{id} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // 2 — Pages
    offsets.push(buf.len());
    buf.extend_from_slice(
        format!("2 0 obj\n<< /Type /Pages /Kids [{kids}] /Count {n} >>\nendobj\n").as_bytes(),
    );

    for (i, (w, h, dpi, rgb)) in pages.iter().enumerate() {
        // Page size in PDF points (1 pt = 1/72 inch)
        let pt_w = *w as f32 * 72.0 / *dpi as f32;
        let pt_h = *h as f32 * 72.0 / *dpi as f32;
        let page_id = page_obj_ids[i];
        let image_id = image_obj_ids[i];

        // Content stream: place image filling the page
        let content = format!("q {pt_w:.2} 0 0 {pt_h:.2} 0 0 cm /Im{i} Do Q\n");

        // Compress image data
        let compressed = deflate_compress(rgb);
        let img_len = compressed.len();
        let rgb_len = rgb.len();
        let content_len = content.len();

        // Page object
        offsets.push(buf.len());
        buf.extend_from_slice(
            format!(
                "{page_id} 0 obj\n\
                 << /Type /Page /Parent 2 0 R\n\
                    /MediaBox [0 0 {pt_w:.2} {pt_h:.2}]\n\
                    /Contents {cs_id} 0 R\n\
                    /Resources << /XObject << /Im{i} {image_id} 0 R >> >> >>\n\
                 endobj\n",
                cs_id = image_id + 1, // content stream object right after image... wait, need to restructure
            )
            .as_bytes(),
        );

        // Actually let me restructure: use a separate content stream object.
        // Rewrite: page_id, content_id = page_id+1 (but that conflicts with image_id)
        // Let me use a different numbering scheme.
        let _ = (content, compressed, img_len, rgb_len, content_len);
    }

    // The above approach has a numbering conflict. Restart with clean numbering.
    drop(buf);
    build_pdf_clean(pages)
}

fn build_pdf_clean(pages: &[(u32, u32, u32, Vec<u8>)]) -> Vec<u8> {
    // Object layout per page (3 objects each):
    //  1         — Catalog
    //  2         — Pages
    //  3+i*3     — Page
    //  3+i*3+1   — Content stream
    //  3+i*3+2   — Image XObject
    let n = pages.len();
    let mut objects: Vec<(usize, Vec<u8>)> = Vec::new(); // (obj_id, bytes)

    let page_ids: Vec<usize> = (0..n).map(|i| 3 + i * 3).collect();
    let content_ids: Vec<usize> = (0..n).map(|i| 3 + i * 3 + 1).collect();
    let image_ids: Vec<usize> = (0..n).map(|i| 3 + i * 3 + 2).collect();

    // Catalog
    objects.push((1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()));

    // Pages
    let kids = page_ids
        .iter()
        .map(|id| format!("{id} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");
    objects.push((
        2,
        format!("<< /Type /Pages /Kids [{kids}] /Count {n} >>").into_bytes(),
    ));

    for (i, (w, h, dpi, rgb)) in pages.iter().enumerate() {
        let pt_w = *w as f32 * 72.0 / *dpi as f32;
        let pt_h = *h as f32 * 72.0 / *dpi as f32;
        let page_id = page_ids[i];
        let content_id = content_ids[i];
        let image_id = image_ids[i];

        // Page
        objects.push((
            page_id,
            format!(
                "<< /Type /Page /Parent 2 0 R\n\
                   /MediaBox [0 0 {pt_w:.2} {pt_h:.2}]\n\
                   /Contents {content_id} 0 R\n\
                   /Resources << /XObject << /Im{i} {image_id} 0 R >> >> >>"
            )
            .into_bytes(),
        ));

        // Content stream
        let content = format!("q {pt_w:.2} 0 0 {pt_h:.2} 0 0 cm /Im{i} Do Q\n");
        let content_bytes = content.as_bytes();
        let cs_len = content_bytes.len();
        let mut cs_obj = format!("<< /Length {cs_len} >>\nstream\n").into_bytes();
        cs_obj.extend_from_slice(content_bytes);
        cs_obj.extend_from_slice(b"\nendstream");
        objects.push((content_id, cs_obj));

        // Image XObject
        let compressed = deflate_compress(rgb);
        let img_len = compressed.len();
        let mut img_obj = format!(
            "<< /Type /XObject /Subtype /Image\n\
               /Width {w} /Height {h}\n\
               /ColorSpace /DeviceRGB /BitsPerComponent 8\n\
               /Filter /FlateDecode /Length {img_len} >>\nstream\n"
        )
        .into_bytes();
        img_obj.extend_from_slice(&compressed);
        img_obj.extend_from_slice(b"\nendstream");
        objects.push((image_id, img_obj));
    }

    // Serialize
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");

    let mut offsets: Vec<(usize, usize)> = Vec::new(); // (obj_id, byte_offset)
    for (obj_id, body) in &objects {
        offsets.push((*obj_id, buf.len()));
        buf.extend_from_slice(format!("{obj_id} 0 obj\n").as_bytes());
        buf.extend_from_slice(body);
        buf.extend_from_slice(b"\nendobj\n");
    }

    // Cross-reference table
    let xref_offset = buf.len();
    let total_objects = 3 + n * 3; // 1 catalog + 1 pages + 3 per page
    buf.extend_from_slice(format!("xref\n0 {}\n", total_objects + 1).as_bytes());
    buf.extend_from_slice(b"0000000000 65535 f \n");

    let mut offset_map = vec![None; total_objects + 1];
    for (obj_id, off) in &offsets {
        if *obj_id <= total_objects {
            offset_map[*obj_id] = Some(*off);
        }
    }
    for entry in offset_map.iter().skip(1) {
        match entry {
            Some(off) => buf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes()),
            None => buf.extend_from_slice(b"0000000000 65535 f \n"),
        }
    }

    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            total_objects + 1,
            xref_offset
        )
        .as_bytes(),
    );

    buf
}

/// Compress bytes using zlib/deflate (FlateDecode in PDF terms).
fn deflate_compress(data: &[u8]) -> Vec<u8> {
    use miniz_oxide::deflate::compress_to_vec_zlib;
    compress_to_vec_zlib(data, 6)
}

// ── PNG helpers ───────────────────────────────────────────────────────────────

fn render_page_png(
    doc: &Document,
    idx: usize,
    dpi: u32,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let page = doc.page(idx)?;
    let native_dpi = page.dpi() as f32;
    let scale = dpi as f32 / native_dpi;
    let w = ((page.width() as f32 * scale).round() as u32).max(1);
    let h = ((page.height() as f32 * scale).round() as u32).max(1);
    let pixmap = page.render_to_size(w, h)?;
    let file = std::fs::File::create(out)?;
    let mut writer = std::io::BufWriter::new(file);
    encode_png(&mut writer, pixmap.width, pixmap.height, &pixmap.data)?;
    Ok(())
}

fn encode_png(
    out: &mut impl std::io::Write,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut encoder = png::Encoder::new(out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(rgba)?;
    Ok(())
}

// ── text ──────────────────────────────────────────────────────────────────────

fn cmd_text(path: &Path, page: usize, all: bool) -> Result<(), Box<dyn std::error::Error>> {
    let doc = open(path)?;
    let count = doc.page_count();

    if all {
        for i in 0..count {
            println!("--- Page {} ---", i + 1);
            print_page_text(&doc, i)?;
        }
    } else {
        let idx = page_idx(page, count)?;
        print_page_text(&doc, idx)?;
    }
    Ok(())
}

fn print_page_text(doc: &Document, idx: usize) -> Result<(), Box<dyn std::error::Error>> {
    let page = doc.page(idx)?;
    match page.text()? {
        Some(text) if !text.trim().is_empty() => print!("{text}"),
        _ => println!("No text layer"),
    }
    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn open(path: &Path) -> Result<Document, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err(format!("{}: no such file", path.display()).into());
    }
    let data = std::fs::read(path)?;
    let doc = Document::from_bytes(data).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(doc)
}

/// Convert 1-based user page number to 0-based index, with bounds check.
fn page_idx(page: usize, count: usize) -> Result<usize, Box<dyn std::error::Error>> {
    if page == 0 || page > count {
        return Err(format!("page {page} out of range (document has {count} pages)").into());
    }
    Ok(page - 1)
}
