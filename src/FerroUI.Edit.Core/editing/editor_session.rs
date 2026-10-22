mod virtual_space;

use super::{
    next_caret_position, plan_text_transformation, CaretMovement, DefaultIndentationStrategy,
    IReadOnlySectionProvider, IndentationError, IndentationStrategy, NoReadOnlySections, Selection,
    SelectionSegment, TextTransformation,
};
use crate::document::{
    AnchorMovementType, DocumentError, DocumentSubscription, OffsetChangeMappingType,
    SimpleSegment, TextDocument, TextUtilities,
};
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::fmt;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorOptions {
    pub indentation_size: usize,
    pub convert_tabs_to_spaces: bool,
    pub enable_virtual_space: bool,
    pub allow_scroll_below_document: bool,
    pub show_spaces: bool,
    pub show_tabs: bool,
    pub show_end_of_line: bool,
    pub highlight_current_line: bool,
    pub newline: String,
}
impl Default for EditorOptions {
    fn default() -> Self {
        Self {
            indentation_size: 4,
            convert_tabs_to_spaces: false,
            enable_virtual_space: false,
            allow_scroll_below_document: false,
            show_spaces: false,
            show_tabs: false,
            show_end_of_line: false,
            highlight_current_line: false,
            newline: "\n".into(),
        }
    }
}
impl EditorOptions {
    pub fn indentation_string(&self) -> String {
        if self.convert_tabs_to_spaces {
            " ".repeat(self.indentation_size.clamp(1, 256))
        } else {
            "\t".into()
        }
    }
}

#[derive(Debug)]
pub enum EditError {
    Document(DocumentError),
    Indentation(IndentationError),
    ReadOnly,
    InvalidSelection,
    ReplacementCount,
    AmbiguousVirtualCarets,
    VirtualSpaceTooLarge,
    StaleEditPlan,
}
impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Document(e) => e.fmt(f),
            Self::Indentation(e) => e.fmt(f),
            Self::ReadOnly => f.write_str("The edit intersects a read-only section"),
            Self::InvalidSelection => f.write_str("The selection lies outside the document"),
            Self::ReplacementCount => {
                f.write_str("Supply one replacement or one replacement per selection")
            }
            Self::AmbiguousVirtualCarets => f.write_str(
                "Distinct virtual selections at one physical line end cannot be edited together",
            ),
            Self::VirtualSpaceTooLarge => {
                f.write_str("The requested virtual-space padding is too large")
            }
            Self::StaleEditPlan => {
                f.write_str("The document changed while the edit was being prepared")
            }
        }
    }
}
impl std::error::Error for EditError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Document(error) => Some(error),
            Self::Indentation(error) => Some(error),
            _ => None,
        }
    }
}
impl From<DocumentError> for EditError {
    fn from(value: DocumentError) -> Self {
        Self::Document(value)
    }
}
impl From<IndentationError> for EditError {
    fn from(value: IndentationError) -> Self {
        Self::Indentation(value)
    }
}

struct SessionInner {
    document: TextDocument,
    selection: Rc<RefCell<Selection>>,
    options: RefCell<EditorOptions>,
    desired_columns: RefCell<Vec<Option<usize>>>,
    indentation_strategy: RefCell<Option<Rc<dyn IndentationStrategy>>>,
    read_only: Cell<bool>,
    read_only_sections: RefCell<Rc<dyn IReadOnlySectionProvider>>,
    subscription: DocumentSubscription,
}
impl Drop for SessionInner {
    fn drop(&mut self) {
        self.subscription.dispose();
    }
}

