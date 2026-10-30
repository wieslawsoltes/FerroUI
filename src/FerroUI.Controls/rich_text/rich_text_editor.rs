use super::{RangeGravity, SelectionOptions, SelectionType};
use crate::documents::{
    CharacterFormat, DocumentError, DocumentSnapshot, ParagraphFormat, RichTextDocument,
    TextGravity,
};
use ferroui_base::media::TextDecorations;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{Ref, WeakRef};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::ops::Range;
use std::rc::Rc;

/// A directed selection. Offsets count UTF-16 code units and always lie on scalar boundaries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EditorSelection {
    pub anchor: usize,
    pub active: usize,
}

impl EditorSelection {
    pub const fn new(anchor: usize, active: usize) -> Self {
        Self { anchor, active }
    }
    pub const fn caret(offset: usize) -> Self {
        Self::new(offset, offset)
    }
    pub fn range(self) -> Range<usize> {
        self.anchor.min(self.active)..self.anchor.max(self.active)
    }
    pub fn is_empty(self) -> bool {
        self.anchor == self.active
    }
}

/// The semantic boundary used when grouping adjacent keyboard edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKind {
    Typing,
    Delete,
    Paste,
    Formatting,
    Composition,
    Programmatic,
    Group,
}

/// Accepted IME replacements, after normalization and the editor's length limit.
///
/// Preview carries only the replacement substring that differs from the previous
/// preview, on Unicode scalar boundaries. Finalized carries the complete accepted
/// commit, or an empty string when the preview is cancelled or no text is accepted.
/// These causes support TextEdit automation without inferring composition from an
/// ordinary text-change notification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RichTextCompositionChange {
    Preview { changed_text: String },
    Finalized { text: String },
}

/// The accepted final text and range captured by a composition transaction.
/// Control events use this result rather than reconstructing accepted input from
/// a caret that an application may move while the edit is being published.
pub(crate) struct CompositionCommitResult {
    pub text: String,
    pub range: Range<usize>,
    pub selection: EditorSelection,
    pub mutation_version: u64,
}

/// An editing failure. Groups roll back ordinary transaction changes. Detected
/// independent application edits survive failure and invalidate stale undo and
/// composition state instead of being overwritten by an earlier snapshot.
#[derive(Debug)]
#[non_exhaustive]
pub enum EditError {
    ReadOnly,
    ReentrantEdit,
    Document(DocumentError),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadOnly => f.write_str("The rich text document is read-only."),
            Self::ReentrantEdit => f.write_str("An edit is already being published."),
            Self::Document(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for EditError {}
impl From<DocumentError> for EditError {
    fn from(error: DocumentError) -> Self {
        Self::Document(error)
    }
}

#[derive(Clone, PartialEq)]
struct EditorSnapshot {
    document: DocumentSnapshot,
    selection: EditorSelection,
    typing_format: CharacterFormat,
    default_character_format: CharacterFormat,
    default_paragraph_format: ParagraphFormat,
}

#[derive(Clone)]
/// One undo unit. The snapshots are shared: a unit's `before` is the `after` of the
/// unit before it whenever nothing changed in between, so neighbouring units hold one
/// copy of the document, not two (#243 review, S8; delta units follow in F1).
struct UndoUnit {
    before: Rc<EditorSnapshot>,
    after: Rc<EditorSnapshot>,
    kind: EditKind,
    merge_generation: u64,
}

#[derive(Clone)]
struct Composition {
    // Deep format reads must never run while History is borrowed.
    before: Rc<EditorSnapshot>,
    preview_version: u64,
    preview_text: String,
    exact: Option<ExactComposition>,
}

#[derive(Clone)]
struct ExactComposition {
    /// Current structured content before temporary IME decorations.
    document: Rc<DocumentSnapshot>,
    /// A native start has no composed range yet. Its first replacement defines
    /// the extent independently of the committed selection and the 0/0 event
    /// sentinel used while the native composition string is unavailable.
    range: Option<Range<usize>>,
}

#[derive(Default)]
struct History {
    undo: Vec<UndoUnit>,
    redo: Vec<UndoUnit>,
    composition: Option<Composition>,
}

#[derive(Clone, Copy)]
struct NavigationAffinity {
    vertical_x: Option<f64>,
    line_end_start: Option<usize>,
    selection: EditorSelection,
    mutation_version: u64,
    checkpoint: u64,
}

/// Portable rich-text transactions shared by input, commands, clipboard, and automation.
///
/// The document remains the authoritative structured object model. Undo snapshots retain
/// spans, hyperlinks, paragraph properties, and inline controls; no plain-text reconstruction
/// is involved. An external mutation invalidates history before the next edit, so undo cannot
/// silently overwrite an application's subsequent document changes.
pub struct RichTextEditor {
    document: RichTextDocument,
    text_view: RefCell<Option<WeakRef<crate::RichTextBlock>>>,
    selection: Cell<EditorSelection>,
    selection_options: Cell<SelectionOptions>,
    selection_options_generation: Cell<u64>,
    selection_gravity: Cell<RangeGravity>,
    selection_gravity_generation: Cell<u64>,
    navigation_affinity: Cell<Option<NavigationAffinity>>,
    navigation_generation: Cell<u64>,
    typing_format: RefCell<Rc<CharacterFormat>>,
    default_character_format: RefCell<Rc<CharacterFormat>>,
    default_paragraph_format: RefCell<Rc<ParagraphFormat>>,
    selection_projection: RefCell<Option<Rc<dyn Fn(EditorSelection) -> EditorSelection>>>,
    projecting_selection: Cell<bool>,
    history: RefCell<History>,
    history_clear_generation: Cell<u64>,
    known_revision: Cell<u64>,
    merge_generation: Cell<u64>,
    /// Set while a Backspace at a collapsed caret is recorded: it joins the
    /// typing unit before it (#243 review, S1).
    recording_backspace: Cell<bool>,
    is_read_only: Cell<bool>,
    max_length: Cell<usize>,
    undo_limit: Cell<usize>,
    in_operation: Cell<bool>,
    group_depth: Cell<usize>,
    external_change_generation: Cell<u64>,
    changed: Rc<HandlerList<dyn Fn()>>,
    composition_changed: Rc<HandlerList<dyn Fn(&RichTextCompositionChange)>>,
    pending_composition_changes: RefCell<Vec<RichTextCompositionChange>>,
}

struct Operation<'a>(&'a Cell<bool>);
impl Drop for Operation<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

struct SelectionProjection<'a> {
    flag: &'a Cell<bool>,
    previous: bool,
}
impl Drop for SelectionProjection<'_> {
    fn drop(&mut self) {
        self.flag.set(self.previous);
    }
}

