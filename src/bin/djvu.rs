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
        /// Layer to extract: composite (default), mask, foreground, background.
        #[arg(short, long, default_value = "composite", value_enum)]
        layer: Layer,
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

#[derive(Clone, ValueEnum)]
enum Layer {
    /// Full composite render (default).
    Composite,
    /// JB2 bilevel mask only.
    Mask,
    /// IW44 foreground layer only.
    Foreground,
    /// IW44 background layer only.
    Background,
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
            layer,
            output,
        } => cmd_render(&file, page, all, dpi, format, layer, &output),
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
    layer: Layer,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // PDF uses the new DjVuDocument API directly (preserves text, bookmarks, links)
    if matches!(format, Format::Pdf) {
        return render_pdf_structured(path, output);
    }

    // Layer extraction uses the DjVuDocument API
    if !matches!(layer, Layer::Composite) {
        return render_layer(path, page, all, layer, output);
    }

    let doc = open(path)?;
    let count = doc.page_count();

    match format {
        Format::Png => render_png(&doc, page, all, dpi, count, output),
        Format::Pdf => unreachable!(),
        Format::Cbz => render_cbz(&doc, page, all, dpi, count, output),
    }
}

fn render_layer(
    path: &Path,
    page: usize,
    all: bool,
    layer: Layer,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read(path)?;
    let doc = djvu_rs::djvu_document::DjVuDocument::parse(&data)?;
    let count = doc.page_count();

    let pages: Vec<usize> = if all {
        (0..count).collect()
    } else {
        vec![page_idx(page, count)?]
    };

    if all {
        std::fs::create_dir_all(output)?;
    } else if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    for idx in pages {
        let pg = doc.page(idx)?;
        let out_path = if all {
            output.join(format!("page_{:04}.png", idx + 1))
        } else {
            output.to_path_buf()
        };

        match layer {
            Layer::Mask => {
                let bm = pg.extract_mask()?.ok_or("page has no JB2 mask layer")?;
                // Convert 1-bit bitmap to RGBA (black/white)
                let w = bm.width;
                let h = bm.height;
                let mut rgba = vec![255u8; (w * h * 4) as usize];
                for y in 0..h {
                    for x in 0..w {
                        if bm.get(x, y) {
                            let off = ((y * w + x) * 4) as usize;
                            rgba[off] = 0;
                            rgba[off + 1] = 0;
                            rgba[off + 2] = 0;
                        }
                    }
                }
                let file = std::fs::File::create(&out_path)?;
                let mut writer = std::io::BufWriter::new(file);
                encode_png(&mut writer, w, h, &rgba)?;
            }
            Layer::Foreground => {
                let pm = pg
                    .extract_foreground()?
                    .ok_or("page has no foreground layer")?;
                let rgba = pixmap_to_rgba(&pm);
                let file = std::fs::File::create(&out_path)?;
                let mut writer = std::io::BufWriter::new(file);
                encode_png(&mut writer, pm.width, pm.height, &rgba)?;
            }
            Layer::Background => {
                let pm = pg
                    .extract_background()?
                    .ok_or("page has no background layer")?;
                let rgba = pixmap_to_rgba(&pm);
                let file = std::fs::File::create(&out_path)?;
                let mut writer = std::io::BufWriter::new(file);
                encode_png(&mut writer, pm.width, pm.height, &rgba)?;
            }
            Layer::Composite => unreachable!(),
        }
    }
    Ok(())
}

/// Convert an RGB Pixmap to RGBA bytes.
fn pixmap_to_rgba(pm: &djvu_rs::Pixmap) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((pm.width * pm.height * 4) as usize);
    for y in 0..pm.height {
        for x in 0..pm.width {
            let (r, g, b) = pm.get_rgb(x, y);
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    rgba
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

fn render_pdf_structured(path: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let data = std::fs::read(path)?;
    let doc = djvu_rs::djvu_document::DjVuDocument::parse(&data)?;
    let pdf = djvu_rs::pdf::djvu_to_pdf(&doc)?;
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
