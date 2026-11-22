use super::line_manager::{LineEvent, LineManager};
use super::text_anchor_tree::TextAnchorTree;
use super::undo_stack::{DocumentEditEffect, SharedUndoHistory, UndoHistory, UndoOperation};
use super::{
    AnchorMovementType, DocumentChangeEventArgs, DocumentLine, DocumentSubscription,
    DocumentVersion, ILineTracker, ITextSource, LiveDocumentLine, OffsetChangeMap,
    OffsetChangeMappingType, TextAnchor, TextLocation, TextSnapshot, UndoStack,
};
use crate::utils::Event;
use std::any::{Any, TypeId};
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::rc::{Rc, Weak};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentError {
    InvalidOffset {
        offset: usize,
        text_length: usize,
    },
    InvalidRange {
        offset: usize,
        length: usize,
        text_length: usize,
    },
    InvalidLine {
        line_number: usize,
        line_count: usize,
    },
    InvalidLocation,
    InvalidNewLine,
    InvalidOffsetMap,
    DeletedAnchor,
    DeletedLine,
    DifferentDocument,
    InvalidVersion,
    VersionOverflow,
    ReentrantChange,
    ReadOnly,
    NoUpdate,
    UpdateInProgress,
    OverlappingEdits,
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOffset {
                offset,
                text_length,
            } => write!(
                f,
                "UTF-16 offset {offset} is outside document length {text_length}"
            ),
            Self::InvalidRange {
                offset,
                length,
                text_length,
            } => write!(
                f,
                "UTF-16 range ({offset}, {length}) is outside document length {text_length}"
            ),
            Self::InvalidLine {
                line_number,
                line_count,
            } => write!(f, "line {line_number} is outside 1..={line_count}"),
            Self::InvalidLocation => {
                f.write_str("expected a line and column separated by a comma or semicolon")
            }
            Self::InvalidNewLine => f.write_str("expected CR, LF, or CRLF newline sequence"),
            Self::InvalidOffsetMap => f.write_str("the offset map does not describe this edit"),
            Self::DeletedAnchor => f.write_str("the anchor was deleted"),
            Self::DeletedLine => f.write_str("the document line was deleted"),
            Self::DifferentDocument => f.write_str("the versions belong to different documents"),
            Self::InvalidVersion => f.write_str("the version is not a valid successor"),
            Self::VersionOverflow => f.write_str("the document revision counter is exhausted"),
            Self::ReentrantChange => {
                f.write_str("cannot mutate the document during a change notification")
            }
            Self::ReadOnly => f.write_str("the document is read-only"),
            Self::NoUpdate => f.write_str("no document update is active"),
            Self::UpdateInProgress => {
                f.write_str("cannot alter undo history while a document update is active")
            }
            Self::OverlappingEdits => f.write_str("batch edits overlap in the original document"),
        }
    }
}
impl std::error::Error for DocumentError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentEdit {
    pub offset: usize,
    pub length: usize,
    pub text: String,
}

impl DocumentEdit {
    pub fn new(offset: usize, length: usize, text: impl Into<String>) -> Self {
        Self {
            offset,
            length,
            text: text.into(),
        }
    }
}

struct DocumentState {
    snapshot: TextSnapshot,
    anchors: TextAnchorTree,
    update_depth: usize,
    change_pending: bool,
    old_text_length: usize,
    old_line_count: usize,
    is_read_only: bool,
}

struct DocumentInner {
    state: RefCell<DocumentState>,
    history: RefCell<SharedUndoHistory>,
    lines: RefCell<Option<Rc<LineManager>>>,
    extensions: RefCell<HashMap<TypeId, Rc<dyn Any>>>,
    file_name: RefCell<Option<String>>,
    file_name_changed: Event<()>,
    in_change: Cell<bool>,
    finishing: Cell<bool>,
    changing: Event<DocumentChangeEventArgs>,
    tracked_segments: OnceCell<Event<DocumentChangeEventArgs>>,
    changed: Event<DocumentChangeEventArgs>,
    text_changed: Event<()>,
    update_started: Event<()>,
    update_finished: Event<()>,
    undo_group_started: Event<()>,
    undo_playback_finished: Event<()>,
    text_length_changed: Event<()>,
    line_count_changed: Event<()>,
    property_changed: Event<String>,
    notified_can_undo: Cell<bool>,
    notified_can_redo: Cell<bool>,
    notified_is_modified: Cell<bool>,
}