/// The editing state shared by a text area and its commands. All positions use
/// UTF-16. A session can be cloned without copying the document or selections.
#[derive(Clone)]
pub struct EditorSession {
    inner: Rc<SessionInner>,
}
impl EditorSession {
    pub fn new(document: TextDocument) -> Self {
        let inner = Rc::new_cyclic(|weak: &std::rc::Weak<SessionInner>| {
            let listener = weak.clone();
            let subscription = document.changed(move |change| {
                if let Some(inner) = listener.upgrade() {
                    let updated = inner.selection.borrow().update_on_document_change(change);
                    let updated = virtual_space::normalize_selection(&inner.document, updated);
                    *inner.selection.borrow_mut() = updated;
                    inner.desired_columns.borrow_mut().clear();
                }
            });
            SessionInner {
                document,
                selection: Rc::new(RefCell::new(Selection::default())),
                options: RefCell::new(EditorOptions::default()),
                desired_columns: RefCell::new(Vec::new()),
                indentation_strategy: RefCell::new(Some(Rc::new(DefaultIndentationStrategy))),
                read_only: Cell::new(false),
                read_only_sections: RefCell::new(Rc::new(NoReadOnlySections)),
                subscription,
            }
        });
        Self { inner }
    }

    pub fn document(&self) -> &TextDocument {
        &self.inner.document
    }
    pub fn selection(&self) -> Selection {
        self.inner.selection.borrow().clone()
    }
    pub fn options(&self) -> EditorOptions {
        self.inner.options.borrow().clone()
    }
    pub fn set_options(&self, mut options: EditorOptions) {
        options.indentation_size = options.indentation_size.clamp(1, 256);
        if !matches!(options.newline.as_str(), "\n" | "\r" | "\r\n") {
            options.newline = "\n".into();
        }
        let disable_virtual = !options.enable_virtual_space;
        *self.inner.options.borrow_mut() = options;
        self.inner.desired_columns.borrow_mut().clear();
        let selection = self.selection();
        if disable_virtual && !selection.is_rectangular() {
            *self.inner.selection.borrow_mut() = Selection::from_segments(
                selection
                    .segments()
                    .iter()
                    .map(|s| SelectionSegment::new(s.anchor, s.caret)),
                selection.primary_index(),
            );
        }
    }
    pub fn indentation_strategy(&self) -> Option<Rc<dyn IndentationStrategy>> {
        self.inner.indentation_strategy.borrow().clone()
    }
    /// Selects the strategy used by Enter and explicit reindent commands.
    /// `None` inserts only the configured newline without adjusting indentation.
    pub fn set_indentation_strategy(&self, strategy: Option<Rc<dyn IndentationStrategy>>) {
        *self.inner.indentation_strategy.borrow_mut() = strategy;
    }
    pub fn is_read_only(&self) -> bool {
        self.inner.read_only.get()
    }
    pub fn set_is_read_only(&self, value: bool) {
        self.inner.read_only.set(value);
    }
    pub fn set_read_only_section_provider(&self, provider: Rc<dyn IReadOnlySectionProvider>) {
        *self.inner.read_only_sections.borrow_mut() = provider;
    }
    pub fn set_selection(&self, selection: Selection) -> Result<(), EditError> {
        self.validate_selection(&selection)?;
        *self.inner.selection.borrow_mut() = selection;
        self.inner.desired_columns.borrow_mut().clear();
        Ok(())
    }

    pub fn select(&self, start: usize, length: usize) -> Result<(), EditError> {
        self.set_selection(Selection::simple(
            start,
            start
                .checked_add(length)
                .ok_or(EditError::InvalidSelection)?,
        ))
    }
    pub fn select_all(&self) {
        *self.inner.selection.borrow_mut() = Selection::simple(0, self.document().text_length());
        self.inner.desired_columns.borrow_mut().clear();
    }
    pub fn caret_offset(&self) -> usize {
        self.selection().primary().caret
    }
    pub fn set_caret_offset(&self, offset: usize) -> Result<(), EditError> {
        self.set_caret_position(offset, 0)
    }
    pub fn caret_virtual_space(&self) -> usize {
        self.selection().primary().caret_virtual_space
    }
    /// Places a caret at a UTF-16 offset and, optionally, cells beyond its
    /// logical line end. Virtual cells are valid only at that end offset.
    pub fn set_caret_position(&self, offset: usize, virtual_space: usize) -> Result<(), EditError> {
        self.set_selection(Selection::from_segments(
            [SelectionSegment::with_virtual_space(
                offset,
                virtual_space,
                offset,
                virtual_space,
            )],
            0,
        ))
    }
    pub fn selected_text(&self) -> Result<String, EditError> {
        let mut text = String::new();
        for (index, segment) in self.selection().segments().iter().enumerate() {
            if index != 0 {
                text.push_str(&self.options().newline);
            }
            text.push_str(
                &self
                    .document()
                    .get_text(segment.start_offset(), segment.length())?,
            );
        }
        Ok(text)
    }

