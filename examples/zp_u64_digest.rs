//! Byte-identity harness for the ZP_U64 experiment (perf round 51).
//!
//! Renders every page of every corpus document and prints a per-page,
//! per-file, and grand-total hash of the decoded pixel bytes. Run once on an
//! unmodified checkout to capture a baseline, then again after the ZP
//! bit-buffer change and diff the two outputs — any divergence means the
//! decode is no longer byte-exact.
//!
//! Usage: cargo run --release --example zp_u64_digest -- tests/corpus/*.djvu

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        // Default: every .djvu file under tests/corpus, sorted for determinism.
        let mut found = Vec::new();
        for entry in std::fs::read_dir("tests/corpus")? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("djvu") {
                found.push(path.to_string_lossy().into_owned());
            }
        }
        found.sort();
        paths = found;
    }

    let mut grand = DefaultHasher::new();
    let mut total_pages = 0usize;

    for path in &paths {
        let doc = djvu_rs::Document::open(path)?;
        let n = doc.page_count();
        let mut file_hasher = DefaultHasher::new();
        for i in 0..n {
            let page = doc.page(i)?;
            let pixmap = page.render()?;
            let mut page_hasher = DefaultHasher::new();
            pixmap.width.hash(&mut page_hasher);
            pixmap.height.hash(&mut page_hasher);
            pixmap.data.hash(&mut page_hasher);
            let digest = page_hasher.finish();
            println!(
                "{path}\tpage={i}\tw={}\th={}\tdigest={digest:016x}",
                pixmap.width, pixmap.height
            );
            digest.hash(&mut file_hasher);
            digest.hash(&mut grand);
            total_pages += 1;
        }
    }

    println!(
        "TOTAL\tfiles={}\tpages={total_pages}\tgrand_digest={:016x}",
        paths.len(),
        grand.finish()
    );
    Ok(())
}