ferroui_system::styled_value!(TextDocument);

/// Shared UI-thread document with persistent UTF-16 storage, incremental line
/// metrics, undo groups, immutable snapshots and stable text anchors.
#[derive(Clone)]
pub struct TextDocument(Rc<DocumentInner>);

/// A document reference that lets an extension observe its owner without
/// extending the lifetime of the live document.
#[derive(Clone)]
pub struct WeakTextDocument(Weak<DocumentInner>);

impl WeakTextDocument {
    pub fn upgrade(&self) -> Option<TextDocument> {
        self.0.upgrade().map(TextDocument)
    }
    pub(super) fn is_alive(&self) -> bool {
        self.0.strong_count() != 0
    }
    pub(super) fn points_to(&self, document: &TextDocument) -> bool {
        self.0.as_ptr() == Rc::as_ptr(&document.0)
    }
}

impl fmt::Debug for TextDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextDocument")
            .field("text_length", &self.text_length())
            .field("line_count", &self.line_count())
            .field("version", &self.version())
            .finish()
    }
}
impl PartialEq for TextDocument {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for TextDocument {}
impl Default for TextDocument {
    fn default() -> Self {
        Self::new()
    }
}

struct FlagReset<'a>(&'a Cell<bool>);
impl Drop for FlagReset<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// RAII edit group. Keep this value alive while making grouped changes.
/// `finish()` closes explicitly; dropping closes it automatically.
#[must_use = "keep the guard alive while performing the grouped document edits"]
pub struct UpdateGuard {
    scope: Option<UpdateScope>,
}

enum UpdateScope {
    Document(TextDocument),
    History(SharedUndoHistory),
}

impl UpdateGuard {
    pub(super) fn history(history: SharedUndoHistory) -> Self {
        Self { scope: Some(UpdateScope::History(history)) }
    }

    pub fn finish(mut self) -> Result<(), DocumentError> {
        match self.scope.take() {
            Some(UpdateScope::Document(document)) => document.end_update(),
            Some(UpdateScope::History(history)) => {
                Self::end_history_group(&history);
                Ok(())
            }
            None => Ok(()),
        }
    }

    fn end_history_group(history: &SharedUndoHistory) {
        let closed = {
            let mut state = history.borrow_mut();
            state.finish();
            state.depth == 0 && !state.replaying
        };
        if !std::thread::panicking() {
            UndoStack::flush_retired(history);
            if closed {
                UndoStack::notify_members(history);
            }
        }
    }
}
impl Drop for UpdateGuard {
    fn drop(&mut self) {
        match self.scope.take() {
            Some(UpdateScope::Document(document)) if std::thread::panicking() => {
                document.end_update_silent();
            }
            Some(UpdateScope::Document(document)) => {
                let _ = document.end_update();
            }
            Some(UpdateScope::History(history)) => Self::end_history_group(&history),
            None => {}
        }
    }
}

impl TextDocument {
    pub fn downgrade(&self) -> WeakTextDocument {
        WeakTextDocument(Rc::downgrade(&self.0))
    }

    pub fn new() -> Self {
        Self::from_snapshot(TextSnapshot::empty())
    }
    pub fn from_text(text: &str) -> Self {
        Self::from_snapshot(TextSnapshot::from_text(text))
    }
    pub fn from_utf16(text: &[u16]) -> Self {
        Self::from_snapshot(TextSnapshot::from_utf16(text))
    }

    pub fn from_snapshot(mut snapshot: TextSnapshot) -> Self {
        snapshot.version = Some(DocumentVersion::initial());
        let history = Rc::new(RefCell::new(UndoHistory::default()));
        let document = Self(Rc::new(DocumentInner {
            state: RefCell::new(DocumentState {
                old_text_length: snapshot.text_length(),
                old_line_count: snapshot.line_count(),
                snapshot,
                anchors: TextAnchorTree::default(),
                update_depth: 0,
                change_pending: false,
                is_read_only: false,
            }),
            history: RefCell::new(history.clone()),
            lines: RefCell::new(None),
            extensions: RefCell::new(HashMap::new()),
            file_name: RefCell::new(None),
            file_name_changed: Event::default(),
            in_change: Cell::new(false),
            finishing: Cell::new(false),
            changing: Event::default(),
            tracked_segments: OnceCell::new(),
            changed: Event::default(),
            text_changed: Event::default(),
            update_started: Event::default(),
            update_finished: Event::default(),
            undo_group_started: Event::default(),
            undo_playback_finished: Event::default(),
            text_length_changed: Event::default(),
            line_count_changed: Event::default(),
            property_changed: Event::default(),
            notified_can_undo: Cell::new(false),
            notified_can_redo: Cell::new(false),
            notified_is_modified: Cell::new(false),
        }));
        history.borrow_mut().attach(&document);
        document
    }

