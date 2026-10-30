use crate::rich_text::{EditorSelection, RichTextEditor};
use crate::RichEditBox;
use ferroui_base::input::text_input::{
    ContextMenuAction, TextInputMethodClient, TextInputMethodClientEvents, TextInputMethodEdit,
    TextInputMethodEditing, TextInputMethodError, TextInputMethodRangeGeometry,
    TextInputMethodState, TextSelection,
};
use ferroui_base::input::InputElement;
use ferroui_base::{Rect, Ref, Visual, WeakRef};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct RichInputMethodState {
    editor: Weak<RichTextEditor>,
    mutation: u64,
    selection: EditorSelection,
    composing: bool,
}

impl PartialEq for RichInputMethodState {
    fn eq(&self, other: &Self) -> bool {
        self.editor.ptr_eq(&other.editor)
            && self.mutation == other.mutation
            && self.selection == other.selection
            && self.composing == other.composing
    }
}

#[derive(Default)]
struct InputMethodRevision {
    value: u64,
    observed: Option<RichInputMethodState>,
}

/// Platform input-method projection of the live rich document and layout geometry.
/// Its owner is weak, so retaining a platform client cannot retain a removed control.
pub struct RichEditBoxTextInputMethodClient {
    events: TextInputMethodClientEvents,
    owner: RefCell<Option<WeakRef<RichEditBox>>>,
    revision: RefCell<InputMethodRevision>,
}

impl RichEditBoxTextInputMethodClient {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            events: TextInputMethodClientEvents::new(),
            owner: RefCell::new(None),
            revision: RefCell::new(InputMethodRevision::default()),
        })
    }
    pub(crate) fn attach(&self, owner: Option<&Ref<RichEditBox>>) {
        *self.owner.borrow_mut() = owner.map(Ref::downgrade);
        self.raise_text_view_visual_changed();
        self.raise_cursor_rectangle_changed();
    }
    fn owner(&self) -> Option<Ref<RichEditBox>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }
}

impl TextInputMethodClient for RichEditBoxTextInputMethodClient {
    fn events(&self) -> &TextInputMethodClientEvents {
        &self.events
    }
    fn input_method_owner(&self) -> Option<Ref<InputElement>> {
        self.owner().map(Ref::upcast)
    }
    fn retains_input_method_focus(&self, focused: &InputElement) -> bool {
        self.owner().is_some_and(|owner| {
            !owner.is_read_only() && owner.context_flyout_contains_focus(Some(focused))
        })
    }
    fn input_method_editing(&self) -> Option<&dyn TextInputMethodEditing> {
        Some(self)
    }
    fn input_method_state_revision(&self) -> Option<u64> {
        let observed = self.owner().map(|owner| {
            let editor = owner.editor();
            RichInputMethodState {
                editor: Rc::downgrade(&editor),
                mutation: editor.rich_document().mutation_version(),
                selection: editor.selection(),
                composing: editor.is_composing(),
            }
        });
        let mut revision = self.revision.borrow_mut();
        if revision.observed != observed {
            revision.value = revision.value.wrapping_add(1);
            revision.observed = observed;
        }
        // Document mutation identity advances before an outer DocumentEditBatch
        // publishes its event. The weak editor identity also distinguishes an
        // equal-looking replacement document without retaining its control.
        Some(revision.value)
    }
    fn text_view_visual(&self) -> Option<Ref<Visual>> {
        self.owner().map(|owner| owner.text_presenter().upcast())
    }
    fn supports_preedit(&self) -> bool {
        true
    }
    fn supports_surrounding_text(&self) -> bool {
        true
    }
    fn surrounding_text(&self) -> String {
        self.owner()
            .map_or_else(String::new, |owner| owner.editor().surrounding_text())
    }
    fn cursor_rectangle(&self) -> Rect {
        self.owner()
            .map_or_else(Rect::default, |owner| owner.cursor_rectangle())
    }
    fn text_range_geometry(
        &self,
        range: TextSelection,
    ) -> Result<TextInputMethodRangeGeometry, TextInputMethodError> {
        self.query_text_range_geometry(range)
    }
    fn selection(&self) -> TextSelection {
        self.owner().map_or_else(TextSelection::default, |owner| {
            let selection = if owner.editor().is_composing() {
                owner.editor().input_method_selection()
            } else {
                crate::rich_text::EditorSelection::new(
                    owner.selection_start(),
                    owner.selection_end(),
                )
            };
            TextSelection::new(
                selection.anchor.min(i32::MAX as usize) as i32,
                selection.active.min(i32::MAX as usize) as i32,
            )
        })
    }
    fn set_selection(&self, selection: TextSelection) {
        if let Some(owner) = self.owner() {
            // Native offsets refer to the committed surrounding text, so remove the
            // visible preview before applying the platform's new selection.
            if owner.editor().is_composing() {
                owner.handle_edit(owner.set_preedit_text(None, None));
            }
            owner.select(
                selection.start.max(0) as usize,
                selection.end.max(0) as usize,
            );
        }
    }
    fn set_preedit_text(&self, text: Option<&str>) {
        self.set_preedit_text_with_cursor(text, None);
    }
    fn set_preedit_text_with_cursor(&self, text: Option<&str>, cursor: Option<i32>) {
        if let Some(owner) = self.owner() {
            owner.handle_edit(
                owner.set_preedit_text(text, cursor.map(|value| value.max(0) as usize)),
            );
        }
    }
    fn execute_context_menu_action(&self, action: ContextMenuAction) {
        if let Some(owner) = self.owner() {
            match action {
                ContextMenuAction::Copy => owner.copy(),
                ContextMenuAction::Cut => owner.cut(),
                ContextMenuAction::Paste => owner.paste(),
                ContextMenuAction::SelectAll => owner.select_all(),
            }
        }
    }
}

