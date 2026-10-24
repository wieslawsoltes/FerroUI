use super::indentation_strategy::validate_range;
use super::{IIndentationStrategy, IndentationError};
use crate::document::{DocumentEdit, TextDocument, TextUtilities};

/// Copies the preceding line's spaces and tabs when indenting a single line.
/// Reindenting a range has no effect without a language-aware strategy.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultIndentationStrategy;

impl IIndentationStrategy for DefaultIndentationStrategy {
    fn indent_line(
        &self,
        document: &TextDocument,
        line_number: usize,
    ) -> Result<Vec<DocumentEdit>, IndentationError> {
        let snapshot = document.create_snapshot();
        validate_range(line_number, line_number, snapshot.line_count())?;
        if line_number == 1 {
            return Ok(Vec::new());
        }
        let previous = snapshot.get_line_by_number(line_number - 1)?;
        let current = snapshot.get_line_by_number(line_number)?;
        let previous_indent = TextUtilities::get_whitespace_after(&snapshot, previous.offset)?;
        let current_indent = TextUtilities::get_whitespace_after(&snapshot, current.offset)?;
        let indentation = snapshot.get_text(previous_indent.offset, previous_indent.length)?;
        if snapshot.get_text(current_indent.offset, current_indent.length)? == indentation {
            return Ok(Vec::new());
        }
        Ok(vec![DocumentEdit::new(
            current_indent.offset,
            current_indent.length,
            indentation,
        )])
    }

    fn indent_lines(
        &self,
        document: &TextDocument,
        begin_line: usize,
        end_line: usize,
    ) -> Result<Vec<DocumentEdit>, IndentationError> {
        validate_range(begin_line, end_line, document.line_count())?;
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_spaces_and_tabs_without_touching_crlf_or_supplementary_text() {
        let document = TextDocument::from_text(" \tbefore😀\r\n    after😀\r\n");
        let original = document.text();
        let edits = DefaultIndentationStrategy
            .indent_line(&document, 2)
            .unwrap();
        assert_eq!(document.text(), original);
        assert_eq!(edits, vec![DocumentEdit::new(12, 4, " \t")]);
        document.apply_edits(&edits).unwrap();
        assert_eq!(document.text(), " \tbefore😀\r\n \tafter😀\r\n");
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), original);
    }

    #[test]
    fn first_line_and_range_are_noops_and_invalid_ranges_are_errors() {
        let document = TextDocument::from_text("  first\n second");
        assert!(
            DefaultIndentationStrategy
                .indent_line(&document, 1)
                .unwrap()
                .is_empty()
        );
        assert!(
            DefaultIndentationStrategy
                .indent_lines(&document, 1, 2)
                .unwrap()
                .is_empty()
        );
        for (begin, end) in [(0, 1), (2, 1), (1, 3), (usize::MAX, usize::MAX)] {
            assert!(matches!(
                DefaultIndentationStrategy.indent_lines(&document, begin, end),
                Err(IndentationError::InvalidLineRange { .. })
            ));
        }
    }

    #[test]
    fn copies_only_spaces_and_tabs_and_skips_unchanged_indentation() {
        let document = TextDocument::from_text("\u{2003}before\n \tafter");
        let edits = DefaultIndentationStrategy
            .indent_line(&document, 2)
            .unwrap();
        document.apply_edits(&edits).unwrap();
        assert_eq!(document.text(), "\u{2003}before\nafter");
        assert!(
            DefaultIndentationStrategy
                .indent_line(&document, 2)
                .unwrap()
                .is_empty()
        );
    }
}
