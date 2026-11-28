use super::*;
use crate::TextDocument;
use ferroui_edit_core::document::{DocumentSubscription, IUndoableOperation};

#[cfg(test)]
#[path = "text_area_history_tests.rs"]
mod tests;

#[derive(Clone, Copy)]
struct ViewCaret {
    primary: SelectionSegment,
    rectangular: bool,
    at_end_of_row: bool,
    box_caret: Option<BoxCaret>,
    context_caret: Option<ContextCaret>,
}

#[derive(Clone, Copy)]
pub(super) struct PendingUndoCaret {
    caret: ViewCaret,
    revision: u64,
}

struct RestoreViewCaret {
    area: WeakRef<TextArea>,
    document_epoch: u64,
    caret: ViewCaret,
}

impl IUndoableOperation for RestoreViewCaret {
    fn undo(&self) {
        let Some(area) = self.area.upgrade() else {
            return;
        };
        let Some(editor) = area
            .editor()
            .filter(|editor| editor.document_epoch() == self.document_epoch)
        else {
            return;
        };
        // The session's selection operation may still run. Apply view state
        // after the document update or the custom-only playback finishes.
        area.pending_undo_caret.set(Some(PendingUndoCaret {
            caret: self.caret,
            revision: editor.document().version().revision(),
        }));
    }

    fn redo(&self) {
        // Forward document changes translate the restored input selection
        // and reset its affinity, as they did on the original edit. A group
        // containing only application state has no forward text change.
        self.undo();
    }
}

impl TextArea {
    pub(crate) fn track_caret_undo(
        &self,
        document: &TextDocument,
        document_epoch: u64,
    ) -> DocumentSubscription {
        self.pending_undo_caret.set(None);
        let weak = self.to_ref().downgrade();
        document.undo_group_started(move || {
            let Some(area) = weak.upgrade() else {
                return;
            };
            let Some(editor) = area
                .editor()
                .filter(|editor| editor.document_epoch() == document_epoch)
            else {
                return;
            };
            let selection = editor.selection();
            let caret = ViewCaret {
                primary: selection.primary(),
                rectangular: selection.is_rectangular(),
                at_end_of_row: area.caret_at_end_of_line.get(),
                box_caret: area.active_box_caret(&selection),
                context_caret: area.active_context_caret(&selection),
            };
            // Even a default caret is recorded. In a continued undo group the
            // oldest snapshot wins, including when only later edits had affinity.
            let _ = editor
                .document()
                .undo_stack()
                .push_optional(Rc::new(RestoreViewCaret {
                    area: weak.clone(),
                    document_epoch,
                    caret,
                }));
        })
    }

    pub(crate) fn restore_undo_caret(&self, editor: &TextEditor) -> bool {
        let Some(pending) = self.pending_undo_caret.take() else {
            return false;
        };
        let selection = editor.selection();
        let caret = pending.caret;
        if pending.revision != editor.document().version().revision()
            || caret.primary != selection.primary()
            || caret.rectangular != selection.is_rectangular()
        {
            return false;
        }
        let previous = self.navigation_caret(&selection);
        self.box_caret.set(caret.box_caret);
        self.context_caret.set(caret.context_caret);
        self.apply_caret_affinity(caret.at_end_of_row);
        editor.notify_selection_changed(previous != self.navigation_caret(&editor.selection()));
        true
    }
}
