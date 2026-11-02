use super::document_snapshot::{normalize_paragraph_breaks, slice_inlines, utf16_to_byte};
use super::{
    Block, BlockCollection, Bold, CharacterFormat, DocumentLinkField, DocumentSnapshot, Hyperlink,
    IInlineHost, Inline, InlineCollection, InlineSnapshot, InlineSnapshotKind, InlineUIContainer,
    Italic, LineBreak, Paragraph, ParagraphFormat, ParagraphSnapshot, Run, Span, TextElement,
    TextGravity, TextPointer, TextPosition, TextRange, Underline,
};
use crate::Control;
use ferroui_base::media::text_formatting::unicode::{GraphemeEnumerator, WordBreakEnumerator};
use ferroui_base::reactive::{Disposable, IDisposable, Subscription};
use ferroui_base::utilities::{HandlerList, HandlerSnapshot};
use ferroui_base::{Ref, StyledElement};
use std::cell::{Cell, RefCell};
use std::collections::{HashSet, VecDeque};
use std::ops::Range;
use std::rc::{Rc, Weak};

/// Validation or reentrancy failure. A rejected operation does not install its
/// proposed content. Independently accepted callback edits retain their content
/// and revision when an older operation rejects its stale preparation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DocumentError {
    OffsetOutOfRange { offset: usize, length: usize },
    ReversedRange { start: usize, end: usize },
    InvalidUtf16Boundary { offset: usize },
    InvalidFormat(&'static str),
    InvalidLinkField(&'static str),
    ContentTooDeep,
    DuplicateEmbeddedControl,
    EmbeddedControlAlreadyParented,
    DocumentDropped,
    EditInProgress,
}

impl std::fmt::Display for DocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OffsetOutOfRange { offset, length } => {
                write!(f, "Text offset {offset} exceeds document length {length}")
            }
            Self::ReversedRange { start, end } => {
                write!(f, "Range start {start} exceeds end {end}")
            }
            Self::InvalidUtf16Boundary { offset } => {
                write!(f, "Text offset {offset} splits a UTF-16 surrogate pair")
            }
            Self::InvalidFormat(message) | Self::InvalidLinkField(message) => f.write_str(message),
            Self::ContentTooDeep => f.write_str("Inline nesting exceeds 256 levels"),
            Self::DuplicateEmbeddedControl => {
                f.write_str("An embedded control cannot occur twice in one document")
            }
            Self::EmbeddedControlAlreadyParented => {
                f.write_str("An embedded control already belongs to another content tree")
            }
            Self::DocumentDropped => f.write_str("The text document no longer exists"),
            Self::EditInProgress => {
                f.write_str("A structured document edit is already in progress")
            }
        }
    }
}
impl std::error::Error for DocumentError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentChangeKind {
    Content,
    Formatting,
    Structure,
}

/// A replacement in the coordinates immediately before that replacement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentTextChange {
    pub range: Range<usize>,
    pub inserted_length: usize,
}

impl DocumentTextChange {
    pub fn map_position(&self, position: TextPosition) -> TextPosition {
        let offset = if position.offset < self.range.start {
            position.offset
        } else if position.offset > self.range.end {
            position
                .offset
                .saturating_sub(self.range.len())
                .saturating_add(self.inserted_length)
        } else if position.offset == self.range.end && self.range.start != self.range.end {
            self.range.start + self.inserted_length
        } else {
            self.range.start
                + if position.gravity == TextGravity::Forward {
                    self.inserted_length
                } else {
                    0
                }
        };
        TextPosition { offset, ..position }
    }
}

/// One committed, coalesced document update. `range` addresses the old version;
/// the replacement's length is measured in UTF-16 code units.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentChangedEventArgs {
    pub revision: u64,
    pub range: Range<usize>,
    pub removed_length: usize,
    pub inserted_length: usize,
    pub old_length: usize,
    pub new_length: usize,
    pub kind: DocumentChangeKind,
    /// Sequential replacements inside an edit batch, retaining interior anchors.
    pub changes: Vec<DocumentTextChange>,
}

impl DocumentChangedEventArgs {
    pub fn map_position(&self, position: TextPosition) -> TextPosition {
        let mapped = self
            .changes
            .iter()
            .fold(position, |position, change| change.map_position(position));
        TextPosition {
            offset: mapped.offset.min(self.new_length),
            ..mapped
        }
    }
}

/// Authoritative structured content shared by read-only and editable text controls.
/// Its plain string is a derived read cache; every mutation changes the block tree.
#[derive(Clone)]
pub struct RichTextDocument(Rc<RichTextDocumentData>);

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParagraphAlignmentSource {
    Leading,
    Trailing,
}

type RangePositionHandler = dyn Fn(&DocumentTextChange, Option<&DocumentTextChange>);
type RangePositionHandlers = HandlerSnapshot<RangePositionHandler>;

struct RangeTrackingReference {
    position: Weak<Cell<TextRange>>,
    paired_generation: Weak<Cell<u64>>,
}

pub(crate) struct RangeTrackingCheckpoint {
    positions: Vec<(RangeTrackingReference, TextRange, u64)>,
}

/// A one-shot description of an explicit range SetText mutation. It is installed
/// only at the final Run splice or completed structural install, after reads
/// which can call application services. Matching the complete before/after text
/// prevents a callback-owned mutation from inheriting the enclosing origin.
struct PendingRangeTextChange {
    generation: u64,
    expected_mutation: u64,
    before: Rc<str>,
    byte_range: Range<usize>,
    inserted: String,
    change: DocumentTextChange,
    handlers: RangePositionHandlers,
    interrupted: Rc<Cell<bool>>,
}

impl PendingRangeTextChange {
    fn matches(&self, before: &Rc<str>, after: &str, mutation: u64) -> bool {
        if self.expected_mutation != mutation || !Rc::ptr_eq(&self.before, before) {
            return false;
        }
        let Some(inserted_end) = self.byte_range.start.checked_add(self.inserted.len()) else {
            return false;
        };
        after.get(..self.byte_range.start) == before.get(..self.byte_range.start)
            && after.get(self.byte_range.start..inserted_end) == Some(self.inserted.as_str())
            && after.get(inserted_end..) == before.get(self.byte_range.end..)
    }
}

struct RangeTextChangeScope {
    document: RichTextDocument,
    generation: u64,
    before_mutation: u64,
    interrupted: Rc<Cell<bool>>,
}

impl RangeTextChangeScope {
    fn was_interrupted(&self) -> bool {
        let current = self.document.mutation_version();
        self.interrupted.get()
            || (current != self.before_mutation && current != self.before_mutation.wrapping_add(1))
    }
}

impl Drop for RangeTextChangeScope {
    fn drop(&mut self) {
        let mut pending = self.document.0.pending_range_change.borrow_mut();
        if pending
            .as_ref()
            .is_some_and(|change| change.generation == self.generation)
        {
            *pending = None;
        }
    }
}

struct RichTextDocumentData {
    blocks: BlockCollection,
    revision: Cell<u64>,
    mutation_version: Cell<u64>,
    changed: Rc<HandlerList<dyn Fn(&DocumentChangedEventArgs)>>,
    position_changed: Rc<HandlerList<dyn Fn(&DocumentTextChange)>>,
    range_position_changed: Rc<HandlerList<RangePositionHandler>>,
    range_positions: RefCell<Vec<RangeTrackingReference>>,
    pending_range_change: RefCell<Option<PendingRangeTextChange>>,
    /// The replacement an edit of the model is making (its range and its normalized
    /// text). The next text observation reports it as the change when it explains the
    /// new text, so live positions follow the real edit and not a diff, which cannot
    /// tell where a repeated character was inserted (#243 review, S2).
    pending_edit: RefCell<Option<(Range<usize>, String)>>,
    range_change_generation: Cell<u64>,
    cached_text: RefCell<Rc<str>>,
    /// The UTF-16 length of `cached_text` (F1: no scan of the text per query).
    cached_text_length: Cell<usize>,
    observed_text: RefCell<Rc<str>>,
    /// The mutation version `observed_text` was taken at (F1b): when it is still current,
    /// `publish_change` takes the text from it instead of reading the tree again.
    observed_version: Cell<u64>,
    /// Set while the fast path of `replace_text_core` splices one run: the tree's text is then
    /// the observed text with the pending edit applied, without reading the tree (F1b).
    exact_splice: Cell<bool>,
    /// The text of the tree after a replacement of some of its blocks, set just before the
    /// replacement is observed (F1c): the observed text with the replaced paragraphs swapped,
    /// so the observation reads no other paragraph of the tree.
    exact_text: RefCell<Option<String>>,
    /// Test only: every installation replaces all blocks, the path before F1c, the reference the
    /// incremental paths are compared with.
    #[cfg(test)]
    full_rebuild: Cell<bool>,
    text_changes: RefCell<Vec<DocumentTextChange>>,
    installing: Cell<bool>,
    delivering: Cell<bool>,
    pending_events: RefCell<VecDeque<DocumentChangedEventArgs>>,
    edit_depth: Cell<usize>,
    silent_depth: Cell<usize>,
    dirty: Cell<bool>,
    change_kind: Cell<Option<DocumentChangeKind>>,
    subscription: RefCell<Option<Subscription>>,
    link_fields: RefCell<Vec<DocumentLinkField>>,
    next_link_field_id: Cell<u64>,
    link_allocation_floor: Cell<u64>,
    installed_field_snapshot: Cell<bool>,
    reconciling_fields: Cell<bool>,
}

impl Drop for RichTextDocumentData {
    fn drop(&mut self) {
        if let Some(subscription) = self.subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}

#[derive(Clone)]
pub struct WeakRichTextDocument(Weak<RichTextDocumentData>);
impl WeakRichTextDocument {
    pub fn upgrade(&self) -> Option<RichTextDocument> {
        self.0.upgrade().map(RichTextDocument)
    }
}

impl PartialEq for RichTextDocument {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for RichTextDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RichTextDocument")
            .field("revision", &self.revision())
            .field("paragraph_count", &self.0.blocks.count())
            .field("text_length", &self.text_length())
            .finish()
    }
}

impl Default for RichTextDocument {
    fn default() -> Self {
        Self::new()
    }
}

ferroui_base::ferro_markup_type!(class RichTextDocument {
    namespace: "FerroUI.Controls.Documents",
    handles: [RichTextDocument, Option<RichTextDocument>],
    constructors: [() => RichTextDocument::new],
    properties: [Blocks: BlockCollection { get: RichTextDocument::blocks }],
});

impl RichTextDocument {
    pub fn new() -> Self {
        Self::with_blocks(BlockCollection::new())
    }

