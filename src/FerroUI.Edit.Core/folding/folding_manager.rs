use super::folding_events::{FoldingEvents, FoldingSubscription};
use super::{FoldingSection, NewFolding};
use crate::document::{
    DocumentChangeEventArgs, DocumentError, DocumentSubscription, SegmentError, TextDocument,
    TextSegmentCollection,
};
use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

#[derive(Debug)]
pub enum FoldingError {
    Document(DocumentError),
    Segment(SegmentError),
    InvalidRange,
    Unsorted,
}
impl fmt::Display for FoldingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Document(e) => e.fmt(f),
            Self::Segment(e) => e.fmt(f),
            Self::InvalidRange => f.write_str("Folding must be a nonempty document range"),
            Self::Unsorted => f.write_str("New foldings must be sorted by start offset"),
        }
    }
}
impl std::error::Error for FoldingError {}
impl From<DocumentError> for FoldingError {
    fn from(value: DocumentError) -> Self {
        Self::Document(value)
    }
}
impl From<SegmentError> for FoldingError {
    fn from(value: SegmentError) -> Self {
        Self::Segment(value)
    }
}

struct FoldingState {
    foldings: TextSegmentCollection<FoldingSection>,
    events: Rc<FoldingEvents>,
}
impl FoldingState {
    fn remove(&self, section: &FoldingSection) {
        if self.foldings.remove(section) {
            section.detach();
            self.events.notify();
        }
    }

    fn document_changed(&self, document: &TextDocument, change: &DocumentChangeEventArgs) {
        if self.foldings.is_empty() {
            return;
        }
        let insertion_end = change.offset + change.insertion_length();
        let end = document
            .get_line_by_offset(insertion_end)
            .map_or(insertion_end, |line| line.offset() + line.total_length());
        let Ok(affected) = self
            .foldings
            .find_overlapping_segments(change.offset, end - change.offset)
        else {
            return;
        };
        let _batch = self.events.batch();
        for section in affected {
            if section.length() == 0 {
                self.remove(&section);
            }
        }
    }
}

pub struct FoldingManager {
    document: TextDocument,
    state: Rc<FoldingState>,
    first_update: Cell<bool>,
    _document_subscription: DocumentSubscription,
}
impl FoldingManager {
    pub fn new(document: TextDocument) -> Self {
        let state = Rc::new(FoldingState {
            foldings: TextSegmentCollection::with_document(&document),
            events: Rc::default(),
        });
        let weak = Rc::downgrade(&state);
        let owner = document.downgrade();
        let subscription = document.changed(move |change| {
            if let (Some(state), Some(document)) = (weak.upgrade(), owner.upgrade()) {
                state.document_changed(&document, change);
            }
        });
        Self {
            document,
            state,
            first_update: Cell::new(true),
            _document_subscription: subscription,
        }
    }
    pub fn document(&self) -> &TextDocument {
        &self.document
    }
    pub fn changed(&self, handler: impl Fn() + 'static) -> FoldingSubscription {
        self.state.events.subscribe(handler)
    }
    pub fn create_folding(&self, start: usize, end: usize) -> Result<FoldingSection, FoldingError> {
        if start >= end || end > self.document.text_length() {
            return Err(FoldingError::InvalidRange);
        }
        let section = FoldingSection::new(&self.document, start, end, &self.state.events)?;
        self.state.foldings.add(section.clone())?;
        self.state.events.notify();
        Ok(section)
    }
    pub fn remove_folding(&self, section: &FoldingSection) {
        self.state.remove(section);
    }
    pub fn clear(&self) {
        for section in self.all_foldings() {
            section.detach();
        }
        self.state.foldings.clear();
        self.state.events.notify();
    }
    pub fn all_foldings(&self) -> Vec<FoldingSection> {
        self.state.foldings.segments()
    }
    pub fn get_next_folding(&self, offset: usize) -> Option<FoldingSection> {
        self.state
            .foldings
            .find_first_segment_with_start_after(offset)
    }
    pub fn get_next_folded_folding_start(&self, offset: usize) -> Option<usize> {
        let mut current = self.get_next_folding(offset);
        while let Some(section) = current {
            if section.is_folded() {
                return Some(section.start_offset());
            }
            current = self
                .state
                .foldings
                .get_next_segment(&section)
                .ok()
                .flatten();
        }
        None
    }
    pub fn get_foldings_at(&self, offset: usize) -> Vec<FoldingSection> {
        self.get_foldings_containing(offset)
            .into_iter()
            .filter(|s| s.start_offset() == offset)
            .collect()
    }
    pub fn get_foldings_containing(&self, offset: usize) -> Vec<FoldingSection> {
        self.state.foldings.find_segments_containing(offset)
    }
    pub fn unfold_at(&self, offset: usize) {
        let _batch = self.state.events.batch();
        for section in self.get_foldings_containing(offset) {
            if section.start_offset() < offset && offset < section.end_offset() {
                section.set_is_folded(false);
            }
        }
    }
    pub fn set_all_folded(&self, folded: bool, definitions_only: bool) {
        let _batch = self.state.events.batch();
        for section in self.all_foldings() {
            if !definitions_only || section.is_definition() {
                section.set_is_folded(folded);
            }
        }
    }

