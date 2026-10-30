//! Rich state follows the TextBox's existing text undo snapshots. This module
//! adds no second history and deliberately keeps text-only snapshot equality.

use super::TextBox;
use crate::presenters::TextStyleOverrides;
use ferroui_base::media::text_formatting::TextParagraphStyles;
use std::any::Any;
use std::fmt;
use std::rc::Rc;

/// A native text undo checkpoint. Caret, styles and the application's immutable
/// payload travel with the text; they do not create separate undo steps.
#[derive(Clone)]
pub(crate) struct UndoRedoState {
    pub(super) text: Option<String>,
    caret_position: i32,
    styles: Option<TextStyleOverrides>,
    paragraph_styles: Option<TextParagraphStyles>,
    application_state: Option<Rc<dyn Any>>,
}

impl UndoRedoState {
    pub(super) fn new(text: Option<String>, caret_position: i32) -> Self {
        Self {
            text,
            caret_position,
            styles: None,
            paragraph_styles: None,
            application_state: None,
        }
    }
}

impl fmt::Debug for UndoRedoState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UndoRedoState")
            .field("text", &self.text)
            .field("caret_position", &self.caret_position)
            .field(
                "style_count",
                &self.styles.as_ref().map_or(0, |styles| styles.len()),
            )
            .field(
                "paragraph_count",
                &self
                    .paragraph_styles
                    .as_ref()
                    .map_or(0, |styles| styles.len()),
            )
            .field("has_application_state", &self.application_state.is_some())
            .finish()
    }
}

impl PartialEq for UndoRedoState {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl TextBox {
    /// The application's immutable rich-text snapshot for the current text.
    ///
    /// Native undo and redo restore this value, together with character and
    /// paragraph styles, before the synchronous `TextChanging` notification.
    /// An editor can downcast it and restore its document ranges exactly rather
    /// than guessing the deleted styling from the old and new strings. The
    /// application should verify that the snapshot describes the current Text.
    pub fn text_undo_state(&self) -> Option<Rc<dyn Any>> {
        self.text_undo_state.borrow().clone()
    }

    /// Associate an immutable application snapshot with the current text.
    ///
    /// Call this after updating rich styles in `TextChanging`, including after
    /// an external Text replacement. If the native undo helper has already
    /// captured this exact text, its current final snapshot is updated in place.
    /// This neither adds an undo step nor discards redo. Snapshots share the
    /// native undo limit and coalescing rules; callers must not mutate a payload
    /// after passing it here.
    pub fn set_text_undo_state(&self, value: Option<Rc<dyn Any>>) {
        // Do not hold a RefCell borrow while dropping an application object.
        let previous = self.text_undo_state.replace(value);
        drop(previous);
        if !self.is_undo_enabled() {
            return;
        }
        // update_last_state_with alone always writes the deque's last entry.
        // It must not overwrite a future redo checkpoint when the cursor is at
        // an earlier state with coincidentally equal text.
        if let Some(last) = self.undo_redo_helper.try_get_last_state() {
            let current = self.capture_text_history_state();
            if last.text == current.text {
                self.undo_redo_helper.update_last_state_with(current);
            }
        }
    }

    /// Whether native undo or redo is currently restoring a text snapshot.
    ///
    /// This is intended for synchronous `TextChanging` handlers. The later,
    /// asynchronous `TextChanged` notification runs after restoration has ended.
    pub fn is_restoring_text_history(&self) -> bool {
        self.is_undoing_redoing.get()
    }

    pub(super) fn capture_text_history_state(&self) -> UndoRedoState {
        UndoRedoState {
            text: self.text(),
            caret_position: self.caret_index(),
            styles: self.text_style_overrides(),
            paragraph_styles: self.text_paragraph_styles(),
            application_state: self.text_undo_state(),
        }
    }

    pub(super) fn restore_text_history_state(&self, state: UndoRedoState) {
        let generation = self.text_change_generation.get();
        let _change = self.im_client.begin_change();

        // Publish the payload directly, not through the public setter: Text
        // still has its outgoing value and must not update that checkpoint.
        let previous = self.text_undo_state.replace(state.application_state);
        drop(previous);
        if self.text_change_generation.get() != generation {
            return;
        }
        self.set_current_value(Self::text_style_overrides_property(), state.styles);
        if self.text_change_generation.get() != generation {
            return;
        }
        self.set_current_value(
            Self::text_paragraph_styles_property(),
            state.paragraph_styles,
        );
        if self.text_change_generation.get() != generation {
            return;
        }

        // Style-property callbacks can replace Text before this assignment;
        // TextChanging and subclass coercion can replace it during assignment.
        // Never apply a stale caret/selection over a newer handler-owned edit,
        // including a change away from, and then back to, the requested text.
        self.set_current_value(Self::text_property(), state.text.clone());
        if self.text_change_generation.get().wrapping_sub(generation) > 1
            || self.text() != state.text
        {
            return;
        }
        let generation = self.text_change_generation.get();
        self.set_current_value(Self::caret_index_property(), state.caret_position);
        if self.text_change_generation.get() != generation {
            return;
        }
        let caret = self.caret_index();
        self.set_current_value(Self::selection_start_property(), caret);
        if self.text_change_generation.get() != generation {
            return;
        }
        self.set_current_value(Self::selection_end_property(), caret);
    }
}

#[cfg(test)]
#[path = "text_box_history_tests.rs"]
mod tests;
