//! DjVu document metadata parser — phase 4 extension.
//!
//! Parses METa (plain text) and METz (BZZ-compressed) metadata chunks into a
//! structured [`DjVuMetadata`] value.
//!
//! ## Key public types
//!
//! - [`DjVuMetadata`] — key-value metadata extracted from a DjVu document
//! - [`MetadataError`] — typed errors from this module
//!
//! ## Format notes
//!
//! METa/METz encode metadata as an S-expression:
//!
//! ```text
//! (metadata
//!   (author "Author Name")
//!   (title "Book Title")
//!   (subject "Subject")
//!   (year "2023")
//!   (keywords "keyword1, keyword2")
//! )
//! ```
//!
//! This module accepts arbitrary key names; well-known keys populate dedicated
//! fields while anything else goes into [`DjVuMetadata::extra`].

#[cfg(not(feature = "std"))]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{bzz_new::bzz_decode, error::BzzError};

// ---- Error ------------------------------------------------------------------

/// Errors from metadata parsing.
#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    /// BZZ decompression failed.
    #[error("bzz decode failed: {0}")]
    Bzz(#[from] BzzError),

    /// The chunk is not valid UTF-8.
    #[error("metadata chunk is not valid UTF-8")]
    InvalidUtf8,
}

// ---- Public types -----------------------------------------------------------

/// Key-value metadata extracted from a DjVu document's METa/METz chunk.
///
/// Well-known keys populate dedicated fields; everything else is in
/// [`DjVuMetadata::extra`].  All values are plain strings — the DjVu format
/// does not define structured types beyond that.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DjVuMetadata {
    /// Document title.
    pub title: Option<String>,
    /// Author name(s).
    pub author: Option<String>,
    /// Subject or description.
    pub subject: Option<String>,
    /// Publisher name.
    pub publisher: Option<String>,
    /// Publication year.
    pub year: Option<String>,
    /// Comma-separated keywords (raw string as stored).
    pub keywords: Option<String>,
    /// All other key-value pairs, in document order.
    pub extra: Vec<(String, String)>,
}

// ---- Entry points -----------------------------------------------------------

/// Parse a METa (uncompressed) metadata chunk.
///
/// `data` is the raw bytes of the METa chunk (not including the 4-byte chunk
/// ID or the 4-byte length prefix — just the payload).
pub fn parse_metadata(data: &[u8]) -> Result<DjVuMetadata, MetadataError> {
    let text = core::str::from_utf8(data).map_err(|_| MetadataError::InvalidUtf8)?;
    Ok(parse_metadata_text(text))
}

/// Parse a METz (BZZ-compressed) metadata chunk.
///
/// Decompresses with BZZ first, then delegates to [`parse_metadata`].
pub fn parse_metadata_bzz(data: &[u8]) -> Result<DjVuMetadata, MetadataError> {
    let decoded = bzz_decode(data)?;
    parse_metadata(&decoded)
}

// ---- Internal parsing -------------------------------------------------------

fn parse_metadata_text(text: &str) -> DjVuMetadata {
    let tokens = tokenize(text);
    let sexprs = parse_sexprs(&tokens);

    let mut meta = DjVuMetadata::default();

    // Look for a top-level (metadata ...) list
    for expr in &sexprs {
        if let SExpr::List(items) = expr
            && let Some(SExpr::Atom(head)) = items.first()
        {
            if !head.eq_ignore_ascii_case("metadata") {
                continue;
            }
            for item in &items[1..] {
                if let SExpr::List(pair) = item
                    && let (Some(SExpr::Atom(key)), Some(SExpr::Atom(val))) =
                        (pair.first(), pair.get(1))
                {
                    store_kv(&mut meta, key, val);
                }
            }
        }
    }

    meta
}

fn store_kv(meta: &mut DjVuMetadata, key: &str, value: &str) {
    match key.to_lowercase().as_str() {
        "title" => meta.title = Some(value.to_string()),
        "author" => meta.author = Some(value.to_string()),
        "subject" | "description" => meta.subject = Some(value.to_string()),
        "publisher" => meta.publisher = Some(value.to_string()),
        "year" | "date" => meta.year = Some(value.to_string()),
        "keywords" | "keyword" => meta.keywords = Some(value.to_string()),
        _ => meta.extra.push((key.to_string(), value.to_string())),
    }
}

// ---- Minimal S-expression tokenizer/parser ----------------------------------
//
// A self-contained subset that handles the metadata format.
// Supports atoms (unquoted), quoted strings, and nested lists.

#[derive(Debug)]
enum Token<'a> {
    LParen,
    RParen,
    Atom(&'a str),
    Quoted(String),
}

fn tokenize(input: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        match bytes.get(i) {
            Some(b'(') => {
                tokens.push(Token::LParen);
                i += 1;
            }
            Some(b')') => {
                tokens.push(Token::RParen);
                i += 1;
            }
            Some(b'"') => {
                i += 1;
                let mut s = String::new();
                while i < bytes.len() {
                    match bytes.get(i) {
                        Some(b'\\') if i + 1 < bytes.len() => {
                            i += 1;
                            if let Some(&c) = bytes.get(i) {
                                s.push(c as char);
                            }
                            i += 1;
                        }
                        Some(b'"') => {
                            i += 1;
                            break;
                        }
                        Some(&c) => {
                            s.push(c as char);
                            i += 1;
                        }
                        None => break,
                    }
                }
                tokens.push(Token::Quoted(s));
            }
            Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r') => {
                i += 1;
            }
            Some(b';') => {
                while i < bytes.len() && bytes.get(i) != Some(&b'\n') {
                    i += 1;
                }
            }
            _ => {
                let start = i;
                while i < bytes.len() {
                    match bytes.get(i) {
                        Some(b'(') | Some(b')') | Some(b'"') | Some(b' ') | Some(b'\t')
                        | Some(b'\n') | Some(b'\r') => break,
                        _ => i += 1,
                    }
                }
                if let Some(slice) = input.get(start..i)
                    && !slice.is_empty()
                {
                    tokens.push(Token::Atom(slice));
                }
            }
        }
    }

    tokens
}

