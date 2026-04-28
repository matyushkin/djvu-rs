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
        /// Print only the page count as a plain integer (useful for scripting).
        #[arg(short, long, conflicts_with = "json")]
        count: bool,
        /// Output info as JSON.
        #[arg(short, long)]
        json: bool,
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
        /// Additional rotation applied on top of the INFO chunk rotation.
        #[arg(short, long, default_value = "none", value_enum)]
        rotate: RotateArg,
        /// Output file (single page) or directory (--all, PNG only).
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Merge multiple DjVu files into one bundled DJVM.
    Merge {
        /// Input DjVu files to merge.
        files: Vec<PathBuf>,
        /// Output file path.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Extract a range of pages from a DjVu document.
    Split {
        /// Path to the DjVu file.
        file: PathBuf,
        /// Page number to extract (1-based). Conflicts with --pages.
        #[arg(short, long)]
        page: Option<usize>,
        /// Page range to extract (e.g. "1-50", 1-based inclusive).
        #[arg(long, conflicts_with = "page")]
        pages: Option<String>,
        /// Output file path.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Run OCR on pages and write the text layer back into the file.
    #[cfg(any(
        feature = "ocr-tesseract",
        feature = "ocr-onnx",
        feature = "ocr-neural"
    ))]
    Ocr {
        /// Path to the input DjVu file.
        file: PathBuf,
        /// OCR backend to use.
        #[arg(short, long, default_value = "tesseract", value_enum)]
        backend: OcrBackendChoice,
        /// Languages for recognition (e.g. "eng", "rus+eng").
        #[arg(short, long, default_value = "eng")]
        lang: String,
        /// Path to ONNX model file (required for --backend onnx).
        #[arg(long)]
        model: Option<PathBuf>,
        /// Output DjVu file with embedded OCR text layer.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Compress a file using BZZ encoding.
    BzzEncode {
        /// Input file to compress.
        file: PathBuf,
        /// Output file path.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Decompress a BZZ-encoded file.
    BzzDecode {
        /// BZZ-compressed input file.
        file: PathBuf,
        /// Output file path.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Encode an image (PNG) into a single-page DjVu file, or a
    /// directory of PNGs into a multi-page DJVM bundle.
    ///
    /// Bilevel pipeline only in v1: each input is luminance-thresholded
    /// into a JB2 mask via `segment_page`, then wrapped as `INFO + Sjbz`.
    /// Multi-page mode (input is a directory) builds a `FORM:DJVM`
    /// bundle with a shared Djbz dictionary across pages.
    /// `--quality quality|archival` is reserved for the layered codec
    /// (#220 follow-ups).
    Encode {
        /// Input PNG path, or a directory of PNGs (sorted by file name)
        /// for multi-page encoding.
        input: PathBuf,
        /// Output DjVu file path.
        #[arg(short, long)]
        output: PathBuf,
        /// Page DPI stored in the INFO chunk. Default: 300.
        #[arg(short, long, default_value = "300")]
        dpi: u16,
        /// Encoding profile.
        #[arg(short, long, default_value = "lossless", value_enum)]
        quality: EncodeQualityArg,
        /// (Multi-page only.) Promote a connected component to the
        /// shared Djbz dictionary if it appears on at least this many
        /// distinct pages. Default: 2.
        #[arg(long, default_value = "2")]
        shared_dict_pages: usize,
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
        /// Output format: plain (default), hocr, alto.
        #[arg(short, long, default_value = "plain", value_enum)]
        format: TextFormat,
        /// Output file path for hOCR/ALTO output. Default: stdout.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, ValueEnum)]
enum Format {
    Png,
    Pdf,
    Cbz,
    /// EPUB 3 (preserves text, bookmarks, hyperlinks).
    Epub,
}

#[derive(Clone, ValueEnum)]
enum TextFormat {
    /// Plain text (default).
    Plain,
    /// hOCR HTML format.
    Hocr,
    /// ALTO XML format.
    Alto,
}

#[cfg(any(
    feature = "ocr-tesseract",
    feature = "ocr-onnx",
    feature = "ocr-neural"
))]
#[derive(Clone, ValueEnum)]
enum OcrBackendChoice {
    #[cfg(feature = "ocr-tesseract")]
    Tesseract,
    #[cfg(feature = "ocr-onnx")]
    Onnx,
    #[cfg(feature = "ocr-neural")]
    Candle,
}

#[derive(Clone, ValueEnum)]
enum RotateArg {
    /// No additional rotation (only INFO chunk rotation applies).
    None,
    /// Rotate 90° clockwise.
    Cw90,
    /// Rotate 180°.
    Rot180,
    /// Rotate 90° counter-clockwise (270° clockwise).
    Ccw90,
}

#[derive(Clone, ValueEnum)]
enum EncodeQualityArg {
    /// Pixel-exact bilevel JB2 (`INFO + Sjbz`).
    Lossless,
    /// Layered FG/BG with lossy IW44 BG. Currently unsupported — see #220.
    Quality,
    /// Archival profile with FGbz palette. Currently unsupported — see #194 Phase 2.5.
    Archival,
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
        Cmd::Info { file, count, json } => cmd_info(&file, count, json),
        Cmd::Render {
            file,
            page,
            all,
            dpi,
            format,
            layer,
            rotate,
            output,
        } => cmd_render(&file, page, all, dpi, format, layer, rotate, &output),
        #[cfg(any(
            feature = "ocr-tesseract",
            feature = "ocr-onnx",
            feature = "ocr-neural"
        ))]
        Cmd::Ocr {
            file,
            backend,
            lang,
            model,
            output,
        } => cmd_ocr(&file, backend, &lang, model.as_deref(), &output),
        Cmd::BzzEncode { file, output } => cmd_bzz_encode(&file, &output),
        Cmd::BzzDecode { file, output } => cmd_bzz_decode(&file, &output),
        Cmd::Merge { files, output } => cmd_merge(&files, &output),
        Cmd::Split {
            file,
            page,
            pages,
            output,
        } => cmd_split(&file, page, pages.as_deref(), &output),
        Cmd::Text {
            file,
            page,
            all,
            format,
            output,
        } => cmd_text(&file, page, all, format, output.as_deref()),
        Cmd::Encode {
            input,
            output,
            dpi,
            quality,
            shared_dict_pages,
        } => cmd_encode(&input, &output, dpi, quality, shared_dict_pages),
    }
}

