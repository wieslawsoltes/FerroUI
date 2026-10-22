use crate::TextEditor;
use ferroui_base::input::ICommand;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{BoxedValue, Ref, WeakRef};
use ferroui_edit_core::document::{DocumentEdit, DocumentSubscription, TextDocument};
use ferroui_edit_core::editing::{
    next_caret_position, CaretMovement, EditError, TextTransformation,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditorCommandKind {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    SelectAll,
    Indent,
    Unindent,
    Reindent,
    Transform(TextTransformation),
    Find,
    Replace,
}

/// A native framework command with a weak editor owner. It can be assigned
/// directly to Button, MenuItem and key-binding command properties.
pub struct EditorCommand {
    editor: WeakRef<TextEditor>,
    kind: EditorCommandKind,
    enabled: Cell<bool>,
    changed: Rc<HandlerList<dyn Fn()>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    document: RefCell<Option<(TextDocument, Vec<DocumentSubscription>)>>,
}
impl EditorCommand {
    pub fn new(editor: &Ref<TextEditor>, kind: EditorCommandKind) -> Rc<Self> {
        let command = Rc::new(Self {
            editor: editor.downgrade(),
            kind,
            enabled: Cell::new(false),
            changed: Rc::new(HandlerList::new()),
            subscriptions: RefCell::new(Vec::new()),
            document: RefCell::new(None),
        });
        for event in [
            TextEditor::text_changed_event(),
            TextEditor::selection_changed_event(),
        ] {
            let weak = Rc::downgrade(&command);
            command
                .subscriptions
                .borrow_mut()
                .push(editor.add_disposable_handler(event, move |_, _| {
                    if let Some(command) = weak.upgrade() {
                        command.attach_document();
                        command.refresh();
                    }
                }));
        }
        // Property-wide filtered subscriptions avoid registering a general
        // Text observer, which would materialize large document strings.
        for property in [
            TextEditor::is_read_only_property().as_property(),
            TextEditor::document_property().as_property(),
        ] {
            let weak = Rc::downgrade(&command);
            command
                .subscriptions
                .borrow_mut()
                .push(property.changed().subscribe(move |change| {
                    if let Some(command) = weak.upgrade() {
                        if let (Some(editor), Some(sender)) = (
                            command.editor.upgrade(),
                            change.sender().downcast_ref::<TextEditor>(),
                        ) {
                            if editor == sender.to_ref() {
                                command.attach_document();
                                command.refresh();
                            }
                        }
                    }
                }));
        }
        command.attach_document();
        command.enabled.set(command.can_execute(None));
        command
    }
    pub fn kind(&self) -> EditorCommandKind {
        self.kind
    }
    pub fn as_command(self: &Rc<Self>) -> Rc<dyn ICommand> {
        self.clone()
    }
    /// Requeries custom protected-range policies whose changes are outside the
    /// document/editor event stream.
    pub fn refresh(&self) {
        let enabled = self.can_execute(None);
        if self.enabled.replace(enabled) != enabled {
            for (_, handler) in self.changed.snapshot().iter() {
                handler();
            }
        }
    }
    fn attach_document(self: &Rc<Self>) {
        let Some(editor) = self.editor.upgrade() else {
            return;
        };
        let document = editor.document();
        if self
            .document
            .borrow()
            .as_ref()
            .is_some_and(|(current, _)| current == &document)
        {
            return;
        }
        let previous = self.document.borrow_mut().take();
        if let Some((_, subscriptions)) = previous {
            for subscription in subscriptions {
                subscription.dispose();
            }
        }
        let weak = Rc::downgrade(self);
        let update_subscription = document.update_finished(move || {
            if let Some(command) = weak.upgrade() {
                command.refresh();
            }
        });
        let weak = Rc::downgrade(self);
        let property_subscription = document.property_changed(move |property| {
            if matches!(property, "IsReadOnly" | "CanUndo" | "CanRedo") {
                if let Some(command) = weak.upgrade() {
                    command.refresh();
                }
            }
        });
        *self.document.borrow_mut() =
            Some((document, vec![update_subscription, property_subscription]));
    }
    fn replacement_allowed(editor: &TextEditor, insert: bool, delete_next: bool) -> bool {
        let document = editor.document();
        let selection = editor.selection();
        let mut edits = Vec::with_capacity(selection.segments().len());
        for segment in selection.segments() {
            let mut length = segment.length();
            if delete_next && length == 0 && segment.caret < document.text_length() {
                let Ok(line) = document.get_line_by_offset(segment.caret) else {
                    return false;
                };
                let Ok(text) = document.get_text(line.offset, line.total_length()) else {
                    return false;
                };
                length = next_caret_position(
                    &text,
                    segment.caret - line.offset,
                    CaretMovement::CharacterRight,
                )
                .saturating_sub(segment.caret - line.offset);
            }
            edits.push(DocumentEdit::new(
                segment.start_offset(),
                length,
                if insert { "x" } else { "" },
            ));
        }
        editor.session().validate_document_edits(&edits).is_ok()
    }
}
impl ICommand for EditorCommand {
    fn can_execute(&self, _parameter: Option<&BoxedValue>) -> bool {
        let Some(editor) = self.editor.upgrade() else {
            return false;
        };
        let document = editor.document();
        let writable = !editor.is_read_only() && !document.is_read_only();
        let selected = !editor.selection().is_empty();
        match self.kind {
            EditorCommandKind::Undo => writable && document.can_undo(),
            EditorCommandKind::Redo => writable && document.can_redo(),
            EditorCommandKind::Copy => selected,
            EditorCommandKind::Cut => {
                writable && selected && Self::replacement_allowed(&editor, false, false)
            }
            EditorCommandKind::Paste => writable && Self::replacement_allowed(&editor, true, false),
            EditorCommandKind::Delete => {
                writable
                    && (selected
                        || editor
                            .selection()
                            .segments()
                            .iter()
                            .any(|segment| segment.caret < document.text_length()))
                    && Self::replacement_allowed(&editor, false, true)
            }
            EditorCommandKind::SelectAll => document.text_length() != 0,
            EditorCommandKind::Indent
            | EditorCommandKind::Unindent
            | EditorCommandKind::Replace => writable,
            EditorCommandKind::Reindent => {
                writable && editor.session().indentation_strategy().is_some()
            }
            EditorCommandKind::Transform(_) => writable && document.text_length() != 0,
            EditorCommandKind::Find => true,
        }
    }
    fn execute(&self, _parameter: Option<&BoxedValue>) {
        if !self.can_execute(None) {
            return;
        }
        let Some(editor) = self.editor.upgrade() else {
            return;
        };
        let result: Result<(), EditError> = match self.kind {
            EditorCommandKind::Undo => editor.undo().map(|_| ()),
            EditorCommandKind::Redo => editor.redo().map(|_| ()),
            EditorCommandKind::Cut => {
                editor.cut();
                Ok(())
            }
            EditorCommandKind::Copy => {
                editor.copy();
                Ok(())
            }
            EditorCommandKind::Paste => {
                editor.paste();
                Ok(())
            }
            EditorCommandKind::Delete => {
                let result = editor.session().delete(false, false);
                editor.selection_changed();
                result
            }
            EditorCommandKind::SelectAll => {
                editor.select_all();
                Ok(())
            }
            EditorCommandKind::Indent | EditorCommandKind::Unindent => {
                let result = editor
                    .session()
                    .indent_selection(self.kind == EditorCommandKind::Unindent);
                editor.selection_changed();
                result
            }
            EditorCommandKind::Reindent => {
                let result = editor.session().reindent_selection();
                editor.selection_changed();
                result
            }
            EditorCommandKind::Transform(transformation) => {
                let result = editor.session().transform_text(transformation);
                editor.selection_changed();
                result
            }
            EditorCommandKind::Find => {
                editor.search_panel().show(false);
                Ok(())
            }
            EditorCommandKind::Replace => {
                editor.search_panel().show(true);
                Ok(())
            }
        };
        if let Err(error) = result {
            editor.report_error(error.to_string());
        }
        self.refresh();
    }
    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let handlers = self.changed.clone();
        let token = handlers.add(handler);
        Disposable::create(move || {
            handlers.remove(token);
        })
    }
}
impl Drop for EditorCommand {
    fn drop(&mut self) {
        if let Some((_, subscriptions)) = self.document.get_mut().take() {
            for subscription in subscriptions {
                subscription.dispose();
            }
        }
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_notifications_follow_committed_history_selection_and_readonly() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        let undo = EditorCommand::new(&editor, EditorCommandKind::Undo);
        let events = Rc::new(Cell::new(0));
        let count = events.clone();
        let subscription = undo.can_execute_changed(Rc::new(move || count.set(count.get() + 1)));
        assert!(!undo.can_execute(None));
        editor.document().insert(0, "a😀b").unwrap();
        assert!(undo.can_execute(None));
        assert_eq!(events.get(), 1);
        editor.set_is_read_only(true);
        assert!(!undo.can_execute(None));
        assert_eq!(events.get(), 2);
        editor.set_is_read_only(false);
        assert_eq!(events.get(), 3);
        undo.execute(None);
        assert_eq!(editor.text(), "");
        assert_eq!(events.get(), 4);
        subscription.dispose();
        editor.redo().unwrap();
        assert_eq!(events.get(), 4);
        let copy = EditorCommand::new(&editor, EditorCommandKind::Copy);
        assert!(!copy.can_execute(None));
        editor.select_all();
        assert!(copy.can_execute(None));
        assert_eq!(editor.property_changed_subscriber_count(), 0);
    }
    #[test]
    fn delete_respects_graphemes_protection_and_weak_owner_lifetime() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("😀x").unwrap();
        editor.set_caret_offset(0).unwrap();
        let delete = EditorCommand::new(&editor, EditorCommandKind::Delete);
        assert!(delete.can_execute(None));
        delete.execute(None);
        assert_eq!(editor.text(), "x");
        let provider = Rc::new(
            ferroui_edit_core::editing::TextSegmentReadOnlySectionProvider::new(editor.document()),
        );
        provider.add(0, 1).unwrap();
        editor.session().set_read_only_section_provider(provider);
        assert!(!delete.can_execute(None));
        drop(editor);
        assert!(!delete.can_execute(None));
    }

    #[test]
    fn direct_document_policy_and_history_changes_refresh_bound_commands() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        let document = editor.document();
        document.insert(0, "a").unwrap();
        let undo = EditorCommand::new(&editor, EditorCommandKind::Undo);
        let redo = EditorCommand::new(&editor, EditorCommandKind::Redo);
        let undo_events = Rc::new(Cell::new(0));
        let redo_events = Rc::new(Cell::new(0));
        let count = undo_events.clone();
        let _undo_subscription =
            undo.can_execute_changed(Rc::new(move || count.set(count.get() + 1)));
        let count = redo_events.clone();
        let _redo_subscription =
            redo.can_execute_changed(Rc::new(move || count.set(count.get() + 1)));

        document.set_is_read_only(true);
        assert!(!undo.can_execute(None));
        assert_eq!(undo_events.get(), 1);
        document.set_is_read_only(false);
        assert!(undo.can_execute(None));
        assert_eq!(undo_events.get(), 2);
        document.clear_undo_history().unwrap();
        assert!(!undo.can_execute(None));
        assert_eq!(undo_events.get(), 3);

        document.insert(1, "b").unwrap();
        document.undo().unwrap();
        assert!(redo.can_execute(None));
        assert_eq!(redo_events.get(), 1);
        document.undo_stack().clear_redo_stack().unwrap();
        assert!(!redo.can_execute(None));
        assert_eq!(redo_events.get(), 2);

        document.insert(1, "c").unwrap();
        assert!(undo.can_execute(None));
        let before = undo_events.get();
        document.set_undo_size_limit(0);
        assert!(!undo.can_execute(None));
        assert_eq!(undo_events.get(), before + 1);
    }

    #[test]
    fn language_and_transform_commands_use_the_session_and_group_history() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor
            .set_text("class C {\nvoid M() {\nCall();\n}\n}")
            .unwrap();
        editor.session().set_indentation_strategy(Some(Rc::new(
            ferroui_edit_core::editing::CSharpIndentationStrategy::new(),
        )));
        editor.document().clear_undo_history().unwrap();
        let reindent = editor.command(EditorCommandKind::Reindent);
        assert!(reindent.can_execute(None));
        reindent.execute(None);
        assert_eq!(
            editor.text(),
            "class C {\n\tvoid M() {\n\t\tCall();\n\t}\n}"
        );
        assert_eq!(editor.document().undo_count(), 1);
        let transform = editor.command(EditorCommandKind::Transform(TextTransformation::Uppercase));
        transform.execute(None);
        assert!(editor.text().starts_with("CLASS C"));
        assert_eq!(editor.document().undo_count(), 2);
        editor.undo().unwrap();
        editor.undo().unwrap();
        assert_eq!(editor.text(), "class C {\nvoid M() {\nCall();\n}\n}");
        editor.set_is_read_only(true);
        assert!(!transform.can_execute(None));
        transform.execute(None);
        assert_eq!(editor.text(), "class C {\nvoid M() {\nCall();\n}\n}");
    }
}
