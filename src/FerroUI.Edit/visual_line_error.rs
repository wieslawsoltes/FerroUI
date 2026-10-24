use ferroui_edit_core::document::DocumentError;
use std::fmt;

/// A rejected element, generator result, transformation, or text run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VisualLineError {
    Document(DocumentError),
    InvalidVisualLength,
    InvalidRange {
        start: usize,
        end: usize,
        length: usize,
    },
    GeneratorInterestBeforeStart {
        requested: usize,
        returned: usize,
    },
    ElementEndsInDelimiter {
        offset: usize,
    },
    DocumentLengthChanged {
        expected: usize,
        actual: usize,
    },
    InvalidRunLength {
        visual_column: usize,
        length: i32,
        remaining: usize,
    },
    InvalidRunMetrics {
        visual_column: usize,
    },
    InvalidPrecedingText {
        visual_column: usize,
        length: usize,
        remaining: usize,
    },
    InvalidCaretPosition {
        requested: isize,
        returned: usize,
    },
    CannotSplitElement {
        visual_column: usize,
    },
    InvalidSplit {
        visual_column: usize,
    },
    OffsetOverflow,
    AllocationFailed,
    RecursiveTransformation,
    RecursiveConstruction,
    ConstructionInvalidated,
    Extension(String),
}
impl fmt::Display for VisualLineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Document(error) => error.fmt(f),
            Self::InvalidVisualLength => {
                f.write_str("A visual element must occupy at least one visual column")
            }
            Self::InvalidRange { start, end, length } => {
                write!(f, "Visual range {start}..{end} is outside 0..{length}")
            }
            Self::GeneratorInterestBeforeStart {
                requested,
                returned,
            } => write!(
                f,
                "An element generator returned offset {returned} before the requested offset {requested}"
            ),
            Self::ElementEndsInDelimiter { offset } => write!(
                f,
                "A generated visual element ends inside a line delimiter at offset {offset}"
            ),
            Self::DocumentLengthChanged { expected, actual } => write!(
                f,
                "A visual transformation changed document length from {expected} to {actual}"
            ),
            Self::InvalidRunLength {
                visual_column,
                length,
                remaining,
            } => write!(
                f,
                "Text run at visual column {visual_column} has length {length}; expected 1..={remaining}"
            ),
            Self::InvalidRunMetrics { visual_column } => write!(
                f,
                "Drawable text run at visual column {visual_column} has non-finite or negative dimensions"
            ),
            Self::InvalidPrecedingText {
                visual_column,
                length,
                remaining,
            } => write!(
                f,
                "Preceding text at visual column {visual_column} has length {length}; expected at most {remaining}"
            ),
            Self::InvalidCaretPosition {
                requested,
                returned,
            } => write!(
                f,
                "A visual element returned invalid caret column {returned} from {requested}"
            ),
            Self::CannotSplitElement { visual_column } => write!(
                f,
                "The visual element cannot be split at column {visual_column}"
            ),
            Self::InvalidSplit { visual_column } => write!(
                f,
                "The visual element returned inconsistent lengths when split at column {visual_column}"
            ),
            Self::OffsetOverflow => {
                f.write_str("The visual line exceeds the formatter's supported index range")
            }
            Self::AllocationFailed => f.write_str("The visual line cannot allocate its projection"),
            Self::RecursiveTransformation => {
                f.write_str("A visual line transformer cannot recursively transform a line")
            }
            Self::RecursiveConstruction => f.write_str(
                "A visual line extension cannot recursively construct a line in the same view",
            ),
            Self::ConstructionInvalidated => f.write_str(
                "The document or view changed while a visual line was being constructed",
            ),
            Self::Extension(message) => f.write_str(message),
        }
    }
}
impl std::error::Error for VisualLineError {}
impl From<DocumentError> for VisualLineError {
    fn from(error: DocumentError) -> Self {
        Self::Document(error)
    }
}
