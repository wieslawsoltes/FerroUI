use crate::snippet_background_renderer::SnippetBackgroundRegistration;
use crate::{
    CompletionData, CompletionWindow, EditorInputHandler, EditorInputLayer, ICompletionData,
    KnownLayer, SnippetBackgroundRenderer, TextArea, TextEditor,
};
use ferroui_base::input::{Key, KeyEventArgs, KeyModifiers};
use ferroui_base::reactive::Subscription;
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::TopLevel;
use ferroui_edit_core::document::{
    DocumentEdit, DocumentError, DocumentSubscription, SimpleSegment, TextDocument, TextUtilities,
};
use ferroui_edit_core::editing::EditError;
use ferroui_edit_core::snippets::{
    DeactivateReason, Snippet, SnippetError, SnippetEventArgs, SnippetInsertionOptions,
    SnippetSession,
};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Connects a linked snippet session to the editor's selection and keyboard
/// routing. Retain the controller while the snippet is active; dropping it
/// removes its input subscriptions and ends placeholder synchronization.
pub struct SnippetController {
    weak_self: Weak<Self>,
    editor: WeakRef<TextEditor>,
    document: TextDocument,
    session: SnippetSession,
    subscriptions: RefCell<Vec<Subscription>>,
    input_layer: RefCell<Option<EditorInputLayer>>,
    deactivated: RefCell<Option<DocumentSubscription>>,
    backgrounds: RefCell<Option<SnippetBackgroundRegistration>>,
    choice_window: RefCell<Option<WeakRef<CompletionWindow>>>,
}

struct SnippetInput(Weak<SnippetController>);

impl EditorInputHandler for SnippetInput {
    fn on_key_down(&self, area: &TextArea, event: &KeyEventArgs) {
        if area.has_preedit() {
            return;
        }
        if let Some(controller) = self.0.upgrade() {
            controller.handle_key(event);
        }
    }

    fn detach(&self, _: &TextArea) {
        if let Some(controller) = self.0.upgrade() {
            controller.finish();
        }
    }
}

impl SnippetController {
    pub fn insert(editor: &Ref<TextEditor>, snippet: &Snippet) -> Result<Rc<Self>, SnippetError> {
        let _insertion = editor.begin_snippet_insertion()?;
        Self::insert_over_selection(editor, snippet)
    }

    pub(crate) fn insert_over_selection(
        editor: &Ref<TextEditor>,
        snippet: &Snippet,
    ) -> Result<Rc<Self>, SnippetError> {
        let selection = editor.selection();
        if selection.segments().len() != 1 {
            return Err(SnippetError::MultipleSelections);
        }
        let selected = selection.primary();
        let document = editor.document();
        let selected_text = document.get_text(selected.start_offset(), selected.length())?;
        // Preserve selected leading indentation, as upstream Snippet.Insert
        // does, while keeping whitespace-only selections within their bounds.
        let insertion_offset = if selected.length() != 0 {
            TextUtilities::get_whitespace_after(&document, selected.start_offset())?
                .end_offset()
                .min(selected.end_offset())
        } else {
            selected.start_offset()
        };
        Self::insert_at(editor, snippet, SimpleSegment::new(insertion_offset, selected.end_offset() - insertion_offset), &selected_text)
    }

