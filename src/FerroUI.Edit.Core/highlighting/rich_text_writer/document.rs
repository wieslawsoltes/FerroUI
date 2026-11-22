use super::{
    checked_end, HighlightingColor, RichText, RichTextError, RichTextModel, RichTextWriter,
    TextBuffer,
};
use crate::document::{
    AnchorMovementType, DocumentError, DocumentVersion, TextAnchor, TextDocument, TextUtilities,
};
use std::cell::RefCell;
use std::rc::Rc;

struct DocumentStyles {
    version: DocumentVersion,
    model: Rc<RichTextModel>,
}

impl DocumentStyles {
    fn advance_to(&mut self, version: DocumentVersion) -> Result<(), RichTextError> {
        if self.version == version {
            return Ok(());
        }
        let changes = self.version.get_changes_to(&version)?;
        if let [change] = changes.as_slice() {
            Rc::make_mut(&mut self.model).update_offsets_for_document_change(change)?;
        } else {
            let mut model = (*self.model).clone();
            for change in &changes {
                model.update_offsets_for_document_change(change)?;
            }
            self.model = Rc::new(model);
        }
        self.version = version;
        Ok(())
    }
}

/// Formatting bound to a shared text document. Ordinary document edits remap
/// style boundaries. Writes through a rich writer record text and styles in the
/// same undo step. Clones share the same formatting.
#[derive(Clone)]
pub struct RichTextDocument {
    document: TextDocument,
    styles: Rc<RefCell<DocumentStyles>>,
}

impl RichTextDocument {
    pub fn new(document: TextDocument) -> Self {
        Self::with_model(document, &RichTextModel::new())
    }

    pub fn with_model(document: TextDocument, model: &RichTextModel) -> Self {
        let styles = DocumentStyles {
            version: document.version(),
            model: Rc::new(model.clipped(document.text_length())),
        };
        Self {
            document,
            styles: Rc::new(RefCell::new(styles)),
        }
    }

    pub fn document(&self) -> &TextDocument {
        &self.document
    }

    /// Returns the style model for the current document revision.
    pub fn style_model(&self) -> Result<Rc<RichTextModel>, RichTextError> {
        let mut styles = self.styles.borrow_mut();
        styles.advance_to(self.document.version())?;
        Ok(styles.model.clone())
    }

    pub fn snapshot(&self) -> Result<RichText, RichTextError> {
        let snapshot = self.document.create_snapshot();
        let model = self.style_model()?;
        Ok(RichText::from_owned_utf16_with_model(
            snapshot.get_text_utf16(0, snapshot.text_length())?,
            &model,
        ))
    }

    pub fn writer(&self, insertion_offset: usize) -> Result<DocumentRichTextWriter, RichTextError> {
        DocumentRichTextWriter::new(self.clone(), insertion_offset)
    }

    fn insert(
        &self,
        offset: usize,
        text: &[u16],
        color: &HighlightingColor,
    ) -> Result<(), RichTextError> {
        checked_end(self.document.text_length(), text.len())?;
        self.styles
            .borrow_mut()
            .advance_to(self.document.version())?;
        let styles = Rc::downgrade(&self.styles);
        let length = text.len();
        let color = color.clone();
        self.document.insert_utf16_with_effect(
            offset,
            text,
            Rc::new(move |document, forward| {
                if let Some(styles) = styles.upgrade() {
                    let mut styles = styles.borrow_mut();
                    // Every boundary is inside this document and replay uses its
                    // own version chain. Undo removes the inserted range and the
                    // surviving suffix restores the original style at the join.
                    styles
                        .advance_to(document.version())
                        .expect("valid document style offsets");
                    if forward {
                        Rc::make_mut(&mut styles.model)
                            .set_highlighting(offset, length, Some(&color))
                            .expect("validated rich insertion range");
                    }
                }
            }),
        )?;
        Ok(())
    }
}

