use ferroui_protranslate_formats::TranslationFormatDiagnostic;

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum StudioError {
    InvalidInput(String),
    Format(Vec<TranslationFormatDiagnostic>),
    Serialization(String),
    Storage(String),
    EntryNotFound(u64),
    StaleSuggestion,
    Cancelled,
    Provider(String),
}

impl std::fmt::Display for StudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(message) => write!(f, "Invalid Studio input: {message}"),
            Self::Format(diagnostics) => write!(
                f,
                "Catalog conversion failed: {}",
                diagnostics
                    .iter()
                    .map(|d| d.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            Self::Serialization(message) => write!(f, "Invalid Studio document: {message}"),
            Self::Storage(message) => write!(f, "Studio storage failed: {message}"),
            Self::EntryNotFound(id) => write!(f, "Catalog entry {id} no longer exists"),
            Self::StaleSuggestion => {
                f.write_str("The entry changed after this translation was requested")
            }
            Self::Cancelled => f.write_str("Translation was cancelled"),
            Self::Provider(message) => write!(f, "Translation provider failed: {message}"),
        }
    }
}

impl std::error::Error for StudioError {}

impl From<serde_json::Error> for StudioError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value.to_string())
    }
}