    pub(crate) fn insert_at(
        editor: &Ref<TextEditor>,
        snippet: &Snippet,
        range: SimpleSegment,
        selected_text: &str,
    ) -> Result<Rc<Self>, SnippetError> {
        let area = editor.text_area();
        if area.is_changing_input_handlers() {
            return Err(DocumentError::ReentrantChange.into());
        }
        let selection = editor.selection();
        if selection.segments().len() != 1 {
            return Err(SnippetError::MultipleSelections);
        }
        let document = editor.document();
        let epoch = editor.document_epoch();
        let insertion_offset = range.offset;
        let session = editor.session();
        let selected = selection.primary();
        let mut padding = if insertion_offset == selected.start_offset() {
            session
                .virtual_space_padding_at(insertion_offset, selected.start_virtual_space())
                .map_err(edit_error)?
        } else {
            String::new()
        };
        let mut options = SnippetInsertionOptions::from_document(
            &document,
            insertion_offset,
            editor.options().indentation_string(),
        )?;
        if !padding.is_empty() {
            let line = document.get_line_by_offset(insertion_offset)?;
            if options.indentation.encode_utf16().count() == line.length() {
                options.indentation.push_str(&padding);
            }
        }
        let version = document.version();
        let is_current = || {
            editor.document_epoch() == epoch
                && document.version() == version
                && editor.selection() == selection
        };
        let prepared = snippet.prepare_with_options(selected_text, options)?;
        if !is_current() {
            return Err(DocumentError::ReentrantChange.into());
        }
        // Padding contains ASCII whitespace. The snippet's relative offsets
        // begin after it, while both parts belong to the same edit and undo.
        let snippet_offset = insertion_offset
            .checked_add(padding.len())
            .ok_or(SnippetError::InvalidExpansion("virtual-space padding is too large"))?;
        let insertion = if padding.is_empty() {
            prepared.text()
        } else {
            padding
                .try_reserve_exact(prepared.text().len())
                .map_err(|_| SnippetError::InvalidExpansion("virtual-space padding is too large"))?;
            padding.push_str(prepared.text());
            &padding
        };
        let edits = [DocumentEdit::new(insertion_offset, range.length, insertion)];
        let update = document.begin_update()?;
        if !is_current() {
            return Err(DocumentError::ReentrantChange.into());
        }
        let provider = session.read_only_section_provider();
        let policy = session.validate_document_edits(&edits);
        if !is_current() || !Rc::ptr_eq(&provider, &session.read_only_section_provider()) {
            return Err(DocumentError::ReentrantChange.into());
        }
        policy.map_err(edit_error)?;
        document.apply_edits(&edits)?;
        let weak_editor = editor.downgrade();
        let expected_document = document.clone();
        let validator: Rc<ferroui_edit_core::snippets::SnippetEditValidator> =
            Rc::new(move |edits| {
                let editor = weak_editor.upgrade().ok_or(DocumentError::ReadOnly)?;
                if editor.document() != expected_document {
                    return Err(DocumentError::DifferentDocument);
                }
                editor
                    .session()
                    .validate_document_edits(edits)
                    .map_err(|error| match error {
                        EditError::Document(error) => error,
                        _ => DocumentError::ReadOnly,
                    })?;
                if editor.document() != expected_document {
                    return Err(DocumentError::DifferentDocument);
                }
                Ok(())
            });
        let session = prepared.activate_with_validator(
            document.clone(),
            snippet_offset,
            Some(validator),
        )?;
        let controller = Rc::new_cyclic(|weak| Self {
            weak_self: weak.clone(),
            editor: editor.downgrade(),
            document,
            session,
            subscriptions: RefCell::new(Vec::new()),
            input_layer: RefCell::new(None),
            deactivated: RefCell::new(None),
            backgrounds: RefCell::new(None),
            choice_window: RefCell::new(None),
        });
        let weak = Rc::downgrade(&controller);
        *controller.deactivated.borrow_mut() =
            Some(controller.session.deactivated(move |_, args| {
                if let Some(controller) = weak.upgrade() {
                    controller.dispose_input();
                    if args.moves_to_final_caret() {
                        if let Err(error) = controller.select_active() {
                            if let Some(editor) = controller.editor.upgrade() {
                                editor.report_error(error.to_string());
                            }
                        }
                    }
                }
            }));
        let handler = Rc::new(SnippetInput(Rc::downgrade(&controller)));
        let layer = area
            .push_stacked_input_handler(handler)
            .map_err(|_| DocumentError::ReentrantChange)?;
        controller.input_layer.replace(Some(layer));
        let weak = Rc::downgrade(&controller);
        controller
            .subscriptions
            .borrow_mut()
            .push(
                editor.add_disposable_handler(TextEditor::text_changed_event(), move |_, _| {
                    if let Some(controller) = weak.upgrade() {
                        if controller
                            .editor
                            .upgrade()
                            .is_none_or(|editor| editor.document() != controller.document)
                        {
                            controller.finish();
                        } else {
                            controller.dismiss_choices();
                            controller.invalidate_backgrounds(false);
                        }
                    }
                }),
            );
        let weak = Rc::downgrade(&controller);
        controller
            .subscriptions
            .borrow_mut()
            .push(editor.add_disposable_handler(
                TextEditor::selection_changed_event(),
                move |_, _| {
                    if let Some(controller) = weak.upgrade() {
                        if let Some(editor) = controller.editor.upgrade() {
                            if !controller.selection_in_active_field(&editor) {
                                controller.dismiss_choices();
                            }
                        }
                        controller.invalidate_backgrounds(true);
                    }
                },
            ));
        let backgrounds =
            SnippetBackgroundRegistration::attach(&editor.text_area(), &controller.session);
        *controller.backgrounds.borrow_mut() = backgrounds;
        drop(update);
        controller.select_active()?;
        if !controller.is_active() {
            controller.dispose_input();
        }
        Ok(controller)
    }

