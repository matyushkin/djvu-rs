use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
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
    /// Render pages to PNG images.
    Render {
        /// Path to the DjVu file.
        file: PathBuf,
        /// Page number to render (1-based). Default: 1.
        #[arg(short, long, default_value = "1")]
        page: usize,
        /// Render all pages. Output must be a directory.
        #[arg(long, conflicts_with = "page")]
        all: bool,
        /// Output DPI. Default: 150.
        #[arg(short, long, default_value = "150")]
        dpi: u32,
        /// Output file (single page) or directory (--all).
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
            output,
        } => cmd_render(&file, page, all, dpi, &output),
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
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let doc = open(path)?;
    let count = doc.page_count();

    if all {
        // output must be (or will become) a directory
        std::fs::create_dir_all(output)?;
        for i in 0..count {
            let out = output.join(format!("page_{:04}.png", i + 1));
            render_page(&doc, i, dpi, &out)?;
        }
    } else {
        let idx = page_idx(page, count)?;
        if let Some(parent) = output.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        render_page(&doc, idx, dpi, output)?;
    }
    Ok(())
}

fn render_page(
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
    write_png(out, pixmap.width, pixmap.height, &pixmap.data)?;
    Ok(())
}

fn write_png(
    path: &Path,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(path)?;
    let ref mut w = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, width, height);
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