    pub fn with_blocks(blocks: BlockCollection) -> Self {
        let text: Rc<str> = Rc::from(blocks.text());
        let document = Self(Rc::new(RichTextDocumentData {
            blocks,
            revision: Cell::new(0),
            mutation_version: Cell::new(0),
            changed: Rc::new(HandlerList::new()),
            position_changed: Rc::new(HandlerList::new()),
            range_position_changed: Rc::new(HandlerList::new()),
            range_positions: RefCell::new(Vec::new()),
            pending_range_change: RefCell::new(None),
            pending_edit: RefCell::new(None),
            range_change_generation: Cell::new(0),
            cached_text_length: Cell::new(text.encode_utf16().count()),
            cached_text: RefCell::new(text.clone()),
            observed_text: RefCell::new(text),
            observed_version: Cell::new(u64::MAX),
            exact_splice: Cell::new(false),
            exact_text: RefCell::new(None),
            #[cfg(test)]
            full_rebuild: Cell::new(false),
            text_changes: RefCell::new(Vec::new()),
            installing: Cell::new(false),
            delivering: Cell::new(false),
            pending_events: RefCell::new(VecDeque::new()),
            edit_depth: Cell::new(0),
            silent_depth: Cell::new(0),
            dirty: Cell::new(false),
            change_kind: Cell::new(None),
            subscription: RefCell::new(None),
            link_fields: RefCell::new(Vec::new()),
            next_link_field_id: Cell::new(1),
            link_allocation_floor: Cell::new(1),
            installed_field_snapshot: Cell::new(false),
            reconciling_fields: Cell::new(false),
        }));
        let weak = document.downgrade();
        let subscription = document.blocks().invalidated(move || {
            if let Some(document) = weak.upgrade() {
                document.on_invalidated();
            }
        });
        *document.0.subscription.borrow_mut() = Some(subscription);
        document
    }

    pub fn from_text(text: &str) -> Self {
        let blocks = BlockCollection::new();
        for paragraph in normalize_paragraph_breaks(text).split('\n') {
            blocks.add(Paragraph::with_text(paragraph));
        }
        Self::with_blocks(blocks)
    }

    pub fn blocks(&self) -> BlockCollection {
        self.0.blocks.clone()
    }
    pub fn revision(&self) -> u64 {
        self.0.revision.get()
    }
    /// Internal state identity advances during a batch, before public revisions
    /// are published. IME previews must detect an intervening model mutation even
    /// while an application holds an outer edit scope open.
    pub(crate) fn mutation_version(&self) -> u64 {
        self.0.mutation_version.get()
    }
    pub(crate) fn link_allocation_floor(&self) -> u64 {
        self.0.link_allocation_floor.get()
    }

    pub fn downgrade(&self) -> WeakRichTextDocument {
        WeakRichTextDocument(Rc::downgrade(&self.0))
    }
    pub fn paragraphs(&self) -> Vec<Ref<Paragraph>> {
        self.0
            .blocks
            .snapshot()
            .iter()
            .filter_map(|block| block.cast::<Paragraph>())
            .collect()
    }

    pub fn text(&self) -> String {
        if self.0.dirty.get() {
            self.0.blocks.text()
        } else {
            self.0.cached_text.borrow().to_string()
        }
    }

    pub fn text_length(&self) -> usize {
        if self.0.dirty.get() {
            self.0.blocks.text_length()
        } else {
            self.0.cached_text_length.get()
        }
    }

    /// Calls `f` with the text, borrowed when it is current (no copy of the document).
    fn with_text<R>(&self, f: impl FnOnce(&str, usize) -> R) -> R {
        if self.0.dirty.get() {
            let text = self.0.blocks.text();
            let length = text.encode_utf16().count();
            f(&text, length)
        } else {
            let text = self.0.cached_text.borrow().clone();
            f(&text, self.0.cached_text_length.get())
        }
    }

    /// Dropping the returned guard removes the handler; [`Subscription::detach`] keeps it for the
    /// life of the document.
    pub fn changed(
        &self,
        handler: impl Fn(&DocumentChangedEventArgs) + 'static,
    ) -> Subscription {
        let token = self.0.changed.add(Rc::new(handler));
        let weak = Rc::downgrade(&self.0.changed);
        Subscription::new(Disposable::create(move || {
            if let Some(handlers) = weak.upgrade() {
                handlers.remove(token);
            }
        }))
    }

    /// Internal position tracking runs at each completed model replacement. The
    /// public Changed event stays batched, but pointers must already address the
    /// current text when another pointer is created or used inside that batch.
    pub(super) fn position_changed(
        &self,
        handler: impl Fn(&DocumentTextChange) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.0.position_changed.add(Rc::new(handler));
        let weak = Rc::downgrade(&self.0.position_changed);
        Disposable::create(move || {
            if let Some(handlers) = weak.upgrade() {
                handlers.remove(token);
            }
        })
    }

