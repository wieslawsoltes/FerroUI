use super::*;
use crate::text_editor::EditTarget;
use crate::{TextCopiedEventArgs, TextPastedEventArgs};
use ferroui_base::input::text_input::TextInputMethodEdit;
use ferroui_base::interactivity::{RoutedEvent, RoutingStrategies};
use ferroui_edit_core::document::DocumentEdit;
use ferroui_edit_core::editing::EditError;

#[cfg(test)]
#[path = "text_area_typing_tests.rs"]
mod tests;

impl TextArea {
    ferroui_base::ferro_routed_event!(
        /// Raised before committed text is edited. Handling the event cancels
        /// the editor's insertion and allows the handler to implement its own.
        pub fn text_entering_event() -> RoutedEvent<TextInputEventArgs> {
            RoutedEvent::register::<TextArea, _>("TextEntering", RoutingStrategies::BUBBLE)
        }
    );

    ferroui_base::ferro_routed_event!(
        /// Raised after accepted text input and its document and selection updates.
        /// A line break in a read-only editor also completes the input lifecycle.
        pub fn text_entered_event() -> RoutedEvent<TextInputEventArgs> {
            RoutedEvent::register::<TextArea, _>("TextEntered", RoutingStrategies::BUBBLE)
        }
    );

    ferroui_base::ferro_routed_event!(
        pub fn text_copied_event() -> RoutedEvent<TextCopiedEventArgs> {
            RoutedEvent::register::<TextArea, _>("TextCopied", RoutingStrategies::BUBBLE)
        }
    );

    ferroui_base::ferro_routed_event!(
        pub fn text_pasted_event() -> RoutedEvent<TextPastedEventArgs> {
            RoutedEvent::register::<TextArea, _>("TextPasted", RoutingStrategies::BUBBLE)
        }
    );

    /// Sends committed text through the same path used by routed input.
    /// Returns false for empty or handled input, a read-only line break, or a non-hosted area.
    pub fn perform_text_input(&self, text: &str) -> Result<bool, EditError> {
        self.perform_committed_input(text, false)
    }

    pub(crate) fn insert_line_break(&self) -> Result<(), EditError> {
        if self.preedit.borrow().is_some() {
            return Ok(());
        }
        self.perform_text_input("\n").map(|_| ())
    }

    pub(crate) fn can_delete_forward(&self, editor: &TextEditor) -> bool {
        let target = EditTarget::new(editor);
        let session = editor.session();
        let selection = session.selection();
        let document = session.document();
        let available = selection.segments().iter().copied().any(|segment| {
            if (!segment.is_empty() && segment.length() == 0)
                || !selection.is_rectangular() && segment.is_empty()
                && segment.caret_virtual_space != 0 && session.options().enable_virtual_space
            {
                return true;
            }
            let range = if segment.is_empty() {
                let offset = match self.visual_deletion_offset(document, segment, false, false) {
                    Ok(Some(offset)) => offset,
                    Ok(None) => match session.logical_deletion_offset(segment.caret, false, false) {
                        Ok(offset) => offset,
                        Err(_) => return false,
                    },
                    Err(_) => return false,
                };
                SelectionSegment::new(segment.caret, offset).segment()
            } else {
                segment.segment()
            };
            range.length != 0 && session.writable_deletions(range)
                .is_ok_and(|edits| !edits.is_empty())
        });
        available && target.is_current(editor) && session.selection() == selection
    }

    pub(crate) fn delete_directionally(
        &self,
        editor: &TextEditor,
        backward: bool,
        by_word: bool,
    ) -> Result<(), EditError> {
        if self.preedit.borrow().is_some() {
            return Ok(());
        }
        let epoch = editor.document_epoch();
        let target = EditTarget::new(editor);
        if backward && !by_word && !self.overstrike_mode()
            && crate::language_pairs::try_backspace(editor)?
        {
            if editor.document_epoch() == epoch {
                editor.selection_changed();
            }
            return Ok(());
        }
        let session = editor.session();
        let selection = session.selection();
        let revision = self.visual_lines.revision.get();
        session.delete_checked(
            backward,
            by_word,
            |segment| self.visual_deletion_offset(session.document(), segment, backward, by_word)
                .map_err(|error| match error {
                    crate::VisualLineError::Document(error) => error.into(),
                    _ => EditError::StaleEditPlan,
                }),
            || target.is_current(editor) && self.visual_lines.revision.get() == revision,
        )?;
        if editor.document_epoch() == epoch && (session.selection() != selection || !target.is_current(editor)) {
            editor.selection_changed();
        }
        Ok(())
    }

