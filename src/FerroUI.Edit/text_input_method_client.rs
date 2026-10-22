use crate::TextArea;
use ferroui_base::input::text_input::{
    ContextMenuAction, TextInputMethodClient, TextInputMethodClientEvents, TextSelection,
};
use ferroui_base::{Rect, Ref, Visual, WeakRef};
use ferroui_edit_core::editing::Selection;
use std::rc::Rc;

/// The platform IME sees UTF-16 surrounding text and selection through the same
/// framework contract as other text controls. The client never owns the view.
pub(crate) struct EditorTextInputMethodClient {
    area: WeakRef<TextArea>,
    events: TextInputMethodClientEvents,
}

impl EditorTextInputMethodClient {
    pub fn new(area: WeakRef<TextArea>) -> Rc<Self> {
        Rc::new(Self {
            area,
            events: TextInputMethodClientEvents::new(),
        })
    }
}

impl TextInputMethodClient for EditorTextInputMethodClient {
    fn events(&self) -> &TextInputMethodClientEvents {
        &self.events
    }
    fn text_view_visual(&self) -> Option<Ref<Visual>> {
        self.area.upgrade().map(Ref::upcast)
    }
    fn supports_preedit(&self) -> bool {
        true
    }
    fn supports_surrounding_text(&self) -> bool {
        true
    }
    fn surrounding_text(&self) -> String {
        self.area
            .upgrade()
            .and_then(|area| area.editor())
            .map_or_else(String::new, |editor| editor.text())
    }
    fn cursor_rectangle(&self) -> Rect {
        self.area
            .upgrade()
            .map_or_else(Rect::default, |area| area.cursor_rectangle())
    }
    fn selection(&self) -> TextSelection {
        let Some(editor) = self.area.upgrade().and_then(|area| area.editor()) else {
            return TextSelection::default();
        };
        let selection = editor.selection().primary();
        TextSelection::new(
            selection.anchor.min(i32::MAX as usize) as i32,
            selection.caret.min(i32::MAX as usize) as i32,
        )
    }
    fn set_selection(&self, value: TextSelection) {
        if let Some(editor) = self.area.upgrade().and_then(|area| area.editor()) {
            let length = editor.document().text_length();
            let _ = editor.set_selection(Selection::simple(
                (value.start.max(0) as usize).min(length),
                (value.end.max(0) as usize).min(length),
            ));
        }
    }
    fn set_preedit_text(&self, text: Option<&str>) {
        self.set_preedit_text_with_cursor(text, None);
    }
    fn set_preedit_text_with_cursor(&self, text: Option<&str>, cursor: Option<i32>) {
        if let Some(area) = self.area.upgrade() {
            area.set_preedit(text, cursor);
        }
    }
    fn execute_context_menu_action(&self, action: ContextMenuAction) {
        if let Some(editor) = self.area.upgrade().and_then(|area| area.editor()) {
            match action {
                ContextMenuAction::Copy => editor.copy(),
                ContextMenuAction::Cut => editor.cut(),
                ContextMenuAction::Paste => editor.paste(),
                ContextMenuAction::SelectAll => editor.select_all(),
            }
        }
    }
}