    /// Private paired-range bookkeeping. Ordinary TextPointer listeners keep
    /// receiving the visible minimal change through their separate channel.
    /// The optional operation describes only an accepted explicit range SetText;
    /// typed input, rich fragments, restoration and external tree writes omit it.
    pub(crate) fn range_position_changed(
        &self,
        position: &Rc<Cell<TextRange>>,
        paired_generation: &Rc<Cell<u64>>,
        handler: impl Fn(&DocumentTextChange, Option<&DocumentTextChange>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let mut positions = self.0.range_positions.borrow_mut();
        positions.retain(|entry| entry.position.strong_count() != 0);
        positions.push(RangeTrackingReference {
            position: Rc::downgrade(position),
            paired_generation: Rc::downgrade(paired_generation),
        });
        drop(positions);
        let token = self.0.range_position_changed.add(Rc::new(handler));
        let weak = Rc::downgrade(&self.0.range_position_changed);
        Disposable::create(move || {
            if let Some(handlers) = weak.upgrade() {
                handlers.remove(token);
            }
        })
    }

    pub(crate) fn capture_range_tracking(&self) -> RangeTrackingCheckpoint {
        let mut positions = self.0.range_positions.borrow_mut();
        positions.retain(|entry| entry.position.strong_count() != 0);
        RangeTrackingCheckpoint {
            positions: positions
                .iter()
                .filter_map(|entry| {
                    let position = entry.position.upgrade()?;
                    let generation = entry.paired_generation.upgrade()?;
                    Some((
                        RangeTrackingReference {
                            position: entry.position.clone(),
                            paired_generation: entry.paired_generation.clone(),
                        },
                        position.get(),
                        generation.get(),
                    ))
                })
                .collect(),
        }
    }

    /// Restore only trackers which participated in an explicit SetText and still
    /// have their original identity. An explicit endpoint setter replaces that
    /// identity; new or dropped ranges cannot be resurrected by a failed group.
    pub(crate) fn restore_range_tracking(&self, checkpoint: RangeTrackingCheckpoint) {
        for (entry, before, generation) in checkpoint.positions {
            let (Some(position), Some(current)) =
                (entry.position.upgrade(), entry.paired_generation.upgrade())
            else {
                continue;
            };
            if current.get() != generation {
                position.set(before);
                current.set(current.get().wrapping_add(1));
            }
        }
    }

    fn begin_range_text_change(
        &self,
        range: Range<usize>,
        inserted: String,
        handlers: RangePositionHandlers,
    ) -> Option<RangeTextChangeScope> {
        let before = self.0.observed_text.borrow().clone();
        let byte_range = utf16_to_byte(&before, range.start)?..utf16_to_byte(&before, range.end)?;
        let generation = self.0.range_change_generation.get().wrapping_add(1);
        self.0.range_change_generation.set(generation);
        let inserted_length = inserted.encode_utf16().count();
        let interrupted = Rc::new(Cell::new(false));
        let before_mutation = self.mutation_version();
        *self.0.pending_range_change.borrow_mut() = Some(PendingRangeTextChange {
            generation,
            expected_mutation: before_mutation.wrapping_add(1),
            before,
            byte_range,
            inserted,
            change: DocumentTextChange {
                range,
                inserted_length,
            },
            handlers,
            interrupted: interrupted.clone(),
        });
        Some(RangeTextChangeScope {
            document: self.clone(),
            generation,
            before_mutation,
            interrupted,
        })
    }

    /// Nested batches publish one revision and one Changed event at the outer end.
    pub fn begin_edit(&self) -> DocumentEditBatch {
        self.0.edit_depth.set(self.0.edit_depth.get() + 1);
        DocumentEditBatch {
            document: Some(self.clone()),
        }
    }

    pub fn snapshot(&self) -> DocumentSnapshot {
        DocumentSnapshot {
            paragraphs: self
                .0
                .blocks
                .snapshot()
                .iter()
                .map(|block| {
                    if let Some(paragraph) = block.cast::<Paragraph>() {
                        ParagraphSnapshot::from_paragraph(&paragraph)
                    } else {
                        ParagraphSnapshot {
                            format: ParagraphFormat {
                                character: CharacterFormat::from_element(block),
                                ..Default::default()
                            },
                            inlines: vec![InlineSnapshot::run(block.text())],
                        }
                    }
                })
                .collect(),
            link_fields: self.0.link_fields.borrow().clone(),
            next_link_field_id: self.0.next_link_field_id.get(),
        }
    }

    pub fn restore(&self, snapshot: &DocumentSnapshot) -> Result<(), DocumentError> {
        self.install(snapshot, DocumentChangeKind::Structure)
    }

    pub fn validate_range(&self, range: Range<usize>) -> Result<(), DocumentError> {
        self.with_text(|text, length| validate_text_range_of_length(text, length, range))
    }

    pub fn get_text(&self, range: Range<usize>) -> Result<String, DocumentError> {
        self.with_text(|text, length| {
            validate_text_range_of_length(text, length, range.clone())?;
            let start =
                utf16_to_byte(text, range.start).ok_or(DocumentError::InvalidUtf16Boundary {
                    offset: range.start,
                })?;
            let end = utf16_to_byte(text, range.end)
                .ok_or(DocumentError::InvalidUtf16Boundary { offset: range.end })?;
            Ok(text[start..end].to_owned())
        })
    }

    pub fn slice(&self, range: Range<usize>) -> Result<DocumentSnapshot, DocumentError> {
        self.validate_range(range.clone())?;
        Ok(self.snapshot().slice_unchecked(range))
    }

    pub fn replace_text(&self, range: Range<usize>, text: &str) -> Result<(), DocumentError> {
        self.replace_text_core(range, text, ParagraphAlignmentSource::Leading, false)
    }

    /// Explicit range SetText is the only measured origin for paired endpoint
    /// tracking. Its accepted operation span is distinct from a string diff.
    pub(crate) fn replace_range_text(
        &self,
        range: Range<usize>,
        text: &str,
    ) -> Result<(), DocumentError> {
        self.replace_text_core(range, text, ParagraphAlignmentSource::Leading, true)
    }

    /// Typed replacement across paragraphs retains the trailing paragraph's
    /// alignment. The remaining paragraph-format fields keep the established
    /// structured-document policy: the native observation corpus characterizes
    /// alignment, not tabs, indents, spacing or arbitrary paragraph defaults.
    pub(crate) fn replace_typed_text(
        &self,
        range: Range<usize>,
        text: &str,
    ) -> Result<(), DocumentError> {
        self.replace_text_core(range, text, ParagraphAlignmentSource::Trailing, false)
    }

    fn replace_text_core(
        &self,
        range: Range<usize>,
        text: &str,
        alignment_source: ParagraphAlignmentSource,
        paired_tracking: bool,
    ) -> Result<(), DocumentError> {
        self.ensure_available()?;
        self.validate_range(range.clone())?;
        if range.is_empty() && text.is_empty() {
            return Ok(());
        }
        let start_version = self.mutation_version();
        let normalized = normalize_paragraph_breaks(text);
        // The explicit range SetText reports its own operation span (paired tracking);
        // every other replacement reports its real range instead of a text diff.
        if !paired_tracking {
            *self.0.pending_edit.borrow_mut() = Some((range.clone(), normalized.clone()));
        }
        let _clear_pending_edit = PendingEditScope(self);
        if !normalized.contains('\n') && self.0.link_fields.borrow().is_empty() {
            if let Some(found) = self.find_splice_run(range.clone()) {
                let mut value = found.run.text().unwrap_or_default();
                let start = utf16_to_byte(&value, range.start - found.offset).ok_or(
                    DocumentError::InvalidUtf16Boundary {
                        offset: range.start,
                    },
                )?;
                let end = utf16_to_byte(&value, range.end - found.offset)
                    .ok_or(DocumentError::InvalidUtf16Boundary { offset: range.end })?;
                if value[start..end] == normalized {
                    return Ok(());
                }
                value.replace_range(start..end, &normalized);
                let _batch = self.begin_edit();
                self.0.change_kind.set(Some(DocumentChangeKind::Content));
                let tracking = if paired_tracking {
                    self.begin_range_text_change(
                        range,
                        normalized,
                        self.0.range_position_changed.snapshot(),
                    )
                } else {
                    None
                };
                // Only a splice of a tree whose text was observed and that no reentrant read
                // changed is known exactly; any other is read and compared.
                self.0.exact_splice.set(
                    self.mutation_version() == start_version && self.0.observed_version.get() == start_version,
                );
                found.run.set_text(Some(&value));
                self.0.exact_splice.set(false);
                if tracking
                    .as_ref()
                    .is_some_and(|scope| scope.was_interrupted())
                {
                    return Err(DocumentError::EditInProgress);
                }
                return Ok(());
            }
        }
        // A document that can be edited a paragraph at a time reads the paragraph of the
        // insertion alone (F1c); any other takes a snapshot of the whole document.
        let table = self.paragraph_table();
        let snapshot = table.is_none().then(|| self.snapshot());
        let (paragraph_index, offset) = match (&table, &snapshot) {
            (Some(table), _) => table.locate(range.start),
            (None, Some(snapshot)) => snapshot.locate(range.start),
            (None, None) => (0, 0),
        };
        let paragraph = match (&table, &snapshot) {
            (Some(table), _) => table
                .paragraphs
                .get(paragraph_index)
                .map(|paragraph| ParagraphSnapshot::from_paragraph(paragraph)),
            (None, Some(snapshot)) => snapshot.paragraphs.get(paragraph_index).cloned(),
            (None, None) => None,
        };
        let format = self
            .element_at(range.start, TextGravity::Backward)
            .map(|element| CharacterFormat::from_element(&element))
            .unwrap_or_default();
        let mut replacement = DocumentSnapshot::from_text(text);
        for paragraph in &mut replacement.paragraphs {
            if paragraph.inlines.is_empty() {
                paragraph.inlines.push(InlineSnapshot {
                    format: format.clone(),
                    kind: InlineSnapshotKind::Run(String::new()),
                });
            }
        }
        if let Some(paragraph) = &paragraph {
            let context = insertion_context(&paragraph.inlines, offset);
            for replacement_paragraph in &mut replacement.paragraphs {
                replacement_paragraph.format = paragraph.format.clone();
                for inline in &mut replacement_paragraph.inlines {
                    inline.format = format.clone();
                }
                for mut container in context.iter().rev().cloned() {
                    if let Some(children) = container.children_mut() {
                        *children = std::mem::take(&mut replacement_paragraph.inlines);
                    }
                    replacement_paragraph.inlines = vec![container];
                }
            }
        } else {
            for paragraph in &mut replacement.paragraphs {
                for inline in &mut paragraph.inlines {
                    inline.format = format.clone();
                }
            }
        }
        // A plain insertion inherits its existing semantic context. When that
        // context belongs to a field, preserve the shared identity explicitly.
        let mut inherited = None;
        fn find_field(inlines: &[InlineSnapshot]) -> Option<u64> {
            for inline in inlines {
                if let InlineSnapshotKind::Hyperlink {
                    field_id: Some(id), ..
                } = &inline.kind
                {
                    return Some(*id);
                }
                if let Some(id) = inline.children().and_then(find_field) {
                    return Some(id);
                }
            }
            None
        }
        for paragraph in &replacement.paragraphs {
            if let Some(id) = find_field(&paragraph.inlines) {
                inherited = Some(id);
                break;
            }
        }
        if replacement.text_length() == 0 {
            fn clear_empty_projection(inlines: &mut [InlineSnapshot]) {
                for inline in inlines {
                    if let InlineSnapshotKind::Hyperlink {
                        field_id: Some(_),
                        inlines,
                        ..
                    } = &mut inline.kind
                    {
                        inline.kind = InlineSnapshotKind::Span(std::mem::take(inlines));
                    }
                    if let Some(children) = inline.children_mut() {
                        clear_empty_projection(children);
                    }
                }
            }
            for paragraph in &mut replacement.paragraphs {
                clear_empty_projection(&mut paragraph.inlines);
            }
        } else if let Some(snapshot) = &snapshot {
            // A document read a paragraph at a time has no link fields to inherit.
            if let Some(field) =
                inherited.and_then(|id| snapshot.link_fields.iter().find(|field| field.id == id))
            {
                let mut field = field.clone();
                field.range = 0..replacement.text_length();
                replacement.link_fields.push(field);
                replacement.next_link_field_id = snapshot.next_link_field_id;
            }
        }
        self.replace_range_internal(range, &replacement, true, alignment_source, paired_tracking)
    }

    pub fn replace_range(
        &self,
        range: Range<usize>,
        fragment: &DocumentSnapshot,
    ) -> Result<(), DocumentError> {
        self.replace_range_internal(
            range,
            fragment,
            false,
            ParagraphAlignmentSource::Leading,
            false,
        )
    }

    /// Internal text transformations retain existing field identity; copied
    /// rich fragments use replace_range and receive fresh destination IDs.
    pub(crate) fn replace_range_preserving_link_fields(
        &self,
        range: Range<usize>,
        fragment: &DocumentSnapshot,
    ) -> Result<(), DocumentError> {
        let fields = self.0.link_fields.borrow().clone();
        for field in &fragment.link_fields {
            if !fields.iter().any(|original| {
                original.id == field.id
                    && original.target == field.target
                    && original.target_name == field.target_name
                    && original.has_sentinel == field.has_sentinel
            }) {
                return Err(DocumentError::InvalidLinkField(
                    "An inherited fragment must refer to an unchanged destination field.",
                ));
            }
        }
        self.replace_range_internal(
            range,
            fragment,
            true,
            ParagraphAlignmentSource::Leading,
            false,
        )
    }

    fn replace_range_internal(
        &self,
        range: Range<usize>,
        fragment: &DocumentSnapshot,
        inherited: bool,
        alignment_source: ParagraphAlignmentSource,
        paired_tracking: bool,
    ) -> Result<(), DocumentError> {
        self.ensure_available()?;
        self.validate_range(range.clone())?;
        fragment.validate()?;
        if range.is_empty() && fragment.text_length() == 0 {
            return Ok(());
        }
        // Without link fields the replacement reads and rebuilds the paragraphs it spans,
        // not the document (F1c).
        let version = self.mutation_version();
        if fragment.link_fields.is_empty() {
            if let Some(table) = self.paragraph_table() {
                let edit = ParagraphEdit {
                    range: range.clone(),
                    inherited,
                    alignment_source,
                    paired_tracking,
                };
                if let Some(result) = self.replace_paragraphs(version, &table, edit, fragment) {
                    return result;
                }
            }
        }
        let before = self.snapshot();
        let mut next = before.next_link_field_id.max(self.link_allocation_floor());
        let mut fragment = fragment.clone();
        if !inherited {
            fragment.remap_link_field_ids(&mut next)?;
        } else {
            next = next.max(fragment.next_link_field_id);
        }
        let mut snapshot = before.clone();
        if snapshot.paragraphs.is_empty() {
            snapshot.paragraphs.push(ParagraphSnapshot::default());
        }
        let (start_index, start_offset) = snapshot.locate(range.start);
        let (end_index, end_offset) = snapshot.locate(range.end);
        let inserted = replacement_paragraphs(
            &snapshot.paragraphs[start_index..=end_index],
            start_offset..end_offset,
            &fragment,
            alignment_source,
        );
        snapshot
            .paragraphs
            .splice(start_index..=end_index, inserted);
        let tracked = paired_tracking.then(|| (range.clone(), fragment.text()));
        snapshot.replace_link_fields(&before, range, &fragment, next)?;
        self.install_with_range_change(&snapshot, DocumentChangeKind::Content, tracked)
    }

    /// The replacement of `edit.range` by `fragment` (which has no link fields) in a document
    /// without link fields, reading and rebuilding only the paragraphs the range spans (F1c).
    /// It has the result of the path through a snapshot of the document: the same paragraphs,
    /// the same identity allocation for fields. `None` when it does not apply (the inserted
    /// content holds an embedded control from another paragraph or from outside the document);
    /// nothing has changed then.
    fn replace_paragraphs(
        &self,
        version: u64,
        table: &ParagraphTable,
        edit: ParagraphEdit,
        fragment: &DocumentSnapshot,
    ) -> Option<Result<(), DocumentError>> {
        let mut next = self
            .0
            .next_link_field_id
            .get()
            .max(self.link_allocation_floor());
        let mut fragment = fragment.clone();
        if !edit.inherited {
            if let Err(error) = fragment.remap_link_field_ids(&mut next) {
                return Some(Err(error));
            }
        } else {
            next = next.max(fragment.next_link_field_id);
        }
        let (start_index, start_offset) = table.locate(edit.range.start);
        let (end_index, end_offset) = table.locate(edit.range.end);
        let old = table.snapshots(start_index..end_index + 1);
        let inserted = replacement_paragraphs(
            &old,
            start_offset..end_offset,
            &fragment,
            edit.alignment_source,
        );
        // A control of another paragraph would be a duplicate, one from outside may have a
        // parent: the path through a snapshot of the document tells them apart.
        let controls = embedded_controls(&old);
        if embedded_controls(&inserted)
            .iter()
            .any(|control| !controls.contains(control))
        {
            return None;
        }
        let candidate = DocumentSnapshot {
            paragraphs: inserted,
            link_fields: Vec::new(),
            next_link_field_id: next,
        };
        if let Err(error) = candidate.validate() {
            return Some(Err(error));
        }
        let tracked = edit
            .paired_tracking
            .then(|| (edit.range.clone(), fragment.text()));
        Some(self.install_paragraph_change(
            version,
            table,
            ParagraphChange {
                start: start_index,
                old: &old,
                new: &candidate.paragraphs,
            },
            Some((Vec::new(), next)),
            DocumentChangeKind::Content,
            tracked,
        ))
    }

    /// Installs `change.new` in place of the paragraphs of `table` that `change.old` holds the
    /// snapshots of, leaving alone the paragraphs at either end that do not change (F1c).
    fn install_paragraph_change(
        &self,
        version: u64,
        table: &ParagraphTable,
        change: ParagraphChange<'_>,
        fields: Option<(Vec<DocumentLinkField>, u64)>,
        kind: DocumentChangeKind,
        range_change: Option<(Range<usize>, String)>,
    ) -> Result<(), DocumentError> {
        let ParagraphChange { start, old, new } = change;
        let (prefix, suffix) = common_paragraphs(&|_| true, old, new);
        let removed = start + prefix..start + old.len() - suffix;
        let inserted = &new[prefix..new.len() - suffix];
        let text = self.replaced_text(version, &table.lengths, removed.clone(), inserted);
        let blocks: Vec<Ref<Block>> = table.paragraphs[removed.clone()]
            .iter()
            .map(|paragraph| paragraph.clone().upcast::<Block>())
            .collect();
        self.install_blocks(
            version,
            BlockReplacement {
                start: removed.start,
                removed: &blocks,
                inserted,
                text,
            },
            fields,
            kind,
            range_change,
        )
    }

    pub fn apply_format(
        &self,
        range: Range<usize>,
        format: &CharacterFormat,
    ) -> Result<(), DocumentError> {
        self.ensure_available()?;
        self.validate_range(range.clone())?;
        format.validate()?;
        if range.is_empty() || *format == CharacterFormat::default() {
            return Ok(());
        }
        if let Some(found) = self.find_splice_run(range.clone()) {
            self.format_run(found, range, format)?;
            return Ok(());
        }
        // Without link fields only the paragraphs of the range are read and rebuilt (F1c).
        let version = self.mutation_version();
        if let Some(table) = self.paragraph_table() {
            let mut touched: Option<Range<usize>> = None;
            let mut offset = 0;
            for (index, length) in table.lengths.iter().copied().enumerate() {
                if formats_paragraph(&range, offset, length) {
                    let start = touched.as_ref().map_or(index, |touched| touched.start);
                    touched = Some(start..index + 1);
                }
                offset += length + 1;
            }
            let Some(touched) = touched else {
                return Ok(());
            };
            let old = table.snapshots(touched.clone());
            let mut offset = table.start(touched.start);
            let mut new = old.clone();
            for paragraph in &mut new {
                let length = paragraph.text_length();
                format_paragraph(paragraph, &range, offset, length, format);
                offset += length + 1;
            }
            let candidate = DocumentSnapshot {
                paragraphs: new,
                ..DocumentSnapshot::default()
            };
            candidate.validate()?;
            return self.install_paragraph_change(
                version,
                &table,
                ParagraphChange {
                    start: touched.start,
                    old: &old,
                    new: &candidate.paragraphs,
                },
                None,
                DocumentChangeKind::Formatting,
                None,
            );
        }
        let mut snapshot = self.snapshot();
        let mut offset = 0;
        for paragraph in &mut snapshot.paragraphs {
            let length = paragraph.text_length();
            format_paragraph(paragraph, &range, offset, length, format);
            offset += length + 1;
        }
        self.install(&snapshot, DocumentChangeKind::Formatting)
    }

    pub fn apply_paragraph_format(
        &self,
        range: Range<usize>,
        format: &ParagraphFormat,
    ) -> Result<(), DocumentError> {
        self.ensure_available()?;
        self.validate_range(range.clone())?;
        format.validate()?;
        // Without link fields only the paragraphs of the range are read and rebuilt (F1c).
        let version = self.mutation_version();
        if let Some(table) = self.paragraph_table() {
            let (start, _) = table.locate(range.start);
            let (mut end, end_offset) = table.locate(range.end);
            if end > start && end_offset == 0 && !range.is_empty() {
                end -= 1;
            }
            let old = table.snapshots(start..end + 1);
            let mut new = old.clone();
            for paragraph in &mut new {
                paragraph.format = paragraph.format.overlay(format);
            }
            let candidate = DocumentSnapshot {
                paragraphs: new,
                ..DocumentSnapshot::default()
            };
            candidate.validate()?;
            return self.install_paragraph_change(
                version,
                &table,
                ParagraphChange {
                    start,
                    old: &old,
                    new: &candidate.paragraphs,
                },
                None,
                DocumentChangeKind::Formatting,
                None,
            );
        }
        let mut snapshot = self.snapshot();
        if snapshot.paragraphs.is_empty() {
            snapshot.paragraphs.push(ParagraphSnapshot::default());
        }
        let (start, _) = snapshot.locate(range.start);
        let (mut end, end_offset) = snapshot.locate(range.end);
        if end > start && end_offset == 0 && !range.is_empty() {
            end -= 1;
        }
        for paragraph in &mut snapshot.paragraphs[start..=end] {
            paragraph.format = paragraph.format.overlay(format);
        }
        self.install(&snapshot, DocumentChangeKind::Formatting)
    }

    pub fn character_format_at(&self, offset: usize) -> Result<CharacterFormat, DocumentError> {
        self.validate_range(offset..offset)?;
        if let Some(element) = self.element_at(offset, TextGravity::Backward) {
            return Ok(CharacterFormat::effective(&element));
        }
        Ok(CharacterFormat::default())
    }

    pub fn element_at(&self, offset: usize, gravity: TextGravity) -> Option<Ref<TextElement>> {
        if offset > self.text_length() {
            return None;
        }
        let mut remaining = offset;
        let blocks = self.blocks().snapshot();
        for (index, block) in blocks.iter().enumerate() {
            let length = block.text_length();
            if remaining <= length || index + 1 == blocks.len() {
                if let Some(paragraph) = block.cast::<Paragraph>() {
                    return find_inline(&paragraph.inlines(), remaining.min(length), gravity)
                        .map(|inline| inline.upcast())
                        .or_else(|| Some(paragraph.upcast()));
                }
                return Some(block.clone().upcast());
            }
            remaining = remaining.saturating_sub(length + 1);
        }
        None
    }

    pub fn create_pointer(
        &self,
        offset: usize,
        gravity: TextGravity,
    ) -> Result<TextPointer, DocumentError> {
        TextPointer::new(self, offset, gravity)
    }

    pub fn is_grapheme_boundary(&self, offset: usize) -> bool {
        let text: Vec<u16> = self.text().encode_utf16().collect();
        offset == text.len() || GraphemeEnumerator::new(&text).any(|g| g.offset() == offset)
    }

    pub fn previous_grapheme(&self, offset: usize) -> usize {
        let text: Vec<u16> = self.text().encode_utf16().collect();
        GraphemeEnumerator::new(&text)
            .map(|g| g.offset())
            .take_while(|v| *v < offset.min(text.len()))
            .last()
            .unwrap_or(0)
    }

    pub fn next_grapheme(&self, offset: usize) -> usize {
        let text: Vec<u16> = self.text().encode_utf16().collect();
        GraphemeEnumerator::new(&text)
            .map(|g| g.offset() + g.length())
            .find(|v| *v > offset)
            .unwrap_or(text.len())
    }

    pub fn previous_word(&self, offset: usize) -> usize {
        let text: Vec<u16> = self.text().encode_utf16().collect();
        let mut previous = 0;
        for word in WordBreakEnumerator::new(&text) {
            if word.offset() >= offset.min(text.len()) {
                break;
            }
            if !segment_is_whitespace(word.text()) {
                previous = word.offset();
            }
        }
        previous
    }

    pub fn next_word(&self, offset: usize) -> usize {
        let text: Vec<u16> = self.text().encode_utf16().collect();
        let mut consumed_current = false;
        for word in WordBreakEnumerator::new(&text) {
            if word.offset() + word.length() <= offset {
                continue;
            }
            if !consumed_current {
                consumed_current = true;
                continue;
            }
            if !segment_is_whitespace(word.text()) {
                return word.offset();
            }
        }
        text.len()
    }

    pub(crate) fn set_inline_host(&self, host: Option<Rc<dyn IInlineHost>>) {
        self.0.silent_depth.set(self.0.silent_depth.get() + 1);
        self.0.blocks.set_inline_host(host);
        self.0.silent_depth.set(self.0.silent_depth.get() - 1);
    }

    pub(crate) fn set_logical_parent(&self, parent: Option<&StyledElement>) {
        self.0.silent_depth.set(self.0.silent_depth.get() + 1);
        self.0.blocks.set_logical_parent(parent);
        self.0.silent_depth.set(self.0.silent_depth.get() - 1);
    }

    fn install(
        &self,
        snapshot: &DocumentSnapshot,
        kind: DocumentChangeKind,
    ) -> Result<(), DocumentError> {
        self.install_with_range_change(snapshot, kind, None)
    }

    fn install_with_range_change(
        &self,
        snapshot: &DocumentSnapshot,
        kind: DocumentChangeKind,
        range_change: Option<(Range<usize>, String)>,
    ) -> Result<(), DocumentError> {
        self.ensure_available()?;
        let version = self.mutation_version();
        snapshot.validate()?;
        let old = self.snapshot();
        let unchanged = old == *snapshot;
        // Snapshot capture and custom brush equality can call application code.
        // A completed nested write owns its state even when our candidate is a
        // no-op relative to the earlier image.
        self.ensure_install_read(version)?;
        if unchanged {
            return Ok(());
        }
        // The paragraphs at either end that are the same before and after stay in the tree
        // with their elements, and so with their layout in the views (F1c): an undo, a restore
        // or a structural edit through a snapshot rebuilds the paragraphs it changes.
        let blocks = self.blocks().snapshot();
        let (prefix, suffix) = if self.rebuilds_everything() {
            (0, 0)
        } else {
            common_paragraphs(
                &|index| blocks[index].is::<Paragraph>(),
                &old.paragraphs,
                &snapshot.paragraphs,
            )
        };
        let removed = prefix..old.paragraphs.len() - suffix;
        let inserted = &snapshot.paragraphs[prefix..snapshot.paragraphs.len() - suffix];
        let lengths: Vec<usize> = old
            .paragraphs
            .iter()
            .map(ParagraphSnapshot::text_length)
            .collect();
        let text = if self.rebuilds_everything() {
            None
        } else {
            self.replaced_text(version, &lengths, removed.clone(), inserted)
        };
        self.install_blocks(
            version,
            BlockReplacement {
                start: prefix,
                removed: &blocks[removed],
                inserted,
                text,
            },
            Some((snapshot.link_fields.clone(), snapshot.next_link_field_id)),
            kind,
            range_change,
        )
    }

    /// Replaces the blocks `replacement.removed` (at `replacement.start`) with paragraphs built
    /// from `replacement.inserted`, reusing the elements of the removed paragraphs; every other
    /// block stays as it is (F1c). `fields` are the link fields and the next field identity
    /// after the replacement (`None` keeps them). `version` is the mutation version every read
    /// of the caller was made at: a write since rejects the installation.
    fn install_blocks(
        &self,
        version: u64,
        replacement: BlockReplacement<'_>,
        fields: Option<(Vec<DocumentLinkField>, u64)>,
        kind: DocumentChangeKind,
        range_change: Option<(Range<usize>, String)>,
    ) -> Result<(), DocumentError> {
        self.ensure_available()?;
        let fields_unchanged = fields.as_ref().is_none_or(|(fields, next)| {
            *self.0.link_fields.borrow() == *fields && self.0.next_link_field_id.get() == *next
        });
        if replacement.removed.is_empty() && replacement.inserted.is_empty() && fields_unchanged {
            return self.ensure_install_read(version);
        }
        let mut pool = ContentPool::capture_blocks(replacement.removed);
        validate_control_parents(replacement.inserted, &pool.controls())?;
        // Reuse preparation captures formatting again. Fence its final read
        // before clearing the live tree or reserving structural installation.
        self.ensure_install_read(version)?;
        // A range created by an installation callback already addresses that
        // tree state; it must not receive the replacement which preceded it.
        let range_change = range_change
            .map(|(range, inserted)| (range, inserted, self.0.range_position_changed.snapshot()));
        let _batch = self.begin_edit();
        self.0.installing.set(true);
        self.0.change_kind.set(Some(kind));
        self.blocks()
            .remove_range(replacement.start, replacement.removed.len());
        pool.detach_contents();
        let built: Vec<Ref<Block>> = replacement
            .inserted
            .iter()
            .map(|paragraph| pool.build_paragraph(paragraph).upcast::<Block>())
            .collect();
        self.blocks().insert_range(replacement.start, built);
        if let Some((fields, next)) = fields {
            *self.0.link_fields.borrow_mut() = fields;
            self.0.next_link_field_id.set(next);
        }
        self.0.link_allocation_floor.set(
            self.link_allocation_floor()
                .max(self.0.next_link_field_id.get()),
        );
        self.0.installed_field_snapshot.set(true);
        self.0.installing.set(false);
        let tracking = range_change.and_then(|(range, inserted, handlers)| {
            self.begin_range_text_change(range, inserted, handlers)
        });
        *self.0.exact_text.borrow_mut() = replacement.text;
        self.on_invalidated();
        self.0.exact_text.borrow_mut().take();
        if tracking
            .as_ref()
            .is_some_and(|scope| scope.was_interrupted())
        {
            return Err(DocumentError::EditInProgress);
        }
        Ok(())
    }

    /// The text of the tree once the paragraphs `removed` of a tree whose paragraphs have the
    /// UTF-16 `lengths` are replaced by `inserted`, made from the observed text (F1c). `None`
    /// when the observed text is not the tree's at `version` or does not have those lengths:
    /// the observation of the replacement then reads the tree.
    fn replaced_text(
        &self,
        version: u64,
        lengths: &[usize],
        removed: Range<usize>,
        inserted: &[ParagraphSnapshot],
    ) -> Option<String> {
        if self.0.observed_version.get() != version {
            return None;
        }
        let observed = self.0.observed_text.borrow().clone();
        let count = lengths.len();
        let first_start = lengths[..removed.start].iter().sum::<usize>() + removed.start;
        let rest_start =
            first_start + lengths[removed.clone()].iter().sum::<usize>() + removed.len();
        let total = lengths.iter().sum::<usize>() + count.saturating_sub(1);
        if utf16_to_byte(&observed, total)? != observed.len() {
            return None;
        }
        // The text before the replaced paragraphs, without the separator that follows it, and
        // the text after them; each next to a paragraph separator of the observed text.
        let before_end = if removed.start > 0 {
            Some(utf16_to_byte(&observed, first_start - 1)?)
        } else {
            None
        };
        let after_start = if removed.end < count {
            Some(utf16_to_byte(&observed, rest_start)?)
        } else {
            None
        };
        let separator_at = |byte: Option<usize>| {
            byte.and_then(|byte| observed.as_bytes().get(byte)) == Some(&b'\n')
        };
        if (removed.start > 0 && removed.start < count && !separator_at(before_end))
            || (removed.end > 0
                && removed.end < count
                && !separator_at(after_start.and_then(|start| start.checked_sub(1))))
        {
            return None;
        }
        let mut middle = String::new();
        for (index, paragraph) in inserted.iter().enumerate() {
            if index != 0 {
                middle.push('\n');
            }
            for inline in &paragraph.inlines {
                inline.append_text(&mut middle);
            }
        }
        let parts = [
            before_end.map(|end| &observed[..end]),
            (!inserted.is_empty()).then_some(middle.as_str()),
            after_start.map(|start| &observed[start..]),
        ];
        let mut text = String::with_capacity(observed.len() + middle.len() + 1);
        for (index, part) in parts.iter().flatten().enumerate() {
            if index != 0 {
                text.push('\n');
            }
            text.push_str(part);
        }
        Some(text)
    }

    /// The paragraphs of a document that an edit can read and rebuild a paragraph at a time
    /// (F1c): one whose blocks are all paragraphs, with no link fields (their extents span the
    /// document). `None` for any other document, which edits through a snapshot of it.
    fn paragraph_table(&self) -> Option<ParagraphTable> {
        if self.rebuilds_everything() || !self.0.link_fields.borrow().is_empty() {
            return None;
        }
        let blocks = self.0.blocks.snapshot();
        if blocks.is_empty() {
            return None;
        }
        let mut paragraphs = Vec::with_capacity(blocks.len());
        let mut lengths = Vec::with_capacity(blocks.len());
        for block in blocks.iter() {
            let paragraph = block.cast::<Paragraph>()?;
            lengths.push(paragraph.text_length());
            paragraphs.push(paragraph);
        }
        Some(ParagraphTable {
            paragraphs,
            lengths,
        })
    }

    #[cfg(not(test))]
    fn rebuilds_everything(&self) -> bool {
        false
    }

    /// Test only: whether every installation replaces all blocks (`full_rebuild`).
    #[cfg(test)]
    fn rebuilds_everything(&self) -> bool {
        self.0.full_rebuild.get()
    }

    /// Test only: makes every edit and installation take the path before F1c (a snapshot of the
    /// document, all blocks rebuilt), the reference of the incremental paths.
    #[cfg(test)]
    pub(crate) fn set_full_rebuild(&self, value: bool) {
        self.0.full_rebuild.set(value);
    }

    fn ensure_install_read(&self, version: u64) -> Result<(), DocumentError> {
        if self.mutation_version() == version {
            Ok(())
        } else {
            Err(DocumentError::EditInProgress)
        }
    }

    /// Replaces the whole text of the run that holds exactly `range` and whose text is
    /// `expected` with `text`, in place (one run, its format kept). Returns `false`, and changes
    /// nothing, when no such run exists or the document has link fields.
    pub(crate) fn replace_run_text_in_place(
        &self,
        range: Range<usize>,
        expected: &str,
        text: &str,
    ) -> Result<bool, DocumentError> {
        if !self.0.link_fields.borrow().is_empty() || text.contains('\n') {
            return Ok(false);
        }
        let Some(found) = self.find_splice_run(range.clone()) else {
            return Ok(false);
        };
        let current = found.run.text().unwrap_or_default();
        if found.offset != range.start || current != expected {
            return Ok(false);
        }
        self.replace_text(range, text)?;
        Ok(true)
    }

    fn find_splice_run(&self, range: Range<usize>) -> Option<RunLocation> {
        let mut offset = 0;
        for block in self.0.blocks.snapshot().iter() {
            if let Some(paragraph) = block.cast::<Paragraph>() {
                if range.start >= offset && range.end <= offset + paragraph.text_length() {
                    if let Some(found) =
                        find_run_in_collection(&paragraph.inlines(), &range, offset)
                    {
                        return Some(found);
                    }
                }
            }
            offset += block.text_length() + 1;
        }
        None
    }

    fn format_run(
        &self,
        found: RunLocation,
        range: Range<usize>,
        format: &CharacterFormat,
    ) -> Result<(), DocumentError> {
        let source = found.run.text().unwrap_or_default();
        let length = source.encode_utf16().count();
        let local = range.start - found.offset..range.end - found.offset;
        let before = CharacterFormat::from_element(&found.run);
        let after = before.overlay(format);
        if before == after {
            return Ok(());
        }
        let _batch = self.begin_edit();
        self.0.change_kind.set(Some(DocumentChangeKind::Formatting));
        if local.start == 0 && local.end == length {
            self.0.installing.set(true);
            format.apply_to(&found.run);
            self.0.installing.set(false);
            self.on_invalidated();
            return Ok(());
        }
        let start =
            utf16_to_byte(&source, local.start).ok_or(DocumentError::InvalidUtf16Boundary {
                offset: range.start,
            })?;
        let end = utf16_to_byte(&source, local.end)
            .ok_or(DocumentError::InvalidUtf16Boundary { offset: range.end })?;
        let mut pieces: Vec<Ref<Inline>> = Vec::with_capacity(3);
        if start != 0 {
            let run = Run::with_text(Some(&source[..start]));
            before.apply_to(&run);
            pieces.push(run.upcast());
        }
        let middle = Run::with_text(Some(&source[start..end]));
        after.apply_to(&middle);
        pieces.push(middle.upcast());
        if end != source.len() {
            let run = Run::with_text(Some(&source[end..]));
            before.apply_to(&run);
            pieces.push(run.upcast());
        }
        self.0.installing.set(true);
        found.collection.remove_at(found.index);
        found.collection.insert_range(found.index, pieces);
        self.0.installing.set(false);
        self.on_invalidated();
        Ok(())
    }

    fn ensure_available(&self) -> Result<(), DocumentError> {
        if self.0.installing.get() {
            Err(DocumentError::EditInProgress)
        } else {
            Ok(())
        }
    }

    fn on_invalidated(&self) {
        if self.0.silent_depth.get() != 0 {
            return;
        }
        self.0
            .mutation_version
            .set(self.mutation_version().wrapping_add(1));
        self.0.dirty.set(true);
        if !self.0.installing.get() {
            self.observe_text_change();
        }
        if self.0.edit_depth.get() == 0 {
            self.publish_change();
        }
    }

    fn observe_text_change(&self) {
        // Take the hint before virtual text reads. A nested application mutation
        // cannot consume it, and the post-read identity check rejects stale text.
        let tracked = self.0.pending_range_change.borrow_mut().take();
        let pending_edit = self.0.pending_edit.borrow_mut().take();
        let range_handlers = tracked.as_ref().map_or_else(
            || self.0.range_position_changed.snapshot(),
            |tracked| tracked.handlers.clone(),
        );
        let mutation = self.mutation_version();
        // A replacement of some blocks gives the new text, made from the observed one (F1c).
        // One run spliced by the editor's fast path: the new text is the observed one with the
        // edit applied (no read of the tree, which can call application code, and no diff).
        let replaced = self.0.exact_text.borrow_mut().take();
        let spliced = if replaced.is_some() {
            replaced
        } else if self.0.exact_splice.get() {
            pending_edit.as_ref().and_then(|(range, inserted)| {
                let before = self.0.observed_text.borrow().clone();
                let start = utf16_to_byte(&before, range.start)?;
                let end = utf16_to_byte(&before, range.end)?;
                let mut text = String::with_capacity(before.len() - (end - start) + inserted.len());
                text.push_str(&before[..start]);
                text.push_str(inserted);
                text.push_str(&before[end..]);
                Some(text)
            })
        } else {
            None
        };
        let text = match spliced {
            Some(text) => text,
            None => self.0.blocks.text(),
        };
        if self.mutation_version() != mutation {
            // A virtual text read accepted a nested model mutation. Its own
            // observation already owns the current text and coordinates; this
            // outer read must not publish a stale synthetic replacement.
            if let Some(tracked) = &tracked {
                tracked.interrupted.set(true);
            }
            return;
        }
        let previous = self.0.observed_text.replace(Rc::from(text.as_str()));
        self.0.observed_version.set(mutation);
        let change = (previous.as_ref() != text).then(|| {
            let (range, inserted_length) = pending_edit
                .as_ref()
                .and_then(|(range, inserted)| explained_change(&previous, &text, range, inserted))
                .unwrap_or_else(|| changed_range(&previous, &text));
            DocumentTextChange {
                range,
                inserted_length,
            }
        });
        let installed = self.0.installed_field_snapshot.replace(false);
        let operation = tracked.as_ref().filter(|tracked| {
            let matches = tracked.matches(&previous, &text, mutation);
            if !matches {
                tracked.interrupted.set(true);
            }
            matches
        });
        if let Some(change) = &change {
            self.0.text_changes.borrow_mut().push(change.clone());
            // These handlers are private pointer bookkeeping, with no application
            // callbacks. Apply each replacement once before public notifications;
            // a pointer created afterwards starts in the new coordinates and must
            // not replay this replacement when the enclosing batch is published.
            for (_, handler) in self.0.position_changed.snapshot().iter() {
                handler(change);
            }
            for (_, handler) in range_handlers.iter() {
                handler(change, operation.map(|tracked| &tracked.change));
            }
        }
        // Semantic reconciliation can call custom inline/block readers. Both
        // private coordinate channels must already address the accepted text
        // before a nested mutation can emit its subsequent replacement.
        let has_fields = !self.0.link_fields.borrow().is_empty();
        if !installed && has_fields {
            self.reconcile_external_fields(change.as_ref());
        }
    }

    fn reconcile_external_fields(&self, change: Option<&DocumentTextChange>) {
        if self.0.reconciling_fields.replace(true) {
            return;
        }
        let version = self.mutation_version();
        // Reconciliation needs semantic extents, not frozen brush/font values.
        // Avoid invoking formatting callbacks recursively while an external
        // property or text invalidation is already being observed.
        fn semantic_inline(inline: &Ref<Inline>) -> InlineSnapshot {
            let kind = if let Some(run) = inline.cast::<Run>() {
                InlineSnapshotKind::Run(run.text().unwrap_or_default())
            } else if let Some(link) = inline.cast::<Hyperlink>() {
                InlineSnapshotKind::Hyperlink {
                    inlines: link
                        .inlines()
                        .snapshot()
                        .iter()
                        .map(semantic_inline)
                        .collect(),
                    field_id: link.field_id(),
                    behavior: Default::default(),
                    navigate_uri: link.navigate_uri(),
                    target_name: link.target_name(),
                    is_enabled: true,
                    is_tab_stop: true,
                    tab_index: 0,
                    is_visited: false,
                    underline_style: super::UnderlineStyle::Single,
                }
            } else if let Some(span) = inline.cast::<Span>() {
                InlineSnapshotKind::Span(
                    span.inlines()
                        .snapshot()
                        .iter()
                        .map(semantic_inline)
                        .collect(),
                )
            } else if inline.is::<LineBreak>() {
                InlineSnapshotKind::LineBreak
            } else if let Some(container) = inline.cast::<InlineUIContainer>() {
                InlineSnapshotKind::Embedded(container.child())
            } else {
                let mut text = String::new();
                inline.append_text(&mut text);
                InlineSnapshotKind::Run(text)
            };
            InlineSnapshot {
                format: CharacterFormat::default(),
                kind,
            }
        }
        let paragraphs = self
            .blocks()
            .snapshot()
            .iter()
            .map(|block| ParagraphSnapshot {
                format: ParagraphFormat::default(),
                inlines: block.cast::<Paragraph>().map_or_else(
                    || vec![InlineSnapshot::run(block.text())],
                    |paragraph| {
                        paragraph
                            .inlines()
                            .snapshot()
                            .iter()
                            .map(semantic_inline)
                            .collect()
                    },
                ),
            })
            .collect();
        let mut snapshot = DocumentSnapshot {
            paragraphs,
            link_fields: self.0.link_fields.borrow().clone(),
            next_link_field_id: self.0.next_link_field_id.get(),
        };
        snapshot.reconcile_external_link_fields(change);
        if self.mutation_version() == version {
            let retained: HashSet<_> = snapshot.link_fields.iter().map(|field| field.id).collect();
            fn clear_retired(inlines: &InlineCollection, retained: &HashSet<u64>) {
                for inline in inlines.snapshot().iter() {
                    if let Some(link) = inline.cast::<Hyperlink>() {
                        if link.field_id().is_some_and(|id| !retained.contains(&id)) {
                            link.set_field_id(None);
                        }
                    }
                    if let Some(span) = inline.cast::<Span>() {
                        clear_retired(&span.inlines(), retained);
                    }
                }
            }
            for paragraph in self.paragraphs() {
                clear_retired(&paragraph.inlines(), &retained);
            }
            *self.0.link_fields.borrow_mut() = snapshot.link_fields;
        }
        self.0.reconciling_fields.set(false);
    }

    fn publish_change(&self) {
        if !self.0.dirty.replace(false) {
            return;
        }
        // The text the last observation read is current when nothing changed since (F1b).
        let observed = (self.0.observed_version.get() == self.mutation_version())
            .then(|| self.0.observed_text.borrow().clone());
        let new_text: Rc<str> = observed.unwrap_or_else(|| Rc::from(self.0.blocks.text().as_str()));
        let old_text = self.0.cached_text.replace(new_text.clone());
        let old_length = self.0.cached_text_length.get();
        let new_length = new_text.encode_utf16().count();
        self.0.cached_text_length.set(new_length);
        // One recorded replacement since the last publication is the change itself; any other
        // sequence is compared.
        let recorded = {
            let changes = self.0.text_changes.borrow();
            (changes.len() == 1
                && old_length + changes[0].inserted_length == new_length + changes[0].range.len())
            .then(|| (changes[0].range.clone(), changes[0].inserted_length))
        };
        let (range, inserted_length) = recorded.unwrap_or_else(|| changed_range(&old_text, &new_text));
        let requested_kind = self.0.change_kind.take();
        let kind = if old_text.as_ref() != new_text.as_ref() {
            DocumentChangeKind::Content
        } else {
            requested_kind.unwrap_or(DocumentChangeKind::Formatting)
        };
        let revision = self.revision().wrapping_add(1);
        self.0.revision.set(revision);
        let change = DocumentChangedEventArgs {
            revision,
            removed_length: range.len(),
            range,
            inserted_length,
            old_length,
            new_length,
            kind,
            changes: std::mem::take(&mut *self.0.text_changes.borrow_mut()),
        };
        self.0.pending_events.borrow_mut().push_back(change);
        if self.0.delivering.replace(true) {
            return;
        }
        loop {
            let change = self.0.pending_events.borrow_mut().pop_front();
            let Some(change) = change else { break };
            for (_, handler) in self.0.changed.snapshot().iter() {
                handler(&change);
            }
        }
        self.0.delivering.set(false);
    }
}

/// An edit scope. Dropping or explicitly finishing it ends its nesting level.
pub struct DocumentEditBatch {
    document: Option<RichTextDocument>,
}
impl DocumentEditBatch {
    pub fn finish(mut self) {
        self.close();
    }
    fn close(&mut self) {
        let Some(document) = self.document.take() else {
            return;
        };
        let depth = document.0.edit_depth.get().saturating_sub(1);
        document.0.edit_depth.set(depth);
        if depth == 0 {
            document.publish_change();
        }
    }
}
impl Drop for DocumentEditBatch {
    fn drop(&mut self) {
        self.close();
    }
}

/// Checks `range` against `text`, whose UTF-16 length is `length`.
fn validate_text_range_of_length(text: &str, length: usize, range: Range<usize>) -> Result<(), DocumentError> {
    if range.start > range.end {
        return Err(DocumentError::ReversedRange {
            start: range.start,
            end: range.end,
        });
    }
    // The length is the one kept with the text (F1c: no count of the document per edit).
    if range.end > length {
        return Err(DocumentError::OffsetOutOfRange {
            offset: range.end,
            length,
        });
    }
    for offset in [range.start, range.end] {
        if utf16_to_byte(text, offset).is_none() {
            return Err(DocumentError::InvalidUtf16Boundary { offset });
        }
    }
    Ok(())
}

fn segment_is_whitespace(text: &[u16]) -> bool {
    char::decode_utf16(text.iter().copied()).all(|value| value.is_ok_and(char::is_whitespace))
}

/// Clears the pending edit of a replacement when it ends, whatever way it ends.
struct PendingEditScope<'a>(&'a RichTextDocument);