    pub fn session(&self) -> SnippetSession {
        self.session.clone()
    }
    pub fn is_active(&self) -> bool {
        !self.session.is_closed()
    }

    /// The live fill renderer, for host-specific snippet field styling.
    pub fn field_background_renderer(&self) -> Option<Rc<SnippetBackgroundRenderer>> {
        self.backgrounds
            .borrow()
            .as_ref()
            .map(|backgrounds| backgrounds.background.clone())
    }

    /// The live dotted-border renderer for the caret's field and its mirrors.
    pub fn active_field_renderer(&self) -> Option<Rc<SnippetBackgroundRenderer>> {
        self.backgrounds
            .borrow()
            .as_ref()
            .map(|backgrounds| backgrounds.foreground.clone())
    }

    /// Ends the session without changing document text or current selection.
    pub fn finish(&self) {
        self.session
            .deactivate(DeactivateReason::InputHandlerDetached);
        self.dispose_input();
    }

    fn dispose_input(&self) {
        self.dismiss_choices();
        let layer = self.input_layer.take();
        if let Some(layer) = layer {
            let _ = layer.pop();
        }
        let backgrounds = self.backgrounds.take();
        drop(backgrounds);
        if let Some(subscription) = self.deactivated.take() {
            subscription.dispose();
        }
        let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
    }

    fn invalidate_backgrounds(&self, foreground_only: bool) {
        if !self.is_active() {
            return;
        }
        if let Some(editor) = self.editor.upgrade() {
            let view = editor.text_area();
            if !foreground_only {
                view.invalidate_layer(KnownLayer::Background);
            }
            view.invalidate_layer(KnownLayer::Text);
        }
    }

    pub fn next_placeholder(&self, backwards: bool) -> Result<bool, SnippetError> {
        if !self.is_active() {
            return Ok(false);
        }
        self.dismiss_choices();
        self.session.try_next_element(backwards)?;
        self.select_active()?;
        if !self.is_active() {
            self.finish();
        }
        Ok(true)
    }

    fn select_active(&self) -> Result<(), SnippetError> {
        let Some(editor) = self.editor.upgrade() else {
            self.finish();
            return Ok(());
        };
        if editor.document() != self.document {
            self.finish();
            return Ok(());
        }
        if let Some(segment) = self.session.active_segment() {
            editor
                .select(segment.offset, segment.length)
                .map_err(edit_error)?;
            self.show_choices(&editor, segment)?;
        } else if self
            .session
            .deactivation_reason()
            .is_some_and(|reason| SnippetEventArgs::new(reason).moves_to_final_caret())
        {
            if let Some(offset) = self.session.caret_offset() {
                editor.set_caret_offset(offset).map_err(edit_error)?;
            }
        }
        Ok(())
    }

    fn selection_in_active_field(&self, editor: &TextEditor) -> bool {
        let selection = editor.selection();
        let primary = selection.primary();
        self.session.active_segment().is_some_and(|segment| {
            selection.segments().len() == 1
                && primary.start_offset() >= segment.offset
                && primary.end_offset() <= segment.end_offset()
        })
    }