    pub(crate) fn delete_selection(&self, editor: &TextEditor) -> Result<(), EditError> {
        if self.preedit.borrow().is_some() {
            return Ok(());
        }
        let session = editor.session();
        let selection = session.selection();
        if selection.is_empty() {
            let caret = selection.primary();
            if !selection.is_rectangular()
                && selection.segments().len() == 1
                && caret.caret_virtual_space != 0
            {
                return editor.set_caret_offset(caret.caret);
            }
        } else if !session.is_read_only() && !session.document().is_read_only() {
            let epoch = editor.document_epoch();
            let target = EditTarget::new(editor);
            session.replace_writable_selection_with_text_checked("", || target.is_current(editor))?;
            if editor.document_epoch() != epoch {
                return Ok(());
            }
            if session.selection() != selection || !target.is_current(editor) {
                editor.selection_changed();
                return Ok(());
            }
        }
        self.bring_caret_into_view();
        Ok(())
    }

    pub(crate) fn delete_lines(&self, editor: &TextEditor) -> Result<(), EditError> {
        let target = EditTarget::new(editor);
        let epoch = editor.document_epoch();
        let session = editor.session();
        let selection = session.selection();
        let position = self.navigation_caret(&selection);
        session.delete_lines_checked(|| target.is_current(editor))?;
        if editor.document_epoch() != epoch {
            return Ok(());
        }
        if target.is_current(editor) {
            let updated = session.selection();
            if updated == selection {
                return Ok(());
            }
            // A fully protected line stays selected around the original caret.
            let segment = updated.primary();
            if !segment.is_empty()
                && segment.contains_position(position.offset, position.virtual_space)
            {
                self.context_caret.set(Some(ContextCaret { position, segment }));
                self.box_caret.set(None);
                self.apply_caret_affinity(position.at_end_of_row);
                editor.notify_selection_changed(false);
                return Ok(());
            }
        }
        editor.selection_changed();
        Ok(())
    }

    pub(super) fn perform_committed_input(&self, text: &str, from_input_method: bool) -> Result<bool, EditError> {
        let Some(editor) = self.editor().filter(|_| !text.is_empty()) else {
            return Ok(false);
        };
        let epoch = editor.document_epoch();
        if self.raise_text_input_lifecycle(Self::text_entering_event(), text) {
            return Ok(false);
        }
        if editor.document_epoch() != epoch {
            return Err(EditError::StaleEditPlan);
        }

        if matches!(text, "\n" | "\r" | "\r\n") {
            if editor.session().is_read_only() || editor.document().is_read_only() {
                self.raise_text_input_lifecycle(Self::text_entered_event(), text);
                if editor.document_epoch() == epoch {
                    self.bring_caret_into_view();
                }
                return Ok(false);
            }
            crate::language_input::insert_new_line(&editor)?;
            if editor.document_epoch() != epoch {
                return Ok(true);
            }
        } else if self.overstrike_mode() && editor.selection().is_empty() {
            editor.session().overstrike_with_text(text)?;
        } else if from_input_method || self.overstrike_mode() || !crate::language_pairs::try_type(&editor, text)? {
            editor.session().replace_writable_selection_with_text(text)?;
        }
        editor.selection_changed();
        self.raise_text_input_lifecycle(Self::text_entered_event(), text);
        Ok(true)
    }

