use crate::document::{DocumentEdit, DocumentError, TextDocument, TextSnapshot};
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
    LanguageRules(String),
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
            Self::LanguageRules(error) => f.write_str(error),
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
    /// Lets an editor use grammar indentation during typing, paste and
    /// Reindent, and for Enter when no explicit Enter rule applies.
    fn allows_language_indentation(&self) -> bool {
        false
    }

    /// Determines whether a line break at this UTF-16 insertion offset keeps
    /// the suffix's whitespace and bypasses automatic indentation.
    fn preserve_new_line_whitespace(
        &self,
        _source: &TextSnapshot,
        _offset: usize,
    ) -> Result<bool, IndentationError> {
        Ok(false)
    }

    fn indent_line(
        &self,
        document: &TextDocument,
        line_number: usize,
    ) -> Result<Vec<DocumentEdit>, IndentationError>;

    /// Plans automatic indentation when Enter must map a caret through the
    /// replacement. Reapplying the same prefix can move a caret within it.
    fn indent_line_on_enter(
        &self,
        document: &TextDocument,
        line_number: usize,
    ) -> Result<Vec<DocumentEdit>, IndentationError> {
        self.indent_line(document, line_number)
    }

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