    pub fn replace_selection_with_text(&self, text: &str) -> Result<(), EditError> {
        self.replace_selections(&[text.to_owned()])
    }
    pub fn replace_selections(&self, replacements: &[String]) -> Result<(), EditError> {
        self.replace_virtual_selections(replacements)
    }

    fn replace_segments(
        &self,
        edits: &[(SimpleSegment, String)],
        primary: usize,
    ) -> Result<(), EditError> {
        self.execute_replacements(
            edits
                .iter()
                .map(|(segment, text)| virtual_space::PlannedReplacement {
                    segment: *segment,
                    text: text.clone(),
                    virtual_space: 0,
                })
                .collect(),
            primary,
            false,
            false,
        )
    }

    fn validate_edits(&self, edits: &[(SimpleSegment, String)]) -> Result<(), EditError> {
        self.validate_ranges(
            edits
                .iter()
                .map(|(segment, text)| (*segment, !text.is_empty())),
        )
    }
    fn validate_ranges(
        &self,
        edits: impl IntoIterator<Item = (SimpleSegment, bool)>,
    ) -> Result<(), EditError> {
        if self.is_read_only() || self.document().is_read_only() {
            return Err(EditError::ReadOnly);
        }
        let provider = self.inner.read_only_sections.borrow().clone();
        let length = self.document().text_length();
        for (segment, inserts_text) in edits {
            if segment.offset > length || segment.length > length - segment.offset {
                return Err(EditError::InvalidSelection);
            }
            if inserts_text && !provider.can_insert(segment.offset) {
                return Err(EditError::ReadOnly);
            }
            let writable: usize = provider
                .get_deletable_segments(segment)
                .iter()
                .map(|s| s.length)
                .sum();
            if writable != segment.length {
                return Err(EditError::ReadOnly);
            }
        }
        Ok(())
    }
    fn apply_edits(&self, edits: &[(SimpleSegment, String)]) -> Result<(), EditError> {
        self.validate_edits(edits)?;
        let _update = self.document().begin_update()?;
        for (segment, text) in edits.iter().rev() {
            self.document()
                .replace(segment.offset, segment.length, text)?;
        }
        Ok(())
    }

    /// Applies a precomputed edit plan using original document coordinates.
    /// All protected ranges are checked before the document validates overlaps
    /// and commits the plan as one undo unit.
    pub fn apply_document_edits(
        &self,
        edits: &[crate::document::DocumentEdit],
    ) -> Result<(), EditError> {
        self.validate_document_edits(edits)?;
        self.document().apply_edits(edits)?;
        Ok(())
    }
    /// Checks the host's read-only policy without changing the document. Other
    /// editing services can use this before applying their own undo grouping.
    pub fn validate_document_edits(
        &self,
        edits: &[crate::document::DocumentEdit],
    ) -> Result<(), EditError> {
        self.validate_ranges(edits.iter().map(|edit| {
            (
                SimpleSegment::new(edit.offset, edit.length),
                !edit.text.is_empty(),
            )
        }))
    }
    pub fn move_caret(&self, movement: CaretMovement, extend_selection: bool) {
        self.move_caret_with_virtual_space(movement, extend_selection);
    }

