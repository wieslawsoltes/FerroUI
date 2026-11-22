use super::{
    DocumentChangeEventArgs, DocumentError, IUndoableOperation, IUndoableOperationWithContext,
    TextDocument, UpdateGuard, WeakTextDocument,
};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// Internal state committed together with a text edit, including on playback.
pub(crate) type DocumentEditEffect = Rc<dyn Fn(&TextDocument, bool)>;

#[derive(Clone)]
pub(crate) enum UndoOperation {
    Document(WeakTextDocument, DocumentChangeEventArgs, Option<DocumentEditEffect>),
    Custom(Rc<dyn IUndoableOperation>),
    WithContext(WeakTextDocument, Rc<dyn IUndoableOperationWithContext>),
}
impl std::fmt::Debug for UndoOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Document(_, change, _) => f.debug_tuple("Document").field(change).finish(),
            Self::Custom(_) => f.write_str("Custom"),
            Self::WithContext(..) => f.write_str("WithContext"),
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
    pub depth: usize,
    pub size_limit: usize,
    pub current_state: u64,
    pub original_state: Option<u64>,
    pub next_state: u64,
    pub replaying: bool,
    pub allow_continue: bool,
    pub continued: bool,
    pub last_group_descriptor: Option<Rc<dyn Any>>,
    pub retired: Vec<UndoOperation>,
    members: Vec<WeakTextDocument>,
    captured_documents: Vec<WeakTextDocument>,
}

pub(super) type SharedUndoHistory = Rc<RefCell<UndoHistory>>;

impl Default for UndoHistory {
    fn default() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            pending: None,
            depth: 0,
            size_limit: usize::MAX,
            current_state: 0,
            original_state: Some(0),
            next_state: 1,
            replaying: false,
            allow_continue: false,
            continued: false,
            last_group_descriptor: None,
            retired: Vec::new(),
            members: Vec::new(),
            captured_documents: Vec::new(),
        }
    }
}

impl UndoHistory {
    pub(super) fn attach(&mut self, document: &TextDocument) {
        self.members.retain(|member| member.is_alive());
        self.members.push(document.downgrade());
    }

    pub(super) fn detach(&mut self, document: &TextDocument) {
        self.members.retain(|member| member.is_alive() && !member.points_to(document));
    }

    pub(super) fn capture_document(&mut self, document: &TextDocument) -> bool {
        if self.replaying || self.captured_documents.iter().any(|entry| entry.points_to(document)) {
            return false;
        }
        self.captured_documents.push(document.downgrade());
        true
    }

    pub fn begin(
        &mut self,
        continued: bool,
        descriptor: Option<Rc<dyn Any>>,
    ) -> Option<Rc<dyn Any>> {
        if self.replaying {
            return descriptor;
        }
        self.depth += 1;
        if self.depth != 1 {
            return descriptor;
        }
        self.continued =
            continued && self.allow_continue && self.redo.is_empty() && !self.undo.is_empty();
        self.captured_documents.clear();
        self.pending = Some(UndoGroup {
            changes: Vec::new(),
            essential_count: 0,
            before: self.current_state,
            after: self.current_state,
        });
        std::mem::replace(&mut self.last_group_descriptor, descriptor)
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
        if self.replaying || self.depth == 0 {
            return;
        }
        self.depth -= 1;
        if self.depth != 0 {
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
    pub fn clear_all(&mut self) -> Option<Rc<dyn Any>> {
        let descriptor = if self.undo.is_empty() {
            None
        } else {
            self.last_group_descriptor.take()
        };
        for mut group in self.undo.drain(..) {
            self.retired.append(&mut group.changes);
        }
        self.clear_redo();
        self.allow_continue = false;
        descriptor
    }
}

#[cfg(test)]
mod tests;

/// Undo history that one or more documents can share. Changes retain persistent
/// rope fragments and weak references to their document.
#[derive(Clone)]
pub struct UndoStack {
    document: TextDocument,
    pub(super) history: SharedUndoHistory,
}

impl std::fmt::Debug for UndoStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UndoStack")
            .field("undo_count", &self.undo_count())
            .field("redo_count", &self.redo_count())
            .finish()
    }
}

impl PartialEq for UndoStack {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.history, &other.history)
    }
}
impl Eq for UndoStack {}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new()
    }
}

impl UndoStack {
    pub(super) fn attached(document: TextDocument, history: SharedUndoHistory) -> Self {
        Self { document, history }
    }