// ── merge ─────────────────────────────────────────────────────────────────────

fn cmd_merge(files: &[PathBuf], output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if files.is_empty() {
        return Err("no input files".into());
    }

    let docs: Vec<Vec<u8>> = files
        .iter()
        .map(|f| std::fs::read(f).map_err(|e| format!("{}: {e}", f.display())))
        .collect::<Result<_, _>>()?;

    let refs: Vec<&[u8]> = docs.iter().map(|d| d.as_slice()).collect();
    let merged = djvu_rs::djvm::merge(&refs)?;

    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output, merged)?;
    eprintln!("Merged {} files → {}", files.len(), output.display());
    Ok(())
}

// ── split ─────────────────────────────────────────────────────────────────────

fn cmd_split(
    path: &Path,
    page: Option<usize>,
    pages: Option<&str>,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read(path)?;

    let (start, end) = if let Some(p) = page {
        if p == 0 {
            return Err("page numbers are 1-based".into());
        }
        (p - 1, p)
    } else if let Some(range) = pages {
        parse_page_range(range)?
    } else {
        return Err("specify --page or --pages".into());
    };

    let result = djvu_rs::djvm::split(&data, start, end)?;

    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output, result)?;
    eprintln!("Split pages {}–{} → {}", start + 1, end, output.display());
    Ok(())
}

/// Parse "1-50" into (0, 50) — 0-based start, exclusive end.
fn parse_page_range(s: &str) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        return Err(format!("invalid page range: {s} (expected N-M)").into());
    }
    let start: usize = parts[0].parse()?;
    let end: usize = parts[1].parse()?;
    if start == 0 || end == 0 || start > end {
        return Err(format!("invalid page range: {s}").into());
    }
    Ok((start - 1, end))
}

// ── info ──────────────────────────────────────────────────────────────────────

