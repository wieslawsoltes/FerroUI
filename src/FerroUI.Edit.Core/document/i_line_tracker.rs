use super::{DocumentChangeEventArgs, LiveDocumentLine};

/// Low-level line index notifications. The live line index is being updated
/// during the first three callbacks; complete document text is already visible.
/// Callbacks run without internal borrows, but must not modify the document or
/// publish UI events that assume the line index is complete. `change_complete`
/// runs after lines and anchors are consistent, before the document changed event.
pub trait ILineTracker {
    fn before_remove_line(&self, line: &LiveDocumentLine);
    fn set_line_length(&self, line: &LiveDocumentLine, new_total_length: usize);
    fn line_inserted(&self, insertion_position: &LiveDocumentLine, new_line: &LiveDocumentLine);
    fn rebuild_document(&self);
    fn change_complete(&self, change: &DocumentChangeEventArgs);
}
