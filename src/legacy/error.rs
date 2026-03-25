use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Input data is shorter than expected.
    UnexpectedEof,
    /// A required magic number or tag was not found.
    InvalidMagic,
    /// A chunk or field has an invalid length.
    InvalidLength,
    /// A required chunk is missing.
    MissingChunk(&'static str),
    /// An unsupported feature or version was encountered.
    Unsupported(&'static str),
    /// Generic format violation.
    FormatError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnexpectedEof => write!(f, "unexpected end of input"),
            Error::InvalidMagic => write!(f, "invalid magic number"),
            Error::InvalidLength => write!(f, "invalid length"),
            Error::MissingChunk(id) => write!(f, "missing required chunk: {}", id),
            Error::Unsupported(msg) => write!(f, "unsupported: {}", msg),
            Error::FormatError(msg) => write!(f, "format error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}
