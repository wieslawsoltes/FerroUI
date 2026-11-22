mod rectangular_enter;
mod rectangular_paste;
mod virtual_space;

#[cfg(test)]
mod preflight_tests;

use super::{
    CaretMovement, DefaultIndentationStrategy, IReadOnlySectionProvider, IndentationError,
    IndentationStrategy, LineHeightFactor, NoReadOnlySections, Selection, SelectionSegment, TextTransformation,
    next_caret_position,
};
use super::text_transformations::plan_text_transformation_with;
use crate::document::{
    AnchorMovementType, DocumentEdit, DocumentError, DocumentSubscription, DocumentVersion,
    IUndoableOperation, OffsetChangeMappingType, SimpleSegment, TextAnchor, TextDocument, TextSnapshot, TextUtilities, UpdateGuard,
};
use crate::unicode_case::CaseLocale;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::{Rc, Weak};

ferroui_system::styled_value!(EditorOptions);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorOptions {
    pub indentation_size: usize,
    pub convert_tabs_to_spaces: bool,
    /// Lets the default input handler use Tab for editing instead of focus navigation.
    pub accepts_tab: bool,
    /// Adjusts indentation for ordinary single-selection clipboard paste.
    pub auto_indent_on_paste: bool,
    /// Allows the paste indentation planner to adjust string content.
    pub auto_indent_on_paste_within_string: bool,
    /// Copies or cuts the current logical line when there is one empty caret.
    pub cut_copy_whole_line: bool,
    pub enable_text_drag_drop: bool,
    pub enable_rectangular_selection: bool,
    pub extend_selection_on_mouse_up: bool,
    pub allow_toggle_overstrike_mode: bool,
    pub enable_virtual_space: bool,
    pub allow_scroll_below_document: bool,
    pub show_spaces: bool,
    pub space_glyph: String,
    pub show_tabs: bool,
    pub tab_glyph: String,
    pub show_end_of_line: bool,
    pub end_of_line_crlf_glyph: String,
    pub end_of_line_cr_glyph: String,
    pub end_of_line_lf_glyph: String,
    pub show_box_for_control_characters: bool,
    pub enable_hyperlinks: bool,
    pub enable_email_hyperlinks: bool,
    pub require_control_modifier_for_hyperlink_click: bool,
    pub highlight_current_line: bool,
    /// Draws column guides at the configured wide-space columns.
    pub show_column_rulers: bool,
    pub column_ruler_positions: Vec<i32>,
    pub line_height_factor: LineHeightFactor,
    pub newline: String,
}
impl Default for EditorOptions {
    fn default() -> Self {
        Self {
            indentation_size: 4,
            convert_tabs_to_spaces: false,
            accepts_tab: true,
            auto_indent_on_paste: false,
            auto_indent_on_paste_within_string: true,
            cut_copy_whole_line: true,
            enable_text_drag_drop: true,
            enable_rectangular_selection: true,
            extend_selection_on_mouse_up: true,
            allow_toggle_overstrike_mode: false,
            enable_virtual_space: false,
            allow_scroll_below_document: false,
            show_spaces: false,
            space_glyph: "·".into(),
            show_tabs: false,
            tab_glyph: "→".into(),
            show_end_of_line: false,
            end_of_line_crlf_glyph: "¶".into(),
            end_of_line_cr_glyph: "\\r".into(),
            end_of_line_lf_glyph: "\\n".into(),
            show_box_for_control_characters: true,
            enable_hyperlinks: true,
            enable_email_hyperlinks: true,
            require_control_modifier_for_hyperlink_click: true,
            highlight_current_line: false,
            show_column_rulers: false,
            column_ruler_positions: vec![80],
            line_height_factor: LineHeightFactor::default(),
            newline: "\n".into(),
        }
    }
}
impl EditorOptions {
    pub fn indentation_string(&self) -> String {
        self.indentation_at_column(0)
    }

    fn indentation_at_column(&self, column: usize) -> String {
        if self.convert_tabs_to_spaces {
            let size = self.indentation_size.clamp(1, 256);
            " ".repeat(size - column % size)
        } else {
            "\t".into()
        }
    }
}

/// Text replacing a selection on Enter. Both parts are included in the same undo unit.
pub struct LineBreakInsertion {
    pub before_caret: String,
    pub after_caret: String,
}

/// A planned Enter operation. Indentation replacement consumes only the
/// remaining leading whitespace on the line containing the selection end.
pub enum LineBreakPlan {
    Insert(LineBreakInsertion),
    ReplaceIndentation(LineBreakInsertion),
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
    undo_subscription: DocumentSubscription,
}
impl Drop for SessionInner {
    fn drop(&mut self) {
        self.subscription.dispose();
        self.undo_subscription.dispose();
    }
}

/// The editing state shared by a text area and its commands. All positions use
/// UTF-16. A session can be cloned without copying the document or selections.
#[derive(Clone)]
pub struct EditorSession {
    inner: Rc<SessionInner>,
}

struct RestoreSelection {
    session: Weak<SessionInner>,
    selection: Selection,
    redo_only: bool,
}

impl RestoreSelection {
    fn restore(&self) {
        if let Some(inner) = self.session.upgrade() {
            let session = EditorSession { inner };
            let _ = session.set_selection(self.selection.clone());
        }
    }
}

