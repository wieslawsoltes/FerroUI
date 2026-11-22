use crate::{TextArea, TextEditor};
use ferroui_base::input::InputElement;
use ferroui_base::input::text_input::{
    ContextMenuAction, TextInputMethodClient, TextInputMethodClientEvents, TextInputMethodEditing,
    TextInputMethodError, TextSelection,
};
use ferroui_base::{Rect, Ref, Visual, WeakRef};
use ferroui_edit_core::document::TextSnapshot;
use ferroui_edit_core::editing::{Selection, SelectionSegment};
use std::{cell::RefCell, ops::Range, rc::Rc};

pub(crate) struct SurroundingText {
    snapshot: TextSnapshot,
    pub(crate) range: Range<usize>,
    selection: SelectionSegment,
}

impl SurroundingText {
    pub(crate) fn for_editor(editor: &TextEditor) -> Option<Self> {
        let snapshot = editor.document().create_snapshot();
        let selection = editor.selection().primary();
        let start = snapshot.get_line_by_offset(selection.start_offset()).ok()?;
        let end = snapshot.get_line_by_offset(selection.end_offset()).ok()?;
        // Programmatic endpoints may be inside a CRLF pair.
        let range = start.offset()..end.end_offset().max(selection.end_offset());
        Some(Self {
            snapshot,
            range,
            selection,
        })
    }
}

/// The platform IME sees UTF-16 surrounding text and selection through the same
/// framework contract as other text controls. The client never owns the view.
pub(crate) struct EditorTextInputMethodClient {
    area: WeakRef<TextArea>,
    events: TextInputMethodClientEvents,
    revision: RefCell<InputRevision>,
    managed_preview: RefCell<Option<EditorInputState>>,
}

#[derive(PartialEq, Eq)]
struct EditorInputState {
    epoch: u64,
    revision: u64,
    selection: Selection,
}

impl EditorInputState {
    fn capture(editor: &TextEditor) -> Self {
        Self {
            epoch: editor.document_epoch(),
            revision: editor.document().version().revision(),
            selection: editor.selection(),
        }
    }
}

#[derive(Default)]
struct InputRevision {
    number: u64,
    state: Option<EditorInputState>,
}

impl EditorTextInputMethodClient {
    pub fn new(area: WeakRef<TextArea>) -> Rc<Self> {
        Rc::new(Self {
            area,
            events: TextInputMethodClientEvents::new(),
            revision: RefCell::default(),
            managed_preview: RefCell::new(None),
        })
    }

    fn editor(&self) -> Option<Ref<TextEditor>> {
        self.area.upgrade()?.editor()
    }

    pub(crate) fn preview_changed(&self) {
        let mut revision = self.revision.borrow_mut();
        revision.number = revision.number.wrapping_add(1);
    }

    pub(crate) fn discard_stale_preview(&self, area: &TextArea) {
        if self.managed_preview.borrow().is_none() {
            return;
        }
        let current = area.editor().map(|editor| EditorInputState::capture(&editor));
        if *self.managed_preview.borrow() != current {
            self.managed_preview.take();
            area.set_preedit(None, None);
        }
    }

    fn writable_editor(&self) -> Result<Ref<TextEditor>, TextInputMethodError> {
        let editor = self.editor().ok_or(TextInputMethodError::Unavailable)?;
        if editor.is_read_only() || editor.document().is_read_only() {
            return Err(TextInputMethodError::ReadOnly);
        }
        Ok(editor)
    }
}

impl TextInputMethodClient for EditorTextInputMethodClient {
    fn events(&self) -> &TextInputMethodClientEvents {
        &self.events
    }
    fn input_method_owner(&self) -> Option<Ref<InputElement>> {
        self.area.upgrade().map(Ref::upcast)
    }
    fn supports_surrounding_text_edits(&self) -> bool {
        true
    }
    fn input_method_editing(&self) -> Option<&dyn TextInputMethodEditing> {
        Some(self)
    }
    fn input_method_state_revision(&self) -> Option<u64> {
        let editor = self.editor()?;
        // Hosts can use the editing session directly, before the view has
        // announced the new document revision or selection to the input method.
        let state = EditorInputState::capture(&editor);
        let mut revision = self.revision.borrow_mut();
        if revision.state.as_ref() != Some(&state) {
            revision.number = revision.number.wrapping_add(1);
            revision.state = Some(state);
        }
        Some(revision.number)
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
        self.editor()
            .and_then(|editor| SurroundingText::for_editor(&editor))
            .and_then(|context| {
                context
                    .snapshot
                    .get_text(context.range.start, context.range.len())
                    .ok()
            })
            .unwrap_or_default()
    }
    fn cursor_rectangle(&self) -> Rect {
        self.area
            .upgrade()
            .map_or_else(Rect::default, |area| area.cursor_rectangle())
    }
    fn selection(&self) -> TextSelection {
        let Some(context) = self
            .editor()
            .and_then(|editor| SurroundingText::for_editor(&editor))
        else {
            return TextSelection::default();
        };
        let relative = |offset: usize| (offset - context.range.start).min(i32::MAX as usize) as i32;
        TextSelection::new(
            relative(context.selection.anchor),
            relative(context.selection.caret),
        )
    }
    fn set_selection(&self, value: TextSelection) {
        let Some(editor) = self.editor() else {
            return;
        };
        let Some(context) = SurroundingText::for_editor(&editor) else {
            return;
        };
        let absolute =
            |offset: i32| context.range.start + (offset.max(0) as usize).min(context.range.len());
        let anchor = absolute(value.start);
        let caret = absolute(value.end);
        // An echo of the mapped selection should preserve virtual cells and secondary carets.
        if (anchor, caret) != (context.selection.anchor, context.selection.caret) {
            let _ = editor.set_selection(Selection::simple(anchor, caret));
        }
    }
    fn set_preedit_text(&self, text: Option<&str>) {
        self.set_preedit_text_with_cursor(text, None);
    }
    fn set_preedit_text_with_cursor(&self, text: Option<&str>, cursor: Option<i32>) {
        self.managed_preview.take();
        if let Some(area) = self.area.upgrade() {
            area.set_preedit(text, cursor);
        }
    }
    fn execute_context_menu_action(&self, action: ContextMenuAction) {
        if let Some(editor) = self.editor() {
            match action {
                ContextMenuAction::Copy => editor.copy(),
                ContextMenuAction::Cut => editor.cut(),
                ContextMenuAction::Paste => editor.paste(),
                ContextMenuAction::SelectAll => editor.select_all(),
            }
        }
    }
}

impl TextInputMethodEditing for EditorTextInputMethodClient {
    fn begin_composition(&self) -> Result<(), TextInputMethodError> {
        self.writable_editor()?;
        self.set_preedit_text(None);
        Ok(())
    }

    fn update_composition(&self, text: &str, cursor: Option<i32>) -> Result<(), TextInputMethodError> {
        let editor = self.writable_editor()?;
        let area = self.area.upgrade().ok_or(TextInputMethodError::Unavailable)?;
        self.managed_preview.replace((!text.is_empty()).then(|| EditorInputState::capture(&editor)));
        area.set_preedit(Some(text), cursor);
        Ok(())
    }

    fn cancel_composition(&self) -> Result<bool, TextInputMethodError> {
        let area = self.area.upgrade().ok_or(TextInputMethodError::Unavailable)?;
        let had_preview = area.has_preedit();
        self.set_preedit_text(None);
        Ok(had_preview)
    }
}