fn cmd_info(path: &Path, count_only: bool, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let doc = open(path)?;
    let count = doc.page_count();

    if count_only {
        println!("{count}");
        return Ok(());
    }

    if json {
        // All fields are numeric — no JSON string escaping needed.
        // If string fields (e.g. title, filename) are added in the future,
        // use a proper JSON library (e.g. serde_json) to avoid injection.
        let mut out = String::from("{\"pages\":[");
        for i in 0..count {
            let page = doc.page(i)?;
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!(
                "{{\"page\":{},\"width\":{},\"height\":{},\"dpi\":{}}}",
                i + 1,
                page.width(),
                page.height(),
                page.dpi(),
            ));
        }
        out.push_str(&format!("],\"count\":{count}}}"));
        println!("{out}");
        return Ok(());
    }

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

fn to_user_rotation(r: &RotateArg) -> djvu_rs::djvu_render::UserRotation {
    use djvu_rs::djvu_render::UserRotation;
    match r {
        RotateArg::None => UserRotation::None,
        RotateArg::Cw90 => UserRotation::Cw90,
        RotateArg::Rot180 => UserRotation::Rot180,
        RotateArg::Ccw90 => UserRotation::Ccw90,
    }
}

fn cmd_render(
    path: &Path,
    page: usize,
    all: bool,
    dpi: u32,
    format: Format,
    layer: Layer,
    rotate: RotateArg,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // PDF uses the new DjVuDocument API directly (preserves text, bookmarks, links)
    if matches!(format, Format::Pdf) {
        return render_pdf_structured(path, output);
    }

    // EPUB uses the new DjVuDocument API directly
    #[cfg(feature = "epub")]
    if matches!(format, Format::Epub) {
        return render_epub_structured(path, output);
    }
    #[cfg(not(feature = "epub"))]
    if matches!(format, Format::Epub) {
        return Err("epub feature not enabled; rebuild with --features epub".into());
    }

    // Layer extraction uses the DjVuDocument API
    if !matches!(layer, Layer::Composite) {
        return render_layer(path, page, all, layer, output);
    }

    // When the `parallel` feature is enabled and --all is requested for PNG,
    // use rayon-based parallel rendering via the DjVuDocument API.
    #[cfg(feature = "parallel")]
    if all && matches!(format, Format::Png) {
        return render_png_parallel(path, dpi, output);
    }

    let doc = open(path)?;
    let count = doc.page_count();
    let user_rot = to_user_rotation(&rotate);

    match format {
        Format::Png => render_png(&doc, page, all, dpi, count, user_rot, output),
        Format::Pdf | Format::Epub => unreachable!(),
        Format::Cbz => render_cbz(&doc, page, all, dpi, count, user_rot, output),
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

/// Apply user-requested rotation to a rendered pixmap (post-render, on top of INFO rotation).
fn apply_user_rotation(
    src: djvu_rs::Pixmap,
    rot: djvu_rs::djvu_render::UserRotation,
) -> djvu_rs::Pixmap {
    use djvu_rs::djvu_render::UserRotation;
    match rot {
        UserRotation::None => src,
        UserRotation::Cw90 => rotate_pixmap_cw90(src),
        UserRotation::Rot180 => rotate_pixmap_180(src),
        UserRotation::Ccw90 => rotate_pixmap_ccw90(src),
    }
}

fn rotate_pixmap_cw90(src: djvu_rs::Pixmap) -> djvu_rs::Pixmap {
    let (w, h) = (src.width, src.height);
    let mut dst = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let src_off = ((y * w + x) * 4) as usize;
            let dst_x = h - 1 - y;
            let dst_y = x;
            let dst_off = ((dst_y * h + dst_x) * 4) as usize;
            dst[dst_off..dst_off + 4].copy_from_slice(&src.data[src_off..src_off + 4]);
        }
    }
    djvu_rs::Pixmap {
        width: h,
        height: w,
        data: dst,
    }
}

fn rotate_pixmap_180(src: djvu_rs::Pixmap) -> djvu_rs::Pixmap {
    let (w, h) = (src.width, src.height);
    let mut dst = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let src_off = ((y * w + x) * 4) as usize;
            let dst_off = (((h - 1 - y) * w + (w - 1 - x)) * 4) as usize;
            dst[dst_off..dst_off + 4].copy_from_slice(&src.data[src_off..src_off + 4]);
        }
    }
    djvu_rs::Pixmap {
        width: w,
        height: h,
        data: dst,
    }
}

