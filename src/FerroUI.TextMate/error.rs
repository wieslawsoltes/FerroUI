use std::fmt;

/// Grammar, selector or regular-expression failures are reported to the caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Json(String),
    InvalidGrammar(String),
    MissingGrammar(String),
    MissingResource(String),
    InvalidSelector(String),
    InvalidFilenamePattern {
        pattern: String,
        error: crate::FilenameGlobError,
    },
    Regex {
        pattern: String,
        message: String,
    },
    RegexLimit(String),
    InvalidState,
    InvalidLine(usize),
    InvalidOffset(usize),
    ResourceCycle(String),
    BackendNotInitialized,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(s) => write!(f, "invalid JSON: {s}"),
            Self::InvalidGrammar(s) => write!(f, "invalid grammar: {s}"),
            Self::MissingGrammar(s) => write!(f, "grammar is not registered: {s}"),
            Self::MissingResource(s) => write!(f, "resource is not registered: {s}"),
            Self::InvalidSelector(s) => write!(f, "invalid scope selector: {s}"),
            Self::InvalidFilenamePattern { pattern, error } => {
                write!(f, "filename pattern {pattern:?}: {error}")
            }
            Self::Regex { pattern, message } => {
                write!(f, "regular expression {pattern:?}: {message}")
            }
            Self::RegexLimit(s) => write!(f, "regular expression work limit: {s}"),
            Self::InvalidState => f.write_str("tokenization state belongs to a different grammar"),
            Self::InvalidLine(line) => write!(f, "line index out of range: {line}"),
            Self::InvalidOffset(offset) => write!(f, "invalid UTF-16 offset: {offset}"),
            Self::ResourceCycle(s) => write!(f, "resource include cycle: {s}"),
            Self::BackendNotInitialized => {
                f.write_str("initialize_browser_oniguruma must finish before tokenization")
            }
        }
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
