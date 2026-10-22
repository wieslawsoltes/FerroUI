use super::line_manager::{LineEvent, LineManager};
use super::text_anchor_tree::TextAnchorTree;
use super::undo_stack::{UndoHistory, UndoOperation};
use super::{
    AnchorMovementType, DocumentChangeEventArgs, DocumentLine, DocumentSubscription,
    DocumentVersion, ILineTracker, ITextSource, LiveDocumentLine, OffsetChangeMap,
    OffsetChangeMappingType, TextAnchor, TextLocation, TextSnapshot, UndoStack,
};
use crate::utils::Event;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

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
    history: UndoHistory,
    is_read_only: bool,
}

struct DocumentInner {
    state: RefCell<DocumentState>,
    lines: RefCell<Option<Rc<LineManager>>>,
    in_change: Cell<bool>,
    finishing: Cell<bool>,
    changing: Event<DocumentChangeEventArgs>,
    changed: Event<DocumentChangeEventArgs>,
    text_changed: Event<()>,
    update_started: Event<()>,
    update_finished: Event<()>,
    text_length_changed: Event<()>,
    line_count_changed: Event<()>,
    property_changed: Event<String>,
}

/// Shared UI-thread document with persistent UTF-16 storage, incremental line
/// metrics, undo groups, immutable snapshots and stable text anchors.
#[derive(Clone)]
pub struct TextDocument(Rc<DocumentInner>);

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
    document: Option<TextDocument>,
}

impl UpdateGuard {
    pub fn finish(mut self) -> Result<(), DocumentError> {
        if let Some(document) = self.document.take() {
            document.end_update()
        } else {
            Ok(())
        }
    }
}
impl Drop for UpdateGuard {
    fn drop(&mut self) {
        if let Some(document) = self.document.take() {
            if std::thread::panicking() {
                document.end_update_silent();
            } else {
                let _ = document.end_update();
            }
        }
    }
}

