use crate::TextEditor;
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Ref, WeakRef};
use ferroui_edit_core::document::{
    DocumentEdit, DocumentError, DocumentSubscription, TextDocument,
};
use ferroui_edit_core::editing::EditError;
use ferroui_edit_core::snippets::{
    DeactivateReason, Snippet, SnippetError, SnippetEventArgs, SnippetSession,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Connects a linked snippet session to the editor's selection and keyboard
/// routing. Retain the controller while the snippet is active; dropping it
/// removes its input subscriptions and ends placeholder synchronization.
pub struct SnippetController {
    editor: WeakRef<TextEditor>,
    document: TextDocument,
    session: SnippetSession,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    deactivated: RefCell<Option<DocumentSubscription>>,
}

impl SnippetController {
    pub fn insert(editor: &Ref<TextEditor>, snippet: &Snippet) -> Result<Rc<Self>, SnippetError> {
        let selection = editor.selection();
        if selection.segments().len() != 1 {
            return Err(SnippetError::MultipleSelections);
        }
        let selected = selection.primary();
        let document = editor.document();
        let selected_text = document.get_text(selected.start_offset(), selected.length())?;
        let prepared = snippet.prepare(&selected_text)?;
        let update = document.begin_update()?;
        editor
            .session()
            .apply_document_edits(&[DocumentEdit::new(
                selected.start_offset(),
                selected.length(),
                prepared.text(),
            )])
            .map_err(edit_error)?;
        let session = prepared.activate(document.clone(), selected.start_offset())?;
        let weak_editor = editor.downgrade();
        session.set_edit_validator(Some(Rc::new(move |edits| {
            let editor = weak_editor.upgrade().ok_or(DocumentError::ReadOnly)?;
            editor
                .session()
                .validate_document_edits(edits)
                .map_err(|error| match error {
                    EditError::Document(error) => error,
                    _ => DocumentError::ReadOnly,
                })
        })));
        let controller = Rc::new(Self {
            editor: editor.downgrade(),
            document,
            session,
            subscriptions: RefCell::new(Vec::new()),
            deactivated: RefCell::new(None),
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
        let weak = Rc::downgrade(&controller);
        controller
            .subscriptions
            .borrow_mut()
            .push(editor.text_area().add_disposable_handler_with(
                InputElement::key_down_event(),
                move |_, event: &KeyEventArgs| {
                    if let Some(controller) = weak.upgrade() {
                        controller.handle_key(event);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            ));
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
                        }
                    }
                }),
            );
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

    /// Ends the session without changing document text or current selection.
    pub fn finish(&self) {
        self.session
            .deactivate(DeactivateReason::InputHandlerDetached);
        self.dispose_input();
    }

    fn dispose_input(&self) {
        if let Some(subscription) = self.deactivated.borrow_mut().take() {
            subscription.dispose();
        }
        let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
    }

    pub fn next_placeholder(&self, backwards: bool) -> Result<bool, SnippetError> {
        if !self.is_active() {
            return Ok(false);
        }
        self.session.next_element(backwards);
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

    fn handle_key(&self, event: &KeyEventArgs) {
        if event.handled() || !self.is_active() {
            return;
        }
        let Some(editor) = self.editor.upgrade() else {
            self.finish();
            return;
        };
        if editor.document() != self.document {
            self.finish();
            return;
        }
        // A completion popup temporarily owns Tab/Escape. When it closes,
        // snippet field traversal resumes without reinstalling handlers.
        if crate::CompletionWindow::active_for(&editor).is_some() {
            return;
        }
        match event.key {
            Key::Escape => {
                self.session.deactivate(DeactivateReason::EscapePressed);
                event.set_handled(true);
            }
            Key::Return => {
                self.session.deactivate(DeactivateReason::ReturnPressed);
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
                    self.finish();
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
        _ => SnippetError::Document(DocumentError::ReadOnly),
    }
}

impl Drop for SnippetController {
    fn drop(&mut self) {
        self.session
            .deactivate(DeactivateReason::InputHandlerDetached);
        if let Some(subscription) = self.deactivated.get_mut().take() {
            subscription.dispose();
        }
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_edit_core::snippets::SnippetElement;

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
}