    pub fn text(&self) -> String {
        self.create_snapshot().text()
    }
    pub fn text_length(&self) -> usize {
        self.0.state.borrow().snapshot.text_length()
    }

    /// Gets application state attached to this live document by type.
    pub fn extension<T: 'static>(&self) -> Option<Rc<T>> {
        self.0.extensions.borrow().get(&TypeId::of::<T>())?.clone().downcast().ok()
    }

    /// Shares one lazily initialized extension across handles to this document.
    /// Extensions are independent of its text, snapshots, and undo history.
    pub fn get_or_insert_extension_with<T: 'static>(&self, create: impl FnOnce() -> Rc<T>) -> Rc<T> {
        if let Some(existing) = self.extension() {
            return existing;
        }
        let value = create();
        let stored = self.0.extensions.borrow_mut().entry(TypeId::of::<T>())
            .or_insert_with(|| value.clone()).clone();
        stored.downcast().expect("extension type agrees with its key")
    }
    /// Host-owned file metadata; changing it does not edit text or history.
    pub fn file_name(&self) -> Option<String> {
        self.0.file_name.borrow().clone()
    }
    pub fn set_file_name(&self, value: Option<String>) -> bool {
        {
            let mut name = self.0.file_name.borrow_mut();
            if *name == value {
                return false;
            }
            *name = value;
        }
        self.0.file_name_changed.raise(&());
        true
    }
    pub fn file_name_changed(&self, handler: impl Fn() + 'static) -> DocumentSubscription {
        self.0.file_name_changed.subscribe(move |_| handler())
    }
    pub fn line_count(&self) -> usize {
        self.0.state.borrow().snapshot.line_count()
    }
    pub fn get_text(&self, offset: usize, length: usize) -> Result<String, DocumentError> {
        self.create_snapshot().get_text(offset, length)
    }
    pub fn get_text_utf16(&self, offset: usize, length: usize) -> Result<Vec<u16>, DocumentError> {
        self.create_snapshot().get_text_utf16(offset, length)
    }
    pub fn get_char_at(&self, offset: usize) -> Result<u16, DocumentError> {
        self.create_snapshot().get_char_at(offset)
    }
    pub fn create_snapshot(&self) -> TextSnapshot {
        self.0.state.borrow().snapshot.clone()
    }
    pub fn create_snapshot_range(
        &self,
        offset: usize,
        length: usize,
    ) -> Result<TextSnapshot, DocumentError> {
        self.create_snapshot().slice(offset, length)
    }
    pub fn version(&self) -> DocumentVersion {
        self.0
            .state
            .borrow()
            .snapshot
            .version
            .clone()
            .expect("documents always have a version")
    }
    pub fn get_line_by_number(&self, line_number: usize) -> Result<DocumentLine, DocumentError> {
        self.create_snapshot().get_line_by_number(line_number)
    }
    pub fn get_line_by_offset(&self, offset: usize) -> Result<DocumentLine, DocumentError> {
        self.create_snapshot().get_line_by_offset(offset)
    }
    fn ensure_live_lines(&self) -> Rc<LineManager> {
        if let Some(manager) = self.0.lines.borrow().clone() {
            return manager;
        }
        let manager = LineManager::new(&self.create_snapshot());
        *self.0.lines.borrow_mut() = Some(manager.clone());
        manager
    }
    pub fn get_live_line_by_number(
        &self,
        line_number: usize,
    ) -> Result<LiveDocumentLine, DocumentError> {
        self.ensure_live_lines().by_number(line_number)
    }
    pub fn get_live_line_by_offset(
        &self,
        offset: usize,
    ) -> Result<LiveDocumentLine, DocumentError> {
        self.ensure_live_lines().by_offset(offset)
    }
    pub fn live_lines(&self) -> Vec<LiveDocumentLine> {
        self.ensure_live_lines().lines()
    }
    pub fn add_line_tracker(&self, tracker: Rc<dyn ILineTracker>) -> DocumentSubscription {
        self.subscribe_line_events(move |event| event.dispatch(tracker.as_ref()))
    }
    pub(crate) fn subscribe_line_events(
        &self,
        handler: impl Fn(&LineEvent) + 'static,
    ) -> DocumentSubscription {
        self.ensure_live_lines().subscribe(handler)
    }
    pub fn get_location(&self, offset: usize) -> Result<TextLocation, DocumentError> {
        self.create_snapshot().get_location(offset)
    }
    pub fn get_offset(&self, location: TextLocation) -> Result<usize, DocumentError> {
        self.create_snapshot().get_offset(location)
    }
    pub fn get_offset_at(&self, line: usize, column: usize) -> Result<usize, DocumentError> {
        self.get_offset(TextLocation::new(line, column))
    }

    pub fn lines(&self) -> impl ExactSizeIterator<Item = DocumentLine> {
        let snapshot = self.create_snapshot();
        (0..snapshot.line_count()).map(move |line| {
            snapshot
                .get_line_by_number(line + 1)
                .expect("validated line iterator")
        })
    }

    pub fn is_read_only(&self) -> bool {
        self.0.state.borrow().is_read_only
    }
    pub fn set_is_read_only(&self, value: bool) {
        let changed = {
            let mut state = self.0.state.borrow_mut();
            std::mem::replace(&mut state.is_read_only, value) != value
        };
        if changed {
            self.0.property_changed.raise(&"IsReadOnly".to_owned());
        }
    }
    pub fn is_in_update(&self) -> bool {
        self.0.state.borrow().update_depth != 0
    }
    pub fn undo_stack(&self) -> UndoStack {
        UndoStack::attached(self.clone(), self.history())
    }
    fn history(&self) -> SharedUndoHistory {
        self.0.history.borrow().clone()
    }
    pub(super) fn uses_history(&self, history: &SharedUndoHistory) -> bool {
        Rc::ptr_eq(&self.0.history.borrow(), history)
    }
    pub(super) fn is_changing(&self) -> bool {
        self.0.in_change.get()
    }
    /// Attaches this document to another history. The old history is cleared
    /// so saved handles cannot replay stale changes against this document.
    pub fn set_undo_stack(&self, stack: &UndoStack) -> Result<(), DocumentError> {
        let old = self.history();
        if Rc::ptr_eq(&old, &stack.history) {
            return Ok(());
        }
        if self.is_changing() {
            return Err(DocumentError::ReentrantChange);
        }
        if self.is_in_update() || stack.history.borrow().replaying {
            return Err(DocumentError::UpdateInProgress);
        }
        let descriptor = {
            let mut history = old.borrow_mut();
            if history.depth != 0 || history.replaying {
                return Err(DocumentError::UpdateInProgress);
            }
            history.detach(self);
            history.clear_all()
        };
        *self.0.history.borrow_mut() = stack.history.clone();
        stack.history.borrow_mut().attach(self);
        drop(descriptor);
        UndoStack::flush_retired(&old);
        UndoStack::notify_members(&old);
        self.0.property_changed.raise(&"UndoStack".to_owned());
        self.notify_history_properties();
        Ok(())
    }
    pub fn can_undo(&self) -> bool {
        self.undo_count() != 0
    }
    pub fn can_redo(&self) -> bool {
        self.redo_count() != 0
    }
    pub fn undo_count(&self) -> usize {
        self.0.history.borrow().borrow().undo.len()
    }
    pub fn redo_count(&self) -> usize {
        self.0.history.borrow().borrow().redo.len()
    }
    pub fn undo_size_limit(&self) -> usize {
        self.0.history.borrow().borrow().size_limit
    }
    pub fn accept_changes(&self) -> bool {
        !self.0.history.borrow().borrow().replaying
    }
    pub fn is_modified(&self) -> bool {
        !self.undo_stack().is_original_file()
    }
    pub fn mark_as_original_file(&self) {
        self.undo_stack().mark_as_original_file();
    }
    pub fn discard_original_file_marker(&self) {
        self.undo_stack().discard_original_file_marker();
    }
    pub fn set_undo_size_limit(&self, limit: usize) {
        self.undo_stack().set_size_limit(limit);
    }

    pub fn clear_undo_history(&self) -> Result<(), DocumentError> {
        self.undo_stack().clear_all()
    }

    pub(super) fn notify_history_properties(&self) {
        // Store each observed value before calling client code. A callback may
        // query or change history, so re-read the next property after it returns.
        if self.0.notified_can_undo.replace(self.can_undo()) != self.can_undo() {
            self.0.property_changed.raise(&"CanUndo".to_owned());
        }
        if self.0.notified_can_redo.replace(self.can_redo()) != self.can_redo() {
            self.0.property_changed.raise(&"CanRedo".to_owned());
        }
        if self.0.notified_is_modified.replace(self.is_modified()) != self.is_modified() {
            self.0.property_changed.raise(&"IsModified".to_owned());
        }
    }

    pub fn changing(
        &self,
        handler: impl Fn(&DocumentChangeEventArgs) + 'static,
    ) -> DocumentSubscription {
        self.0.changing.subscribe(handler)
    }
    pub fn changed(
        &self,
        handler: impl Fn(&DocumentChangeEventArgs) + 'static,
    ) -> DocumentSubscription {
        self.0.changed.subscribe(handler)
    }
    pub(super) fn track_segments(
        &self,
        handler: impl Fn(&DocumentChangeEventArgs) + 'static,
    ) -> DocumentSubscription {
        self.0
            .tracked_segments
            .get_or_init(Event::default)
            .subscribe(handler)
    }
    pub fn text_changed(&self, handler: impl Fn() + 'static) -> DocumentSubscription {
        self.0.text_changed.subscribe(move |_| handler())
    }
    pub fn update_started(&self, handler: impl Fn() + 'static) -> DocumentSubscription {
        self.0.update_started.subscribe(move |_| handler())
    }
    pub fn update_finished(&self, handler: impl Fn() + 'static) -> DocumentSubscription {
        self.0.update_finished.subscribe(move |_| handler())
    }
    /// Runs before this document first participates in a history group.
    pub fn undo_group_started(&self, handler: impl Fn() + 'static) -> DocumentSubscription {
        self.0.undo_group_started.subscribe(move |_| handler())
    }
    /// Runs after an undo or redo, including groups containing only custom actions.
    pub fn undo_playback_finished(&self, handler: impl Fn() + 'static) -> DocumentSubscription {
        self.0.undo_playback_finished.subscribe(move |_| handler())
    }
    pub(super) fn raise_undo_group_started(&self) {
        self.0.undo_group_started.raise(&());
    }
    pub(super) fn raise_undo_playback_finished(&self) {
        self.0.undo_playback_finished.raise(&());
    }
    pub fn text_length_changed(&self, handler: impl Fn() + 'static) -> DocumentSubscription {
        self.0.text_length_changed.subscribe(move |_| handler())
    }
    pub fn line_count_changed(&self, handler: impl Fn() + 'static) -> DocumentSubscription {
        self.0.line_count_changed.subscribe(move |_| handler())
    }
    pub fn property_changed(&self, handler: impl Fn(&str) + 'static) -> DocumentSubscription {
        self.0
            .property_changed
            .subscribe(move |property| handler(property))
    }

    pub fn begin_update(&self) -> Result<UpdateGuard, DocumentError> {
        self.begin_update_impl(false)
    }
    /// Starts a manually paired group. Call `end_update` or the document
    /// interface's `end_undoable_action` exactly once after a successful call.
    pub fn start_undoable_action(&self) -> Result<(), DocumentError> {
        let mut guard = self.begin_update()?;
        guard.scope.take();
        Ok(())
    }
    pub fn run_update(&self) -> Result<UpdateGuard, DocumentError> {
        self.begin_update()
    }
    pub fn begin_continued_update(&self) -> Result<UpdateGuard, DocumentError> {
        self.begin_update_impl(true)
    }

    fn begin_update_impl(&self, continued: bool) -> Result<UpdateGuard, DocumentError> {
        if self.0.in_change.get() {
            return Err(DocumentError::ReentrantChange);
        }
        let first = {
            let mut state = self.0.state.borrow_mut();
            let first = state.update_depth == 0;
            state.update_depth += 1;
            first
        };
        let guard = UpdateGuard {
            scope: Some(UpdateScope::Document(self.clone())),
        };
        if first {
            let (capture, descriptor) = {
                let history = self.history();
                let mut history = history.borrow_mut();
                let descriptor = history.begin(continued, None);
                (history.capture_document(self), descriptor)
            };
            if capture {
                self.raise_undo_group_started();
            }
            self.0.update_started.raise(&());
            drop(descriptor);
        }
        Ok(guard)
    }

    fn end_update_silent(&self) {
        let mut state = self.0.state.borrow_mut();
        if state.update_depth > 0 {
            state.update_depth -= 1;
            if state.update_depth == 0 {
                self.0.history.borrow().borrow_mut().finish();
            }
        }
    }

    pub fn end_update(&self) -> Result<(), DocumentError> {
        if self.0.in_change.get() {
            return Err(DocumentError::ReentrantChange);
        }
        {
            let mut state = self.0.state.borrow_mut();
            if state.update_depth == 0 {
                return Err(DocumentError::NoUpdate);
            }
            if state.update_depth > 1 {
                state.update_depth -= 1;
                return Ok(());
            }
            if self.0.finishing.get() {
                return Err(DocumentError::ReentrantChange);
            }
        }
        self.0.finishing.set(true);
        let finishing = FlagReset(&self.0.finishing);
        loop {
            let pending = {
                let mut state = self.0.state.borrow_mut();
                std::mem::take(&mut state.change_pending)
            };
            if !pending {
                break;
            }
            self.0.text_changed.raise(&());
            self.0.property_changed.raise(&"Text".to_owned());
            let (length_changed, lines_changed) = {
                let mut state = self.0.state.borrow_mut();
                let length = state.snapshot.text_length();
                let lines = state.snapshot.line_count();
                let result = (
                    state.old_text_length != length,
                    state.old_line_count != lines,
                );
                state.old_text_length = length;
                state.old_line_count = lines;
                result
            };
            if length_changed {
                self.0.text_length_changed.raise(&());
                self.0.property_changed.raise(&"TextLength".to_owned());
            }
            if lines_changed {
                self.0.line_count_changed.raise(&());
                self.0.property_changed.raise(&"LineCount".to_owned());
            }
        }
        {
            let mut state = self.0.state.borrow_mut();
            state.update_depth = 0;
        }
        drop(finishing);
        UpdateGuard::end_history_group(&self.history());
        self.0.update_finished.raise(&());
        Ok(())
    }

    pub fn create_anchor(&self, offset: usize) -> Result<TextAnchor, DocumentError> {
        let text_length = self.text_length();
        if offset > text_length {
            return Err(DocumentError::InvalidOffset {
                offset,
                text_length,
            });
        }
        let anchor = TextAnchor::new(self.clone());
        let node = self
            .0
            .state
            .borrow_mut()
            .anchors
            .insert(offset, Rc::downgrade(&anchor.0));
        anchor.0.node.set(Some(node));
        Ok(anchor)
    }

    pub(super) fn anchor_offset(&self, node: usize) -> Option<usize> {
        self.0.state.borrow().anchors.offset(node)
    }
    pub(super) fn remove_anchor(&self, node: usize) {
        self.0.state.borrow_mut().anchors.remove(node);
    }

    #[cfg(test)]
    pub(super) fn validate_anchor_tree(&self) {
        self.0.state.borrow().anchors.validate();
    }

    pub fn set_text(&self, text: &str) -> Result<(), DocumentError> {
        self.replace(0, self.text_length(), text)
    }
    pub fn insert(&self, offset: usize, text: &str) -> Result<(), DocumentError> {
        self.replace(offset, 0, text)
    }
    pub fn insert_with_movement(
        &self,
        offset: usize,
        text: &str,
        movement: AnchorMovementType,
    ) -> Result<(), DocumentError> {
        let mapping = if movement == AnchorMovementType::BeforeInsertion {
            OffsetChangeMappingType::KeepAnchorBeforeInsertion
        } else {
            OffsetChangeMappingType::Normal
        };
        self.replace_with_mapping(offset, 0, text, mapping)
    }
    pub fn remove(&self, offset: usize, length: usize) -> Result<(), DocumentError> {
        self.replace(offset, length, "")
    }
    pub fn replace(&self, offset: usize, length: usize, text: &str) -> Result<(), DocumentError> {
        self.replace_with_mapping(offset, length, text, OffsetChangeMappingType::Normal)
    }
    pub fn replace_utf16(
        &self,
        offset: usize,
        length: usize,
        text: &[u16],
    ) -> Result<(), DocumentError> {
        self.replace_snapshot(
            offset,
            length,
            TextSnapshot::from_utf16(text),
            OffsetChangeMappingType::Normal,
        )
    }
    pub fn replace_with_mapping(
        &self,
        offset: usize,
        length: usize,
        text: &str,
        mapping: OffsetChangeMappingType,
    ) -> Result<(), DocumentError> {
        self.replace_snapshot(offset, length, TextSnapshot::from_text(text), mapping)
    }

    pub fn replace_snapshot(
        &self,
        offset: usize,
        length: usize,
        text: TextSnapshot,
        mapping: OffsetChangeMappingType,
    ) -> Result<(), DocumentError> {
        let map = OffsetChangeMap::for_change(offset, length, text.text_length(), mapping);
        self.replace_with_map(offset, length, text, map)
    }

    pub fn replace_with_map(
        &self,
        offset: usize,
        length: usize,
        text: TextSnapshot,
        map: OffsetChangeMap,
    ) -> Result<(), DocumentError> {
        self.replace_with_map_and_effect(offset, length, text, map, None)
    }

    pub(crate) fn insert_utf16_with_effect(
        &self,
        offset: usize,
        text: &[u16],
        effect: DocumentEditEffect,
    ) -> Result<(), DocumentError> {
        let map = OffsetChangeMap::for_change(offset, 0, text.len(), OffsetChangeMappingType::Normal);
        self.replace_with_map_and_effect(offset, 0, TextSnapshot::from_utf16(text), map, Some(effect))
    }

    fn replace_with_map_and_effect(
        &self,
        offset: usize,
        length: usize,
        text: TextSnapshot,
        map: OffsetChangeMap,
        effect: Option<DocumentEditEffect>,
    ) -> Result<(), DocumentError> {
        self.validate_mutation(offset, length)?;
        if !map.is_valid_for_document_change(offset, length, text.text_length()) {
            return Err(DocumentError::InvalidOffsetMap);
        }
        if length == 0 && text.text_length() == 0 {
            return Ok(());
        }
        let group = self.begin_update()?;
        // UpdateStarted subscribers can edit the document, so validate against
        // the actual revision after that event has completed.
        self.validate_mutation(offset, length)?;
        let removed = self.create_snapshot_range(offset, length)?;
        let change = DocumentChangeEventArgs::with_map(offset, removed, text, map)?;
        self.apply_change_with_effect(change, effect.as_ref(), true)?;
        group.finish()
    }

    fn validate_mutation(&self, offset: usize, length: usize) -> Result<(), DocumentError> {
        if self.0.in_change.get() {
            return Err(DocumentError::ReentrantChange);
        }
        let state = self.0.state.borrow();
        if self.0.history.borrow().borrow().replaying {
            return Err(DocumentError::ReentrantChange);
        }
        if state.is_read_only {
            return Err(DocumentError::ReadOnly);
        }
        let text_length = state.snapshot.text_length();
        if offset > text_length || length > text_length - offset {
            return Err(DocumentError::InvalidRange {
                offset,
                length,
                text_length,
            });
        }
        Ok(())
    }

    fn apply_change(&self, change: DocumentChangeEventArgs) -> Result<(), DocumentError> {
        self.apply_change_with_effect(change, None, true)
    }

    pub(super) fn apply_change_with_effect(
        &self,
        change: DocumentChangeEventArgs,
        effect: Option<&DocumentEditEffect>,
        forward: bool,
    ) -> Result<(), DocumentError> {
        if self.0.in_change.replace(true) {
            return Err(DocumentError::ReentrantChange);
        }
        let _changing = FlagReset(&self.0.in_change);
        let snapshot = self.create_snapshot();
        let rope = snapshot
            .rope
            .replace(
                change.offset,
                change.removal_length(),
                &change.inserted_text.rope,
            )
            .ok_or(DocumentError::InvalidRange {
                offset: change.offset,
                length: change.removal_length(),
                text_length: snapshot.text_length(),
            })?;
        let version = snapshot
            .version
            .clone()
            .expect("documents always have a version");
        if version.revision() == u64::MAX {
            return Err(DocumentError::VersionOverflow);
        }
        self.0.changing.raise(&change);
        let line_manager = self.0.lines.borrow().clone();
        let version = version.append(change.clone())?;
        let mut deleted = Vec::new();
        {
            let mut state = self.0.state.borrow_mut();
            state.snapshot = TextSnapshot {
                rope,
                version: Some(version),
            };
            state.change_pending = true;
        }
        let history = self.history();
        history.borrow_mut().record_operation(
            UndoOperation::Document(self.downgrade(), change.clone(), effect.cloned()),
            false,
        );
        if let Some(effect) = effect {
            effect(self, forward);
        }
        if let Some(manager) = &line_manager {
            manager.apply_change(&snapshot, &self.create_snapshot(), &change);
        }
        {
            let mut state = self.0.state.borrow_mut();
            for entry in change.offset_change_map.entries() {
                state.anchors.handle_text_change(entry, &mut deleted);
            }
        }
        // Connected segment collections must agree with the new document
        // before public observers can query their ranges.
        if let Some(tracked_segments) = self.0.tracked_segments.get() {
            tracked_segments.raise(&change);
        }
        if let Some(manager) = &line_manager {
            manager.change_complete(&change);
        }
        UndoStack::flush_retired(&history);
        // All anchor deletion flags are set before any deletion handler runs.
        for anchor in deleted {
            anchor.deleted_event.raise(&());
        }
        self.0.changed.raise(&change);
        Ok(())
    }

    /// Applies nonoverlapping edits expressed in the same original revision.
    /// All ranges are checked before editing, and the result is one undo step.
    pub fn apply_edits(&self, edits: &[DocumentEdit]) -> Result<(), DocumentError> {
        self.apply_edits_with_mapping(edits, OffsetChangeMappingType::Normal)
    }

    /// Applies a complete edit batch using the requested anchor-mapping policy.
    /// The policy is shared by all replacements, while ranges and read-only
    /// state are validated for the entire batch before its first mutation.
    pub fn apply_edits_with_mapping(
        &self,
        edits: &[DocumentEdit],
        mapping: OffsetChangeMappingType,
    ) -> Result<(), DocumentError> {
        self.validate_mutation(0, 0)?;
        let mut edits: Vec<_> = edits.iter().collect();
        edits.sort_by_key(|edit| edit.offset);
        let validate = || -> Result<(), DocumentError> {
            let mut previous_end = 0;
            for (index, edit) in edits.iter().enumerate() {
                self.validate_mutation(edit.offset, edit.length)?;
                if index != 0 && edit.offset < previous_end {
                    return Err(DocumentError::OverlappingEdits);
                }
                previous_end = edit.offset + edit.length;
            }
            Ok(())
        };
        validate()?;
        if edits.is_empty() {
            return Ok(());
        }
        let group = self.begin_update()?;
        validate()?;
        for edit in edits.into_iter().rev() {
            let removed = self.create_snapshot_range(edit.offset, edit.length)?;
            let inserted = TextSnapshot::from_text(&edit.text);
            let map = OffsetChangeMap::for_change(
                edit.offset,
                edit.length,
                inserted.text_length(),
                mapping,
            );
            if edit.length != 0 || inserted.text_length() != 0 {
                self.apply_change(DocumentChangeEventArgs::with_map(
                    edit.offset,
                    removed,
                    inserted,
                    map,
                )?)?;
            }
        }
        group.finish()
    }

    pub fn undo(&self) -> Result<bool, DocumentError> {
        self.undo_stack().undo()
    }
    pub fn redo(&self) -> Result<bool, DocumentError> {
        self.undo_stack().redo()
    }
}

impl ITextSource for TextDocument {
    fn text_length(&self) -> usize {
        self.text_length()
    }
    fn get_char_at(&self, offset: usize) -> Result<u16, DocumentError> {
        self.get_char_at(offset)
    }
    fn get_text(&self, offset: usize, length: usize) -> Result<String, DocumentError> {
        self.get_text(offset, length)
    }
    fn get_text_utf16(&self, offset: usize, length: usize) -> Result<Vec<u16>, DocumentError> {
        self.get_text_utf16(offset, length)
    }
    fn create_snapshot(&self) -> TextSnapshot {
        self.create_snapshot()
    }
}