    /// A standalone history backed by an otherwise empty document.
    pub fn new() -> Self {
        TextDocument::new().undo_stack()
    }
    /// The document associated with this handle. During a context-aware
    /// operation, this is the document from which the operation was recorded.
    pub fn document(&self) -> TextDocument {
        self.document.clone()
    }
    pub fn push(&self, operation: Rc<dyn IUndoableOperation>) -> Result<bool, DocumentError> {
        self.push_operation(UndoOperation::Custom(operation), false)
    }
    pub fn push_optional(
        &self,
        operation: Rc<dyn IUndoableOperation>,
    ) -> Result<bool, DocumentError> {
        self.push_operation(UndoOperation::Custom(operation), true)
    }
    pub fn push_with_context(
        &self,
        operation: Rc<dyn IUndoableOperationWithContext>,
    ) -> Result<bool, DocumentError> {
        self.push_context_operation(operation, false)
    }
    pub fn push_optional_with_context(
        &self,
        operation: Rc<dyn IUndoableOperationWithContext>,
    ) -> Result<bool, DocumentError> {
        self.push_context_operation(operation, true)
    }
    fn push_context_operation(
        &self,
        operation: Rc<dyn IUndoableOperationWithContext>,
        optional: bool,
    ) -> Result<bool, DocumentError> {
        if !self.document.uses_history(&self.history) {
            return Err(DocumentError::DifferentDocument);
        }
        self.push_operation(UndoOperation::WithContext(self.document.downgrade(), operation), optional)
    }
    fn push_operation(&self, operation: UndoOperation, optional: bool) -> Result<bool, DocumentError> {
        let needs_group = {
            let history = self.history.borrow();
            if history.replaying {
                return Ok(false);
            }
            if optional && history.depth == 0 {
                return Err(DocumentError::NoUpdate);
            }
            if history.size_limit == 0 {
                return Ok(false);
            }
            history.depth == 0
        };
        let guard = needs_group.then(|| self.start_undo_group()).transpose()?;
        self.capture_context();
        self.history.borrow_mut().record_operation(operation, optional);
        Self::flush_retired(&self.history);
        if let Some(guard) = guard {
            guard.finish()?;
        }
        Ok(true)
    }
    pub fn clear_redo_stack(&self) -> Result<(), DocumentError> {
        {
            let mut history = self.history.borrow_mut();
            if history.replaying {
                return Err(DocumentError::UpdateInProgress);
            }
            history.clear_redo();
        }
        Self::flush_retired(&self.history);
        Self::notify_members(&self.history);
        Ok(())
    }
    pub fn can_undo(&self) -> bool {
        self.undo_count() != 0
    }
    pub fn can_redo(&self) -> bool {
        self.redo_count() != 0
    }
    pub fn undo(&self) -> Result<bool, DocumentError> {
        self.replay(false)
    }
    pub fn redo(&self) -> Result<bool, DocumentError> {
        self.replay(true)
    }
    pub fn clear_all(&self) -> Result<(), DocumentError> {
        let descriptor = {
            let mut history = self.history.borrow_mut();
            if history.depth != 0 || history.replaying {
                return Err(DocumentError::UpdateInProgress);
            }
            history.clear_all()
        };
        drop(descriptor);
        Self::flush_retired(&self.history);
        Self::notify_members(&self.history);
        Ok(())
    }
    pub fn size_limit(&self) -> usize {
        self.history.borrow().size_limit
    }
    pub fn set_size_limit(&self, value: usize) {
        {
            let mut history = self.history.borrow_mut();
            history.size_limit = value;
            if history.depth == 0 && !history.replaying {
                history.enforce_limit();
            }
        }
        Self::flush_retired(&self.history);
        Self::notify_members(&self.history);
    }
    pub fn is_original_file(&self) -> bool {
        let history = self.history.borrow();
        history.original_state == Some(history.current_state)
    }
    pub fn mark_as_original_file(&self) {
        {
            let mut history = self.history.borrow_mut();
            history.original_state = Some(history.current_state);
        }
        Self::notify_members(&self.history);
    }
    pub fn discard_original_file_marker(&self) {
        self.history.borrow_mut().original_state = None;
        Self::notify_members(&self.history);
    }
    pub fn accept_changes(&self) -> bool {
        !self.history.borrow().replaying
    }
    pub fn undo_count(&self) -> usize {
        self.history.borrow().undo.len()
    }
    pub fn redo_count(&self) -> usize {
        self.history.borrow().redo.len()
    }
    /// Returns typed metadata from the most recently started top-level undo
    /// group. Undo or redo playback clears the descriptor.
    pub fn last_group_descriptor<T: Any>(&self) -> Option<Rc<T>> {
        let descriptor = self.history.borrow().last_group_descriptor.clone()?;
        descriptor.downcast().ok()
    }
    pub fn start_undo_group(&self) -> Result<UpdateGuard, DocumentError> {
        self.begin_group(false, None)
    }
    /// Starts a group with metadata for extensions that want to identify it
    /// later. Nested groups keep the outer group's descriptor.
    pub fn start_undo_group_with_descriptor(
        &self,
        descriptor: Rc<dyn Any>,
    ) -> Result<UpdateGuard, DocumentError> {
        self.begin_group(false, Some(descriptor))
    }
    pub fn start_continued_undo_group(&self) -> Result<UpdateGuard, DocumentError> {
        self.begin_group(true, None)
    }
    /// Continues the previous group when possible and sets its new descriptor.
    /// A nested group keeps the outer group's descriptor.
    pub fn start_continued_undo_group_with_descriptor(
        &self,
        descriptor: Rc<dyn Any>,
    ) -> Result<UpdateGuard, DocumentError> {
        self.begin_group(true, Some(descriptor))
    }