impl Drop for PendingEditScope<'_> {
    fn drop(&mut self) {
        self.0 .0.pending_edit.borrow_mut().take();
    }
}

/// The change of a replacement of `range` (UTF-16) by `inserted` when it turns `before`
/// into `after`, else `None` (the edit was normalized, or something else changed).
/// Text the replacement keeps at either end of the range is not part of the change,
/// so replacing "ab" by "abX" inserts at the end of "ab"; outside the range the real
/// edit wins over a text diff (#243 review, S2).
fn explained_change(before: &str, after: &str, range: &Range<usize>, inserted: &str) -> Option<(Range<usize>, usize)> {
    let start = utf16_to_byte(before, range.start)?;
    let end = utf16_to_byte(before, range.end)?;
    let explains = after.len() == before.len() - (end - start) + inserted.len()
        && after.starts_with(&before[..start])
        && after[start..].starts_with(inserted)
        && after.ends_with(&before[end..]);
    if !explains {
        return None;
    }
    let (prefix, replaced, inserted) = trim_common(&before[start..end], inserted);
    let prefix = before[start..start + prefix].encode_utf16().count();
    let replaced_length = replaced.encode_utf16().count();
    let changed_start = range.start + prefix;
    Some((changed_start..changed_start + replaced_length, inserted.encode_utf16().count()))
}