    fn show_choices(
        &self,
        editor: &Ref<TextEditor>,
        segment: SimpleSegment,
    ) -> Result<(), SnippetError> {
        let Some((field, choices)) = self
            .session
            .active_placeholder()
            .zip(self.session.active_choices())
        else {
            return Ok(());
        };
        if TopLevel::get_top_level(Some(&editor.text_area())).is_none() {
            return Ok(());
        }
        let current = self.document.get_text(segment.offset, segment.length)?;
        let selected = choices
            .iter()
            .position(|value| value == &current)
            .unwrap_or(0);
        let items = choices
            .iter()
            .enumerate()
            .map(|(index, choice)| {
                let weak = self.weak_self.clone();
                Rc::new(CompletionData {
                    text: choice.clone(),
                    action: Some(Rc::new(move |editor, range, _| {
                        let Some(controller) = weak.upgrade() else {
                            return Ok(false);
                        };
                        if controller.document != editor.document()
                            || controller.session.active_placeholder() != Some(field)
                            || controller.session.active_segment() != Some(range)
                            || !controller.selection_in_active_field(editor)
                        {
                            return Ok(false);
                        }
                        match controller
                            .session
                            .choose_active(index)
                            .and_then(|()| controller.next_placeholder(false))
                        {
                            Ok(advanced) => Ok(advanced),
                            Err(SnippetError::Document(error)) => Err(error),
                            Err(error) => {
                                editor.report_error(error.to_string());
                                Ok(false)
                            }
                        }
                    })),
                    ..CompletionData::default()
                }) as Rc<dyn ICompletionData>
            })
            .collect();
        let window = CompletionWindow::show_choices(
            editor,
            segment.offset,
            segment.end_offset(),
            items,
            selected,
        )?;
        if self.session.active_segment() == Some(segment) && !window.is_closed() {
            self.choice_window.replace(Some(window.downgrade()));
        } else {
            window.dismiss();
        }
        Ok(())
    }

    fn dismiss_choices(&self) {
        let window = self.choice_window.take();
        if let Some(window) = window.and_then(|window| window.upgrade()) {
            window.dismiss();
        }
    }

    fn handle_key(&self, event: &KeyEventArgs) {
        if event.handled() || !self.is_active() {
            return;
        }
        let Some(editor) = self.editor.upgrade() else {
            self.finish();
            return;
        };
        if editor.text_area().is_hosted_child_event(event) {
            return;
        }
        if editor.document() != self.document {
            self.finish();
            return;
        }
        match event.key {
            Key::Escape => {
                self.session.deactivate(DeactivateReason::EscapePressed);
                event.set_handled(true);
            }
            Key::Return => {
                match self.session.commit_active_transforms() {
                    Ok(()) => self.session.deactivate(DeactivateReason::ReturnPressed),
                    Err(error) => editor.report_error(error.to_string()),
                }
                event.set_handled(true);
            }
            Key::Tab
                if !event
                    .key_modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::META | KeyModifiers::ALT) =>
            {
                if let Some(segment) = self.session.active_segment() {
                    let selection = editor.selection().primary();
                    if selection.start_offset() < segment.offset
                        || selection.end_offset() > segment.end_offset()
                    {
                        self.finish();
                        return;
                    }
                }
                if let Err(error) =
                    self.next_placeholder(event.key_modifiers.contains(KeyModifiers::SHIFT))
                {
                    editor.report_error(error.to_string());
                }
                event.set_handled(true);
            }
            _ => {}
        }
    }
}

fn edit_error(error: EditError) -> SnippetError {
    match error {
        EditError::Document(error) => SnippetError::Document(error),
        EditError::VirtualSpaceTooLarge => {
            SnippetError::InvalidExpansion("virtual-space padding is too large")
        }
        EditError::InvalidSelection => SnippetError::InvalidExpansion("invalid snippet insertion position"),
        _ => SnippetError::Document(DocumentError::ReadOnly),
    }
}

