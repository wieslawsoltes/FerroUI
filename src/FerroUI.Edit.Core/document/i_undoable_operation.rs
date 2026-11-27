use super::UndoStack;

/// An already-performed action that can restore its preceding and following
/// states. Recording an action does not execute it. Callbacks run outside all
/// document/history borrows. Document writes during playback remain forbidden;
/// document edits use the built-in change operations instead.
pub trait IUndoableOperation {
    fn undo(&self);
    fn redo(&self);
}

/// Context-aware actions can use the current stack without capturing a strong
/// reference to the document that owns them. This prevents ownership cycles for
/// caret/selection and host-service undo actions.
pub trait IUndoableOperationWithContext {
    fn undo(&self, stack: &UndoStack);
    fn redo(&self, stack: &UndoStack);
}