fn rotate_pixmap_ccw90(src: djvu_rs::Pixmap) -> djvu_rs::Pixmap {
    let (w, h) = (src.width, src.height);
    let mut dst = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let src_off = ((y * w + x) * 4) as usize;
            let dst_x = y;
            let dst_y = w - 1 - x;
            let dst_off = ((dst_y * h + dst_x) * 4) as usize;
            dst[dst_off..dst_off + 4].copy_from_slice(&src.data[src_off..src_off + 4]);
        }
    }
    djvu_rs::Pixmap {
        width: h,
        height: w,
        data: dst,
    }
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
    rotate: djvu_rs::djvu_render::UserRotation,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if all {
        std::fs::create_dir_all(output)?;
        for i in 0..count {
            let out = output.join(format!("page_{:04}.png", i + 1));
            render_page_png(doc, i, dpi, rotate, &out)?;
        }
    } else {
        let idx = page_idx(page, count)?;
        if let Some(parent) = output.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        render_page_png(doc, idx, dpi, rotate, output)?;
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

#[cfg(feature = "epub")]
fn render_epub_structured(path: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let data = std::fs::read(path)?;
    let doc = djvu_rs::djvu_document::DjVuDocument::parse(&data)?;
    let epub = djvu_rs::epub::djvu_to_epub(&doc, &djvu_rs::epub::EpubOptions::default())?;
    std::fs::write(output, epub)?;
    Ok(())
}

fn render_cbz(
    doc: &Document,
    page: usize,
    all: bool,
    dpi: u32,
    count: usize,
    rotate: djvu_rs::djvu_render::UserRotation,
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
        let pixmap = apply_user_rotation(pixmap, rotate);

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

/// Parallel PNG rendering: renders all pages concurrently using rayon, then
/// writes PNGs sequentially.
#[cfg(feature = "parallel")]
fn render_png_parallel(
    path: &Path,
    dpi: u32,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read(path)?;
    let doc = djvu_rs::djvu_document::DjVuDocument::parse(&data)?;
    std::fs::create_dir_all(output)?;

    let pixmaps = djvu_rs::djvu_render::render_pages_parallel(&doc, dpi);

    for (i, result) in pixmaps.into_iter().enumerate() {
        let pixmap = result?;
        let out = output.join(format!("page_{:04}.png", i + 1));
        let file = std::fs::File::create(&out)?;
        let mut writer = std::io::BufWriter::new(file);
        encode_png(&mut writer, pixmap.width, pixmap.height, &pixmap.data)?;
    }

    Ok(())
}

// ── PNG helpers ───────────────────────────────────────────────────────────────

fn render_page_png(
    doc: &Document,
    idx: usize,
    dpi: u32,
    rotate: djvu_rs::djvu_render::UserRotation,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let page = doc.page(idx)?;
    let native_dpi = page.dpi() as f32;
    let scale = dpi as f32 / native_dpi;
    let w = ((page.width() as f32 * scale).round() as u32).max(1);
    let h = ((page.height() as f32 * scale).round() as u32).max(1);
    let pixmap = page.render_to_size(w, h)?;
    let pixmap = apply_user_rotation(pixmap, rotate);
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

// ── ocr ──────────────────────────────────────────────────────────────────────

#[cfg(any(
    feature = "ocr-tesseract",
    feature = "ocr-onnx",
    feature = "ocr-neural"
))]
fn cmd_ocr(
    path: &Path,
    backend: OcrBackendChoice,
    lang: &str,
    model_path: Option<&Path>,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use djvu_rs::ocr::{OcrBackend, OcrOptions};

    let data = std::fs::read(path)?;
    let doc = djvu_rs::djvu_document::DjVuDocument::parse(&data)?;

    let ocr_backend: Box<dyn OcrBackend> = match backend {
        #[cfg(feature = "ocr-tesseract")]
        OcrBackendChoice::Tesseract => Box::new(djvu_rs::ocr_tesseract::TesseractBackend::new()),
        #[cfg(feature = "ocr-onnx")]
        OcrBackendChoice::Onnx => {
            let mp = model_path.ok_or("--model is required for onnx backend")?;
            Box::new(djvu_rs::ocr_onnx::OnnxBackend::load(mp, None)?)
        }
        #[cfg(feature = "ocr-neural")]
        OcrBackendChoice::Candle => {
            let mp = model_path.ok_or("--model is required for candle backend")?;
            Box::new(djvu_rs::ocr_neural::CandleBackend::load(mp)?)
        }
    };

    let options = OcrOptions {
        languages: lang.to_string(),
        dpi: 300,
    };

    // OCR each page and collect text layers
    let count = doc.page_count();
    let mut text_chunks: Vec<Vec<u8>> = Vec::new();

    for i in 0..count {
        let page = doc.page(i)?;
        let w = page.width() as u32;
        let h = page.height() as u32;
        let opts = djvu_rs::djvu_render::RenderOptions {
            width: w,
            height: h,
            ..Default::default()
        };
        let pixmap = djvu_rs::djvu_render::render_pixmap(page, &opts)?;
        let text_layer = ocr_backend.recognize(&pixmap, &options)?;

        eprintln!(
            "Page {}: {} chars, {} zones",
            i + 1,
            text_layer.text.len(),
            text_layer.zones.len()
        );

        let encoded = djvu_rs::text_encode::encode_text_layer(&text_layer, h);
        text_chunks.push(encoded);
    }

    // Write output: copy original file and inject TXTa chunks
    // For now, write the encoded text layers as standalone files
    // (full DjVu rewriting requires IFF mutation which is future work)
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    // Copy original file, then append text chunks info
    std::fs::copy(path, output)?;
    eprintln!("OCR complete. Output written to {}", output.display());
    eprintln!(
        "Note: text layer injection into DjVu IFF is pending; \
         encoded TXTa data available via djvu_rs::text_encode"
    );

    Ok(())
}