pub(crate) fn input_method_error(error: &crate::rich_text::EditError) -> TextInputMethodError {
    match error {
        crate::rich_text::EditError::ReadOnly => TextInputMethodError::ReadOnly,
        crate::rich_text::EditError::ReentrantEdit => TextInputMethodError::ReentrantEdit,
        crate::rich_text::EditError::Document(_) => TextInputMethodError::Rejected,
    }
}

impl TextInputMethodEditing for RichEditBoxTextInputMethodClient {
    fn supports_exact_edits(&self) -> bool {
        true
    }

    fn edit_state(&self) -> Result<TextInputMethodState, TextInputMethodError> {
        let owner = self.owner().ok_or(TextInputMethodError::Unavailable)?;
        let editor = owner.editor();
        let selection = editor.selection();
        Ok(TextInputMethodState {
            text: editor.rich_document().text(),
            selection: TextSelection::new(
                i32::try_from(selection.anchor).map_err(|_| TextInputMethodError::InvalidRange)?,
                i32::try_from(selection.active).map_err(|_| TextInputMethodError::InvalidRange)?,
            ),
        })
    }

    fn apply_edit(
        &self,
        edit: &TextInputMethodEdit,
        composing: bool,
    ) -> Result<bool, TextInputMethodError> {
        let owner = self.owner().ok_or(TextInputMethodError::Unavailable)?;
        edit.apply_to(&owner.rich_document().text())?;
        let result = owner.apply_input_method_edit(edit, composing);
        let accepted = result.as_ref().copied().map_err(input_method_error);
        owner.handle_edit(result);
        accepted
    }

    fn complete_composition(&self) -> Result<bool, TextInputMethodError> {
        let owner = self.owner().ok_or(TextInputMethodError::Unavailable)?;
        let result = owner.complete_input_method_composition();
        let accepted = result.as_ref().copied().map_err(input_method_error);
        owner.handle_edit(result);
        accepted
    }

    fn select_exact(&self, selection: TextSelection) -> Result<(), TextInputMethodError> {
        let owner = self.owner().ok_or(TextInputMethodError::Unavailable)?;
        let editor = owner.editor();
        TextInputMethodEdit::new(TextSelection::new(0, 0), "", selection)
            .apply_to(&editor.rich_document().text())?;
        let result = editor.select_exact_input(selection);
        let accepted = result.as_ref().map(|_| ()).map_err(input_method_error);
        owner.handle_edit(result);
        accepted
    }

    fn begin_composition(&self) -> Result<(), TextInputMethodError> {
        let owner = self.owner().ok_or(TextInputMethodError::Unavailable)?;
        let result = owner.begin_input_method_composition();
        let accepted = result.as_ref().map(|_| ()).map_err(input_method_error);
        owner.handle_edit(result);
        accepted
    }

    fn update_composition(
        &self,
        text: &str,
        cursor: Option<i32>,
    ) -> Result<(), TextInputMethodError> {
        let owner = self.owner().ok_or(TextInputMethodError::Unavailable)?;
        let result =
            owner.set_preedit_text(Some(text), cursor.map(|cursor| cursor.max(0) as usize));
        let accepted = result.as_ref().map(|_| ()).map_err(input_method_error);
        owner.handle_edit(result);
        accepted
    }

    fn cancel_composition(&self) -> Result<bool, TextInputMethodError> {
        let owner = self.owner().ok_or(TextInputMethodError::Unavailable)?;
        let result = owner.cancel_composition();
        let accepted = result.as_ref().copied().map_err(input_method_error);
        owner.handle_edit(result);
        accepted
    }
}

#[cfg(test)]
#[path = "rich_edit_input_method_tests.rs"]
mod tests;

#[path = "rich_edit_input_range_geometry.rs"]
mod range_geometry;