/// Removes the common prefix and then the common suffix (whole characters) of two
/// strings, returning the length of the prefix in bytes and what is left of each.
fn trim_common<'a>(left: &'a str, right: &'a str) -> (usize, &'a str, &'a str) {
    let prefix: usize = left
        .chars()
        .zip(right.chars())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum();
    let (left, right) = (&left[prefix..], &right[prefix..]);
    let suffix: usize = left
        .chars()
        .rev()
        .zip(right.chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum();
    (prefix, &left[..left.len() - suffix], &right[..right.len() - suffix])
}

fn changed_range(before: &str, after: &str) -> (Range<usize>, usize) {
    let prefix = before
        .chars()
        .zip(after.chars())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf16())
        .sum::<usize>();
    let old_len = before.encode_utf16().count();
    let new_len = after.encode_utf16().count();
    let max_suffix = old_len.min(new_len).saturating_sub(prefix);
    let mut suffix = 0;
    for (a, b) in before.chars().rev().zip(after.chars().rev()) {
        if a != b || suffix + a.len_utf16() > max_suffix {
            break;
        }
        suffix += a.len_utf16();
    }
    (prefix..old_len - suffix, new_len - prefix - suffix)
}

fn wrap_with_format(inlines: Vec<InlineSnapshot>, format: &CharacterFormat) -> Vec<InlineSnapshot> {
    if inlines.is_empty() || *format == CharacterFormat::default() {
        inlines
    } else {
        vec![InlineSnapshot {
            format: format.clone(),
            kind: InlineSnapshotKind::Span(inlines),
        }]
    }
}