impl RichTextEditor {
    pub fn new(document: RichTextDocument) -> Self {
        Self {
            known_revision: Cell::new(document.revision()),
            document,
            text_view: RefCell::new(None),
            selection: Cell::new(EditorSelection::default()),
            selection_options: Cell::new(SelectionOptions::NONE),
            selection_options_generation: Cell::new(0),
            selection_gravity: Cell::new(RangeGravity::default()),
            selection_gravity_generation: Cell::new(0),
            navigation_affinity: Cell::new(None),
            navigation_generation: Cell::new(0),
            typing_format: RefCell::new(Rc::new(CharacterFormat::default())),
            default_character_format: RefCell::new(Rc::new(CharacterFormat::default())),
            default_paragraph_format: RefCell::new(Rc::new(ParagraphFormat::default())),
            selection_projection: RefCell::new(None),
            projecting_selection: Cell::new(false),
            history: RefCell::new(History::default()),
            history_clear_generation: Cell::new(0),
            merge_generation: Cell::new(0),
            recording_backspace: Cell::new(false),
            is_read_only: Cell::new(false),
            max_length: Cell::new(0),
            undo_limit: Cell::new(100),
            in_operation: Cell::new(false),
            group_depth: Cell::new(0),
            external_change_generation: Cell::new(0),
            changed: Rc::new(HandlerList::new()),
            composition_changed: Rc::new(HandlerList::new()),
            pending_composition_changes: RefCell::new(Vec::new()),
        }
    }

    #[doc(hidden)]
    pub fn rich_document(&self) -> RichTextDocument {
        self.document()
    }
    pub fn document(&self) -> RichTextDocument {
        self.document.clone()
    }

    /// Associates the live text layout used by display-dependent TOM units. The
    /// weak association does not keep a control alive or participate in history.
    pub fn set_text_view(&self, view: Option<Ref<crate::RichTextBlock>>) -> Result<(), EditError> {
        if view
            .as_ref()
            .is_some_and(|view| view.document() != self.document)
        {
            return Err(DocumentError::InvalidFormat(
                "The text view must display this editor's document.",
            )
            .into());
        }
        // Layout and scrolling re-announce the presenter's live association.
        // Keep its caret affinity when the validated view is unchanged. An
        // explicit detach still invalidates state, including an expired weak
        // association, so a subsequent attachment starts a new navigation run.
        if view
            .as_ref()
            .is_some_and(|view| self.text_view().as_ref() == Some(view))
        {
            return Ok(());
        }
        let previous = self.text_view.replace(view.as_ref().map(Ref::downgrade));
        drop(previous);
        self.clear_navigation_affinity();
        Ok(())
    }

