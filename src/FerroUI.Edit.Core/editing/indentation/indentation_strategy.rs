use crate::document::{DocumentEdit, DocumentError, TextDocument};
use std::fmt;

/// Errors reported while planning indentation. Planning never mutates the document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IndentationError {
    Document(DocumentError),
    InvalidLineRange {
        begin_line: usize,
        end_line: usize,
        line_count: usize,
    },
    InvalidIndentationString,
    InvalidAccessorState,
    InvalidLineText,
}

impl fmt::Display for IndentationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Document(error) => error.fmt(f),
            Self::InvalidLineRange {
                begin_line,
                end_line,
                line_count,
            } => write!(
                f,
                "indentation range {begin_line}..={end_line} must lie within 1..={line_count}"
            ),
            Self::InvalidIndentationString => {
                f.write_str("indentation must contain one or more spaces or tabs")
            }
            Self::InvalidAccessorState => f.write_str("the document accessor has no current line"),
            Self::InvalidLineText => f.write_str("line text cannot contain CR or LF delimiters"),
        }
    }
}

impl std::error::Error for IndentationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Document(error) => Some(error),
            _ => None,
        }
    }
}

impl From<DocumentError> for IndentationError {
    fn from(value: DocumentError) -> Self {
        Self::Document(value)
    }
}

/// Plans indentation changes at UTF-16 offsets in the original document.
///
/// Line numbers are one-based and ranges are inclusive. The caller can check
/// every edit against protected sections before passing the whole plan to
/// `TextDocument::apply_edits`. Apply a plan before making other document changes.
pub trait IIndentationStrategy {
    fn indent_line(
        &self,
        document: &TextDocument,
        line_number: usize,
    ) -> Result<Vec<DocumentEdit>, IndentationError>;

    fn indent_lines(
        &self,
        document: &TextDocument,
        begin_line: usize,
        end_line: usize,
    ) -> Result<Vec<DocumentEdit>, IndentationError>;
}

pub(super) fn validate_range(
    begin_line: usize,
    end_line: usize,
    line_count: usize,
) -> Result<(), IndentationError> {
    if begin_line == 0 || begin_line > end_line || end_line > line_count {
        return Err(IndentationError::InvalidLineRange {
            begin_line,
            end_line,
            line_count,
        });
    }
    Ok(())
}