    fn begin_group(
        &self,
        continued: bool,
        descriptor: Option<Rc<dyn Any>>,
    ) -> Result<UpdateGuard, DocumentError> {
        if self.document.uses_history(&self.history) && self.document.is_changing() {
            return Err(DocumentError::ReentrantChange);
        }
        let descriptor = {
            let mut history = self.history.borrow_mut();
            if history.replaying {
                return Err(DocumentError::UpdateInProgress);
            }
            history.begin(continued, descriptor)
        };
        let guard = UpdateGuard::history(self.history.clone());
        self.capture_context();
        drop(descriptor);
        Ok(guard)
    }

    fn capture_context(&self) {
        let capture = self.document.uses_history(&self.history)
            && self.history.borrow_mut().capture_document(&self.document);
        if capture {
            self.document.raise_undo_group_started();
        }
    }

    pub(super) fn flush_retired(history: &SharedUndoHistory) {
        // Release application-owned payloads outside the history borrow.
        let retired = std::mem::take(&mut history.borrow_mut().retired);
        drop(retired);
    }

    pub(super) fn notify_members(history: &SharedUndoHistory) {
        let single = {
            let state = history.borrow();
            (state.members.len() == 1).then(|| state.members[0].upgrade())
        };
        if let Some(single) = single {
            if let Some(document) = single {
                document.notify_history_properties();
            }
            return;
        }
        for document in Self::live_members(history) {
            if document.uses_history(history) {
                document.notify_history_properties();
            }
        }
    }

    fn live_members(history: &SharedUndoHistory) -> Vec<TextDocument> {
        history.borrow().members.iter().filter_map(WeakTextDocument::upgrade).collect()
    }

    fn replay(&self, redo: bool) -> Result<bool, DocumentError> {
        if self.document.uses_history(&self.history) && self.document.is_changing() {
            return Err(DocumentError::ReentrantChange);
        }
        let (group, descriptor) = {
            let mut history = self.history.borrow_mut();
            if history.depth != 0 || history.replaying {
                return Err(DocumentError::UpdateInProgress);
            }
            if self.document.uses_history(&self.history) && self.document.is_read_only() {
                return Err(DocumentError::ReadOnly);
            }
            let stack = if redo { &mut history.redo } else { &mut history.undo };
            let Some(group) = stack.last() else {
                return Ok(false);
            };
            for operation in &group.changes {
                if let UndoOperation::Document(target, ..) = operation {
                    if target.upgrade().is_some_and(|document| document.is_read_only()) {
                        return Err(DocumentError::ReadOnly);
                    }
                }
            }
            let group = stack.pop().expect("checked the top undo group");
            history.replaying = true;
            history.current_state = if redo { group.after } else { group.before };
            history.allow_continue = false;
            if redo {
                history.undo.push(group.clone());
            } else {
                history.redo.push(group.clone());
            }
            (group, history.last_group_descriptor.take())
        };
        struct ResetReplay(SharedUndoHistory);
        impl Drop for ResetReplay {
            fn drop(&mut self) {
                self.0.borrow_mut().replaying = false;
            }
        }
        let reset = ResetReplay(self.history.clone());
        let mut updates = Vec::new();
        let mut replay = |operation: &UndoOperation| self.replay_operation(operation, redo, &mut updates);
        if redo {
            group.changes.iter().try_for_each(&mut replay)?;
        } else {
            group.changes.iter().rev().try_for_each(&mut replay)?;
        }
        for (_, update) in updates {
            update.finish()?;
        }
        Self::notify_members(&self.history);
        drop(reset);
        for member in Self::live_members(&self.history) {
            if member.uses_history(&self.history) {
                member.raise_undo_playback_finished();
            }
        }
        drop(descriptor);
        Ok(true)
    }

    fn replay_operation(
        &self,
        operation: &UndoOperation,
        redo: bool,
        updates: &mut Vec<(TextDocument, UpdateGuard)>,
    ) -> Result<(), DocumentError> {
        match operation {
            UndoOperation::Document(target, change, effect) => {
                if let Some(document) = target.upgrade() {
                    if !updates.iter().any(|(active, _)| active == &document) {
                        updates.push((document.clone(), document.begin_update()?));
                    }
                    let change = if redo { change.clone() } else { change.invert() };
                    document.apply_change_with_effect(change, effect.as_ref(), redo)?;
                }
            }
            UndoOperation::Custom(operation) => {
                if redo {
                    operation.redo();
                } else {
                    operation.undo();
                }
            }
            UndoOperation::WithContext(origin, operation) => {
                if let Some(document) = origin.upgrade() {
                    let context = Self::attached(document, self.history.clone());
                    if redo {
                        operation.redo(&context);
                    } else {
                        operation.undo(&context);
                    }
                }
            }
        }
        Ok(())
    }
}