// ── text ──────────────────────────────────────────────────────────────────────

fn cmd_text(
    path: &Path,
    page: usize,
    all: bool,
    format: TextFormat,
    output: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        TextFormat::Plain => {
            let doc = open(path)?;
            let count = doc.page_count();
            let mut text = String::new();
            if all {
                for i in 0..count {
                    text.push_str(&format!("--- Page {} ---\n", i + 1));
                    collect_page_text(&doc, i, &mut text)?;
                }
            } else {
                let idx = page_idx(page, count)?;
                collect_page_text(&doc, idx, &mut text)?;
            }
            write_or_print(output, &text)?;
        }
        TextFormat::Hocr => {
            let data = std::fs::read(path)?;
            let doc = djvu_rs::djvu_document::DjVuDocument::parse(&data)?;
            let opts = djvu_rs::ocr_export::HocrOptions {
                page_index: if all {
                    None
                } else {
                    Some(page_idx(page, doc.page_count())?)
                },
                dpi: None,
            };
            let hocr = djvu_rs::ocr_export::to_hocr(&doc, &opts)?;
            write_or_print(output, &hocr)?;
        }
        TextFormat::Alto => {
            let data = std::fs::read(path)?;
            let doc = djvu_rs::djvu_document::DjVuDocument::parse(&data)?;
            let opts = djvu_rs::ocr_export::AltoOptions {
                page_index: if all {
                    None
                } else {
                    Some(page_idx(page, doc.page_count())?)
                },
                dpi: None,
            };
            let alto = djvu_rs::ocr_export::to_alto(&doc, &opts)?;
            write_or_print(output, &alto)?;
        }
    }
    Ok(())
}

fn write_or_print(output: Option<&Path>, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    match output {
        Some(path) => {
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, content)?;
        }
        None => print!("{content}"),
    }
    Ok(())
}