    pub(super) fn text_view(&self) -> Option<Ref<crate::RichTextBlock>> {
        self.text_view.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    pub fn selection(&self) -> EditorSelection {
        self.selection.get()
    }

    /// Accepted selection changes and edit checkpoints invalidate stored caret
    /// affinity. Commands record it only after accepted publication; cancelled
    /// and failed requests retain the preceding state.
    pub(super) fn navigation_checkpoint(&self) -> u64 {
        self.merge_generation.get()
    }

    pub(super) fn navigation_generation(&self) -> u64 {
        self.navigation_generation.get()
    }

    fn current_navigation_affinity(&self) -> Option<NavigationAffinity> {
        self.navigation_affinity.get().filter(|navigation| {
            navigation.selection == self.selection()
                && navigation.mutation_version == self.document.mutation_version()
                && navigation.checkpoint == self.navigation_checkpoint()
        })
    }

    pub(super) fn vertical_navigation_x(&self) -> Option<f64> {
        self.current_navigation_affinity()
            .and_then(|navigation| navigation.vertical_x)
    }

    pub(super) fn line_end_navigation_start(&self) -> Option<usize> {
        self.current_navigation_affinity()
            .and_then(|navigation| navigation.line_end_start)
    }

    pub(super) fn remember_vertical_navigation_x(&self, x: f64) {
        self.remember_navigation_affinity(Some(x), None);
    }

    pub(super) fn remember_line_end_navigation(&self, start: usize) {
        self.remember_navigation_affinity(None, Some(start));
    }

    fn remember_navigation_affinity(&self, vertical_x: Option<f64>, line_end_start: Option<usize>) {
        self.navigation_generation
            .set(self.navigation_generation.get().wrapping_add(1));
        self.navigation_affinity.set(Some(NavigationAffinity {
            vertical_x,
            line_end_start,
            selection: self.selection(),
            mutation_version: self.document.mutation_version(),
            checkpoint: self.navigation_checkpoint(),
        }));
    }

    pub(super) fn clear_navigation_affinity(&self) {
        self.navigation_generation
            .set(self.navigation_generation.get().wrapping_add(1));
        self.navigation_affinity.set(None);
    }
    pub fn is_read_only(&self) -> bool {
        self.is_read_only.get()
    }
    pub fn set_is_read_only(&self, value: bool) {
        self.is_read_only.set(value);
    }
    /// Zero means unlimited. The limit is measured in UTF-16 code units.
    pub fn max_length(&self) -> usize {
        self.max_length.get()
    }
    pub fn set_max_length(&self, value: usize) {
        self.max_length.set(value);
    }
    pub fn undo_limit(&self) -> usize {
        self.undo_limit.get()
    }
    pub fn set_undo_limit(&self, value: usize) {
        self.undo_limit.set(value);
        let mut history = self.history.borrow_mut();
        let remove = history.undo.len().saturating_sub(value);
        history.undo.drain(..remove);
        // The redo stack is limited too (#243 review, N3); it keeps its most recent
        // units, the ones a redo reaches first.
        let remove = history.redo.len().saturating_sub(value);
        history.redo.drain(..remove);
    }
    pub fn can_undo(&self) -> bool {
        let history = self.history.borrow();
        self.history_is_current(&history) && !history.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        let history = self.history.borrow();
        self.history_is_current(&history) && !history.redo.is_empty()
    }
    pub fn is_composing(&self) -> bool {
        self.history.borrow().composition.is_some()
    }
    /// Absolute UTF-16 extent of the current IME preview. An explicitly started
    /// native composition has no range until its first accepted update; the
    /// selected committed text remains outside that extent until replaced.
    /// Routed lifecycle events project that pending state to the native 0/0
    /// sentinel. A later external model mutation also invalidates this extent
    /// instead of exposing stale composition text.
    pub fn active_composition_range(&self) -> Option<Range<usize>> {
        let (version, result) = {
            let history = self.history.borrow();
            let composition = history.composition.as_ref()?;
            let range = if let Some(exact) = &composition.exact {
                exact.range.clone()?
            } else {
                let start = composition.before.selection.range().start;
                start..start.checked_add(composition.preview_text.encode_utf16().count())?
            };
            (composition.preview_version, range)
        };
        if self.document.mutation_version() != version {
            return None;
        }
        if self.document.mutation_version() != version
            || self.document.validate_range(result.clone()).is_err()
        {
            return None;
        }
        Some(result)
    }
    pub fn typing_format(&self) -> CharacterFormat {
        let snapshot = self.typing_format.borrow().clone();
        (*snapshot).clone()
    }

    /// Replaces the temporary typing snapshot used by an independent range
    /// inside a grouped transaction, including exact restoration afterwards.
    /// Applying a patch would merge unrelated selection and range defaults.
    pub(super) fn replace_range_typing_format(
        &self,
        format: &CharacterFormat,
    ) -> Result<(), EditError> {
        if self.group_depth.get() == 0 || self.in_operation.get() || self.projecting_selection.get()
        {
            return Err(EditError::ReentrantEdit);
        }
        format.validate()?;
        let mutation = self.document.mutation_version();
        let checkpoint = self.navigation_checkpoint();
        let replacement = format.clone();
        if self.document.mutation_version() != mutation {
            self.observe_external_document_change();
            return Err(EditError::ReentrantEdit);
        }
        if self.navigation_checkpoint() != checkpoint {
            return Err(EditError::ReentrantEdit);
        }
        let old = self.typing_format.replace(Rc::new(replacement));
        drop(old);
        if self.document.mutation_version() != mutation {
            self.observe_external_document_change();
            return Err(EditError::ReentrantEdit);
        }
        if self.navigation_checkpoint() != checkpoint {
            return Err(EditError::ReentrantEdit);
        }
        Ok(())
    }

    /// Assigned document defaults. Unassigned fields continue to inherit the host's
    /// FerroUI text properties; callers receive an independent value snapshot.
    pub fn default_character_format(&self) -> CharacterFormat {
        let snapshot = self.default_character_format.borrow().clone();
        (*snapshot).clone()
    }

    pub fn default_paragraph_format(&self) -> ParagraphFormat {
        let snapshot = self.default_paragraph_format.borrow().clone();
        (*snapshot).clone()
    }

    /// Updates the defaults for empty content and document replacement without
    /// restyling existing text. The change participates in grouped undo/rollback.
    pub fn set_default_character_format(&self, format: &CharacterFormat) -> Result<(), EditError> {
        format.validate()?;
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let before = self.capture();
        let merged = self.default_character_format().overlay(format).clone();
        let previous = self.default_character_format.replace(Rc::new(merged));
        if self.document.text_length() == 0 {
            if let Err(error) = self
                .document
                .apply_paragraph_format(0..0, &self.insertion_defaults())
            {
                let failed = self.default_character_format.replace(previous);
                drop(failed);
                return Err(error.into());
            }
        }
        drop(previous);
        self.checkpoint();
        self.record(before, EditKind::Formatting);
        Ok(())
    }

    pub fn set_default_paragraph_format(&self, format: &ParagraphFormat) -> Result<(), EditError> {
        format.validate()?;
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let before = self.capture();
        let merged = self.default_paragraph_format().overlay(format).clone();
        let previous = self.default_paragraph_format.replace(Rc::new(merged));
        if self.document.text_length() == 0 {
            if let Err(error) = self
                .document
                .apply_paragraph_format(0..0, &self.insertion_defaults())
            {
                let failed = self.default_paragraph_format.replace(previous);
                drop(failed);
                return Err(error.into());
            }
        }
        drop(previous);
        self.checkpoint();
        self.record(before, EditKind::Formatting);
        Ok(())
    }

    pub(super) fn insertion_defaults(&self) -> ParagraphFormat {
        let mut format = self.default_paragraph_format();
        format.character = format.character.overlay(&self.default_character_format());
        format
    }

    pub(crate) fn is_in_operation(&self) -> bool {
        self.in_operation.get()
    }

    /// The control projects proposed selection through its cancellable routed event.
    /// This callback owns no control strongly and runs without an interior borrow.
    pub(crate) fn set_selection_projection(
        &self,
        projection: Option<Rc<dyn Fn(EditorSelection) -> EditorSelection>>,
    ) {
        let old = self.selection_projection.replace(projection);
        drop(old);
    }

    fn project_selection(&self) {
        if self.projecting_selection.get() || self.group_depth.get() > 0 {
            return;
        }
        self.invoke_selection_projection();
    }

    fn invoke_selection_projection(&self) {
        if self.group_depth.get() > 0 {
            return;
        }
        let projection = self.selection_projection.borrow().clone();
        let Some(projection) = projection else {
            return;
        };
        let _projection = SelectionProjection {
            flag: &self.projecting_selection,
            previous: self.projecting_selection.replace(true),
        };
        let candidate = self.selection();
        let accepted = projection(candidate);
        // The owner hook is internal, but never let a stale/reentrant result put a
        // surrogate interior or an out-of-document offset in the editor state.
        let accepted = if self.document.validate_range(accepted.range()).is_ok() {
            accepted
        } else {
            let length = self.document.text_length();
            EditorSelection::new(
                self.scalar_boundary(candidate.anchor.min(length)),
                self.scalar_boundary(candidate.active.min(length)),
            )
        };
        self.selection.set(accepted);
    }

    /// Effective formatting at the selection start or insertion point, including the
    /// current collapsed-selection format overrides.
    pub fn current_character_format(&self) -> Result<CharacterFormat, EditError> {
        let checkpoint = self.selection_format_checkpoint();
        let selection = self.selection();
        let element = if selection.is_empty() {
            self.selection_gravity()
                .caret_element(&self.document, selection.active)
        } else {
            self.document
                .element_at(selection.range().start, TextGravity::Forward)
        };
        let format = element
            .map(|element| CharacterFormat::effective(&element))
            .unwrap_or_else(|| self.insertion_defaults().character)
            .overlay(&self.typing_format());
        self.ensure_selection_format(checkpoint)?;
        Ok(format)
    }

    /// Observes completed editing or selection changes. Handlers run after document
    /// notifications and outside all history borrows.
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.changed.add(Rc::new(handler));
        let weak = Rc::downgrade(&self.changed);
        Disposable::create(move || {
            if let Some(events) = weak.upgrade() {
                events.remove(token);
            }
        })
    }

    /// Observes accepted composition causes after the editor's change notifications.
    /// A completed editor edit group delivers its queued causes in order; callbacks
    /// observe the settled group state. A failed group discards them. A separately
    /// held model edit batch may defer its public document notification further.
    pub fn composition_changed(
        &self,
        handler: impl Fn(&RichTextCompositionChange) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.composition_changed.add(Rc::new(handler));
        let weak = Rc::downgrade(&self.composition_changed);
        Disposable::create(move || {
            if let Some(events) = weak.upgrade() {
                events.remove(token);
            }
        })
    }

    fn queue_composition_change(&self, change: RichTextCompositionChange) {
        self.pending_composition_changes.borrow_mut().push(change);
    }

    fn publish_changed(&self) {
        if self.group_depth.get() > 0 || self.projecting_selection.get() {
            return;
        }
        let mutation_version = self.document.mutation_version();
        self.project_selection();
        for (_, handler) in self.changed.snapshot().iter() {
            handler();
        }
        if self.document.mutation_version() != mutation_version {
            // A notification callback replaced the state these queued causes
            // described. Preserve its edit without emitting obsolete IME events.
            self.observe_external_document_change();
            return;
        }
        let changes = std::mem::take(&mut *self.pending_composition_changes.borrow_mut());
        for change in changes {
            for (_, handler) in self.composition_changed.snapshot().iter() {
                handler(&change);
            }
            if self.document.mutation_version() != mutation_version {
                self.observe_external_document_change();
                return;
            }
        }
    }

