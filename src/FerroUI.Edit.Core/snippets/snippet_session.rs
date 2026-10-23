use super::snippet::Placeholder;
use super::{DeactivateReason, SnippetError, SnippetEventArgs};
use crate::document::{
    AnchorMovementType, AnchorSegment, DocumentEdit, DocumentError, DocumentSubscription,
    IUndoableOperation, SimpleSegment, TextAnchor, TextDocument,
};
use crate::utils::Event;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

type InsertionHandler = Rc<dyn Fn(&SnippetSession)>;
type DeactivationHandler = Rc<dyn Fn(&SnippetSession, &SnippetEventArgs)>;
#[derive(Default)]
pub(crate) struct SnippetLifecycle {
    pub insertion_completed: Vec<InsertionHandler>,
    pub deactivated: Vec<DeactivationHandler>,
}

pub type SnippetEditValidator = dyn Fn(&[DocumentEdit]) -> Result<(), DocumentError>;

struct ActiveElement {
    id: usize,
    start: TextAnchor,
    end: TextAnchor,
}
impl ActiveElement {
    fn segment(&self) -> Option<SimpleSegment> {
        let start = self.start.offset().ok()?;
        let end = self.end.offset().ok()?;
        (end >= start).then(|| SimpleSegment::new(start, end - start))
    }
}
struct SessionInner {
    document: TextDocument,
    elements: Vec<ActiveElement>,
    tab_order: Vec<usize>,
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
    last_text: RefCell<String>,
    subscriptions: RefCell<Vec<DocumentSubscription>>,
    error: RefCell<Option<String>>,
    edit_validator: RefCell<Option<Rc<SnippetEditValidator>>>,
}
impl Drop for SessionInner {
    fn drop(&mut self) {
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
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

/// A linked-placeholder insertion context. Edits through any host to the active
/// editable element propagate to its bound occurrences after the document's
/// transaction completes. The mirror edits continue the original undo group.
#[derive(Clone)]
pub struct SnippetSession {
    inner: Rc<SessionInner>,
}
impl SnippetSession {
    pub(crate) fn new(
        document: TextDocument,
        offset: usize,
        placeholders: Vec<Placeholder>,
        caret: usize,
        caret_order: usize,
        length: usize,
        lifecycle: SnippetLifecycle,
    ) -> Result<Self, SnippetError> {
        let mut elements = Vec::new();
        let mut order = BTreeMap::new();
        for placeholder in placeholders {
            let start = document.create_anchor(offset + placeholder.start)?;
            let end = document.create_anchor(offset + placeholder.end)?;
            start.set_survive_deletion(true);
            end.set_survive_deletion(true);
            if placeholder.editable {
                order.entry(placeholder.id).or_insert(elements.len());
            }
            elements.push(ActiveElement {
                id: placeholder.id,
                start,
                end,
            });
        }
        let caret = document.create_anchor(offset + caret)?;
        caret.set_movement_type(AnchorMovementType::AfterInsertion);
        caret.set_survive_deletion(true);
        let tab_order: Vec<_> = order.into_values().collect();
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
            last_text: RefCell::new(String::new()),
            subscriptions: RefCell::new(Vec::new()),
            error: RefCell::new(None),
            edit_validator: RefCell::new(None),
        });
        let session = Self { inner };
        for handler in lifecycle.deactivated {
            session.deactivated(move |context, args| handler(context, args));
        }
        session.restore_active_owner();
        session.refresh_last_text();
        let weak = Rc::downgrade(&session.inner);
        let changing = document.changing(move |_| {
            if let Some(inner) = weak.upgrade() {
                SnippetSession { inner }.record_owner();
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
            }
        });
        *session.inner.subscriptions.borrow_mut() = vec![changing, changed, subscription];
        session.inner.completed.set(true);
        for handler in lifecycle.insertion_completed {
            handler(&session);
        }
        if session.inner.tab_order.is_empty() {
            session.deactivate(DeactivateReason::NoActiveElements);
        }
        Ok(session)
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
        self.inner.elements.get(self.active_index()?)?.segment()
    }
    pub fn active_placeholder(&self) -> Option<usize> {
        self.inner.elements.get(self.active_index()?).map(|e| e.id)
    }
    pub fn caret_offset(&self) -> Option<usize> {
        self.inner.caret.offset().ok()
    }
    pub fn next_element(&self, backwards: bool) -> Option<SimpleSegment> {
        let index = self.inner.active.get()?;
        let next = if backwards {
            index.saturating_sub(1)
        } else {
            index + 1
        };
        if next >= self.inner.tab_order.len() {
            self.deactivate(DeactivateReason::Completed);
            return None;
        }
        self.inner.active.set(Some(next));
        self.restore_active_owner();
        self.refresh_last_text();
        self.active_segment()
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
        self.inner
            .deactivated
            .raise(&(self.clone(), SnippetEventArgs::new(reason)));
    }
    pub fn replace_active(&self, text: &str) -> Result<(), SnippetError> {
        let segment = self.active_segment().ok_or(SnippetError::Closed)?;
        let _update = self.inner.document.begin_update()?;
        self.inner
            .document
            .replace(segment.offset, segment.length, text)?;
        Ok(())
    }
    fn active_index(&self) -> Option<usize> {
        self.inner.tab_order.get(self.inner.active.get()?).copied()
    }
    fn restore_active_owner(&self) {
        if let Some(owner) = self.active_index() {
            self.set_owner(owner);
        }
    }
    fn set_owner(&self, owner: usize) {
        self.inner.owner.set(Some(owner));
        for (index, element) in self.inner.elements.iter().enumerate() {
            let (start, end) = if index == owner {
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
            element.start.set_movement_type(start);
            element.end.set_movement_type(end);
        }
        self.inner
            .caret
            .set_movement_type(if self.inner.caret_order <= owner {
                AnchorMovementType::BeforeInsertion
            } else {
                AnchorMovementType::AfterInsertion
            });
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
    fn refresh_last_text(&self) {
        if let Some(segment) = self.active_segment() {
            if let Ok(text) = self.inner.document.get_text(segment.offset, segment.length) {
                *self.inner.last_text.borrow_mut() = text;
            }
        }
    }
    fn synchronize(&self) -> Result<(), SnippetError> {
        if self.is_closed() || self.inner.syncing.get() {
            return Ok(());
        }
        self.restore_active_owner();
        let Some(segment) = self.active_segment() else {
            self.finish();
            return Ok(());
        };
        let text = self
            .inner
            .document
            .get_text(segment.offset, segment.length)?;
        if *self.inner.last_text.borrow() == text {
            return Ok(());
        }
        *self.inner.last_text.borrow_mut() = text.clone();
        // Undo/redo already contain the mirror operations and must replay them
        // without causing additional changes from this listener.
        if !self.inner.document.accept_changes() {
            return Ok(());
        }
        let active = self.active_index().ok_or(SnippetError::Closed)?;
        let id = self.inner.elements[active].id;
        let mut targets: Vec<_> = self
            .inner
            .elements
            .iter()
            .enumerate()
            .filter(|(i, e)| *i != active && e.id == id)
            .filter_map(|(index, element)| element.segment().map(|range| (index, range)))
            .collect();
        targets.sort_by_key(|(index, range)| (range.offset, *index));
        if targets.is_empty() {
            return Ok(());
        }
        for pair in targets.windows(2) {
            if pair[0].1.offset + pair[0].1.length > pair[1].1.offset {
                return Err(DocumentError::OverlappingEdits.into());
            }
        }
        let edits: Vec<_> = targets
            .iter()
            .map(|(_, range)| DocumentEdit::new(range.offset, range.length, &text))
            .collect();
        let validator = self.inner.edit_validator.borrow().clone();
        if let Some(validator) = validator {
            validator(&edits)?;
        }
        self.inner.syncing.set(true);
        struct Reset<'a>(&'a SnippetSession);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.restore_active_owner();
                self.0.inner.syncing.set(false);
            }
        }
        let _reset = Reset(self);
        let _update = self.inner.document.begin_continued_update()?;
        for ((owner, _), edit) in targets.iter().zip(&edits).rev() {
            self.set_owner(*owner);
            self.inner
                .document
                .replace(edit.offset, edit.length, &edit.text)?;
        }
        Ok(())
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
            session.deactivated(move |session, args| {
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
