use super::{
    DocumentChangeEventArgs, DocumentError, IUndoableOperation, IUndoableOperationWithContext,
    TextDocument, UpdateGuard,
};
use std::rc::Rc;

#[derive(Clone)]
pub(crate) enum UndoOperation {
    Document(DocumentChangeEventArgs),
    Custom(Rc<dyn IUndoableOperation>),
    WithContext(Rc<dyn IUndoableOperationWithContext>),
}
impl std::fmt::Debug for UndoOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Document(change) => f.debug_tuple("Document").field(change).finish(),
            Self::Custom(_) => f.write_str("Custom"),
            Self::WithContext(_) => f.write_str("WithContext"),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct UndoGroup {
    pub changes: Vec<UndoOperation>,
    pub essential_count: usize,
    pub before: u64,
    pub after: u64,
}

pub(crate) struct UndoHistory {
    pub undo: Vec<UndoGroup>,
    pub redo: Vec<UndoGroup>,
    pub pending: Option<UndoGroup>,
    pub size_limit: usize,
    pub current_state: u64,
    pub original_state: Option<u64>,
    pub next_state: u64,
    pub replaying: bool,
    pub allow_continue: bool,
    pub continued: bool,
    pub retired: Vec<UndoOperation>,
}

impl Default for UndoHistory {
    fn default() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            pending: None,
            size_limit: usize::MAX,
            current_state: 0,
            original_state: Some(0),
            next_state: 1,
            replaying: false,
            allow_continue: false,
            continued: false,
            retired: Vec::new(),
        }
    }
}

impl UndoHistory {
    pub fn begin(&mut self, continued: bool) {
        if self.replaying {
            return;
        }
        self.continued =
            continued && self.allow_continue && self.redo.is_empty() && !self.undo.is_empty();
        self.pending = Some(UndoGroup {
            changes: Vec::new(),
            essential_count: 0,
            before: self.current_state,
            after: self.current_state,
        });
    }

    pub fn record(&mut self, change: DocumentChangeEventArgs) {
        self.record_operation(UndoOperation::Document(change), false);
    }

    pub fn record_operation(&mut self, operation: UndoOperation, optional: bool) {
        if self.replaying {
            return;
        }
        if let Some(group) = &mut self.pending {
            group.changes.push(operation);
            if !optional {
                group.essential_count += 1;
                group.after = self.next_state;
                self.current_state = self.next_state;
                self.next_state = self.next_state.wrapping_add(1);
            }
            self.clear_redo();
        }
    }

    pub fn finish(&mut self) {
        if self.replaying {
            return;
        }
        if let Some(mut pending) = self.pending.take() {
            if pending.changes.is_empty() {
                if !self.continued {
                    self.allow_continue = false;
                }
                return;
            }
            if pending.essential_count == 0 && !self.continued {
                self.retired.append(&mut pending.changes);
                self.allow_continue = false;
                return;
            }
            if self.continued {
                if let Some(previous) = self.undo.pop() {
                    let mut changes = previous.changes;
                    changes.append(&mut pending.changes);
                    pending.changes = changes;
                    pending.before = previous.before;
                    pending.essential_count += previous.essential_count;
                }
            }
            self.undo.push(pending);
            self.allow_continue = true;
            self.enforce_limit();
        }
    }

    pub fn enforce_limit(&mut self) {
        if self.undo.len() > self.size_limit {
            for mut group in self.undo.drain(..self.undo.len() - self.size_limit) {
                self.retired.append(&mut group.changes);
            }
        }
        if self.redo.len() > self.size_limit {
            for mut group in self.redo.drain(..self.redo.len() - self.size_limit) {
                self.retired.append(&mut group.changes);
            }
        }
    }
    pub fn clear_redo(&mut self) {
        for mut group in self.redo.drain(..) {
            self.retired.append(&mut group.changes);
        }
    }
    pub fn clear_all(&mut self) {
        for mut group in self.undo.drain(..) {
            self.retired.append(&mut group.changes);
        }
        self.clear_redo();
        self.allow_continue = false;
    }
}

#[cfg(test)]
mod tests;

/// Undo history attached to a document. Changes are retained as persistent rope
/// fragments; grouping does not duplicate complete document strings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UndoStack(pub(crate) TextDocument);

impl Default for UndoStack {
    fn default() -> Self {
        Self::new()
    }
}

impl UndoStack {
    /// A standalone history backed by an otherwise empty document.
    pub fn new() -> Self {
        TextDocument::new().undo_stack()
    }
    pub fn document(&self) -> TextDocument {
        self.0.clone()
    }
    pub fn push(&self, operation: Rc<dyn IUndoableOperation>) -> Result<bool, DocumentError> {
        self.0
            .push_undo_operation(UndoOperation::Custom(operation), false)
    }
    pub fn push_optional(
        &self,
        operation: Rc<dyn IUndoableOperation>,
    ) -> Result<bool, DocumentError> {
        self.0
            .push_undo_operation(UndoOperation::Custom(operation), true)
    }
    pub fn push_with_context(
        &self,
        operation: Rc<dyn IUndoableOperationWithContext>,
    ) -> Result<bool, DocumentError> {
        self.0
            .push_undo_operation(UndoOperation::WithContext(operation), false)
    }
    pub fn push_optional_with_context(
        &self,
        operation: Rc<dyn IUndoableOperationWithContext>,
    ) -> Result<bool, DocumentError> {
        self.0
            .push_undo_operation(UndoOperation::WithContext(operation), true)
    }
    pub fn clear_redo_stack(&self) -> Result<(), DocumentError> {
        self.0.clear_redo_history()
    }
    pub fn can_undo(&self) -> bool {
        self.0.can_undo()
    }
    pub fn can_redo(&self) -> bool {
        self.0.can_redo()
    }
    pub fn undo(&self) -> Result<bool, DocumentError> {
        self.0.undo()
    }
    pub fn redo(&self) -> Result<bool, DocumentError> {
        self.0.redo()
    }
    pub fn clear_all(&self) -> Result<(), DocumentError> {
        self.0.clear_undo_history()
    }
    pub fn size_limit(&self) -> usize {
        self.0.undo_size_limit()
    }
    pub fn set_size_limit(&self, value: usize) {
        self.0.set_undo_size_limit(value);
    }
    pub fn is_original_file(&self) -> bool {
        !self.0.is_modified()
    }
    pub fn mark_as_original_file(&self) {
        self.0.mark_as_original_file();
    }
    pub fn discard_original_file_marker(&self) {
        self.0.discard_original_file_marker();
    }
    pub fn accept_changes(&self) -> bool {
        self.0.accept_changes()
    }
    pub fn undo_count(&self) -> usize {
        self.0.undo_count()
    }
    pub fn redo_count(&self) -> usize {
        self.0.redo_count()
    }
    pub fn start_undo_group(&self) -> Result<UpdateGuard, DocumentError> {
        self.0.begin_update()
    }
    pub fn start_continued_undo_group(&self) -> Result<UpdateGuard, DocumentError> {
        self.0.begin_continued_update()
    }
}