/// The paragraphs that take the place of `paragraphs` (those a replacement spans, in order)
/// when the text from `offsets.start` in the first of them to `offsets.end` in the last is
/// replaced by `fragment`: the first keeps its format and its text before the range, the last
/// its text after the range.
fn replacement_paragraphs(
    paragraphs: &[ParagraphSnapshot],
    offsets: Range<usize>,
    fragment: &DocumentSnapshot,
    alignment_source: ParagraphAlignmentSource,
) -> Vec<ParagraphSnapshot> {
    let first = &paragraphs[0];
    let last = &paragraphs[paragraphs.len() - 1];
    let spans_paragraphs = paragraphs.len() > 1;
    let prefix = slice_inlines(&first.inlines, 0..offsets.start);
    let mut suffix = slice_inlines(&last.inlines, offsets.end..last.text_length());
    if spans_paragraphs && first.format.character != last.format.character {
        suffix = wrap_with_format(suffix, &last.format.character);
    }
    let mut inserted = if fragment.paragraphs.is_empty() {
        vec![ParagraphSnapshot::default()]
    } else {
        fragment.paragraphs.clone()
    };
    if inserted.len() == 1 {
        let mut middle = wrap_with_format(
            std::mem::take(&mut inserted[0].inlines),
            &inserted[0].format.character,
        );
        let mut inlines = prefix;
        inlines.append(&mut middle);
        inlines.append(&mut suffix);
        inserted[0].inlines = inlines;
        inserted[0].format = first.format.clone();
    } else {
        let mut middle = wrap_with_format(
            std::mem::take(&mut inserted[0].inlines),
            &inserted[0].format.character,
        );
        inserted[0].inlines = prefix;
        inserted[0].inlines.append(&mut middle);
        inserted[0].format = first.format.clone();
        let inserted_last = inserted.len() - 1;
        inserted[inserted_last].inlines.append(&mut suffix);
    }
    if alignment_source == ParagraphAlignmentSource::Trailing && spans_paragraphs {
        // A typed break creates another paragraph in the merged context;
        // every inserted paragraph therefore receives the same alignment.
        // Preserve None exactly so inherited alignment remains inherited.
        for paragraph in &mut inserted {
            paragraph.format.text_alignment = last.format.text_alignment;
        }
    }
    inserted
}

