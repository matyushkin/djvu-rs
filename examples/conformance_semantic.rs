//! Semantic conformance comparison against DjVuLibre's `djvused`.
//!
//! Emits one JSONL record for text and annotations on every page, plus one
//! bookmarks record per document. Signatures deliberately ignore printer-only
//! formatting while retaining semantic content.

use std::path::Path;
use std::process::{Command, ExitCode};

use djvu_rs::DjVuDocument;
use djvu_rs::djvu_document::DjVuBookmark;
use serde_json::json;

fn text_signature(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn bookmarks_value(bookmarks: &[DjVuBookmark]) -> serde_json::Value {
    serde_json::Value::Array(
        bookmarks
            .iter()
            .map(|bookmark| {
                json!({
                    "title": text_signature(&bookmark.title),
                    "url": text_signature(&bookmark.url),
                    "children": bookmarks_value(&bookmark.children),
                })
            })
            .collect(),
    )
}

#[derive(Debug, PartialEq)]
enum OutlineToken {
    Left,
    Right,
    Atom(String),
    String(String),
}

/// Tokenize djvused S-expressions, including octal byte escapes.
fn outline_tokens(raw: &str) -> Result<Vec<OutlineToken>, String> {
    let bytes = raw.as_bytes();
    let mut output = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
            continue;
        }
        if bytes[cursor] == b'(' {
            output.push(OutlineToken::Left);
            cursor += 1;
            continue;
        }
        if bytes[cursor] == b')' {
            output.push(OutlineToken::Right);
            cursor += 1;
            continue;
        }
        if bytes[cursor] != b'"' {
            let start = cursor;
            while cursor < bytes.len()
                && !bytes[cursor].is_ascii_whitespace()
                && !matches!(bytes[cursor], b'(' | b')')
            {
                cursor += 1;
            }
            output.push(OutlineToken::Atom(
                String::from_utf8_lossy(&bytes[start..cursor]).into_owned(),
            ));
            continue;
        }
        cursor += 1;
        let mut value = Vec::new();
        while cursor < bytes.len() && bytes[cursor] != b'"' {
            if bytes[cursor] == b'\\' && cursor + 1 < bytes.len() {
                let escaped = bytes[cursor + 1];
                if escaped.is_ascii_digit() {
                    let mut octal = 0u32;
                    let mut count = 0;
                    while count < 3
                        && cursor + 1 + count < bytes.len()
                        && bytes[cursor + 1 + count].is_ascii_digit()
                    {
                        octal = octal * 8 + u32::from(bytes[cursor + 1 + count] - b'0');
                        count += 1;
                    }
                    value.push((octal & 0xff) as u8);
                    cursor += 1 + count;
                } else {
                    value.push(match escaped {
                        b'n' => b'\n',
                        b'r' => b'\r',
                        b't' => b'\t',
                        other => other,
                    });
                    cursor += 2;
                }
            } else {
                value.push(bytes[cursor]);
                cursor += 1;
            }
        }
        if cursor >= bytes.len() {
            return Err("unterminated string in djvused outline".into());
        }
        cursor += 1;
        output.push(OutlineToken::String(
            String::from_utf8_lossy(&value).into_owned(),
        ));
    }
    Ok(output)
}

fn parse_outline_node(
    tokens: &[OutlineToken],
    cursor: &mut usize,
) -> Result<serde_json::Value, String> {
    if tokens.get(*cursor) != Some(&OutlineToken::Left) {
        return Err("bookmark node must start with '('".into());
    }
    *cursor += 1;
    let OutlineToken::String(title) = tokens.get(*cursor).ok_or("missing bookmark title")? else {
        return Err("bookmark title must be a string".into());
    };
    *cursor += 1;
    let OutlineToken::String(url) = tokens.get(*cursor).ok_or("missing bookmark URL")? else {
        return Err("bookmark URL must be a string".into());
    };
    *cursor += 1;
    let mut children = Vec::new();
    while tokens.get(*cursor) == Some(&OutlineToken::Left) {
        children.push(parse_outline_node(tokens, cursor)?);
    }
    if tokens.get(*cursor) != Some(&OutlineToken::Right) {
        return Err("bookmark node missing ')'".into());
    }
    *cursor += 1;
    Ok(json!({
        "title": text_signature(title),
        "url": text_signature(url),
        "children": children,
    }))
}