    fn move_position(&self, offset: usize, movement: CaretMovement) -> usize {
        let document = self.document();
        if movement == CaretMovement::DocumentStart {
            return 0;
        }
        if movement == CaretMovement::DocumentEnd {
            return document.text_length();
        }
        let Ok(line) = document.get_line_by_offset(offset) else {
            return offset.min(document.text_length());
        };
        match movement {
            CaretMovement::LineStart => return line.offset,
            CaretMovement::LineEnd => return line.offset + line.length,
            CaretMovement::LineUp | CaretMovement::LineDown => {
                let number = if movement == CaretMovement::LineUp {
                    line.line_number.saturating_sub(1)
                } else {
                    line.line_number + 1
                };
                let Ok(target) = document.get_line_by_number(number) else {
                    return if movement == CaretMovement::LineUp {
                        0
                    } else {
                        document.text_length()
                    };
                };
                let column = offset - line.offset;
                let position = target.offset + column.min(target.length);
                if column >= target.length {
                    return target.offset + target.length;
                }
                // Snap a UTF-16 column that would split a surrogate or grapheme.
                let Ok(text) = document.get_text(target.offset, target.length) else {
                    return position;
                };
                if column == 0 {
                    return target.offset;
                }
                return target.offset
                    + next_caret_position(
                        &text,
                        column.min(target.length) + 1,
                        CaretMovement::CharacterLeft,
                    );
            }
            CaretMovement::CharacterLeft | CaretMovement::WordLeft if offset == line.offset => {
                return document
                    .get_line_by_number(line.line_number.saturating_sub(1))
                    .map_or(0, |l| l.offset + l.length);
            }
            CaretMovement::CharacterRight | CaretMovement::WordRight
                if offset >= line.offset + line.length =>
            {
                return (line.offset + line.length + line.delimiter_length)
                    .min(document.text_length());
            }
            _ => {}
        }
        let Ok(text) = document.get_text(line.offset, line.length) else {
            return offset;
        };
        line.offset + next_caret_position(&text, offset - line.offset, movement)
    }
    pub fn delete(&self, backward: bool, by_word: bool) -> Result<(), EditError> {
        self.delete_with_virtual_space(backward, by_word)
    }

    pub fn insert_new_line(&self) -> Result<(), EditError> {
        let options = self.options();
        let Some(strategy) = self.indentation_strategy() else {
            return self.replace_selection_with_text(&options.newline);
        };
        let selection = self.selection();
        self.validate_virtual_edit_positions(&selection)?;
        let source = self.document().create_snapshot();
        let mut edits = Vec::with_capacity(selection.segments().len());
        for (index, segment) in selection.segments().iter().enumerate() {
            let whitespace = TextUtilities::get_whitespace_after(&source, segment.end_offset())?;
            let limit = selection
                .segments()
                .get(index + 1)
                .map_or(source.text_length(), |next| next.start_offset());
            // A later caret owns the whitespace following its own selection.
            // This keeps the composed multicaret edits disjoint.
            let end = whitespace.end_offset().min(limit);
            edits.push((
                SimpleSegment::new(segment.start_offset(), end - segment.start_offset()),
                options.newline.clone(),
            ));
        }
        self.validate_edits(&edits)?;

        // A persistent snapshot shares the source rope. Strategies observe the
        // complete prospective document without mutating the real document or
        // exposing partially formatted text to its change handlers.
        let preview = TextDocument::from_snapshot(source);
        let mut lines = Vec::with_capacity(selection.segments().len());
        for segment in selection.segments().iter().rev() {
            preview.replace(segment.start_offset(), segment.length(), &options.newline)?;
            let anchor = preview
                .create_anchor(segment.start_offset() + options.newline.encode_utf16().count())?;
            anchor.set_movement_type(AnchorMovementType::AfterInsertion);
            anchor.set_survive_deletion(true);
            lines.push(anchor);
        }
        lines.reverse();
        for (index, anchor) in lines.iter().enumerate() {
            let line = preview.get_line_by_offset(anchor.offset()?)?;
            let plan = strategy.indent_line(&preview, line.line_number)?;
            preview.apply_edits(&plan)?;
            let line = preview.get_line_by_offset(anchor.offset()?)?;
            let indentation = TextUtilities::get_whitespace_after(&preview, line.offset)?;
            // Automatic indentation changes the new line's prefix only. A
            // strategy's trailing-space cleanup belongs to explicit reindent.
            edits[index]
                .1
                .push_str(&preview.get_text(indentation.offset, indentation.length)?);
        }
        self.replace_segments(&edits, selection.primary_index())
    }
    pub fn insert_tab(&self) -> Result<(), EditError> {
        self.insert_tab_with_virtual_space()
    }

