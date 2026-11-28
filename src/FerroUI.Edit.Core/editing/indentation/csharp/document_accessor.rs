use super::super::IndentationError;
use super::super::indentation_strategy::validate_range;
use crate::document::{DocumentEdit, DocumentLine, TextDocument, TextSnapshot};

/// Streaming line access for an indentation reformatter. Read-only context
/// lines still participate in parsing, but their text must not be changed.
pub trait IDocumentAccessor {
    fn is_read_only(&self) -> bool;
    fn line_number(&self) -> usize;
    fn text(&self) -> &str;
    fn set_text(&mut self, text: String) -> Result<(), IndentationError>;
    fn move_next(&mut self) -> Result<bool, IndentationError>;
}

/// Reads an immutable snapshot line by line and records edits without mutation.
/// Lines before `begin_line` provide read-only context; iteration stops at
/// `end_line`. Newline delimiters are excluded from both text and planned edits.
pub struct TextDocumentAccessor {
    snapshot: TextSnapshot,
    begin_line: usize,
    end_line: usize,
    current: Option<DocumentLine>,
    finished: bool,
    original: String,
    text: String,
    edits: Vec<DocumentEdit>,
}

impl TextDocumentAccessor {
    pub fn new(document: &TextDocument) -> Self {
        let snapshot = document.create_snapshot();
        let end_line = snapshot.line_count();
        Self::from_snapshot(snapshot, 1, end_line)
    }

    pub fn with_range(
        document: &TextDocument,
        begin_line: usize,
        end_line: usize,
    ) -> Result<Self, IndentationError> {
        let snapshot = document.create_snapshot();
        validate_range(begin_line, end_line, snapshot.line_count())?;
        Ok(Self::from_snapshot(snapshot, begin_line, end_line))
    }

    fn from_snapshot(snapshot: TextSnapshot, begin_line: usize, end_line: usize) -> Self {
        Self {
            snapshot,
            begin_line,
            end_line,
            current: None,
            finished: false,
            original: String::new(),
            text: String::new(),
            edits: Vec::new(),
        }
    }

    /// Completes the current line and returns nonoverlapping edits at original
    /// UTF-16 offsets. Unchanged contents are never replaced when only padding
    /// changes, so anchors and unpaired UTF-16 surrogates in code are preserved.
    pub fn into_edits(mut self) -> Vec<DocumentEdit> {
        self.flush();
        self.edits
    }

    fn flush(&mut self) {
        let Some(line) = self.current else { return };
        if self.original == self.text {
            return;
        }
        let (old_leading, old_body, old_trailing) = padding(&self.original);
        let (new_leading, new_body, new_trailing) = padding(&self.text);
        if old_body == new_body && !old_body.is_empty() {
            if old_leading != new_leading {
                self.edits.push(DocumentEdit::new(
                    line.offset,
                    old_leading.encode_utf16().count(),
                    new_leading,
                ));
            }
            if old_trailing != new_trailing {
                let length = old_trailing.encode_utf16().count();
                self.edits.push(DocumentEdit::new(
                    line.end_offset() - length,
                    length,
                    new_trailing,
                ));
            }
        } else {
            // The public accessor also permits callers to replace line content.
            // Keep the common prefix/suffix outside that replacement, and never
            // split a supplementary Unicode scalar while finding its bounds.
            let prefix_bytes = self
                .original
                .chars()
                .zip(self.text.chars())
                .take_while(|(a, b)| a == b)
                .map(|(a, _)| a.len_utf8())
                .sum::<usize>();
            let old_tail = &self.original[prefix_bytes..];
            let new_tail = &self.text[prefix_bytes..];
            let suffix_bytes = old_tail
                .chars()
                .rev()
                .zip(new_tail.chars().rev())
                .take_while(|(a, b)| a == b)
                .map(|(a, _)| a.len_utf8())
                .sum::<usize>();
            let prefix_length = self.original[..prefix_bytes].encode_utf16().count();
            let removed = &old_tail[..old_tail.len() - suffix_bytes];
            let inserted = &new_tail[..new_tail.len() - suffix_bytes];
            self.edits.push(DocumentEdit::new(
                line.offset + prefix_length,
                removed.encode_utf16().count(),
                inserted,
            ));
        }
        self.original.clone_from(&self.text);
    }
}

