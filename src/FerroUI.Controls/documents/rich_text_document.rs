use super::document_snapshot::{normalize_paragraph_breaks, slice_inlines, utf16_to_byte};
use super::{
    BlockCollection, Bold, CharacterFormat, DocumentLinkField, DocumentSnapshot, Hyperlink,
    IInlineHost, Inline, InlineCollection, InlineSnapshot, InlineSnapshotKind, InlineUIContainer,
    Italic, LineBreak, Paragraph, ParagraphFormat, ParagraphSnapshot, Run, Span, TextElement,
    TextGravity, TextPointer, TextPosition, TextRange, Underline,
};
use crate::Control;
use ferroui_base::media::text_formatting::unicode::{GraphemeEnumerator, WordBreakEnumerator};
use ferroui_base::reactive::{Disposable, IDisposable};
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
    observed_text: RefCell<Rc<str>>,
    text_changes: RefCell<Vec<DocumentTextChange>>,
    installing: Cell<bool>,
    delivering: Cell<bool>,
    pending_events: RefCell<VecDeque<DocumentChangedEventArgs>>,
    edit_depth: Cell<usize>,
    silent_depth: Cell<usize>,
    dirty: Cell<bool>,
    change_kind: Cell<Option<DocumentChangeKind>>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
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
            cached_text: RefCell::new(text.clone()),
            observed_text: RefCell::new(text),
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
            self.0.cached_text.borrow().encode_utf16().count()
        }
    }

    pub fn changed(
        &self,
        handler: impl Fn(&DocumentChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.0.changed.add(Rc::new(handler));
        let weak = Rc::downgrade(&self.0.changed);
        Disposable::create(move || {
            if let Some(handlers) = weak.upgrade() {
                handlers.remove(token);
            }
        })
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
        validate_text_range(&self.text(), range)
    }

    pub fn get_text(&self, range: Range<usize>) -> Result<String, DocumentError> {
        let text = self.text();
        validate_text_range(&text, range.clone())?;
        let start =
            utf16_to_byte(&text, range.start).ok_or(DocumentError::InvalidUtf16Boundary {
                offset: range.start,
            })?;
        let end = utf16_to_byte(&text, range.end)
            .ok_or(DocumentError::InvalidUtf16Boundary { offset: range.end })?;
        Ok(text[start..end].to_owned())
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
        let normalized = normalize_paragraph_breaks(text);
        *self.0.pending_edit.borrow_mut() = Some((range.clone(), normalized.clone()));
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
                found.run.set_text(Some(&value));
                if tracking
                    .as_ref()
                    .is_some_and(|scope| scope.was_interrupted())
                {
                    return Err(DocumentError::EditInProgress);
                }
                return Ok(());
            }
        }
        let snapshot = self.snapshot();
        let (paragraph_index, offset) = snapshot.locate(range.start);
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
        if let Some(paragraph) = snapshot.paragraphs.get(paragraph_index) {
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
        } else {
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
        let first = &snapshot.paragraphs[start_index];
        let last = &snapshot.paragraphs[end_index];
        let prefix = slice_inlines(&first.inlines, 0..start_offset);
        let mut suffix = slice_inlines(&last.inlines, end_offset..last.text_length());
        if start_index != end_index && first.format.character != last.format.character {
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
        if alignment_source == ParagraphAlignmentSource::Trailing && start_index != end_index {
            // A typed break creates another paragraph in the merged context;
            // every inserted paragraph therefore receives the same alignment.
            // Preserve None exactly so inherited alignment remains inherited.
            for paragraph in &mut inserted {
                paragraph.format.text_alignment = last.format.text_alignment;
            }
        }
        snapshot
            .paragraphs
            .splice(start_index..=end_index, inserted);
        let tracked = paired_tracking.then(|| (range.clone(), fragment.text()));
        snapshot.replace_link_fields(&before, range, &fragment, next)?;
        self.install_with_range_change(&snapshot, DocumentChangeKind::Content, tracked)
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
        let mut snapshot = self.snapshot();
        let mut offset = 0;
        for paragraph in &mut snapshot.paragraphs {
            let length = paragraph.text_length();
            if range.start < offset + length && range.end > offset {
                paragraph.inlines = format_inlines(
                    &paragraph.inlines,
                    range.start.saturating_sub(offset)..range.end.min(offset + length) - offset,
                    format,
                );
            } else if length == 0 && range.start <= offset && range.end > offset {
                paragraph.format.character = paragraph.format.character.overlay(format);
            }
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
        let mut pool = ContentPool::capture(&self.blocks());
        let current_controls: HashSet<usize> = pool
            .inlines
            .iter()
            .filter_map(|entry| {
                if let InlineSnapshotKind::Embedded(Some(control)) = &entry.snapshot.kind {
                    Some(&**control as *const Control as usize)
                } else {
                    None
                }
            })
            .collect();
        validate_control_parents(snapshot, &current_controls)?;
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
        self.blocks().clear();
        pool.detach_contents();
        for paragraph in &snapshot.paragraphs {
            let value = pool.build_paragraph(paragraph);
            self.blocks().add(value);
        }
        *self.0.link_fields.borrow_mut() = snapshot.link_fields.clone();
        self.0.next_link_field_id.set(snapshot.next_link_field_id);
        self.0.link_allocation_floor.set(
            self.link_allocation_floor()
                .max(snapshot.next_link_field_id),
        );
        self.0.installed_field_snapshot.set(true);
        self.0.installing.set(false);
        let tracking = range_change.and_then(|(range, inserted, handlers)| {
            self.begin_range_text_change(range, inserted, handlers)
        });
        self.on_invalidated();
        if tracking
            .as_ref()
            .is_some_and(|scope| scope.was_interrupted())
        {
            return Err(DocumentError::EditInProgress);
        }
        Ok(())
    }

    fn ensure_install_read(&self, version: u64) -> Result<(), DocumentError> {
        if self.mutation_version() == version {
            Ok(())
        } else {
            Err(DocumentError::EditInProgress)
        }
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
        let text = self.0.blocks.text();
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
        let new_text = self.0.blocks.text();
        let old_text = self.0.cached_text.replace(Rc::from(new_text.as_str()));
        let old_length = old_text.encode_utf16().count();
        let new_length = new_text.encode_utf16().count();
        let (range, inserted_length) = changed_range(&old_text, &new_text);
        let requested_kind = self.0.change_kind.take();
        let kind = if old_text.as_ref() != new_text {
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

fn validate_text_range(text: &str, range: Range<usize>) -> Result<(), DocumentError> {
    if range.start > range.end {
        return Err(DocumentError::ReversedRange {
            start: range.start,
            end: range.end,
        });
    }
    let length = text.encode_utf16().count();
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
fn explained_change(before: &str, after: &str, range: &Range<usize>, inserted: &str) -> Option<(Range<usize>, usize)> {
    let start = utf16_to_byte(before, range.start)?;
    let end = utf16_to_byte(before, range.end)?;
    let explains = after.len() == before.len() - (end - start) + inserted.len()
        && after.starts_with(&before[..start])
        && after[start..].starts_with(inserted)
        && after.ends_with(&before[end..]);
    explains.then(|| (range.clone(), inserted.encode_utf16().count()))
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
    snapshot: &DocumentSnapshot,
    current: &HashSet<usize>,
) -> Result<(), DocumentError> {
    let mut pending: Vec<&InlineSnapshot> = snapshot
        .paragraphs
        .iter()
        .flat_map(|p| p.inlines.iter())
        .collect();
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
}

impl ContentPool {
    fn capture(blocks: &BlockCollection) -> Self {
        let mut pool = Self {
            paragraphs: Vec::new(),
            inlines: Vec::new(),
        };
        for paragraph in blocks
            .snapshot()
            .iter()
            .filter_map(|block| block.cast::<Paragraph>())
        {
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
        let index = self
            .paragraphs
            .iter()
            .position(|entry| !entry.used && entry.snapshot == *snapshot)
            .or_else(|| self.paragraphs.iter().position(|entry| !entry.used));
        let paragraph = if let Some(index) = index {
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
        let index = self
            .inlines
            .iter()
            .position(|entry| !entry.used && entry.snapshot == *snapshot)
            .or_else(|| {
                self.inlines.iter().position(|entry| {
                    !entry.used && compatible(&entry.snapshot.kind, &snapshot.kind)
                })
            });
        let inline = if let Some(index) = index {
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
