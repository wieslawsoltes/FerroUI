use super::i_active_element::ActiveElementRegistration;
use super::snippet::{FieldTransform, Placeholder};
use super::{
    AnchorElement, DeactivateReason, IActiveElement, IReplaceableActiveElement, SnippetElementRef,
    SnippetError, SnippetEventArgs, SnippetTextTransform,
};
use crate::document::{
    AnchorMovementType, AnchorSegment, DocumentChangeEventArgs, DocumentEdit, DocumentError,
    DocumentSubscription, DocumentVersion, IUndoableOperation, SimpleSegment, TextAnchor,
    TextDocument,
};
use crate::utils::Event;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::{Rc, Weak};

type InsertionHandler = Rc<dyn Fn(&SnippetSession)>;
type DeactivationHandler = Rc<dyn Fn(&SnippetSession, &SnippetEventArgs)>;
#[derive(Default)]
pub(crate) struct SnippetLifecycle {
    pub insertion_completed: Vec<InsertionHandler>,
    pub deactivated: Vec<DeactivationHandler>,
}

pub type SnippetEditValidator = dyn Fn(&[DocumentEdit]) -> Result<(), DocumentError>;

/// Current anchored ranges belonging to one built-in replaceable field.
/// Ranges use document UTF-16 offsets. Bound occurrences retain their insertion
/// order and may have different lengths after a text transformation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnippetField {
    pub id: usize,
    pub segment: SimpleSegment,
    pub bound_segments: Vec<SimpleSegment>,
}

struct ActiveElement {
    id: usize,
    anchors: RefCell<AnchorSegment>,
    editable: bool,
    choices: Option<Rc<[String]>>,
    enabled: Cell<bool>,
    parent: Option<usize>,
    transform: Option<FieldTransform>,
    text_changed: Event<()>,
    cached_text: RefCell<String>,
}
impl IActiveElement for ActiveElement {
    fn is_editable(&self) -> bool {
        self.editable && self.enabled.get()
    }
    fn segment(&self) -> Option<SimpleSegment> {
        self.segment()
    }
}
impl IReplaceableActiveElement for ActiveElement {
    fn text(&self) -> Result<String, SnippetError> {
        let segment = self.segment().ok_or(DocumentError::DeletedAnchor)?;
        Ok(self
            .anchors
            .borrow()
            .start
            .document()
            .get_text(segment.offset, segment.length)?)
    }
    fn text_changed(&self, handler: Rc<dyn Fn()>) -> DocumentSubscription {
        self.text_changed.subscribe(move |_| handler())
    }
}
impl ActiveElement {
    fn segment(&self) -> Option<SimpleSegment> {
        if !self.enabled.get() {
            return None;
        }
        let anchors = self.anchors.borrow();
        let start = anchors.start.offset().ok()?;
        let end = anchors.end.offset().ok()?;
        (end >= start).then(|| SimpleSegment::new(start, end - start))
    }

    fn set_movement(&self, start: AnchorMovementType, end: AnchorMovementType) {
        let anchors = self.anchors.borrow();
        anchors.start.set_movement_type(start);
        anchors.end.set_movement_type(end);
    }
}
struct SessionInner {
    document: TextDocument,
    elements: Vec<Rc<ActiveElement>>,
    tab_order: Vec<usize>,
    mirror_order: Option<Vec<usize>>,
    named_anchors: RefCell<Vec<Rc<AnchorElement>>>,
    custom_elements: RefCell<Vec<(SnippetElementRef, Rc<dyn IActiveElement>)>>,
    custom_tab_order: RefCell<Vec<usize>>,
    active: Cell<Option<usize>>,
    owner: Cell<Option<usize>>,
    caret: TextAnchor,
    caret_order: usize,
    closed: Cell<bool>,
    completed: Cell<bool>,
    deactivation_reason: Cell<Option<DeactivateReason>>,
    whole_range: AnchorSegment,
    deactivate_when_empty: Cell<bool>,
    deactivated: Event<(SnippetSession, SnippetEventArgs)>,
    syncing: Cell<bool>,
    subscriptions: RefCell<Vec<DocumentSubscription>>,
    error: RefCell<Option<String>>,
    edit_validator: RefCell<Option<Rc<SnippetEditValidator>>>,
}

// A mirror inside an editable field contributes to that field's own mirrors.
// Visit the source of the inner mirror first, independently of Tab order.
fn mirror_order(elements: &[Rc<ActiveElement>], tab_order: &[usize]) -> Option<Vec<usize>> {
    if !elements
        .iter()
        .any(|element| !element.editable && element.parent.is_some())
    {
        return None;
    }
    let by_id: BTreeMap<_, _> = tab_order
        .iter()
        .enumerate()
        .map(|(rank, &index)| (elements[index].id, rank))
        .collect();
    let mut successors = vec![BTreeSet::new(); tab_order.len()];
    let mut incoming = vec![0; tab_order.len()];
    for (index, element) in elements.iter().enumerate() {
        let Some(&source) = by_id.get(&element.id) else {
            continue;
        };
        if tab_order[source] == index {
            continue;
        }
        let mut parent = element.parent;
        while let Some(index) = parent {
            let ancestor = &elements[index];
            if let Some(&dependent) = by_id.get(&ancestor.id) {
                if tab_order[dependent] == index
                    && dependent != source
                    && successors[source].insert(dependent)
                {
                    incoming[dependent] += 1;
                }
            }
            parent = ancestor.parent;
        }
    }
    let mut ready: BTreeSet<_> = (0..tab_order.len())
        .filter(|&rank| incoming[rank] == 0)
        .collect();
    let mut ordered = Vec::with_capacity(tab_order.len());
    while let Some(source) = ready.pop_first() {
        ordered.push(tab_order[source]);
        for &dependent in &successors[source] {
            incoming[dependent] -= 1;
            if incoming[dependent] == 0 {
                ready.insert(dependent);
            }
        }
    }
    (ordered.len() == tab_order.len()).then_some(ordered)
}
impl Drop for SessionInner {
    fn drop(&mut self) {
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
        if !self.closed.get() {
            let args = SnippetEventArgs::new(DeactivateReason::InputHandlerDetached);
            for (_, element) in self.custom_elements.get_mut().iter() {
                element.deactivate(&args);
            }
        }
    }
}

struct RestoreOwner {
    session: Weak<SessionInner>,
    owner: usize,
}
impl RestoreOwner {
    fn restore(&self) {
        if let Some(inner) = self.session.upgrade() {
            let session = SnippetSession { inner };
            if !session.is_closed() {
                session.set_owner(self.owner);
            }
        }
    }
}
impl IUndoableOperation for RestoreOwner {
    fn undo(&self) {
        self.restore();
    }
    fn redo(&self) {
        self.restore();
    }
}

struct DeactivateOnInsertionUndo(WeakSnippetSession);
impl IUndoableOperation for DeactivateOnInsertionUndo {
    fn undo(&self) {
        if let Some(session) = self.0.upgrade() {
            session.deactivate(DeactivateReason::Deleted);
        }
    }
    fn redo(&self) {}
}