    /// Committed text exposed to the platform while a separate preview is visible.
    pub fn surrounding_text(&self) -> String {
        let committed = self
            .history
            .borrow()
            .composition
            .as_ref()
            .filter(|composition| composition.preview_version == self.document.mutation_version())
            .map(|composition| composition.before.clone());
        committed.map_or_else(|| self.document.text(), |snapshot| snapshot.document.text())
    }

    /// Selection in the committed surrounding text, excluding the uncommitted preview.
    pub fn input_method_selection(&self) -> EditorSelection {
        self.history
            .borrow()
            .composition
            .as_ref()
            .filter(|composition| composition.preview_version == self.document.mutation_version())
            .map_or(self.selection(), |composition| composition.before.selection)
    }

    pub fn set_selection(&self, selection: EditorSelection) -> Result<(), EditError> {
        self.document.get_text(selection.range())?;
        if self.selection() == selection {
            return Ok(());
        }
        if self.in_operation.get() && !self.projecting_selection.get() {
            return Err(EditError::ReentrantEdit);
        }
        let previous = self.selection.replace(selection);
        if previous != selection {
            let previous_format = self
                .typing_format
                .replace(Rc::new(CharacterFormat::default()));
            let navigation_generation = self.navigation_generation();
            // A nested owner request must immediately update the view's proposal.
            // The view suppresses recursive SelectionChanging, while this guard
            // suppresses nested editor publication until the outer request settles.
            self.invoke_selection_projection();
            if self.selection() == previous {
                let discarded = self.typing_format.replace(previous_format);
                drop(discarded);
                return Ok(());
            }
            // A nested navigation may accept the proposed offsets without
            // changing them, yet choose its own desired column or line affinity.
            // Preserve that accepted state across this enclosing request's undo
            // checkpoint. Ordinary checkpoints still invalidate prior affinity.
            let nested_affinity = (self.navigation_generation() != navigation_generation)
                .then(|| self.current_navigation_affinity())
                .flatten();
            self.checkpoint();
            if let Some(mut affinity) = nested_affinity {
                affinity.checkpoint = self.navigation_checkpoint();
                self.navigation_affinity.set(Some(affinity));
            }
            self.publish_changed();
        }
        Ok(())
    }

    pub fn selected_text(&self) -> Result<String, EditError> {
        Ok(self.document.get_text(self.selection().range())?)
    }
    pub fn selected_fragment(&self) -> Result<DocumentSnapshot, EditError> {
        Ok(self.document.slice(self.selection().range())?)
    }

    /// Separates the next typed characters from the preceding undo unit.
    pub fn checkpoint(&self) {
        self.merge_generation
            .set(self.merge_generation.get().wrapping_add(1));
    }
    pub fn clear_history(&self) {
        self.history_clear_generation
            .set(self.history_clear_generation.get().wrapping_add(1));
        let old = std::mem::take(&mut *self.history.borrow_mut());
        drop(old);
        self.known_revision.set(self.document.revision());
        self.checkpoint();
    }

    pub(super) fn external_change_generation(&self) -> u64 {
        self.external_change_generation.get()
    }

    /// Called only after a read/layout callback is proved to have mutated the
    /// model. An enclosing transaction then cannot safely restore its snapshot.
    pub(super) fn observe_external_document_change(&self) {
        self.external_change_generation
            .set(self.external_change_generation.get().wrapping_add(1));
        self.discard_external_edit_state();
    }

    fn discard_external_edit_state(&self) {
        self.clear_history();
        let old_format = self
            .typing_format
            .replace(Rc::new(CharacterFormat::default()));
        drop(old_format);
        self.pending_composition_changes.borrow_mut().clear();
        self.clamp_selection();
    }