/// A UTF-16 rich writer whose successful writes are immediately visible in the
/// document. The insertion anchor follows edits and survives deletions.
pub struct DocumentRichTextWriter {
    document: RichTextDocument,
    insertion: TextAnchor,
    newline: String,
    buffer: TextBuffer,
    color: HighlightingColor,
    spans: Vec<HighlightingColor>,
}

impl DocumentRichTextWriter {
    pub fn new(document: RichTextDocument, insertion_offset: usize) -> Result<Self, RichTextError> {
        let insertion = Self::anchor(document.document(), insertion_offset)?;
        let line = document.document().get_line_by_offset(insertion_offset)?;
        let newline =
            TextUtilities::get_new_line_from_document(document.document(), line.line_number())?;
        let color = document
            .style_model()?
            .get_highlighting_at(insertion_offset.saturating_sub(1))
            .clone();
        Ok(Self {
            document,
            insertion,
            newline,
            buffer: TextBuffer::default(),
            color,
            spans: Vec::new(),
        })
    }

    fn anchor(document: &TextDocument, offset: usize) -> Result<TextAnchor, DocumentError> {
        let anchor = document.create_anchor(offset)?;
        anchor.set_movement_type(AnchorMovementType::AfterInsertion);
        anchor.set_survive_deletion(true);
        Ok(anchor)
    }

    pub fn insertion_offset(&self) -> Result<usize, DocumentError> {
        self.insertion.offset()
    }

    pub fn set_insertion_offset(&mut self, offset: usize) -> Result<(), DocumentError> {
        self.insertion = Self::anchor(self.document.document(), offset)?;
        Ok(())
    }

    pub fn set_indentation_string(&mut self, value: &str) -> Result<(), RichTextError> {
        self.buffer.set_indentation_string(value)
    }

    pub fn set_new_line(&mut self, value: &str) -> Result<(), DocumentError> {
        if !matches!(value, "\r" | "\n" | "\r\n") {
            return Err(DocumentError::InvalidNewLine);
        }
        value.clone_into(&mut self.newline);
        Ok(())
    }

    /// Checks span balance. Successful writes are already part of the document.
    pub fn finish(self) -> Result<(), RichTextError> {
        if !self.spans.is_empty() {
            return Err(RichTextError::UnclosedSpans(self.spans.len()));
        }
        Ok(())
    }
}

impl RichTextWriter for DocumentRichTextWriter {
    fn write_utf16(&mut self, text: &[u16]) -> Result<(), RichTextError> {
        if text.is_empty() {
            return Ok(());
        }
        let document = self.document.document();
        if document.is_read_only() {
            return Err(DocumentError::ReadOnly.into());
        }
        let group = document.begin_update()?;
        // UpdateStarted can move the insertion anchor by editing the document.
        let offset = self.insertion.offset()?;
        self.buffer.need_indentation = document.get_line_by_offset(offset)?.offset() == offset;
        self.buffer.write_utf16(text)?;
        let result = self.document.insert(offset, &self.buffer.text, &self.color);
        self.buffer.text.clear();
        result?;
        group.finish()?;
        Ok(())
    }

    fn write_line(&mut self, text: &str) -> Result<(), RichTextError> {
        let units: Vec<_> = text
            .encode_utf16()
            .chain(self.newline.encode_utf16())
            .collect();
        self.write_utf16(&units)
    }

    fn begin_span(&mut self, color: &HighlightingColor) -> Result<(), RichTextError> {
        self.spans.push(self.color.clone());
        self.color = self.color.merge_with(color);
        Ok(())
    }

    fn end_span(&mut self) -> Result<(), RichTextError> {
        self.color = self.spans.pop().ok_or(RichTextError::SpanUnderflow)?;
        Ok(())
    }

    fn indent(&mut self) -> Result<(), RichTextError> {
        self.buffer.indent()
    }