impl IUndoableOperation for RestoreSelection {
    fn undo(&self) {
        if !self.redo_only {
            self.restore();
        }
    }

    fn redo(&self) {
        self.restore();
    }
}

impl EditorSession {
    pub fn new(document: TextDocument) -> Self {
        let inner = Rc::new_cyclic(|weak: &Weak<SessionInner>| {
            let listener = weak.clone();
            let subscription = document.changed(move |change| {
                if let Some(inner) = listener.upgrade() {
                    let updated = inner.selection.borrow().update_on_document_change(change);
                    let updated = virtual_space::normalize_selection(&inner.document, updated);
                    *inner.selection.borrow_mut() = updated;
                    inner.desired_columns.borrow_mut().clear();
                }
            });
            let listener = weak.clone();
            let undo_subscription = document.undo_group_started(move || {
                if let Some(inner) = listener.upgrade() {
                    let selection = inner.selection.borrow().clone();
                    let _ = inner.document.undo_stack().push_optional(Rc::new(RestoreSelection {
                        session: listener.clone(),
                        selection,
                        redo_only: false,
                    }));
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
                undo_subscription,
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
    /// Returns the policy used by editor commands to check writable ranges.
    pub fn read_only_section_provider(&self) -> Rc<dyn IReadOnlySectionProvider> {
        self.inner.read_only_sections.borrow().clone()
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

    /// Replaces the writable parts of an ordinary selection or each rectangle
    /// row. Protected text stays in place and the last writable span receives
    /// the replacement. Independent multicaret edits use the atomic policy.
    pub fn replace_writable_selection_with_text(&self, text: &str) -> Result<(), EditError> {
        self.replace_writable_selection_with_text_checked(text, || true)
    }

    /// Allows the host to reject the target if policy or update-start
    /// callbacks change the document displayed by the view.
    pub fn replace_writable_selection_with_text_checked(
        &self,
        text: &str,
        is_current: impl Fn() -> bool,
    ) -> Result<(), EditError> {
        let validate_host = || {
            if is_current() {
                Ok(())
            } else {
                Err(EditError::StaleEditPlan)
            }
        };
        let selection = {
            let current = self.inner.selection.borrow();
            if current.is_rectangular() {
                drop(current);
                return self.replace_writable_rectangle(text, is_current);
            }
            if current.segments().len() != 1 || current.primary().length() == 0 {
                drop(current);
                return self.replace_virtual_selections(&[text.to_owned()], false, validate_host);
            }
            current.clone()
        };
        let selected = selection.primary();
        let range = selected.segment();
        if self.is_read_only() || self.document().is_read_only() {
            return Err(EditError::ReadOnly);
        }

        let version = self.document().version();
        let provider = self.read_only_section_provider();
        let validate_target = || {
            validate_host()?;
            self.validate_protected_edit_context(&selection, &version, &provider)
        };
        let writable = Self::writable_parts(provider.as_ref(), range);
        validate_target()?;
        let writable = writable?;
        if writable.as_slice() == [range] {
            return self.replace_virtual_selections(&[text.to_owned()], false, validate_target);
        }
        let Some((last, earlier)) = writable.split_last() else {
            return Ok(());
        };
        let mut edits: Vec<_> = earlier
            .iter()
            .filter(|segment| segment.length != 0)
            .map(|segment| DocumentEdit::new(segment.offset, segment.length, ""))
            .collect();
        if last.length != 0 || !text.is_empty() {
            edits.push(DocumentEdit::new(last.offset, last.length, text));
        }
        let update = self.begin_prepared_update(&edits, &version)?;
        validate_target()?;
        let (caret, virtual_space) = if text.is_empty() {
            (range.offset, selected.start_virtual_space())
        } else {
            let removed: usize = earlier.iter().map(|segment| segment.length).sum();
            (last.offset - removed + text.encode_utf16().count(), 0)
        };
        self.document().apply_edits(&edits)?;
        let caret = SelectionSegment::with_virtual_space(caret, virtual_space, caret, virtual_space);
        let selection = Selection::from_segments([caret], 0);
        self.set_selection(virtual_space::normalize_selection(self.document(), selection))?;
        if let Some(update) = update {
            update.finish()?;
        }
        Ok(())
    }

    /// Plans deletions of the writable spans in a fixed document range.
    pub fn writable_deletions(&self, range: SimpleSegment) -> Result<Vec<DocumentEdit>, EditError> {
        let length = self.document().text_length();
        if range.offset > length || range.length > length - range.offset {
            return Err(EditError::InvalidSelection);
        }
        let selection = self.selection();
        let version = self.document().version();
        let provider = self.read_only_section_provider();
        self.validate_protected_edit_context(&selection, &version, &provider)?;
        let parts = Self::writable_parts(provider.as_ref(), range);
        self.validate_protected_edit_context(&selection, &version, &provider)?;
        Ok(parts?
            .into_iter()
            .filter(|part| part.length != 0)
            .map(|part| DocumentEdit::new(part.offset, part.length, ""))
            .collect())
    }

    fn writable_parts(
        provider: &dyn IReadOnlySectionProvider,
        range: SimpleSegment,
    ) -> Result<Vec<SimpleSegment>, EditError> {
        let writable = provider.get_deletable_segments(range);
        let mut end = range.offset;
        for segment in &writable {
            if segment.offset < end
                || segment.offset > range.end_offset()
                || segment.length > range.end_offset() - segment.offset
            {
                return Err(EditError::InvalidSelection);
            }
            end = segment.end_offset();
        }
        Ok(writable)
    }

    fn validate_protected_edit_context(
        &self,
        selection: &Selection,
        version: &DocumentVersion,
        provider: &Rc<dyn IReadOnlySectionProvider>,
    ) -> Result<(), EditError> {
        self.ensure_document_version(version)?;
        if &*self.inner.selection.borrow() != selection
            || !Rc::ptr_eq(provider, &self.read_only_section_provider())
        {
            return Err(EditError::StaleEditPlan);
        }
        if self.is_read_only() || self.document().is_read_only() {
            return Err(EditError::ReadOnly);
        }
        Ok(())
    }

    /// Replaces the next character at each empty caret. The line delimiter and
    /// virtual cells are insertion points. A nonempty selection is replaced.
    pub fn overstrike_with_text(&self, text: &str) -> Result<(), EditError> {
        self.replace_virtual_selections(&[text.to_owned()], true, || Ok(()))
    }
    pub fn replace_selections(&self, replacements: &[String]) -> Result<(), EditError> {
        self.replace_virtual_selections(replacements, false, || Ok(()))
    }

    fn validate_ranges(
        &self,
        edits: impl IntoIterator<Item = (SimpleSegment, bool)>,
    ) -> Result<(), EditError> {
        let version = self.document().version();
        let result = (|| {
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
        })();
        self.ensure_document_version(&version)?;
        // A provider is host code and may change global protection while its
        // insertion/deletion callbacks are running.
        if self.is_read_only() || self.document().is_read_only() {
            return Err(EditError::ReadOnly);
        }
        result
    }
    /// Applies a precomputed edit plan using original document coordinates.
    /// All protected ranges are checked before the document validates overlaps
    /// and commits the plan as one undo unit.
    pub fn apply_document_edits(
        &self,
        edits: &[crate::document::DocumentEdit],
    ) -> Result<(), EditError> {
        self.apply_mapped_document_edits(
            edits,
            OffsetChangeMappingType::Normal,
            &self.document().version(),
            || Ok(()),
        )
    }

    /// Commits original-coordinate edits and a physical selection expressed in
    /// the resulting document, together in one undo unit. The host can reject
    /// a plan after update-start callbacks change its document or view binding.
    pub fn apply_document_edits_and_select(
        &self,
        edits: &[DocumentEdit],
        selection: Selection,
        is_current: impl Fn() -> bool,
    ) -> Result<(), EditError> {
        let original = self.selection();
        let version = self.document().version();
        if !is_current() {
            return Err(EditError::StaleEditPlan);
        }
        let mut length = self.document().text_length();
        for edit in edits {
            length = length.checked_sub(edit.length)
                .and_then(|remaining| remaining.checked_add(edit.text.encode_utf16().count()))
                .ok_or(EditError::InvalidSelection)?;
        }
        if selection.segments().iter().any(|segment| {
            segment.end_offset() > length || segment.start_virtual_space() != 0 || segment.end_virtual_space() != 0
        }) {
            return Err(EditError::InvalidSelection);
        }
        let update = self.begin_prepared_update(edits, &version)?;
        if !is_current() || self.selection() != original {
            return Err(EditError::StaleEditPlan);
        }
        if let Some(update) = update {
            self.document().apply_edits(edits)?;
            // Replay maps the original selection through the text changes.
            // Explicit placements, such as a caret before generated text,
            // are applied after those changes on redo.
            if self.selection() != selection {
                self.document().undo_stack().push_optional(Rc::new(RestoreSelection {
                    session: Rc::downgrade(&self.inner),
                    selection: selection.clone(),
                    redo_only: true,
                }))?;
            }
            self.set_selection(selection)?;
            update.finish()?;
        } else {
            self.set_selection(selection)?;
        }
        Ok(())
    }

    fn apply_mapped_document_edits(
        &self,
        edits: &[DocumentEdit],
        mapping: OffsetChangeMappingType,
        version: &DocumentVersion,
        validate: impl FnOnce() -> Result<(), EditError>,
    ) -> Result<(), EditError> {
        let update = self.begin_prepared_update(edits, version)?;
        validate()?;
        if let Some(update) = update {
            self.document().apply_edits_with_mapping(edits, mapping)?;
            update.finish()?;
        }
        Ok(())
    }

    fn ensure_document_version(&self, expected: &DocumentVersion) -> Result<(), EditError> {
        if self.document().version().revision() != expected.revision() {
            return Err(EditError::StaleEditPlan);
        }
        Ok(())
    }

    /// Keeps an already prepared plan in its original revision across both
    /// provider callbacks and the outer update-start notification.
    fn begin_prepared_update(
        &self,
        edits: &[DocumentEdit],
        version: &DocumentVersion,
    ) -> Result<Option<UpdateGuard>, EditError> {
        self.ensure_document_version(version)?;
        let mut ranges: Vec<_> = edits
            .iter()
            .map(|edit| (edit.offset, edit.length))
            .collect();
        ranges.sort_unstable();
        if ranges
            .windows(2)
            .any(|pair| pair[0].0.saturating_add(pair[0].1) > pair[1].0)
        {
            return Err(DocumentError::OverlappingEdits.into());
        }
        self.validate_document_edits(edits)?;
        if edits.is_empty() {
            return Ok(None);
        }
        let update = self.document().begin_update()?;
        self.ensure_document_version(version)?;
        self.validate_document_edits(edits)?;
        Ok(Some(update))
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
        self.delete_checked(backward, by_word, |_| Ok(None), || true)
    }

    /// Deletes relative to each caret, optionally using a formatted view's
    /// physical destination. The live selection remains in place until the
    /// edit commits so undo restores the original carets.
    pub fn delete_checked(
        &self,
        backward: bool,
        by_word: bool,
        visual_destination: impl Fn(SelectionSegment) -> Result<Option<usize>, EditError>,
        is_current: impl Fn() -> bool,
    ) -> Result<(), EditError> {
        let deletes_selection = {
            let selection = self.inner.selection.borrow();
            if selection.is_rectangular() {
                !selection.is_empty()
            } else {
                selection.segments().len() == 1
                    && selection.primary().length() != 0
                    && selection.primary().start_virtual_space() != 0
            }
        };
        if deletes_selection {
            return self.replace_writable_selection_with_text_checked("", is_current);
        }
        self.delete_with_virtual_space(backward, by_word, visual_destination, is_current)
    }

    pub fn insert_new_line(&self) -> Result<(), EditError> {
        self.insert_line_breaks(false, |_, _, _| Ok(None), || true)
    }

    /// Inserts a line break in the style of each affected document line.
    /// A document without a delimiter uses the configured newline.
    pub fn insert_document_new_line(&self) -> Result<(), EditError> {
        self.insert_document_new_line_with(|_, _, _| Ok(None))
    }

    /// Lets the host plan Enter for ordinary selections against the original
    /// snapshot. Returning `None` uses the session's indentation strategy.
    /// A rectangular selection uses bounded replacement and indents its final caret line.
    /// An absent indentation strategy, protected string whitespace and virtual
    /// cells retain their normal Enter behavior.
    pub fn insert_document_new_line_with(
        &self,
        plan: impl Fn(&TextSnapshot, SelectionSegment, &str) -> Result<Option<LineBreakInsertion>, EditError>,
    ) -> Result<(), EditError> {
        self.insert_document_new_line_with_plan(|source, segment, newline| {
            Ok(plan(source, segment, newline)?.map(LineBreakPlan::Insert))
        })
    }

    pub fn insert_document_new_line_with_plan(
        &self,
        plan: impl Fn(&TextSnapshot, SelectionSegment, &str) -> Result<Option<LineBreakPlan>, EditError>,
    ) -> Result<(), EditError> {
        self.insert_document_new_line_with_plan_checked(plan, || true)
    }

    /// Inserts and indents a line break while the host still owns the edit target.
    /// The host is checked again after document update callbacks and before commit.
    pub fn insert_document_new_line_with_plan_checked(
        &self,
        plan: impl Fn(&TextSnapshot, SelectionSegment, &str) -> Result<Option<LineBreakPlan>, EditError>,
        is_current: impl Fn() -> bool,
    ) -> Result<(), EditError> {
        self.insert_line_breaks(true, plan, is_current)
    }

    fn insert_line_breaks(
        &self,
        document_endings: bool,
        plan: impl Fn(&TextSnapshot, SelectionSegment, &str) -> Result<Option<LineBreakPlan>, EditError>,
        is_current: impl Fn() -> bool,
    ) -> Result<(), EditError> {
        let validate_host = || is_current().then_some(()).ok_or(EditError::StaleEditPlan);
        validate_host()?;
        let version = self.document().version();
        let selection = self.selection();
        self.validate_virtual_edit_positions(&selection)?;
        let configured_newline = self.options().newline;
        if selection.is_rectangular() {
            let newline = if document_endings && self.document().line_count() > 1 {
                let line = self.document().get_line_by_offset(selection.primary().caret)?;
                TextUtilities::get_new_line_from_document(self.document(), line.line_number())?
            } else {
                configured_newline
            };
            return self.insert_rectangular_line_break(&selection, &newline, validate_host);
        }
        let newlines: Vec<_> = if document_endings && self.document().line_count() > 1 {
            selection.segments().iter()
                .map(|segment| {
                    let line = self.document().get_line_by_offset(segment.caret)?;
                    TextUtilities::get_new_line_from_document(self.document(), line.line_number())
                })
                .collect::<Result<_, _>>()?
        } else {
            vec![configured_newline; selection.segments().len()]
        };
        let strategy = self.indentation_strategy();
        let provider = self.read_only_section_provider();
        let validate_target = || {
            validate_host()?;
            self.validate_protected_edit_context(&selection, &version, &provider)
        };
        let source = self.document().create_snapshot();
        let primary = selection.primary();
        let mut indent_writable_line = true;
        if selection.segments().len() == 1
            && primary.start_virtual_space() == 0
            && primary.end_virtual_space() == 0
        {
            let range = primary.segment();
            let writable = if range.length == 0 {
                Ok((!provider.can_insert(range.offset)).then(Vec::new))
            } else {
                Self::writable_parts(provider.as_ref(), range)
                    .map(|parts| (parts.as_slice() != [range]).then_some(parts))
            };
            validate_target()?;
            if let Some(writable) = writable? {
                return self.insert_protected_line_break(
                    source, &selection, &provider, &writable, &newlines[0], validate_target,
                );
            }
            if strategy.is_some() {
                indent_writable_line = Self::line_tail_is_writable(
                    &source,
                    provider.as_ref(),
                    primary.end_offset(),
                )?;
                validate_target()?;
            }
        }
        let Some(strategy) = strategy else {
            return self.replace_virtual_selections(&newlines, false, validate_target);
        };
        let mut edits = Vec::with_capacity(selection.segments().len());
        let mut automatic_indent = Vec::with_capacity(selection.segments().len());
        for (index, (&segment, newline)) in selection.segments().iter().zip(newlines).enumerate() {
            let insertion = segment.end_offset();
            let preserve = !indent_writable_line
                || strategy.preserve_new_line_whitespace(&source, insertion)?;
            let custom = if !preserve && segment.start_virtual_space() == 0 && segment.end_virtual_space() == 0 {
                plan(&source, segment, &newline)?
            } else {
                None
            };
            let indent = !preserve && custom.is_none();
            let end = match &custom {
                None if indent => {
                    TextUtilities::get_whitespace_after(&source, insertion)?.end_offset()
                }
                Some(LineBreakPlan::ReplaceIndentation(_)) => {
                    let line = source.get_line_by_offset(insertion)?;
                    TextUtilities::get_whitespace_after(&source, line.offset)?
                        .end_offset()
                        .max(insertion)
                }
                _ => insertion,
            };
            // A later caret owns the whitespace following its own selection.
            let limit = selection.segments().get(index + 1)
                .map_or(source.text_length(), |next| next.start_offset());
            let end = end.min(limit);
            let (text, caret_from_end) = match custom {
                Some(
                    LineBreakPlan::Insert(mut custom) | LineBreakPlan::ReplaceIndentation(mut custom),
                ) => {
                    let trailing = custom.after_caret.encode_utf16().count();
                    custom.before_caret.push_str(&custom.after_caret);
                    (custom.before_caret, trailing)
                }
                None => (newline, 0),
            };
            edits.push(virtual_space::PlannedReplacement {
                segment: SimpleSegment::new(segment.start_offset(), end - segment.start_offset()),
                text,
                virtual_space: 0,
                caret_from_end,
            });
            automatic_indent.push(indent);
        }
        self.ensure_document_version(&version)?;
        self.validate_ranges(edits.iter().map(|edit| (edit.segment, !edit.text.is_empty())))?;

        // A persistent snapshot shares the source rope. Strategies observe the
        // complete prospective document without mutating the real document or
        // exposing partially formatted text to its change handlers.
        let preview = TextDocument::from_snapshot(source);
        let mut lines = Vec::with_capacity(selection.segments().len());
        for ((segment, edit), indent) in selection.segments().iter().zip(&edits).zip(automatic_indent).rev() {
            let length = if indent { segment.length() } else { edit.segment.length };
            preview.replace(segment.start_offset(), length, &edit.text)?;
            if !indent {
                lines.push(None);
                continue;
            }
            let anchor = preview
                .create_anchor(segment.start_offset() + edit.text.encode_utf16().count())?;
            anchor.set_movement_type(AnchorMovementType::AfterInsertion);
            anchor.set_survive_deletion(true);
            lines.push(Some(anchor));
        }
        lines.reverse();
        for (index, anchor) in lines.iter().enumerate() {
            let Some(anchor) = anchor else { continue };
            edits[index]
                .text
                .push_str(&Self::line_break_indentation(strategy.as_ref(), &preview, anchor)?);
        }
        validate_target()?;
        self.execute_replacements(edits, selection.primary_index(), false, false, validate_target)
    }

    fn line_tail_is_writable(
        source: &TextSnapshot,
        provider: &dyn IReadOnlySectionProvider,
        offset: usize,
    ) -> Result<bool, EditError> {
        let line = source.get_line_by_offset(offset)?;
        let tail = SimpleSegment::new(offset, line.end_offset().saturating_sub(offset));
        Ok(Self::writable_parts(provider, tail)?.as_slice() == [tail])
    }

    fn line_break_indentation(
        strategy: &dyn IndentationStrategy,
        preview: &TextDocument,
        anchor: &TextAnchor,
    ) -> Result<String, EditError> {
        let line = preview.get_line_by_offset(anchor.offset()?)?;
        preview.apply_edits(&strategy.indent_line(preview, line.line_number)?)?;
        let line = preview.get_line_by_offset(anchor.offset()?)?;
        let indentation = TextUtilities::get_whitespace_after(preview, line.offset)?;
        // Automatic indentation only transfers the new line's prefix.
        Ok(preview.get_text(indentation.offset, indentation.length)?)
    }

    fn insert_protected_line_break(
        &self,
        source: TextSnapshot,
        selection: &Selection,
        provider: &Rc<dyn IReadOnlySectionProvider>,
        writable: &[SimpleSegment],
        newline: &str,
        validate_target: impl Fn() -> Result<(), EditError>,
    ) -> Result<(), EditError> {
        let version = self.document().version();
        let strategy = self.indentation_strategy();
        let Some((last, earlier)) = writable.split_last() else {
            let Some(strategy) = strategy else { return Ok(()) };
            let line = source.get_line_by_offset(selection.primary().caret)?;
            if !Self::line_tail_is_writable(&source, provider.as_ref(), line.offset)? {
                validate_target()?;
                return Ok(());
            }
            let preview = TextDocument::from_snapshot(source);
            let edits = strategy.indent_line(&preview, line.line_number)?;
            validate_target()?;
            if let Some(update) = self.begin_prepared_update(&edits, &version)? {
                validate_target()?;
                let segment = selection.primary();
                let protected_selection = if segment.is_empty() {
                    None
                } else {
                    let start = self.document().create_anchor(segment.start_offset())?;
                    start.set_survive_deletion(true);
                    let end = self.document().create_anchor(segment.end_offset())?;
                    end.set_movement_type(AnchorMovementType::BeforeInsertion);
                    end.set_survive_deletion(true);
                    Some((start, end))
                };
                self.document().apply_edits_with_mapping(&edits, OffsetChangeMappingType::RemoveAndInsert)?;
                if let Some((start, end)) = protected_selection {
                    let start = start.offset()?;
                    let end = end.offset()?.max(start);
                    let retained = if segment.is_reversed() {
                        Selection::simple(end, start)
                    } else {
                        Selection::simple(start, end)
                    };
                    self.set_selection(retained)?;
                }
                update.finish()?;
            }
            return Ok(());
        };

        let mut edits: Vec<_> = earlier
            .iter()
            .filter(|segment| segment.length != 0)
            .map(|segment| DocumentEdit::new(segment.offset, segment.length, ""))
            .collect();
        edits.push(DocumentEdit::new(last.offset, last.length, newline));
        let removed: usize = earlier.iter().map(|segment| segment.length).sum();
        let mut caret = last.offset - removed + newline.encode_utf16().count();
        if let Some(strategy) = strategy {
            let tail = last.end_offset();
            if Self::line_tail_is_writable(&source, provider.as_ref(), tail)?
                && !strategy.preserve_new_line_whitespace(&source, tail)?
            {
                let whitespace = TextUtilities::get_whitespace_after(&source, tail)?;
                let preview = TextDocument::from_snapshot(source);
                preview.apply_edits(&edits)?;
                let anchor = preview.create_anchor(caret)?;
                anchor.set_movement_type(AnchorMovementType::AfterInsertion);
                anchor.set_survive_deletion(true);
                let indentation = Self::line_break_indentation(strategy.as_ref(), &preview, &anchor)?;
                let replacement = edits.last_mut().expect("the last writable span holds the line break");
                replacement.length += whitespace.length;
                replacement.text.push_str(&indentation);
                caret += indentation.encode_utf16().count();
            }
        }
        validate_target()?;
        let update = self.begin_prepared_update(&edits, &version)?;
        validate_target()?;
        self.document().apply_edits(&edits)?;
        self.set_caret_offset(caret)?;
        if let Some(update) = update {
            update.finish()?;
        }
        Ok(())
    }
    pub fn insert_tab(&self) -> Result<(), EditError> {
        self.insert_tab_checked(self.caret_offset(), || true)
    }

    /// Inserts indentation at the active caret column, or indents selected
    /// lines. The view can supply the physical caret inside an active selection.
    pub fn insert_tab_checked(
        &self,
        active_caret: usize,
        is_current: impl Fn() -> bool,
    ) -> Result<(), EditError> {
        self.insert_tab_with_virtual_space(active_caret, is_current)
    }

    pub fn indent_selection(&self, unindent: bool) -> Result<(), EditError> {
        self.indent_selection_checked(unindent, || true)
    }

    /// Edits the writable indentation of selected lines in a single undo group.
    pub fn indent_selection_checked(
        &self,
        unindent: bool,
        is_current: impl Fn() -> bool,
    ) -> Result<(), EditError> {
        let document = self.document();
        let version = document.version();
        let selection = self.selection();
        let provider = self.read_only_section_provider();
        let options = self.options();
        let validate = || {
            if !is_current() || self.options() != options {
                return Err(EditError::StaleEditPlan);
            }
            self.validate_protected_edit_context(&selection, &version, &provider)
        };
        validate()?;
        let ranges = if unindent && selection.is_rectangle_without_text() {
            let line = document.get_line_by_offset(selection.primary().caret)?.line_number;
            vec![(line, line)]
        } else {
            self.selected_line_ranges(false)?
        };
        let selected_lines = ranges.into_iter().flat_map(|(begin, end)| begin..=end);
        let indentation = options.indentation_string();
        let mut edits = Vec::new();
        for index in selected_lines {
            let offset = document.get_line_by_number(index)?.offset();
            if !unindent {
                let can_insert = provider.can_insert(offset);
                validate()?;
                if can_insert {
                    edits.push(DocumentEdit::new(offset, 0, &indentation));
                }
                continue;
            }
            let prefix =
                TextUtilities::get_single_indentation_segment(document, offset, options.indentation_size)?;
            if prefix.length == 0 {
                continue;
            }
            let parts = Self::writable_parts(provider.as_ref(), prefix);
            validate()?;
            if let Some(part) = parts?.first().filter(|part| part.length != 0) {
                edits.push(DocumentEdit::new(part.offset, part.length, ""));
            }
        }
        let mapping = if unindent {
            OffsetChangeMappingType::Normal
        } else {
            OffsetChangeMappingType::KeepAnchorBeforeInsertion
        };
        self.apply_mapped_document_edits(&edits, mapping, &version, validate)
    }

    /// Reindents the selected logical lines using the installed language
    /// strategy. An empty selection or a rectangle without text reindents the
    /// entire document. Plans are checked first and committed as one undo
    /// unit. A selection ending at a line start includes that line.
    pub fn reindent_selection(&self) -> Result<(), EditError> {
        self.reindent_selection_checked(|| true)
    }

    /// Lets a host keep a reindent operation bound to its current editor view.
    pub fn reindent_selection_checked(&self, is_current: impl Fn() -> bool) -> Result<(), EditError> {
        let version = self.document().version();
        let Some(strategy) = self.indentation_strategy() else {
            return Ok(());
        };
        let selection = self.selection();
        let options = self.options();
        let provider = self.read_only_section_provider();
        let validate = || {
            if !is_current()
                || self.options() != options
                || !self.indentation_strategy()
                    .is_some_and(|current| Rc::ptr_eq(&current, &strategy))
            {
                return Err(EditError::StaleEditPlan);
            }
            self.validate_protected_edit_context(&selection, &version, &provider)
        };
        validate()?;
        let mut edits = Vec::new();
        for (begin, end) in self.reindent_line_ranges()? {
            let planned = strategy.indent_lines(self.document(), begin, end);
            validate()?;
            edits.extend(planned?);
        }
        if let Some(update) = self.begin_prepared_update(&edits, &version)? {
            validate()?;
            self.document().apply_edits(&edits)?;
            update.finish()?;
        } else {
            validate()?;
        }
        Ok(())
    }

    /// Inclusive ranges for explicit reindent. An empty selection or a
    /// rectangle without text targets the whole document. Overlapping
    /// selections share one range.
    pub fn reindent_line_ranges(&self) -> Result<Vec<(usize, usize)>, EditError> {
        let selection = self.selection();
        if selection.is_empty() || selection.is_rectangle_without_text() {
            return Ok(vec![(1, self.document().line_count())]);
        }
        self.selected_line_ranges(true)
    }

    /// Deletes the writable parts of the selected lines in one undo group.
    /// A rectangle without selected text targets its active line. A selection
    /// endpoint at the start of a line includes that line.
    /// A fully protected range remains selected and gains no undo entry.
    pub fn delete_lines_checked(&self, is_current: impl Fn() -> bool) -> Result<(), EditError> {
        let original = self.selection();
        let document = self.document();
        let version = document.version();
        let provider = self.read_only_section_provider();
        let validate = || {
            if !is_current() {
                return Err(EditError::StaleEditPlan);
            }
            self.validate_protected_edit_context(&original, &version, &provider)
        };
        validate()?;
        let snapshot = document.create_snapshot();
        let lines = super::text_transformations::line_ranges(&snapshot, &original, true)?;
        if let [(first, _)] = lines.as_slice() {
            let line = snapshot.get_line_by_number(*first)?;
            if line.total_length() == 0 {
                if original.is_rectangular() {
                    self.set_selection(Selection::empty(line.offset))?;
                }
                return Ok(());
            }
        }
        let mut edits = Vec::new();
        let mut selection = Vec::with_capacity(lines.len());
        let mut primary = 0;
        let mut removed = 0;
        for (first, last) in lines {
            let first = snapshot.get_line_by_number(first)?;
            let last = snapshot.get_line_by_number(last)?;
            let range = SimpleSegment::new(
                first.offset,
                last.offset + last.total_length() - first.offset,
            );
            if (range.offset..=range.end_offset()).contains(&original.primary().caret) {
                primary = selection.len();
            }
            let start = range.offset - removed;
            let parts = Self::writable_parts(provider.as_ref(), range);
            validate()?;
            let previous_edit_count = edits.len();
            for part in parts?.into_iter().filter(|part| part.length != 0) {
                removed += part.length;
                edits.push(DocumentEdit::new(part.offset, part.length, ""));
            }
            let end = if edits.len() == previous_edit_count {
                start + range.length
            } else {
                start
            };
            selection.push(SelectionSegment::new(start, end));
        }
        self.apply_document_edits_and_select(
            &edits,
            Selection::from_segments(selection, primary),
            || is_current() && Rc::ptr_eq(&provider, &self.read_only_section_provider()),
        )
    }

    /// Applies a text or line transformation to writable spans in one undo unit.
    /// Case and tab conversions keep interior caret and anchor positions.
    /// The planner documents each operation's empty-selection behavior.
    pub fn transform_text(&self, transformation: TextTransformation) -> Result<(), EditError> {
        self.transform_text_checked(transformation, || true)
    }

    /// Allows a view to cancel the plan if its document binding changes during
    /// permission checks or when opening the undo group.
    pub fn transform_text_checked(
        &self,
        transformation: TextTransformation,
        is_current: impl Fn() -> bool,
    ) -> Result<(), EditError> {
        self.transform_text_in_language_checked(transformation, "", is_current)
    }

    /// Applies a transformation using simple casing for the supplied language
    /// tag. An empty or undetermined tag selects invariant casing.
    pub fn transform_text_in_language_checked(
        &self,
        transformation: TextTransformation,
        language_tag: &str,
        is_current: impl Fn() -> bool,
    ) -> Result<(), EditError> {
        if transformation == TextTransformation::DeleteLines {
            return self.delete_lines_checked(is_current);
        }
        let document = self.document();
        let version = document.version();
        let selection = self.selection();
        let provider = self.read_only_section_provider();
        let options = self.options();
        let validate = || {
            if !is_current() || *self.inner.options.borrow() != options {
                return Err(EditError::StaleEditPlan);
            }
            self.validate_protected_edit_context(&selection, &version, &provider)
        };
        validate()?;
        let edits = plan_text_transformation_with(
            &document.create_snapshot(),
            &selection,
            &options,
            transformation,
            CaseLocale::from_language_tag(language_tag),
            |range| {
                let parts = Self::writable_parts(provider.as_ref(), range);
                validate()?;
                parts
            },
        )?;
        self.apply_mapped_document_edits(
            &edits,
            transformation.offset_change_mapping(),
            &version,
            validate,
        )
    }

    fn selected_line_ranges(&self, include_end_line: bool) -> Result<Vec<(usize, usize)>, EditError> {
        let mut ranges: Vec<(usize, usize)> = Vec::new();
        for segment in self.selection().segments() {
            let first = self
                .document()
                .get_line_by_offset(segment.start_offset())?
                .line_number;
            let end = if !include_end_line && segment.length() > 0 && segment.end_virtual_space() == 0 {
                segment.end_offset() - 1
            } else {
                segment.end_offset()
            };
            let last = self.document().get_line_by_offset(end)?.line_number;
            match ranges.last_mut() {
                Some((_, end)) if first <= end.saturating_add(1) => *end = (*end).max(last),
                _ => ranges.push((first, last)),
            }
        }
        Ok(ranges)
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
    fn mixed_newlines_keep_raw_string_spaces_and_indent_the_code_caret() {
        let text = concat!(
            "class C {\r\n",
            "    string Text = \"\"\"\r\n",
            "      left😀      right  \r\n",
            "    \"\"\";\r\n",
            "    void M() {  Work();\r\n",
            "    }\r\n}\r\n",
        );
        let document = TextDocument::from_text(text);
        let editor = EditorSession::new(document.clone());
        editor.set_indentation_strategy(Some(Rc::new(
            CSharpIndentationStrategy::with_indentation_string("    ").unwrap(),
        )));
        let raw = text[..text.find("      right").unwrap()].encode_utf16().count();
        let code = text[..text.find("  Work").unwrap()].encode_utf16().count();
        editor.set_selection(Selection::from_segments([
            SelectionSegment::empty(raw), SelectionSegment::empty(code),
        ], 1)).unwrap();

        editor.insert_document_new_line().unwrap();
        assert_eq!(document.text(), text
            .replace("😀      right", "😀\r\n      right")
            .replace("{  Work", "{\r\n        Work"));
        assert!(editor.undo().unwrap());
        assert_eq!(document.text(), text);
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
    fn reindent_includes_the_endpoint_line_and_preserves_the_unselected_suffix() {
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
            "class C {\n  void M() {\n    Call();\n  }\n}\n"
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
    fn deleting_disjoint_lines_keeps_the_intervening_text_and_primary_caret() {
        let document = TextDocument::from_text("A\r\nkeep\r\nBXXB\r\nend");
        let editor = EditorSession::new(document.clone());
        let policy = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
        policy.add(10, 2).unwrap();
        editor.set_read_only_section_provider(policy);
        let selection = Selection::from_segments([SelectionSegment::empty(0), SelectionSegment::empty(12)], 1);
        editor.set_selection(selection.clone()).unwrap();

        editor.transform_text(TextTransformation::DeleteLines).unwrap();
        assert_eq!(document.text(), "keep\r\nXXend");
        assert_eq!(editor.selection(), Selection::from_segments([SelectionSegment::empty(0), SelectionSegment::empty(6)], 1));
        assert_eq!(document.undo_count(), 1);
        assert!(editor.undo().unwrap());
        assert_eq!(document.text(), "A\r\nkeep\r\nBXXB\r\nend");
        assert_eq!(editor.selection(), selection);
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
    fn transformation_rejects_invalid_policy_ranges_before_creating_an_undo_group() {
        struct InvalidSecondLine;
        impl IReadOnlySectionProvider for InvalidSecondLine {
            fn can_insert(&self, _: usize) -> bool {
                true
            }
            fn get_deletable_segments(&self, mut range: SimpleSegment) -> Vec<SimpleSegment> {
                if range.offset != 0 {
                    range.length += 1;
                }
                vec![range]
            }
        }

        let document = TextDocument::from_text("ab\r\ncd");
        let editor = EditorSession::new(document.clone());
        editor.set_read_only_section_provider(Rc::new(InvalidSecondLine));
        editor.set_caret_offset(1).unwrap();
        assert!(matches!(
            editor.transform_text(TextTransformation::Uppercase),
            Err(EditError::InvalidSelection)
        ));
        assert_eq!(document.text(), "ab\r\ncd");
        assert_eq!(editor.caret_offset(), 1);
        assert!(!document.can_undo());
    }
}