    fn enter(&self, require_writable: bool) -> Result<Operation<'_>, EditError> {
        if require_writable && self.is_read_only() {
            return Err(EditError::ReadOnly);
        }
        if self.projecting_selection.get() || self.in_operation.replace(true) {
            return Err(EditError::ReentrantEdit);
        }
        Ok(Operation(&self.in_operation))
    }

    /// Discards history for completed model changes before a facade captures
    /// callback checkpoints. Doing this only after entering its edit group
    /// would make ordinary synchronization invalidate the facade's own read.
    pub(super) fn synchronize(&self) {
        let current = self.history_is_current(&self.history.borrow());
        if current {
            // Closing an application-owned model batch publishes a revision without
            // changing the accepted preview. Its mutation identity remains valid.
            self.known_revision.set(self.document.revision());
        } else {
            self.clear_history();
            self.clamp_selection();
        }
    }

    fn history_is_current(&self, history: &History) -> bool {
        history.composition.as_ref().map_or_else(
            || self.document.revision() == self.known_revision.get(),
            |composition| composition.preview_version == self.document.mutation_version(),
        )
    }

    fn clamp_selection(&self) {
        let length = self.document.text_length();
        let selection = self.selection();
        self.selection.set(EditorSelection::new(
            self.scalar_boundary(selection.anchor.min(length)),
            self.scalar_boundary(selection.active.min(length)),
        ));
    }

    fn scalar_boundary(&self, offset: usize) -> usize {
        if self.document.get_text(offset..offset).is_ok() {
            offset
        } else {
            offset.saturating_sub(1)
        }
    }

    fn capture(&self) -> EditorSnapshot {
        EditorSnapshot {
            document: self.document.snapshot(),
            selection: self.selection(),
            typing_format: self.typing_format(),
            default_character_format: self.default_character_format(),
            default_paragraph_format: self.default_paragraph_format(),
        }
    }

    fn restore_document_snapshot(&self, snapshot: &DocumentSnapshot) -> Result<(), EditError> {
        let version = self.document.mutation_version();
        let history_generation = self.history_clear_generation.get();
        let result = self.document.restore(snapshot);
        if result.is_err() && self.document.mutation_version() != version {
            // The model rejected a stale preflight after an application write.
            // Its new story owns the failure; callers must not roll it back.
            self.observe_external_document_change();
            return Err(EditError::ReentrantEdit);
        }
        if self.history_clear_generation.get() != history_generation {
            // A model-preparation brush may clear history without editing text.
            // Do not reinstall a removed composition or the popped undo unit.
            self.clamp_selection();
            self.known_revision.set(self.document.revision());
            return Err(EditError::ReentrantEdit);
        }
        result.map_err(Into::into)
    }

    fn restore(&self, snapshot: &EditorSnapshot) -> Result<(), EditError> {
        let version = self.document.mutation_version();
        let history_generation = self.history_clear_generation.get();
        let typing = Rc::new(snapshot.typing_format.clone());
        let character = Rc::new(snapshot.default_character_format.clone());
        let paragraph = Rc::new(snapshot.default_paragraph_format.clone());
        if self.document.mutation_version() != version {
            self.observe_external_document_change();
            return Err(EditError::ReentrantEdit);
        }
        if self.history_clear_generation.get() != history_generation {
            return Err(EditError::ReentrantEdit);
        }
        // These are explicit historical formats, not a new affinity query.
        // A newer Gravity value survives restoration, including group rollback.
        self.restore_document_snapshot(&snapshot.document)?;
        self.selection.set(snapshot.selection);
        let old_typing = self.typing_format.replace(typing);
        let old_character = self.default_character_format.replace(character);
        let old_paragraph = self.default_paragraph_format.replace(paragraph);
        drop((old_typing, old_character, old_paragraph));
        self.known_revision.set(self.document.revision());
        Ok(())
    }

    fn record(&self, before: EditorSnapshot, kind: EditKind) {
        self.known_revision.set(self.document.revision());
        if self.group_depth.get() > 0 {
            return;
        }
        let mutation_version = self.document.mutation_version();
        let history_generation = self.history_clear_generation.get();
        self.project_selection();
        if self.document.mutation_version() != mutation_version {
            self.observe_external_document_change();
            self.publish_changed();
            return;
        }
        if self.history_clear_generation.get() != history_generation {
            return;
        }
        // A group may finish with a visible preview. Only its preceding committed
        // edits belong in the undo unit; committing or cancelling the preview later
        // must not resurrect uncommitted IME text when redo restores this group.
        let committed = self
            .history
            .borrow()
            .composition
            .as_ref()
            .map(|composition| composition.before.clone());
        let after = committed.map_or_else(|| self.capture(), |snapshot| (*snapshot).clone());
        let unchanged = before.document == after.document
            && before.typing_format == after.typing_format
            && before.default_character_format == after.default_character_format
            && before.default_paragraph_format == after.default_paragraph_format;
        if self.document.mutation_version() != mutation_version {
            self.observe_external_document_change();
            self.publish_changed();
            return;
        }
        if self.history_clear_generation.get() != history_generation {
            return;
        }
        if unchanged {
            self.publish_changed();
            return;
        }
        let mut history = self.history.borrow_mut();
        history.redo.clear();
        if self.undo_limit.get() == 0 {
            drop(history);
            self.publish_changed();
            return;
        }
        let generation = self.merge_generation.get();
        // RichEdit groups typing "including characters deleted by using the
        // BackSpace key": a Backspace at a collapsed caret is recorded as typing.
        let kind = if kind == EditKind::Delete && self.recording_backspace.get() && before.selection.is_empty() {
            EditKind::Typing
        } else {
            kind
        };
        if kind == EditKind::Typing && before.selection.is_empty() {
            if let Some(last) = history.undo.last_mut().filter(|unit| {
                unit.kind == kind
                    && unit.merge_generation == generation
                    && unit.after.selection == before.selection
                    && unit.after.typing_format == before.typing_format
            }) {
                last.after = Rc::new(after);
                drop(history);
                self.publish_changed();
                return;
            }
        }
        // Share the snapshot with the unit before when it is the same state.
        let before = match history.undo.last() {
            Some(last) if *last.after == before => last.after.clone(),
            _ => Rc::new(before),
        };
        history.undo.push(UndoUnit {
            before,
            after: Rc::new(after),
            kind,
            merge_generation: generation,
        });
        let remove = history.undo.len().saturating_sub(self.undo_limit.get());
        history.undo.drain(..remove);
        drop(history);
        self.publish_changed();
    }

    fn permitted_text(&self, text: &str, replacing: Range<usize>) -> String {
        let normalized = normalize_text(text);
        if self.max_length() == 0 {
            return normalized;
        }
        let retained = self
            .document
            .text_length()
            .saturating_sub(replacing.end - replacing.start);
        let mut remaining = self.max_length().saturating_sub(retained);
        normalized
            .chars()
            .take_while(|ch| {
                let length = ch.len_utf16();
                if length > remaining {
                    false
                } else {
                    remaining -= length;
                    true
                }
            })
            .collect()
    }

    /// Replaces the selection and applies the current insertion format in one undo unit.
    /// Typing uses the selected-paragraph endpoint/alignment contract; bulk and
    /// composition edits retain their independent replacement policies.
    pub fn insert_text(&self, text: &str, kind: EditKind) -> Result<bool, EditError> {
        self.insert_text_core(text, kind, false)
    }

    /// Only the explicit independent range SetText facade supplies this origin.
    /// Generic programmatic edits and typed input keep their own tracking policy.
    pub(in crate::rich_text) fn insert_range_text(&self, text: &str) -> Result<bool, EditError> {
        self.insert_text_core(text, EditKind::Programmatic, true)
    }

    fn insert_text_core(
        &self,
        text: &str,
        kind: EditKind,
        paired_tracking: bool,
    ) -> Result<bool, EditError> {
        if self.is_composing() && matches!(kind, EditKind::Typing | EditKind::Composition) {
            return self.commit_composition(text);
        }
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let range = self.selection().range();
        let range = if kind == EditKind::Typing {
            self.prepare_typing_range(range)?
        } else {
            range
        };
        if kind == EditKind::Typing
            && self
                .selection_options()
                .contains(SelectionOptions::OVERTYPE)
        {
            let (range, text) = self.prepare_overtype_text(text, range)?;
            return self.replace_text_core(&text, kind, range, false);
        }
        self.replace_text_core(text, kind, range, paired_tracking)
    }

    fn replace_text_core(
        &self,
        text: &str,
        kind: EditKind,
        range: Range<usize>,
        paired_tracking: bool,
    ) -> Result<bool, EditError> {
        let Some((before, text)) =
            self.replace_text_unrecorded(text, kind, range, paired_tracking)?
        else {
            return Ok(false);
        };
        if text.contains('\n') {
            self.checkpoint();
        }
        self.record(before, kind);
        Ok(true)
    }

    fn replace_text_unrecorded(
        &self,
        text: &str,
        kind: EditKind,
        range: Range<usize>,
        paired_tracking: bool,
    ) -> Result<Option<(EditorSnapshot, String)>, EditError> {
        let checkpoint = self.selection_format_checkpoint();
        let limit = self.max_length();
        let text = self.permitted_text(text, range.clone());
        // Rejected input at a length limit must not delete a selected range.
        if text.is_empty() && (!text_is_deletion(kind) || range.is_empty()) {
            return Ok(None);
        }
        let before = self.capture();
        self.ensure_selection_format(checkpoint)?;
        let length = text.encode_utf16().count();
        let format = if length > 0 {
            self.insertion_character_format(&range)?
        } else {
            CharacterFormat::default()
        };
        self.ensure_selection_format(checkpoint)?;
        if self.max_length() != limit || self.is_read_only() {
            return Err(EditError::ReentrantEdit);
        }
        {
            let _batch = self.document.begin_edit();
            // Removing a paragraph mark keeps the trailing paragraph's format whether
            // it is typed over or deleted with Backspace, Delete or Cut (#243 review,
            // S6), as the native typing observations show.
            if kind == EditKind::Typing || (kind == EditKind::Delete && text.is_empty()) {
                self.document.replace_typed_text(range.clone(), &text)?;
            } else if paired_tracking {
                match self.document.replace_range_text(range.clone(), &text) {
                    Err(DocumentError::EditInProgress) => {
                        // The model proved that a callback accepted a newer
                        // write during preflight or a virtual observation read.
                        // Retire our transaction before a failed group can
                        // restore an older image over that callback's content.
                        self.observe_external_document_change();
                        return Err(EditError::ReentrantEdit);
                    }
                    result => result?,
                }
            } else {
                self.document.replace_text(range.clone(), &text)?;
            }
            if before.document.text_length() == 0 || self.document.text_length() == 0 {
                self.document.apply_paragraph_format(
                    0..self.document.text_length(),
                    &self.insertion_defaults(),
                )?;
            }
            if length > 0 {
                self.document
                    .apply_format(range.start..range.start + length, &format)?;
            }
            if kind == EditKind::Typing && length > 0 {
                // Clear accepted typing intent before publication. A later
                // application projection can install its own newer mode state.
                self.clear_typing_end_of_line();
            }
            self.selection
                .set(EditorSelection::caret(range.start + length));
        }
        Ok(Some((before, text)))
    }

    pub(super) fn prepare_edit(&self) -> Result<bool, EditError> {
        if self.is_read_only() {
            return Err(EditError::ReadOnly);
        }
        if self.in_operation.get() || self.projecting_selection.get() {
            return Err(EditError::ReentrantEdit);
        }
        let composing = self.is_composing();
        if composing {
            self.cancel_composition()?;
            self.checkpoint();
        }
        Ok(composing)
    }

    /// Replaces the selection with a structured fragment, retaining all supported inline kinds.
    pub fn insert_fragment(
        &self,
        fragment: &DocumentSnapshot,
        kind: EditKind,
    ) -> Result<bool, EditError> {
        self.insert_fragment_core(fragment, kind, false)
    }

    pub(super) fn insert_fragment_preserving_link_fields(
        &self,
        fragment: &DocumentSnapshot,
        kind: EditKind,
    ) -> Result<bool, EditError> {
        self.insert_fragment_core(fragment, kind, true)
    }

    fn insert_fragment_core(
        &self,
        fragment: &DocumentSnapshot,
        kind: EditKind,
        preserve_fields: bool,
    ) -> Result<bool, EditError> {
        fragment.validate()?;
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let range = self.selection().range();
        let fragment = if self.max_length() > 0 {
            let retained = self
                .document
                .text_length()
                .saturating_sub(range.end - range.start);
            let limit = self.max_length().saturating_sub(retained);
            if limit < fragment.text_length() {
                let temp = RichTextDocument::new();
                temp.restore(fragment)?;
                let mut end = limit;
                if temp.get_text(end..end).is_err() {
                    end = end.saturating_sub(1);
                }
                temp.slice(0..end)?
            } else {
                fragment.clone()
            }
        } else {
            fragment.clone()
        };
        if fragment.text_length() == 0 {
            return Ok(false);
        }
        let before = self.capture();
        {
            let _batch = self.document.begin_edit();
            if preserve_fields {
                self.document
                    .replace_range_preserving_link_fields(range.clone(), &fragment)?;
            } else {
                self.document.replace_range(range.clone(), &fragment)?;
            }
            self.selection
                .set(EditorSelection::caret(range.start + fragment.text_length()));
        }
        self.checkpoint();
        self.record(before, kind);
        Ok(true)
    }

    pub fn delete_selection(&self) -> Result<bool, EditError> {
        self.prepare_edit()?;
        if self.selection().is_empty() {
            return Ok(false);
        }
        self.insert_text("", EditKind::Delete)
    }

    pub fn delete_backward(&self, whole_word: bool) -> Result<bool, EditError> {
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let range = if self.selection().is_empty() {
            let position = self.selection().active;
            let next = if whole_word {
                self.document.previous_word(position)
            } else {
                self.document.previous_grapheme(position)
            };
            next..position
        } else {
            self.selection().range()
        };
        let backspace = self.selection().is_empty() && !whole_word;
        let previous = self.recording_backspace.replace(backspace);
        let result = self.replace_text_core("", EditKind::Delete, range, false);
        self.recording_backspace.set(previous);
        result
    }

    pub fn delete_forward(&self, whole_word: bool) -> Result<bool, EditError> {
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let range = if self.selection().is_empty() {
            let position = self.selection().active;
            let next = if whole_word {
                self.document.next_word(position)
            } else {
                self.document.next_grapheme(position)
            };
            position..next
        } else {
            self.selection().range()
        };
        self.replace_text_core("", EditKind::Delete, range, false)
    }

    pub fn apply_format(&self, format: &CharacterFormat) -> Result<(), EditError> {
        format.validate()?;
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let before = self.capture();
        if self.selection().is_empty() {
            // Validate the format through the same model path as a nonempty selection.
            self.document
                .apply_format(self.selection().range(), format)?;
            let merged = merge_format(self.typing_format(), format).clone();
            let previous = self.typing_format.replace(Rc::new(merged));
            drop(previous);
        } else {
            self.document
                .apply_format(self.selection().range(), format)?;
        }
        self.checkpoint();
        self.record(before, EditKind::Formatting);
        Ok(())
    }

    /// Installs a prepared field image inside an existing edit group. The
    /// projected service never substitutes its range for the real selection.
    pub(super) fn install_field_snapshot(
        &self,
        snapshot: &DocumentSnapshot,
        expected_version: u64,
    ) -> Result<(), EditError> {
        let _operation = self.enter(true)?;
        snapshot.validate()?;
        if self.document.mutation_version() != expected_version {
            self.observe_external_document_change();
            return Err(EditError::ReentrantEdit);
        }
        let result = self.document.restore(snapshot);
        if result.is_err() && self.document.mutation_version() != expected_version {
            // The model rejected its callback-interrupted preparation. Mark the
            // independent write so the enclosing edit group cannot roll it back.
            self.observe_external_document_change();
            return Err(EditError::ReentrantEdit);
        }
        result?;
        Ok(())
    }

    pub fn apply_paragraph_format(&self, format: &ParagraphFormat) -> Result<(), EditError> {
        format.validate()?;
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let before = self.capture();
        self.document
            .apply_paragraph_format(self.selection().range(), format)?;
        self.checkpoint();
        self.record(before, EditKind::Formatting);
        Ok(())
    }

    /// Executes multiple edits as one undo unit. An error rolls the group back
    /// atomically. If a layout callback independently mutates the model, that
    /// external state survives and the interrupted group discards its history.
    pub fn edit_group<T>(
        &self,
        action: impl FnOnce(&Self) -> Result<T, EditError>,
    ) -> Result<T, EditError> {
        self.prepare_edit()?;
        if self.is_read_only() {
            return Err(EditError::ReadOnly);
        }
        if self.in_operation.get() {
            return Err(EditError::ReentrantEdit);
        }
        self.synchronize();
        let before = self.capture();
        let range_tracking = self.document.capture_range_tracking();
        let before_limit = self.max_length();
        let before_options = self.selection_options.get();
        let before_gravity = self.selection_gravity();
        let external_generation = self.external_change_generation();
        let pending_start = self.pending_composition_changes.borrow().len();
        let depth = self.group_depth.get();
        self.group_depth.set(depth + 1);
        let batch = self.document.begin_edit();
        let result = action(self);
        let interrupted = self.external_change_generation() != external_generation;
        // Nested code may catch its immediate layout error. Every enclosing
        // transaction still rejects the stale before/after history pair.
        let mut result = if interrupted {
            Err(EditError::ReentrantEdit)
        } else {
            result
        };
        let _operation = self.enter(false)?;
        if interrupted {
            self.discard_external_edit_state();
        } else if result.is_err() {
            // TOM SetText may grow MaxLength inside a nested group. Failed
            // transactions restore that configuration along with the story.
            self.set_max_length(before_limit);
            self.store_selection_options(before_options);
            self.set_selection_gravity(before_gravity);
            let composition = self.history.borrow_mut().composition.take();
            drop(composition);
            self.pending_composition_changes
                .borrow_mut()
                .truncate(pending_start);
            match self.restore(&before) {
                Ok(()) if self.external_change_generation() == external_generation => {
                    self.document.restore_range_tracking(range_tracking);
                }
                Ok(()) => {}
                Err(error) => result = Err(error),
            }
        }
        let publication_version = self.document.mutation_version();
        drop(batch);
        if self.document.mutation_version() != publication_version {
            // Application document-change handlers run when the outer batch
            // closes. Their direct model writes are independent of this group,
            // even though editor commands remain guarded during publication.
            self.observe_external_document_change();
            result = Err(EditError::ReentrantEdit);
        }
        self.group_depth.set(depth);
        self.known_revision.set(self.document.revision());
        let notification_version = self.document.mutation_version();
        if result.is_ok() {
            self.checkpoint();
            self.record(before, EditKind::Group);
        } else {
            self.publish_changed();
        }
        if self.document.mutation_version() != notification_version {
            self.observe_external_document_change();
            result = Err(EditError::ReentrantEdit);
        }
        result
    }

    pub fn undo(&self) -> Result<bool, EditError> {
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let unit = self.history.borrow_mut().undo.pop();
        let Some(unit) = unit else {
            return Ok(false);
        };
        let history_generation = self.history_clear_generation.get();
        if let Err(error) = self.restore(&unit.before) {
            if self.history_clear_generation.get() == history_generation {
                self.history.borrow_mut().undo.push(unit);
            }
            return Err(error);
        }
        self.history.borrow_mut().redo.push(unit);
        self.checkpoint();
        self.publish_changed();
        Ok(true)
    }

    pub fn redo(&self) -> Result<bool, EditError> {
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let unit = self.history.borrow_mut().redo.pop();
        let Some(unit) = unit else {
            return Ok(false);
        };
        let history_generation = self.history_clear_generation.get();
        if let Err(error) = self.restore(&unit.after) {
            if self.history_clear_generation.get() == history_generation {
                self.history.borrow_mut().redo.push(unit);
            }
            return Err(error);
        }
        self.history.borrow_mut().undo.push(unit);
        self.checkpoint();
        self.publish_changed();
        Ok(true)
    }

    /// Updates visible IME text without adding an undo unit. The first update captures the
    /// entire pre-composition selection and structure; later updates replace that preview.
    pub fn set_preedit(&self, text: &str, cursor: Option<usize>) -> Result<(), EditError> {
        let _operation = self.enter(true)?;
        let cursor = cursor.map(|offset| {
            normalize_text(utf16_prefix(text, offset))
                .encode_utf16()
                .count()
        });
        self.synchronize();
        let read_checkpoint = self.selection_format_checkpoint();
        let external_generation = self.external_change_generation();
        let history_generation = self.history_clear_generation.get();
        let previous = self.history.borrow().composition.clone();
        let visible = self.capture();
        let before = previous.as_ref().map_or_else(
            || Rc::new(visible.clone()),
            |composition| composition.before.clone(),
        );
        self.ensure_selection_format(read_checkpoint)?;
        // Keep the committed context available before publishing the model change:
        // platform callbacks can synchronously query surrounding text while it is raised.
        self.history.borrow_mut().composition = Some(Composition {
            before: before.clone(),
            preview_version: self.document.mutation_version(),
            preview_text: String::new(),
            exact: None,
        });
        let batch = self.document.begin_edit();
        let result = (|| -> Result<String, EditError> {
            self.restore(&before)?;
            let range = before.selection.range();
            let text = self.permitted_text(text, range.clone());
            let length = text.encode_utf16().count();
            let insertion_format = if length > 0 {
                self.insertion_character_format(&range)?
            } else {
                CharacterFormat::default()
            };
            self.document.replace_text(range.clone(), &text)?;
            if before.document.text_length() == 0 || self.document.text_length() == 0 {
                self.document.apply_paragraph_format(
                    0..self.document.text_length(),
                    &self.insertion_defaults(),
                )?;
            }
            if length > 0 {
                let preedit_format = insertion_format.overlay(&CharacterFormat {
                    text_decorations: Some(Some(TextDecorations::underline())),
                    ..CharacterFormat::default()
                });
                self.document
                    .apply_format(range.start..range.start + length, &preedit_format)?;
            }
            let offset = range.start + cursor.unwrap_or(length).min(length);
            self.selection
                .set(EditorSelection::caret(self.scalar_boundary(offset)));
            Ok(text)
        })();
        let text = match result {
            Ok(text) => text,
            Err(error) => {
                // Metadata-only reentry retains the original preview. A direct
                // application model write detected during a format read owns its
                // new story and must not be overwritten by the old snapshot.
                if self.external_change_generation() == external_generation
                    && self.history_clear_generation.get() == history_generation
                {
                    self.restore_composition_state(&visible, previous)?;
                } else if self.external_change_generation() != external_generation {
                    self.discard_external_edit_state();
                }
                drop(batch);
                self.known_revision.set(self.document.revision());
                return Err(error);
            }
        };
        let old_text = previous
            .as_ref()
            .map_or("", |composition| composition.preview_text.as_str());
        let changed_text = changed_composition_text(old_text, &text);
        // Install the final state before model callbacks. Public revisions may be
        // delayed by a caller's outer batch; the actual mutation identity is current.
        self.history.borrow_mut().composition = Some(Composition {
            before,
            preview_version: self.document.mutation_version(),
            preview_text: text,
            exact: None,
        });
        if let Some(changed_text) = changed_text {
            self.queue_composition_change(RichTextCompositionChange::Preview { changed_text });
        }
        drop(batch);
        self.known_revision.set(self.document.revision());
        self.publish_changed();
        Ok(())
    }

    /// Cancels only our preview. An intervening external edit is preserved and invalidates history.
    pub fn cancel_composition(&self) -> Result<bool, EditError> {
        let _operation = self.enter(false)?;
        let composition = self.history.borrow_mut().composition.take();
        let Some(composition) = composition else {
            return Ok(false);
        };
        let batch = self.document.begin_edit();
        if self.document.mutation_version() == composition.preview_version {
            let history_generation = self.history_clear_generation.get();
            if let Err(error) = self.restore(&composition.before) {
                if self.history_clear_generation.get() == history_generation {
                    self.history.borrow_mut().composition = Some(composition);
                }
                return Err(error);
            }
        } else {
            self.clear_history();
            self.clamp_selection();
        }
        let version = self.document.mutation_version();
        drop(batch);
        if self.document.mutation_version() != version {
            self.clear_history();
            self.clamp_selection();
            return Err(EditError::ReentrantEdit);
        }
        self.known_revision.set(self.document.revision());
        self.queue_composition_change(RichTextCompositionChange::Finalized {
            text: String::new(),
        });
        self.publish_changed();
        if self.document.mutation_version() != version {
            self.clear_history();
            self.clamp_selection();
            return Err(EditError::ReentrantEdit);
        }
        Ok(true)
    }

    /// Ends a preview and inserts the accepted committed text as one transaction.
    /// Empty or fully rejected input ends the preview without replacing the original
    /// committed selection. Failed write or entry checks preserve the preview;
    /// an external edit during publication is retained and rejects the older commit.
    pub fn commit_composition(&self, text: &str) -> Result<bool, EditError> {
        self.commit_composition_result(text)
            .map(|(changed, _)| changed)
    }

    pub(crate) fn commit_composition_result(
        &self,
        text: &str,
    ) -> Result<(bool, Option<CompositionCommitResult>), EditError> {
        let _operation = self.enter(true)?;
        self.synchronize();
        let composition = self.history.borrow().composition.clone();
        let Some(composition) = composition else {
            self.checkpoint();
            return self
                .replace_text_core(text, EditKind::Composition, self.selection().range(), false)
                .map(|changed| (changed, None));
        };
        let start = composition.before.selection.range().start;
        let awaiting_text = composition
            .exact
            .as_ref()
            .is_some_and(|exact| exact.range.is_none());
        let read_checkpoint = self.selection_format_checkpoint();
        let external_generation = self.external_change_generation();
        let history_generation = self.history_clear_generation.get();
        let visible = self.capture();
        self.ensure_selection_format(read_checkpoint)?;
        let previous = self.history.borrow_mut().composition.take();
        drop(previous);
        let batch = self.document.begin_edit();
        let result = (|| {
            self.restore(&composition.before)?;
            self.replace_text_unrecorded(
                text,
                EditKind::Composition,
                composition.before.selection.range(),
                false,
            )
        })();
        let accepted = match result {
            Ok(accepted) => accepted,
            Err(error) => {
                if self.external_change_generation() == external_generation
                    && self.history_clear_generation.get() == history_generation
                {
                    self.restore_composition_state(&visible, Some(composition))?;
                } else if self.external_change_generation() != external_generation {
                    self.discard_external_edit_state();
                }
                drop(batch);
                self.known_revision.set(self.document.revision());
                return Err(error);
            }
        };
        let text = accepted
            .as_ref()
            .map_or_else(String::new, |(_, text)| text.clone());
        let range = if awaiting_text && text.is_empty() {
            0..0
        } else {
            start..start + text.encode_utf16().count()
        };
        let mutation_version = self.document.mutation_version();
        drop(batch);
        if self.document.mutation_version() != mutation_version {
            self.synchronize();
            return Err(EditError::ReentrantEdit);
        }
        self.known_revision.set(self.document.revision());
        self.checkpoint();
        self.queue_composition_change(RichTextCompositionChange::Finalized { text: text.clone() });
        let changed = if let Some((before, _)) = accepted {
            self.record(before, EditKind::Composition);
            true
        } else {
            self.publish_changed();
            false
        };
        if self.document.mutation_version() != mutation_version || self.is_composing() {
            self.synchronize();
            return Err(EditError::ReentrantEdit);
        }
        Ok((
            changed,
            Some(CompositionCommitResult {
                text,
                range,
                selection: self.selection(),
                mutation_version,
            }),
        ))
    }

    fn restore_composition_state(
        &self,
        visible: &EditorSnapshot,
        composition: Option<Composition>,
    ) -> Result<(), EditError> {
        self.restore(visible)?;
        self.history.borrow_mut().composition = composition.map(|mut composition| {
            composition.preview_version = self.document.mutation_version();
            composition
        });
        Ok(())
    }
}

