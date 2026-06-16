//! Shared S-expression reader for DjVu text chunks.
//!
//! Both the ANTa/ANTz annotation parser ([`crate::annotation`]) and the
//! METa/METz metadata parser ([`crate::metadata`]) consume the same
//! S-expression syntax: parenthesised lists of unquoted atoms and `"`-quoted
//! strings, with `;` line comments. This module owns the single tokenizer and
//! recursive-descent reader they share, including the recursion-depth guard
//! that bounds stack usage on crafted (deeply nested) input.
//!
//! The reader is lenient and best-effort: malformed fragments (unmatched
//! parens, payload past the depth limit) are dropped rather than reported, so
//! that a partially-valid chunk still yields the forms it can. Each caller
//! keeps its own `&[u8]` → `&str` decode policy (annotation is lossy, metadata
//! rejects invalid UTF-8) and its own interpretation of the resulting tree.

#[cfg(not(feature = "std"))]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

/// A node in a parsed S-expression tree.
///
/// Quoted strings and unquoted atoms both reduce to [`SExpr::Atom`]; no current
/// consumer needs to tell them apart.
#[derive(Debug)]
pub(crate) enum SExpr {
    Atom(String),
    List(Vec<SExpr>),
}

/// Maximum nesting depth for parsed lists.
///
/// Bounds recursion in [`parse_sexprs`] so a deeply nested payload cannot
/// overflow the stack. Lists deeper than this are truncated at the limit.
const MAX_SEXPR_DEPTH: usize = 64;

/// Minimal S-expression token.
#[derive(Debug, PartialEq)]
enum Token<'a> {
    LParen,
    RParen,
    Atom(&'a str),
    Quoted(String),
}

/// Parse `input` into a flat list of top-level S-expressions.
///
/// Lenient: see the module docs.
pub(crate) fn parse_sexprs(input: &str) -> Vec<SExpr> {
    let tokens = tokenize(input);
    let mut result = Vec::new();
    let mut pos = 0usize;
    while pos < tokens.len() {
        if let Some(expr) = parse_one(&tokens, &mut pos, 0) {
            result.push(expr);
        }
    }
    result
}

/// Tokenize an S-expression string into a flat `Vec` of tokens.
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
                // line comment
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

fn parse_one(tokens: &[Token<'_>], pos: &mut usize, depth: usize) -> Option<SExpr> {
    if depth > MAX_SEXPR_DEPTH {
        return None;
    }
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
                        if let Some(child) = parse_one(tokens, pos, depth + 1) {
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
            // Unexpected RParen — skip
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

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(e: &SExpr) -> Option<&str> {
        match e {
            SExpr::Atom(s) => Some(s.as_str()),
            SExpr::List(_) => None,
        }
    }

    #[test]
    fn parses_flat_list() {
        let exprs = parse_sexprs("(zoom 100)");
        assert_eq!(exprs.len(), 1);
        let SExpr::List(items) = &exprs[0] else {
            panic!("expected list")
        };
        assert_eq!(atom(&items[0]), Some("zoom"));
        assert_eq!(atom(&items[1]), Some("100"));
    }

    #[test]
    fn quoted_and_atom_both_reduce_to_atom() {
        let exprs = parse_sexprs(r#"(title "My Book")"#);
        let SExpr::List(items) = &exprs[0] else {
            panic!("expected list")
        };
        assert_eq!(atom(&items[1]), Some("My Book"));
    }

    #[test]
    fn line_comment_is_skipped() {
        let exprs = parse_sexprs("; a comment\n(a b)");
        assert_eq!(exprs.len(), 1);
    }

    #[test]
    fn escaped_quote_inside_string() {
        let exprs = parse_sexprs(r#"(k "a\"b")"#);
        let SExpr::List(items) = &exprs[0] else {
            panic!("expected list")
        };
        assert_eq!(atom(&items[1]), Some("a\"b"));
    }

    #[test]
    fn depth_guard_bounds_recursion() {
        // Far deeper than MAX_SEXPR_DEPTH — must not overflow the stack and
        // must still return (truncated) rather than panic.
        let deep = format!("{}{}", "(".repeat(10_000), ")".repeat(10_000));
        let exprs = parse_sexprs(&deep);
        assert_eq!(exprs.len(), 1);
    }
}