    pub fn indent_selection(&self, unindent: bool) -> Result<(), EditError> {
        let selected_lines = self.selected_lines()?;
        let options = self.options();
        let mut edits = Vec::new();
        for index in selected_lines {
            let line = self.document().get_line_by_number(index)?;
            if !unindent {
                edits.push((
                    SimpleSegment::new(line.offset, 0),
                    options.indentation_string(),
                ));
                continue;
            }
            let prefix = self
                .document()
                .get_text(line.offset, line.length.min(options.indentation_size))?;
            let length = if prefix.starts_with('\t') {
                1
            } else {
                prefix.chars().take_while(|c| *c == ' ').count()
            };
            if length != 0 {
                edits.push((SimpleSegment::new(line.offset, length), String::new()));
            }
        }
        self.apply_edits(&edits)
    }

    /// Reindents the selected logical lines using the installed language
    /// strategy, or the entire document when there is no selected text. All
    /// plans are checked before any real document mutation and committed as one
    /// undo unit. A selection ending at a line start excludes that last line,
    /// matching the Tab/Shift+Tab line-selection behavior.
    pub fn reindent_selection(&self) -> Result<(), EditError> {
        let Some(strategy) = self.indentation_strategy() else {
            return Ok(());
        };
        if self.selection().is_empty() {
            let edits = strategy.indent_lines(self.document(), 1, self.document().line_count())?;
            return self.apply_document_edits(&edits);
        }
        let selected_lines = self.selected_lines()?;
        let mut ranges: Vec<(usize, usize)> = Vec::new();
        for line in selected_lines {
            match ranges.last_mut() {
                Some((_, end)) if *end + 1 == line => *end = line,
                _ => ranges.push((line, line)),
            }
        }
        let mut edits = Vec::new();
        for (begin, end) in ranges {
            edits.extend(strategy.indent_lines(self.document(), begin, end)?);
        }
        self.apply_document_edits(&edits)
    }

    /// Applies a text or line transformation as one undo unit after checking
    /// every affected range. Case and tab conversions use character-preserving
    /// offset mapping so interior carets and anchors survive those conversions.
    /// The planner documents each operation's empty-selection behavior.
    pub fn transform_text(&self, transformation: TextTransformation) -> Result<(), EditError> {
        let edits = plan_text_transformation(
            self.document(),
            &self.selection(),
            &self.options(),
            transformation,
        )?;
        self.validate_document_edits(&edits)?;
        let mapping = transformation.offset_change_mapping();
        if mapping == OffsetChangeMappingType::Normal {
            self.document().apply_edits(&edits)?;
        } else if !edits.is_empty() {
            let _update = self.document().begin_update()?;
            for edit in edits.iter().rev() {
                self.document().replace_with_mapping(
                    edit.offset,
                    edit.length,
                    &edit.text,
                    mapping,
                )?;
            }
        }
        Ok(())
    }