/// The smallest scalar-aligned replacement in the new preview. Empty replacement
/// text is meaningful for a deletion, while None denotes a cursor-only update.
fn changed_composition_text(previous: &str, current: &str) -> Option<String> {
    if previous == current {
        return None;
    }
    let prefix = previous
        .chars()
        .zip(current.chars())
        .take_while(|(before, after)| before == after)
        .map(|(_, ch)| ch.len_utf8())
        .sum::<usize>();
    let suffix = previous[prefix..]
        .chars()
        .rev()
        .zip(current[prefix..].chars().rev())
        .take_while(|(before, after)| before == after)
        .map(|(_, ch)| ch.len_utf8())
        .sum::<usize>();
    Some(current[prefix..current.len() - suffix].to_owned())
}

/// Gets the prefix at or before a UTF-16 position without splitting a scalar.
pub(crate) fn utf16_prefix(text: &str, mut length: usize) -> &str {
    for (index, ch) in text.char_indices() {
        if ch.len_utf16() > length {
            return &text[..index];
        }
        length -= ch.len_utf16();
    }
    text
}

fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn text_is_deletion(kind: EditKind) -> bool {
    matches!(kind, EditKind::Delete | EditKind::Programmatic)
}

fn merge_format(current: CharacterFormat, update: &CharacterFormat) -> CharacterFormat {
    current.overlay(update)
}

#[cfg(test)]
#[path = "rich_text_composition_tests.rs"]
mod composition_tests;

#[path = "rich_text_exact_edit.rs"]
mod exact_edit;

#[path = "rich_text_editor_selection.rs"]
mod selection;

#[path = "rich_text_editor_gravity.rs"]
mod gravity;

#[cfg(test)]
#[path = "rich_text_editor_paragraph_typing_tests.rs"]
mod paragraph_typing_tests;

#[cfg(test)]
#[path = "rich_text_undo_tests.rs"]
mod undo_tests;