struct RestoreNestedFields {
    session: WeakSnippetSession,
    removed: Vec<(usize, SimpleSegment)>,
}
impl IUndoableOperation for RestoreNestedFields {
    fn undo(&self) {
        let Some(session) = self
            .session
            .upgrade()
            .filter(|session| !session.is_closed())
        else {
            return;
        };
        for &(index, range) in &self.removed {
            let field = &session.inner.elements[index];
            if let Ok(anchors) =
                AnchorSegment::new(&session.inner.document, range.offset, range.length)
            {
                *field.anchors.borrow_mut() = anchors;
                field.enabled.set(true);
            }
        }
        session.restore_active_owner();
    }

    fn redo(&self) {
        let Some(session) = self
            .session
            .upgrade()
            .filter(|session| !session.is_closed())
        else {
            return;
        };
        for &(index, _) in &self.removed {
            session.inner.elements[index].enabled.set(false);
        }
        session.restore_active_owner();
    }
}

/// A linked-placeholder insertion context. Edits through any host to the active
/// editable element propagate to its bound occurrences after the document's
/// transaction completes. The mirror edits continue the original undo group.
#[derive(Clone)]
pub struct SnippetSession {
    inner: Rc<SessionInner>,
}

struct MirrorWrite<'a>(&'a SnippetSession);

impl Drop for MirrorWrite<'_> {
    fn drop(&mut self) {
        self.0.restore_active_owner();
        self.0.inner.syncing.set(false);
    }
}
#[derive(Clone)]
pub struct WeakSnippetSession(Weak<SessionInner>);
impl WeakSnippetSession {
    pub fn upgrade(&self) -> Option<SnippetSession> {
        self.0.upgrade().map(|inner| SnippetSession { inner })
    }
}