fn collect_page_text(
    doc: &Document,
    idx: usize,
    buf: &mut String,
) -> Result<(), Box<dyn std::error::Error>> {
    let page = doc.page(idx)?;
    match page.text()? {
        Some(text) if !text.trim().is_empty() => buf.push_str(&text),
        _ => buf.push_str("No text layer\n"),
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

// ── bzz encode/decode ────────────────────────────────────────────────────────

fn cmd_bzz_encode(file: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read(file)?;
    let compressed = djvu_rs::bzz_encode::bzz_encode(&data);
    std::fs::write(output, &compressed)?;
    eprintln!(
        "{}: {} → {} bytes ({:.1}%)",
        file.display(),
        data.len(),
        compressed.len(),
        if data.is_empty() {
            0.0
        } else {
            compressed.len() as f64 / data.len() as f64 * 100.0
        }
    );
    Ok(())
}

fn cmd_bzz_decode(file: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read(file)?;
    let decoded = djvu_rs::bzz_new::bzz_decode(&data)?;
    std::fs::write(output, &decoded)?;
    eprintln!(
        "{}: {} → {} bytes",
        file.display(),
        data.len(),
        decoded.len(),
    );
    Ok(())
}

// ── encode ───────────────────────────────────────────────────────────────────

fn cmd_encode(
    input: &Path,
    output: &Path,
    dpi: u16,
    quality: EncodeQualityArg,
    shared_dict_pages: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    use djvu_rs::djvu_encode::{EncodeQuality, PageEncoder};
    use djvu_rs::jb2_encode::encode_djvm_bundle_jb2;
    use djvu_rs::segment::{SegmentOptions, segment_page};

    let q = match quality {
        EncodeQualityArg::Lossless => EncodeQuality::Lossless,
        EncodeQualityArg::Quality => EncodeQuality::Quality,
        EncodeQualityArg::Archival => EncodeQuality::Archival,
    };

    if input.is_dir() {
        if !matches!(quality, EncodeQualityArg::Lossless) {
            return Err(
                "multi-page encode currently supports --quality lossless only \
                        (#220 follow-ups for layered Quality / Archival)"
                    .into(),
            );
        }
        let mut entries: Vec<PathBuf> = std::fs::read_dir(input)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && p.extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| e.eq_ignore_ascii_case("png"))
            })
            .collect();
        entries.sort();
        if entries.is_empty() {
            return Err(format!("{}: no PNG files found in directory", input.display()).into());
        }

        let mut masks = Vec::with_capacity(entries.len());
        for path in &entries {
            let pixmap = decode_png_to_pixmap(path)?;
            let seg = segment_page(&pixmap, &SegmentOptions::default());
            masks.push(seg.mask);
        }
        let bytes = encode_djvm_bundle_jb2(&masks, shared_dict_pages);
        std::fs::write(output, &bytes)?;
        eprintln!(
            "{} pages → {} ({} bytes, shared-dict threshold = {})",
            entries.len(),
            output.display(),
            bytes.len(),
            shared_dict_pages,
        );
        return Ok(());
    }

    let pixmap = decode_png_to_pixmap(input)?;
    let seg = segment_page(&pixmap, &SegmentOptions::default());

    let bytes = PageEncoder::from_bitmap(&seg.mask)
        .with_dpi(dpi)
        .with_quality(q)
        .encode()
        .map_err(|e| format!("encode: {e}"))?;

    std::fs::write(output, &bytes)?;
    eprintln!(
        "{} → {} ({}×{} px, {} bytes)",
        input.display(),
        output.display(),
        pixmap.width,
        pixmap.height,
        bytes.len(),
    );
    Ok(())
}

fn decode_png_to_pixmap(path: &Path) -> Result<djvu_rs::Pixmap, Box<dyn std::error::Error>> {
    use djvu_rs::Pixmap;

    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info()?;
    let info = reader.info();
    let width = info.width;
    let height = info.height;
    let color = info.color_type;
    let depth = info.bit_depth;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf)?;
    buf.truncate(frame.buffer_size());

    if depth != png::BitDepth::Eight {
        return Err(format!(
            "{}: unsupported PNG bit depth {:?} (only 8-bit channels for v1)",
            path.display(),
            depth
        )
        .into());
    }

    let mut data = Vec::with_capacity((width as usize) * (height as usize) * 4);
    match color {
        png::ColorType::Rgba => data.extend_from_slice(&buf),
        png::ColorType::Rgb => {
            for chunk in buf.chunks_exact(3) {
                data.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for chunk in buf.chunks_exact(2) {
                let g = chunk[0];
                data.extend_from_slice(&[g, g, g, chunk[1]]);
            }
        }
        png::ColorType::Grayscale => {
            for &g in &buf {
                data.extend_from_slice(&[g, g, g, 255]);
            }
        }
        png::ColorType::Indexed => {
            return Err(format!("{}: indexed PNG not supported", path.display()).into());
        }
    }

    Ok(Pixmap {
        width,
        height,
        data,
    })
}