    fn unindent(&mut self) -> Result<(), RichTextError> {
        self.buffer.unindent()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn write_undo_and_redo_publish_text_and_style_together() {
        let document = TextDocument::from_text("ab");
        let mut original = RichTextModel::new();
        original.set_foreground(0, 1, Some("red")).unwrap();
        original.set_foreground(1, 1, Some("blue")).unwrap();
        let rich = RichTextDocument::with_model(document.clone(), &original);
        let before = rich.snapshot().unwrap();
        let changes = Rc::new(RefCell::new(Vec::new()));
        let _subscription = document.changed({
            let rich = rich.clone();
            let changes = changes.clone();
            move |_| changes.borrow_mut().push(rich.snapshot().unwrap())
        });
        let mut writer = rich.writer(1).unwrap();
        writer.begin_bold_span(true).unwrap();
        writer.write_utf16(&[0xd83d, 0xde00, 0xd800]).unwrap();
        assert_eq!(writer.insertion_offset().unwrap(), 4);
        writer.end_span().unwrap();
        writer.finish().unwrap();
        let after = rich.snapshot().unwrap();
        assert_eq!(after.text_utf16(), &[97, 0xd83d, 0xde00, 0xd800, 98]);
        assert_eq!(
            after.get_highlighting_at(2).unwrap().foreground.as_deref(),
            Some("red")
        );
        assert_eq!(after.get_highlighting_at(2).unwrap().bold, Some(true));
        assert_eq!(
            after.get_highlighting_at(4).unwrap().foreground.as_deref(),
            Some("blue")
        );
        document.undo().unwrap();
        document.redo().unwrap();
        assert_eq!(*changes.borrow(), [after.clone(), before, after]);
    }

    #[test]
    fn writer_follows_an_anchor_and_groups_nested_spans_and_document_newlines() {
        let document = TextDocument::from_text("header\r\nbody");
        let rich = RichTextDocument::new(document.clone());
        let mut writer = rich.writer(8).unwrap();
        document.insert(0, "+").unwrap();
        assert_eq!(writer.insertion_offset().unwrap(), 9);
        document.clear_undo_history().unwrap();
        let group = document.begin_update().unwrap();
        writer.set_indentation_string("  ").unwrap();
        writer.indent().unwrap();
        writer.begin_foreground_span("green").unwrap();
        writer.write_line("start").unwrap();
        writer.begin_italic_span(true).unwrap();
        writer.write("x").unwrap();
        writer.end_span().unwrap();
        writer.end_span().unwrap();
        group.finish().unwrap();
        let after = rich.snapshot().unwrap();
        assert_eq!(after.text(), "+header\r\n  start\r\n  xbody");
        assert_eq!(after.get_highlighting_at(20).unwrap().italic, Some(true));
        assert_eq!(after.get_highlighting_at(21).unwrap().foreground, None);
        assert_eq!(document.undo_count(), 1);
        document.undo().unwrap();
        assert_eq!(rich.snapshot().unwrap().text(), "+header\r\nbody");
        document.redo().unwrap();
        assert_eq!(rich.snapshot().unwrap(), after);
    }

    #[test]
    fn update_started_edits_rebase_the_writer_and_rejected_writes_leave_no_pending_buffer() {
        let document = TextDocument::from_text("ab");
        let rich = RichTextDocument::new(document.clone());
        let mut writer = rich.writer(1).unwrap();
        let first = Rc::new(Cell::new(true));
        let _subscription = document.update_started({
            let document = document.clone();
            move || {
                if first.replace(false) {
                    document.insert(0, "+").unwrap();
                    document.set_is_read_only(true);
                }
            }
        });
        assert!(matches!(
            writer.write("rejected"),
            Err(RichTextError::Document(DocumentError::ReadOnly))
        ));
        assert_eq!(rich.snapshot().unwrap().text(), "+ab");
        document.set_is_read_only(false);
        writer.write("!").unwrap();
        assert_eq!(rich.snapshot().unwrap().text(), "+a!b");
        assert_eq!(writer.insertion_offset().unwrap(), 3);
    }
}