impl Drop for SnippetController {
    fn drop(&mut self) {
        self.session
            .deactivate(DeactivateReason::InputHandlerDetached);
        self.dispose_input();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::input::InputElement;
    use ferroui_edit_core::editing::IReadOnlySectionProvider;
    use ferroui_edit_core::snippets::{
        ISnippetElement, SnippetElement, SnippetElementRef, SnippetInsertionContext,
    };
    use std::cell::Cell;

    #[test]
    fn virtual_space_snippet_preserves_the_column_and_field_offsets() {
        use ferroui_edit_core::editing::{Selection, SelectionSegment};

        let _scope = crate::tests::test_scope();
        let editor = crate::tests::laid_out_editor("ab\r\nnext");
        editor
            .set_selection(Selection::from_segments(
                [SelectionSegment::with_virtual_space(2, 5, 2, 2)],
                0,
            ))
            .unwrap();
        let snippet = Snippet::parse("${1:😀} / $1$0").unwrap();
        let controller = editor.insert_snippet(&snippet).unwrap();
        assert_eq!(editor.text(), "ab  😀 / 😀\r\nnext");
        assert_eq!(editor.selection().primary().segment(), SimpleSegment::new(4, 2));
        assert!(controller.is_active());
        assert!(editor.undo().unwrap());
        assert_eq!(editor.text(), "ab\r\nnext");
        assert!(!controller.is_active());
    }

    #[test]
    fn virtual_space_snippet_uses_materialized_indentation_on_continuation_lines() {
        let _scope = crate::tests::test_scope();
        let editor = crate::tests::laid_out_editor("\t\r\nend");
        let mut options = editor.options();
        options.enable_virtual_space = true;
        options.convert_tabs_to_spaces = false;
        options.indentation_size = 4;
        editor.set_options(options);
        editor.session().set_caret_position(1, 6).unwrap();
        let controller = editor
            .insert_snippet(&Snippet::parse("${1:A}\n$1$0").unwrap())
            .unwrap();
        assert_eq!(editor.text(), "\t\t  A\r\n\t\t  A\r\nend");
        assert_eq!(editor.selection().primary().segment(), SimpleSegment::new(4, 1));
        editor.replace_selection_with_text("Z").unwrap();
        assert_eq!(editor.text(), "\t\t  Z\r\n\t\t  Z\r\nend");
        assert!(controller.is_active());
    }

    #[test]
    fn virtual_space_snippet_places_empty_fields_and_rejects_invalid_padding() {
        let _scope = crate::tests::test_scope();
        let editor = crate::tests::laid_out_editor("ab");
        let snippet = Snippet::parse("${1:}$0").unwrap();
        editor.session().set_caret_position(2, usize::MAX).unwrap();
        assert!(matches!(
            editor.insert_snippet(&snippet),
            Err(SnippetError::InvalidExpansion("virtual-space padding is too large"))
        ));
        assert_eq!(editor.text(), "ab");
        editor.session().set_caret_position(2, 3).unwrap();
        editor.set_is_read_only(true);
        assert!(matches!(
            editor.insert_snippet(&snippet),
            Err(SnippetError::Document(DocumentError::ReadOnly))
        ));
        assert_eq!(editor.text(), "ab");
        editor.set_is_read_only(false);
        let controller = editor.insert_snippet(&snippet).unwrap();
        assert_eq!(editor.text(), "ab   ");
        assert_eq!(editor.caret_offset(), 5);
        assert!(controller.is_active());
    }

    #[test]
    fn replacing_a_snippet_rejects_nested_insertions_from_deactivation() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        let previous = editor
            .insert_snippet(&Snippet::parse("${1:first}$0").unwrap())
            .unwrap();
        let weak = editor.downgrade();
        let attempts = Rc::new(Cell::new(0));
        let count = attempts.clone();
        let _subscription = previous.session().deactivated(move |_, _| {
            let editor = weak.upgrade().unwrap();
            let nested = Snippet::parse("${1:nested}$0").unwrap();
            assert!(matches!(
                editor.insert_snippet(&nested),
                Err(SnippetError::Document(DocumentError::ReentrantChange))
            ));
            assert!(matches!(
                SnippetController::insert(&editor, &nested),
                Err(SnippetError::Document(DocumentError::ReentrantChange))
            ));
            count.set(count.get() + 1);
        });

        let replacement = editor
            .insert_snippet(&Snippet::parse("${1:second}$0").unwrap())
            .unwrap();
        assert_eq!(attempts.get(), 1);
        assert!(!previous.is_active());
        assert!(replacement.is_active());
        assert_eq!(editor.text(), "second");
        assert_eq!(editor.selected_text().unwrap(), "second");
        editor.undo().unwrap();
        assert_eq!(editor.text(), "first");
        assert!(!replacement.is_active());
    }