fn djvused_bookmarks_value(raw: &str) -> Result<serde_json::Value, String> {
    if raw.trim().is_empty() {
        return Ok(json!([]));
    }
    let tokens = outline_tokens(raw)?;
    let mut cursor = 0;
    if tokens.get(cursor) != Some(&OutlineToken::Left)
        || tokens.get(cursor + 1) != Some(&OutlineToken::Atom("bookmarks".into()))
    {
        return Err("djvused outline missing bookmarks root".into());
    }
    cursor += 2;
    let mut bookmarks = Vec::new();
    while tokens.get(cursor) == Some(&OutlineToken::Left) {
        bookmarks.push(parse_outline_node(&tokens, &mut cursor)?);
    }
    if tokens.get(cursor) != Some(&OutlineToken::Right) || cursor + 1 != tokens.len() {
        return Err("djvused outline has malformed trailing tokens".into());
    }
    Ok(serde_json::Value::Array(bookmarks))
}

fn djvused(path: &Path, command: &str) -> Result<String, String> {
    let output = Command::new("djvused")
        .arg(path)
        .arg("-e")
        .arg(command)
        .output()
        .map_err(|error| format!("cannot run djvused: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "djvused failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn emit(file: &Path, page: usize, plane: &str, ours: String, theirs: String) -> bool {
    let matches = ours == theirs;
    println!(
        "{}",
        json!({
            "file": file.to_string_lossy(),
            "page": page,
            "plane": plane,
            "status": if matches { "match" } else { "diverge" },
            "ours": ours,
            "djvulibre": theirs,
        })
    );
    matches
}

fn process(path: &Path, max_pages: usize) -> Result<bool, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let document = DjVuDocument::parse(&bytes).map_err(|error| error.to_string())?;
    let mut all_match = true;

    let theirs_bookmarks = djvused_bookmarks_value(&djvused(path, "print-outline")?)?;
    all_match &= emit(
        path,
        0,
        "bookmarks",
        bookmarks_value(document.bookmarks()).to_string(),
        theirs_bookmarks.to_string(),
    );

    let page_count = if max_pages == 0 {
        document.page_count()
    } else {
        document.page_count().min(max_pages)
    };
    for page_index in 0..page_count {
        let page = document
            .page(page_index)
            .map_err(|error| error.to_string())?;
        let ours_text = page
            .text()
            .map_err(|error| error.to_string())?
            .map(|value| text_signature(&value))
            .unwrap_or_default();
        let theirs_text = text_signature(&djvused(
            path,
            &format!("select {}; print-pure-txt", page_index + 1),
        )?);
        all_match &= emit(path, page_index, "text", ours_text, theirs_text);

        let ours_areas = page
            .annotations()
            .map_err(|error| error.to_string())?
            .map(|(_, areas)| areas.len())
            .unwrap_or(0);
        let theirs_annotations = djvused(path, &format!("select {}; print-ant", page_index + 1))?;
        let theirs_areas = theirs_annotations.matches("(maparea").count();
        all_match &= emit(
            path,
            page_index,
            "annotations",
            ours_areas.to_string(),
            theirs_areas.to_string(),
        );
    }
    Ok(all_match)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut files = Vec::new();
    let mut max_pages = 0;
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--max-pages" {
            index += 1;
            max_pages = args
                .get(index)
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
        } else {
            files.push(args[index].clone());
        }
        index += 1;
    }
    if files.is_empty() {
        eprintln!("usage: conformance_semantic <file.djvu> [...]");
        return ExitCode::from(2);
    }
    let mut success = true;
    for file in files {
        match process(Path::new(&file), max_pages) {
            // Content divergences are valid JSONL results. The report builder
            // must see them so it can publish a FAIL dashboard and artifact.
            Ok(_) => {}
            Err(error) => {
                eprintln!("semantic conformance failed for {file}: {error}");
                success = false;
            }
        }
    }
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_signature_preserves_word_boundaries() {
        assert_eq!(text_signature(" a\n bc\t"), "a bc");
        assert_ne!(text_signature("a bc"), text_signature("ab c"));
    }

    #[test]
    fn outline_signature_preserves_tree_and_pairing() {
        let nested = r##"(bookmarks ("A" "#1" ("B" "#2")))"##;
        let siblings = r##"(bookmarks ("A" "#1") ("B" "#2"))"##;
        let swapped = r##"(bookmarks ("#1" "A" ("B" "#2")))"##;
        let value = djvused_bookmarks_value(nested).unwrap();
        assert_ne!(value, djvused_bookmarks_value(siblings).unwrap());
        assert_ne!(value, djvused_bookmarks_value(swapped).unwrap());
    }
}