    fn selected_lines(&self) -> Result<BTreeSet<usize>, EditError> {
        let mut selected_lines = BTreeSet::new();
        for segment in self.selection().segments() {
            let first = self
                .document()
                .get_line_by_offset(segment.start_offset())?
                .line_number;
            let end = if segment.length() == 0 || segment.end_virtual_space() != 0 {
                segment.end_offset()
            } else {
                segment.end_offset() - 1
            };
            let last = self.document().get_line_by_offset(end)?.line_number;
            selected_lines.extend(first..=last);
        }
        Ok(selected_lines)
    }
    pub fn undo(&self) -> Result<bool, EditError> {
        if self.is_read_only() {
            return Err(EditError::ReadOnly);
        }
        Ok(self.document().undo()?)
    }
    pub fn redo(&self) -> Result<bool, EditError> {
        if self.is_read_only() {
            return Err(EditError::ReadOnly);
        }
        Ok(self.document().redo()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::DocumentEdit;
    use crate::editing::{CSharpIndentationStrategy, TextSegmentReadOnlySectionProvider};
    #[test]
    fn multicaret_edit_is_single_undo_and_survives_external_insert() {
        let document = TextDocument::from_text("abc def ghi");
        let editor = EditorSession::new(document.clone());
        editor
            .set_selection(Selection::from_segments(
                [SelectionSegment::new(0, 3), SelectionSegment::new(8, 11)],
                0,
            ))
            .unwrap();
        editor.replace_selection_with_text("😀").unwrap();
        assert_eq!(document.text(), "😀 def 😀");
        assert_eq!(
            editor.selection().segments(),
            &[SelectionSegment::empty(2), SelectionSegment::empty(9)]
        );
        assert!(editor.undo().unwrap());
        assert_eq!(document.text(), "abc def ghi");
        document.insert(0, "x").unwrap();
        assert_eq!(editor.selection().segments()[0].caret, 4);
    }
    #[test]
    fn protected_multicaret_edit_is_preflighted_before_mutation() {
        let document = TextDocument::from_text("abc def");
        let editor = EditorSession::new(document.clone());
        let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
        protected.add(0, 3).unwrap();
        editor.set_read_only_section_provider(protected);
        editor
            .set_selection(Selection::from_segments(
                [SelectionSegment::empty(1), SelectionSegment::empty(6)],
                0,
            ))
            .unwrap();
        assert!(matches!(
            editor.replace_selection_with_text("X"),
            Err(EditError::ReadOnly)
        ));
        assert_eq!(document.text(), "abc def");
    }
    #[test]
    fn indent_excludes_line_at_selection_end_and_newline_preserves_indentation() {
        let document = TextDocument::from_text("a\nb\n");
        let editor = EditorSession::new(document.clone());
        editor.select(0, 2).unwrap();
        editor.indent_selection(false).unwrap();
        assert_eq!(document.text(), "\ta\nb\n");
        editor.set_caret_offset(2).unwrap();
        editor.insert_new_line().unwrap();
        assert_eq!(document.text(), "\ta\n\t\nb\n");
    }

    #[test]
    fn newline_replaces_existing_indentation_and_can_disable_the_strategy() {
        let document = TextDocument::from_text("    value😀\r\n");
        let editor = EditorSession::new(document.clone());
        editor.set_options(EditorOptions {
            newline: "\r\n".into(),
            ..EditorOptions::default()
        });
        editor.set_caret_offset(2).unwrap();
        editor.insert_new_line().unwrap();
        assert_eq!(document.text(), "  \r\n  value😀\r\n");
        assert_eq!(editor.caret_offset(), 6);
        assert!(editor.undo().unwrap());
        assert_eq!(document.text(), "    value😀\r\n");
        editor.set_indentation_strategy(None);
        editor.set_caret_offset(2).unwrap();
        editor.insert_new_line().unwrap();
        assert_eq!(document.text(), "  \r\n  value😀\r\n");
        assert_eq!(editor.caret_offset(), 4);
    }

    #[test]
    fn language_newline_uses_context_and_notifies_only_after_planning() {
        let document = TextDocument::from_text("if (ready) {");
        let editor = EditorSession::new(document.clone());
        editor.set_indentation_strategy(Some(Rc::new(
            CSharpIndentationStrategy::with_indentation_string("    ").unwrap(),
        )));
        editor.set_caret_offset(document.text_length()).unwrap();
        let observed = Rc::new(RefCell::new(Vec::new()));
        let output = observed.clone();
        let changed_document = document.clone();
        let subscription = document.changed(move |_| {
            output.borrow_mut().push(changed_document.text());
        });
        editor.insert_new_line().unwrap();
        assert_eq!(document.text(), "if (ready) {\n    ");
        assert_eq!(editor.caret_offset(), document.text_length());
        assert_eq!(&*observed.borrow(), &["if (ready) {\n    "]);
        assert!(editor.undo().unwrap());
        assert_eq!(document.text(), "if (ready) {");
        subscription.dispose();
    }

    #[test]
    fn multicaret_newlines_share_prospective_context_and_one_undo_group() {
        let document = TextDocument::from_text("      value");
        let editor = EditorSession::new(document.clone());
        editor
            .set_selection(Selection::from_segments(
                [SelectionSegment::empty(2), SelectionSegment::empty(4)],
                1,
            ))
            .unwrap();
        editor.insert_new_line().unwrap();
        assert_eq!(document.text(), "  \n  \n  value");
        assert_eq!(
            editor.selection().segments(),
            &[SelectionSegment::empty(5), SelectionSegment::empty(8)]
        );
        assert_eq!(editor.selection().primary_index(), 1);
        assert!(editor.undo().unwrap());
        assert_eq!(document.text(), "      value");
        assert!(!document.can_undo());
    }

    #[test]
    fn newline_preflights_indentation_ranges_before_mutating_any_caret() {
        let document = TextDocument::from_text("  a\n  b");
        let editor = EditorSession::new(document.clone());
        let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
        protected.add(5, 1).unwrap();
        editor.set_read_only_section_provider(protected);
        editor
            .set_selection(Selection::from_segments(
                [SelectionSegment::empty(1), SelectionSegment::empty(5)],
                0,
            ))
            .unwrap();
        let selection = editor.selection();
        assert!(matches!(editor.insert_new_line(), Err(EditError::ReadOnly)));
        assert_eq!(document.text(), "  a\n  b");
        assert_eq!(editor.selection(), selection);
        assert!(!document.can_undo());
    }

    #[test]
    fn indentation_failure_does_not_insert_a_newline_or_change_the_selection() {
        struct RejectIndentation;
        impl IndentationStrategy for RejectIndentation {
            fn indent_line(
                &self,
                _: &TextDocument,
                _: usize,
            ) -> Result<Vec<DocumentEdit>, IndentationError> {
                Err(IndentationError::InvalidAccessorState)
            }
            fn indent_lines(
                &self,
                _: &TextDocument,
                _: usize,
                _: usize,
            ) -> Result<Vec<DocumentEdit>, IndentationError> {
                Err(IndentationError::InvalidAccessorState)
            }
        }
        let document = TextDocument::from_text("  selected");
        let editor = EditorSession::new(document.clone());
        editor.set_indentation_strategy(Some(Rc::new(RejectIndentation)));
        editor.select(2, 8).unwrap();
        let selection = editor.selection();
        assert!(matches!(
            editor.insert_new_line(),
            Err(EditError::Indentation(
                IndentationError::InvalidAccessorState
            ))
        ));
        assert_eq!(document.text(), "  selected");
        assert_eq!(editor.selection(), selection);
        assert!(!document.can_undo());
    }

    #[test]
    fn reindent_uses_selected_lines_and_preserves_the_unselected_suffix() {
        let document = TextDocument::from_text("class C {\nvoid M() {\nCall();\n}\n}\n");
        let editor = EditorSession::new(document.clone());
        editor.set_indentation_strategy(Some(Rc::new(
            CSharpIndentationStrategy::with_indentation_string("  ").unwrap(),
        )));
        let second = document.get_line_by_number(2).unwrap().offset;
        let fourth = document.get_line_by_number(4).unwrap().offset;
        editor.select(second, fourth - second).unwrap();
        editor.reindent_selection().unwrap();
        assert_eq!(
            document.text(),
            "class C {\n  void M() {\n    Call();\n}\n}\n"
        );
        assert!(editor.undo().unwrap());
        assert_eq!(document.text(), "class C {\nvoid M() {\nCall();\n}\n}\n");
        assert!(!document.can_undo());
    }

    #[test]
    fn reindent_preflights_every_line_and_keeps_protected_text_intact() {
        let document = TextDocument::from_text("class C {\nvoid M() {\nCall();\n}\n}");
        let editor = EditorSession::new(document.clone());
        editor.set_indentation_strategy(Some(Rc::new(CSharpIndentationStrategy::new())));
        let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
        let third = document.get_line_by_number(3).unwrap().offset;
        protected.add(third - 1, 8).unwrap();
        editor.set_read_only_section_provider(protected);
        editor.select_all();
        let original = document.text();
        assert!(matches!(
            editor.reindent_selection(),
            Err(EditError::ReadOnly)
        ));
        assert_eq!(document.text(), original);
        assert!(!document.can_undo());
    }

    #[test]
    fn reindent_without_selected_text_formats_the_entire_document() {
        let document = TextDocument::from_text("class C {\nvoid M() {\nCall();\n}\n}");
        let editor = EditorSession::new(document.clone());
        editor.set_indentation_strategy(Some(Rc::new(CSharpIndentationStrategy::new())));
        editor.set_caret_offset(1).unwrap();
        editor.reindent_selection().unwrap();
        assert_eq!(
            document.text(),
            "class C {\n\tvoid M() {\n\t\tCall();\n\t}\n}"
        );
        assert_eq!(editor.caret_offset(), 1);
        assert!(editor.undo().unwrap());
        assert_eq!(document.text(), "class C {\nvoid M() {\nCall();\n}\n}");
    }

    #[test]
    fn transformations_preserve_interior_character_anchors_and_selection() {
        let document = TextDocument::from_text("abc\tdef");
        let editor = EditorSession::new(document.clone());
        let middle = document.create_anchor(1).unwrap();
        editor.set_caret_offset(2).unwrap();
        editor
            .transform_text(TextTransformation::Uppercase)
            .unwrap();
        assert_eq!(document.text(), "ABC\tDEF");
        assert_eq!(middle.offset().unwrap(), 1);
        assert_eq!(editor.caret_offset(), 2);
        editor
            .transform_text(TextTransformation::TabsToSpaces)
            .unwrap();
        assert_eq!(document.text(), "ABC    DEF");
        assert_eq!(middle.offset().unwrap(), 1);
        assert_eq!(editor.caret_offset(), 2);
        assert!(editor.undo().unwrap());
        assert_eq!(document.text(), "ABC\tDEF");
        assert!(editor.undo().unwrap());
        assert_eq!(document.text(), "abc\tdef");
    }

    #[test]
    fn transformation_rejects_protected_changes_before_creating_an_undo_group() {
        let document = TextDocument::from_text("ab cd");
        let editor = EditorSession::new(document.clone());
        let provider = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
        provider.add(3, 2).unwrap();
        editor.set_read_only_section_provider(provider);
        editor.set_caret_offset(1).unwrap();
        assert!(matches!(
            editor.transform_text(TextTransformation::Uppercase),
            Err(EditError::ReadOnly)
        ));
        assert_eq!(document.text(), "ab cd");
        assert_eq!(editor.caret_offset(), 1);
        assert!(!document.can_undo());
    }
}