/// Whether a character format of `range` changes the paragraph at `offset` of `length` units:
/// its text in the range, or an empty paragraph whose mark the range covers.
fn formats_paragraph(range: &Range<usize>, offset: usize, length: usize) -> bool {
    (range.start < offset + length && range.end > offset)
        || (length == 0 && range.start <= offset && range.end > offset)
}

/// Applies a character format of `range` to the paragraph at `offset` of `length` units.
fn format_paragraph(
    paragraph: &mut ParagraphSnapshot,
    range: &Range<usize>,
    offset: usize,
    length: usize,
    format: &CharacterFormat,
) {
    if range.start < offset + length && range.end > offset {
        paragraph.inlines = format_inlines(
            &paragraph.inlines,
            range.start.saturating_sub(offset)..range.end.min(offset + length) - offset,
            format,
        );
    } else if length == 0 && range.start <= offset && range.end > offset {
        paragraph.format.character = paragraph.format.character.overlay(format);
    }
}

/// How many paragraphs at the start and at the end of a replacement of `old` by `new` stay as
/// they are: the same snapshot before and after, of a block that is a paragraph
/// (`is_paragraph`, by the index in `old`; another block is always rebuilt as a paragraph).
/// The two never overlap.
fn common_paragraphs(
    is_paragraph: &dyn Fn(usize) -> bool,
    old: &[ParagraphSnapshot],
    new: &[ParagraphSnapshot],
) -> (usize, usize) {
    let limit = old.len().min(new.len());
    let prefix = (0..limit)
        .take_while(|&index| is_paragraph(index) && old[index] == new[index])
        .count();
    let suffix = (0..limit - prefix)
        .take_while(|&index| {
            let at = old.len() - 1 - index;
            is_paragraph(at) && old[at] == new[new.len() - 1 - index]
        })
        .count();
    (prefix, suffix)
}

/// The embedded controls of some paragraphs, by identity.
fn embedded_controls(paragraphs: &[ParagraphSnapshot]) -> Vec<usize> {
    let mut controls = Vec::new();
    let mut pending: Vec<&InlineSnapshot> =
        paragraphs.iter().flat_map(|p| p.inlines.iter()).collect();
    while let Some(inline) = pending.pop() {
        if let InlineSnapshotKind::Embedded(Some(control)) = &inline.kind {
            controls.push(&**control as *const Control as usize);
        }
        if let Some(children) = inline.children() {
            pending.extend(children);
        }
    }
    controls
}

/// The paragraphs of a document that an edit reads a paragraph at a time, and their UTF-16
/// lengths (F1c; `RichTextDocument::paragraph_table`).
struct ParagraphTable {
    paragraphs: Vec<Ref<Paragraph>>,
    lengths: Vec<usize>,
}

impl ParagraphTable {
    /// The paragraph of `offset` and the offset in it, as `DocumentSnapshot::locate` gives them.
    fn locate(&self, offset: usize) -> (usize, usize) {
        let mut remaining = offset;
        for (index, length) in self.lengths.iter().copied().enumerate() {
            if remaining <= length || index + 1 == self.lengths.len() {
                return (index, remaining.min(length));
            }
            remaining -= length + 1;
        }
        (0, 0)
    }

    /// The UTF-16 offset of the start of the paragraph `index`.
    fn start(&self, index: usize) -> usize {
        self.lengths[..index].iter().sum::<usize>() + index
    }

    fn snapshots(&self, range: Range<usize>) -> Vec<ParagraphSnapshot> {
        self.paragraphs[range]
            .iter()
            .map(|paragraph| ParagraphSnapshot::from_paragraph(paragraph))
            .collect()
    }
}

/// A replacement of text by a fragment, read a paragraph at a time (F1c).
struct ParagraphEdit {
    range: Range<usize>,
    /// The fragment keeps the identities of its link fields (no new ones are allocated).
    inherited: bool,
    alignment_source: ParagraphAlignmentSource,
    paired_tracking: bool,
}

/// The paragraphs from `start` of a document read a paragraph at a time (`old`, their
/// snapshots) and the paragraphs that take their place (`new`) (F1c).
struct ParagraphChange<'a> {
    start: usize,
    old: &'a [ParagraphSnapshot],
    new: &'a [ParagraphSnapshot],
}

/// Some consecutive blocks of the tree and the paragraphs that take their place (F1c).
struct BlockReplacement<'a> {
    /// The index of the first block replaced (or of the insertion).
    start: usize,
    removed: &'a [Ref<Block>],
    inserted: &'a [ParagraphSnapshot],
    /// The text of the tree after the replacement, when the observed text gives it.
    text: Option<String>,
}

fn insertion_context(inlines: &[InlineSnapshot], offset: usize) -> Vec<InlineSnapshot> {
    let mut position = 0;
    for inline in inlines {
        let end = position + inline.text_length();
        if offset <= end && (end > position || offset == position) {
            if let Some(children) = inline.children() {
                let inner = insertion_context(children, offset.saturating_sub(position));
                let mut result = vec![inline.clone()];
                if let Some(children) = result[0].children_mut() {
                    children.clear();
                }
                result.extend(inner);
                return result;
            }
            return Vec::new();
        }
        position = end;
    }
    Vec::new()
}

fn format_inlines(
    inlines: &[InlineSnapshot],
    range: Range<usize>,
    format: &CharacterFormat,
) -> Vec<InlineSnapshot> {
    let mut result = Vec::new();
    let mut offset = 0;
    for inline in inlines {
        let length = inline.text_length();
        let end = offset + length;
        if range.start >= end || range.end <= offset {
            result.push(inline.clone());
        } else {
            let selected = range.start.saturating_sub(offset)..range.end.min(end) - offset;
            if let Some(children) = inline.children() {
                let mut container = inline.clone();
                if selected.start == 0 && selected.end == length {
                    container.format = container.format.overlay(format);
                }
                if let Some(target) = container.children_mut() {
                    *target = format_inlines(children, selected, format);
                }
                result.push(container);
            } else {
                if let Some(before) = inline.slice(0..selected.start) {
                    result.push(before);
                }
                if let Some(mut middle) = inline.slice(selected.clone()) {
                    middle.format = middle.format.overlay(format);
                    result.push(middle);
                }
                if let Some(after) = inline.slice(selected.end..length) {
                    result.push(after);
                }
            }
        }
        offset = end;
    }
    result
}