    pub(super) fn perform_surrounding_edit(
        &self,
        editor: &TextEditor,
        before: usize,
        after: usize,
        text: &str,
        owns_input: impl Fn() -> bool,
    ) -> Result<bool, EditError> {
        let primary = editor.selection().primary();
        let start = primary.start_offset().checked_sub(before).ok_or(EditError::InvalidSelection)?;
        let end = primary.end_offset().checked_add(after)
            .filter(|end| *end <= editor.document().text_length())
            .ok_or(EditError::InvalidSelection)?;
        let caret = start.checked_add(text.encode_utf16().count()).ok_or(EditError::InvalidSelection)?;
        self.apply_input_replacement(editor, DocumentEdit::new(start, end - start, text),
            Selection::simple(caret, caret), owns_input)
    }

    pub(super) fn perform_composition_commit(
        &self,
        editor: &TextEditor,
        text: &str,
        owns_input: impl Fn() -> bool,
    ) -> Result<bool, EditError> {
        let target = EditTarget::new(editor);
        let selection = editor.selection();
        let is_current = || owns_input() && target.is_current(editor) && editor.selection() == selection;
        if !is_current() {
            return Err(EditError::StaleEditPlan);
        }
        let cancelled = text.is_empty() || self.raise_text_input_lifecycle(Self::text_entering_event(), text);
        if !is_current() {
            return Err(EditError::StaleEditPlan);
        }
        if cancelled {
            self.set_preedit(None, None);
            return Ok(false);
        }

        let document = editor.document();
        let version = document.version();
        let epoch = editor.document_epoch();
        editor.session().replace_writable_selection_with_text_checked(text, is_current)?;
        let changed = document.version() != version;
        if editor.document_epoch() == epoch {
            self.set_preedit(None, None);
            editor.selection_changed();
            if changed && editor.document_epoch() == epoch {
                self.raise_text_input_lifecycle(Self::text_entered_event(), text);
            }
        }
        Ok(changed)
    }

    pub(super) fn perform_local_input_edit(
        &self,
        editor: &TextEditor,
        edit: &TextInputMethodEdit,
        owns_input: impl Fn() -> bool,
    ) -> Result<bool, EditError> {
        let context = crate::text_input_method_client::SurroundingText::for_editor(editor)
            .ok_or(EditError::InvalidSelection)?;
        let absolute = |offset| usize::try_from(offset).ok()
            .and_then(|offset| context.range.start.checked_add(offset))
            .ok_or(EditError::InvalidSelection);
        let start = absolute(edit.range.start)?;
        let end = absolute(edit.range.end)?;
        let length = end.checked_sub(start).filter(|_| end <= context.range.end)
            .ok_or(EditError::InvalidSelection)?;
        let selection = Selection::simple(absolute(edit.selection.start)?, absolute(edit.selection.end)?);
        self.apply_input_replacement(editor, DocumentEdit::new(start, length, &edit.text), selection, owns_input)
    }

    fn apply_input_replacement(
        &self,
        editor: &TextEditor,
        edit: DocumentEdit,
        replacement_selection: Selection,
        owns_input: impl Fn() -> bool,
    ) -> Result<bool, EditError> {
        let target = EditTarget::new(editor);
        let session = editor.session();
        let selection = session.selection();
        let is_current = || owns_input() && target.is_current(editor) && session.selection() == selection
            && self.preedit.borrow().is_none();
        if !is_current() {
            return Err(EditError::StaleEditPlan);
        }
        if edit.text.is_empty() && edit.length == 0 {
            return Ok(false);
        }
        if !edit.text.is_empty() && self.raise_text_input_lifecycle(Self::text_entering_event(), &edit.text) {
            return Ok(false);
        }
        session.apply_document_edits_and_select(
            std::slice::from_ref(&edit),
            replacement_selection,
            is_current,
        )?;
        if target.is_current_after(editor, 1) || target.is_current(editor) {
            editor.selection_changed();
            if !edit.text.is_empty() {
                self.raise_text_input_lifecycle(Self::text_entered_event(), &edit.text);
            }
        }
        Ok(true)
    }

    fn raise_text_input_lifecycle(
        &self,
        event: &RoutedEvent<TextInputEventArgs>,
        text: &str,
    ) -> bool {
        if !event.has_instance_handlers() && !event.has_raised_subscriptions() {
            return false;
        }
        let mut args = TextInputEventArgs::new();
        args.text = Some(text.to_owned());
        args.set_routed_event(Some(event));
        self.raise_event(&args);
        args.handled()
    }
}