impl IDocumentAccessor for TextDocumentAccessor {
    fn is_read_only(&self) -> bool {
        self.current
            .is_none_or(|line| line.line_number < self.begin_line)
    }

    fn line_number(&self) -> usize {
        self.current.map_or(0, |line| line.line_number)
    }

    fn text(&self) -> &str {
        &self.text
    }

    fn set_text(&mut self, text: String) -> Result<(), IndentationError> {
        if self.current.is_none() || self.finished {
            return Err(IndentationError::InvalidAccessorState);
        }
        if self.is_read_only() {
            return Ok(());
        }
        if text.contains(['\r', '\n']) {
            return Err(IndentationError::InvalidLineText);
        }
        self.text = text;
        Ok(())
    }

    fn move_next(&mut self) -> Result<bool, IndentationError> {
        if self.finished {
            return Ok(false);
        }
        self.flush();
        let next_line = self.line_number() + 1;
        if next_line > self.end_line {
            self.finished = true;
            return Ok(false);
        }
        let line = self.snapshot.get_line_by_number(next_line)?;
        self.text = self.snapshot.get_text(line.offset, line.length)?;
        self.original.clone_from(&self.text);
        self.current = Some(line);
        Ok(true)
    }
}

fn padding(text: &str) -> (&str, &str, &str) {
    let without_leading = text.trim_start();
    let leading = text.len() - without_leading.len();
    let body = without_leading.trim_end();
    (&text[..leading], body, &without_leading[body.len()..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessor_reads_context_and_keeps_original_utf16_offsets() {
        let document = TextDocument::from_text("😀\r\n  b  \rc\n");
        let mut accessor = TextDocumentAccessor::with_range(&document, 2, 3).unwrap();
        assert!(matches!(
            accessor.set_text("x".into()),
            Err(IndentationError::InvalidAccessorState)
        ));
        assert!(accessor.move_next().unwrap());
        assert!(accessor.is_read_only());
        accessor.set_text("ignored".into()).unwrap();
        assert_eq!(accessor.text(), "😀");
        assert!(accessor.move_next().unwrap());
        accessor.set_text("\tb".into()).unwrap();
        assert!(accessor.move_next().unwrap());
        accessor.set_text("\tc".into()).unwrap();
        assert!(!accessor.move_next().unwrap());
        assert!(!accessor.move_next().unwrap());
        let edits = accessor.into_edits();
        assert_eq!(
            edits,
            vec![
                DocumentEdit::new(4, 2, "\t"),
                DocumentEdit::new(7, 2, ""),
                DocumentEdit::new(10, 0, "\t"),
            ]
        );
        assert_eq!(document.text(), "😀\r\n  b  \rc\n");
        document.apply_edits(&edits).unwrap();
        assert_eq!(document.text(), "😀\r\n\tb\r\tc\n");
    }

    #[test]
    fn accessor_rejects_newlines_without_losing_pending_line() {
        let document = TextDocument::from_text("x");
        let mut accessor = TextDocumentAccessor::new(&document);
        accessor.move_next().unwrap();
        for text in ["a\nb", "a\rb", "a\r\nb"] {
            assert_eq!(
                accessor.set_text(text.into()),
                Err(IndentationError::InvalidLineText)
            );
            assert_eq!(accessor.text(), "x");
        }
        accessor.set_text(" y".into()).unwrap();
        document.apply_edits(&accessor.into_edits()).unwrap();
        assert_eq!(document.text(), " y");
    }

    #[test]
    fn whitespace_edits_preserve_unpaired_utf16_and_content_anchors() {
        let document = TextDocument::from_utf16(&[32, 32, 0xd800, 0x61, 32, 32]);
        let anchor = document.create_anchor(3).unwrap();
        let mut accessor = TextDocumentAccessor::new(&document);
        accessor.move_next().unwrap();
        accessor.set_text("\t\u{fffd}a".into()).unwrap();
        document.apply_edits(&accessor.into_edits()).unwrap();
        assert_eq!(
            document.get_text_utf16(0, 3).unwrap(),
            vec![9, 0xd800, 0x61]
        );
        assert_eq!(anchor.offset().unwrap(), 2);
    }
}
