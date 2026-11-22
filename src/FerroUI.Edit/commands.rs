use crate::text_editor::EditTarget;
use crate::{EditorClipboard, TextEditor};
use ferroui_base::input::ICommand;
use ferroui_base::reactive::{Disposable, Subscription, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{BoxedValue, Ref, WeakRef};
use ferroui_edit_core::document::{DocumentEdit, DocumentSubscription, TextDocument};
use ferroui_edit_core::editing::{CaretMovement, EditError, TextTransformation};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A caret or selection operation performed in the editor's formatted view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditorNavigation {
    Caret {
        movement: CaretMovement,
        extend_selection: bool,
    },
    Page {
        upward: bool,
        extend_selection: bool,
    },
    /// Extends a rectangle horizontally, vertically, or to a row boundary.
    Rectangle(CaretMovement),
}

impl EditorNavigation {
    pub(crate) fn is_available(self, editor: &TextEditor) -> bool {
        match self {
            Self::Rectangle(movement) => {
                editor.options().enable_rectangular_selection
                    && !matches!(movement, CaretMovement::DocumentStart | CaretMovement::DocumentEnd)
            }
            _ => true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditorCommandKind {
    Undo,
    Redo,
    ToggleOverstrike,
    Cut,
    Copy,
    Paste,
    /// Removes the selection without extending it to the next character.
    DeleteSelection,
    /// Deletes forward at the caret or removes the selected text.
    Delete,
    Backspace,
    DeletePreviousWord,
    DeleteNextWord,
    /// Inserts and indents a line break while the text area has keyboard focus.
    InsertLineBreak,
    /// Inserts indentation at the active caret column or indents a multiline selection.
    InsertTab,
    Navigate(EditorNavigation),
    SelectAll,
    Indent,
    Unindent,
    Reindent,
    Transform(TextTransformation),
    Find,
    Replace,
    FindNext,
    FindPrevious,
    ReplaceNext,
    ReplaceAll,
    CloseSearchPanel,
}

/// A native framework command with a weak editor owner. It can be assigned
/// directly to Button, MenuItem and key-binding command properties.
pub struct EditorCommand {
    editor: WeakRef<TextEditor>,
    kind: EditorCommandKind,
    enabled: Cell<bool>,
    changed: Rc<HandlerList<dyn Fn()>>,
    subscriptions: RefCell<Vec<Subscription>>,
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
            TextEditor::search_state_changed_event(),
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
            TextEditor::options_property().as_property(),
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
    fn paste_allowed(editor: &TextEditor) -> bool {
        let document = editor.document();
        let selection = editor.selection();
        if selection.is_rectangular() || selection.segments().len() == 1 {
            let target = EditTarget::new(editor);
            let session = editor.session();
            let provider = session.read_only_section_provider();
            return provider.can_insert(selection.primary().caret)
                && target.is_current(editor)
                && editor.selection() == selection
                && Rc::ptr_eq(&provider, &session.read_only_section_provider())
                && !session.is_read_only()
                && !document.is_read_only();
        }
        let edits: Vec<_> = selection.segments().iter()
            .map(|segment| DocumentEdit::new(segment.start_offset(), segment.length(), "x"))
            .collect();
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
        match self.kind {
            EditorCommandKind::Undo => writable && document.can_undo(),
            EditorCommandKind::Redo => writable && document.can_redo(),
            EditorCommandKind::ToggleOverstrike => editor.options().allow_toggle_overstrike_mode,
            EditorCommandKind::Copy => EditorClipboard::can_copy(&editor),
            EditorCommandKind::Cut => writable && EditorClipboard::can_cut(&editor),
            EditorCommandKind::Paste => writable && Self::paste_allowed(&editor),
            EditorCommandKind::DeleteSelection
            | EditorCommandKind::InsertLineBreak
            | EditorCommandKind::SelectAll => true,
            EditorCommandKind::Delete => writable && editor.text_area().can_delete_forward(&editor),
            EditorCommandKind::Backspace | EditorCommandKind::DeletePreviousWord
            | EditorCommandKind::DeleteNextWord => writable,
            EditorCommandKind::Navigate(navigation) => navigation.is_available(&editor),
            EditorCommandKind::InsertTab
            | EditorCommandKind::Indent
            | EditorCommandKind::Unindent
            | EditorCommandKind::Replace => writable,
            EditorCommandKind::Reindent => {
                writable && editor.session().indentation_strategy().is_some()
            }
            EditorCommandKind::Transform(_) => writable && document.text_length() != 0,
            EditorCommandKind::Find => true,
            EditorCommandKind::FindNext
            | EditorCommandKind::FindPrevious
            | EditorCommandKind::CloseSearchPanel => {
                editor.installed_search_panel().is_some_and(|panel| panel.is_open())
            }
            EditorCommandKind::ReplaceNext | EditorCommandKind::ReplaceAll => {
                writable && editor.installed_search_panel().is_some_and(|panel| {
                    panel.is_open() && panel.is_replace_mode()
                })
            }
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
            EditorCommandKind::ToggleOverstrike => {
                let area = editor.text_area();
                area.set_overstrike_mode(!area.overstrike_mode());
                Ok(())
            }
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
            EditorCommandKind::DeleteSelection => editor.text_area().delete_selection(&editor),
            EditorCommandKind::Delete => editor.text_area().delete_directionally(&editor, false, false),
            EditorCommandKind::Backspace => editor.text_area().delete_directionally(&editor, true, false),
            EditorCommandKind::DeletePreviousWord => editor.text_area().delete_directionally(&editor, true, true),
            EditorCommandKind::DeleteNextWord => editor.text_area().delete_directionally(&editor, false, true),
            EditorCommandKind::InsertLineBreak => {
                let area = editor.text_area();
                if area.is_focused() {
                    area.insert_line_break()
                } else {
                    Ok(())
                }
            }
            EditorCommandKind::SelectAll => {
                editor.select_all();
                Ok(())
            }
            EditorCommandKind::Navigate(navigation) => {
                editor.text_area().navigate(&editor, navigation)
            }
            EditorCommandKind::InsertTab => editor.indent(false, true),
            EditorCommandKind::Indent | EditorCommandKind::Unindent => {
                editor.indent(self.kind == EditorCommandKind::Unindent, false)
            }
            EditorCommandKind::Reindent => editor.reindent_selection(),
            EditorCommandKind::Transform(transformation) => editor.transform_text(transformation),
            EditorCommandKind::Find => {
                editor.search_panel().show(false);
                Ok(())
            }
            EditorCommandKind::Replace => {
                editor.search_panel().show(true);
                Ok(())
            }
            EditorCommandKind::FindNext
            | EditorCommandKind::FindPrevious
            | EditorCommandKind::ReplaceNext
            | EditorCommandKind::ReplaceAll
            | EditorCommandKind::CloseSearchPanel => {
                if let Some(panel) = editor.installed_search_panel() {
                    panel.run_search_command(self.kind);
                }
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

    fn control_key(editor: &TextEditor, key: ferroui_base::input::Key) {
        editor_key(editor, key, ferroui_base::input::KeyModifiers::CONTROL);
    }

    fn editor_key(
        editor: &TextEditor,
        key: ferroui_base::input::Key,
        modifiers: ferroui_base::input::KeyModifiers,
    ) {
        use ferroui_base::input::{InputElement, KeyEventArgs};
        let mut args = KeyEventArgs::new();
        args.key = key;
        args.key_modifiers = modifiers;
        args.set_routed_event(Some(InputElement::key_down_event()));
        editor.text_area().raise_event(&args);
        assert!(args.handled());
    }

    #[test]
    fn select_all_resets_a_virtual_caret_in_an_empty_document() {
        use ferroui_base::input::{Key, KeyModifiers};
        use ferroui_edit_core::editing::Selection;

        let _scope = crate::tests::test_scope();
        for read_only in [false, true] {
            let editor = crate::tests::laid_out_editor("");
            let mut options = editor.options();
            options.enable_virtual_space = true;
            editor.set_options(options);
            editor.document().clear_undo_history().unwrap();
            editor.set_is_read_only(read_only);
            let command = editor.command(EditorCommandKind::SelectAll);
            for via_keyboard in [false, true] {
                editor.session().set_caret_position(0, 4).unwrap();
                assert_eq!(editor.session().caret_virtual_space(), 4);
                assert!(command.can_execute(None));
                if via_keyboard {
                    editor_key(&editor, Key::A, KeyModifiers::CONTROL);
                } else {
                    command.execute(None);
                }
                assert_eq!(editor.selection(), Selection::empty(0));
                assert!(!editor.document().can_undo());
            }
        }
    }

    #[test]
    fn escape_preserves_selection_with_the_search_panel_closed() {
        use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
        use ferroui_edit_core::editing::Selection;

        let _scope = crate::tests::test_scope();
        let editor = crate::tests::laid_out_editor("one two");
        editor.document().clear_undo_history().unwrap();
        let selection = Selection::simple(1, 3);
        editor.set_selection(selection.clone()).unwrap();
        for modifiers in [KeyModifiers::NONE, KeyModifiers::CONTROL] {
            let mut args = KeyEventArgs::new();
            args.key = Key::Escape;
            args.key_modifiers = modifiers;
            args.set_routed_event(Some(InputElement::key_down_event()));
            editor.text_area().raise_event(&args);
            assert_eq!(args.handled(), modifiers.is_empty());
            assert_eq!(editor.selection(), selection);
            assert!(!editor.document().can_undo());
        }
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn only_bound_modifier_combinations_change_the_document_or_caret() {
        use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
        use ferroui_edit_core::editing::Selection;

        let _scope = crate::tests::test_scope();
        let text = "zero one two\r\nabcdef ghijkl\r\nthree four five";
        let ctrl = KeyModifiers::CONTROL;
        let alt = KeyModifiers::ALT;
        let shift = KeyModifiers::SHIFT;
        for (key, modifiers) in [
            (Key::Left, alt),
            (Key::Left, ctrl | alt),
            (Key::Up, ctrl),
            (Key::Down, ctrl),
            (Key::Down, ctrl | alt | shift),
            (Key::Home, ctrl | alt | shift),
            (Key::Delete, ctrl | shift),
            (Key::Back, alt),
            (Key::Return, ctrl),
        ] {
            let editor = crate::tests::laid_out_editor(text);
            let selection = Selection::simple(4, 6);
            editor.set_selection(selection.clone()).unwrap();
            editor.document().clear_undo_history().unwrap();
            let mut args = KeyEventArgs::new();
            args.key = key;
            args.key_modifiers = modifiers;
            args.set_routed_event(Some(InputElement::key_down_event()));
            editor.text_area().raise_event(&args);
            assert!(!args.handled(), "the editor consumed {modifiers:?}+{key:?}");
            assert_eq!(editor.selection(), selection);
            assert_eq!(editor.text(), text);
            assert!(!editor.document().can_undo());
        }

        let editor = crate::tests::laid_out_editor(text);
        editor.set_caret_offset(8).unwrap();
        editor_key(&editor, Key::Left, ctrl);
        assert_eq!(editor.caret_offset(), 5);
        editor.set_caret_offset(8).unwrap();
        editor_key(&editor, Key::Left, alt | shift);
        assert!(editor.selection().is_rectangular());
        assert_eq!(editor.selected_text().unwrap(), "e");
        editor.set_caret_offset(8).unwrap();
        editor_key(&editor, Key::Left, ctrl | alt | shift);
        assert!(editor.selection().is_rectangular());
        assert_eq!(editor.selected_text().unwrap(), "one");

        let mut options = editor.options();
        options.enable_rectangular_selection = false;
        editor.set_options(options);
        editor.set_caret_offset(8).unwrap();
        editor_key(&editor, Key::Left, alt | shift);
        assert_eq!(editor.selection(), Selection::simple(8, 7));
        editor_key(&editor, Key::Home, ctrl | shift);
        assert_eq!(editor.selection(), Selection::simple(8, 0));
        editor.set_caret_offset(6).unwrap();
        editor_key(&editor, Key::Return, shift);
        assert_eq!(editor.text(), "zero o\r\nne two\r\nabcdef ghijkl\r\nthree four five");
    }

    #[test]
    fn tab_and_shift_tab_edit_only_writable_indentation_and_keep_the_directed_selection() {
        use ferroui_base::input::{Key, KeyModifiers};
        use ferroui_edit_core::editing::{Selection, TextSegmentReadOnlySectionProvider};
        use EditorCommandKind::{Indent, InsertTab, Unindent};

        let _scope = crate::tests::test_scope();
        for (unindent, text, anchor, caret, protected, expected, next_anchor, next_caret) in [
            (false, "a\r\nb\r\nc\r\nd", 0, 7, (1, 3), "    a\r\nb\r\n    c\r\nd", 0, 15),
            (false, "a\r\nb\r\nc\r\nd", 7, 0, (1, 3), "    a\r\nb\r\n    c\r\nd", 15, 0),
            (false, "a\r\nb\r\nc\r\nd", 0, 6, (1, 3), "    a\r\nb\r\nc\r\nd", 0, 10),
            (true, "    a\r\n    b", 0, 8, (0, 1), " a\r\nb", 0, 4),
            (true, "    a\r\n    b", 0, 8, (1, 1), "   a\r\nb", 0, 6),
            (true, "    a\r\n    b", 0, 7, (0, 0), "a\r\n    b", 0, 3),
        ] {
            let commands: &[Option<EditorCommandKind>] = if unindent {
                &[None, Some(Unindent)]
            } else {
                &[None, Some(Indent), Some(InsertTab)]
            };
            for &command in commands {
                let editor = crate::tests::laid_out_editor(text);
                let mut options = editor.options();
                options.convert_tabs_to_spaces = true;
                editor.set_options(options);
                let document = editor.document();
                document.clear_undo_history().unwrap();
                let (offset, length) = protected;
                if length != 0 {
                    let provider = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
                    provider.add(offset, length).unwrap();
                    editor.session().set_read_only_section_provider(provider);
                }
                let original = Selection::simple(anchor, caret);
                let updated = Selection::simple(next_anchor, next_caret);
                editor.set_selection(original.clone()).unwrap();
                let start = document.create_anchor(0).unwrap();
                if let Some(kind) = command {
                    editor.command(kind).execute(None);
                } else {
                    let modifiers = if unindent { KeyModifiers::SHIFT } else { KeyModifiers::NONE };
                    editor_key(&editor, Key::Tab, modifiers);
                }
                assert_eq!(editor.text(), expected, "{text:?}, {protected:?}");
                assert_eq!(editor.selection(), updated);
                assert_eq!(start.offset().unwrap(), 0);
                assert_eq!(document.undo_count(), 1);
                assert!(editor.undo().unwrap());
                assert_eq!(editor.text(), text);
                assert_eq!(editor.selection(), original);
                assert!(editor.redo().unwrap());
                assert_eq!(editor.text(), expected);
                assert_eq!(editor.selection(), updated);
            }
        }
    }

    #[test]
    fn single_line_tab_uses_the_active_logical_column_and_protected_spans() {
        use ferroui_base::input::{Key, KeyModifiers};
        use ferroui_edit_core::editing::{Selection, TextSegmentReadOnlySectionProvider};

        let _scope = crate::tests::test_scope();
        for (text, anchor, caret, protected, expected, next_caret, redo_selection) in [
            ("abcXYZdef", 1, 8, Some((3, 3)), "aXYZ    f", 8, (1, 8)),
            ("abcXYZdef", 8, 1, Some((3, 3)), "aXYZ   f", 7, (7, 1)),
            ("aXYZb", 0, 5, Some((1, 3)), "XYZ   ", 6, (0, 6)),
            ("XYZ", 0, 0, Some((0, 3)), "    XYZ", 4, (4, 4)),
            ("XYZ", 1, 1, Some((0, 3)), "XYZ", 1, (1, 1)),
            ("\tab", 2, 2, None, "\ta  b", 4, (4, 4)),
            ("abcdef", 1, 4, None, "a    ef", 5, (1, 5)),
        ] {
            let editor = crate::tests::laid_out_editor(text);
            let mut options = editor.options();
            options.convert_tabs_to_spaces = true;
            editor.set_options(options);
            let document = editor.document();
            document.clear_undo_history().unwrap();
            if let Some((offset, length)) = protected {
                let provider = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
                provider.add(offset, length).unwrap();
                editor.session().set_read_only_section_provider(provider);
            }
            let original = Selection::simple(anchor, caret);
            editor.set_selection(original.clone()).unwrap();
            editor_key(&editor, Key::Tab, KeyModifiers::NONE);
            assert_eq!(editor.text(), expected, "{text:?} at {caret}");
            assert_eq!(editor.selection(), Selection::empty(next_caret));
            assert_eq!(document.undo_count(), usize::from(text != expected));
            if text != expected {
                assert!(editor.undo().unwrap());
                assert_eq!(editor.text(), text);
                assert_eq!(editor.selection(), original);
                assert!(editor.redo().unwrap());
                assert_eq!(editor.text(), expected);
                assert_eq!(
                    editor.selection(),
                    Selection::simple(redo_selection.0, redo_selection.1),
                    "{text:?} at {caret} after redo"
                );
            }
        }
    }

    #[test]
    fn tab_can_pass_to_focus_navigation_and_indentation_rejects_a_rebound_document() {
        use ferroui_base::input::{Key, KeyModifiers};
        use ferroui_base::layout::ILayoutManager;
        use ferroui_controls::testing::TestServices;
        use ferroui_controls::{Button, Control, StackPanel, Window, as_clickable_control};

        let _scope = crate::tests::test_scope_with_services(TestServices::focusable_window());
        let editor = crate::tests::laid_out_editor("                   ");
        editor.set_caret_offset(0).unwrap();
        editor.set_height(100.0);
        let mut options = editor.options();
        options.accepts_tab = false;
        options.convert_tabs_to_spaces = true;
        editor.set_options(options);
        let before = Button::new();
        let after = Button::new();
        let insert_tab = editor.command(EditorCommandKind::InsertTab);
        after.set_command(Some(insert_tab.as_command()));
        let panel = StackPanel::new();
        panel.children().add(before.clone());
        panel.children().add(editor.clone());
        panel.children().add(after.clone());
        let window = Window::new();
        window.set_width(360.0);
        window.set_height(180.0);
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        window.layout_manager().execute_layout_pass();
        for (modifiers, neighbor) in [
            (KeyModifiers::NONE, after.clone()),
            (KeyModifiers::SHIFT, before),
        ] {
            assert!(editor.text_area().focus());
            editor_key(&editor, Key::Tab, modifiers);
            assert!(neighbor.is_focused());
        }
        assert_eq!(editor.text(), "                   ");
        editor.command(EditorCommandKind::Unindent).execute(None);
        assert_eq!(editor.text().len(), 15);

        editor.set_text("tab stop").unwrap();
        editor.set_caret_offset(1).unwrap();
        let click_insert_tab = || as_clickable_control(&after).unwrap().raise_click();
        editor.set_is_read_only(true);
        assert!(!insert_tab.can_execute(None));
        click_insert_tab();
        assert_eq!(editor.text(), "tab stop");
        editor.set_is_read_only(false);
        assert!(insert_tab.can_execute(None));
        click_insert_tab();
        assert_eq!(editor.text(), "t   ab stop");
        assert_eq!(editor.caret_offset(), 4);
        window.close();

        for (unindent, from_tab) in [(false, true), (false, false), (true, true)] {
            let editor = crate::tests::laid_out_editor("    text\r\n    second");
            let document = editor.document();
            editor.set_caret_offset(0).unwrap();
            document.clear_undo_history().unwrap();
            let weak = editor.downgrade();
            let _subscription = document.update_started(move || {
                weak.upgrade().unwrap().set_document(TextDocument::from_text("replacement"));
            });
            assert!(matches!(editor.indent(unindent, from_tab), Err(EditError::StaleEditPlan)));
            assert_eq!(document.text(), "    text\r\n    second");
            assert!(!document.can_undo());
            assert_eq!(editor.text(), "replacement");
        }
    }

    #[test]
    fn directional_deletion_matches_protection_word_starts_and_codepoint_boundaries() {
        use ferroui_base::input::{Key, KeyModifiers};
        use ferroui_edit_core::editing::{Selection, TextSegmentReadOnlySectionProvider};
        use EditorCommandKind::{Backspace, Delete, DeleteNextWord, DeletePreviousWord};

        let _scope = crate::tests::test_scope();
        for (kind, original, anchor, active, protected, expected, caret) in [
            (DeleteNextWord, "aXYb next", 0, 0, Some((1, 2)), "XYnext", 0),
            (DeletePreviousWord, "aXYb next", 4, 4, Some((1, 2)), "XY next", 0),
            (DeleteNextWord, "XXab next", 0, 0, Some((0, 2)), "XXnext", 0),
            (DeletePreviousWord, "aXYb next", 4, 4, Some((0, 4)), "aXYb next", 4),
            (DeletePreviousWord, "aXYb next", 4, 0, Some((1, 2)), "XY next", 0),
            (DeleteNextWord, "one\r\n  next", 3, 3, None, "onenext", 3),
            (DeleteNextWord, "one\r\n  next", 0, 0, None, "\r\n  next", 0),
            (DeletePreviousWord, "one  \r\n  next", 7, 7, None, "one    next", 5),
            (DeleteNextWord, "one\r\n  \r\nlast", 3, 3, None, "one\r\nlast", 3),
            (DeleteNextWord, "one... next", 0, 0, None, "... next", 0),
            (Delete, "e\u{301}😀 word", 0, 0, None, "😀 word", 0),
            (Backspace, "e\u{301}😀 word", 2, 2, None, "e😀 word", 1),
            (Backspace, "👨‍👩", 5, 5, None, "👨‍", 3),
            (Delete, "👨‍👩", 0, 0, None, "‍👩", 0),
        ] {
            for routed in [false, true] {
                let editor = crate::tests::laid_out_editor(original);
                let document = editor.document();
                document.clear_undo_history().unwrap();
                if let Some((offset, length)) = protected {
                    let policy = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
                    policy.add(offset, length).unwrap();
                    editor.session().set_read_only_section_provider(policy);
                }
                let original_selection = Selection::simple(anchor, active);
                editor.set_selection(original_selection.clone()).unwrap();
                if routed {
                    match kind {
                        Delete => editor_key(&editor, Key::Delete, KeyModifiers::NONE),
                        Backspace => editor_key(&editor, Key::Back, KeyModifiers::SHIFT),
                        DeleteNextWord => control_key(&editor, Key::Delete),
                        DeletePreviousWord => control_key(&editor, Key::Back),
                        _ => unreachable!(),
                    }
                } else {
                    let command = editor.command(kind);
                    assert!(command.can_execute(None));
                    command.execute(None);
                }
                assert_eq!(editor.text(), expected, "{kind:?} at {active} in {original:?}");
                assert_eq!(editor.selection(), Selection::empty(caret), "{kind:?} at {active} in {original:?}");
                assert_eq!(document.undo_count(), usize::from(expected != original));
                if expected != original {
                    assert!(editor.undo().unwrap());
                    assert_eq!(editor.text(), original);
                    assert_eq!(editor.selection(), original_selection);
                    assert!(editor.redo().unwrap());
                    assert_eq!(editor.text(), expected);
                    assert_eq!(editor.selection(), Selection::empty(caret), "redo: {kind:?} at {active} in {original:?}");
                }
            }
        }
    }

    #[test]
    fn directional_deletion_uses_fold_boundaries_and_aborts_after_rebinding() {
        use EditorCommandKind::{Backspace, Delete, DeleteNextWord};
        let _scope = crate::tests::test_scope();
        let original = "a{\n hidden\n}z\nlast";
        for (kind, caret) in [(Delete, 1), (DeleteNextWord, 1), (Backspace, 12)] {
            let editor = crate::tests::laid_out_editor(original);
            let fold = editor.install_folding().create_folding(1, 12).unwrap();
            fold.set_is_folded(true);
            editor.set_caret_offset(caret).unwrap();
            editor.command(kind).execute(None);
            assert_eq!(editor.text(), "az\nlast", "{kind:?}");
            assert_eq!(editor.caret_offset(), 1);
            assert!(editor.undo().unwrap());
            assert_eq!(editor.text(), original);
            assert_eq!(editor.caret_offset(), caret);
        }
        let editor = crate::tests::laid_out_editor("first second");
        editor.set_caret_offset(0).unwrap();
        let document = editor.document();
        document.clear_undo_history().unwrap();
        let weak = editor.downgrade();
        let _rebind = document.update_started(move || {
            weak.upgrade().unwrap().set_document(TextDocument::from_text("replacement"));
        });
        assert!(matches!(
            editor.text_area().delete_directionally(&editor, false, true),
            Err(EditError::StaleEditPlan)
        ));
        assert_eq!(editor.text(), "replacement");
        assert_eq!(document.text(), "first second");
        assert!(!document.can_undo());
    }

    #[test]
    fn delete_line_command_and_shortcut_match_reference_protected_cases() {
        use ferroui_base::input::Key;
        use ferroui_edit_core::editing::{Selection, TextSegmentReadOnlySectionProvider};
        let _scope = crate::tests::test_scope();
        for (original, anchor, end, protected, expected, caret, span) in [
            ("aXXb\r\nsecond", 3, 3, Some((1, 2)), "XXsecond", 0, None),
            ("zero\r\nfirst\r\nsecond", 14, 6, Some((7, 2)), "zero\r\nir", 6, None),
            ("first\r\nsecond\r\nthird", 1, 7, None, "third", 0, None),
            ("first\r\nsecond", 2, 2, Some((0, 7)), "first\r\nsecond", 2, Some((0, 7))),
            ("first\r\n", 7, 7, None, "first\r\n", 7, None),
        ] {
            for use_shortcut in [false, true] {
                let editor = crate::tests::laid_out_editor(original);
                let document = editor.document();
                document.clear_undo_history().unwrap();
                if let Some((offset, length)) = protected {
                    let policy = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
                    policy.add(offset, length).unwrap();
                    editor.session().set_read_only_section_provider(policy);
                }
                let original_selection = Selection::simple(anchor, end);
                editor.set_selection(original_selection.clone()).unwrap();
                if use_shortcut {
                    control_key(&editor, Key::D);
                } else {
                    editor.command(EditorCommandKind::Transform(TextTransformation::DeleteLines)).execute(None);
                }
                assert_eq!(document.text(), expected, "{original:?}");
                assert_eq!(editor.caret_offset(), caret, "{original:?}");
                let selection = editor.selection().primary();
                assert_eq!((!selection.is_empty()).then(|| (selection.start_offset(), selection.end_offset())), span);
                assert_eq!(document.undo_count(), usize::from(expected != original));
                if expected != original {
                    assert!(editor.undo().unwrap());
                    assert_eq!(document.text(), original);
                    assert_eq!(editor.selection(), original_selection);
                    assert!(editor.redo().unwrap());
                    assert_eq!(document.text(), expected);
                    assert_eq!(editor.caret_offset(), caret);
                }
            }
        }
    }

    #[test]
    fn delete_line_command_uses_the_active_row_of_a_rectangle_with_no_text() {
        use ferroui_base::input::Key;
        use ferroui_edit_core::editing::{RectangleSelection, Selection, TextSegmentReadOnlySectionProvider};

        let _scope = crate::tests::test_scope();
        let lines = "aaa\r\nbbb\r\nccc\r\nlast";
        for (text, corners, protection, expected, caret, protected_range) in [
            (lines, [0, 1, 2, 1], None, "aaa\r\nbbb\r\nlast", 10, None),
            (lines, [2, 1, 0, 1], None, "bbb\r\nccc\r\nlast", 0, None),
            (lines, [0, 1, 2, 1], Some((11, 2)), "aaa\r\nbbb\r\ncclast", 10, None),
            (lines, [0, 1, 2, 1], Some((10, 5)), lines, 11, Some((10, 15))),
            ("a\r\nb\r\nc\r\nlast", [0, 7, 2, 9], None, "a\r\nb\r\nlast", 6, None),
            ("aaa\r\nbbb\r\n", [0, 0, 2, 0], None, "aaa\r\nbbb\r\n", 10, None),
            (lines, [0, 1, 2, 2], None, "last", 0, None),
        ] {
            for shortcut in [false, true] {
                let editor = crate::tests::laid_out_editor(text);
                let mut options = editor.options();
                options.enable_virtual_space = true;
                editor.set_options(options);
                let document = editor.document();
                document.clear_undo_history().unwrap();
                let [anchor_line, anchor_column, caret_line, caret_column] = corners;
                let rectangle = RectangleSelection { anchor_line, anchor_column, caret_line, caret_column }
                    .to_document_selection(&document, 4)
                    .unwrap();
                editor.set_selection(rectangle.clone()).unwrap();
                if let Some((offset, length)) = protection {
                    let provider = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
                    provider.add(offset, length).unwrap();
                    editor.session().set_read_only_section_provider(provider);
                }

                if shortcut {
                    control_key(&editor, Key::D);
                } else {
                    editor.command(EditorCommandKind::Transform(TextTransformation::DeleteLines)).execute(None);
                }
                let expected_selection = protected_range.map_or_else(|| Selection::empty(caret), |(start, end)| Selection::simple(start, end));
                assert_eq!(document.text(), expected, "{text:?}, {corners:?}, {protection:?}, shortcut={shortcut}");
                assert_eq!(editor.selection(), expected_selection);
                assert_eq!(editor.caret_offset(), caret);
                let edited = text != expected;
                assert_eq!(document.undo_count(), usize::from(edited));
                if edited {
                    assert!(editor.undo().unwrap());
                    assert_eq!(document.text(), text);
                    assert_eq!(editor.selection(), rectangle);
                    assert!(editor.redo().unwrap());
                    assert_eq!(document.text(), expected);
                    assert_eq!(editor.selection(), expected_selection);
                }
            }
        }
    }

    #[test]
    fn reindent_command_and_shortcut_include_the_endpoint_line() {
        use ferroui_base::input::Key;
        use ferroui_edit_core::editing::{IndentationError, IndentationStrategy, RectangleSelection, Selection};

        #[derive(Default)]
        struct RecordRange(Cell<(usize, usize)>);

        impl IndentationStrategy for RecordRange {
            fn indent_line(&self, _: &TextDocument, _: usize) -> Result<Vec<DocumentEdit>, IndentationError> {
                Ok(Vec::new())
            }

            fn indent_lines(&self, _: &TextDocument, first: usize, last: usize) -> Result<Vec<DocumentEdit>, IndentationError> {
                self.0.set((first, last));
                Ok(Vec::new())
            }
        }

        let _scope = crate::tests::test_scope();
        let editor = crate::tests::laid_out_editor("a\r\nb\r\nc");
        let strategy = Rc::new(RecordRange::default());
        editor.session().set_indentation_strategy(Some(strategy.clone()));
        for (anchor, caret, last) in [(0, 3, 2), (3, 0, 2), (3, 3, 3)] {
            editor.set_selection(Selection::simple(anchor, caret)).unwrap();
            control_key(&editor, Key::I);
            assert_eq!(strategy.0.replace((0, 0)), (1, last));
            editor.command(EditorCommandKind::Reindent).execute(None);
            assert_eq!(strategy.0.replace((0, 0)), (1, last));
        }

        let mut options = editor.options();
        options.enable_virtual_space = true;
        editor.set_options(options);
        let rectangle = RectangleSelection {
            anchor_line: 1,
            anchor_column: 5,
            caret_line: 2,
            caret_column: 7,
        }.to_document_selection(&editor.document(), 4).unwrap();
        assert!(!rectangle.is_empty());
        assert_eq!(rectangle.length(), 0);
        editor.set_selection(rectangle).unwrap();
        control_key(&editor, Key::I);
        assert_eq!(strategy.0.replace((0, 0)), (1, 3));
        editor.command(EditorCommandKind::Reindent).execute(None);
        assert_eq!(strategy.0.replace((0, 0)), (1, 3));
    }

    #[test]
    fn indentation_and_transforms_follow_the_reference_scope_for_text_free_rectangles() {
        use ferroui_base::input::{Key, KeyModifiers};
        use ferroui_edit_core::editing::RectangleSelection;
        use EditorCommandKind::{InsertTab, Transform, Unindent};

        let _scope = crate::tests::test_scope();
        let text = "    one\r\n    two\r\n    three\r\n    last";
        for (command, columns, shortcut, expected) in [
            (Unindent, (1, 1), Some(KeyModifiers::SHIFT), "    one\r\n    two\r\nthree\r\n    last"),
            (InsertTab, (1, 1), Some(KeyModifiers::NONE), "    one\r\n        two\r\n        three\r\n    last"),
            (Transform(TextTransformation::Uppercase), (13, 17), None, "    ONE\r\n    TWO\r\n    THREE\r\n    LAST"),
            (Transform(TextTransformation::RemoveLeadingWhitespace), (13, 17), None, "one\r\ntwo\r\nthree\r\nlast"),
        ] {
            for key in [None, shortcut].into_iter().take(if shortcut.is_some() { 2 } else { 1 }) {
                let editor = crate::tests::laid_out_editor(text);
                let mut options = editor.options();
                options.enable_virtual_space = true;
                options.convert_tabs_to_spaces = true;
                editor.set_options(options);
                let document = editor.document();
                document.clear_undo_history().unwrap();
                let rectangle = RectangleSelection {
                    anchor_line: 1,
                    anchor_column: columns.0,
                    caret_line: 2,
                    caret_column: columns.1,
                }.to_document_selection(&document, 4).unwrap();
                assert_eq!(rectangle.length(), 0);
                editor.set_selection(rectangle.clone()).unwrap();
                if let Some(key) = key {
                    editor_key(&editor, Key::Tab, key);
                } else {
                    editor.command(command).execute(None);
                }
                assert_eq!(document.text(), expected, "{command:?}, {key:?}");
                assert_eq!(document.undo_count(), 1);
                assert!(editor.undo().unwrap());
                assert_eq!(document.text(), text);
                assert_eq!(editor.selection(), rectangle);
                assert!(editor.redo().unwrap());
                assert_eq!(document.text(), expected);
            }
        }
    }

    #[test]
    fn line_edits_stop_if_an_update_callback_rebinds_the_editor() {
        use ferroui_edit_core::editing::CSharpIndentationStrategy;
        let _scope = crate::tests::test_scope();
        let original = "class C {\r\nvoid M() {}\r\n}";
        for reindent in [false, true] {
            let editor = crate::tests::laid_out_editor(original);
            editor.session().set_indentation_strategy(Some(Rc::new(CSharpIndentationStrategy::new())));
            let document = editor.document();
            document.clear_undo_history().unwrap();
            let weak = editor.downgrade();
            let _rebind = document.update_started(move || {
                weak.upgrade().unwrap().set_document(TextDocument::from_text("replacement"));
            });
            let result = if reindent { editor.reindent_selection() } else { editor.delete_lines() };
            assert!(matches!(result, Err(EditError::StaleEditPlan)));
            assert_eq!(editor.text(), "replacement");
            assert_eq!(document.text(), original);
            assert!(!document.can_undo());
        }
    }

    #[test]
    fn native_navigation_works_in_read_only_documents_and_keeps_utf16_boundaries() {
        use CaretMovement::*;
        let _scope = crate::tests::test_scope();
        let unicode = "e\u{301}😀 word\r\nlast";
        let indented = "    abcd\r\nxyz";
        for (text, start, movement, extend_selection, end, selected) in [
            (unicode, 0, CharacterRight, false, 2, ""),
            (unicode, 2, CharacterRight, true, 4, "😀"),
            (unicode, 4, CharacterLeft, true, 2, "😀"),
            (unicode, 5, WordRight, false, 9, ""),
            (unicode, 9, WordLeft, false, 5, ""),
            (indented, 7, LineStart, false, 4, ""),
            (indented, 4, LineStart, false, 0, ""),
            (unicode, 0, LineEnd, false, 9, ""),
            (unicode, 0, DocumentEnd, false, 15, ""),
            (unicode, 15, DocumentStart, true, 0, unicode),
        ] {
            let editor = crate::tests::laid_out_editor(text);
            editor.set_is_read_only(true);
            editor.set_caret_offset(start).unwrap();
            let command = editor.command(EditorCommandKind::Navigate(EditorNavigation::Caret {
                movement, extend_selection,
            })).as_command();
            assert!(command.can_execute(None));
            command.execute(None);
            let selection = editor.selection().primary();
            assert_eq!(selection.anchor, if extend_selection { start } else { end }, "{movement:?}");
            assert_eq!(selection.caret, end, "{movement:?}");
            assert_eq!(editor.selected_text().unwrap(), selected, "{movement:?}");
        }
    }

    #[test]
    fn rectangle_navigation_observes_the_editor_option() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        let command = editor.command(EditorCommandKind::Navigate(EditorNavigation::Rectangle(CaretMovement::LineDown)));
        let changes = Rc::new(Cell::new(0));
        let count = changes.clone();
        let _subscription = command.can_execute_changed(Rc::new(move || count.set(count.get() + 1)));
        assert!(command.can_execute(None));
        let mut options = editor.options();
        options.enable_rectangular_selection = false;
        editor.set_options(options);
        assert!(!command.can_execute(None));
        assert_eq!(changes.get(), 1);
    }

    #[test]
    fn navigation_bound_to_a_button_moves_inside_a_wrapped_logical_line() {
        use ferroui_base::{Rect, Size};
        use ferroui_controls::{Button, as_clickable_control};
        let _scope = crate::tests::test_scope();
        let text = format!("{}\nlast", "0123456789 ".repeat(15));
        let editor = crate::tests::laid_out_editor(&text);
        editor.set_word_wrap(true);
        let area = editor.text_area();
        let size = Size::new(90.0, 140.0);
        area.measure(size);
        area.arrange(Rect::from_size(size));
        editor.set_caret_offset(1).unwrap();

        let command = editor.command(EditorCommandKind::Navigate(EditorNavigation::Caret {
            movement: CaretMovement::LineDown,
            extend_selection: true,
        }));
        let button = Button::new();
        button.set_command(Some(command.as_command()));
        as_clickable_control(&button).unwrap().raise_click();

        let selection = editor.selection().primary();
        assert_eq!(selection.anchor, 1);
        assert!(selection.caret > selection.anchor);
        assert_eq!(editor.document().get_line_by_offset(selection.caret).unwrap().line_number(), 1);
    }

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
        let mut options = editor.options();
        options.cut_copy_whole_line = false;
        editor.set_options(options);
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
    fn delete_selection_button_matches_the_application_command_at_the_caret_and_in_protected_text() {
        use ferroui_controls::{Button, as_clickable_control};
        use ferroui_edit_core::editing::{Selection, TextSegmentReadOnlySectionProvider};

        let _scope = crate::tests::test_scope();
        let text = "abXXcd\r\nefYYgh\r\nijk";
        let editor = crate::tests::laid_out_editor(text);
        let document = editor.document();
        let selection_delete = editor.command(EditorCommandKind::DeleteSelection);
        let button = Button::new();
        button.set_command(Some(selection_delete.as_command()));
        let click = || as_clickable_control(&button).unwrap().raise_click();
        let caret = Selection::empty(1);
        let reversed = Selection::simple(6, 1);
        let protected_selection = Selection::simple(4, 2);

        for (selection, read_only, protected, expected, after, after_redo) in [
            (caret.clone(), false, false, text, caret.clone(), caret.clone()),
            (reversed.clone(), false, false, "a\r\nefYYgh\r\nijk", caret.clone(), caret.clone()),
            (reversed.clone(), false, true, "aXX\r\nefYYgh\r\nijk", caret, Selection::simple(3, 1)),
            (protected_selection.clone(), false, true, text, protected_selection.clone(), protected_selection),
            (reversed.clone(), true, false, text, reversed.clone(), reversed.clone()),
        ] {
            editor.set_is_read_only(read_only);
            let policy = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
            if protected {
                policy.add(2, 2).unwrap();
            }
            editor.session().set_read_only_section_provider(policy);
            editor.set_selection(selection.clone()).unwrap();
            document.clear_undo_history().unwrap();
            assert!(selection_delete.can_execute(None));
            click();
            assert_eq!(editor.text(), expected);
            assert_eq!(editor.selection(), after);
            assert_eq!(document.undo_count(), usize::from(expected != text));
            if expected != text {
                assert!(editor.undo().unwrap());
                assert_eq!(editor.text(), text);
                assert_eq!(editor.selection(), selection);
                assert!(editor.redo().unwrap());
                assert_eq!(editor.text(), expected);
                assert_eq!(editor.selection(), after_redo);
                assert!(editor.undo().unwrap());
            }
        }

        editor.set_is_read_only(false);
        document.set_is_read_only(true);
        assert!(selection_delete.can_execute(None));
        click();
        assert_eq!(editor.text(), text);
        assert_eq!(editor.selection(), reversed);
        assert!(!document.can_undo());
        document.set_is_read_only(false);

        editor.set_caret_offset(1).unwrap();
        editor_key(&editor, ferroui_base::input::Key::Delete, ferroui_base::input::KeyModifiers::NONE);
        assert_eq!(editor.text(), "aXXcd\r\nefYYgh\r\nijk");
        drop(editor);
        assert!(!selection_delete.can_execute(None));
    }

    #[test]
    fn delete_selection_preserves_column_mode_and_handles_virtual_cells() {
        use ferroui_edit_core::editing::{Selection, SelectionSegment, TextSegmentReadOnlySectionProvider};

        let _scope = crate::tests::test_scope();
        let text = "abXXcd\r\nefYYgh\r\nijk";
        let editor = crate::tests::laid_out_editor(text);
        let document = editor.document();
        let mut options = editor.options();
        options.enable_virtual_space = true;
        editor.set_options(options);
        let policy = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
        policy.add(2, 2).unwrap();
        policy.add(10, 2).unwrap();
        editor.session().set_read_only_section_provider(policy);
        let command = editor.command(EditorCommandKind::DeleteSelection);
        let rectangle = |first, last| Selection::from_rectangle_segments([first, last], 1);
        let empty_box = rectangle(SelectionSegment::new(1, 1), SelectionSegment::new(9, 9));
        let protected_box = rectangle(SelectionSegment::new(1, 5), SelectionSegment::new(9, 13));
        let virtual_span = |start, end| {
            Selection::from_segments([SelectionSegment::with_virtual_space(6, start, 6, end)], 0)
        };
        for (selection, after) in [
            (empty_box.clone(), empty_box),
            (virtual_span(3, 3), Selection::empty(6)),
            (virtual_span(1, 3), virtual_span(1, 1)),
        ] {
            document.clear_undo_history().unwrap();
            editor.set_selection(selection).unwrap();
            command.execute(None);
            assert_eq!(editor.text(), text);
            assert_eq!(editor.selection(), after);
            assert!(!document.can_undo());
        }

        editor.set_selection(protected_box.clone()).unwrap();
        command.execute(None);
        assert_eq!(editor.text(), "aXXd\r\neYYh\r\nijk");
        assert_eq!(editor.selection(), rectangle(SelectionSegment::new(1, 1), SelectionSegment::new(7, 7)));
        assert_eq!(document.undo_count(), 1);
        assert!(editor.undo().unwrap());
        assert_eq!(editor.text(), text);
        assert_eq!(editor.selection(), protected_box);
        assert!(editor.redo().unwrap());
        assert_eq!(editor.text(), "aXXd\r\neYYh\r\nijk");
        assert_eq!(editor.selection(), rectangle(SelectionSegment::new(1, 3), SelectionSegment::new(7, 9)));
    }

    #[test]
    fn delete_selection_aborts_if_an_update_callback_rebinds_the_editor() {
        let _scope = crate::tests::test_scope();
        let editor = crate::tests::laid_out_editor("selected text");
        let document = editor.document();
        editor.select(0, 8).unwrap();
        document.clear_undo_history().unwrap();
        let weak = editor.downgrade();
        let _subscription = document.update_started(move || {
            weak.upgrade().unwrap().set_document(TextDocument::from_text("replacement"));
        });

        let command = editor.command(EditorCommandKind::DeleteSelection);
        command.execute(None);
        assert_eq!(editor.text(), "replacement");
        assert_eq!(document.text(), "selected text");
        assert!(!document.can_undo());
    }

    #[test]
    fn line_break_command_and_keys_share_the_reference_input_lifecycle() {
        use crate::TextArea;
        use ferroui_base::input::{Key, KeyModifiers};
        use ferroui_base::layout::ILayoutManager;
        use ferroui_controls::testing::TestServices;
        use ferroui_controls::{Button, Control, StackPanel, Window};
        use ferroui_edit_core::editing::{Selection, TextSegmentReadOnlySectionProvider};

        let _scope = crate::tests::test_scope_with_services(TestServices::focusable_window());
        let text = "    leftXXright\r\nlast";
        let editor = crate::tests::laid_out_editor(text);
        editor.set_height(140.0);
        let neighbor = Button::new();
        let panel = StackPanel::new();
        panel.children().add(neighbor.clone());
        panel.children().add(editor.clone());
        let root = Window::new();
        root.set_width(400.0);
        root.set_height(185.0);
        root.set_content(Some(Control::boxed(panel)));
        root.show();
        root.layout_manager().execute_layout_pass();

        let area = editor.text_area();
        let command = editor.command(EditorCommandKind::InsertLineBreak);
        let selected = Selection::simple(12, 5);
        let ordinary_result = "    l\r\n    ght\r\nlast";
        for (gesture, read_only, protected, cancel, expected, caret, stages) in [
            (None, false, None, false, ordinary_result, 11, 2),
            (Some(KeyModifiers::NONE), false, None, false, ordinary_result, 11, 2),
            (Some(KeyModifiers::SHIFT), false, None, false, ordinary_result, 11, 2),
            (None, true, None, false, text, 5, 2),
            (Some(KeyModifiers::SHIFT), true, None, false, text, 5, 2),
            (None, false, Some((8, 2)), false, "    lXX\r\n    ght\r\nlast", 13, 2),
            (None, false, Some((0, 18)), false, text, 5, 2),
            (None, false, None, true, text, 5, 1),
        ] {
            let document = editor.document();
            let policy = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
            if let Some((start, length)) = protected {
                policy.add(start, length).unwrap();
            }
            editor.session().set_read_only_section_provider(policy);
            editor.set_is_read_only(read_only);
            editor.set_selection(selected.clone()).unwrap();
            document.clear_undo_history().unwrap();

            let observed = Rc::new(RefCell::new(Vec::new()));
            let recorded = observed.clone();
            let weak = editor.downgrade();
            let _entering = area.add_disposable_handler(TextArea::text_entering_event(), move |_, e| {
                let editor = weak.upgrade().unwrap();
                assert_eq!(e.text.as_deref(), Some("\n"));
                recorded.borrow_mut().push(("before", editor.text(), editor.caret_offset()));
                e.set_handled(cancel);
            });
            let recorded = observed.clone();
            let weak = editor.downgrade();
            let _entered = area.add_disposable_handler(TextArea::text_entered_event(), move |_, e| {
                let editor = weak.upgrade().unwrap();
                assert_eq!(e.text.as_deref(), Some("\n"));
                recorded.borrow_mut().push(("after", editor.text(), editor.caret_offset()));
            });

            assert!(neighbor.focus());
            assert!(command.can_execute(None));
            command.execute(None);
            assert!(observed.borrow().is_empty());
            assert_eq!(editor.text(), text);
            assert_eq!(editor.selection(), selected);
            assert!(!document.can_undo());
            assert!(area.focus());
            if let Some(modifiers) = gesture {
                editor_key(&editor, Key::Return, modifiers);
            } else {
                command.execute(None);
            }

            assert_eq!(editor.text(), expected);
            assert_eq!(editor.caret_offset(), caret);
            assert_eq!(editor.last_error(), None);
            let observed = observed.borrow();
            assert_eq!(observed.len(), stages);
            assert_eq!(observed[0], ("before", text.to_string(), 5));
            if stages == 2 {
                assert_eq!(observed[1], ("after", expected.to_string(), caret));
            }
            assert_eq!(document.undo_count(), usize::from(expected != text));
            if expected != text {
                assert!(editor.undo().unwrap());
                assert_eq!(editor.text(), text);
            } else {
                assert_eq!(editor.selection(), selected);
            }
        }
        root.close();
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

    #[test]
    fn native_transform_button_edits_writable_fragments_and_checks_the_editor_binding() {
        use ferroui_controls::{Button, as_clickable_control};
        use ferroui_edit_core::editing::{Selection, TextSegmentReadOnlySectionProvider};

        let _scope = crate::tests::test_scope();
        let editor = crate::tests::laid_out_editor("abXycd\r\n ef");
        let document = editor.document();
        document.clear_undo_history().unwrap();
        let provider = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
        provider.add(2, 2).unwrap();
        editor.session().set_read_only_section_provider(provider);
        let selection = Selection::simple(10, 0);
        editor.set_selection(selection.clone()).unwrap();
        let button = Button::new();
        let command = editor.command(EditorCommandKind::Transform(TextTransformation::Uppercase));
        button.set_command(Some(command.as_command()));
        as_clickable_control(&button).unwrap().raise_click();
        assert_eq!(editor.text(), "ABXyCD\r\n Ef");
        assert_eq!(editor.selection(), selection);
        assert_eq!(document.undo_count(), 1);
        assert!(editor.undo().unwrap());
        assert_eq!(editor.text(), "abXycd\r\n ef");
        assert_eq!(editor.selection(), selection);

        let weak = editor.downgrade();
        let _subscription = document.update_started(move || {
            weak.upgrade().unwrap().set_document(TextDocument::from_text("replacement"));
        });
        assert!(matches!(
            editor.transform_text(TextTransformation::Uppercase),
            Err(EditError::StaleEditPlan)
        ));
        assert_eq!(document.text(), "abXycd\r\n ef");
        assert_eq!(editor.text(), "replacement");
        assert!(!document.can_undo());
    }

    #[test]
    fn native_casing_uses_current_text_locale_and_redoes_the_original_edit() {
        use ferroui_base::utilities::{
            CultureInfo, Locale, current_ui_locale, locale, set_current_locale, set_current_ui_locale,
        };
        use ferroui_edit_core::editing::{Selection, TextSegmentReadOnlySectionProvider};

        struct RestoreLocales(CultureInfo, Locale);
        impl Drop for RestoreLocales {
            fn drop(&mut self) {
                CultureInfo::set_current_formatting(self.0.clone());
                set_current_ui_locale(self.1.clone());
            }
        }
        let _restore = RestoreLocales(CultureInfo::current_formatting(), current_ui_locale());
        set_current_locale(locale!("en-US"));
        set_current_ui_locale(locale!("tr-TR"));
        let _scope = crate::tests::test_scope();
        let original = "i i ı ß ﬃ 𐐨";
        let editor = crate::tests::laid_out_editor(original);
        let document = editor.document();
        document.clear_undo_history().unwrap();
        let policy = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
        policy.add(2, 1).unwrap();
        editor.session().set_read_only_section_provider(policy);
        let selection = Selection::simple(document.text_length(), 0);
        editor.set_selection(selection.clone()).unwrap();
        let command = editor.command(EditorCommandKind::Transform(TextTransformation::Uppercase));

        command.execute(None);
        assert_eq!(editor.text(), "I i I ß ﬃ 𐐀");
        assert_eq!(editor.selection(), selection);
        assert!(editor.undo().unwrap());
        set_current_locale(locale!("az-Latn-AZ"));
        set_current_ui_locale(locale!("en-US"));
        command.execute(None);
        assert_eq!(editor.text(), "İ i I ß ﬃ 𐐀");
        assert_eq!(editor.selection(), selection);
        assert!(editor.undo().unwrap());
        assert_eq!(editor.text(), original);
        set_current_locale(locale!("en-US"));
        assert!(editor.redo().unwrap());
        assert_eq!(editor.text(), "İ i I ß ﬃ 𐐀");
        assert_eq!(editor.selection(), selection);
    }
}