    /// Reconciles new syntax regions with tracked sections, retaining their
    /// folded state. After a parse error, unmatched later sections survive.
    pub fn update_foldings(
        &self,
        new: &[NewFolding],
        first_error_offset: Option<usize>,
    ) -> Result<(), FoldingError> {
        // Validate the complete update before altering existing state.
        if new
            .windows(2)
            .any(|p| p[0].start_offset > p[1].start_offset)
        {
            return Err(FoldingError::Unsorted);
        }
        if new
            .iter()
            .any(|s| s.start_offset > s.end_offset || s.end_offset > self.document.text_length())
        {
            return Err(FoldingError::InvalidRange);
        }
        let _batch = self.state.events.batch();
        let old = self.all_foldings();
        let mut old_index = 0;
        for region in new.iter().filter(|s| s.start_offset < s.end_offset) {
            while old_index < old.len() && old[old_index].start_offset() < region.start_offset {
                self.state.remove(&old[old_index]);
                old_index += 1;
            }
            let section =
                if old_index < old.len() && old[old_index].start_offset() == region.start_offset {
                    let section = old[old_index].clone();
                    old_index += 1;
                    section.set_end_offset(region.end_offset)?;
                    section
                } else {
                    let section = self.create_folding(region.start_offset, region.end_offset)?;
                    section.set_tag(Some(Rc::new(region.clone())));
                    if self.first_update.get() {
                        section.set_is_folded(region.default_closed);
                    }
                    section
                };
            section.set_title(region.name.clone());
            section.set_is_definition(region.is_definition);
        }
        for section in old.into_iter().skip(old_index) {
            if first_error_offset.is_some_and(|offset| section.start_offset() >= offset) {
                break;
            }
            self.state.remove(&section);
        }
        self.first_update.set(false);
        self.state.events.notify();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    #[test]
    fn folds_track_edits_and_reconcile_without_losing_user_state() {
        let doc = TextDocument::from_text("01234567890123456789");
        let manager = FoldingManager::new(doc.clone());
        let mut a = NewFolding::new(2, 8);
        a.default_closed = true;
        manager
            .update_foldings(&[a.clone(), NewFolding::new(10, 18)], None)
            .unwrap();
        let first = manager.all_foldings()[0].clone();
        assert!(first.is_folded());
        doc.insert(0, "abc").unwrap();
        assert_eq!(first.start_offset(), 5);
        let mut a = NewFolding::new(5, 12);
        a.default_closed = false;
        manager.update_foldings(&[a], Some(13)).unwrap();
        assert!(manager.all_foldings()[0].is_folded());
        assert_eq!(manager.all_foldings().len(), 2);
        manager.unfold_at(6);
        assert!(!first.is_folded());
        doc.remove(5, 7).unwrap();
        assert_eq!(manager.all_foldings().len(), 1);
    }
    #[test]
    fn invalid_update_is_atomic() {
        let manager = FoldingManager::new(TextDocument::from_text("123456"));
        manager.create_folding(0, 4).unwrap();
        assert!(manager
            .update_foldings(&[NewFolding::new(2, 5), NewFolding::new(1, 6)], None)
            .is_err());
        assert_eq!(manager.all_foldings().len(), 1);
    }

    #[test]
    fn same_start_regions_keep_their_identity_and_folded_state() {
        let manager = FoldingManager::new(TextDocument::from_text("abcd\nEFGH\nijkl\nMNOP"));
        let outer = manager.create_folding(2, 11).unwrap();
        let inner = manager.create_folding(2, 6).unwrap();
        outer.set_is_folded(true);
        let mut updated_outer = NewFolding::new(2, 12);
        updated_outer.name = Some("outer".into());
        let mut updated_inner = NewFolding::new(2, 8);
        updated_inner.name = Some("inner".into());
        manager
            .update_foldings(&[updated_outer, updated_inner], None)
            .unwrap();

        let regions = manager.get_foldings_at(2);
        assert!(regions[0] == outer);
        assert!(regions[1] == inner);
        assert_eq!(
            (outer.end_offset(), outer.title().as_deref()),
            (12, Some("outer"))
        );
        assert_eq!(
            (inner.end_offset(), inner.title().as_deref()),
            (8, Some("inner"))
        );
        assert!(outer.is_folded());
        assert!(!inner.is_folded());
    }

    #[test]
    fn live_section_ranges_and_tags_survive_reconciliation_and_whole_range_replacement() {
        let document = TextDocument::from_text("zero\none line\ntwo line\nthree line\nfour");
        let stored = Rc::new(RefCell::new(None::<FoldingSection>));
        let observation = Rc::new(RefCell::new(Vec::new()));
        let previous = stored.clone();
        let seen = observation.clone();
        let _observer = document.changed(move |_| {
            if let Some(section) = previous.borrow().as_ref() {
                seen.borrow_mut()
                    .push((section.start_offset(), section.end_offset()));
            }
        });
        let manager = FoldingManager::new(document.clone());
        let mut region = NewFolding::new(1, 9);
        region.name = Some("first".into());
        region.default_closed = true;
        manager
            .update_foldings(&[region.clone(), NewFolding::new(10, 14)], None)
            .unwrap();
        let section = manager.get_next_folding(0).unwrap();
        let neighbor = manager.get_next_folding(10).unwrap();
        assert_eq!(section.tag::<NewFolding>().as_deref(), Some(&region));
        let payload = Rc::new("app metadata".to_owned());
        section.set_tag(Some(payload.clone()));
        assert!(section.tag::<NewFolding>().is_none());

        section.set_start_offset(15).unwrap();
        assert_eq!(section.end_offset(), 23);
        section.set_end_offset(28).unwrap();
        section.set_length(17).unwrap();
        assert!(manager.get_next_folding(0).unwrap() == neighbor);
        assert!(manager.get_foldings_containing(20)[0] == section);
        assert_eq!(section.text_content().unwrap(), "wo line\nthree lin");
        assert_eq!(
            section.set_end_offset(14),
            Err(SegmentError::InvalidEndOffset)
        );
        assert!(matches!(
            section.set_length(100),
            Err(SegmentError::OutsideDocument { .. })
        ));
        assert_eq!(section.end_offset(), 32);

        manager
            .update_foldings(&[NewFolding::new(10, 14), NewFolding::new(15, 32)], None)
            .unwrap();
        assert!(manager.get_next_folding(15).unwrap() == section);
        assert!(Rc::ptr_eq(&section.tag::<String>().unwrap(), &payload));
        assert!(section.is_folded());
        *stored.borrow_mut() = Some(section.clone());

        document.replace(15, 17, "😀\nQ").unwrap();
        assert_eq!(*observation.borrow(), [(15, 19)]);
        assert_eq!(section.text_content().unwrap(), "😀\nQ");
        assert!(section.is_folded());
        section.set_length(0).unwrap();
        assert_eq!(manager.get_next_folded_folding_start(15), Some(15));
        section.set_length(4).unwrap();
        document.remove(15, 4).unwrap();
        assert!(manager.get_next_folding(15).is_none());
        assert_eq!(section.length(), 0);
        assert!(!section.is_folded());
        document.insert(0, "prefix").unwrap();
        assert_eq!(section.start_offset(), 15);
        assert_eq!(section.text_content().unwrap(), "");
    }

    #[test]
    fn notifications_observe_committed_state_and_batch_bulk_updates() {
        let manager = Rc::new(FoldingManager::new(TextDocument::from_text("0123456789")));
        let observed = Rc::new(RefCell::new(Vec::new()));
        let weak = Rc::downgrade(&manager);
        let values = observed.clone();
        let subscription = manager.changed(move || {
            let manager = weak.upgrade().unwrap();
            values.borrow_mut().push(
                manager
                    .all_foldings()
                    .iter()
                    .filter(|s| s.is_folded())
                    .count(),
            );
        });
        let first = manager.create_folding(0, 4).unwrap();
        manager.create_folding(5, 9).unwrap();
        first.set_is_folded(true);
        manager.set_all_folded(true, false);
        manager.clear();
        assert_eq!(*observed.borrow(), vec![0, 0, 1, 2, 0]);
        subscription.dispose();
        manager.create_folding(0, 2).unwrap();
        assert_eq!(observed.borrow().len(), 5);
    }
}