fn find_inline(
    collection: &InlineCollection,
    offset: usize,
    gravity: TextGravity,
) -> Option<Ref<Inline>> {
    let inlines = collection.snapshot();
    let mut position = 0;
    for (index, inline) in inlines.iter().enumerate() {
        let length = InlineSnapshot::from_inline(inline).text_length();
        let end = position + length;
        if offset < end
            || (offset == end && gravity == TextGravity::Backward)
            || index + 1 == inlines.len()
        {
            if let Some(span) = inline.cast::<Span>() {
                return find_inline(
                    &span.inlines(),
                    offset.saturating_sub(position).min(length),
                    gravity,
                )
                .or_else(|| Some(inline.clone()));
            }
            return Some(inline.clone());
        }
        position = end;
    }
    None
}

fn validate_control_parents(
    paragraphs: &[ParagraphSnapshot],
    current: &HashSet<usize>,
) -> Result<(), DocumentError> {
    let mut pending: Vec<&InlineSnapshot> =
        paragraphs.iter().flat_map(|p| p.inlines.iter()).collect();
    while let Some(inline) = pending.pop() {
        if let InlineSnapshotKind::Embedded(Some(control)) = &inline.kind {
            if control.parent().is_some()
                && !current.contains(&(&**control as *const Control as usize))
            {
                return Err(DocumentError::EmbeddedControlAlreadyParented);
            }
        }
        if let Some(children) = inline.children() {
            pending.extend(children);
        }
    }
    Ok(())
}

struct RunLocation {
    run: Ref<Run>,
    collection: InlineCollection,
    index: usize,
    offset: usize,
}

fn find_run_in_collection(
    collection: &InlineCollection,
    range: &Range<usize>,
    mut offset: usize,
) -> Option<RunLocation> {
    for (index, inline) in collection.snapshot().iter().enumerate() {
        let length = if let Some(run) = inline.cast::<Run>() {
            let length = run.text().unwrap_or_default().encode_utf16().count();
            if offset <= range.start && range.end <= offset + length {
                return Some(RunLocation {
                    run,
                    collection: collection.clone(),
                    index,
                    offset,
                });
            }
            length
        } else if let Some(span) = inline.cast::<Span>() {
            if let Some(found) = find_run_in_collection(&span.inlines(), range, offset) {
                return Some(found);
            }
            span.inlines()
                .text()
                .unwrap_or_default()
                .encode_utf16()
                .count()
        } else {
            let mut text = String::new();
            inline.append_text(&mut text);
            text.encode_utf16().count()
        };
        offset += length;
    }
    None
}

struct InlineEntry {
    value: Ref<Inline>,
    snapshot: InlineSnapshot,
    used: bool,
}
struct ParagraphEntry {
    value: Ref<Paragraph>,
    snapshot: ParagraphSnapshot,
    used: bool,
}
struct ContentPool {
    paragraphs: Vec<ParagraphEntry>,
    inlines: Vec<InlineEntry>,
    /// Where the next lookups start: a rebuild meets the pooled elements mostly in their old
    /// order, so the entry after the last one taken usually matches (F1b, #243 review S9:
    /// no scan of the pool per element).
    next_paragraph: usize,
    next_inline: usize,
}

impl ContentPool {
    /// The elements of the paragraphs among `blocks` (the blocks an installation replaces).
    fn capture_blocks(blocks: &[Ref<Block>]) -> Self {
        let mut pool = Self {
            paragraphs: Vec::new(),
            inlines: Vec::new(),
            next_paragraph: 0,
            next_inline: 0,
        };
        for paragraph in blocks.iter().filter_map(|block| block.cast::<Paragraph>()) {
            pool.capture_inlines(&paragraph.inlines());
            let snapshot = ParagraphSnapshot::from_paragraph(&paragraph);
            pool.paragraphs.push(ParagraphEntry {
                value: paragraph,
                snapshot,
                used: false,
            });
        }
        pool
    }

    fn capture_inlines(&mut self, collection: &InlineCollection) {
        for inline in collection.snapshot().iter() {
            let snapshot = InlineSnapshot::from_inline(inline);
            self.inlines.push(InlineEntry {
                value: inline.clone(),
                snapshot,
                used: false,
            });
            if let Some(span) = inline.cast::<Span>() {
                self.capture_inlines(&span.inlines());
            }
        }
    }

    /// The embedded controls of the pooled elements, by identity.
    fn controls(&self) -> HashSet<usize> {
        self.inlines
            .iter()
            .filter_map(|entry| {
                if let InlineSnapshotKind::Embedded(Some(control)) = &entry.snapshot.kind {
                    Some(&**control as *const Control as usize)
                } else {
                    None
                }
            })
            .collect()
    }

    fn detach_contents(&self) {
        for paragraph in &self.paragraphs {
            paragraph.value.inlines().clear();
        }
        for inline in &self.inlines {
            if let Some(span) = inline.value.cast::<Span>() {
                span.inlines().clear();
            }
            if let Some(container) = inline.value.cast::<InlineUIContainer>() {
                container.set_child(None::<Ref<Control>>);
            }
        }
    }

    fn build_paragraph(&mut self, snapshot: &ParagraphSnapshot) -> Ref<Paragraph> {
        let hint = self.next_paragraph;
        let index = self
            .paragraphs
            .get(hint)
            .filter(|entry| !entry.used && entry.snapshot == *snapshot)
            .map(|_| hint)
            .or_else(|| {
                self.paragraphs
                    .iter()
                    .position(|entry| !entry.used && entry.snapshot == *snapshot)
            })
            .or_else(|| self.paragraphs.iter().position(|entry| !entry.used));
        let paragraph = if let Some(index) = index {
            self.next_paragraph = index + 1;
            self.paragraphs[index].used = true;
            self.paragraphs[index].value.clone()
        } else {
            Paragraph::new()
        };
        snapshot.format.replace_on(&paragraph);
        for inline in &snapshot.inlines {
            paragraph.inlines().add(self.build_inline(inline));
        }
        paragraph
    }

    fn build_inline(&mut self, snapshot: &InlineSnapshot) -> Ref<Inline> {
        let hint = self.next_inline;
        let index = self
            .inlines
            .get(hint)
            .filter(|entry| !entry.used && entry.snapshot == *snapshot)
            .map(|_| hint)
            .or_else(|| {
                self.inlines
                    .iter()
                    .position(|entry| !entry.used && entry.snapshot == *snapshot)
            })
            .or_else(|| {
                self.inlines.iter().position(|entry| {
                    !entry.used && compatible(&entry.snapshot.kind, &snapshot.kind)
                })
            });
        let inline = if let Some(index) = index {
            self.next_inline = index + 1;
            self.inlines[index].used = true;
            self.inlines[index].value.clone()
        } else {
            match &snapshot.kind {
                InlineSnapshotKind::Run(_) => Run::new().upcast(),
                InlineSnapshotKind::Span(_) => Span::new().upcast(),
                InlineSnapshotKind::Bold(_) => Bold::new().upcast(),
                InlineSnapshotKind::Italic(_) => Italic::new().upcast(),
                InlineSnapshotKind::Underline(_) => Underline::new().upcast(),
                InlineSnapshotKind::Hyperlink { behavior, .. } => {
                    Hyperlink::with_snapshot_behavior(behavior).upcast()
                }
                InlineSnapshotKind::LineBreak => LineBreak::new().upcast(),
                InlineSnapshotKind::Embedded(_) => InlineUIContainer::new().upcast(),
            }
        };
        match &snapshot.kind {
            InlineSnapshotKind::Run(text) => {
                if let Some(run) = inline.cast::<Run>() {
                    run.set_text(Some(text));
                }
            }
            InlineSnapshotKind::Embedded(control) => {
                if let Some(container) = inline.cast::<InlineUIContainer>() {
                    container.set_child(control.clone());
                }
            }
            InlineSnapshotKind::Hyperlink {
                field_id,
                navigate_uri,
                target_name,
                is_enabled,
                is_tab_stop,
                tab_index,
                is_visited,
                underline_style,
                ..
            } => {
                if let Some(link) = inline.cast::<Hyperlink>() {
                    link.set_field_id(*field_id);
                    link.set_navigate_uri(navigate_uri.clone());
                    link.set_target_name(target_name.clone());
                    link.set_is_enabled(*is_enabled);
                    link.set_is_tab_stop(*is_tab_stop);
                    link.set_tab_index(*tab_index);
                    link.set_is_visited(*is_visited);
                    link.set_underline_style(*underline_style);
                }
            }
            _ => {}
        }
        // Restore assigned properties exactly, then reinstate unspecified
        // semantic defaults at their constructor's current-value priority.
        // Materializing them as local values would change the next snapshot
        // and prevent a subsequent application style from overriding them.
        let format = &snapshot.format;
        format.replace_on(&inline);
        match &snapshot.kind {
            InlineSnapshotKind::Bold(_) if format.font_weight.is_none() => {
                inline.set_current_value(
                    TextElement::font_weight_property(),
                    ferroui_base::media::FontWeight::Bold,
                );
            }
            InlineSnapshotKind::Italic(_) if format.font_style.is_none() => {
                inline.set_current_value(
                    TextElement::font_style_property(),
                    ferroui_base::media::FontStyle::Italic,
                );
            }
            InlineSnapshotKind::Underline(_) if format.text_decorations.is_none() => {
                inline.set_current_value(
                    Inline::text_decorations_property(),
                    Some(ferroui_base::media::TextDecorations::underline()),
                );
            }
            InlineSnapshotKind::Hyperlink {
                underline_style, ..
            } if format.text_decorations.is_none() => {
                inline.set_current_value(
                    Inline::text_decorations_property(),
                    (*underline_style != super::UnderlineStyle::None)
                        .then(ferroui_base::media::TextDecorations::underline),
                );
            }
            _ => {}
        }
        if let Some(children) = snapshot.children() {
            if let Some(span) = inline.cast::<Span>() {
                for child in children {
                    span.inlines().add(self.build_inline(child));
                }
            }
        }
        inline
    }
}

fn compatible(a: &InlineSnapshotKind, b: &InlineSnapshotKind) -> bool {
    match (a, b) {
        (
            InlineSnapshotKind::Hyperlink {
                field_id: ai,
                behavior: ab,
                navigate_uri: a,
                target_name: at,
                ..
            },
            InlineSnapshotKind::Hyperlink {
                field_id: bi,
                behavior: bb,
                navigate_uri: b,
                target_name: bt,
                ..
            },
        ) => match (ai, bi) {
            (Some(a), Some(b)) => a == b,
            (None, None) => a == b && at == bt,
            _ => ab.shares_handlers(bb),
        },
        (InlineSnapshotKind::Embedded(a), InlineSnapshotKind::Embedded(b)) => a == b,
        _ => std::mem::discriminant(a) == std::mem::discriminant(b),
    }
}

#[cfg(test)]
#[path = "rich_text_structural_edit_tests.rs"]
mod structural_edit_tests;