impl SnippetSession {
    pub fn downgrade(&self) -> WeakSnippetSession {
        WeakSnippetSession(Rc::downgrade(&self.inner))
    }
    pub(crate) fn track_insertion_undo(&self) -> Result<(), SnippetError> {
        // A host may atomically replace the selection with the expansion.
        // Restoring that selection can leave every anchor at a nonempty
        // range, so range deletion alone cannot identify insertion undo.
        if !self.is_closed()
            && self.inner.document.is_in_update()
            && self.inner.document.accept_changes()
        {
            self.inner
                .document
                .undo_stack()
                .push_optional(Rc::new(DeactivateOnInsertionUndo(self.downgrade())))?;
        }
        Ok(())
    }
    /// Links existing document ranges without inserting or replacing text.
    /// Subsequent edits of the primary range update every bound range in the
    /// same undo group. All ranges are validated before anchors are created.
    pub fn link(
        document: TextDocument,
        main: SimpleSegment,
        bound: &[SimpleSegment],
    ) -> Result<Self, SnippetError> {
        let bound: Vec<_> = bound.iter().map(|range| (*range, None)).collect();
        Self::link_with_transforms(document, main, &bound)
    }
    pub fn link_with_transforms(
        document: TextDocument,
        main: SimpleSegment,
        bound: &[(SimpleSegment, Option<SnippetTextTransform>)],
    ) -> Result<Self, SnippetError> {
        let mut ranges = vec![(main, true, None)];
        ranges.extend(
            bound
                .iter()
                .map(|(range, transform)| (*range, false, transform.clone())),
        );
        for (range, _, _) in &ranges {
            if range.offset > document.text_length()
                || range.length > document.text_length() - range.offset
            {
                return Err(DocumentError::InvalidRange {
                    offset: range.offset,
                    length: range.length,
                    text_length: document.text_length(),
                }
                .into());
            }
        }
        ranges.sort_by_key(|(range, _, _)| range.offset);
        for pair in ranges.windows(2) {
            if pair[0].0.end_offset() > pair[1].0.offset {
                return Err(DocumentError::OverlappingEdits.into());
            }
        }
        let start = ranges[0].0.offset;
        let end = ranges
            .iter()
            .map(|(range, _, _)| range.end_offset())
            .max()
            .unwrap();
        let count = ranges.len();
        let placeholders = ranges
            .into_iter()
            .map(|(range, editable, transform)| Placeholder {
                id: 1,
                start: range.offset - start,
                end: range.end_offset() - start,
                editable,
                choices: None,
                transform: transform.map(FieldTransform::Live),
                parent: None,
            })
            .collect();
        Self::new(
            document,
            start,
            placeholders,
            end - start,
            count,
            end - start,
            SnippetLifecycle::default(),
            Vec::new(),
            Vec::new(),
            None,
        )
    }
    pub(crate) fn new(
        document: TextDocument,
        offset: usize,
        placeholders: Vec<Placeholder>,
        caret: usize,
        caret_order: usize,
        length: usize,
        lifecycle: SnippetLifecycle,
        anchors: Vec<(String, usize)>,
        registrations: Vec<ActiveElementRegistration>,
        edit_validator: Option<Rc<SnippetEditValidator>>,
    ) -> Result<Self, SnippetError> {
        let mut elements = Vec::new();
        let mut order = BTreeMap::new();
        for placeholder in placeholders {
            let anchors = AnchorSegment::new(
                &document,
                offset + placeholder.start,
                placeholder.end - placeholder.start,
            )?;
            if placeholder.editable {
                order.entry(placeholder.id).or_insert(elements.len());
            }
            let cached_text = document.get_text(
                offset + placeholder.start,
                placeholder.end - placeholder.start,
            )?;
            elements.push(Rc::new(ActiveElement {
                id: placeholder.id,
                anchors: RefCell::new(anchors),
                editable: placeholder.editable,
                choices: placeholder.choices,
                enabled: Cell::new(true),
                parent: placeholder.parent,
                transform: placeholder.transform,
                text_changed: Event::default(),
                cached_text: RefCell::new(cached_text),
            }));
        }
        let caret = document.create_anchor(offset + caret)?;
        caret.set_movement_type(AnchorMovementType::AfterInsertion);
        caret.set_survive_deletion(true);
        let tab_order: Vec<_> = order.into_values().collect();
        let mirror_order = mirror_order(&elements, &tab_order);
        let active = (!tab_order.is_empty()).then_some(0);
        let owner = tab_order.first().copied();
        let whole_range = AnchorSegment::new(&document, offset, length)?;
        whole_range
            .start
            .set_movement_type(AnchorMovementType::BeforeInsertion);
        whole_range
            .end
            .set_movement_type(AnchorMovementType::AfterInsertion);
        let inner = Rc::new(SessionInner {
            document: document.clone(),
            elements,
            tab_order,
            mirror_order,
            named_anchors: RefCell::new(Vec::new()),
            custom_elements: RefCell::new(Vec::new()),
            custom_tab_order: RefCell::new(Vec::new()),
            active: Cell::new(active),
            owner: Cell::new(owner),
            caret,
            caret_order,
            closed: Cell::new(false),
            completed: Cell::new(false),
            deactivation_reason: Cell::new(None),
            whole_range,
            deactivate_when_empty: Cell::new(length != 0),
            deactivated: Event::default(),
            syncing: Cell::new(false),
            subscriptions: RefCell::new(Vec::new()),
            error: RefCell::new(None),
            edit_validator: RefCell::new(edit_validator),
        });
        let session = Self { inner };
        for handler in lifecycle.deactivated {
            session
                .deactivated(move |context, args| handler(context, args))
                .detach();
        }
        for (name, position) in anchors {
            let anchor = Rc::new(AnchorElement::new(&session, name, offset + position)?);
            session.inner.named_anchors.borrow_mut().push(anchor);
        }
        for registration in registrations {
            let version = document.version();
            let element = match (registration.factory)(&session) {
                Ok(element) => element,
                Err(error) => {
                    session.deactivate(DeactivateReason::Error);
                    return Err(error);
                }
            };
            if document.version() != version || session.is_closed() {
                element.deactivate(&SnippetEventArgs::new(DeactivateReason::Error));
                session.deactivate(DeactivateReason::Error);
                return Err(SnippetError::InvalidExpansion(
                    "active factories cannot edit or deactivate the insertion context",
                ));
            }
            if let Some(range) = element.segment() {
                if range.offset > document.text_length()
                    || range.length > document.text_length() - range.offset
                {
                    element.deactivate(&SnippetEventArgs::new(DeactivateReason::Error));
                    session.deactivate(DeactivateReason::Error);
                    return Err(SnippetError::InvalidExpansion(
                        "custom active segment is outside the document",
                    ));
                }
            }
            let editable = element.is_editable() && element.segment().is_some();
            let index = session.inner.custom_elements.borrow().len();
            session
                .inner
                .custom_elements
                .borrow_mut()
                .push((registration.owner, element));
            if editable {
                session.inner.custom_tab_order.borrow_mut().push(index);
            }
        }
        session
            .inner
            .active
            .set((session.tab_count() != 0).then_some(0));
        session.restore_active_owner();
        let weak = Rc::downgrade(&session.inner);
        let changing = document.changing(move |change| {
            if let Some(inner) = weak.upgrade() {
                let session = SnippetSession { inner };
                session.prepare_edit_owner(change);
                session.record_owner();
            }
        });
        let weak = Rc::downgrade(&session.inner);
        let changed = document.changed(move |_| {
            if let Some(inner) = weak.upgrade() {
                SnippetSession { inner }.record_owner();
            }
        });
        let weak = Rc::downgrade(&session.inner);
        let subscription = document.text_changed(move || {
            if let Some(inner) = weak.upgrade() {
                let session = SnippetSession { inner };
                if session.should_deactivate_after_deletion() {
                    session.deactivate(DeactivateReason::Deleted);
                } else if let Err(error) = session.synchronize() {
                    *session.inner.error.borrow_mut() = Some(error.to_string());
                    session.deactivate(DeactivateReason::Error);
                }
                if !session.inner.syncing.get() && !session.is_closed() {
                    session.publish_field_changes();
                }
            }
        });
        *session.inner.subscriptions.borrow_mut() = vec![changing, changed, subscription];
        for element in session.active_elements() {
            if let Err(error) = element.on_insertion_completed(&session) {
                session.deactivate(DeactivateReason::Error);
                return Err(error);
            }
            if session.is_closed() {
                break;
            }
        }
        session.inner.completed.set(true);
        for handler in lifecycle.insertion_completed {
            handler(&session);
        }
        if session.active_elements().is_empty() {
            session.deactivate(DeactivateReason::NoActiveElements);
        }
        Ok(session)
    }
    /// Returns a snapshot, so extension callbacks never run under a collection borrow.
    pub fn active_elements(&self) -> Vec<Rc<dyn IActiveElement>> {
        let mut elements: Vec<Rc<dyn IActiveElement>> = self
            .inner
            .elements
            .iter()
            .map(|element| element.clone() as Rc<dyn IActiveElement>)
            .collect();
        elements.extend(
            self.inner
                .named_anchors
                .borrow()
                .iter()
                .map(|element| element.clone() as Rc<dyn IActiveElement>),
        );
        elements.extend(
            self.inner
                .custom_elements
                .borrow()
                .iter()
                .map(|(_, element)| element.clone()),
        );
        elements
    }
    pub fn get_active_element(&self, owner: &SnippetElementRef) -> Option<Rc<dyn IActiveElement>> {
        self.inner
            .custom_elements
            .borrow()
            .iter()
            .find(|(key, _)| key == owner)
            .map(|(_, element)| element.clone())
    }
    pub fn replaceable_element(&self, id: usize) -> Option<Rc<dyn IReplaceableActiveElement>> {
        self.inner
            .elements
            .iter()
            .find(|element| element.is_editable() && element.id == id)
            .map(|element| element.clone() as Rc<dyn IReplaceableActiveElement>)
    }
    /// Snapshots built-in replaceable fields in document insertion order, with
    /// the exact bound occurrences belonging to each field identity. Named
    /// anchors and custom active elements are not built-in field decorations.
    /// Closed sessions and ranges whose anchors have been deleted are omitted.
    pub fn fields(&self) -> Vec<SnippetField> {
        if self.is_closed() {
            return Vec::new();
        }
        let mut bound = BTreeMap::<usize, Vec<SimpleSegment>>::new();
        for element in self
            .inner
            .elements
            .iter()
            .filter(|element| !element.editable)
        {
            if let Some(segment) = element.segment() {
                bound.entry(element.id).or_default().push(segment);
            }
        }
        self.inner
            .elements
            .iter()
            .filter(|element| element.editable)
            .filter_map(|element| {
                Some(SnippetField {
                    id: element.id,
                    segment: element.segment()?,
                    bound_segments: bound.get(&element.id).cloned().unwrap_or_default(),
                })
            })
            .collect()
    }
    pub fn named_anchor(&self, name: &str) -> Option<Rc<AnchorElement>> {
        self.inner
            .named_anchors
            .borrow()
            .iter()
            .find(|anchor| anchor.name() == name)
            .cloned()
    }
    fn tab_count(&self) -> usize {
        self.inner.tab_order.len() + self.inner.custom_tab_order.borrow().len()
    }
    fn tab_element(&self, index: usize) -> Option<Rc<dyn IActiveElement>> {
        if let Some(&element) = self.inner.tab_order.get(index) {
            return Some(self.inner.elements[element].clone());
        }
        let index = index.checked_sub(self.inner.tab_order.len())?;
        let custom_index = *self.inner.custom_tab_order.borrow().get(index)?;
        self.inner
            .custom_elements
            .borrow()
            .get(custom_index)
            .map(|(_, element)| element.clone())
    }
    pub fn is_closed(&self) -> bool {
        self.inner.closed.get()
    }
    pub fn is_insertion_completed(&self) -> bool {
        self.inner.completed.get()
    }
    pub fn document(&self) -> TextDocument {
        self.inner.document.clone()
    }
    pub fn segment(&self) -> Option<SimpleSegment> {
        self.inner.whole_range.segment().ok()
    }
    pub fn start_position(&self) -> Option<usize> {
        self.inner.whole_range.offset().ok()
    }
    pub fn deactivation_reason(&self) -> Option<DeactivateReason> {
        self.inner.deactivation_reason.get()
    }
    /// Controls upstream's deletion rule. It is enabled for an initially
    /// nonempty expansion. Hosts retaining empty linked fields may opt out.
    pub fn set_deactivate_when_empty(&self, value: bool) {
        self.inner.deactivate_when_empty.set(value);
    }
    pub fn deactivated(
        &self,
        handler: impl Fn(&SnippetSession, &SnippetEventArgs) + 'static,
    ) -> DocumentSubscription {
        self.inner
            .deactivated
            .subscribe(move |(session, args)| handler(session, args))
    }
    fn should_deactivate_after_deletion(&self) -> bool {
        self.inner.completed.get()
            && !self.is_closed()
            && self.inner.deactivate_when_empty.get()
            && self
                .inner
                .whole_range
                .length()
                .is_ok_and(|length| length == 0)
    }
    pub fn error(&self) -> Option<String> {
        self.inner.error.borrow().clone()
    }
    /// Installs the host's protected-range policy for automatic mirror writes.
    /// A rejected mirror ends the session and is exposed through `error()`;
    /// the user's already accepted edit remains in the document.
    pub fn set_edit_validator(&self, validator: Option<Rc<SnippetEditValidator>>) {
        *self.inner.edit_validator.borrow_mut() = validator;
    }
    pub fn active_segment(&self) -> Option<SimpleSegment> {
        if self.is_closed() {
            return None;
        }
        self.tab_element(self.inner.active.get()?)?.segment()
    }
    pub fn active_placeholder(&self) -> Option<usize> {
        self.inner
            .elements
            .get(self.active_index()?)
            .filter(|element| element.enabled.get())
            .map(|e| e.id)
    }
    /// Choices for the active field in declaration order.
    pub fn active_choices(&self) -> Option<Rc<[String]>> {
        self.inner
            .elements
            .get(self.active_index()?)
            .filter(|element| element.is_editable())?
            .choices
            .clone()
    }
    pub fn caret_offset(&self) -> Option<usize> {
        self.inner.caret.offset().ok()
    }
    pub fn next_element(&self, backwards: bool) -> Option<SimpleSegment> {
        match self.try_next_element(backwards) {
            Ok(segment) => segment,
            Err(error) => {
                *self.inner.error.borrow_mut() = Some(error.to_string());
                None
            }
        }
    }
    /// Applies the current stop's transforms before moving the selection.
    pub fn try_next_element(&self, backwards: bool) -> Result<Option<SimpleSegment>, SnippetError> {
        let Some(mut index) = self.inner.active.get() else {
            return Ok(None);
        };
        let current = index;
        loop {
            let next = if backwards {
                index.saturating_sub(1)
            } else {
                index + 1
            };
            if next >= self.tab_count() {
                self.commit_active_transforms()?;
                self.deactivate(DeactivateReason::Completed);
                return Ok(None);
            }
            if let Some(element) = self.tab_element(next) {
                if element.is_editable() && element.segment().is_some() {
                    if next != current {
                        self.commit_active_transforms()?;
                        if self.is_closed() {
                            return Ok(None);
                        }
                    }
                    self.inner.active.set(Some(next));
                    self.restore_active_owner();
                    return Ok(self.active_segment());
                }
            }
            if next == index {
                return Ok(None);
            }
            index = next;
        }
    }
    /// Positional cyclic traversal corresponding to upstream's input handler.
    /// Numeric `next_element` keeps its additional finish-after-last behavior.
    pub fn next_element_from_offset(
        &self,
        offset: usize,
        backwards: bool,
    ) -> Option<SimpleSegment> {
        match self.try_next_element_from_offset(offset, backwards) {
            Ok(segment) => segment,
            Err(error) => {
                *self.inner.error.borrow_mut() = Some(error.to_string());
                None
            }
        }
    }
    pub fn try_next_element_from_offset(
        &self,
        offset: usize,
        backwards: bool,
    ) -> Result<Option<SimpleSegment>, SnippetError> {
        if self.is_closed() {
            return Ok(None);
        }
        let anchor = self.inner.document.create_anchor(offset).ok();
        if let Some(anchor) = &anchor {
            anchor.set_survive_deletion(true);
        }
        self.commit_active_transforms()?;
        let offset = anchor
            .and_then(|anchor| anchor.offset().ok())
            .unwrap_or(offset);
        let mut candidates = Vec::new();
        for index in 0..self.tab_count() {
            if let Some(element) = self.tab_element(index) {
                if element.is_editable() {
                    if let Some(range) = element.segment() {
                        candidates.push((index, range));
                    }
                }
            }
        }
        candidates.sort_by_key(|(_, range)| range.offset);
        let candidate = if backwards {
            candidates
                .iter()
                .rev()
                .find(|(_, range)| offset > range.end_offset())
                .or_else(|| candidates.last())
        } else {
            candidates
                .iter()
                .find(|(_, range)| offset < range.offset)
                .or_else(|| candidates.first())
        };
        let Some((index, _)) = candidate else {
            return Ok(None);
        };
        self.inner.active.set(Some(*index));
        self.restore_active_owner();
        Ok(self.active_segment())
    }
    pub fn finish(&self) {
        self.deactivate(DeactivateReason::Unknown);
    }
    /// Terminal, idempotent transition. State and document subscriptions are
    /// cleared before application callbacks, allowing safe reentrant teardown.
    pub fn deactivate(&self, reason: DeactivateReason) {
        if self.inner.closed.replace(true) {
            return;
        }
        self.inner.active.set(None);
        self.inner.deactivation_reason.set(Some(reason));
        let subscriptions = std::mem::take(&mut *self.inner.subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
        let args = SnippetEventArgs::new(reason);
        for element in self.active_elements() {
            element.deactivate(&args);
        }
        self.inner.deactivated.raise(&(self.clone(), args));
    }
    pub fn replace_active(&self, text: &str) -> Result<(), SnippetError> {
        let segment = self.active_segment().ok_or(SnippetError::Closed)?;
        let version = self.inner.document.version();
        self.validate_edits(&[DocumentEdit::new(segment.offset, segment.length, text)])?;
        if self.is_closed() {
            return Err(SnippetError::Closed);
        }
        if self.inner.document.version() != version || self.active_segment() != Some(segment) {
            return Err(DocumentError::ReentrantChange.into());
        }
        let _update = self.inner.document.begin_update()?;
        if self.inner.document.version() != version || self.active_segment() != Some(segment) {
            return Err(DocumentError::ReentrantChange.into());
        }
        self.inner
            .document
            .replace(segment.offset, segment.length, text)?;
        Ok(())
    }

    /// Replaces the current choice field while keeping it active. Linked edits
    /// join the same undo group as the chosen value.
    pub fn choose_active(&self, index: usize) -> Result<(), SnippetError> {
        let choices = self
            .active_choices()
            .ok_or(SnippetError::InvalidExpansion("no active snippet choice"))?;
        let value = choices.get(index).ok_or(SnippetError::InvalidExpansion(
            "invalid snippet choice index",
        ))?;
        let active = self.active_index().ok_or(SnippetError::Closed)?;
        let segment = self.active_segment().ok_or(SnippetError::Closed)?;
        if self
            .inner
            .document
            .get_text(segment.offset, segment.length)?
            == *value
        {
            return Ok(());
        }
        let version = self.inner.document.version();
        let primary = DocumentEdit::new(segment.offset, segment.length, value);
        let pending = self.plan_mirror_edits(&version, vec![(active, primary)], None)?;
        self.apply_explicit_edits(&version, active, pending)
    }

    /// Commits the current field's deferred transforms without navigating.
    pub fn commit_active_transforms(&self) -> Result<(), SnippetError> {
        let Some(active) = self.active_index() else {
            return Ok(());
        };
        let id = self.inner.elements[active].id;
        let version = self.inner.document.version();
        let mut pending = Vec::new();
        for (index, target) in self.inner.elements.iter().enumerate() {
            if target.id != id {
                continue;
            }
            let (Some(FieldTransform::OnExit { transform, options }), Some(range)) =
                (&target.transform, target.segment())
            else {
                continue;
            };
            let before = target.text()?;
            let after = transform.convert(&before)?;
            self.check_explicit_target(&version, active)?;
            if after != before {
                let after = options.format_text(&after)?;
                if after != before {
                    pending.push((
                        index,
                        DocumentEdit::new(range.offset, range.length, after.as_ref()),
                    ));
                }
            }
        }
        if pending.is_empty() {
            return Ok(());
        }
        let pending = self.plan_mirror_edits(&version, pending, Some(id))?;
        self.apply_explicit_edits(&version, active, pending)
    }

    fn check_explicit_target(
        &self,
        version: &DocumentVersion,
        active: usize,
    ) -> Result<(), SnippetError> {
        if self.inner.syncing.get()
            || &self.inner.document.version() != version
            || self.active_index() != Some(active)
        {
            return Err(DocumentError::ReentrantChange.into());
        }
        Ok(())
    }

    fn apply_explicit_edits(
        &self,
        version: &DocumentVersion,
        active: usize,
        pending: Vec<(usize, DocumentEdit)>,
    ) -> Result<(), SnippetError> {
        self.check_explicit_target(version, active)?;
        let edits: Vec<_> = pending.iter().map(|(_, edit)| edit.clone()).collect();
        self.validate_edits(&edits)?;
        self.check_explicit_target(version, active)?;
        let update = self.inner.document.begin_update()?;
        self.check_explicit_target(version, active)?;
        self.inner.syncing.set(true);
        let syncing = MirrorWrite(self);
        self.write_mirror_edits(pending)?;
        drop(update);
        drop(syncing);
        if !self.is_closed() {
            self.publish_field_changes();
        }
        Ok(())
    }
    pub(crate) fn validate_edits(&self, edits: &[DocumentEdit]) -> Result<(), DocumentError> {
        let validator = self.inner.edit_validator.borrow().clone();
        if let Some(validator) = validator {
            validator(edits)?;
        }
        Ok(())
    }
    fn publish_field_changes(&self) {
        for element in &self.inner.elements {
            if !element.editable {
                continue;
            }
            if let Ok(text) = element.text() {
                let changed = *element.cached_text.borrow() != text;
                if changed {
                    *element.cached_text.borrow_mut() = text;
                    element.text_changed.raise(&());
                }
            }
        }
    }
    fn active_index(&self) -> Option<usize> {
        self.inner.tab_order.get(self.inner.active.get()?).copied()
    }
    fn restore_active_owner(&self) {
        if let Some(owner) = self.active_index() {
            self.set_owner(owner);
        } else {
            self.inner.owner.set(None);
            let active = self.active_segment();
            for element in &self.inner.elements {
                let range = element.segment();
                if range.is_some_and(|range| range.length == 0) {
                    let movement =
                        if active.is_some_and(|active| range.unwrap().offset < active.offset) {
                            AnchorMovementType::BeforeInsertion
                        } else {
                            AnchorMovementType::AfterInsertion
                        };
                    element.set_movement(movement, movement);
                } else {
                    element.set_movement(
                        AnchorMovementType::AfterInsertion,
                        AnchorMovementType::BeforeInsertion,
                    );
                }
            }
        }
    }
    fn set_owner(&self, owner: usize) {
        self.inner.owner.set(Some(owner));
        for (index, element) in self.inner.elements.iter().enumerate() {
            let (start, end) = if index == owner || self.is_descendant(owner, index) {
                (
                    AnchorMovementType::BeforeInsertion,
                    AnchorMovementType::AfterInsertion,
                )
            } else if element.segment().is_some_and(|range| range.length == 0) {
                // Coincident empty fields retain their order in the expansion.
                let movement = if index < owner {
                    AnchorMovementType::BeforeInsertion
                } else {
                    AnchorMovementType::AfterInsertion
                };
                (movement, movement)
            } else {
                (
                    AnchorMovementType::AfterInsertion,
                    AnchorMovementType::BeforeInsertion,
                )
            };
            element.set_movement(start, end);
        }
        self.inner
            .caret
            .set_movement_type(if self.inner.caret_order <= owner {
                AnchorMovementType::BeforeInsertion
            } else {
                AnchorMovementType::AfterInsertion
            });
    }

    fn is_descendant(&self, mut child: usize, parent: usize) -> bool {
        while let Some(index) = self.inner.elements[child].parent {
            if index == parent {
                return true;
            }
            child = index;
        }
        false
    }

    fn prepare_edit_owner(&self, change: &DocumentChangeEventArgs) {
        if self.is_closed() || self.inner.syncing.get() {
            return;
        }
        let end = change.offset + change.removal_length();
        let contains = |index: usize| {
            self.inner.elements[index]
                .segment()
                .is_some_and(|range| range.offset <= change.offset && end <= range.end_offset())
        };
        let owner = self
            .active_index()
            .filter(|index| contains(*index))
            .or_else(|| {
                self.inner
                    .tab_order
                    .iter()
                    .copied()
                    .filter(|index| contains(*index))
                    .min_by_key(|&index| {
                        self.inner.elements[index]
                            .segment()
                            .map_or(usize::MAX, |range| range.length)
                    })
            });
        let accepting = self.inner.document.accept_changes();
        if change.removal_length() != 0
            && !self.prepare_nested_removal(change.offset, end, owner, accepting)
        {
            return;
        }
        if !accepting {
            return;
        }
        if let Some(owner) = owner {
            self.set_owner(owner);
        }
    }

    fn prepare_nested_removal(
        &self,
        start: usize,
        end: usize,
        owner: Option<usize>,
        accepting: bool,
    ) -> bool {
        let mut removed = Vec::new();
        for (index, field) in self.inner.elements.iter().enumerate() {
            let Some(range) = field.segment() else {
                continue;
            };
            let crosses = (start < range.offset && range.offset < end)
                || (start < range.end_offset() && range.end_offset() < end);
            let is_child = owner.is_some_and(|owner| self.is_descendant(index, owner));
            let replaces_child = is_child
                && ((start <= range.offset && range.end_offset() <= end && range.length != 0)
                    || (range.length == 0
                        && owner
                            .and_then(|owner| self.inner.elements[owner].segment())
                            .is_some_and(|outer| {
                                start == outer.offset && end == outer.end_offset()
                            })));
            if accepting
                && is_child
                && (crosses
                    || replaces_child
                    || removed
                        .iter()
                        .any(|&(ancestor, _)| self.is_descendant(index, ancestor)))
            {
                removed.push((index, range));
            } else if crosses && field.editable {
                self.deactivate(DeactivateReason::Deleted);
                return false;
            }
        }
        if removed.is_empty() {
            return true;
        }
        for &(index, _) in &removed {
            self.inner.elements[index].enabled.set(false);
        }
        if let Err(error) =
            self.inner
                .document
                .undo_stack()
                .push_optional(Rc::new(RestoreNestedFields {
                    session: self.downgrade(),
                    removed,
                }))
        {
            *self.inner.error.borrow_mut() = Some(error.to_string());
            self.deactivate(DeactivateReason::Error);
            return false;
        }
        true
    }
    fn record_owner(&self) {
        if self.is_closed() {
            return;
        }
        let Some(owner) = self.inner.owner.get() else {
            return;
        };
        // Recompute after changes as well: deleting a field makes both of its
        // endpoints obey the same source-order affinity before the next edit.
        self.set_owner(owner);
        if self.inner.document.accept_changes() {
            // Bracketing each edit restores its owner before either undo or
            // redo, independently of the currently selected tab stop. Weak
            // references keep history from retaining a finished session.
            if let Err(error) =
                self.inner
                    .document
                    .undo_stack()
                    .push_optional(Rc::new(RestoreOwner {
                        session: Rc::downgrade(&self.inner),
                        owner,
                    }))
            {
                *self.inner.error.borrow_mut() = Some(error.to_string());
                self.deactivate(DeactivateReason::Error);
            }
        }
    }
    fn synchronize(&self) -> Result<(), SnippetError> {
        if self.is_closed() || self.inner.syncing.get() {
            return Ok(());
        }
        // Undo/redo already contain every mirror operation.
        if !self.inner.document.accept_changes() {
            self.restore_active_owner();
            return Ok(());
        }
        self.inner.syncing.set(true);
        let _reset = MirrorWrite(self);
        let version = self.inner.document.version();
        let pending = self.plan_mirror_edits(&version, Vec::new(), None)?;
        if pending.is_empty() {
            return Ok(());
        }
        let edits: Vec<_> = pending.iter().map(|(_, edit)| edit.clone()).collect();
        self.validate_edits(&edits)?;
        if self.is_closed() {
            return Ok(());
        }
        if self.inner.document.version() != version {
            return Err(DocumentError::ReentrantChange.into());
        }
        let _update = self.inner.document.begin_continued_update()?;
        self.write_mirror_edits(pending)
    }

    fn write_mirror_edits(&self, pending: Vec<(usize, DocumentEdit)>) -> Result<(), SnippetError> {
        for (owner, edit) in pending.into_iter().rev() {
            self.set_owner(owner);
            self.inner
                .document
                .replace(edit.offset, edit.length, &edit.text)?;
        }
        Ok(())
    }

    fn plan_mirror_edits(
        &self,
        version: &DocumentVersion,
        mut pending: Vec<(usize, DocumentEdit)>,
        transformed_id: Option<usize>,
    ) -> Result<Vec<(usize, DocumentEdit)>, SnippetError> {
        let order = self
            .inner
            .mirror_order
            .as_deref()
            .unwrap_or(&self.inner.tab_order);
        for &source_index in order {
            let source = &self.inner.elements[source_index];
            if !source.enabled.get() {
                continue;
            }
            let Some(source_range) = source.segment() else {
                self.deactivate(DeactivateReason::Deleted);
                return Ok(Vec::new());
            };
            let text = self.projected_field_text(source_index, source_range, &pending)?;
            if *source.cached_text.borrow() == text {
                continue;
            }
            for (target_index, target) in self.inner.elements.iter().enumerate() {
                if target_index == source_index || target.id != source.id {
                    continue;
                }
                let Some(range) = target.segment() else {
                    continue;
                };
                if range.offset < source_range.end_offset()
                    && source_range.offset < range.end_offset()
                {
                    return Err(DocumentError::OverlappingEdits.into());
                }
                let replacement = match &target.transform {
                    Some(FieldTransform::Live(transform)) => transform.convert(&text)?,
                    Some(FieldTransform::OnExit { .. }) if transformed_id == Some(source.id) => {
                        continue
                    }
                    _ => text.clone(),
                };
                if &self.inner.document.version() != version {
                    return Err(DocumentError::ReentrantChange.into());
                }
                if self.inner.document.get_text(range.offset, range.length)? != replacement {
                    pending.push((
                        target_index,
                        DocumentEdit::new(range.offset, range.length, replacement),
                    ));
                }
            }
        }
        pending.sort_by_key(|(index, edit)| (edit.offset, *index));
        for pair in pending.windows(2) {
            if pair[0].1.offset + pair[0].1.length > pair[1].1.offset {
                return Err(DocumentError::OverlappingEdits.into());
            }
        }
        Ok(pending)
    }

    fn projected_field_text(
        &self,
        source: usize,
        range: SimpleSegment,
        pending: &[(usize, DocumentEdit)],
    ) -> Result<String, DocumentError> {
        let document = &self.inner.document;
        let mut inner_edits: Vec<_> = pending
            .iter()
            .filter(|(owner, _)| {
                let mut cursor = Some(*owner);
                while let Some(element) = cursor {
                    if element == source {
                        return true;
                    }
                    cursor = self.inner.elements[element].parent;
                }
                false
            })
            .collect();
        if inner_edits.is_empty() {
            return document.get_text(range.offset, range.length);
        }
        inner_edits.sort_by_key(|(owner, edit)| (edit.offset, *owner));
        let mut text = String::new();
        let mut offset = range.offset;
        for (_, edit) in inner_edits {
            let end = edit.offset + edit.length;
            if edit.offset < offset || end > range.end_offset() {
                return Err(DocumentError::OverlappingEdits);
            }
            text.push_str(&document.get_text(offset, edit.offset - offset)?);
            text.push_str(&edit.text);
            offset = end;
        }
        text.push_str(&document.get_text(offset, range.end_offset() - offset)?);
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snippets::{Snippet, SnippetElement};

    fn assert_fields(session: &SnippetSession, fields: &[(usize, usize)]) {
        let actual: Vec<_> = session
            .inner
            .elements
            .iter()
            .map(|element| {
                let segment = element.segment().expect("a field must remain ordered");
                (segment.offset, segment.length)
            })
            .collect();
        assert_eq!(actual, fields);
        assert!(!session.is_closed());
        assert_eq!(session.error(), None);
    }

    #[test]
    fn bound_elements_mirror_external_edits_in_one_undo_group() {
        let document = TextDocument::new();
        let snippet = Snippet::parse("let ${1:name} = ${2:value}; $1$0").unwrap();
        let session = snippet.insert(document.clone(), 0, "").unwrap();
        assert_eq!(document.text(), "let name = value; name");
        let segment = session.active_segment().unwrap();
        document
            .replace(segment.offset, segment.length, "😀")
            .unwrap();
        assert_eq!(document.text(), "let 😀 = value; 😀");
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), "let name = value; name");
        assert!(document.redo().unwrap());
        assert_eq!(document.text(), "let 😀 = value; 😀");
        assert_eq!(session.next_element(false).unwrap().length, 5);
        session.replace_active("42").unwrap();
        assert_eq!(document.text(), "let 😀 = 42; 😀");
        assert!(session.next_element(false).is_none());
        assert!(session.is_closed());
    }
    #[test]
    fn missing_defaults_and_parse_errors() {
        let document = TextDocument::new();
        let session = Snippet::parse("$1-$1$0")
            .unwrap()
            .insert(document.clone(), 0, "")
            .unwrap();
        session.replace_active("x").unwrap();
        assert_eq!(document.text(), "x-x");
        assert!(Snippet::parse("${name}").is_err());
        assert!(Snippet::parse("${1:broken").is_err());
    }