#[derive(Debug)]
enum SExpr {
    Atom(String),
    List(Vec<SExpr>),
}

fn parse_sexprs(tokens: &[Token<'_>]) -> Vec<SExpr> {
    let mut result = Vec::new();
    let mut pos = 0usize;
    while pos < tokens.len() {
        if let Some(expr) = parse_one(tokens, &mut pos) {
            result.push(expr);
        }
    }
    result
}

fn parse_one(tokens: &[Token<'_>], pos: &mut usize) -> Option<SExpr> {
    match tokens.get(*pos) {
        Some(Token::LParen) => {
            *pos += 1;
            let mut items = Vec::new();
            loop {
                match tokens.get(*pos) {
                    Some(Token::RParen) => {
                        *pos += 1;
                        break;
                    }
                    None => break,
                    _ => {
                        if let Some(child) = parse_one(tokens, pos) {
                            items.push(child);
                        } else {
                            break;
                        }
                    }
                }
            }
            Some(SExpr::List(items))
        }
        Some(Token::RParen) => {
            *pos += 1;
            None
        }
        Some(Token::Atom(s)) => {
            let s = s.to_string();
            *pos += 1;
            Some(SExpr::Atom(s))
        }
        Some(Token::Quoted(s)) => {
            let s = s.clone();
            *pos += 1;
            Some(SExpr::Atom(s))
        }
        None => None,
    }
}

// ---- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_returns_default() {
        let meta = parse_metadata(b"").unwrap();
        assert_eq!(meta, DjVuMetadata::default());
    }

    #[test]
    fn basic_metadata_block() {
        let text = br#"(metadata (title "My Book") (author "Jane Doe") (year "2023"))"#;
        let meta = parse_metadata(text).unwrap();
        assert_eq!(meta.title.as_deref(), Some("My Book"));
        assert_eq!(meta.author.as_deref(), Some("Jane Doe"));
        assert_eq!(meta.year.as_deref(), Some("2023"));
        assert!(meta.subject.is_none());
    }

    #[test]
    fn subject_and_keywords() {
        let text = br#"(metadata (subject "Science") (keywords "physics, chemistry"))"#;
        let meta = parse_metadata(text).unwrap();
        assert_eq!(meta.subject.as_deref(), Some("Science"));
        assert_eq!(meta.keywords.as_deref(), Some("physics, chemistry"));
    }

    #[test]
    fn description_alias_maps_to_subject() {
        let text = br#"(metadata (description "A long description"))"#;
        let meta = parse_metadata(text).unwrap();
        assert_eq!(meta.subject.as_deref(), Some("A long description"));
    }

    #[test]
    fn date_alias_maps_to_year() {
        let text = br#"(metadata (date "2020-01-15"))"#;
        let meta = parse_metadata(text).unwrap();
        assert_eq!(meta.year.as_deref(), Some("2020-01-15"));
    }

    #[test]
    fn extra_keys_go_to_extra_vec() {
        let text = br#"(metadata (custom-field "value1") (another "value2"))"#;
        let meta = parse_metadata(text).unwrap();
        assert_eq!(meta.extra.len(), 2);
        assert_eq!(
            meta.extra[0],
            ("custom-field".to_string(), "value1".to_string())
        );
        assert_eq!(meta.extra[1], ("another".to_string(), "value2".to_string()));
    }

    #[test]
    fn publisher_field() {
        let text = br#"(metadata (publisher "Oxford University Press"))"#;
        let meta = parse_metadata(text).unwrap();
        assert_eq!(meta.publisher.as_deref(), Some("Oxford University Press"));
    }

    #[test]
    fn case_insensitive_keys() {
        let text = br#"(metadata (TITLE "Upper") (Author "Mixed"))"#;
        let meta = parse_metadata(text).unwrap();
        assert_eq!(meta.title.as_deref(), Some("Upper"));
        assert_eq!(meta.author.as_deref(), Some("Mixed"));
    }

    #[test]
    fn escaped_quotes_in_value() {
        let text = br#"(metadata (title "Book with \"quotes\""))"#;
        let meta = parse_metadata(text).unwrap();
        assert_eq!(meta.title.as_deref(), Some(r#"Book with "quotes""#));
    }

    #[test]
    fn no_metadata_wrapper_returns_default() {
        // If there is no (metadata ...) block, return default
        let text = br#"(background #ffffff)"#;
        let meta = parse_metadata(text).unwrap();
        assert_eq!(meta, DjVuMetadata::default());
    }

    #[test]
    fn multiline_metadata() {
        let text = b"(metadata\n  (title \"Line1\")\n  (author \"Line2\")\n)";
        let meta = parse_metadata(text).unwrap();
        assert_eq!(meta.title.as_deref(), Some("Line1"));
        assert_eq!(meta.author.as_deref(), Some("Line2"));
    }

    #[test]
    fn invalid_utf8_returns_error() {
        let invalid = b"\xFF\xFE";
        assert!(matches!(
            parse_metadata(invalid),
            Err(MetadataError::InvalidUtf8)
        ));
    }
}
