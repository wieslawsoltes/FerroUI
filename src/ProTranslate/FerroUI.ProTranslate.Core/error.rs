use std::fmt;

/// A recoverable error from culture parsing, providers, formatting, or units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranslationError {
    InvalidCulture(String),
    InvalidKey,
    Provider(String),
    Format(String),
    Region(String),
    IncompatibleUnits,
    UnsupportedProfileUnit(String),
}
impl fmt::Display for TranslationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCulture(v) => write!(f, "Invalid culture: {v}"),
            Self::InvalidKey => f.write_str("A translation key must not be empty or whitespace"),
            Self::Provider(v) => write!(f, "Translation provider failed: {v}"),
            Self::Format(v) => write!(f, "Invalid composite format: {v}"),
            Self::Region(v) => write!(f, "Region profile could not be resolved: {v}"),
            Self::IncompatibleUnits => {
                f.write_str("Units must belong to the same measurement category")
            }
            Self::UnsupportedProfileUnit(v) => {
                write!(f, "Unsupported measurement profile unit: {v}")
            }
        }
    }
}
impl std::error::Error for TranslationError {}