    #[test]
    fn replacement_keeps_its_original_selection_across_deactivation_callbacks() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        let previous = editor
            .insert_snippet(&Snippet::parse("${1:first}$0").unwrap())
            .unwrap();
        let weak = editor.downgrade();
        let _subscription = previous.session().deactivated(move |_, _| {
            weak.upgrade().unwrap().set_caret_offset(5).unwrap();
        });
        let replacement = Snippet::parse("${1:second}$0").unwrap();
        assert!(matches!(
            editor.insert_snippet(&replacement),
            Err(SnippetError::Document(DocumentError::ReentrantChange))
        ));
        assert_eq!(editor.text(), "first");
        assert_eq!(editor.caret_offset(), 5);
        assert!(editor.insert_snippet(&replacement).unwrap().is_active());
        assert_eq!(editor.text(), "firstsecond");
    }

    #[test]
    fn document_rebinding_closes_the_snippet_before_allowing_another_insertion() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        let original = editor.document();
        let previous = editor
            .insert_snippet(&Snippet::parse("${1:first}$0").unwrap())
            .unwrap();
        let weak = editor.downgrade();
        let _subscription = previous.session().deactivated(move |_, _| {
            let editor = weak.upgrade().unwrap();
            let nested = Snippet::parse("${1:unexpected}$0").unwrap();
            assert!(matches!(
                editor.insert_snippet(&nested),
                Err(SnippetError::Document(DocumentError::ReentrantChange))
            ));
        });

        editor.set_document(TextDocument::from_text("new"));
        assert_eq!(original.text(), "first");
        assert!(!previous.is_active());
        editor.select_all();
        let current = editor
            .insert_snippet(&Snippet::parse("${1:second}$0").unwrap())
            .unwrap();
        assert!(current.is_active());
        assert_eq!(editor.text(), "second");
    }

    #[test]
    fn explicit_range_insertion_stays_guarded_during_custom_completion() {
        struct CompleteWithInsertion(WeakRef<TextEditor>, Rc<Cell<bool>>);
        impl ISnippetElement for CompleteWithInsertion {
            fn insert(&self, context: &mut SnippetInsertionContext) -> Result<(), SnippetError> {
                let editor = self.0.clone();
                let rejected = self.1.clone();
                context.on_insertion_completed(move |_| {
                    let nested = Snippet::parse("${1:unexpected}$0").unwrap();
                    rejected.set(matches!(
                        editor.upgrade().unwrap().insert_snippet(&nested),
                        Err(SnippetError::Document(DocumentError::ReentrantChange))
                    ));
                });
                Ok(())
            }
        }

        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("query").unwrap();
        let rejected = Rc::new(Cell::new(false));
        let mut snippet = Snippet::parse("${1:result}$0").unwrap();
        snippet.elements.push(SnippetElement::Custom(SnippetElementRef::new(
            CompleteWithInsertion(editor.downgrade(), rejected.clone()),
        )));
        let controller = editor
            .insert_snippet_at(&snippet, SimpleSegment::new(0, 5), "")
            .unwrap();
        assert!(rejected.get());
        assert!(controller.is_active());
        assert_eq!(editor.text(), "result");
        assert_eq!(editor.selected_text().unwrap(), "result");
        editor.undo().unwrap();
        assert_eq!(editor.text(), "query");
    }

    #[test]
    fn policy_callback_cannot_insert_into_a_document_it_unbound() {
        struct RebindOnInsert(WeakRef<TextEditor>, TextDocument);
        impl IReadOnlySectionProvider for RebindOnInsert {
            fn can_insert(&self, _: usize) -> bool {
                self.0.upgrade().unwrap().set_document(self.1.clone());
                true
            }

            fn get_deletable_segments(&self, segment: SimpleSegment) -> Vec<SimpleSegment> {
                vec![segment]
            }
        }

        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("original").unwrap();
        editor.set_caret_offset(2).unwrap();
        let original = editor.document();
        let replacement = TextDocument::from_text("replacement");
        editor.session().set_read_only_section_provider(Rc::new(RebindOnInsert(
            editor.downgrade(),
            replacement.clone(),
        )));
        let snippet = Snippet::parse("${1:inserted}$0").unwrap();
        assert!(matches!(
            SnippetController::insert(&editor, &snippet),
            Err(SnippetError::Document(DocumentError::ReentrantChange))
        ));
        assert_eq!(original.text(), "original");
        assert_eq!(editor.document(), replacement);
        assert_eq!(editor.text(), "replacement");
    }

    #[test]
    fn controller_replaces_selection_tracks_fields_and_finishes_at_final_caret() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("old").unwrap();
        editor.select_all();
        let controller =
            SnippetController::insert(&editor, &Snippet::parse("${1:name} = $1$0").unwrap())
                .unwrap();
        assert_eq!(editor.text(), "name = name");
        assert_eq!(editor.selected_text().unwrap(), "name");
        editor.replace_selection_with_text("😀").unwrap();
        assert_eq!(editor.text(), "😀 = 😀");
        assert!(controller.next_placeholder(false).unwrap());
        assert!(!controller.is_active());
        assert_eq!(editor.caret_offset(), 7);
        editor.undo().unwrap();
        assert_eq!(editor.text(), "name = name");
        editor.undo().unwrap();
        assert_eq!(editor.text(), "old");
    }

    #[test]
    fn invalid_and_read_only_snippets_leave_selection_unchanged() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("keep").unwrap();
        editor.select_all();
        let invalid = Snippet {
            elements: vec![SnippetElement::Bound { id: 42 }],
        };
        assert!(SnippetController::insert(&editor, &invalid).is_err());
        assert_eq!(editor.text(), "keep");
        editor.set_is_read_only(true);
        assert!(SnippetController::insert(&editor, &Snippet::parse("${1:x}").unwrap()).is_err());
        assert_eq!(editor.text(), "keep");
    }
    #[test]
    fn completion_temporarily_handles_tab_before_snippet_navigation() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        let controller =
            SnippetController::insert(&editor, &Snippet::parse("${1:R}$0").unwrap()).unwrap();
        let window = crate::CompletionWindow::show_for(
            &editor,
            0,
            1,
            vec![ferroui_edit_core::completion::CompletionItem::new("Read")],
        )
        .unwrap();
        let mut tab = KeyEventArgs::new();
        tab.key = Key::Tab;
        tab.set_routed_event(Some(InputElement::key_down_event()));
        editor.text_area().raise_event(&tab);
        assert!(tab.handled());
        assert_eq!(editor.text(), "Read");
        assert!(window.is_closed());
        assert!(controller.is_active());
        let mut tab = KeyEventArgs::new();
        tab.key = Key::Tab;
        tab.set_routed_event(Some(InputElement::key_down_event()));
        editor.text_area().raise_event(&tab);
        assert!(tab.handled());
        assert!(!controller.is_active());
    }
    #[test]
    fn composing_in_a_linked_field_keeps_snippet_key_actions_suspended() {
        use ferroui_base::input::TextInputEventArgs;
        use ferroui_base::input::text_input::TextInputMethodClientRequestedEventArgs;

        let _scope = crate::tests::test_scope();
        let editor = crate::tests::laid_out_editor("");
        let controller = SnippetController::insert(
            &editor,
            &Snippet::parse("${1:name} = $1; ${2:value}$0").unwrap(),
        )
        .unwrap();
        let area = editor.text_area();
        let request = TextInputMethodClientRequestedEventArgs::new();
        request.set_routed_event(Some(InputElement::text_input_method_client_requested_event()));
        area.raise_event(&request);
        let client = request.client().unwrap();
        let document = editor.document();
        let undo_count = document.undo_count();
        let original_selection = editor.selection();
        let active_field = controller.session().active_segment();
        let route_key = |key, modifiers| {
            let mut event = KeyEventArgs::new();
            event.key = key;
            event.key_modifiers = modifiers;
            event.set_routed_event(Some(InputElement::key_down_event()));
            area.raise_event(&event);
            event.handled()
        };
        client.set_preedit_text(Some("編"));
        for (key, modifiers) in [
            (Key::Tab, KeyModifiers::NONE),
            (Key::Tab, KeyModifiers::SHIFT),
            (Key::Return, KeyModifiers::NONE),
            (Key::Escape, KeyModifiers::NONE),
        ] {
            route_key(key, modifiers);
            assert!(controller.is_active());
            assert_eq!(controller.session().active_segment(), active_field, "{key:?}");
            assert_eq!(editor.selection(), original_selection);
            assert_eq!(editor.text(), "name = name; value");
            assert_eq!(document.undo_count(), undo_count);
        }
        client.set_preedit_text(None);
        assert_eq!(editor.selection(), original_selection);

        client.set_preedit_text(Some("候補"));
        let mut input = TextInputEventArgs::new();
        input.text = Some("漢😀".into());
        input.set_routed_event(Some(InputElement::text_input_event()));
        area.raise_event(&input);
        assert!(input.handled());
        assert_eq!(editor.text(), "漢😀 = 漢😀; value");
        assert_eq!(document.undo_count(), undo_count + 1);
        assert!(route_key(Key::Tab, KeyModifiers::NONE));
        assert_eq!(editor.selected_text().unwrap(), "value");
        assert!(editor.undo().unwrap());
        assert_eq!(editor.text(), "name = name; value");
    }

    #[test]
    fn return_accepts_at_final_caret_and_escape_preserves_selection() {
        for (key, reason, expected_caret) in [
            (Key::Return, DeactivateReason::ReturnPressed, 5),
            (Key::Escape, DeactivateReason::EscapePressed, 4),
        ] {
            let _scope = crate::tests::test_scope();
            let editor = TextEditor::new();
            let controller =
                SnippetController::insert(&editor, &Snippet::parse("${1:name};$0tail").unwrap())
                    .unwrap();
            let mut event = KeyEventArgs::new();
            event.key = key;
            event.set_routed_event(Some(InputElement::key_down_event()));
            editor.text_area().raise_event(&event);
            assert!(event.handled());
            assert_eq!(editor.text(), "name;tail");
            assert_eq!(editor.caret_offset(), expected_caret);
            assert_eq!(controller.session().deactivation_reason(), Some(reason));
            assert!(!controller.is_active());
            assert!(controller.subscriptions.borrow().is_empty());
        }
    }

    #[test]
    fn undoing_the_insertion_removes_snippet_keyboard_handlers() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        let controller =
            SnippetController::insert(&editor, &Snippet::parse("${1:name}-$1$0").unwrap()).unwrap();
        editor.undo().unwrap();
        assert_eq!(editor.text(), "");
        assert_eq!(
            controller.session().deactivation_reason(),
            Some(DeactivateReason::Deleted)
        );
        assert!(controller.subscriptions.borrow().is_empty());
        assert_eq!(editor.caret_offset(), 0);
    }
    #[test]
    fn selected_leading_indentation_survives_context_formatted_insertion() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("  old\r\n").unwrap();
        editor.select(0, 5).unwrap();
        let controller =
            SnippetController::insert(&editor, &Snippet::parse("{\n\t${1:name}\n}$0").unwrap())
                .unwrap();
        let expected = format!(
            "  {{\r\n  {}name\r\n  }}\r\n",
            editor.options().indentation_string()
        );
        assert_eq!(editor.text(), expected);
        assert_eq!(editor.selected_text().unwrap(), "name");
        assert!(controller.is_active());
        editor.undo().unwrap();
        assert_eq!(editor.text(), "  old\r\n");
        assert!(!controller.is_active());
    }
}