    #[test]
    fn adjacent_empty_mirrors_remain_distinct_through_unicode_edits_and_history() {
        let document = TextDocument::new();
        let session = Snippet::parse("$1$1$1$0")
            .unwrap()
            .insert(document.clone(), 0, "")
            .unwrap();
        document.insert(0, "😀").unwrap();
        assert_eq!(document.text(), "😀😀😀");
        assert_fields(&session, &[(0, 2), (2, 2), (4, 2)]);
        assert_eq!(session.caret_offset(), Some(6));
        assert_eq!(document.undo_count(), 1);

        document.insert(2, "x").unwrap();
        assert_eq!(document.text(), "😀x😀x😀x");
        assert_fields(&session, &[(0, 3), (3, 3), (6, 3)]);
        assert_eq!(session.caret_offset(), Some(9));
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), "😀😀😀");
        assert_fields(&session, &[(0, 2), (2, 2), (4, 2)]);
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), "");
        assert_fields(&session, &[(0, 0), (0, 0), (0, 0)]);
        assert!(document.redo().unwrap());
        assert!(document.redo().unwrap());
        assert_eq!(document.text(), "😀x😀x😀x");
        assert_fields(&session, &[(0, 3), (3, 3), (6, 3)]);
        session.replace_active("z").unwrap();
        assert_eq!(document.text(), "zzz");
        assert_fields(&session, &[(0, 1), (1, 1), (2, 1)]);
    }

    #[test]
    fn adjacent_placeholders_only_accept_insertions_into_the_active_field() {
        let document = TextDocument::new();
        let session = Snippet::parse("${1:a}${2:b}$1$2$0")
            .unwrap()
            .insert(document.clone(), 0, "")
            .unwrap();
        document.insert(1, "😀").unwrap();
        assert_eq!(document.text(), "a😀ba😀b");
        assert_fields(&session, &[(0, 3), (3, 1), (4, 3), (7, 1)]);
        assert_eq!(session.next_element(false), Some(SimpleSegment::new(3, 1)));
        document.insert(3, "z").unwrap();
        assert_eq!(document.text(), "a😀zba😀zb");
        assert_fields(&session, &[(0, 3), (3, 2), (5, 3), (8, 2)]);
        assert_eq!(session.next_element(true), Some(SimpleSegment::new(0, 3)));
        document.insert(0, "!").unwrap();
        assert_eq!(document.text(), "!a😀zb!a😀zb");
        assert_fields(&session, &[(0, 4), (4, 2), (6, 4), (10, 2)]);
        assert_eq!(session.caret_offset(), Some(12));
    }

    #[test]
    fn empty_fields_follow_source_order_when_tab_order_differs() {
        let document = TextDocument::new();
        let session = Snippet::parse("$3$2$1$2$3$0")
            .unwrap()
            .insert(document.clone(), 0, "")
            .unwrap();
        assert_eq!(session.active_placeholder(), Some(1));
        session.replace_active("😀").unwrap();
        assert_eq!(document.text(), "😀");
        assert_fields(&session, &[(0, 0), (0, 0), (0, 2), (2, 0), (2, 0)]);
        assert_eq!(session.next_element(false), Some(SimpleSegment::new(0, 0)));
        assert_eq!(session.active_placeholder(), Some(2));
        session.replace_active("b").unwrap();
        assert_eq!(document.text(), "b😀b");
        assert_fields(&session, &[(0, 0), (0, 1), (1, 2), (3, 1), (4, 0)]);
        assert_eq!(session.next_element(false), Some(SimpleSegment::new(0, 0)));
        assert_eq!(session.active_placeholder(), Some(3));
        session.replace_active("c").unwrap();
        assert_eq!(document.text(), "cb😀bc");
        assert_fields(&session, &[(0, 1), (1, 1), (2, 2), (4, 1), (5, 1)]);
    }

    #[test]
    fn collapsed_fields_restore_ownership_when_history_replays_after_navigation() {
        let document = TextDocument::new();
        let session = Snippet::parse("${1:A}${2:B}$1$2$0")
            .unwrap()
            .insert(document.clone(), 0, "")
            .unwrap();
        session.set_deactivate_when_empty(false);
        session.replace_active("").unwrap();
        assert_eq!(document.text(), "BB");
        assert_eq!(session.next_element(false), Some(SimpleSegment::new(0, 1)));
        session.replace_active("").unwrap();
        assert_eq!(document.text(), "");
        assert_fields(&session, &[(0, 0), (0, 0), (0, 0), (0, 0)]);

        assert!(document.undo().unwrap());
        assert_eq!(document.text(), "BB");
        assert_fields(&session, &[(0, 0), (0, 1), (1, 0), (1, 1)]);
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), "ABAB");
        assert_fields(&session, &[(0, 1), (1, 1), (2, 1), (3, 1)]);
        assert_eq!(session.active_segment(), Some(SimpleSegment::new(1, 1)));
        assert!(document.redo().unwrap());
        assert_eq!(document.text(), "BB");
        assert_eq!(session.active_segment(), Some(SimpleSegment::new(0, 1)));
        assert!(document.redo().unwrap());
        assert_eq!(document.text(), "");
        session.replace_active("😀").unwrap();
        assert_eq!(document.text(), "😀😀");
        assert_fields(&session, &[(0, 0), (0, 2), (2, 0), (2, 2)]);
        assert_eq!(session.next_element(true), Some(SimpleSegment::new(0, 0)));
        session.replace_active("x").unwrap();
        assert_eq!(document.text(), "x😀x😀");
        assert_fields(&session, &[(0, 1), (1, 2), (3, 1), (4, 2)]);
    }

    #[test]
    fn final_caret_retains_its_position_between_coincident_fields() {
        for (source, expected) in [("$0$1$1", 0), ("$1$0$1", 2), ("$1$1$0", 4), ("$1$1", 4)] {
            let document = TextDocument::new();
            let session = Snippet::parse(source)
                .unwrap()
                .insert(document.clone(), 0, "")
                .unwrap();
            session.replace_active("😀").unwrap();
            assert_eq!(document.text(), "😀😀", "{source}");
            assert_eq!(session.caret_offset(), Some(expected), "{source}");
            assert!(document.undo().unwrap());
            assert_eq!(session.caret_offset(), Some(0), "{source}");
            assert!(document.redo().unwrap());
            assert_eq!(session.caret_offset(), Some(expected), "{source}");
        }
    }

    #[test]
    fn prepared_nested_fields_preserve_boundaries_at_nonzero_offsets() {
        let document = TextDocument::from_text("[]");
        let snippet = Snippet {
            elements: vec![
                SnippetElement::Bound { id: 1 },
                SnippetElement::Container(vec![
                    SnippetElement::Caret,
                    SnippetElement::Replaceable {
                        id: 1,
                        text: String::new(),
                    },
                ]),
            ],
        };
        let prepared = snippet.prepare("").unwrap();
        document.insert(1, prepared.text()).unwrap();
        let session = prepared.activate(document.clone(), 1).unwrap();
        assert_eq!(session.active_segment(), Some(SimpleSegment::new(1, 0)));
        session.replace_active("😀").unwrap();
        assert_eq!(document.text(), "[😀😀]");
        assert_fields(&session, &[(1, 2), (3, 2)]);
        assert_eq!(session.active_segment(), Some(SimpleSegment::new(3, 2)));
        assert_eq!(session.caret_offset(), Some(3));
    }

    #[test]
    fn protected_coincident_mirrors_are_all_checked_before_any_mirror_changes() {
        let document = TextDocument::new();
        let session = Snippet::parse("$1$1$1$0")
            .unwrap()
            .insert(document.clone(), 0, "")
            .unwrap();
        let checked = Rc::new(RefCell::new(Vec::new()));
        let captured = checked.clone();
        session.set_edit_validator(Some(Rc::new(move |edits| {
            *captured.borrow_mut() = edits.to_vec();
            Err(DocumentError::ReadOnly)
        })));
        document.insert(0, "x").unwrap();
        assert_eq!(
            *checked.borrow(),
            [DocumentEdit::new(1, 0, "x"), DocumentEdit::new(1, 0, "x")]
        );
        assert_eq!(document.text(), "x");
        assert!(session.is_closed());
        assert_eq!(session.error(), Some(DocumentError::ReadOnly.to_string()));
        assert!(matches!(
            session.replace_active("y"),
            Err(SnippetError::Closed)
        ));
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), "");
    }

    #[test]
    fn history_and_subscriptions_do_not_retain_dropped_sessions() {
        let document = TextDocument::new();
        let session = Snippet::parse("$1$1")
            .unwrap()
            .insert(document.clone(), 0, "")
            .unwrap();
        session.replace_active("x").unwrap();
        let weak = Rc::downgrade(&session.inner);
        drop(session);
        assert!(weak.upgrade().is_none());
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), "");
        assert!(document.redo().unwrap());
        assert_eq!(document.text(), "xx");
    }

    #[test]
    fn rejected_mirror_stops_session_without_writing_protected_text() {
        let document = TextDocument::new();
        let session = Snippet::parse("${1:name}-$1")
            .unwrap()
            .insert(document.clone(), 0, "")
            .unwrap();
        session.set_edit_validator(Some(Rc::new(|_| Err(DocumentError::ReadOnly))));
        document.replace(0, 4, "changed").unwrap();
        assert_eq!(document.text(), "changed-name");
        assert!(session.is_closed());
        assert!(session.error().is_some());
    }
    #[test]
    fn prepared_callbacks_observe_completed_before_immediate_deactivation() {
        let document = TextDocument::new();
        let mut prepared = Snippet::parse("plain$0").unwrap().prepare("").unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        let captured = events.clone();
        prepared.on_insertion_completed(move |session| {
            assert!(session.is_insertion_completed());
            assert!(!session.is_closed());
            assert_eq!(session.segment(), Some(SimpleSegment::new(0, 5)));
            captured.borrow_mut().push("completed");
        });
        let captured = events.clone();
        prepared.on_deactivated(move |session, args| {
            assert!(session.is_closed());
            assert_eq!(args.reason, DeactivateReason::NoActiveElements);
            assert!(args.moves_to_final_caret());
            captured.borrow_mut().push("deactivated");
            session.finish(); // Reentrant deactivation is an idempotent no-op.
        });
        document.insert(0, prepared.text()).unwrap();
        let session = prepared.activate(document, 0).unwrap();
        assert_eq!(*events.borrow(), ["completed", "deactivated"]);
        assert_eq!(
            session.deactivation_reason(),
            Some(DeactivateReason::NoActiveElements)
        );
    }

    #[test]
    fn deleting_or_undoing_the_inserted_snippet_ends_it_once_without_mirror_writes() {
        for undo in [false, true] {
            let document = TextDocument::from_text("[]");
            let session = Snippet::parse("${1:name}-$1$0")
                .unwrap()
                .insert(document.clone(), 1, "")
                .unwrap();
            let reasons = Rc::new(RefCell::new(Vec::new()));
            let captured = reasons.clone();
            let _subscription = session.deactivated(move |session, args| {
                captured.borrow_mut().push(args.reason);
                session.deactivate(DeactivateReason::EscapePressed);
            });
            if undo {
                document.undo().unwrap();
            } else {
                document.remove(1, 9).unwrap();
            }
            assert_eq!(document.text(), "[]");
            assert!(session.is_closed());
            assert_eq!(*reasons.borrow(), [DeactivateReason::Deleted]);
            document.insert(1, "new").unwrap();
            assert_eq!(document.text(), "[new]");
            assert_eq!(*reasons.borrow(), [DeactivateReason::Deleted]);
        }
    }

    #[test]
    fn empty_initial_fields_remain_interactive_and_terminal_listeners_are_disposable() {
        let document = TextDocument::new();
        let session = Snippet::parse("$1$1")
            .unwrap()
            .insert(document.clone(), 0, "")
            .unwrap();
        let count = Rc::new(Cell::new(0));
        let captured = count.clone();
        let subscription = session.deactivated(move |_, _| captured.set(captured.get() + 1));
        subscription.dispose();
        session.replace_active("x").unwrap();
        session.replace_active("").unwrap();
        assert!(!session.is_closed());
        session.deactivate(DeactivateReason::ReturnPressed);
        assert_eq!(count.get(), 0);
        assert_eq!(
            session.deactivation_reason(),
            Some(DeactivateReason::ReturnPressed)
        );
        assert!(session.next_element(false).is_none());
    }
}
