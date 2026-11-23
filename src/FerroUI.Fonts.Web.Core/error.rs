//! Why a font was refused.

use std::fmt;

/// Why a font could not be decoded or did not pass the sanitizer. A font that fails is not
/// used, and the next `src` entry is tried.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontDecodeError {
    /// The data ends before a structure does.
    Truncated,
    /// The file is not of a known font format.
    UnknownFormat,
    /// A required table is missing.
    MissingTable([u8; 4]),
    /// A structure breaks the rules of its format.
    Malformed(&'static str),
    /// The font is over a limit of [`WebFontLimits`](crate::WebFontLimits).
    LimitExceeded(&'static str),
    /// The format is known, but this build cannot decode it.
    Unsupported(&'static str),
}

impl fmt::Display for FontDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => f.write_str("the font data is truncated"),
            Self::UnknownFormat => f.write_str("the data is not a known font format"),
            Self::MissingTable(tag) => write!(f, "the font has no {} table", String::from_utf8_lossy(tag)),
            Self::Malformed(what) => write!(f, "malformed font: {what}"),
            Self::LimitExceeded(what) => write!(f, "the font is over the limit for {what}"),
            Self::Unsupported(what) => write!(f, "unsupported font: {what}"),
        }
    }
}

impl std::error::Error for FontDecodeError {}