impl TextDocument {
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
        Self(Rc::new(DocumentInner {
            state: RefCell::new(DocumentState {
                old_text_length: snapshot.text_length(),
                old_line_count: snapshot.line_count(),
                snapshot,
                anchors: TextAnchorTree::default(),
                update_depth: 0,
                change_pending: false,
                history: UndoHistory::default(),
                is_read_only: false,
            }),
            lines: RefCell::new(None),
            in_change: Cell::new(false),
            finishing: Cell::new(false),
            changing: Event::default(),
            changed: Event::default(),
            text_changed: Event::default(),
            update_started: Event::default(),
            update_finished: Event::default(),
            text_length_changed: Event::default(),
            line_count_changed: Event::default(),
            property_changed: Event::default(),
        }))
    }

    pub fn text(&self) -> String {
        self.create_snapshot().text()
    }
    pub fn text_length(&self) -> usize {
        self.0.state.borrow().snapshot.text_length()
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
        self.0.state.borrow_mut().is_read_only = value;
    }
    pub fn is_in_update(&self) -> bool {
        self.0.state.borrow().update_depth != 0
    }
    pub fn undo_stack(&self) -> UndoStack {
        UndoStack(self.clone())
    }
    pub fn can_undo(&self) -> bool {
        !self.0.state.borrow().history.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.0.state.borrow().history.redo.is_empty()
    }
    pub fn undo_count(&self) -> usize {
        self.0.state.borrow().history.undo.len()
    }
    pub fn redo_count(&self) -> usize {
        self.0.state.borrow().history.redo.len()
    }
    pub fn undo_size_limit(&self) -> usize {
        self.0.state.borrow().history.size_limit
    }
    pub fn accept_changes(&self) -> bool {
        !self.0.state.borrow().history.replaying
    }
    pub fn is_modified(&self) -> bool {
        let state = self.0.state.borrow();
        state.history.original_state != Some(state.history.current_state)
    }
    pub fn mark_as_original_file(&self) {
        let mut state = self.0.state.borrow_mut();
        state.history.original_state = Some(state.history.current_state);
    }
    pub fn discard_original_file_marker(&self) {
        self.0.state.borrow_mut().history.original_state = None;
    }
    pub fn set_undo_size_limit(&self, limit: usize) {
        {
            let mut state = self.0.state.borrow_mut();
            state.history.size_limit = limit;
            if state.update_depth == 0 {
                state.history.enforce_limit();
            }
        }
        self.flush_retired_operations();
    }

    pub fn clear_undo_history(&self) -> Result<(), DocumentError> {
        {
            let mut state = self.0.state.borrow_mut();
            if state.update_depth != 0 || state.history.replaying {
                return Err(DocumentError::UpdateInProgress);
            }
            state.history.clear_all();
        }
        self.flush_retired_operations();
        Ok(())
    }

    pub(crate) fn clear_redo_history(&self) -> Result<(), DocumentError> {
        {
            let mut state = self.0.state.borrow_mut();
            if state.history.replaying {
                return Err(DocumentError::UpdateInProgress);
            }
            state.history.clear_redo();
        }
        self.flush_retired_operations();
        Ok(())
    }

    fn flush_retired_operations(&self) {
        // Custom payload destructors may call back into the document. Transfer
        // ownership out of the RefCell before releasing user-owned operations.
        let retired = std::mem::take(&mut self.0.state.borrow_mut().history.retired);
        drop(retired);
    }

    pub(crate) fn push_undo_operation(
        &self,
        operation: UndoOperation,
        optional: bool,
    ) -> Result<bool, DocumentError> {
        let needs_group = {
            let state = self.0.state.borrow();
            if optional && state.update_depth == 0 {
                return Err(DocumentError::NoUpdate);
            }
            if state.history.replaying || state.history.size_limit == 0 {
                return Ok(false);
            }
            state.update_depth == 0
        };
        let group = if needs_group {
            Some(self.begin_update()?)
        } else {
            None
        };
        self.0
            .state
            .borrow_mut()
            .history
            .record_operation(operation, optional);
        self.flush_retired_operations();
        if let Some(group) = group {
            group.finish()?;
        }
        Ok(true)
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
    pub fn text_changed(&self, handler: impl Fn() + 'static) -> DocumentSubscription {
        self.0.text_changed.subscribe(move |_| handler())
    }
    pub fn update_started(&self, handler: impl Fn() + 'static) -> DocumentSubscription {
        self.0.update_started.subscribe(move |_| handler())
    }
    pub fn update_finished(&self, handler: impl Fn() + 'static) -> DocumentSubscription {
        self.0.update_finished.subscribe(move |_| handler())
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
            if first {
                state.history.begin(continued);
            }
            first
        };
        let guard = UpdateGuard {
            document: Some(self.clone()),
        };
        if first {
            self.0.update_started.raise(&());
        }
        Ok(guard)
    }

    fn end_update_silent(&self) {
        let mut state = self.0.state.borrow_mut();
        if state.update_depth > 0 {
            state.update_depth -= 1;
            if state.update_depth == 0 {
                state.history.finish();
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
            state.history.finish();
            state.update_depth = 0;
        }
        drop(finishing);
        self.flush_retired_operations();
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
        self.apply_change(change)?;
        group.finish()
    }

    fn validate_mutation(&self, offset: usize, length: usize) -> Result<(), DocumentError> {
        if self.0.in_change.get() {
            return Err(DocumentError::ReentrantChange);
        }
        let state = self.0.state.borrow();
        if state.history.replaying {
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
            state.history.record(change.clone());
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
        if let Some(manager) = &line_manager {
            manager.change_complete(&change);
        }
        self.flush_retired_operations();
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
                OffsetChangeMappingType::Normal,
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
        self.replay_history(false)
    }
    pub fn redo(&self) -> Result<bool, DocumentError> {
        self.replay_history(true)
    }

    fn replay_history(&self, redo: bool) -> Result<bool, DocumentError> {
        if self.0.in_change.get() {
            return Err(DocumentError::ReentrantChange);
        }
        let history_group = {
            let mut state = self.0.state.borrow_mut();
            if state.update_depth != 0 || state.history.replaying {
                return Err(DocumentError::UpdateInProgress);
            }
            if state.is_read_only {
                return Err(DocumentError::ReadOnly);
            }
            let group = if redo {
                state.history.redo.pop()
            } else {
                state.history.undo.pop()
            };
            let Some(group) = group else {
                return Ok(false);
            };
            state.history.replaying = true;
            state.history.current_state = if redo { group.after } else { group.before };
            state.history.allow_continue = false;
            // Like ordinary replay, a panicking client operation is a caller
            // error. Preserve the group on the opposite stack even in that case.
            if redo {
                state.history.undo.push(group.clone());
            } else {
                state.history.redo.push(group.clone());
            }
            group
        };
        struct ResetReplay(TextDocument);
        impl Drop for ResetReplay {
            fn drop(&mut self) {
                self.0 .0.state.borrow_mut().history.replaying = false;
            }
        }
        let _reset = ResetReplay(self.clone());
        let update = self.begin_update()?;
        if redo {
            for operation in &history_group.changes {
                self.replay_operation(operation, true)?;
            }
        } else {
            for operation in history_group.changes.iter().rev() {
                self.replay_operation(operation, false)?;
            }
        }
        update.finish()?;
        Ok(true)
    }

    fn replay_operation(&self, operation: &UndoOperation, redo: bool) -> Result<(), DocumentError> {
        match operation {
            UndoOperation::Document(change) => self.apply_change(if redo {
                change.clone()
            } else {
                change.invert()
            })?,
            UndoOperation::Custom(operation) => {
                if redo {
                    operation.redo();
                } else {
                    operation.undo();
                }
            }
            UndoOperation::WithContext(operation) => {
                if redo {
                    operation.redo(&self.undo_stack());
                } else {
                    operation.undo(&self.undo_stack());
                }
            }
        }
        Ok(())
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
